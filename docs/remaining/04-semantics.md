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

Timezone selection for the CLI: empty TZ means UTC; named TZ resolves against the configured system database; POSIX TZ expressions are parsed explicitly; a leading colon/file form is loaded as checked TZif where supported; absent TZ uses system local-zone data. Set a 1 MiB zone-file cap, bounded parsing/cache and path policy before IO. Named-zone resolution must prevent path traversal outside the configured database root; explicit TZ file paths are a separate allowed configuration input. Invalid/unavailable configuration is a clear setup error with provenance, not an unnoticed UTC substitution. Any host-native fallback that the pin performs differently is a documented configuration difference.

Selected candidate is `jiff 0.2.38`, Unlicense OR MIT, manifest MSRV 1.70; inspected APIs include immutable TimeZone, IANA get, POSIX expression and TZif construction. Use explicit std/TZif support and controlled database loading; do not enable every default platform/bundled-data feature without evaluating it. Reuse identical TZif bytes for C and Rust comparisons when possible and record tzdata hashes. Candidate capabilities are documented in the [TimeZone API](https://docs.rs/jiff/0.2.38/jiff/tz/struct.TimeZone.html); conversion behavior still needs libc differential tests. Parse/load the chosen zone under the baseline sandbox and before final strict input confinement; do not require lazy zone-file IO after the final filter. Rust API contexts do not hot-reload; C ABI compatibility refreshes a changed coordinated TZ setting at call entry rather than installing a process-global TZ.

Windows FILETIME uses signed 64-bit 100 ns ticks in the pin. Pinned548 retains its bounded calendar normalization/month-boundary/pre-1601 quirks where deterministic; failed native conversion cannot authorize uninitialized `tv_sec`. CorrectedV1 uses checked ticks-to-Unix conversion with the 1601 epoch. CDF's editing-time threshold and duration formatting share this module but are different from rule Windows-date fields. DOS invalid-month substitution, zero day and two-second precision are tested as legacy formatting rather than normalized silently.

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
