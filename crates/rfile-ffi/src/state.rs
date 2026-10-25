// SPDX-License-Identifier: GPL-3.0-only
//! Safe cookie semantics. No foreign pointers or untrusted native parsers.
use rfile::{Database, Engine, Error, Flags, Limits};
use std::{
    ffi::{CString, OsString},
    io::{Read, Write},
    os::unix::ffi::{OsStrExt, OsStringExt},
    path::{Path, PathBuf},
    sync::Arc,
};

pub const DEFAULT_MAGIC: &str = match option_env!("RFILE_MAGIC_PATH") {
    Some(p) => p,
    None => concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/magic.mgc"),
};
pub const PARAM_DEFAULTS: [usize; 10] = [
    50,
    150,
    2048,
    32768,
    256,
    8192,
    7 * 1024 * 1024,
    64 * 1024,
    128 * 1024 * 1024,
    64,
];
pub fn string(mut bytes: Vec<u8>) -> CString {
    if let Some(n) = bytes.iter().position(|&b| b == 0) {
        bytes.truncate(n);
    }
    CString::new(bytes).expect("interior NUL removed")
}
pub fn path(bytes: &[u8]) -> PathBuf {
    PathBuf::from(OsString::from_vec(bytes.to_vec()))
}
pub fn default_path(action: i32) -> Vec<u8> {
    if let Some(magic) = std::env::var_os("MAGIC") {
        return magic.as_bytes().to_vec();
    }
    let default = DEFAULT_MAGIC.as_bytes();
    if action != 0 {
        return default.to_vec();
    }
    if let Some(home) = std::env::var_os("HOME") {
        let mut candidate = home.as_bytes().to_vec();
        candidate.extend_from_slice(b"/.magic.mgc");
        if std::fs::metadata(path(&candidate)).is_err() {
            candidate = home.as_bytes().to_vec();
            candidate.extend_from_slice(b"/.magic");
            match std::fs::metadata(path(&candidate)) {
                Ok(m) if m.is_dir() => {
                    candidate = home.as_bytes().to_vec();
                    candidate.extend_from_slice(b"//.magic/magic.mgc");
                    if std::fs::File::open(path(&candidate)).is_err() {
                        return default.to_vec();
                    }
                }
                Ok(_) => {}
                Err(_) => return default.to_vec(),
            }
        }
        candidate.push(b':');
        candidate.extend_from_slice(default);
        return candidate;
    }
    default.to_vec()
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Load,
    Compile,
    Check,
    List,
}
impl Action {
    pub fn id(self) -> i32 {
        match self {
            Self::Load => 0,
            Self::Check => 1,
            Self::Compile => 2,
            Self::List => 3,
        }
    }
}
pub struct State {
    pub flags: i32,
    pub params: [usize; 10],
    pub loaded: Option<Arc<Database>>,
    pub result: Option<CString>,
    pub error: Option<CString>,
    pub error_number: i32,
    pub warnings: usize,
    last_source: Option<(Vec<u8>, usize)>,
    diagnostic_line: usize,
    read_remaining: usize,
    output_remaining: usize,
}
impl State {
    pub fn new(flags: i32) -> Self {
        Self {
            flags,
            params: PARAM_DEFAULTS,
            loaded: None,
            result: None,
            error: None,
            error_number: -1,
            warnings: 0,
            last_source: None,
            diagnostic_line: 0,
            read_remaining: 128 * 1024 * 1024,
            output_remaining: 16 * 1024 * 1024,
        }
    }
    pub fn fail(&mut self, bytes: Vec<u8>, number: i32) {
        if self.error.is_none() {
            self.error = Some(string(bytes));
            self.error_number = number;
        }
    }
    pub fn rust_error(&mut self, error: Error) {
        let number = match &error {
            Error::Io(e) | Error::Filesystem { source: e, .. } => {
                e.raw_os_error().unwrap_or(libc::EIO)
            }
            _ if error.is_limit() => libc::ERANGE,
            Error::Unsupported(_) => libc::ENOTSUP,
            _ => 0,
        };
        let bytes = error
            .raw_diagnostic()
            .map_or_else(|| error.to_string().into_bytes(), ToOwned::to_owned);
        self.fail(bytes, number);
    }
    pub fn reset(&mut self, check_loaded: bool) -> bool {
        if check_loaded && self.loaded.is_none() {
            self.fail(b"no magic files loaded".to_vec(), 0);
            return false;
        }
        self.result = None;
        self.error = None;
        self.error_number = -1;
        true
    }
    pub fn limits(&self) -> Limits {
        let p = self.params;
        Limits {
            indirect: p[0],
            names: p[1],
            elf_phnum: p[2],
            elf_shnum: p[3],
            elf_notes: p[4],
            regex: p[5],
            bytes: p[6],
            encoding: p[7],
            elf_shsize: p[8],
            magwarn: p[9],
            ..Limits::default()
        }
    }
    pub fn set_param(&mut self, id: usize, value: usize) -> Result<(), i32> {
        if id >= 10 {
            return Err(libc::EINVAL);
        }
        let v = if id <= 5 {
            usize::from(value as u16)
        } else {
            value
        };
        let old = self.params[id];
        self.params[id] = v;
        if let Err(e) = self.limits().validate() {
            self.params[id] = old;
            self.rust_error(e);
            return Err(libc::ERANGE);
        }
        Ok(())
    }
    pub fn engine(&self) -> rfile::Result<Engine> {
        let db = self
            .loaded
            .as_ref()
            .ok_or_else(|| Error::Database("no magic files loaded".into()))?;
        let mut engine = Engine::new(db.clone())
            .with_time_context(rfile::TimeContext::from_environment()?)
            .with_display_context(rfile::DisplayContext::from_environment()?);
        engine.flags = Flags(self.flags as u32);
        engine.limits = self.limits();
        Ok(engine)
    }
    pub fn accept_observed(
        &mut self,
        result: rfile::Result<rfile::LegacyInspection>,
    ) -> *const libc::c_char {
        match result {
            Ok(observed) => {
                if let Some(line) = observed.last_magic_line {
                    self.diagnostic_line = line as usize;
                    if let Some((_, old)) = &mut self.last_source {
                        *old = line as usize;
                    }
                }
                self.accept(observed.result)
            }
            Err(e) => self.accept(Err(e)),
        }
    }
    pub fn accept(&mut self, result: rfile::Result<rfile::Classification>) -> *const libc::c_char {
        match result {
            Ok(c) => {
                self.result = Some(string(c.output));
                self.result.as_ref().expect("stored result").as_ptr()
            }
            Err(e) => {
                self.rust_error(e);
                std::ptr::null()
            }
        }
    }
    fn warning(&mut self, message: &[u8]) -> rfile::Result<()> {
        self.warnings = self.warnings.saturating_add(1);
        let mut bytes = Vec::new();
        if self.warnings == self.params[9] {
            let (name, line) = self
                .last_source
                .as_ref()
                .map_or((b"(null)".as_slice(), self.diagnostic_line), |(n, l)| {
                    (n.as_slice(), *l)
                });
            bytes.extend_from_slice(name);
            bytes.extend_from_slice(
                format!(
                    ", {line}: Maximum number of warnings ({}) exceeded.\n",
                    self.params[9]
                )
                .as_bytes(),
            );
            bytes.extend_from_slice(name);
            bytes.extend_from_slice(
                format!(", {line}: Additional warnings are suppressed.\n").as_bytes(),
            );
        } else if self.warnings < self.params[9] {
            if let Some((name, line)) = &self.last_source {
                bytes.extend_from_slice(name);
                bytes.extend_from_slice(format!(", {line}: ").as_bytes());
            }
            bytes.extend_from_slice(b"Warning: ");
            bytes.extend_from_slice(message);
            bytes.push(b'\n');
        }
        self.emit(&bytes, true)
    }
    fn emit(&mut self, bytes: &[u8], stderr: bool) -> rfile::Result<()> {
        if bytes.len() > self.output_remaining {
            return Err(Error::Limit("database diagnostic output"));
        }
        self.output_remaining -= bytes.len();
        if stderr {
            std::io::stdout().flush()?;
            std::io::stderr().write_all(bytes)?;
        } else {
            std::io::stdout().write_all(bytes)?;
        }
        Ok(())
    }
    fn read(&mut self, path: &Path) -> rfile::Result<Vec<u8>> {
        let mut bytes = Vec::new();
        use std::os::unix::fs::OpenOptionsExt;
        let file = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NONBLOCK | libc::O_NOCTTY)
            .open(path)?;
        if !file.metadata()?.is_file() {
            return Err(Error::Unsupported("database must be a regular file".into()));
        }
        file.take(self.read_remaining as u64 + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() > self.read_remaining {
            return Err(Error::Limit("aggregate database bytes"));
        }
        self.read_remaining -= bytes.len();
        Ok(bytes)
    }
    pub fn load_buffers(&mut self, databases: Vec<Database>) -> i32 {
        match Database::merge(databases) {
            Ok(db) => {
                self.loaded = Some(Arc::new(db));
                0
            }
            Err(e) => {
                self.loaded = None;
                self.rust_error(e);
                -1
            }
        }
    }
    pub fn action(&mut self, explicit: Option<Vec<u8>>, action: Action) -> i32 {
        self.reset(false);
        self.loaded = None;
        self.last_source = None;
        self.read_remaining = 128 * 1024 * 1024;
        self.output_remaining = 16 * 1024 * 1024;
        let paths = explicit.unwrap_or_else(|| default_path(action.id()));
        if paths.len() > 65536 {
            self.rust_error(Error::Limit("database path list"));
            return -1;
        }
        let mut combined: Option<Database> = None;
        let mut any = false;
        for (index, name) in paths.split(|&b| b == b':').enumerate() {
            if name.is_empty() {
                break;
            }
            if index >= 16 {
                self.rust_error(Error::Limit("database list"));
                break;
            }
            match self.one_path(name, action) {
                Ok(None) => {}
                Ok(Some(db)) => {
                    any = true;
                    if action != Action::Compile {
                        let merged = if let Some(old) = combined.take() {
                            Database::merge(vec![old, db])
                        } else {
                            Ok(db)
                        };
                        match merged {
                            Ok(db) => combined = Some(db),
                            Err(e) => {
                                self.rust_error(e);
                                any = false;
                                break;
                            }
                        }
                        if action == Action::List {
                            match combined
                                .as_ref()
                                .expect("combined")
                                .legacy_list(self.output_remaining)
                                .and_then(|b| self.emit(&b, false))
                            {
                                Ok(()) => {}
                                Err(e) => {
                                    self.rust_error(e);
                                    any = false;
                                    break;
                                }
                            }
                        }
                    }
                }
                Err(Error::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => self.rust_error(e),
            }
        }
        if any {
            self.loaded =
                Some(Arc::new(combined.unwrap_or_else(|| {
                    Database::merge(Vec::new()).expect("empty database")
                })));
            0
        } else {
            self.fail(b"could not find any valid magic files!".to_vec(), 0);
            -1
        }
    }
    fn one_path(&mut self, name: &[u8], action: Action) -> rfile::Result<Option<Database>> {
        let path = path(name);
        if action != Action::Compile {
            let mut compiled = name.strip_suffix(b".mgc").unwrap_or(name).to_vec();
            if self.flags as u32 & Flags::MIME != 0 {
                let mut mime = compiled.clone();
                mime.extend_from_slice(b".mime.mgc");
                if std::fs::File::open(super::state::path(&mime)).is_ok() {
                    compiled = mime;
                    self.flags &= Flags::MIME_TYPE as i32;
                } else {
                    compiled.extend_from_slice(b".mgc");
                }
            } else {
                compiled.extend_from_slice(b".mgc");
            }
            if name.windows(5).any(|s| s == b".mime") {
                self.flags &= Flags::MIME_TYPE as i32;
            }
            match self.read(&super::state::path(&compiled)) {
                Ok(bytes) => match crate::compiled::load(&bytes, &compiled, true) {
                    Ok(db) => return Ok(Some(db)),
                    Err(e) => self.rust_error(e),
                },
                Err(Error::Io(_)) => {}
                Err(e) => return Err(e),
            }
            if self.flags as u32 & Flags::CHECK != 0 {
                let mut message = b"using regular magic file `".to_vec();
                message.extend_from_slice(name);
                message.push(b'\'');
                self.warning(&message)?;
            }
        }
        self.flags |= Flags::CHECK as i32;
        let files = source_files(&path)?;
        let mut sources = Vec::new();
        for file in &files {
            let bytes = self.read(file)?;
            sources.push(rfile::SourceFragment {
                name: file.as_os_str().as_bytes().to_vec(),
                bytes,
            });
        }
        let report = Database::source_action(
            &sources,
            &rfile::TimeContext::from_environment()?,
            rfile::SourceActionOptions {
                check: action == Action::Check,
                warning_count: self.warnings,
                warning_limit: self.params[9],
                output_limit: self.output_remaining,
            },
        );
        self.warnings = report.warning_count;
        self.last_source = report.last_source;
        if let Some((_, line)) = &self.last_source {
            self.diagnostic_line = *line;
        }
        if let Some(error) = report.native_error {
            self.fail(error, 0);
        }
        self.emit(&report.output, true)?;
        let db = match report.database {
            Ok(db) => db,
            Err(_) if report.native_failure => return Ok(None),
            Err(e) => return Err(e),
        };
        if action == Action::Compile {
            let base = path.file_name().unwrap_or_default().as_bytes();
            let mut name = base.strip_suffix(b".mgc").unwrap_or(base).to_vec();
            name.extend_from_slice(b".mgc");
            write_compiled(&super::state::path(&name), &db.compile()?)?;
        }
        Ok(Some(db))
    }
}
/// Atomic same-directory replacement; never follow a destination symlink.
/// An intentional ownership/inode difference from C's O_TRUNC behavior.
fn write_compiled(path: &Path, bytes: &[u8]) -> rfile::Result<()> {
    use std::os::unix::fs::OpenOptionsExt;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut created = None;
    for _ in 0..32 {
        let temp = parent.join(format!(
            ".rfile-magic-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o644)
            .open(&temp)
        {
            Ok(file) => {
                created = Some((temp, file));
                break;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e.into()),
        }
    }
    let (temp, mut file) = created.ok_or(Error::Limit("database staging attempts"))?;
    let result = (|| -> rfile::Result<()> {
        file.write_all(bytes)?;
        file.sync_all()?;
        std::fs::rename(&temp, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result
}
fn source_files(path: &Path) -> rfile::Result<Vec<PathBuf>> {
    if !path.is_dir() {
        return Ok(vec![path.to_path_buf()]);
    }
    let mut files = Vec::new();
    for (i, e) in std::fs::read_dir(path)?.enumerate() {
        if i >= 16384 {
            return Err(Error::Limit("magic directory entries"));
        }
        let e = e?;
        if !e.file_name().as_bytes().starts_with(b".") && e.path().is_file() {
            if files.len() >= 4096 {
                return Err(Error::Limit("magic source files"));
            }
            files.push(e.path());
        }
    }
    files.sort();
    Ok(files)
}
