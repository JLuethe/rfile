# rfile platform boundary

This crate provides platform services, not file-format detection. The detection,
magic parser/evaluator, codecs and output formatter remain in the safe Rust core.
No libmagic or native codec bindings are allowed here. Linux/glibc locale support
uses an owned immutable `locale_t` acquisition from `newlocale(LC_CTYPE_MASK,
name, NULL)`. The object is released once with `freelocale`; glibc can share its
static C object/data, so ownership does not mean its address is globally unique.

The certified display set is C, C.UTF-8 and en_US.UTF-8 on the recorded Linux/glibc
host. Other installed ASCII/UTF-8 locales use the same adapter but need their own
comparison evidence. Other encodings and non-GNU platforms return explicit
unsupported configuration errors. The core's default C context is portable Rust
and does not require a native locale object. Locale names are capped at 1,024
bytes; one adapter conversion accepts at most 16 MiB. Host locale data is trusted
configuration loaded before inspection, outside the inspection allocation budget.

The unsafe boundary has these invariants:

- Handles come from successful `newlocale` with a NULL base and are not exposed.
  No method mutates an acquired object. `Send`/`Sync` rely on immutable shared
  locale data and independently bound thread locales.
- A private non-Send/non-Sync guard borrows the owner and restores the previous
  `uselocale` binding on success, error and Rust unwind. It cannot cross async
  suspension through a public API. No `setlocale` or environment mutation occurs.
- `mbrtowc` receives a live readable slice and its exact length, plus distinct,
  aligned writable `wchar_t`/`mbstate_t` objects. The zeroed glibc conversion state
  has its documented initial representation. Error/incomplete counts consume
  one byte/reset for filenames; result rendering restarts byte-wise as upstream.
- Successful conversion counts are checked before slicing. Wide values passed
  to `iswprint_l`/`wcwidth` come from that locale's successful conversion. Byte
  ctype calls receive 0..255, avoiding signed-char undefined behavior.
- Each output preallocates a checked bound (at most four bytes per input byte,
  further restricted by the caller's cap); append checks precede writes. Returned
  vectors own their bytes and retain no C pointer.
- GNU `wchar_t`, `wint_t`, `mbstate_t` and function declarations were checked
  against libc 0.2.191 and the installed C headers. Unsupported ABIs are gated out.

The ABI source contracts are documented in [newlocale](https://man7.org/linux/man-pages/man3/newlocale.3.html)
and [uselocale](https://man7.org/linux/man-pages/man3/uselocale.3.html).
External C callers remain responsible for coordinating their own concurrent
locale-object destruction or process-global `setlocale` changes. The core never
performs those mutations. The standard C library is not rebuilt or proven safe
by this adapter. Sanitizer smoke, concurrency and unwind tests are evidence;
sustained campaigns and independent review remain explicit security gates.

## Optional kernel adapter

The `sandbox` feature on x86_64 Linux/glibc exposes explicit current-thread
`SandboxPlan::prepare` / `install`, never called by detection or locale APIs.
Typed policies generate bounded seccompiler BPF and fixed V5 Landlock rules.
Installation consumes the plan; status is !Send/!Sync. Unexpected query/install
errors propagate; partial irreversible installation is never described as undone.
The caller must not treat a returned status as process-wide protection in a
multithreaded host. The CLI checks single-threaded startup and denies new threads.

First-party unsafe calls are only integer fcntl/PR_GET_SECCOMP queries and the
NULL/zero-length Landlock ABI query. None takes a user-provided raw pointer or
owns/closes an unknown FD. Third-party landlock/seccompiler perform the policy
syscalls; rustix provides safe descriptor-relative staging/publication and must be included in dependency review. Descriptor inventory avoids
closing foreign Rust-owned descriptors; policy denies their content authority.
See [the complete scope and limitations](../../docs/remaining/05-sandbox.md).
