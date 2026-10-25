// SPDX-License-Identifier: GPL-3.0-only
//! Experimental Linux C interface. See README.md for the foreign-caller contract.
#![deny(unsafe_op_in_unsafe_fn)]
#[cfg(not(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu")))]
compile_error!("the experimental C ABI is currently qualified only for Linux x86_64 GNU");
#[cfg(not(panic = "unwind"))]
compile_error!("the C ABI requires panic=unwind for its panic boundary");
mod compiled;
mod state;
use libc::{c_char, c_int, c_void};
use state::{Action, State};
use std::{
    cell::RefCell,
    ffi::CString,
    os::fd::FromRawFd,
    panic::{AssertUnwindSafe, catch_unwind},
    ptr,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

struct Cookie {
    state: Mutex<State>,
    poisoned: AtomicBool,
}
const PANIC: &std::ffi::CStr = c"rfile: cookie poisoned by an internal panic";
fn errno(n: i32) {
    // SAFETY: glibc supplies a live thread-local int pointer.
    unsafe {
        *libc::__errno_location() = n;
    }
}
/// All non-NULL handles must come from magic_open, remain live and be exclusively
/// synchronized by the foreign caller (including access through returned pointers).
unsafe fn with_cookie<T: Copy>(
    handle: *mut c_void,
    failure: T,
    f: impl FnOnce(&mut State) -> T,
) -> T {
    if handle.is_null() {
        return failure;
    }
    // SAFETY: the caller contract guarantees a live, aligned Cookie allocation.
    let cookie = unsafe { &*handle.cast::<Cookie>() };
    if cookie.poisoned.load(Ordering::Acquire) {
        errno(libc::EFAULT);
        return failure;
    }
    match catch_unwind(AssertUnwindSafe(|| {
        let mut guard = cookie.state.lock().unwrap_or_else(|e| e.into_inner());
        f(&mut guard)
    })) {
        Ok(v) => v,
        Err(_) => {
            cookie.poisoned.store(true, Ordering::Release);
            errno(libc::EFAULT);
            failure
        }
    }
}
/// Read at most the accepted path-list ceiling. The caller must provide a live
/// NUL-terminated allocation; this is a size limit, not an invalid-pointer probe.
unsafe fn c_path(value: *const c_char) -> Result<Option<Vec<u8>>, ()> {
    if value.is_null() {
        return Ok(None);
    }
    let mut bytes = Vec::new();
    for i in 0..=65536 {
        // SAFETY: caller guarantees readable bytes through its terminating NUL.
        let b = unsafe { *value.add(i) } as u8;
        if b == 0 {
            return Ok(Some(bytes));
        }
        if i == 65536 {
            return Err(());
        }
        bytes.push(b);
    }
    unreachable!()
}
#[unsafe(no_mangle)]
pub extern "C" fn magic_open(flags: c_int) -> *mut c_void {
    catch_unwind(|| {
        Box::into_raw(Box::new(Cookie {
            state: Mutex::new(State::new(flags)),
            poisoned: AtomicBool::new(false),
        }))
        .cast()
    })
    .unwrap_or_else(|_| {
        errno(libc::EFAULT);
        ptr::null_mut()
    })
}
/// # Safety
/// `handle` is NULL or a live cookie owned by this caller, closed exactly once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn magic_close(handle: *mut c_void) {
    if !handle.is_null() {
        let _ = catch_unwind(AssertUnwindSafe(|| {
            // SAFETY: caller transfers sole ownership of an allocation from open.
            drop(unsafe { Box::from_raw(handle.cast::<Cookie>()) });
        }));
    }
}
/// # Safety
/// `handle` is NULL or a live, externally synchronized cookie.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn magic_error(handle: *mut c_void) -> *const c_char {
    if handle.is_null() {
        return c"Magic database is not open".as_ptr();
    }
    // SAFETY: live-cookie precondition; atomic access needs no state lock.
    if unsafe { &*handle.cast::<Cookie>() }
        .poisoned
        .load(Ordering::Acquire)
    {
        return PANIC.as_ptr();
    }
    // SAFETY: forwards the same cookie lifetime contract.
    unsafe {
        with_cookie(handle, PANIC.as_ptr(), |s| {
            s.error.as_ref().map_or(ptr::null(), |s| s.as_ptr())
        })
    }
}
/// # Safety
/// `handle` is NULL or a live, externally synchronized cookie.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn magic_errno(handle: *mut c_void) -> c_int {
    if handle.is_null() {
        return libc::EINVAL;
    }
    // SAFETY: live-cookie precondition.
    if unsafe { &*handle.cast::<Cookie>() }
        .poisoned
        .load(Ordering::Acquire)
    {
        return libc::EFAULT;
    }
    // SAFETY: forwards the cookie lifetime contract.
    unsafe {
        with_cookie(handle, libc::EFAULT, |s| {
            if s.error.is_some() { s.error_number } else { 0 }
        })
    }
}
/// # Safety
/// `handle` is NULL or a live, externally synchronized cookie.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn magic_getflags(handle: *mut c_void) -> c_int {
    // SAFETY: forwards the cookie lifetime contract.
    unsafe { with_cookie(handle, -1, |s| s.flags) }
}
/// # Safety
/// `handle` is NULL or a live, externally synchronized cookie.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn magic_setflags(handle: *mut c_void, flags: c_int) -> c_int {
    // SAFETY: forwards the cookie lifetime contract.
    unsafe {
        with_cookie(handle, -1, |s| {
            s.flags = flags;
            0
        })
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn magic_version() -> c_int {
    548
}
#[unsafe(no_mangle)]
pub extern "C" fn rfile_implementation_version() -> *const c_char {
    c"{\"implementation\":\"rfile\",\"version\":\"0.1.0\",\"upstream\":\"41b1123083ab0bbdfe32f6d4ec14f6aec9a91e59\",\"profile\":\"Pinned548\",\"manifest_version\":1,\"abi\":\"experimental\",\"drop_in_compatible\":false}".as_ptr()
}
thread_local! { static PATH_RESULT: RefCell<Option<CString>> = const { RefCell::new(None) }; }
/// # Safety
/// `name` is NULL or a live NUL-terminated string. Explicit names are returned
/// unchanged; other results live until the next same-thread getpath mutation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn magic_getpath(name: *const c_char, action: c_int) -> *const c_char {
    if !name.is_null() {
        return name;
    }
    catch_unwind(|| {
        PATH_RESULT.with(|p| {
            let mut p = p.borrow_mut();
            *p = Some(state::string(state::default_path(action)));
            p.as_ref().expect("stored path").as_ptr()
        })
    })
    .unwrap_or_else(|_| {
        errno(libc::EFAULT);
        ptr::null()
    })
}
#[unsafe(no_mangle)]
pub extern "C" fn magic_getmaxparam(id: c_int) -> isize {
    match id {
        0..=5 | 8 => 65535,
        6 | 7 | 9 => 2147483647,
        _ => {
            errno(libc::EINVAL);
            -1
        }
    }
}
/// # Safety
/// A non-NULL `value` points to one writable aligned size_t. Cookie is live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn magic_getparam(
    handle: *mut c_void,
    id: c_int,
    value: *mut c_void,
) -> c_int {
    if handle.is_null() || value.is_null() || !value.cast::<usize>().is_aligned() {
        errno(libc::EFAULT);
        return -1;
    }
    // SAFETY: cookie preconditions and writable size_t contract above.
    unsafe {
        with_cookie(handle, -1, |s| {
            if !(0..10).contains(&id) {
                errno(libc::EINVAL);
                return -1;
            }
            value.cast::<usize>().write(s.params[id as usize]);
            0
        })
    }
}
/// # Safety
/// A non-NULL `value` points to one readable aligned size_t. Cookie is live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn magic_setparam(
    handle: *mut c_void,
    id: c_int,
    value: *const c_void,
) -> c_int {
    if handle.is_null() || value.is_null() || !value.cast::<usize>().is_aligned() {
        errno(libc::EFAULT);
        return -1;
    }
    // SAFETY: cookie preconditions and readable size_t contract above.
    unsafe {
        with_cookie(handle, -1, |s| {
            if !(0..10).contains(&id) {
                errno(libc::EINVAL);
                return -1;
            }
            match s.set_param(id as usize, value.cast::<usize>().read()) {
                Ok(()) => 0,
                Err(n) => {
                    errno(n);
                    -1
                }
            }
        })
    }
}
/// # Safety
/// Cookie is live. A nonempty buffer is a readable allocation of `size` bytes,
/// immutable throughout the call. NULL is accepted only with zero size.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn magic_buffer(
    handle: *mut c_void,
    buffer: *const c_void,
    size: usize,
) -> *const c_char {
    // SAFETY: cookie contract; buffer checked before constructing its borrow.
    unsafe {
        with_cookie(handle, ptr::null(), |s| {
            if !s.reset(true) {
                return ptr::null();
            }
            if size > 256 * 1024 * 1024 {
                s.fail(
                    b"rfile limit exceeded: C buffer size".to_vec(),
                    libc::ERANGE,
                );
                return ptr::null();
            }
            if size != 0 && buffer.is_null() {
                s.fail(b"invalid NULL buffer".to_vec(), libc::EFAULT);
                return ptr::null();
            }
            let bytes = if size == 0 {
                &[]
            } else {
                std::slice::from_raw_parts(buffer.cast::<u8>(), size)
            };
            let result = s.engine().map(|e| e.inspect_complete_buffer(bytes));
            s.accept_observed(result)
        })
    }
}
fn descriptor(s: &mut State, fd: c_int) -> *const c_char {
    if !s.reset(true) {
        return ptr::null();
    }
    if fd == -1 {
        let result = s.engine().map(|e| e.inspect_complete_buffer(&[]));
        return s.accept_observed(result);
    }
    // SAFETY: fcntl validates the integer descriptor. Only its successfully
    // created duplicate is converted to File ownership; caller FD stays open.
    let owned = unsafe { libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, 0) };
    if owned < 0 {
        let e = std::io::Error::last_os_error();
        let number = e.raw_os_error().unwrap_or(libc::EBADF);
        let message = e.to_string();
        let message = message
            .rsplit_once(" (os error ")
            .map_or(message.as_str(), |(prefix, _)| prefix);
        s.fail(
            format!("cannot read fd {fd} ({message})").into_bytes(),
            number,
        );
        return ptr::null();
    }
    // SAFETY: successful fcntl returned a fresh owned descriptor.
    let mut file = unsafe { std::fs::File::from_raw_fd(owned) };
    let result = s.engine().map(|e| e.inspect_file(&mut file));
    s.accept_observed(result)
}
/// # Safety
/// Cookie is live. Caller coordinates concurrent operations on `fd`'s shared
/// open file description; this call never takes ownership of `fd`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn magic_descriptor(handle: *mut c_void, fd: c_int) -> *const c_char {
    // SAFETY: forwards cookie contract; descriptor adapter validates fd via OS.
    unsafe { with_cookie(handle, ptr::null(), |s| descriptor(s, fd)) }
}
/// # Safety
/// Cookie is live; name is NULL (stdin) or a live NUL-terminated path.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn magic_file(handle: *mut c_void, name: *const c_char) -> *const c_char {
    // SAFETY: forwards cookie and path contracts.
    unsafe {
        with_cookie(handle, ptr::null(), |s| {
            if name.is_null() {
                return descriptor(s, 0);
            }
            if !s.reset(true) {
                return ptr::null();
            }
            match c_path(name) {
                Ok(Some(name)) => {
                    let result = s.engine().map(|e| e.inspect_path(&state::path(&name)));
                    s.accept_observed(result)
                }
                _ => {
                    s.fail(b"rfile limit exceeded: path size".to_vec(), libc::ERANGE);
                    ptr::null()
                }
            }
        })
    }
}
macro_rules! action_export {
    ($name:ident,$action:ident) => {
        /// # Safety
        /// Cookie is live; name is NULL or a live NUL-terminated path list.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(handle: *mut c_void, name: *const c_char) -> c_int {
            // SAFETY: forwards cookie and string lifetime contracts.
            unsafe {
                with_cookie(handle, -1, |s| match c_path(name) {
                    Ok(name) => s.action(name, Action::$action),
                    Err(()) => {
                        s.reset(false);
                        s.loaded = None;
                        s.fail(
                            b"rfile limit exceeded: database path size".to_vec(),
                            libc::ERANGE,
                        );
                        -1
                    }
                })
            }
        }
    };
}
action_export!(magic_load, Load);
action_export!(magic_compile, Compile);
action_export!(magic_check, Check);
action_export!(magic_list, List);
/// # Safety
/// Cookie is live. For nonzero count, buffers and sizes are readable aligned
/// arrays of that count. Each nonempty buffer is readable for its declared size.
/// All arrays/bytes remain immutable during the call; none are retained or freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn magic_load_buffers(
    handle: *mut c_void,
    buffers: *const *const c_void,
    sizes: *const usize,
    count: usize,
) -> c_int {
    // SAFETY: cookie and foreign array contracts above; all detectable size/null
    // failures are rejected before constructing any array or buffer borrow.
    unsafe {
        with_cookie(handle, -1, |s| {
            if count == 0 {
                return -1;
            }
            s.reset(false);
            s.loaded = None;
            if count > 16 {
                s.fail(
                    b"rfile limit exceeded: database count".to_vec(),
                    libc::ERANGE,
                );
                return -1;
            }
            if buffers.is_null() || sizes.is_null() || !buffers.is_aligned() || !sizes.is_aligned()
            {
                s.fail(b"invalid database arrays".to_vec(), libc::EFAULT);
                return -1;
            }
            let sizes = std::slice::from_raw_parts(sizes, count);
            let Some(total) = sizes.iter().try_fold(0usize, |n, &v| n.checked_add(v)) else {
                s.fail(
                    b"rfile limit exceeded: database bytes".to_vec(),
                    libc::ERANGE,
                );
                return -1;
            };
            if total > 128 * 1024 * 1024 {
                s.fail(
                    b"rfile limit exceeded: database bytes".to_vec(),
                    libc::ERANGE,
                );
                return -1;
            }
            let pointers = std::slice::from_raw_parts(buffers, count);
            let mut databases = Vec::new();
            for (&p, &n) in pointers.iter().zip(sizes) {
                if p.is_null() && n != 0 {
                    s.fail(b"invalid NULL database buffer".to_vec(), libc::EFAULT);
                    return -1;
                }
                let bytes = if n == 0 {
                    &[]
                } else {
                    std::slice::from_raw_parts(p.cast::<u8>(), n)
                };
                match compiled::load(bytes, b"buffer", false) {
                    Ok(db) => databases.push(db),
                    Err(e) => {
                        s.rust_error(e);
                        return -1;
                    }
                }
            }
            s.load_buffers(databases)
        })
    }
}

/// Stateful testing entry point. Every foreign pointer is backed by a live Rust
/// allocation; fuzz bytes never select filesystem paths or raw pointer values.
#[cfg(feature = "fuzzing")]
#[doc(hidden)]
pub fn fuzz_state_sequence(data: &[u8]) {
    if data.len() > 65536 {
        return;
    }
    struct Owned(*mut c_void);
    impl Drop for Owned {
        fn drop(&mut self) {
            // SAFETY: this sole owner closes its live handle exactly once.
            unsafe { magic_close(self.0) }
        }
    }
    let handle = Owned(magic_open(0));
    if handle.0.is_null() {
        return;
    }
    let db = rfile::Database::from_source("0 string ABC matched\n!:mime application/x-test\n")
        .expect("fixed rules")
        .compile()
        .expect("fixed database");
    // SAFETY: all pointers below are to live, aligned owned allocations, on one
    // thread. Counts always describe those allocations, except NULL+count cases
    // explicitly guaranteed to be rejected before dereferencing.
    unsafe {
        with_cookie(handle.0, (), |s| {
            s.params[0] = 8;
            s.params[1] = 8;
            s.params[6] = 65536;
            s.params[7] = 65536;
        });
        for chunk in data.chunks(32).take(64) {
            let Some((&op, bytes)) = chunk.split_first() else {
                continue;
            };
            match op % 11 {
                0 => {
                    let pointers = [db.as_ptr().cast::<c_void>()];
                    let sizes = [db.len()];
                    magic_load_buffers(handle.0, pointers.as_ptr(), sizes.as_ptr(), 1);
                }
                1 => {
                    let mut changed = db.clone();
                    if bytes.len() >= 2 {
                        let offset =
                            usize::from(u16::from_le_bytes([bytes[0], bytes[1]])) % changed.len();
                        let n = (bytes.len() - 2).min(changed.len() - offset);
                        changed[offset..offset + n].copy_from_slice(&bytes[2..2 + n]);
                    }
                    let pointers = [changed.as_ptr().cast::<c_void>()];
                    let sizes = [changed.len()];
                    magic_load_buffers(handle.0, pointers.as_ptr(), sizes.as_ptr(), 1);
                }
                2 => {
                    magic_buffer(handle.0, bytes.as_ptr().cast(), bytes.len());
                }
                3 => {
                    let mut bits = [0; 4];
                    let n = bytes.len().min(4);
                    bits[..n].copy_from_slice(&bytes[..n]);
                    magic_setflags(handle.0, i32::from_le_bytes(bits));
                }
                4 => {
                    let mut value = [0; 8];
                    let n = bytes.len().min(8);
                    value[..n].copy_from_slice(&bytes[..n]);
                    let value = usize::from_le_bytes(value);
                    magic_setparam(
                        handle.0,
                        bytes.first().map_or(0, |&b| i32::from(b) % 12 - 1),
                        (&value as *const usize).cast(),
                    );
                }
                5 => {
                    let mut value = 0usize;
                    magic_getparam(
                        handle.0,
                        bytes.first().map_or(0, |&b| i32::from(b) % 12 - 1),
                        (&mut value as *mut usize).cast(),
                    );
                }
                6 => {
                    magic_load_buffers(handle.0, ptr::null(), ptr::null(), 0);
                }
                7 => {
                    magic_load_buffers(handle.0, ptr::null(), ptr::null(), 1);
                }
                8 => {
                    magic_buffer(
                        handle.0,
                        ptr::null(),
                        usize::from(bytes.first().copied().unwrap_or(0)),
                    );
                }
                9 => {
                    let pointers = [bytes.as_ptr().cast::<c_void>()];
                    let sizes = [bytes.len()];
                    magic_load_buffers(handle.0, pointers.as_ptr(), sizes.as_ptr(), 1);
                }
                _ => {
                    magic_getflags(handle.0);
                    magic_errno(handle.0);
                    magic_error(handle.0);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detectable_foreign_argument_errors_are_bounded_and_stateful() {
        let h = magic_open(0);
        let db = rfile::Database::from_source("0 string ABC match\n")
            .unwrap()
            .compile()
            .unwrap();
        let pointers = [db.as_ptr().cast::<c_void>()];
        let sizes = [db.len()];
        // SAFETY: one live cookie; allocations match declared sizes, except the
        // documented early-rejected NULL/oversize combinations under test.
        unsafe {
            assert_eq!(
                magic_load_buffers(h, pointers.as_ptr(), sizes.as_ptr(), 1),
                0
            );
            assert!(magic_buffer(h, ptr::null(), 256 * 1024 * 1024 + 1).is_null());
            assert_eq!(magic_errno(h), libc::ERANGE);
            assert!(magic_buffer(h, ptr::null(), 1).is_null());
            assert_eq!(magic_errno(h), libc::EFAULT);
            let p = magic_buffer(h, c"ABC".as_ptr().cast(), 3);
            assert_eq!(std::ffi::CStr::from_ptr(p).to_bytes(), b"match");
            assert!(magic_error(h).is_null());
            assert_eq!(magic_errno(h), 0);
            assert_eq!(magic_getflags(h), 0);
            assert_eq!(magic_setflags(h, 16), 0);
            let mut current = 0usize;
            assert_eq!(magic_getparam(h, 6, (&mut current as *mut usize).cast()), 0);
            // Non-result-mutating queries/flag changes retain the returned bytes.
            assert_eq!(std::ffi::CStr::from_ptr(p).to_bytes(), b"match");
            assert_eq!(magic_load_buffers(h, ptr::null(), ptr::null(), 17), -1);
            assert_eq!(magic_errno(h), libc::ERANGE);
            assert!(magic_buffer(h, c"ABC".as_ptr().cast(), 3).is_null());
            let huge = usize::MAX;
            assert_eq!(magic_setparam(h, 6, (&huge as *const usize).cast()), -1);
            let mut value = 0usize;
            assert_eq!(magic_getparam(h, 6, (&mut value as *mut usize).cast()), 0);
            assert_eq!(value, 7 * 1024 * 1024);
            magic_close(h);
        }
    }
    #[test]
    fn panic_is_contained_and_cookie_is_poisoned() {
        let h = magic_open(0);
        // SAFETY: h is live, used on one thread and closed once.
        unsafe {
            assert_eq!(
                with_cookie(h, -1, |_| -> i32 { panic!("controlled boundary test") }),
                -1
            );
            assert_eq!(magic_errno(h), libc::EFAULT);
            assert_eq!(magic_error(h), PANIC.as_ptr());
            assert_eq!(magic_setflags(h, 1), -1);
            magic_close(h);
            magic_close(ptr::null_mut());
        }
    }
}
