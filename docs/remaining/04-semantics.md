# Locale, time, floating output and regex semantics

SEM-01 through SEM-10 close the semantic portion of the [parent specification](../07-remaining-work-spec.md). The goal is exact declared-profile output with bounded execution. Parsing a regex or displaying a date differently is not an equivalent successful classification.

## Evidence and immediate repair

Current `src/regex_engine.rs:45–54` uses byte-oriented regex-automata with octal syntax enabled. A magic source line containing two literal backslashes before `1` decodes to regex `(ab)\1`. With input `abab\n`, the pinned C CLI emits `match=abab, ASCII text`; current Rust emits `ASCII text`. Both exit zero. This later experiment is outside the old 10,875 custom-case total. The [probe report](../../reports/remaining-spec-observations.json) stores the exact source/input bytes, both outputs/statuses, and longest-alternation/collating-element controls. Preserve it as a new failing regression rather than claiming all unsupported syntax already errors.

Other integration points: `src/cli.rs:158` filename width, `src/format.rs:59/:70` escaping/capture printing, `:119` general float, `:505` dates, `src/encoding.rs:92` content encoding. Reference: `upstream/src/file.c:609/:689` filename printing/width; `src/funcs.c:746` C-locale regex compilation; `src/softmagic.c:2387` regex search-region truncation; `src/print.c:310` date conversion; `src/cdf_time.c:99` Windows timestamp calendar behavior. Content encoding detection and output locale are separate stages.

## SEM-01: immutable formatting configuration

Add `DisplayPolicy::{C, Utf8 { backend }, PlatformLocale { name }}` and a TimeContext holding an immutable checked timezone and provenance. Rust API defaults remain reproducible C display/UTC unless configured. CLI snapshots LC_CTYPE using the same environment selection as the reference (LC_ALL, LC_CTYPE, LANG); it must not apply LC_ALL wholesale to numeric/time formatting merely because a variable exists. The reference explicitly initializes LC_CTYPE, not every category.

Support exact C locale first, then C.UTF-8 and en_US.UTF-8 on certified Linux/glibc hosts. Other installed locales use the reviewed platform adapter and need their own certification; unknown locale names produce a clear configuration error rather than silently behaving as UTF8. An opt-in portable Unicode display policy is a distinct behavior, not advertised as libc-exact.

No engine operation calls process-global setlocale or changes environment variables. Where exact platform character behavior is required, `rfile-platform` wraps per-thread locale APIs: create/duplicate locale, bounded mbrtowc/iswprint/wcwidth operations, and RAII restoration of the prior thread locale. Per-thread locale guards are neither Send nor usable across async suspension. Handles are owned and freed exactly once; no untrusted input pointer escapes its bounded slice. Keep unsafe blocks out of the core. If a host lacks safe thread-local locale support, keep C/portable UTF8 and report unsupported platform-locale mode.

C ABI calls adopt the caller's current locale semantics through a per-call snapshot without changing global state. Concurrent external setlocale/TZ mutation is the caller's coordination responsibility, matching the native API's limitation; independent immutable Rust contexts remain Send+Sync. Tests change environment in isolated subprocesses, never through concurrent test threads.

## SEM-02: filename and output byte contracts

Preserve raw Unix filename bytes through argument/list processing and error text. Define separate operations for filename rendering, filename displayed width, library result escaping and bounded captured-string printing; one generic Unicode formatter cannot substitute for all four.

In C-locale wide-support builds, invalid multibyte bytes count as four display columns even in raw mode. ASCII nonprintable characters normally print octal and count four; in raw mode they print literally and count at least one. Valid wide printable characters use platform width, with nonpositive widths clamped according to the source. Reset decoder state after invalid sequences and consume one offending byte; preserve incomplete-sequence behavior at the end. Keep `-0/-00`, brief and alignment framing outside the escaping function.

The library's nonraw escaping and its 511-byte printable capture scratch retain their own limits and NUL handling. Test embedded newlines/NULs/backspaces and the sole three-byte keep-going separator quirk independently of filenames. Locale display must not change regex's C byte classes or reinterpret original MIME strings. Account for expanded octal/Unicode output against the final output cap before committing results.

Required vectors: ASCII controls; invalid UTF8 at each byte; truncated multibyte sequences; accented letters; combining marks; East Asian wide characters; emoji/variation selectors; zero-width characters; mixed valid/invalid names; raw/default/NUL/brief modes; multi-file alignment and LF-only filename lists. Compare against the same libc/Unicode-table version. Portable unicode-width-style tables can be useful but are not a verified substitute for that version's wcwidth; do not introduce them as an exact backend without a difference manifest.

## SEM-03 and SEM-04: dates, timezone loading and calendar behavior

Separate conversion from rendering:

```rust
enum TimeKind { UnixUtc, UnixLocal, WindowsUtc, DosDate, DosTime }
struct TimeContext { zone: CheckedZone, source: ZoneProvenance }
struct DateFields { year: i32, month: u8, day: u8, hour: u8,
    minute: u8, second: u8, weekday: u8 }
```

Raw numeric width, signedness, endianness and masks remain evaluator semantics. Convert date/ldate/qdate/qldate/Windows/DOS only after that stage. Local families use the configured zone; UTC families stay UTC. English asctime weekday/month names and spacing are fixed by the reference's renderer, not translated by LC_TIME. Preserve valid formatting at years -999..9999 where the pinned host's fixed asctime buffer permits it; larger/out-of-range values render the exact invalid marker or a declared profile error. Do not use a broad ISO8601 formatter as the human output.

Timezone selection for the CLI: empty TZ and `TZ=UTC` use built-in UTC without
requiring tzdata; other named TZ values resolve against the configured system
database; POSIX TZ expressions are parsed explicitly; a leading colon/file form
is loaded as checked TZif where supported. Absent TZ uses system local-zone data,
or UTC if the implicit `/etc/localtime` is absent (including a dangling symlink).
That specific default permits operation in minimal container images. An existing
malformed/unreadable system file, or any explicitly requested invalid/missing zone,
remains a setup error. Set a 1 MiB zone-file cap, bounded parsing/cache and path
policy before IO. Named-zone resolution must prevent path traversal outside the
configured database root; explicit TZ file paths are separate configuration
inputs. This fallback does not suppress explicit configuration errors.

Selected candidate is `jiff 0.2.38`, Unlicense OR MIT, manifest MSRV 1.70; inspected APIs include immutable TimeZone, IANA get, POSIX expression and TZif construction. Use explicit std/TZif support and controlled database loading; do not enable every default platform/bundled-data feature without evaluating it. Reuse identical TZif bytes for C and Rust comparisons when possible and record tzdata hashes. Candidate capabilities are documented in the [TimeZone API](https://docs.rs/jiff/0.2.38/jiff/tz/struct.TimeZone.html); conversion behavior still needs libc differential tests. Parse/load the chosen zone under the baseline sandbox and before final strict input confinement; do not require lazy zone-file IO after the final filter. Rust API contexts do not hot-reload; C ABI compatibility refreshes a changed coordinated TZ setting at call entry rather than installing a process-global TZ.

Windows FILETIME uses signed 64-bit 100 ns ticks in the pin. Pinned548 retains its bounded calendar normalization/month-boundary/pre-1601 quirks where deterministic; failed native conversion cannot authorize uninitialized `tv_sec`. CorrectedV1 uses checked ticks-to-Unix conversion with the 1601 epoch. CDF's editing-time threshold and duration formatting share this module but are different from rule Windows-date fields. DOS invalid-month substitution, zero day and two-second precision are tested as legacy formatting rather than normalized silently.

The semantic/date comparison reports identify the specific native early-return
branch separately: signed ticks at or above
`(10000 - 1601) * 365 * 86400 * 10_000_000` produce an approximate year greater
than 9999 in `src/cdf_time.c:cdf_timestamp_to_timespec`. That branch leaves
`ts.tv_sec` uninitialized, and `src/print.c:file_fmtdatetime` ignores the error.
Observed native bytes are retained but do not count as exact oracle comparisons,
even when they happen to match. Those cases instead require Rfile's exact invalid
datetime marker. Negative signed values and every other mismatch remain outside
this exception. This corrects a harness assumption exposed on Ubuntu 24.04;
it does not change production date semantics or patch the primary oracle.

Golden matrix: epoch/negative epoch, signed 32-bit boundaries including 2038, leap days and century boundaries, year-field limits, FILETIME zero/negative/maximum and month boundaries, DOS zero/invalid fields, UTC and named zones America/Los_Angeles/Europe/Berlin/Asia/Kolkata, a non-hour-offset zone, pre/post DST transitions and POSIX TZ strings. Timestamp-to-local conversion is unambiguous; test reverse calendar normalization separately where the legacy algorithm uses it. Record leap-second and TZif leap-table treatment as a named host contract, not assumed POSIX equivalence.

## SEM-05: floating formatting

Keep matching on exact IEEE values; signed zero and NaN sign are observable in output. A typed printf-compatible formatter handles f/e/g (and uppercase), precision, sign/space/alternate/zero padding and supported widths. Match libc exponent spelling, significant-digit selection, trailing-zero removal and rounding boundaries. Existing `general` is a starting point, not proof of full printf parity.

Use generated f32/f64 bit-pattern samples plus adversarial boundary values: subnormals, largest finite, halfway decimal rounding, exponent switch points for g, zero precision, signed zero, positive/negative infinity and NaNs with sign/payload differences. Compare type-checked format strings only. Do not call C printf with a magic description or pass untrusted format strings through FFI. CorrectedV1 does not invent different rounding unless explicitly versioned. Inspect any proposed decimal conversion dependency before adoption; no new candidate is asserted verified here.

## SEM-06: a real POSIX-ERE compatibility front end

Keep magic-source escape decoding separate from regex lexical decoding. Store original pattern bytes, source span, case/newline/range options, semantic profile and syntax dialect in the compiled-cache key. Detect escapes using an AST-aware lexer with bracket-class context; a string search for `\\1` is insufficient to distinguish an escaped backslash from a backreference.

For the GNU/libc dialect at the pin, recognized backreference syntax cannot be fed to an octal-enabled regular backend. Immediate R2 behavior is `UnsupportedRegex { construct, span }` before evaluation, propagated as a visible classification/load error, never a nonmatch. Diagnose nonexistent groups and ambiguous numeric escapes with oracle vectors; do not declare all libc numeric-escape syntax portable POSIX. The full capability requires SEM-08's bounded VM; rejection alone does not close it.

The lexer normalizes ordinary POSIX classes, ASCII case rules and supported collating/equivalence expressions before dispatch. In the mandatory C-regex locale, supported single-character collating/equivalence elements reduce to explicit byte predicates. Unsupported multicharacter or libc-specific constructs are explicit errors until implemented. Rust-only regex syntax must not be accidentally accepted as a GNU/POSIX extension because regex-automata happens to parse it. Maintain an allowlist of AST node forms and a dialect test table.

## SEM-07: regular subset and search window

Continue using regex-automata 0.4.18 for the proven regular subset with Unicode disabled, bounded automata and shared cache. REG_NEWLINE affects dot, negated classes and line anchors; ASCII ICASE applies to the complete regex, including bracket predicates. Preserve earliest start then longest **overall** match (`a|ab` on `ab` selects `ab`), independent of textual alternation order. Anchor behavior is relative to the correctly bounded search slice; `/s` changes continuation end to match start.

The pin drops the final byte of the bounded search region before regexec and stops at embedded NUL. Pinned548 explicitly constructs that same logical region without mutating input; a one-byte region, empty region, end anchor, final LF and NUL all have goldens. CorrectedV1 may use the complete bounded region only through this named profile; no invisible fix. Byte-range versus line-range limits are applied before regex evaluation in exactly the existing evaluator order.

Fix negated-class translation using parsed class semantics, including escaped closing brackets, leading literal `]`, POSIX classes and newlines. The current textual insertion approach must not rewrite a bracket construct it misidentified. Compilation/memory failures remain errors, not a switch to a different matching language.

## SEM-08: bounded backreferences

Implement a safe capture-aware interpreter for the extended subset, retaining the existing automata fast path for patterns without backreferences. Instructions include byte/class/any, split/jump, capture open/close, assertions, backreference compare and accept. States contain instruction, input position and capture ranges (unset versus empty are distinct). Charge every transition and compared byte, limit live states/capture groups (default 32), and use explicit heap queues/stacks.

Enumerate possible matches at the earliest viable start and select the longest overall end. Capture alternatives must remain distinct when a later backreference can observe them; deduplicating only `(instruction, position)` changes the language. Preserve the pinned dialect's quantified-group capture semantics and empty/unmatched-reference behavior through isolated libc differential vectors. Do not assume capture ranking is irrelevant merely because only the overall span is printed. Tie handling must not throw away a capture history that enables a longer final match.

Bound repetition expansion, compile AST depth and runtime epsilon cycles. On budget exhaustion return Limit, never accept the best partial result or silently reject a valid match. A construction with exponential alternatives is allowed to hit a stated limit; matching unbounded C runtime is not a requirement. No host regex FFI, oniguruma/PCRE native engine or subprocess fallback is permitted.

Required cases include `(ab)\1` against matching/nonmatching input; nonexistent groups; nested/optional/empty captures; repeated captured groups; alternation with equal overall spans but different later reference viability; case-insensitive references; classes versus escaped digits; raw NUL/final-byte truncation; nested bounded repetitions and deliberate state explosions. Include grammar-generated valid and invalid patterns, not only random characters that fail to compile.

## SEM-09 and SEM-10: acceptance and reporting

Deliver separate locale/time/float/regex result manifests and keep host-libc differences visible. All original C-locale/UTC results must remain exact except explicitly added profile changes. Add the known backreference mismatch to the mandatory suite and prove its initial explicit error and eventual correct match. Add parser/compiler roundtrips showing source and imported v21 patterns execute the same dialect.

Run shared-engine calls concurrently with distinct display/time configurations; results must not leak locale, timezone or captures between calls. Test locale-guard restoration and unwind/error cleanup in the platform crate. Fuzz formatting, timezone data, regex parsing and capture VM separately; cap external TZif bytes before parsing. SEM is complete only when the named locale/TZ/ERE/format matrix is exact or its remaining capability is explicitly unsupported; a generic ASCII result in place of an unimplemented regex match is a failure.

## R6a delivered regex contract and remaining fidelity boundary

`regex_dialect` parses bracket tokens and capture/backreference escapes separately from magic-source unescaping. C-regex classes use explicit byte predicates; negated classes exclude LF after ASCII case folding. Backslashes inside brackets are literal bytes, class-set-looking `&&` remains literal, and single-character collating/equivalence elements are supported. Outside brackets, GNU word/boundary/start/end escapes are recognized; unknown escapes quote the following byte, and `\0` is literal zero. Backreferences are the single digits 1–9 (a following zero is a separate literal), must refer to a syntactically closed capture, and cannot be replaced by octal syntax. Rust-only group extensions remain rejected. Capture groups are capped at 32.

Ordinary patterns retain regex-automata DFA/Thompson matching and compiler limits. Internally, marked backreference captures contain a nonempty sentinel so HIR optimization cannot collapse dynamic-reference repetition. The VM replaces the entire marked capture, never executing that sentinel; repeated and empty references have dedicated regressions. Backreference patterns compile normalized HIR into at most 65,536 instructions, with VM repetition expansion capped at 4,096. Instructions cover byte predicates, split/jump, capture boundaries, assertions, reference comparison and accept. Per-call heap states retain instruction, byte position and the complete capture history in their memo key. Unset versus empty captures are distinct; a skipped nested group preserves its previous participating value. All histories at the earliest viable start are explored before selecting the longest end. Epsilon cycles terminate through memoization and explicit limits. Every transition/comparison consumes shared work and the separate regex-step ceiling; live histories respect regex_states and conservative tracked-memory reservations. Exhaustion returns a limit, without accepting a best partial match. Captures never enter shared mutable state.

A standalone C regcomp/regexec control confirmed that host glibc 2.44 has observable quantified-backreference pruning and interval behaviors, beyond normal mathematical capture enumeration. Related primary reports include [glibc #25322](https://sourceware.org/pipermail/glibc-bugs/2019-December/046971.html) and [the repeated-capture discussion](https://sourceware.org/pipermail/glibc-bugs/2013-July/019069.html). These reports support the general risk; they do not prove every local discrepancy is the identical historical defect.

**Pinned548 therefore has an explicit conservative restriction**, rather than silently fixing these results: a referenced capture under a variable-count finite interval with upper bound above one, or a variable-width/reference-containing capture under a positive-minimum unbounded repetition, returns `Unsupported` during evaluation. For example `(a){0,2}\1`, `(a|aa)+\1` and `(a*)+\1` remain pinned-profile fidelity gaps. Fixed-width `+`, zero-minimum `*`, simple/nested/optional/empty captures and ordinary references have their tested support. CorrectedV1 explicitly enumerates all histories for the restricted patterns. The profile is selected at evaluation, so source and imported v21 databases share this boundary. This is partial GNU fidelity, not a declaration that the native history rules have been fully rebuilt.

An additional deterministic glibc quirk is preserved in the pinned front end: an unknown escaped lowercase letter with ICASE, such as `\d`, becomes a predicate matching no byte on this host, while escaped uppercase `\D` folds normally. A standalone C control tested all non-NUL byte values. CorrectedV1 treats unknown quoted letters as ordinary case-folded literals. Profile-specific compiled variants share the immutable cache but are separately counted in its memory budget. Capabilities report backreference support plus the explicit pinned restriction, group and instruction bounds.

Both profiles currently retain the pinned final-byte and embedded-NUL search-window behavior. Full-window corrected matching remains an unimplemented optional profile change. GNU constructs outside the described dialect can still produce explicit syntax/capability errors. Locale display, exceptional custom timezone/calendar cases and expanded float formatting remain separate SEM deliverables; this regex increment does not close those gates.

## R6b implemented locale and output contract

`src/display.rs::DisplayContext` provides portable C bytes or an immutable
`rfile_platform::Locale`. The CLI snapshots LC_ALL/LC_CTYPE/LANG in that precedence
and selects LC_CTYPE only; Rust API construction stays C/UTC unless configured.
`newlocale` acquires the object with a NULL base; its other categories stay C.
The private non-Send guard in `crates/rfile-platform/src/locale.rs` restores the
prior thread binding on success, error and unwind. No production `setlocale` or
environment writes occur. Guard borrowing/Arc ownership keeps acquired data live
across concurrent calls; shared/static C locale data is released according to
`freelocale` semantics, not an assumption of unique pointer addresses.

Names are capped at 1,024 bytes; adapter input at 16 MiB; output reserves a checked
four-byte-per-byte bound under the supplied cap before conversion. Host locale
data is trusted setup configuration, outside per-inspection reservations.
Unknown names and unsupported encodings fail clearly instead of silently using
UTF8; this deliberately differs from the native CLI's ignored setlocale failure.
Only ASCII/UTF8 codesets are currently accepted. Linux/glibc C, C.UTF-8 and
en_US.UTF-8 are compared; other native ABIs are not certified and reject locale
mode. The default C Rust policy remains portable without locale FFI.

The four distinct operations follow the source contracts at
`upstream/src/file.c:609` (`fname_print`), `:689` (`file_mbswidth`),
`src/funcs.c:562` (`file_getbuffer`) and `:866` (`file_printable`). A bad filename
sequence consumes one offending byte and resets conversion state; an incomplete
result sequence restarts escaping for the entire result. Raw filename width
still charges four for invalid bytes. Wide printable width is clamped to at
least one. Nonprintable wide filename characters retain the native low-byte
escape quirk; result escaping octalizes all original encoded bytes instead.
Capture scratch remains byte-oriented and capped at 511, independently of
multibyte final result rendering.

`Engine::check_output` now performs final escaping once after classification,
including compressed inner/outer output. `Evaluator::append` and `append_output`
respect the visible C-string prefix before subsequent prints, matching the
`%s%s` merge in `upstream/src/funcs.c:154–190`. A formatted NUL ends the returned
visible result even in raw mode; later appends can replace the hidden suffix.
Raw formatted bytes remain charged conservatively against safety limits.
Successful empty-format matches return empty bytes rather than an invented data
fallback; the native three-byte keep-going separator case is preserved.
`Error::Filesystem` exposes raw diagnostic bytes and the original OS source;
Display is a lossy UTF8 convenience. A 65,536-byte path cap is checked before
OS conversion/diagnostic copies; scratch reserves `4 * encoded_length + 1024`.
These are explicit resource boundaries rather than unbounded native behavior.

The [platform boundary](../../crates/rfile-platform/README.md) and
[unsafe inventory](../../reports/unsafe.json) record ownership/pointer/ABI
invariants. Native locale-data loading and glibc character conversion are not
reimplemented or certified memory-safe by these wrappers. Final-candidate ASan
smoke, concurrency and unwind controls supplement the exact locale differential;
independent review, sustained campaigns, other encodings and platforms remain
open. Date/float edge closure and full native quantified-regex histories are
separate SEM work.


## R6c reviewed implementation decisions (2026-10-25)

The formatter must not clamp accepted precision to 64. The existing 4,096
width/precision safety ceiling remains explicit; the pinned float path additionally
preserves native field validation and its 1,024-byte per-print output ceiling.
Native `%.1024f` calls the oversized field "width", whereas `%1.1024f` calls it
"precision"; both diagnostics are observable contracts. Overlarge generic safety
requests may fail before native emulation rather than allocate unbounded output.

Pinned548 preserves the native fmtcheck rejection of float `%G`/`%F` and float
length modifiers, including source-line and float/double call-site diagnostics.
CorrectedV1 permits uppercase f/g formatting. Source-validator acceptance is a
separate matrix from evaluating a type-correct patched v21 description. Matching
errors requires matching diagnostic bytes, not merely two absent results.
Alternate `%g` retains the measured native trailing-zero omission when rounding
crosses from fixed to scientific notation. Preserve exact IEEE values/signs;
no native printf call or float-to-decimal dependency is introduced.

For explicit POSIX TZ expressions, Pinned548 evaluates the two transitions in
the timestamp's UTC calendar year and retains the host's epoch-zero anchor for
years at/before 1970. CorrectedV1 uses Jiff's recurring calendar semantics.
The bounded internal representation is built only after Jiff validates the
expression. It handles fixed offsets and explicit J/n/M transition forms,
including signed transition times; ambiguous implicit DST rules remain visibly
unsupported through this constructor. Named TZif contexts retain their existing
checked tables and range policy. Standard offsets and DST flags are retained explicitly. Legacy FILETIME uses
bounded inverse iteration and the host's 601,200-second bidirectional standard-
time probes (229,222,804-second exclusive distance bound), then its one-hour
fallback. This preserves unusual half-hour perpetual-DST cases that do not
reduce to simply subtracting the standard offset. Host mktime cache-dependent
ambiguities remain an explicit investigation rather than shared mutable state.

Observed host behavior is checked against the pinned file build on glibc 2.44.
The explanatory libc source is [GNU tzset.c, compute_change and __tz_compute](https://github.com/bminor/glibc/blob/master/time/tzset.c);
that moving source is corroboration, not a substitute for the recorded native
oracle. Extreme custom TZif/footer and host-specific rounding/locale behavior
remain separate fidelity gates; these repairs do not certify every libc/platform.
