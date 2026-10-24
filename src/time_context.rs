// SPDX-License-Identifier: GPL-3.0-only
//! Immutable timezone configuration. Loading happens before inspection, never
//! by mutating TZ or calling process-global libc timezone/locale functions.
use crate::{Error, Result};
use jiff::{Timestamp, tz::TimeZone};
use std::{
    io::Read,
    path::{Component, Path, PathBuf},
};
const ZONE_LIMIT: usize = 1024 * 1024;
#[derive(Clone, Debug)]
pub struct TimeContext {
    zone: TimeZone,
    source: String,
    last_explicit_transition: i64,
}
impl Default for TimeContext {
    fn default() -> Self {
        Self {
            zone: TimeZone::UTC,
            source: "UTC".into(),
            last_explicit_transition: i64::MIN,
        }
    }
}
impl TimeContext {
    pub fn source(&self) -> &str {
        &self.source
    }
    pub fn posix(expression: &str) -> Result<Self> {
        if expression.len() > 4096 {
            return Err(Error::Limit("timezone expression"));
        }
        Ok(Self {
            zone: TimeZone::posix(expression)
                .map_err(|e| Error::Unsupported(format!("timezone: {e}")))?,
            source: expression.into(),
            last_explicit_transition: i64::MIN,
        })
    }
    pub fn from_tzif(name: &str, bytes: &[u8]) -> Result<Self> {
        if bytes.len() > ZONE_LIMIT {
            return Err(Error::Limit("timezone file"));
        }
        Ok(Self {
            zone: TimeZone::tzif(name, bytes)
                .map_err(|e| Error::Unsupported(format!("timezone: {e}")))?,
            source: name.into(),
            last_explicit_transition: last_transition(bytes).unwrap_or(i64::MAX),
        })
    }
    pub fn from_file(path: &Path) -> Result<Self> {
        let mut options = std::fs::OpenOptions::new();
        options.read(true);
        #[cfg(target_os = "linux")]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.custom_flags(0o4000); // O_NONBLOCK: reject a raced-in FIFO without hanging.
        }
        let file = options.open(path)?;
        if !file.metadata()?.is_file() {
            return Err(Error::Unsupported("timezone is not a regular file".into()));
        }
        let mut bytes = Vec::new();
        file.take((ZONE_LIMIT + 1) as u64).read_to_end(&mut bytes)?;
        Self::from_tzif(&path.to_string_lossy(), &bytes)
    }
    pub fn named(name: &str, root: &Path) -> Result<Self> {
        let relative = Path::new(name);
        if name.is_empty()
            || name.len() > 4096
            || relative
                .components()
                .any(|c| !matches!(c, Component::Normal(_)))
        {
            return Err(Error::Unsupported("invalid named timezone path".into()));
        }
        let root = root.canonicalize()?;
        let path = root.join(relative).canonicalize()?;
        if !path.starts_with(&root) {
            return Err(Error::Unsupported("timezone escapes database root".into()));
        }
        let mut context = Self::from_file(&path)?;
        context.source = name.into();
        Ok(context)
    }
    /// Snapshot CLI configuration once. Explicit TZ file paths are separate
    /// configuration inputs; named zones cannot traverse out of `root`.
    pub fn from_environment() -> Result<Self> {
        let root = PathBuf::from("/usr/share/zoneinfo");
        match std::env::var_os("TZ") {
            None => Self::from_file(Path::new("/etc/localtime")),
            Some(raw) => {
                let text = raw
                    .to_str()
                    .ok_or_else(|| Error::Unsupported("non-UTF8 TZ".into()))?;
                if text.is_empty() {
                    return Ok(Self::default());
                }
                let name = text.strip_prefix(':').unwrap_or(text);
                if Path::new(name).is_absolute() {
                    return Self::from_file(Path::new(name));
                }
                if let Ok(context) = Self::posix(name) {
                    return Ok(context);
                }
                Self::named(name, &root)
            }
        }
    }
    pub(crate) fn legacy_windows_seconds(&self, wall: i64) -> Option<i64> {
        // The pin passes tm_isdst=0 to mktime even for its nominally UTC
        // FILETIME family. Preserve the host's standard-time interpretation.
        let ts = Timestamp::from_second(wall).ok()?;
        let info = self.zone.to_offset_info(ts);
        let offset = if info.dst().is_std() {
            info.offset()
        } else {
            self.zone
                .preceding(ts)
                .take(128)
                .find(|t| t.dst().is_std())
                .map(|t| t.offset())
                .or_else(|| {
                    self.zone
                        .following(ts)
                        .take(128)
                        .find(|t| t.dst().is_std())
                        .map(|t| t.offset())
                })?
        };
        wall.checked_sub(offset.seconds() as i64)
    }
    pub(crate) fn local_seconds(&self, seconds: i64) -> Option<i64> {
        let ts = Timestamp::from_second(seconds).ok().or_else(|| {
            // Jiff reserves one day at its civil range edges. Beyond the last
            // explicit TZif transition, footer rules repeat on the Gregorian
            // 400-year cycle (or the final fixed offset remains in force).
            let shifted = seconds.checked_sub(146097 * 86400)?;
            (seconds > Timestamp::MAX.as_second() && shifted > self.last_explicit_transition)
                .then(|| Timestamp::from_second(shifted).ok())
                .flatten()
        })?;
        seconds.checked_add(self.zone.to_offset(ts).seconds() as i64)
    }
}

// Read only transition-table bounds after the dependency validated the TZif.
// None deliberately disables the extended-range optimization.
fn last_transition(bytes: &[u8]) -> Option<i64> {
    fn u32at(b: &[u8], p: usize) -> Option<usize> {
        Some(u32::from_be_bytes(b.get(p..p.checked_add(4)?)?.try_into().ok()?) as usize)
    }
    let mut header = 0usize;
    let wide = *bytes.get(4)? != 0;
    if wide {
        let size = u32at(bytes, 32)?
            .checked_mul(5)?
            .checked_add(u32at(bytes, 36)?.checked_mul(6)?)?
            .checked_add(u32at(bytes, 40)?)?
            .checked_add(u32at(bytes, 28)?.checked_mul(8)?)?
            .checked_add(u32at(bytes, 24)?)?
            .checked_add(u32at(bytes, 20)?)?;
        header = 44usize.checked_add(size)?;
    }
    let count = u32at(bytes, header.checked_add(32)?)?;
    if count == 0 {
        return Some(i64::MIN);
    }
    let width = if wide { 8 } else { 4 };
    let p = header
        .checked_add(44)?
        .checked_add((count - 1).checked_mul(width)?)?;
    if wide {
        Some(i64::from_be_bytes(
            bytes.get(p..p.checked_add(8)?)?.try_into().ok()?,
        ))
    } else {
        Some(i32::from_be_bytes(bytes.get(p..p.checked_add(4)?)?.try_into().ok()?) as i64)
    }
}
