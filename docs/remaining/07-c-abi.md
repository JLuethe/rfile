# C ABI: independent libmagic-compatible interface

ABI-01 through ABI-12 specify the compatibility library requested in the [parent plan](../07-remaining-work-spec.md). The mainline baseline exposed Rust APIs only; the R8 branch now contains an experimental C interface as recorded below. This is a new interface around the independent Rust engine, not a binding to the original C detector.

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

Generate a reviewed header with all pinned constants/macros, 19 prototypes, opaque handle and C++ extern-C guards. A generation tool is optional; adding cbindgen requires its own version/license review and cannot be assumed to reproduce the legacy header. Compare preprocessed constants and C sizeof/signatures using a compiled client. Export only public magic_* and explicitly prefixed rfile_* extensions through the Rust cdylib linker export allowlist (or a reviewed platform version script); hide Rust/internal symbols. Verify dynamic dependencies contain no libmagic or native detection codecs.

Default install prefix supplies `include/rfile/magic.h`, librfile_magic shared/static artifacts, `rfile-magic.pc`, license/notices, rules and capability manifest. An opt-in libmagic compatibility prefix supplies its conventional names after tests, without changing the machine's system package. Retain the correct SONAME and symlink chain; version script tags do not invent symbol versions unsupported by intended clients. Test actual loader behavior and C/C++ link commands. crates.io publication and global installation are separate explicit release decisions.

## ABI-10 through ABI-12: conformance and release gates

Compile one C test driver twice against the pinned C and Rust libraries. Run each in its own process with the same database/features/locale/TZ; capture stdout/stderr/status/signal, result/error bytes, thread errno, magic_errno, flags/parameters, input-buffer mutation and descriptor lifetime/position. Include every NULL case with a defined contract, unloaded calls, load success/failure/partial lists, buffer loads with source-memory release after return, error recovery, repeated operations and independent-cookie concurrency. Do not deliberately dereference arbitrary invalid pointers and call that a supported client scenario.

Use stateful property/fuzz tests with valid allocations and generated operation sequences; ASan/UBSan on applicable C clients/platform adapters and Miri on applicable Rust ownership code. Force a controlled Rust panic and prove no unwind reaches C; close/query of a poisoned handle must remain safe. Use nm/readelf/ldd and compiled headers to verify all 19 symbols/types, exported flags, SONAME and native linkage.

Integration clients: a minimal C classifier, C++ extern-C client, Python ctypes driver and a pinned representative python-magic version in an isolated environment. Do not install over the system libmagic to perform the test. Compare file/buffer/descriptor paths, getpath/discovery and compiler/check/list diagnostics, not only MIME on one string.

ABI-surface preview can ship with an explicit experimental capability level while detectors remain incomplete. **Drop-in behavioral compatibility** requires the CDF/ELF/codec/SEM gates, diagnostic/action/load-state closure, valid pointer/ownership conformance, supported-platform packaging and SEC review. Every intentional safety limit or profile difference remains visible in the compatibility matrix. No stubbed function, version number or successful link can substitute for those gates.


## R8 execution refinements from source tracing (2026-10-25)

The clean reference is selected through `RFILE_REFERENCE`; see the R7 reference
audit. The experimental ABI preview merged in PR #16 at
`59c7c384241046833de904dafd2440eac53bfc37`. The following source-tracing
notes describe that preview; the R8b update below supersedes its diagnostic counts.

- `apprentice_list` (apprentice.c:1163) prints each set's Binary/Text headings,
  filters by the root mode and selects the first nonempty description and MIME
  independently from the continuation group. Strings use C's first-NUL boundary;
  invalid UTF-8 is preserved as bytes. Each successful path in a list action
  prints the accumulated loaded lists again. Merging every path and printing once
  is not equivalent. Initial pure-renderer evidence matches all five observed
  database/source listing bodies; source discovery stderr is still a gap.
- `file_mdump` (print.c:49) runs during parsing before MIME annotations and final
  sorting. Check output is not a post-load strength listing. Its type/mask/string
  modifier rendering includes observable oddities: a trailing shift in LE
  varints and omitted trim/full-word letters. The bounded dump primitive matches
  221 valid synthetic programs and all 361 selected source fragments. This
  separates the known source-discovery warning from the dump; malformed parser
  warnings and complete action state remain unimplemented requirements.
- `apprentice_1` (apprentice.c:477–515) prefers compiled mapping for check/list;
  source checking emits its header only when source parsing occurs. Compile
  returns before attaching parsed lists: it can leave allocated empty lists,
  whereas successful load/check/list attach parsed data. Verify subsequent
  classification separately for each action, rather than assuming compile loads
  its compiled rules. The earlier broad statement that all actions may leave
  parsed lists must be interpreted with this distinction.
- `file_magwarn` (print.c:275) increments before comparing the cap. The warning
  reaching the cap prints suppression notices instead of its original message;
  `load_1` (apprentice.c:1350) stops subsequent parsing once that count reaches
  the cap. The counter is initialized with the cookie (apprentice.c:568), not
  cleared by `file_reset` (funcs.c:537). Warning state therefore persists across
  operations. Test cap zero/one, repeated loads, and changing the parameter.
- Regular-source list observations confirm the second discovery warning carries
  the previous file and next physical line (`a, 2` after one record). Rule
  provenance line numbers and warning/current physical lines are distinct;
  missing final newline must not collapse them into one counter. Owned Rust
  context replaces dangling source-name storage; undefined native pointer output
  is not a byte-for-byte contract.
- Empty path components terminate the native path-list loop rather than merely
  being skipped. A first empty component fails; a trailing empty component can
  follow successful loads. Preserve partial success/error/cookie state separately.

Diagnostic construction must be checked before growth. The new primitive's cap
is caller-selected up to 16 MiB and source size remains capped at 128 MiB; it
returns completed dump records alongside a parse/limit error. These bounded
primitive APIs do not imply completed loader diagnostics or foreign-pointer ABI
contracts. No C export or system-library replacement exists at this checkpoint.


## R8 preview implementation and refined gates (2026-10-25)

`crates/rfile-ffi` now implements all 19 entry points plus the implementation
extension. This is an experimental surface preview; R8 is **not complete**.
The safe engine remains independent of the native detector. The cdylib export
allowlist was verified with nm/readelf: 20 symbols, SONAME librfile_magic.so.1,
no libmagic dependency. No manual second linker version script is needed on the
initial Rust/Linux toolchain because rustc supplies the export allowlist.

Safe snapshots now parse directory fragments once with global source sorting;
check dumps preserve parse order. ABI buffer classification bypasses only file
sampling, keeping the 256 MiB hard cap and execution/output budgets. The cookie
adapter has a panic boundary and poisoned state; detectable NULL/alignment/size
errors are checked before slices; caller synchronization still owns pointer
lifetimes. A temporary CLOEXEC descriptor duplicate preserves ownership and
seekable cursor restoration. Default flags, native parameter widths, early zero
buffer-load state and compiled-header error precedence have explicit tests.

Database source/compiled reads require a regular file, opened nonblocking before
checking its metadata. This intentionally excludes FIFO/device database sources.
Aggregate action reads are capped at 128 MiB and action output at 16 MiB.
Compilation uses exclusive same-directory staging, mode 0644 subject to umask and
atomic replacement; destination inode/symlink semantics deliberately differ from
C O_TRUNC. The output directory must be trusted. Getpath's configured data file
is real in the isolated package; embedded CLI fallback is not substituted.

The native `apprentice_buf` stores MAP_TYPE_USER pointers (apprentice.c:3306–3325);
its buffers must stay alive until unloading/close. The common C test now honors
that lifetime. An initial harness incorrectly freed native memory early and
failed; it was a harness contract violation, not a newly established upstream
vulnerability. Only Rust tests the stronger copy-and-own guarantee by freeing
source bytes immediately after return. This distinction also applies to Python
ctypes probes: keep their allocations live through close for the native oracle.

Current comparison covers 49 whole action/state scenarios, 43 exact and six
frozen malformed-source differences. Each disagreement records both full byte
channels and state; `data/abi-known-gaps.json` freezes exact observed payloads so
changed failures cannot silently inherit an allowlist label. Six C/C++ client
runs (native/Rust, including C ASan/UBSan) pass. Client sanitization does not
instrument Rust allocations. A separate address-sanitized stateful ABI target
uses only valid owned foreign allocations; it does not generate arbitrary
pointers or host paths. Successful header diagnostics do not mean arbitrary
compiled records accepted by C must bypass Rust structural validation.

Next ABI gates, in dependency order:

1. Add typed source diagnostic events with physical source positions, warning
   counters, native continue-after-error ordering and annotation/error taxonomy.
   Preserve existing strict Rust parsing; do not translate arbitrary Display
   strings or reparse prefixes quadratically to fake a diagnostic stream.
2. Expand state fixtures to multiple failures, malformed directory members,
   cap boundaries mid-file, no-final-newline lines, non-UTF8 provenance,
   conditional rules, invalid format annotations and source/compiled fallback.
   Resolve the six frozen cases with the shared renderer; remove their baseline
   entries only after reference evidence and ordinary regressions pass.
3. Complete IO failure and special-file state cases, errno/locale wording and
   DEBUG behavior decisions. Keep unsupported/native-only behavior explicit.
4. Validate isolated prefix packaging, pkg-config/default discovery, static and
   dynamic clients, all feature combinations and a pinned python-magic consumer.
5. Run the final same-candidate full matrix, stateful sanitizer campaign and
   applicable ownership/Miri tests; retain independent review as a separate gate.

The first step is a parser/diagnostic milestone, not a reason to claim the whole
ABI complete because the exported function count is correct.

## R8b source diagnostic contract and remaining actions

The typed parser-event increment implements `Database::source_action` with raw
byte source names, physical line counters, parse-order dumps, separately counted
warnings and cookie errors, and continue-after-known-error behavior. The strict
Rust constructors retain their first-error contract. No Display-string parsing
or repeated prefix parsing is used. `SourceActionOptions` bounds diagnostics to
16 MiB; action snapshots are bounded to 128 MiB/4,096 fragments. Output limits
and unmapped diagnostics remain explicit Rust errors, not claimed native matches.

Native warning counters survive successive actions. A warning increments the
counter before testing its cap; reaching the cap prints both suppression lines,
and parsing stops before the next physical line. Cookie errors neither increment
the counter nor print as warnings; the first error is retained. Source action
initialization resets the source filename, but preserves the last diagnostic
line. Classification can replace that line with the last actually visited magic
record. Root fast-rejection and continuation-skipping order therefore matter.
`LegacyInspection` conveys this observation without changing public metrics or
sharing mutable state between independent inspections.

Forward continuation-level jumps are accepted by the native source-action and
v21 import routes, with a source warning. Strict Rust source constructors retain
their existing rejection. Before evaluating a shallower sibling, all deeper
match states are cleared: a later jump cannot reuse a previous branch's match.
The new regression checks source and compiled routes against the pinned oracle.
Pascal strings whose stored length including their prefix exceeds the 128-byte
v21 value field are explicitly rejected. This safety bound prevents successful
compilation of a record the safe importer cannot accept; it is not native parity.

Verification: all 49 original action/state scenarios now match; the six frozen
diagnostic differences were removed after replay. A second matrix has 420 exact
scenarios (21 inputs, four actions, five warning limits), each with three repeated
action/classification cycles. It includes CRLF, NUL, missing newline, raw names,
multiple failures, annotation errors and continuation warnings. These counts
cover the named fixtures, not every malformed source grammar production.

Next work remains concrete:

1. Reuse the source-action pipeline for CLI check/compile/list; CLI `-c` still
   uses its old post-load listing. Preserve compiled preference, path-list partial
   success, discovery warnings, stdout/stderr/status and sandbox setup ordering.
2. Extend typed events for accepted truncation/escape warnings, format annotations,
   conditional-rule errors and malformed directory members. Record newly found
   disagreements separately before changing acceptance or expected bytes.
3. Complete IO failure/special-file action state, errno/locale and DEBUG decisions.
   Native-only behavior must be explicit; broader source acceptance must still
   compile/import safely and obey aggregate limits.
4. Rebuild isolated ABI packaging after the final candidate, then run the full
   same-candidate matrix, sustained ownership/parser campaigns and independent
   review. PR #16's passing package is evidence for that earlier candidate only.


## R8c CLI action integration

`src/cli_database.rs` uses the same typed source parser and native compiled-header
validation as the C ABI. Check output is now generated in parse order; a compiled
neighbor takes precedence for check/list. List output repeats the accumulated
sets after each successful path. Empty path components terminate processing;
missing or malformed members can coexist with successful members, as in the pin.
Raw cookie errors are written as bytes rather than lossy UTF-8.

Source discovery warnings retain the previous attempted filename/line. A failed
source open still produces the check header; this behavior is shared with the
ABI. Native CLI database actions create a new CHECK cookie but never call
`applyparam` (file.c:400–443), so `-P magwarn=0` deliberately does not suppress
their warnings. The native default cap remains 64, independent of classification
parameters. These quirks are verified, not inferred fixes.

Action input reads are regular-file-only, aggregate 128 MiB; output is bounded
to 16 MiB, path lists to 16 nonempty components and directories to 4,096 files /
16,384 entries. Linux opens nonblocking before checking handle metadata, so a
FIFO is rejected without waiting. Compilation retains atomic publication through
the existing sandbox adapter. Quota violations abort even after partial success;
this stronger bounded-resource policy is an intentional difference.

Focused evidence: 243 exact CLI scenarios in off/auto/strict sandbox modes,
including invalid headers, directories, path-list partial success, warning caps,
source/compiled preference and ignored action parameters. Only the diagnostic
program-name prefix `file:` is normalized to `rfile:`. Compilation artifact
hashes and all other stdout/stderr/status bytes are compared. Four ordinary Rust
regressions preserve core action contracts; FIFO rejection is a separate safety
check, not counted as native conformance. ABI diagnostics now cover 440 exact
scenarios, including repeated missing-source actions.

Remaining action gaps include default database discovery without `-m` (the CLI
still explicitly requires it for database actions), DEBUG tracing, source-load
warnings in ordinary classification, IO errors after an earlier directory member,
accepted truncation/escape/format warnings and broader conditional grammar.
Directory source bytes are snapshotted before parsing: a racing later read error
can prevent earlier dump records, unlike the native incremental file reader.
These are still open and are not hidden by the passing action matrix.
