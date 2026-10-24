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
