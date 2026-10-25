# C ABI: independent libmagic-compatible interface

ABI-01 through ABI-12 specify the compatibility library requested in the [parent plan](../07-remaining-work-spec.md). The current project exposes Rust APIs only. This is a new interface around the independent Rust engine, not a binding to the original C detector.

## ABI-01: exact target, artifacts and scope

The C ABI defaults to Pinned548; CorrectedV1 is available through the Rust/CLI interfaces unless a separately versioned extension explicitly exposes it.

Target the **19 public functions** declared by pinned `upstream/src/magic.h.in:128–162`, including `magic_getmaxparam`. Use the generated header's constants/signatures as the Linux baseline. Pin numeric flag/parameter values and `MAGIC_VERSION=548`; `magic_version()` is the supported libmagic API/data compatibility version, not the Rust package SemVer. A separate `rfile_implementation_version()` extension returns the actual engine version, upstream commit, profile and capability-manifest version.

Create `crates/rfile-ffi` with `cdylib` and optional `staticlib`, depending on the safe core and reviewed platform adapter. Default shared identity is **librfile_magic.so.1** with `librfile_magic.so` development symlink; headers and pkg-config use a distinct prefix/package (`rfile-magic`). Linux x86_64/glibc is the first certified ABI. macOS dylib/Windows DLL, calling conventions and Windows ssize_t/paths are separate gates.

An opt-in compatibility package can install `libmagic.so.1`, its development symlink, `magic.h` and libmagic pkg-config metadata in a chosen isolated prefix. It must not overwrite system libraries during development. The pinned reference SONAME was observed as `libmagic.so.1`; symbol availability alone does not establish behavioral compatibility. Existing clients can first use the distinctly named library through an explicit path/link option. Test possible symbol interposition when both original and Rust libraries are loaded; use separate oracle processes or RTLD_LOCAL, not a test that accidentally calls the original.

The core keeps forbid(unsafe_code). Unsafe in the FFI crate is allowed only for foreign pointer/length conversion, handle ownership, borrowed descriptors, ABI calls and returned-buffer lifetimes. It does not implement file parsers, regex or decompression. C ABI packaging does not change GPL-3.0-only licensing or erase original notices; do not present ABI compatibility as LGPL-style licensing permission.

## ABI-02: handle, state and concurrency

Public header retains opaque `typedef struct magic_set *magic_t`. Internal allocation is a boxed Cookie; the foreign pointer is opaque and never dereferenced by client code. Suggested owned state:

```rust
struct Cookie {
    state: Mutex<CookieState>,
}
struct CookieState {
    flags: i32,
    parameters: LegacyParameters,
    loaded: Option<Arc<Database>>,
    result: Option<CString>,
    error: Option<CString>,
    error_number: i32,
    poisoned: bool,
    options: CompatibilityOptions,
}
```

`magic_open` allocates an **unloaded** handle; it must not call `Engine::builtin()` and silently load rules. Loading, compilation/check/list and classification have separate observable transitions. Successful ordinary classification returns owned cookie storage; the pointer is borrowed by the caller until the next result-mutating call or close. Callers must not free it. Rust's owned result API keeps its stronger lifetime contract; do not weaken it to imitate C.

Serialize internal handle mutation, but do not promise that the lock extends returned C pointer lifetime across concurrent calls. The client must externally synchronize one cookie, including reads through an outstanding pointer, and never close it during an operation. Independent cookies share immutable databases/caches safely. No engine lock may be held across an arbitrary client callback; no callback API is planned here.

A wrong, dangling, already-closed or concurrently destroyed non-NULL cookie violates the foreign-caller contract. Pointer null/alignment checks cannot establish arbitrary foreign-memory validity. Do not claim all invalid C pointers can be converted to safe errors. Handle registry/debug diagnostics may help development but do not substitute for lifetime preconditions or authorize use-after-free.

## ABI-03: full function contract

| Function | Required behavior / verification |
|---|---|
| `magic_open(int flags)` | unloaded cookie, store flag bit pattern and legacy defaults; allocation failure returns NULL with appropriate errno where recoverable |
| `magic_close(magic_t)` | free every owned resource once; NULL is harmless; never free caller database/input memory |
| `magic_getpath(const char *, int)` | explicit non-NULL path wins and returns the supplied pointer; then MAGIC including empty; load action 0 permits home/default discovery, actions 1/2/3 select check/compile/list default path semantics |
| `magic_file(magic_t,const char *)` | path metadata/read pipeline; NULL path means borrowed stdin; same symlink/error/atime behavior as corresponding C entry point |
| `magic_descriptor(magic_t,int)` | borrow valid descriptor, preserve ownership, restore seek position when seekable on success/error; pipes are consumed |
| `magic_buffer(magic_t,const void *,size_t)` | classify supplied buffer without metadata/seek enrichment; NULL+0 may represent empty; NULL+nonzero is a detectable invalid argument |
| `magic_error(magic_t)` | NULL when no handle error, otherwise cookie-owned error text; NULL cookie yields the reference's static not-open message |
| `magic_errno(magic_t)` | zero when no error, recorded native/logical error number otherwise; NULL cookie returns EINVAL |
| `magic_getflags(magic_t)` | return stored signed-int bit pattern; NULL returns -1; does not reset a prior classification error |
| `magic_setflags(magic_t,int)` | set flags without loading/resetting database; preserve error-state behavior; platform-inapplicable features use a documented error |
| `magic_version(void)` | return 548, independent of rfile package version; build/feature details are in the separate extension |
| `magic_load(magic_t,const char *)` | path-list/discovery load and exact cookie-state transitions; success 0/failure -1; valid partial list handling as below |
| `magic_load_buffers(magic_t,void **,size_t *,size_t)` | validate array/count contract; copy/validate each v21 input into Rust-owned storage; preserve list order and caller ownership |
| `magic_compile(magic_t,const char *)` | perform source compilation/file creation and legacy state/diagnostics; must not be a no-op or Rust list surrogate |
| `magic_check(magic_t,const char *)` | legacy parser/check action, channels/status/state; warning/error renderer is a dependency |
| `magic_list(magic_t,const char *)` | legacy strength listing, ordering/channels/state; not merely current Rust `Database::list` output |
| `magic_setparam(magic_t,int,const void *)` | read one valid aligned size_t and map ID/legacy storage width; validate safety ceilings after width conversion |
| `magic_getparam(magic_t,int,void *)` | write size_t to caller storage; do not expose Rust usize layout through a different C type |
| `magic_getmaxparam(int)` | pinned ssize_t maxima by parameter, invalid ID -1 plus errno EINVAL; do not confuse advisory maximum with current default or hard safety cap |

NULL-cookie operations follow `magic.c`, not a single invented uniform error policy: load/compile/check/list and flag setters return -1, classifications NULL; error/errno queries have the special behavior above. Parameter functions with NULL cookie/value set EFAULT and return -1. Record whether each case touches thread errno and cookie error independently. Preserve valid no-op queries instead of clearing output/error preemptively.

## ABI-04: database discovery, loads and mutating action state

Explicit database path takes precedence over MAGIC even if empty; MAGIC takes precedence over platform defaults. Preserve Unix path-list separators, home `.magic.mgc`/`.magic`/`.magic/magic.mgc` discovery, source/compiled fallback, order and action values. Package default rules in the installed prefix and report its actual discovery path. The embedded database remains the Rust/CLI packaging fallback; an ABI installation claiming legacy discovery must provide a real configured data path rather than returning a fictitious embedded filesystem filename from `magic_getpath`.

The pin discards prior loaded lists at the start of a real load/action. It takes the maximum per-file result and can return success when at least one path loaded, while an earlier file error/diagnostic may still be observable (`apprentice.c:750–800`). Probe and preserve result/error/state separately for valid:missing, missing:valid, malformed:valid, empty elements and all-failing lists. Do not implement an all-or-nothing transactional Rust load and silently preserve the previous DB if C unloads it. `magic_load_buffers` with count zero returns early without the normal reset; add a distinct state regression. A later nonzero failed buffer load clears loaded state according to its failure path.

Successful compile/check/list can leave parsed lists loaded at the pin (`apprentice.c:789–810`); wrappers must test subsequent classification, not assume these are stateless tools. Source warnings and listing output currently differ in Rust; a dedicated compatibility diagnostic/list renderer is a prerequisite for ABI completion. Copy raw source provenance and positions through parsing; format diagnostics safely without native printf on attacker-controlled text.

`magic_load_buffers` copies accepted input bytes, never frees or modifies caller arrays/buffers, and validates aggregate bytes/counts before allocation. Caller buffers need remain valid only during the call. Opposite-endian import/ownership and copy behavior are explicitly tested; any stronger nonmutation/lifetime guarantee than C is recorded as an ownership improvement, not byte-format proof.

## ABI-05: parameter IDs and safe limits

| ID | Legacy parameter / default | Legacy getmax result |
|---|---|---:|
| 0 | indirect / 50 | 65535 |
| 1 | named calls / 150 | 65535 |
| 2 | ELF program headers / 2048 | 65535 |
| 3 | ELF sections / 32768 | 65535 |
| 4 | ELF notes / 256 | 65535 |
| 5 | regex search bytes / 8192 | 65535 |
| 6 | file-read bytes / 7 MiB | 2147483647 |
| 7 | encoding bytes / 64 KiB | 2147483647 |
| 8 | ELF section bytes / 128 MiB | 65535 (observed inconsistency) |
| 9 | magic warnings / 64 | 2147483647 |

The reference stores several fields at 16 bits and truncates a supplied size_t; ID8 uses size_t despite its getmax value. Preserve deterministic storage-width conversion in a LegacyParameters layer; do not truncate all fields to the reported maximum. Validate the resulting effective value against the Rust hard ceilings. Reject a value requiring unsafe/unbounded execution with -1, errno ERANGE and a precise safety-limit message; leave the previous effective value unchanged. This is an explicit difference where C accepts larger work. Zero and wrap boundaries require independent tests, including a value truncating to zero.

Getters report effective stored values; getmax preserves the pinned API result even when it disagrees with defaults or Rust hard limits. A separate rfile capability/limit extension exposes safety maxima rather than altering legacy constants silently.

The C buffer API inspects the full supplied length independently of `bytes_max` in the pin. Keep the current Rust API's bounded sampling behavior, but implement an internal ABI buffer entry point that accepts up to the hard input cap (256 MiB), uses `bytes_max` for its legacy file/decompression purposes and exposes no false whole-buffer claim. Beyond the hard cap return Limit; do not silently truncate. All evaluation/memory/output budgets still apply. Test a tail signature after bytes_max in a valid borrowed buffer and distinguish it from a path-read sample.

## ABI-06: pointer/string ownership boundary

Use `core::ffi`/verified platform C types for int/char/size_t/ssize_t. Foreign callers must not mutate input memory while Rust holds an immutable borrow. Handle NULL+zero-length cases with a real empty slice; `from_raw_parts(NULL, 0)` is not valid Rust. Before making a Rust slice, enforce non-NULL where required, length <=isize::MAX, checked multiplication for pointer arrays and configured count/byte caps. For foreign C strings, the caller guarantees a readable NUL-terminated allocation; a bounded scan can impose a maximum accepted path length but cannot safely probe arbitrary unmapped memory. Parameter pointers require readable/writable aligned size_t storage. Document these preconditions in the header.

Returned result/error strings are NUL-terminated cookie-owned allocations. C strings cannot represent embedded NUL output: preserve the corresponding native string's first-NUL boundary in raw mode and retain length-aware bytes in the Rust API. Never call unchecked CString construction on bytes not proven valid for it. Error text and errno are stored separately; absence of an OS error does not erase a logical error message.

For `magic_getpath`, explicit input retains borrowed pointer identity. Constructed/default/environment results use thread-local owned buffers valid until the next same-thread getpath mutation; this avoids the original global mutable path cache race. Document that stronger independent-thread lifetime and preserve path bytes. Callers must still copy borrowed results they need longer. No global environment mutation or implicit sandbox installation occurs.

## ABI-07: descriptor and IO contracts

Use a reviewed borrowed-descriptor adapter; never construct an owning File from the caller's raw descriptor without correct ownership management. A temporary CLOEXEC duplicate is allowed only with an explicit understanding that dup shares the underlying seek position. Positional reads are preferred, but initial-offset and restoration behavior must still match `magic_descriptor`. Concurrent use of the same open file description is caller-coordinated; restoring one call's position is not a cross-thread transaction.

Test regular files at nonzero offsets, stdin, pipes, sockets/devices under flags, closed/negative descriptor errors and restore failures. Preserve metadata enrichment and mode differences from buffer input. Do not close stdin or the borrowed descriptor. C ABI path operations use native Unix bytes and the same race/bounded-special-file policy as the Rust path API, with explicit intentional differences recorded.

## ABI-08: errors, panics and non-unwinding exports

Implement every export as `extern "C"` with a complete panic boundary. On Rust 2024, use the explicit unsafe export attribute (`#[unsafe(no_mangle)]`) and mark raw-pointer entry points unsafe to call from Rust; a safe Rust signature must not let a Rust caller trigger UB through unchecked pointers. The FFI build requires unwind-capable Rust panic strategy; a panic-abort build cannot advertise catch-and-return containment. Convert a caught panic to NULL/-1, a stable emergency error message and EFAULT; mark that handle poisoned. After poisoning only error/errno and close remain guaranteed, preventing use of partially mutated state. Do not allocate during emergency error construction. Query/no-handle exports also contain initialization/TLS panics. Foreign C++ exceptions must not enter these exports; no C-unwind ABI is promised.

Recoverable reservation failure maps to ENOMEM and a recorded error; Rust allocator abort, double panic, process kill or actual foreign-pointer UB are outside catch_unwind's guarantee. The [Rust FFI guidance](https://doc.rust-lang.org/nomicon/ffi.html) is the boundary reference. A panic guard is not a substitute for proving unsafe pointer operations valid.

Ordinary mapping: preserve the underlying errno for IO; invalid IDs EINVAL; detectable NULL parameter pointers EFAULT; configured safety limits ERANGE; explicit unavailable capability ENOTSUP. Parser/format diagnostics use the pin's logical error-number behavior when no OS error applies. Do not simply expose the current Rust Error Display prefixes as supposedly compatible libmagic errors. Classification reset clears previous errors only at the source-equivalent point; querying or setting flags does not reset them. On POSIX, cookie `magic_errno` and calling-thread errno are different observables and both are tested.

## ABI-09: headers, symbols and package identity

Generate a reviewed header with all pinned constants/macros, 19 prototypes, opaque handle and C++ extern-C guards. A generation tool is optional; adding cbindgen requires its own version/license review and cannot be assumed to reproduce the legacy header. Compare preprocessed constants and C sizeof/signatures using a compiled client. Export only public magic_* and explicitly prefixed rfile_* extensions through a linker version script; hide Rust/internal symbols. Verify dynamic dependencies contain no libmagic or native detection codecs.

Default install prefix supplies `include/rfile/magic.h`, librfile_magic shared/static artifacts, `rfile-magic.pc`, license/notices, rules and capability manifest. An opt-in libmagic compatibility prefix supplies its conventional names after tests, without changing the machine's system package. Retain the correct SONAME and symlink chain; version script tags do not invent symbol versions unsupported by intended clients. Test actual loader behavior and C/C++ link commands. crates.io publication and global installation are separate explicit release decisions.

## ABI-10 through ABI-12: conformance and release gates

Compile one C test driver twice against the pinned C and Rust libraries. Run each in its own process with the same database/features/locale/TZ; capture stdout/stderr/status/signal, result/error bytes, thread errno, magic_errno, flags/parameters, input-buffer mutation and descriptor lifetime/position. Include every NULL case with a defined contract, unloaded calls, load success/failure/partial lists, buffer loads with source-memory release after return, error recovery, repeated operations and independent-cookie concurrency. Do not deliberately dereference arbitrary invalid pointers and call that a supported client scenario.

Use stateful property/fuzz tests with valid allocations and generated operation sequences; ASan/UBSan on applicable C clients/platform adapters and Miri on applicable Rust ownership code. Force a controlled Rust panic and prove no unwind reaches C; close/query of a poisoned handle must remain safe. Use nm/readelf/ldd and compiled headers to verify all 19 symbols/types, exported flags, SONAME and native linkage.

Integration clients: a minimal C classifier, C++ extern-C client, Python ctypes driver and a pinned representative python-magic version in an isolated environment. Do not install over the system libmagic to perform the test. Compare file/buffer/descriptor paths, getpath/discovery and compiler/check/list diagnostics, not only MIME on one string.

ABI-surface preview can ship with an explicit experimental capability level while detectors remain incomplete. **Drop-in behavioral compatibility** requires the CDF/ELF/codec/SEM gates, diagnostic/action/load-state closure, valid pointer/ownership conformance, supported-platform packaging and SEC review. Every intentional safety limit or profile difference remains visible in the compatibility matrix. No stubbed function, version number or successful link can substitute for those gates.
