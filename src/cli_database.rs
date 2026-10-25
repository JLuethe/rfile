// SPDX-License-Identifier: GPL-3.0-only
//! CLI database actions. Source parsing and header diagnostics share the safe
//! ABI primitives; filesystem publication stays in the CLI sandbox boundary.
use super::*;
use crate::{SourceActionOptions, SourceFragment, TimeContext};
struct Action {
    flags: Flags,
    warnings: usize,
    last_source: Option<(Vec<u8>, usize)>,
    read_remaining: usize,
    output_remaining: usize,
    error: Option<Error>,
    time: TimeContext,
}
impl Action {
    fn emit(&mut self, value: &[u8], stderr: bool) -> Result<()> {
        if value.len() > self.output_remaining {
            return Err(Error::Limit("database action output"));
        }
        self.output_remaining -= value.len();
        if stderr {
            io::stdout().flush()?;
            io::stderr().lock().write_all(value)?;
        } else {
            io::stdout().lock().write_all(value)?;
        }
        Ok(())
    }
    fn remember(&mut self, error: Error) {
        if self.error.is_none() {
            self.error = Some(error);
        }
    }
    fn read(&mut self, path: &Path) -> Result<Vec<u8>> {
        let mut bytes = Vec::new();
        rfile_platform::open_regular_database(path)?
            .take(self.read_remaining as u64 + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() > self.read_remaining {
            return Err(Error::Limit("aggregate database bytes"));
        }
        self.read_remaining -= bytes.len();
        Ok(bytes)
    }
    fn source_warning(&mut self, name: &[u8]) -> Result<()> {
        self.warnings += 1;
        let mut message = Vec::new();
        // file.c does not apply -P parameters to database actions: the newly
        // opened action cookie has its default warning cap of 64.
        if self.warnings == 64 {
            let (source, line) = self
                .last_source
                .as_ref()
                .map_or((&b"(null)"[..], 0), |(s, l)| (s.as_slice(), *l));
            for suffix in [
                format!(", {line}: Maximum number of warnings (64) exceeded.\n"),
                format!(", {line}: Additional warnings are suppressed.\n"),
            ] {
                message.extend_from_slice(source);
                message.extend_from_slice(suffix.as_bytes());
            }
        } else if self.warnings < 64 {
            if let Some((source, line)) = &self.last_source {
                message.extend_from_slice(source);
                message.extend_from_slice(format!(", {line}: ").as_bytes());
            }
            message.extend_from_slice(b"Warning: using regular magic file `");
            message.extend_from_slice(name);
            message.extend_from_slice(b"'\n");
        }
        self.emit(&message, true)
    }
    fn one(&mut self, action: char, path: &Path) -> Result<Option<Database>> {
        let name = bytes(path.as_os_str());
        if action != 'C' {
            let base = name.strip_suffix(b".mgc").unwrap_or(&name);
            let mut compiled = base.to_vec();
            if self.flags.has(Flags::MIME) {
                let mut mime = base.to_vec();
                mime.extend_from_slice(b".mime.mgc");
                let mime_path = PathBuf::from(os_bytes(mime.clone()));
                if rfile_platform::open_regular_database(&mime_path).is_ok() {
                    compiled = mime;
                    self.flags.0 &= Flags::MIME_TYPE;
                } else {
                    compiled.extend_from_slice(b".mgc");
                }
            } else {
                compiled.extend_from_slice(b".mgc");
            }
            if name.windows(5).any(|b| b == b".mime") {
                self.flags.0 &= Flags::MIME_TYPE;
            }
            match self.read(Path::new(&os_bytes(compiled.clone()))) {
                Ok(value) => match Database::from_compiled_named(&value, &compiled, true) {
                    Ok(db) => return Ok(Some(db)),
                    Err(e) => self.remember(e),
                },
                Err(Error::Io(_)) => {}
                Err(e) => return Err(e),
            }
            if self.flags.has(Flags::CHECK) {
                self.source_warning(&name)?;
            }
        }
        self.flags.0 |= Flags::CHECK;
        let mut files = Vec::new();
        if path.is_dir() {
            for (index, entry) in std::fs::read_dir(path)?.enumerate() {
                if index >= 16384 {
                    return Err(Error::Limit("magic directory entries"));
                }
                let entry = entry?;
                if !bytes(&entry.file_name()).starts_with(b".") && entry.path().is_file() {
                    if files.len() >= 4096 {
                        return Err(Error::Limit("magic source files"));
                    }
                    files.push(entry.path());
                }
            }
            files.sort();
        } else {
            files.push(path.to_path_buf());
        }
        let mut sources = Vec::new();
        for file in files {
            let value = match self.read(&file) {
                Ok(value) => value,
                Err(error) => {
                    let line = self.last_source.as_ref().map_or(0, |(_, line)| *line);
                    self.last_source = Some((bytes(file.as_os_str()), line));
                    if action == 'c' {
                        self.emit(b"cont\toffset\ttype\topcode\tmask\tvalue\tdesc\n", true)?;
                    }
                    return Err(error);
                }
            };
            sources.push(SourceFragment {
                name: bytes(file.as_os_str()),
                bytes: value,
            });
        }
        let report = Database::source_action(
            &sources,
            &self.time,
            SourceActionOptions {
                check: action == 'c',
                warning_count: self.warnings,
                warning_limit: 64,
                output_limit: self.output_remaining,
            },
        );
        self.warnings = report.warning_count;
        self.last_source = report.last_source;
        if let Some(error) = report.native_error {
            self.remember(Error::LegacyFormat(error));
        }
        self.emit(&report.output, true)?;
        let db = match report.database {
            Ok(db) => db,
            Err(_) if report.native_failure => return Ok(None),
            Err(e) => return Err(e),
        };
        if action == 'C' {
            let base = bytes(path.file_name().unwrap_or_default());
            let mut name = base.strip_suffix(b".mgc").unwrap_or(&base).to_vec();
            name.extend_from_slice(b".mgc");
            atomic_database_write(Path::new(&os_bytes(name)), &db.compile()?)?;
        }
        Ok(Some(db))
    }
}
pub(super) fn execute(action: char, path: &OsStr, flags: Flags) -> Result<()> {
    if bytes(path).len() > 65536 {
        return Err(Error::Limit("database path list"));
    }
    let mut state = Action {
        flags: Flags(flags.0 | Flags::CHECK),
        warnings: 0,
        last_source: None,
        read_remaining: 128 * 1024 * 1024,
        output_remaining: 16 * 1024 * 1024,
        error: None,
        time: TimeContext::from_environment()?,
    };
    let mut combined = None;
    let mut any = false;
    for (index, path) in std::env::split_paths(path).enumerate() {
        if path.as_os_str().is_empty() {
            break;
        }
        if index >= 16 {
            return Err(Error::Limit("database list"));
        }
        match state.one(action, &path) {
            Ok(Some(db)) => {
                any = true;
                if action == 'l' {
                    let db = match combined.take() {
                        Some(old) => Database::merge(vec![old, db])?,
                        None => db,
                    };
                    state.emit(&db.legacy_list(state.output_remaining)?, false)?;
                    combined = Some(db);
                }
            }
            Ok(None) => {}
            Err(Error::Io(e)) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) if e.is_limit() => return Err(e),
            Err(e) => state.remember(e),
        }
    }
    if any {
        Ok(())
    } else {
        Err(state.error.unwrap_or_else(|| {
            Error::LegacyFormat(b"could not find any valid magic files!".to_vec())
        }))
    }
}
