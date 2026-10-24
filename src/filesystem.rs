/*
 * Copyright (c) Christos Zoulas 2003.
 * All Rights Reserved.
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
// Independent Rust implementation; upstream behavior and retained data notices credited in NOTICE.
use crate::{Classification, Engine, Error, Flags, Result};
use std::{
    fs::{self, File},
    io::{Read, Seek, SeekFrom},
    path::Path,
};

pub(crate) fn error_text(e: &std::io::Error) -> String {
    let s = e.to_string();
    s.rsplit_once(" (os error ")
        .map_or(s.clone(), |(a, _)| a.to_owned())
}
fn path_bytes(path: &Path) -> Vec<u8> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        path.as_os_str().as_bytes().to_vec()
    }
    #[cfg(not(unix))]
    {
        path.to_string_lossy().as_bytes().to_vec()
    }
}
fn diagnostic(engine: &Engine, description: Vec<u8>) -> Classification {
    let mut c = engine.basic(
        description.clone(),
        "application/octet-stream",
        "binary",
        false,
    );
    c.output = if engine.flags.has(Flags::RAW) {
        description
    } else {
        crate::format::escape(&description)
    };
    c
}
fn metadata_result(engine: &Engine, description: Vec<u8>, kind: &str) -> Classification {
    engine.basic(description, &format!("inode/{kind}"), "binary", false)
}
pub(crate) fn classify_path(engine: &Engine, path: &Path) -> Result<Classification> {
    let metadata = if engine.flags.has(Flags::SYMLINK) {
        fs::metadata(path)
    } else {
        fs::symlink_metadata(path)
    };
    let meta = match metadata {
        Ok(m) => m,
        Err(e) => {
            if engine.flags.has(Flags::ERROR) {
                return Err(Error::Io(std::io::Error::new(
                    e.kind(),
                    format!(
                        "cannot stat `{}' ({})",
                        path.to_string_lossy(),
                        error_text(&e)
                    ),
                )));
            }
            let mut text = b"cannot open `".to_vec();
            text.extend(path_bytes(path));
            text.extend_from_slice(format!("' ({})", error_text(&e)).as_bytes());
            return Ok(diagnostic(engine, text));
        }
    };
    let annotation_only =
        engine.flags.has(Flags::APPLE | Flags::EXTENSION) && !engine.flags.has(Flags::MIME);
    if meta.is_dir() {
        if annotation_only {
            return Err(Error::Io(std::io::Error::other(format!(
                "cannot read `{}' (Is a directory)",
                path.to_string_lossy()
            ))));
        }
        return Ok(metadata_result(engine, b"directory".to_vec(), "directory"));
    }
    if meta.file_type().is_symlink() {
        let target = fs::read_link(path)?;
        let target_metadata = fs::metadata(path);
        if let Err(e) = target_metadata {
            let mut desc = b"broken symbolic link to ".to_vec();
            desc.extend(path_bytes(&target));
            if engine.flags.has(Flags::ERROR) && !engine.flags.has(Flags::MIME) {
                return Err(Error::Io(std::io::Error::other(format!(
                    "{} ({})",
                    String::from_utf8_lossy(&desc),
                    error_text(&e)
                ))));
            }
            let mut c = diagnostic(engine, desc);
            if engine.flags.has(Flags::MIME_TYPE) {
                c.output = b"inode/symlink".to_vec();
                c.mime_type = "inode/symlink".into();
            } else if engine.flags.has(Flags::MIME_ENCODING) {
                c.output.clear();
            }
            return Ok(c);
        }
        if annotation_only {
            if !target_metadata.as_ref().is_ok_and(|m| m.is_file()) {
                return Err(Error::Unsupported(
                    "annotation content inspection of special symlink target".into(),
                ));
            }
            // The C cookie falls through to reading a nonbroken link in annotation-only mode.
        } else {
            let mut desc = b"symbolic link to ".to_vec();
            desc.extend(path_bytes(&target));
            return Ok(metadata_result(engine, desc, "symlink"));
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::{FileTypeExt, MetadataExt};
        let t = meta.file_type();
        if t.is_socket() {
            if annotation_only {
                let mut desc = Vec::new();
                let mode = meta.mode();
                if mode & 0o200 != 0 {
                    desc.extend_from_slice(b"writable, ");
                }
                if mode & 0o100 != 0 {
                    desc.extend_from_slice(b"executable, ");
                }
                desc.extend_from_slice(b"no read permission");
                return Ok(diagnostic(engine, desc));
            }
            return Ok(metadata_result(engine, b"socket".to_vec(), "socket"));
        }
        if !engine.flags.has(Flags::DEVICES) {
            if t.is_fifo() {
                if annotation_only {
                    return Err(Error::Unsupported(
                        "annotation content inspection of FIFO".into(),
                    ));
                }
                return Ok(metadata_result(
                    engine,
                    b"fifo (named pipe)".to_vec(),
                    "fifo",
                ));
            }
            if t.is_char_device() || t.is_block_device() {
                let d = meta.rdev();
                let major = ((d >> 8) & 0xfff) | ((d >> 32) & 0xfffff000);
                let minor = (d & 0xff) | ((d >> 12) & 0xffffff00);
                return Ok(metadata_result(
                    engine,
                    format!(
                        "{} special ({major}/{minor})",
                        if t.is_char_device() {
                            "character"
                        } else {
                            "block"
                        }
                    )
                    .into_bytes(),
                    if t.is_char_device() {
                        "chardevice"
                    } else {
                        "blockdevice"
                    },
                ));
            }
        }
    }
    if meta.is_file() && meta.len() == 0 && !engine.flags.has(Flags::DEVICES) {
        return if annotation_only {
            engine.classify(&[])
        } else {
            Ok(engine.basic(b"empty".to_vec(), "inode/x-empty", "binary", false))
        };
    }
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(0x800);
    }
    let mut file = match options.open(path) {
        Ok(f) => f,
        Err(e) => {
            if engine.flags.has(Flags::ERROR) {
                return Err(e.into());
            }
            return Ok(engine.basic(
                if meta.is_file() {
                    b"regular file, no read permission".to_vec()
                } else {
                    b"no read permission".to_vec()
                },
                "application/octet-stream",
                "binary",
                false,
            ));
        }
    };
    let result = read_file(engine, &mut file, false);
    if engine.flags.has(Flags::PRESERVE_ATIME) {
        if let (Ok(a), Ok(m)) = (meta.accessed(), meta.modified()) {
            let _ = file.set_times(std::fs::FileTimes::new().set_accessed(a).set_modified(m));
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if !engine.flags.annotation() {
            if let Ok(mut c) = result {
                let mode = meta.permissions().mode();
                let mut prefix = Vec::new();
                for (bit, name) in [
                    (0o4000, b"setuid ".as_slice()),
                    (0o2000, b"setgid ".as_slice()),
                    (0o1000, b"sticky ".as_slice()),
                ] {
                    if mode & bit != 0 {
                        prefix.extend_from_slice(name);
                    }
                }
                if !prefix.is_empty() {
                    c.description =
                        format!("{}{}", String::from_utf8_lossy(&prefix), c.description);
                    prefix.extend(c.output);
                    c.output = prefix;
                }
                return Ok(c);
            }
        }
    }
    result
}
pub(crate) fn classify_file(engine: &Engine, file: &mut File) -> Result<Classification> {
    let position = file.stream_position().ok();
    let result = read_file(engine, file, true);
    if let Some(p) = position {
        file.seek(SeekFrom::Start(p))?;
    }
    result
}
fn read_file(engine: &Engine, file: &mut File, borrowed: bool) -> Result<Classification> {
    let meta = file.metadata()?;
    if !borrowed && !engine.flags.has(Flags::DEVICES) && !meta.is_file() {
        return Err(Error::Unsupported(
            "path changed to a special file during open".into(),
        ));
    }
    let mut prefix = Vec::new();
    file.take(engine.limits.bytes as u64)
        .read_to_end(&mut prefix)?;
    let mut tail = Vec::new();
    if meta.is_file() && meta.len() > prefix.len() as u64 && !prefix.is_empty() {
        let n = prefix.len().min(meta.len().min(usize::MAX as u64) as usize);
        file.seek(SeekFrom::End(-(n as i64)))?;
        file.take(n as u64).read_to_end(&mut tail)?;
    }
    let executable = {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            meta.permissions().mode() & 0o111 != 0
        }
        #[cfg(not(unix))]
        {
            false
        }
    };
    let elf = if !engine
        .flags
        .has(Flags::NO_ELF | Flags::APPLE | Flags::EXTENSION)
    {
        crate::elf::analyze(
            file,
            &prefix,
            &engine.limits,
            executable,
            engine.flags.has(Flags::MIME),
        )?
    } else {
        None
    };
    let executable = elf.as_ref().map_or(executable, |e| e.executable);
    let mut result = engine.classify_input(
        &prefix,
        if tail.is_empty() { None } else { Some(&tail) },
        executable,
        elf.as_ref().map(|info| info.suffix.as_slice()),
    )?;
    result.truncated = meta.len() > prefix.len() as u64;
    Ok(result)
}
