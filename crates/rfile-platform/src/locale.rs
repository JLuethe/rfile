// SPDX-License-Identifier: GPL-3.0-only
//! Linux/glibc ABI boundary. Locale handles are immutable and owned; guards
//! borrow them, remain on one thread, and restore the previous locale on unwind.
use std::{
    ffi::{CStr, CString},
    io,
    marker::PhantomData,
    ptr::NonNull,
    rc::Rc,
};
unsafe extern "C" {
    // glibc wchar_t is i32, wint_t is unsigned int; mbstate_t comes from libc.
    fn mbrtowc(
        wc: *mut libc::wchar_t,
        source: *const libc::c_char,
        n: libc::size_t,
        state: *mut libc::mbstate_t,
    ) -> libc::size_t;
    fn iswprint_l(wc: libc::c_uint, locale: libc::locale_t) -> libc::c_int;
    fn wcwidth(wc: libc::wchar_t) -> libc::c_int;
    fn isprint_l(c: libc::c_int, locale: libc::locale_t) -> libc::c_int;
}
#[derive(Debug)]
pub struct Locale {
    handle: NonNull<libc::c_void>,
    name: String,
}
// SAFETY: newlocale acquires an immutable object reference; methods never mutate/free it.
// A Guard borrows &Locale, so final Drop cannot free a locale in active use.
// Each thread has its own uselocale binding and conversion-state buffer.
unsafe impl Send for Locale {}
// SAFETY: the same owned locale may be selected independently on many threads.
unsafe impl Sync for Locale {}
impl Drop for Locale {
    fn drop(&mut self) {
        // SAFETY: this handle comes only from successful newlocale(base=NULL),
        // is released once for this acquisition, never LC_GLOBAL_LOCALE, and has no live guard.
        unsafe { libc::freelocale(self.handle.as_ptr()) };
    }
}
struct Guard<'a> {
    previous: libc::locale_t,
    _locale: &'a Locale,
    _thread: PhantomData<Rc<()>>,
}
impl Drop for Guard<'_> {
    fn drop(&mut self) {
        // SAFETY: previous is exactly the valid binding returned by uselocale;
        // this guard cannot cross threads and restores before its Locale can drop.
        unsafe { libc::uselocale(self.previous) };
    }
}
fn append(out: &mut Vec<u8>, bytes: &[u8], limit: usize) -> io::Result<()> {
    if out.len().checked_add(bytes.len()).is_none_or(|n| n > limit) {
        return Err(io::Error::new(
            io::ErrorKind::FileTooLarge,
            "display output limit",
        ));
    }
    out.try_reserve(bytes.len())
        .map_err(|_| io::Error::new(io::ErrorKind::OutOfMemory, "display allocation"))?;
    out.extend_from_slice(bytes);
    Ok(())
}
fn buffer(input: usize, limit: usize) -> io::Result<Vec<u8>> {
    let size = input
        .checked_mul(4)
        .ok_or_else(|| io::Error::new(io::ErrorKind::FileTooLarge, "display allocation bound"))?
        .min(limit);
    let mut out = Vec::new();
    out.try_reserve_exact(size)
        .map_err(|_| io::Error::new(io::ErrorKind::OutOfMemory, "display allocation"))?;
    Ok(out)
}
fn octal(out: &mut Vec<u8>, b: u8, limit: usize) -> io::Result<()> {
    append(
        out,
        &[
            b'\\',
            b'0' + (b >> 6),
            b'0' + ((b >> 3) & 7),
            b'0' + (b & 7),
        ],
        limit,
    )
}
fn prefix(bytes: &[u8]) -> &[u8] {
    &bytes[..bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len())]
}
fn zero_state() -> libc::mbstate_t {
    // SAFETY: glibc mbstate_t consists only of integer/character storage. The
    // all-zero representation is its documented initial conversion state.
    unsafe { std::mem::zeroed() }
}
impl Locale {
    pub fn new(name: &str) -> io::Result<Self> {
        if name.len() > 1024 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "locale name limit",
            ));
        }
        let encoded = CString::new(name)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "NUL in locale name"))?;
        // SAFETY: encoded is a valid NUL-terminated string for this call. NULL
        // base requests a fresh owned locale; only LC_CTYPE is configured.
        let handle = NonNull::new(unsafe {
            libc::newlocale(libc::LC_CTYPE_MASK, encoded.as_ptr(), std::ptr::null_mut())
        })
        .ok_or_else(io::Error::last_os_error)?;
        let locale = Self {
            handle,
            name: name.into(),
        };
        // SAFETY: CODESET is valid; the returned immutable NUL-terminated libc
        // string belongs to the still-live locale and is only read here.
        let charset =
            unsafe { CStr::from_ptr(libc::nl_langinfo_l(libc::CODESET, locale.handle.as_ptr())) }
                .to_bytes();
        if ![
            b"UTF-8".as_slice(),
            b"ANSI_X3.4-1968",
            b"US-ASCII",
            b"ASCII",
        ]
        .iter()
        .any(|s| charset.eq_ignore_ascii_case(s))
        {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "only ASCII/UTF-8 locale encodings are certified",
            ));
        }
        Ok(locale)
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    fn enter(&self) -> io::Result<Guard<'_>> {
        // SAFETY: the owned handle stays alive for the complete borrowing guard.
        let previous = unsafe { libc::uselocale(self.handle.as_ptr()) };
        if previous.is_null() {
            return Err(io::Error::last_os_error());
        }
        Ok(Guard {
            previous,
            _locale: self,
            _thread: PhantomData,
        })
    }
    pub fn byte_printable(&self, b: u8) -> bool {
        // SAFETY: ctype accepts every unsigned-char value; locale is valid/live.
        unsafe { isprint_l(b as libc::c_int, self.handle.as_ptr()) != 0 }
    }
    fn printable(&self, c: libc::wchar_t) -> bool {
        // SAFETY: c is a wide character produced by successful mbrtowc under
        // this locale; the unsigned wint_t conversion preserves its value.
        unsafe { iswprint_l(c as libc::c_uint, self.handle.as_ptr()) != 0 }
    }
    fn next(
        bytes: &[u8],
        state: &mut libc::mbstate_t,
    ) -> io::Result<Option<(usize, libc::wchar_t)>> {
        let mut c = 0;
        // SAFETY: the source slice is live and n is exactly its readable bound;
        // c/state are aligned writable objects with the documented ABI/layout.
        let n = unsafe { mbrtowc(&mut c, bytes.as_ptr().cast(), bytes.len(), state) };
        if n == usize::MAX || n == usize::MAX - 1 {
            return Ok(None);
        }
        if n == 0 || n > bytes.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid multibyte conversion count",
            ));
        }
        Ok(Some((n, c)))
    }
    pub fn filename(&self, bytes: &[u8], raw: bool, limit: usize) -> io::Result<Vec<u8>> {
        if bytes.len() > 16 * 1024 * 1024 {
            return Err(io::Error::new(
                io::ErrorKind::FileTooLarge,
                "display input limit",
            ));
        }
        let bytes = prefix(bytes);
        let mut out = buffer(bytes.len(), limit)?;
        if raw {
            append(&mut out, bytes, limit)?;
            return Ok(out);
        }
        let _guard = self.enter()?;
        let mut state = zero_state();
        let mut at = 0;
        while at < bytes.len() {
            match Self::next(&bytes[at..], &mut state)? {
                None => {
                    octal(&mut out, bytes[at], limit)?;
                    at += 1;
                    state = zero_state();
                }
                Some((n, c)) => {
                    if self.printable(c) {
                        append(&mut out, &bytes[at..at + n], limit)?;
                    } else {
                        // Pinned fname_print emits only the low byte here.
                        octal(&mut out, c as u8, limit)?;
                    }
                    at += n;
                }
            }
        }
        Ok(out)
    }
    pub fn filename_width(&self, bytes: &[u8], raw: bool) -> io::Result<usize> {
        if bytes.len() > 16 * 1024 * 1024 {
            return Err(io::Error::new(
                io::ErrorKind::FileTooLarge,
                "display input limit",
            ));
        }
        let bytes = prefix(bytes);
        let _guard = self.enter()?;
        let mut state = zero_state();
        let mut at = 0;
        let mut width = 0usize;
        while at < bytes.len() {
            let (n, columns) = match Self::next(&bytes[at..], &mut state)? {
                None => {
                    state = zero_state();
                    (1, 4)
                }
                Some((n, c)) => {
                    // SAFETY: c came from mbrtowc and the same locale is selected
                    // by the non-Send guard for the entire wcwidth operation.
                    let w = unsafe { wcwidth(c) };
                    (
                        n,
                        if raw || self.printable(c) {
                            w.max(1) as usize
                        } else {
                            4
                        },
                    )
                }
            };
            width = width
                .checked_add(columns)
                .ok_or_else(|| io::Error::new(io::ErrorKind::FileTooLarge, "filename width"))?;
            at += n;
        }
        Ok(width)
    }
    pub fn result(&self, bytes: &[u8], limit: usize) -> io::Result<Vec<u8>> {
        if bytes.len() > 16 * 1024 * 1024 {
            return Err(io::Error::new(
                io::ErrorKind::FileTooLarge,
                "display input limit",
            ));
        }
        let bytes = prefix(bytes);
        let _guard = self.enter()?;
        let mut state = zero_state();
        let mut at = 0;
        let mut out = buffer(bytes.len(), limit)?;
        while at < bytes.len() {
            let Some((n, c)) = Self::next(&bytes[at..], &mut state)? else {
                // file_getbuffer discards all wide output if any sequence is
                // invalid/incomplete, then restarts byte-wise for the whole text.
                out.clear();
                for &b in bytes {
                    if self.byte_printable(b) {
                        append(&mut out, &[b], limit)?;
                    } else {
                        octal(&mut out, b, limit)?;
                    }
                }
                return Ok(out);
            };
            if self.printable(c) {
                append(&mut out, &bytes[at..at + n], limit)?;
            } else {
                for &b in &bytes[at..at + n] {
                    octal(&mut out, b, limit)?;
                }
            }
            at += n;
        }
        Ok(out)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn guard_restores_on_success_error_and_unwind() {
        // SAFETY: NULL queries the current thread locale without changing it.
        let original = unsafe { libc::uselocale(std::ptr::null_mut()) };
        let locale = Locale::new("C").unwrap();
        assert_eq!(locale.filename(b"a\xff", false, 8).unwrap(), b"a\\377");
        assert!(locale.filename(b"a\xff", false, 1).is_err());
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = locale.enter().unwrap();
            panic!("unwind control");
        }));
        // SAFETY: NULL only queries the same thread after all guards dropped.
        assert_eq!(unsafe { libc::uselocale(std::ptr::null_mut()) }, original);
    }
}
