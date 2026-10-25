// SPDX-License-Identifier: GPL-3.0-only
//! Platform services only. Detection and untrusted format parsing stay in the
//! safe core. No libmagic or codec FFI is permitted in this crate.
#![deny(unsafe_op_in_unsafe_fn)]
#![warn(clippy::undocumented_unsafe_blocks)]
#[cfg(all(target_os = "linux", target_env = "gnu"))]
mod locale;
#[cfg(all(target_os = "linux", target_env = "gnu"))]
pub use locale::Locale;

#[cfg(not(all(target_os = "linux", target_env = "gnu")))]
#[derive(Debug)]
pub struct Locale {
    _private: (),
}
#[cfg(not(all(target_os = "linux", target_env = "gnu")))]
impl Locale {
    pub fn new(_: &str) -> std::io::Result<Self> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "platform locales require a certified Linux/glibc adapter",
        ))
    }
    pub fn name(&self) -> &str {
        "unsupported"
    }
    pub fn byte_printable(&self, b: u8) -> bool {
        (32..=126).contains(&b)
    }
    pub fn filename(&self, _: &[u8], _: bool, _: usize) -> std::io::Result<Vec<u8>> {
        Err(std::io::ErrorKind::Unsupported.into())
    }
    pub fn filename_width(&self, _: &[u8], _: bool) -> std::io::Result<usize> {
        Err(std::io::ErrorKind::Unsupported.into())
    }
    pub fn result(&self, _: &[u8], _: usize) -> std::io::Result<Vec<u8>> {
        Err(std::io::ErrorKind::Unsupported.into())
    }
}

#[cfg(all(
    feature = "sandbox",
    target_os = "linux",
    target_env = "gnu",
    target_arch = "x86_64"
))]
pub mod sandbox;

/// Open a regular database source without waiting for a FIFO writer on Linux.
/// Opening arbitrary device sources is outside the database contract.
pub fn open_regular_database(path: &std::path::Path) -> std::io::Result<std::fs::File> {
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NONBLOCK | libc::O_NOCTTY);
    }
    #[cfg(not(all(target_os = "linux", target_env = "gnu")))]
    if !std::fs::metadata(path)?.is_file() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "database must be a regular file",
        ));
    }
    let file = options.open(path)?;
    if !file.metadata()?.is_file() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "database must be a regular file",
        ));
    }
    Ok(file)
}
