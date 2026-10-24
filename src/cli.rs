/*
 * Copyright (c) Ian F. Darwin 1986-1995.
 * Software written by Ian F. Darwin and others;
 * maintained 1995-present by Christos Zoulas and others.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice immediately at the beginning of the file, without modification,
 *    this list of conditions, and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 *
 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED. IN NO EVENT SHALL THE AUTHOR OR CONTRIBUTORS BE LIABLE FOR
 * ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 */
// rfile contributions: Copyright (C) 2026 rfile contributors.
// SPDX-License-Identifier: GPL-3.0-only

// Independent Rust implementation; upstream behavior and retained data notices credited in NOTICE.
use crate::{Capabilities, Database, Engine, Error, Flags, Limits, Result, SemanticProfile};
use std::{
    ffi::{OsStr, OsString},
    io::{self, BufRead, Read, Write},
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Default)]
struct Presentation {
    brief: usize,
    no_pad: bool,
    no_buffer: bool,
    nulls: usize,
    separator: Vec<u8>,
}
fn bytes(s: &OsStr) -> Vec<u8> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        s.as_bytes().to_vec()
    }
    #[cfg(not(unix))]
    {
        s.to_string_lossy().as_bytes().to_vec()
    }
}
fn os_bytes(b: Vec<u8>) -> OsString {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        OsString::from_vec(b)
    }
    #[cfg(not(unix))]
    {
        OsString::from(String::from_utf8_lossy(&b).into_owned())
    }
}
fn db_path(explicit: &Option<OsString>) -> Option<OsString> {
    explicit.clone().or_else(|| std::env::var_os("MAGIC"))
}
fn load(
    path: &Option<OsString>,
    flags: Flags,
    limits: &Limits,
    profile: SemanticProfile,
) -> Result<Engine> {
    let db = if let Some(path) = db_path(path) {
        let mut databases = Vec::new();
        let mut count = 0usize;
        for p in std::env::split_paths(&path) {
            if databases.len() >= 16 || count >= 250_000 {
                return Err(Error::Limit("database list"));
            }
            let db = Database::load(&p)?;
            count = count
                .checked_add(db.rule_count())
                .filter(|&n| n <= 250_000)
                .ok_or(Error::Limit("merged database rules"))?;
            databases.push(db);
        }
        if databases.is_empty() {
            return Err(Error::Database("no magic files loaded".into()));
        }
        Database::merge(databases)?
    } else {
        let mut databases = Vec::new();
        if let Some(home) = std::env::var_os("HOME") {
            let home = PathBuf::from(home);
            for p in [
                home.join(".magic.mgc"),
                home.join(".magic"),
                home.join(".magic/magic.mgc"),
            ] {
                if p.is_file() {
                    databases.push(Database::load(&p)?);
                    break;
                }
            }
        }
        databases.push(Database::builtin()?);
        Database::merge(databases)?
    };
    let mut engine = Engine::new(Arc::new(db));
    engine.flags = flags;
    engine.profile = profile;
    engine.limits = limits.clone();
    engine = engine.with_time_context(crate::TimeContext::from_environment()?);
    Ok(engine)
}
fn operand(engine: &Engine, name: &OsStr, width: usize, p: &Presentation) -> Result<bool> {
    let stdin = name == OsStr::new("-");
    let label = if stdin {
        OsStr::new("/dev/stdin")
    } else {
        name
    };
    let raw_name = bytes(label);
    let result = if stdin {
        engine.classify_reader(&mut io::stdin().lock())
    } else {
        engine.classify_path(Path::new(name))
    };
    let mut line = Vec::new();
    if p.brief == 0 {
        line.extend(if engine.flags.has(Flags::RAW) {
            raw_name
        } else {
            crate::format::escape(&raw_name)
        });
        if p.nulls > 0 {
            line.push(0);
        }
        if p.nulls < 2 {
            line.extend_from_slice(&p.separator);
            if !p.no_pad {
                line.extend(std::iter::repeat_n(
                    b' ',
                    width.saturating_sub(display_width(name, engine.flags)),
                ));
            }
            line.push(b' ');
        }
    }
    let failed = match result {
        Ok(c) => {
            line.extend(c.output);
            false
        }
        Err(e) => {
            line.extend_from_slice(format!("ERROR: {e}").as_bytes());
            true
        }
    };
    line.push(if p.nulls > 1 { 0 } else { b'\n' });
    let mut out = io::stdout().lock();
    out.write_all(&line)?;
    if p.no_buffer {
        out.flush()?;
    }
    Ok(failed)
}
fn display_width(s: &OsStr, flags: Flags) -> usize {
    let b = bytes(s);
    if flags.has(Flags::RAW) {
        // file_mbswidth's C-locale mbrtowc failure branch charges four
        // columns for high bytes even when the filename is printed raw.
        b.iter().map(|&c| if c >= 128 { 4 } else { 1 }).sum()
    } else {
        crate::format::escape(&b).len()
    }
}
fn file_list(engine: &Engine, path: &OsStr, p: &Presentation) -> Result<bool> {
    fn read_list(reader: &mut impl io::BufRead, engine: &Engine, p: &Presentation) -> Result<bool> {
        let mut paths = Vec::new();
        let mut total = 0usize;
        let mut failed = false;
        loop {
            let mut line = Vec::new();
            let n = reader.take(65537).read_until(b'\n', &mut line)?;
            if n == 0 {
                break;
            }
            if line.len() > 65536 {
                return Err(Error::Limit("file-list line"));
            }
            total += line.len();
            if total > 128 * 1024 * 1024 {
                return Err(Error::Limit("file list"));
            }
            if line.last() == Some(&b'\n') {
                line.pop();
            }
            let name = os_bytes(line);
            if p.no_buffer {
                failed |= operand(engine, &name, display_width(&name, engine.flags), p)?;
            } else {
                paths.push(name);
                if paths.len() > 1_000_000 {
                    return Err(Error::Limit("file-list entries"));
                }
            }
        }
        if !p.no_buffer {
            let width = paths
                .iter()
                .map(|x| display_width(x, engine.flags))
                .max()
                .unwrap_or(0);
            for name in paths {
                failed |= operand(engine, &name, width, p)?;
            }
        }
        Ok(failed)
    }
    if path == OsStr::new("-") {
        read_list(&mut io::stdin().lock(), engine, p)
    } else {
        read_list(
            &mut io::BufReader::new(std::fs::File::open(path)?),
            engine,
            p,
        )
    }
}

fn help() {
    println!("Additional options: --profile=pinned-5.48|corrected-v1; --capabilities");
    println!(
        "Usage: rfile [options] FILE...\n       rfile -C [-m MAGIC]\nIndependent Rust file classifier, pinned magic v21.\n\n  -b, --brief                 omit filename\n  -m, --magic-file LIST       colon-separated magic sources/databases\n  -i, --mime                  output MIME type and encoding\n      --mime-type             output MIME type\n      --mime-encoding         output encoding\n      --extension             output extensions\n      --apple                 output Apple creator/type\n  -k, --keep-going            continue matching\n  -e, --exclude TEST          skip a detector\n  -f, --files-from FILE       read newline-delimited filenames (- for stdin)\n  -F, --separator STRING      filename separator\n  -N, --no-pad                disable filename padding\n  -n, --no-buffer             flush each result\n  -0, --print0                NUL after filename; twice for NUL records\n  -L, --dereference           follow symbolic links\n  -h, --no-dereference        inspect symbolic links\n  -s, --special-files         read devices\n  -p, --preserve-date         restore access/modification times\n  -z, --uncompress            inspect gzip/zlib contents (bounded)\n  -Z, --uncompress-noreport   inspect gzip/zlib; omit wrapper\n  -P, --parameter NAME=N      set input/recursion/regex/work limits\n  -E                          make filesystem failures errors\n  -r, --raw                   preserve nonprintable output\n  -d, --debug                 diagnostic evaluation trace\n  -c, --checking-printout     parse/check source and list groups\n  -C, --compile               compile source to v21 .mgc\n  -l, --list                  list rule strengths\n  -S, --no-sandbox            accepted; kernel sandbox not implemented\n  -v, --version               engine, rule identity and features\n      --help                  show this help"
    );
}
fn long_short(name: &str) -> Option<char> {
    Some(match name {
        "brief" => 'b',
        "magic-file" => 'm',
        "mime" => 'i',
        "keep-going" => 'k',
        "exclude" => 'e',
        "files-from" => 'f',
        "separator" => 'F',
        "no-pad" => 'N',
        "no-buffer" => 'n',
        "print0" => '0',
        "dereference" => 'L',
        "no-dereference" => 'h',
        "special-files" => 's',
        "preserve-date" => 'p',
        "uncompress" => 'z',
        "uncompress-noreport" => 'Z',
        "parameter" => 'P',
        "raw" => 'r',
        "debug" => 'd',
        "checking-printout" => 'c',
        "compile" => 'C',
        "list" => 'l',
        "no-sandbox" => 'S',
        "version" => 'v',
        _ => return None,
    })
}
fn execute() -> Result<i32> {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    let mut p = Presentation {
        separator: b":".to_vec(),
        ..Presentation::default()
    };
    let mut flags = Flags::default();
    if std::env::var_os("POSIXLY_CORRECT").is_some() {
        flags.0 |= Flags::SYMLINK;
    }
    let mut limits = Limits::default();
    let mut profile = SemanticProfile::default();
    let mut magic_path = None;
    let mut loaded: Option<Engine> = None;
    let mut action = None;
    let mut names = Vec::new();
    let mut i = 0;
    let mut end = false;
    let mut had_lists = false;
    let mut failed = false;
    while i < args.len() {
        let arg = &args[i];
        i += 1;
        let text = arg.to_string_lossy();
        if end || text == "-" || !text.starts_with('-') {
            names.push(arg.clone());
            if std::env::var_os("POSIXLY_CORRECT").is_some() {
                end = true;
            }
            continue;
        }
        if text == "--" {
            end = true;
            continue;
        }
        let mut options: Vec<(char, Option<OsString>)> = Vec::new();
        if let Some(long) = text.strip_prefix("--") {
            let (name, value) = long
                .split_once('=')
                .map_or((long, None), |(a, b)| (a, Some(OsString::from(b))));
            match name {
                "profile" => {
                    let selected = if let Some(v) = value {
                        v
                    } else {
                        let v = args
                            .get(i)
                            .ok_or_else(|| {
                                Error::Unsupported("--profile requires an argument".into())
                            })?
                            .clone();
                        i += 1;
                        v
                    };
                    profile = SemanticProfile::parse(&selected.to_string_lossy())?;
                    continue;
                }
                "capabilities" => {
                    if value.is_some() {
                        return Err(Error::Unsupported(
                            "--capabilities does not take an argument".into(),
                        ));
                    }
                    let mut capabilities = Capabilities::current(profile);
                    capabilities.time_zone =
                        crate::TimeContext::from_environment()?.source().to_owned();
                    println!("{}", capabilities.json());
                    return Ok(0);
                }
                "help" => {
                    help();
                    return Ok(0);
                }
                "mime-type" => {
                    flags.0 |= Flags::MIME_TYPE;
                    continue;
                }
                "mime-encoding" => {
                    flags.0 |= Flags::MIME_ENCODING;
                    continue;
                }
                "apple" => {
                    flags.0 |= Flags::APPLE;
                    continue;
                }
                "extension" => {
                    flags.0 |= Flags::EXTENSION;
                    continue;
                }
                "exclude-quiet" => {
                    options.push(('q', value));
                }
                _ => options.push((
                    long_short(name)
                        .ok_or_else(|| Error::Unsupported(format!("option --{name}")))?,
                    value,
                )),
            }
        } else {
            let chars: Vec<char> = text[1..].chars().collect();
            let mut j = 0;
            while j < chars.len() {
                let c = chars[j];
                j += 1;
                if "mefFP".contains(c) && j < chars.len() {
                    options.push((
                        c,
                        Some(OsString::from(chars[j..].iter().collect::<String>())),
                    ));
                    break;
                }
                options.push((c, None));
                if "mefFP".contains(c) {
                    break;
                }
            }
        }
        for (c, value) in options {
            let value = if "mefFPq".contains(c) {
                if let Some(value) = value {
                    Some(value)
                } else {
                    let a = args
                        .get(i)
                        .ok_or_else(|| {
                            Error::Unsupported(format!("option -{c} requires an argument"))
                        })?
                        .clone();
                    i += 1;
                    Some(a)
                }
            } else {
                if value.is_some() {
                    return Err(Error::Unsupported(format!(
                        "option -{c} does not take an argument"
                    )));
                }
                None
            };
            match c {
                'b' => p.brief += 1,
                '0' => p.nulls += 1,
                'n' => p.no_buffer = true,
                'N' => p.no_pad = true,
                'F' => p.separator = bytes(value.as_ref().expect("argument")),
                'm' => magic_path = value,
                'i' => flags.0 |= Flags::MIME,
                'k' => flags.0 |= Flags::CONTINUE,
                'E' => flags.0 |= Flags::ERROR,
                'L' => flags.0 |= Flags::SYMLINK,
                'h' => flags.0 &= !Flags::SYMLINK,
                's' => flags.0 |= Flags::DEVICES,
                'p' => flags.0 |= Flags::PRESERVE_ATIME,
                'r' => flags.0 |= Flags::RAW,
                'z' => flags.0 |= Flags::COMPRESS,
                'Z' => flags.0 |= Flags::COMPRESS | Flags::TRANSPARENT,
                'd' => flags.0 |= Flags::DEBUG | Flags::CHECK,
                'S' => {}
                'c' | 'C' | 'l' => action = Some(c),
                'e' | 'q' => {
                    let v = value.as_ref().expect("argument").to_string_lossy();
                    if let Some(f) = Flags::exclude(&v) {
                        flags.0 |= f;
                    } else if c != 'q' {
                        return Err(Error::Unsupported(format!("exclude test {v}")));
                    }
                }
                'P' => {
                    let v = value.as_ref().expect("argument").to_string_lossy();
                    let (name, number) = v
                        .split_once('=')
                        .ok_or_else(|| Error::Unsupported(format!("parameter {v}")))?;
                    let n = number
                        .parse::<usize>()
                        .map_err(|_| Error::Unsupported(format!("parameter {v}")))?;
                    limits.set(name, n)?;
                }
                'f' => {
                    if action.is_some() {
                        return Err(Error::Unsupported("file list with database action".into()));
                    }
                    if loaded.is_none() {
                        loaded = Some(load(&magic_path, flags, &limits, profile)?);
                    }
                    let engine = loaded.as_mut().expect("loaded");
                    engine.limits = limits.clone();
                    failed |= file_list(engine, value.as_ref().expect("argument"), &p)?;
                    had_lists = true;
                }
                'v' => {
                    println!(
                        "rfile-{}\nmagic v{} from {}\nupstream {}\nfeatures: safe-rust regex miniz_oxide; no kernel sandbox",
                        env!("CARGO_PKG_VERSION"),
                        crate::FORMAT_VERSION,
                        db_path(&magic_path).map_or("embedded pinned database".into(), |p| p
                            .to_string_lossy()
                            .into_owned()),
                        crate::UPSTREAM_COMMIT
                    );
                    return Ok(0);
                }
                _ => return Err(Error::Unsupported(format!("option -{c}"))),
            }
        }
    }
    if let Some(action) = action {
        let path = db_path(&magic_path)
            .ok_or_else(|| Error::Unsupported("database action requires -m source".into()))?;
        let mut databases = Vec::new();
        for path in std::env::split_paths(&path) {
            let db = if action == 'C' || action == 'c' {
                Database::load_source(&path)?
            } else {
                Database::load(&path)?
            };
            if action == 'C' {
                let mut output = path
                    .file_name()
                    .unwrap_or(OsStr::new("magic"))
                    .to_os_string();
                output.push(".mgc");
                std::fs::write(output, db.compile()?)?;
            } else {
                databases.push(db);
            }
        }
        if action != 'C' {
            let text = Database::merge(databases)?.list();
            if action == 'c' {
                eprint!("{text}");
            } else {
                print!("{text}");
            }
        }
        return Ok(0);
    }
    if names.is_empty() && !had_lists {
        eprintln!("Usage: rfile [options] FILE...\nTry 'rfile --help'.");
        return Ok(1);
    }
    if loaded.is_none() {
        loaded = Some(load(&magic_path, flags, &limits, profile)?);
    }
    let engine = loaded.as_mut().expect("loaded");
    engine.limits = limits;
    if p.brief == 2 {
        p.brief = usize::from(names.len() <= 1);
    }
    let width = names
        .iter()
        .map(|s| display_width(s, engine.flags))
        .max()
        .unwrap_or(0);
    for name in names {
        failed |= operand(engine, &name, width, &p)?;
    }
    io::stdout().flush()?;
    Ok(i32::from(failed))
}
pub fn run() -> i32 {
    match execute() {
        Ok(status) => status,
        Err(e) => {
            eprintln!("rfile: {e}");
            1
        }
    }
}
