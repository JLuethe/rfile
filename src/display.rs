// SPDX-License-Identifier: GPL-3.0-only
//! Immutable display policy. Rust callers default to portable C byte display;
//! platform locales are explicit and never change process-global locale state.
use crate::{Error, Result};
use std::sync::Arc;
#[derive(Clone, Debug, Default)]
pub enum DisplayContext {
    #[default]
    C,
    Platform(Arc<rfile_platform::Locale>),
}
fn mapped(error: std::io::Error) -> Error {
    match error.kind() {
        std::io::ErrorKind::FileTooLarge => Error::Limit("output"),
        std::io::ErrorKind::OutOfMemory => Error::Limit("display allocation"),
        _ => Error::Io(error),
    }
}
fn prefix(bytes: &[u8]) -> &[u8] {
    &bytes[..bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len())]
}
fn push(out: &mut Vec<u8>, part: &[u8], limit: usize) -> Result<()> {
    if out.len().checked_add(part.len()).is_none_or(|n| n > limit) {
        return Err(Error::Limit("output"));
    }
    out.try_reserve(part.len())
        .map_err(|_| Error::Limit("display allocation"))?;
    out.extend_from_slice(part);
    Ok(())
}
impl DisplayContext {
    pub fn locale(name: &str) -> Result<Self> {
        if name.len() > 1024 {
            return Err(Error::Limit("locale name"));
        }
        if name == "C" || name == "POSIX" {
            return Ok(Self::C);
        }
        if name.is_empty() {
            return Err(Error::Unsupported(
                "display locale requires an explicit nonempty name".into(),
            ));
        }
        rfile_platform::Locale::new(name)
            .map(|locale| Self::Platform(Arc::new(locale)))
            .map_err(|e| Error::Unsupported(format!("display locale {name:?}: {e}")))
    }
    pub fn from_environment() -> Result<Self> {
        for key in ["LC_ALL", "LC_CTYPE", "LANG"] {
            if let Some(name) = std::env::var_os(key).filter(|v| !v.is_empty()) {
                let name = name
                    .to_str()
                    .ok_or_else(|| Error::Unsupported(format!("non-UTF8 {key} locale name")))?;
                return Self::locale(name);
            }
        }
        Ok(Self::C)
    }
    pub fn name(&self) -> &str {
        match self {
            Self::C => "C",
            Self::Platform(locale) => locale.name(),
        }
    }
    pub(crate) fn byte_printable(&self, b: u8) -> bool {
        match self {
            Self::C => (32..=126).contains(&b),
            Self::Platform(locale) => locale.byte_printable(b),
        }
    }
    pub(crate) fn result(&self, bytes: &[u8], raw: bool, limit: usize) -> Result<Vec<u8>> {
        let bytes = prefix(bytes);
        let capacity = bytes
            .len()
            .checked_mul(if raw { 1 } else { 4 })
            .ok_or(Error::Limit("display allocation"))?
            .min(limit);
        if raw {
            let mut out = Vec::new();
            out.try_reserve_exact(capacity)
                .map_err(|_| Error::Limit("display allocation"))?;
            push(&mut out, bytes, limit)?;
            return Ok(out);
        }
        match self {
            Self::C => {
                let mut out = Vec::new();
                out.try_reserve_exact(capacity)
                    .map_err(|_| Error::Limit("display allocation"))?;
                for &b in bytes {
                    if self.byte_printable(b) {
                        push(&mut out, &[b], limit)?;
                    } else {
                        push(
                            &mut out,
                            &[
                                b'\\',
                                b'0' + (b >> 6),
                                b'0' + ((b >> 3) & 7),
                                b'0' + (b & 7),
                            ],
                            limit,
                        )?;
                    }
                }
                Ok(out)
            }
            Self::Platform(locale) => locale.result(bytes, limit).map_err(mapped),
        }
    }
    pub(crate) fn filename(&self, bytes: &[u8], raw: bool) -> Result<Vec<u8>> {
        let limit = bytes
            .len()
            .checked_mul(4)
            .ok_or(Error::Limit("filename display"))?;
        match self {
            Self::C => self.result(bytes, raw, limit),
            Self::Platform(locale) => locale.filename(bytes, raw, limit).map_err(mapped),
        }
    }
    pub(crate) fn filename_width(&self, bytes: &[u8], raw: bool) -> Result<usize> {
        match self {
            Self::Platform(locale) => locale.filename_width(bytes, raw).map_err(mapped),
            Self::C => {
                let mut width = 0usize;
                for &b in prefix(bytes) {
                    let n = if b >= 128 || (!raw && !self.byte_printable(b)) {
                        4
                    } else {
                        1
                    };
                    width = width.checked_add(n).ok_or(Error::Limit("filename width"))?;
                }
                Ok(width)
            }
        }
    }
}
