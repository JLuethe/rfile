# Experimental C interface

This crate exports the 19 file 5.48 `magic_*` functions through the independent
Rust engine, plus `rfile_implementation_version()`. It is an **experimental ABI
preview, not a certified drop-in replacement**. Linux x86_64 GNU is the initial
build target; other ABIs are rejected at build time. No call invokes libmagic or
installs a sandbox. `extended-codecs` is opt-in.

Build with `cargo build -p rfile-ffi --features extended-codecs`. The distinct
shared library is `librfile_magic.so`, with SONAME `librfile_magic.so.1`; a static
archive is also produced. Use `include/rfile/magic.h`. Do not overwrite system
libmagic. Package verification checks exactly 20 dynamic exports and no native
detector linkage. The original header copyright and conditions are retained;
this implementation remains GPL-3.0-only.

The safe core performs all parsing and classification. This crate confines
unsafe operations to exported ABI entry points, live foreign memory borrows,
cookie allocation ownership, thread errno and temporary descriptor duplication.
Every production unsafe block states its precondition. Inputs with detectable
NULL/alignment/count/size errors are rejected before constructing slices.
Non-NULL dangling pointers, falsely declared allocations and concurrent close
are foreign-caller contract violations, not errors Rust can validate.

Cookies open unloaded. Callers own and close each cookie once. Serialize all
operations on one cookie, including use of its borrowed result/error pointers.
The internal mutex does not extend those pointers' lifetimes. Different cookies
may run on different threads. The library does not change global locale or
process sandbox state. A panic is caught and poisons the cookie; error/errno and
close remain available. Builds with panic=abort are rejected; allocator abort
and invalid foreign memory cannot be caught by catch_unwind.

Database buffers are copied into owned parsed structures. Unlike the native
USER mapping, callers may free their source bytes after a successful Rust load.
Descriptor calls own only a CLOEXEC duplicate; they retain the caller FD and
restore seekable position. Concurrent operations on a shared open file
description remain the caller's responsibility. Pipes are consumed.

Bounds: 256 MiB per C classification buffer, 128 MiB aggregate database bytes,
16 database paths/buffers, 4,096 source fragments, 65,536-byte path lists and
16 MiB aggregate action output. Core execution, recursion, allocation and output
limits still apply. Legacy 16-bit parameter conversion precedes transactional
safety-ceiling validation; unsafe effective requests fail with ERANGE.
Compilation atomically replaces a same-directory output with mode 0644 subject
to umask, intentionally replacing inode/symlink ownership instead of native
O_TRUNC behavior. The containing output directory must be trusted.

Current gaps include malformed-source warning/error sequencing, complete
compiled-header diagnostics, source warning recovery/counter edge cases, DEBUG
tracing, locale-specific IO error wording, special-file/failure paths, and the
engine's recorded CDF/ELF/regex/platform limitations. A successful compile/link
or `magic_version()==548` does not certify those behaviors. See
[the ABI specification](../../docs/remaining/07-c-abi.md), `reports/abi-state.json`
and `reports/abi-clients.json` for the exact comparison scope. Packaging and
stateful ABI fuzzing remain release gates until their reports are present.

Run `python3 tools/abi_clients.py --rust-only` after building for C/C++ linkage,
parameter, ownership and concurrency checks (including C ASan/UBSan). With a
clean pinned oracle selected through `RFILE_REFERENCE`, omit `--rust-only` to
compile the same client against both libraries; run `tools/abi_compare.py` for
separate-process state/action/channel comparisons. Native input buffers stay
alive until close; only the Rust-specific test releases them earlier. ASan on
the C client does not imply that Rust allocations are ASan-instrumented.
