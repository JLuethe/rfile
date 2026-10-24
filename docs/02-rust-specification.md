# Rust rebuild specification

Status: implementation contract written before Rust implementation, with explicitly recorded corrections from subsequent oracle experiments. Current delivered scope and evidence are in [implementation status](06-implementation-status.md). Baseline and evidence taxonomy: [architecture](01-architecture.md). Deviations must be recorded in [compatibility](03-compatibility.md) and executable regression tests. No libmagic binding, subprocess delegation to `file`, C translation, or linking the detection engine is permitted.

## Objectives, scope and platforms

Provide independent Rust CLI `rfile` and library `rfile` for classification of paths, byte buffers and readers using upstream magic rules. Memory safety and finite resource consumption outrank matching undefined behavior. Exact output is a stated compatibility target, not “same general file type.” Preserve human descriptions, MIME/charset, ordering and error channels where covered. The executable remains named `rfile` until release gates justify an optional `file` alias.

Initial supported platform is x86_64 Linux with byte-oriented Unix paths. Portable safe parser/evaluator must also compile on macOS, BSD and Windows; those are tier 2 until CI and metadata tests run there. Native endian magic deliberately follows target endian, independently of the byte order of the database that stores it. Big-endian interoperability requires emulated CI. Exotic Amiga/OS2-specific application metadata is out of first-release scope. No archival extraction, content trust decision, antivirus verdict, full format validation, network fetch, or recursive directory traversal is implied by a classification API.

Default build input is upstream's selected 361 files, not every tracked fragment: `etf` is retained in `data/extras` because upstream excludes it.

Stages: (A) checked v21 database loading, primitive evaluator, bounded API/CLI; (B) full source grammar and compilation, continuations/names/search/regex/annotations, text, metadata, gzip; (C) remaining structured detectors, optional codecs, platform/sandbox fidelity; (D) C ABI and distribution stabilization. A working stage A/B binary must not claim stage C parity.

## Package and API structure

Use one Cargo package with library and thin binary first; extract crates only when boundaries justify it. Modules: `database` (validated immutable rules and v21 codec), `parser` (source lexer, directives, grouping/ranking), `eval` (bounded state machine), `format` (typed printf subset/date/GUID), `encoding` (byte classification and UTF conversion), `detectors` (tar/JSON/CSV/SIMH and later ELF/CDF/DER), `filesystem` (path metadata and bounded IO), `limits`, `error`, `cli`/`main`. Add `tools/` differential, corpus, benchmark and upstream-refresh tools, `tests/` semantic integration cases, `fuzz/` isolated fuzz package, `data/` pinned rule asset and notice bundle, `docs/` specifications, `reports/` machine-readable results.

```rust
pub struct Database { /* immutable validated ordinary/named rule sets */ }
pub struct Engine { /* Arc<Database>, Flags, Limits; no global mutable state */ }
pub struct Classification {
    pub description: String,
    pub mime_type: String,
    pub encoding: String,
    // extension and Apple annotations, diagnostics, evidence/truncation
}
pub enum Error {
    Io(std::io::Error), Database(String), Parse { line: usize, message: String },
    Limit(&'static str), Unsupported(String),
}
impl Database {
    pub fn from_compiled(bytes: &[u8]) -> Result<Self, Error>;
    pub fn from_source(source: &str) -> Result<Self, Error>;
    pub fn load(path: &std::path::Path) -> Result<Self, Error>;
    pub fn compile(&self) -> Result<Vec<u8>, Error>;
}
impl Engine {
    pub fn new(database: std::sync::Arc<Database>) -> Self;
    pub fn classify(&self, data: &[u8]) -> Result<Classification, Error>;
    pub fn classify_path(&self, path: &std::path::Path) -> Result<Classification, Error>;
}
```

Owned return values are stable across calls; no C-style invalidated strings. `Database` and `Engine` are `Send + Sync`; per-call mutable continuation state, counters, output and visited-sector sets are local. Regex caches may use immutable precompiled values or `OnceLock<Result<...>>`, never non-thread-safe cells. The delivered internal `Rule` keeps validated wire IDs and bytes rather than a fully typed AST enum; this preserves exact v21 records and is an explicit implementation choice. Store flags as explicit `u32` constants matching upstream; don't permit unknown flags silently in the eventual C ABI. A builder can validate parameter limits without making public structs layout-stable. Changing configuration requires `&mut self` or constructing a new engine. Rust errors are typed; ordinary absence of a match is `Ok(data)`, while limit exhaustion is an error and must not silently turn into a confident fallback.

Core representation: `Rule` carries level, direct/indirect offset expression, signedness, type, relation, value, numeric operation, range/modifiers, description format tokens, MIME/Apple/extension annotations, strength modifier, source span. `Group` holds contiguous preorder nodes rooted at level zero. `Database` contains ordered ordinary groups, named-group index, source identity, database version. Values are enums (`Integer(u64)`, `Float32`, `Float64`, `Bytes`, `Guid`) rather than pointer casts. Offset expressions use checked signed-wide intermediates and explicit `Option<usize>` bounds. Skip failed root literal groups before allocating continuation state; fixed 256-entry continuation arrays are local to each evaluation frame. Preserve original record bytes for v21 import/roundtrip where useful; never use `repr(C)` deserialization.

## Source language: normative parsing rules

1. Parse bytes in the source's ASCII control syntax; UTF8 descriptions can be retained losslessly as bytes internally where needed. Unix LF and CRLF accepted. Blank/comment lines are ignored; `#` is a comment marker at start of a logical line, not arbitrary inline stripping. Use upstream's LF-only trim and line-number behavior, including the final unterminated-line quirk. Apply limits before allocating a line/field. On Unix upstream strips LF only; Rust additionally accepts source CRLF as an explicit input extension. File-list paths retain CR/spaces.
2. A rule is `('>'*) offset whitespace type whitespace test [whitespace description]`. Preorder level is count of leading `>` (v21 stores u8; reject >255). First rule must be level zero; a skipped level is at least a warning and strict mode rejects it. Store continuations under the most recent group. No whitespace-tokenizing the whole line: escaped spaces belong to test strings; remainder after test is description.
3. `!:mime`, `!:apple`, `!:ext` annotate the immediately previous rule; `!:strength OP N` applies to its group's root (verify duplicate directive error policy). Unknown/orphan directive is a parse error. MIME must fit 79 bytes plus NUL; desc 63 plus NUL; ext 119 plus NUL; Apple up to eight bytes. Preserve upstream truncation warnings in compatibility mode, reject in strict mode. Strings store <=127 decoded bytes; `pstring` encoded length additionally accounts for length prefix.
4. Parse C integer forms: optional sign, `0x` hex, leading zero octal, decimal, accepted `uUlL` suffixes. Width-specific sign extension determines equality operands. Check literal overflow. Float values support decimal/scientific, Inf/NaN as upstream strtof/strtod permits; masks on floats require separate tests. Types support `u` numeric prefix and SUS aliases (`dC/d1`, `dS/d2`, `dI/dL/d4`, `dQ/d8`; unsigned equivalents; `s`). Default signed unless `u`.
5. Relations `= ! > < & ^`; absent relation means `=`. Standalone `x` is wildcard and consumes no value. `&` requires `(actual & expected)==expected`; `^` requires `(actual & expected)!=expected` (not necessarily every bit clear). `<=`/`>=` produce warnings/errors under the checked parser, not inclusive comparisons. Historic `&=`/`^=`/`==` aliases skip second equals. Negated tests on unreadable data need the evaluator's explicit no-value behavior below.
6. Decode test strings using C escapes `a b f n r t v`, escaped whitespace/relations/backslash, up to three octal digits (truncate to byte), `x` plus up to two hex digits (`\x` with no digits becomes `x`). Unknown escapes drop slash and can warn; regex is decoded by this same layer, so literal regex `\.` usually needs doubled backslash in source. Embedded NUL participates in length-based literal matching. Description retains its escapes except leading actual backspace or `\b`, which sets NOSPACE and is removed.
7. Numeric type suffix can specify one operation from `& | ^ + - * / %` plus C integer operand; optional `~` inverts after operation. Zero operand means no operation in the C implementation. Keep this quirk explicitly. Integer math wraps at data width where C unsigned arithmetic does, signed checks must be simulated without Rust UB; divide/modulo zero or overflow gives controlled failure, never panic. Date mask/add behavior must follow `cvt_*`, not a generalized date arithmetic assumption.
8. String/search/regex suffixes use `/` separators with a numeric range and character flags in either order. `b/t` select binary/text, `c/C` fold only lower/upper pattern letters respectively, `W` needs each pattern whitespace to consume >=1 target whitespace (last run position may consume more), `w` consumes zero or more, `T` trims displayed value, `f` requires end or whitespace immediately after match. `s` makes search/regex continuation relative to match start. pstring `B/H/h/L/l` select 1 BE, 2 BE, 2 LE, 4 BE, 4 LE length; `J` subtracts length-prefix width from declared length. Regex `l` counts lines rather than bytes, with implicit 80 bytes per requested line and global regex cap. `indirect/r` uses relative indirect classification.

Types and wire IDs are frozen by `src/file.h:268–330`: integer byte (1), short (2), long (4), quad (24), endian variants (7,8,10,11,23,25,26); date families (6,9,12,14–16,21,22,27–32,42–44); strings (5,13,18,19), regex 17, search 20; IEEE floats 33–38; ID3 39/40; indirect 41; name/use 45/46; clear 47; DER 48; GUID 49–51; offset 52; varint 53/54; MSDOS date/time 55–60; octal 61; default 3. Maintain an explicit ID-to-type table rather than range assumptions (many ranges mix kinds). All parsed types must either execute or yield an explicit unsupported error at the affected path; silently dropping rules is forbidden in full compatibility mode.

## Group ranking and scheduling

Source input is split into ordinary and named sets. Directory input visits nonhidden regular fragments sorted bytewise. Compile groups as units, preserving child order. Binary/text classification: numeric/DER/GUID/offset/octal set root BINTEST; regular string/pstring/string16 default binary; regex/search containing textual UTF8 become text unless overridden. At this pin `set_text_binary` tags from the root; do not infer the tag from every continuation. Named roots bypass ordinary root dispatch checks, and children execute in the caller's pass. The broader manual description is not the exact implementation.

Strength starts at 20. Add width*10 for numeric/date/GUID, decoded length*10 for string/pstring/octal, length*5 for string16, `len * max(10/len,1)` for search, same using nonmagic regex literal count (minimum 1) for regex, 10 for DER, zero for other control types. Then relation: wildcard/negation resets to zero; `=` adds 10; `>`/`<` subtract 20; `&`/`^` subtract 10. Apply root strength factor `+ - * / N`, clamp <=0 to 1, add 1 for empty root description. Default roots start at zero before the common clamp/factor/empty-description handling. `set_last_default` only warns if a default is not last; it does not reorder. Regex nonmagic count: escaped char one, `?*.+^$` zero, bracket class one, brace repetition zero, other bytes one. Descending strength; ties descending bytewise root v21 record with line number zeroed (on source host byte order), including hidden bytes after empty description NUL. This architecture-dependent quirk must be retained for exact source-compiler parity or replaced only in a named portable mode. Named entries retain independent groups and lookup index; duplicate name policy must be tested.

## Offsets and matching state machine

Offset grammar is `[&] [(' [ & ])] [+-] integer [('.'|',') indtype] [~] [op [ '(' ] signed_integer [')']] [')']`, with parentheses only for indirect addressing. The leading ampersand outside parentheses adds parent's end to the **resolved** offset; the ampersand inside parentheses adds parent's end to the **location read to resolve** it. Either relative form at level zero is rejected. Store these as separate booleans. Dot means unsigned indirect value; comma means signed. Default indirect type is native long. Suffix mapping: b/B/c/C byte; h/s LE short, H/S BE short; l LE long, L BE long; m PDP long; q/Q LE/BE quad; i/I LE/BE ID3; o octal; e/f/g and E/F/G parse as LE/BE double although evaluator support at this pin is suspect (see risks).

A leading minus marks EOF-relative magnitude, including `-0`. For memory buffers use actual buffer length (implementation supports it even though manual is more restrictive); for fd-backed regular files use bounded tail window read at file size minus prefix capacity. Leading `+` resets to start instead of inheriting tail-window context. Continuations without explicit plus remain in the current beginning/tail context. Add named-call base to direct reads; indirect pointer reads follow original addressing context. Bounds are checked before each read and each arithmetic step. Preserve distinction between no value, mismatch and fatal evaluator error.

Resolve indirection by reading the typed integer at address, optionally reading another operand from address+constant for nested operand parentheses, applying operation and inversion, then parent-relative adjustment if flagged. ID3 indirection decodes four synchsafe 7-bit groups; header-length adjustments belong to rule arithmetic, not an implicit evaluator addition; test it separately. Reject resolved offsets outside valid unsigned-32-bit compatibility domain before converting to usize. Byte/short/long/date32/float/double/GUID reads require their width even on `x`. The pinned `mget` width switch omits quad/date64/Windows-date and DOS-date/time types: copy available bytes into a zero-filled typed-width scratch value, even past EOF, then convert and compare normally. This safe padding is observable compatibility, not an out-of-bounds memory read. `offset` synthesizes current position. Short strings emulate upstream's bounded 128-byte zero-filled scratch representation where matching uses padding, but no read crosses slice bounds.

Evaluator pseudocode per group:

```text
level_state = {parent_succeeded, end_offset, got_sibling_match} for each depth
for rule in preorder:
  if parent did not succeed: skip its subtree
  resolve offset, fetch typed value, transform, compare
  unreadable checked-width value: ordinary comparison fails; ! succeeds with raw native scratch, without mask/endian conversion
  default succeeds only if no sibling has matched at this depth
  clear succeeds and clears sibling-match flag
  on success: set sibling state, render/record annotation, record end offset
  evaluate children; process next sibling independently
if group emitted evidence: stop unless keep-going
```

Sibling match state resets on entering children, not on every sibling. When a checked-width fetch fails, a `!` rule succeeds without comparison, endian conversion or mask application: numeric display uses the zero-padded native-endian scratch bytes. This path is separate from a padded quad comparison that legitimately returns false. Skip children when the resulting parent end exceeds available data; preserve emitted parent text. Parent's end offset is type-sensitive: numeric position+width, except Windows-date types whose pinned `moffset` default is zero; strings `=`/`!` position+stored vallen, other string relations position+bounded displayed string length (pstring includes prefix); search match start+pattern length minus named base, regex match start+actual match length minus named base; `/s` omits match length; use/default/clear/offset retain current position; GUID adds 16; DER moves inside TLV according to its rule length. Treat the implementation's named base subtraction as explicit arithmetic, not interchangeable absolute offsets.

`name` defines a callable group, does not match ordinary input on its own. `use \^name` in source (stored value `^name`) toggles endian for called numeric/string16 reads and nested calls. Save and restore continuation/tail/output-separator context across call; propagate whether any description/annotation was found. Successful prior evidence can affect name/use truth at this pin. Name limit is active call-depth counting (increment and decrement); indirect classification limit is a cumulative counter in a classification. `indirect` re-runs ordinary binary rules on a slice beginning at resolved position; disallow zero progress and enforce independent counters.

## Value and formatting contracts

Signed comparisons sign-extend at the field width to i64; unsigned compare u64. Equality uses transformed width value. Float NaN never equals or orders, and does satisfy `!`. Strings compare unsigned target-minus-pattern bytes over specified pattern length, including NUL. String16 extracts the low byte of each two-byte unit, substitutes space for low-zero/high-nonzero; this is not full UTF16 decoding (separate from text conversion). Pascal length is bounded to scratch capacity and prefix subtraction underflow is a mismatch/error.

Literal search attempts positions within range; fast memmem path can include the boundary differently from flagged path (`softmagic.c:2330`): add a differential boundary fixture and preserve compatibility explicitly. Regex uses extended POSIX syntax, C locale, REG_NEWLINE, optional ICASE. It truncates the final byte of its bounded search region before passing a C string to regexec, also stops at embedded NUL (`softmagic.c:2387`). Preserve this surprising last-byte behavior in compatibility mode. POSIX earliest-start/longest-overall-match selection is implemented with an earliest-start automaton followed by anchored all-match selection. Bounded repetitions use a Thompson VM to avoid DFA explosion. POSIX backreferences, collating elements and libc-specific syntax outside the supported regular subset remain explicit gaps; subcapture ranking is irrelevant because magic prints only the overall span. Compile errors must have source span/rule identity. Current compatibility limitation: with octal syntax enabled, a backreference-looking escape can be accepted as octal instead of rejected. A rule `(ab)\1` matches `abab` in the pinned C engine but currently misses in Rust. Explicit rejection or compatible bounded evaluation remains required; do not claim every unsupported construct is diagnosed.

Implement a safe format parser; never pass attacker-controlled descriptions to C printf. Parse literals/`%%`, flags `#0- +`, width, precision, supported length modifiers `h/hh/l/ll/j/z/t`, and conversions `c d i u o x X s e E f F g G`. At most the upstream-supported one value conversion per rule; reject `%n`, pointer, dynamic `*` widths, invalid type conversions. Apply field-width sign extension before print, and permit numeric `%s` by decimal string conversion as upstream does. Clamp width and output length; parse description variable expansion `${...}` separately (initial release may report unsupported rather than approximate). Exact libc `%g`, floating rounding and local-time rendering require golden vectors.

For string `=`/`!`, `%s` renders pattern, not captured input. Other relations render up to 127 bytes from input ending at NUL (if empty pattern, stop at CR/LF). Search renders the upstream search cursor semantics (the current code retains region start for search), regex renders matched span. Escape nonprintable bytes to octal unless raw, with default compatibility locale pinned C. Descriptions join with one space except NOSPACE. Human keep-going separator newline-`- ` is passed through escaping later. MIME/Apple/extensions select annotations with precedence in `handle_annotation`; human matching and annotation matching may stop at different children.

Dates use POSIX seconds UTC; ldate families local TZ; Windows timestamps use 100ns ticks since 1601 converted to Unix; DOS dates decode 1980+year, month/day, time hour/minute/2-second. Match numerical underlying values. Use calendar arithmetic with checked range and upstream maximum year 9999. UTC default tests use TZ=UTC, but local-time compatibility is a separate platform gate.

## Builtin and filesystem specification

Pipeline order is normative in architecture document. Implement encoding table and ordered BOM/UTF checks, not a generic “valid UTF8 = text” shortcut. Preserve original charset for MIME while text rules see normalized UTF8. Decode at most encoding limit, handle trailing incomplete UTF8 like upstream, and append line-ending/long-line/escape/backspace descriptors in exactly upstream order. At length 1 skip all magic. Empty regular path has metadata MIME `inode/x-empty`, while empty buffer has `application/x-empty`: keep entry points distinct.

Tar validates the 512-byte header and octal checksum with the observed unsigned-byte sum, then ustar variant; retain the GPKG exclusion. A signed-byte alternative would be a semantic change. CSV heuristic mirrors upstream, rather than importing a fully validating CSV library. JSON should use a dedicated bounded recognizer to preserve prefix/scalar/NDJSON heuristics: serde_json is suitable only as a declared interim approximation or test utility. SIMH traverses LE length/trailer frames, mark values and padding with finite record count. DER uses checked TLV tag and length parsing, table-driven names, length qualifiers, child offset and data formatting. CDF uses bounded SAT/mini-SAT traversal with visited-sector detection, directory names, root stream and summary/document-summary properties; fail safely on cycles/overflow. ELF reads class+endian headers, limited program/section tables and notes; enrich human description only on seekable file entry points and in the correct position. Do not guess a CDF subtype from a header alone.

Path IO must inspect metadata before reading and never accidentally block on named pipes/devices in default mode. Default no-follow on Unix unless POSIXLY_CORRECT; symlink text includes target and broken-link distinction. Preserve raw OsString bytes in argument processing/output. Open with nonblocking/cloexec where platform supports it through safe OS wrappers; resolve metadata/open races with fd metadata and bounded reads. Borrowed reader API consumes at most requested limit; seekable API restores position and exposes EOF windows separately. Atime restoration is opt-in and must report platform support accurately.

## Compiled databases, distribution and updating

Initial format is **upstream v21 `.mgc` import and export**; reject unsupported versions with actionable error. Header is one 432-byte record: u32 magic `0xf11e041c`, u32 version 21, u32 ordinary count, u32 named count; rest zero on export. Endianness detected from magic. Records start at byte 432; checked `(1+ordinary+named)*432 == len`. Record offsets: u16 flags @0; u8 level @2, factor @3, relation @4, vallen @5, type @6, in_type @7; in_op @8, mask_op @9, conditional byte @10, factor_op @11; i32 offset @12, in_offset @16; u32 line @20; mask u64 or range u32+flags u32 @24; value 128 bytes @32; desc64 @160; MIME80 @224; Apple8 @304; ext120 @312. Numeric union values follow database host order; string bytes never swap; GUID swapping must follow upstream `swap.c`. File native types still follow executing host endian. Validate strings, types, relations, group structure and counts before evaluation. Avoid memory-mapping mutable untrusted files.

Use deterministic little-endian v21 export with zero initialization; document that native-C tie sort and embedded debug filenames can prevent byte-for-byte source compilation equivalence until normalized. Source compilation is byte-identical to the pinned C compiler when run against the same selected files under the same relative directory name (`magic`); a Rust-only asset rebuild is available. No new native cache format initially: versioned v21 suffices. Future portable cache must have distinct magic, schema/semantic versions, length table/checksum, provenance hashes, unknown-feature rejection; never label it `.mgc` version21.

Vendor upstream rules unmodified, with full `COPYING`, attribution and any fragment-local notices; pin commit and SHA-256 in `data/UPSTREAM.json`. Runtime may load externally supplied rules; shipped asset must work without source checkout, C compiler or C library. The shipped v21 asset is now generated by Rust and verified byte-for-byte against the pinned upstream build. `tools/rebuild_database.py` copies vendored inputs under `magic/` to retain exact hidden path provenance and enforces the pinned SHA-256 before writing. Refresh script fetches an explicit commit into staging, verifies expected revision, archives rules/notices, enumerates new syntax, compiles and runs differential suite before replacing data. Never auto-update at runtime. Database release can be separate from engine SemVer; report engine version, rule commit and compiled format in `--version`. Preserve upstream contribution attribution and do not automatically submit changes upstream.

Licensing: root COPYING is a two-clause BSD-style grant with a distinctive source-notice placement requirement (`COPYING:10–17`). Preserve exact text and source-file notices rather than substituting a generic SPDX license. CDF/DER and fallback regex carry individual notices; fuzz Docker/build metadata is Apache-2.0. The initial local implementation used BSD-2-Clause; the owner selected GPL-3.0-only for publication. Current rfile contributions use GPL-3.0-only; behavior-adapted files should retain applicable upstream copyright headers and a notice bundle. Fixtures may carry independent rights; external corpora must have redistribution review and avoid bundling private system files. The notice audit is an engineering release gate; this specification does not claim a legal opinion.

## Dependency and safety policy

Verified capabilities from primary crate documentation and downloaded metadata must be locked in Cargo.lock; record resolved versions and licenses in `reports/dependencies.json`. Recommendations:

| Candidate | Capability/license | Decision/tradeoff |
|---|---|---|
| `regex-automata` 0.4.18 | byte DFA/Thompson VM, explicit match modes; MIT OR Apache-2.0 | selected backend; two-pass overall POSIX selection, bounded construction, per-database 64 MiB cache and work charging. Regular syntax only; no backreference/locale collation parity. |
| `flate2` with explicit `miniz_oxide` | gzip/zlib/deflate; MIT OR Apache-2.0 | safe Rust backend; disable default features to prevent a backend switch or native-zlib unification. Bounded streaming output and one layer. No external command fallback. |
| `serde_json` | JSON/report serialization; MIT OR Apache-2.0 | used by test/benchmark drivers for reports. Product classification uses a dedicated recognizer matching C heuristics. |
| std | IO, Arc, sync, checked endian conversion | no dependency for magic parsing/formatting; no unchecked casts. |
| additional codecs/ELF/CDF crates | not selected or license-approved yet | do not claim verified recommendations until source/license/API inspection. Prefer bounded pure Rust parsers; reject hidden extraction/allocation and FFI defaults. |

Primary references: [regex byte API](https://docs.rs/regex/latest/regex/bytes/index.html), [flate2 backend/features](https://docs.rs/flate2/latest/flate2/), [serde_json API](https://docs.rs/serde_json/latest/serde_json/). Documentation was inspected 2026-10-23; lockfile is authoritative for installed versions. `regex-automata` arbitrary byte matching disables Unicode and UTF8 restrictions. Its safe public APIs are used; the dependency internally uses unsafe code, distinct from first-party parsing. [Automaton API](https://docs.rs/regex-automata/latest/regex_automata/dfa/trait.Automaton.html) documents matching and safe transition contracts. `flate2` documentation distinguishes miniz safe code from faster unsafe/native alternatives; select explicit backend.

Project crates use `#![forbid(unsafe_code)]`. No libmagic FFI anywhere. Later optional C ABI is isolated in its own crate: unsafe permitted solely for pointer/length validation, copying owned output, handle lifetime and panic containment; it never implements parsing. OS sandbox wrappers can use a reviewed safe dependency or narrowly scoped audited adapter; feature disabled until platform tests pass. Transitive unsafe in standard library/dependencies is separate from first-party parser policy and must be inventoried. No claim that Rust alone prevents logical or resource-exhaustion vulnerabilities.

## Limits and hostile input

| Resource | Compatibility default | Rust safety contract |
|---|---:|---|
| input prefix/tail | 7 MiB each | hard configurable cap <=256 MiB each; read limit before allocation |
| encoding bytes | 64 KiB | <=input cap; checked UTF expansion <=4x characters |
| database bytes/rules | unspecified in C | 128 MiB / 250,000 records; source aggregate same cap; directory <=4096 regular source files/16384 entries; CLI lists <=16 databases and merged <=250,000 rules |
| source line/pattern | bounded compiled fields | 64 KiB line, 127 decoded pattern bytes for v21 |
| continuation depth | u8 wire | <=255; fixed continuation arrays, bounded native call stack |
| name/indirect | 150 / 50 | defaults preserved; hard maximum 150 for either setting. Fixed continuation arrays plus bounded call recursion are tested on a 2 MiB thread stack; lifting the cap requires an explicit heap call stack. |
| regex | 8192 haystack | compiled automaton size <=4 MiB each, total cache budget; unsupported syntax diagnosable |
| rule steps | no global C cap | 1,000,000 evaluated nodes/call with cumulative budget across recursion |
| comparisons/search bytes | no global C cap | 128 MiB aggregate byte work/call; prevent rule_count*input uncontrolled amplification |
| output | internal C growth | 1 MiB; format widths <=4096 and bounded annotation sizes |
| decompression | bytes_max; one layer | <=input cap expanded, <=1 layer by default, stop codec on limit; no arbitrary helpers |
| ELF | 256 notes,2048 phdrs,32768 shdrs,128 MiB section | checked multiplication/seeks, cumulative bytes budget |
| CDF/DER/JSON | implementation-specific | visited-sector tracking, nodes/work limit, JSON depth<=500, DER depth<=128 |
| execution | caller cancellation absent | deterministic work budget; optional deadline/cancellation checked every bounded chunk |

Expose truncation in structured API results (an input cap is normal sampling, not proof the complete file was parsed). A work/output/decompression limit error is distinguishable from mismatch. Refuse absurd parameter requests before allocating. OOM termination by Rust allocator is still possible within process-wide contention; OS process limits remain the way to enforce strict aggregate memory ceilings. Tests must include sparse files, symlink races, recursive use, deeply nested JSON, enormous regex repetition and compressed bombs.

## Packaging and release gates

Ship Cargo.lock, build/install instructions, license bundle, pinned database/provenance and CLI manpage. `cargo build --release --locked` must not require upstream C or network after vendoring dependencies. Build script never compiles C or regenerates rules with libmagic. Default database search uses explicit path > MAGIC > home override > installed/embedded pinned database; embedded fallback is an intentional packaging change and is reported in version output. A strict reproduction invocation always supplies `-m`.

Use 0.x versions until all stated compatibility gates pass. No replacement of system `/usr/bin/file`, no package publication or system installation during development. Release archives for tested targets include hashes, reproducible feature manifests and SBOM. Distinguish basic engine completion from full upstream replacement. Upgrade database versions only behind explicit codec support and golden import/export vectors. Optional C ABI requires symbol/header/version/error/ownership tests before advertising libmagic compatibility.

## Implementation refinements and explicit semantic differences

- Keep database-list boundaries in the in-memory grouping: annotation return can stop the current database while `-k` proceeds to the next. Share separator/space/first-line state through named calls in human keep-going mode. Do not normalize away the resulting apparently redundant separators.
- Final output limits include octal escaping and builtin/metadata fallbacks. String capture printing honors raw mode and upstream's 511-byte printable scratch limit. A changed-to-special-file race after open is rejected before reading when devices were not requested.
- Source diagnostics are currently strict errors and a Rust strength listing; `-c`/`-l` and all warning/truncation text are not yet byte-compatible. CLI parameter parsing rejects nonnumeric/prefix-only values rather than reproducing `atoi` quirks. Debug traces identify Rust rules instead of reproducing C trace text.
- Gzip/zlib decode in process and do not reproduce the native-decoder/no-fork guard failure. Other recognized codecs return an explicit unsupported error when inspection is requested. CRC/truncation failures can be stricter than C's raw zlib path. No kernel sandbox is advertised.
- The buffer API samples at `Limits.bytes` and exposes truncation; C `magic_buffer` can inspect its entire borrowed buffer independently of the file-read limit. Differential buffer runs supply the same bounded prefix to both. Reader truncation is conservative when it exactly reaches the cap.
- Human output is authoritative in `Classification.output`; auxiliary description/annotation fields reflect the requested evaluation pass. Request MIME flags for authoritative MIME output. Returned owned strings/bytes are an intentional Rust API difference.
- Little-endian Linux source sort is proven exact. The portable serializer uses explicit little-endian fields and type-aware value decoding; opposite-endian float/GUID/DER import is not certified against upstream's generic eight-byte union swap. Do not claim full big-endian compatibility.

- Truncated quad/date64/DOS numeric reads use explicit zero padding; checked-width negative tests preserve raw native scratch display. Little-endian string16 retains a final odd low byte; its zero low/nonzero high maps to space. Signed NaN spelling is preserved. Windows dates reproduce bounded C calendar normalization, but conversion failure returns deterministic invalid text rather than consuming uninitialized state (`cdf_time.c:99–150`, `print.c:310–321`). Date formatting has the observed asctime four-character year field limit.
- A keep-going buffer consisting solely of the three-byte `\n- ` separator is not trimmed: `funcs.c` tests length against `sizeof` including NUL. Encoding-only output can therefore begin with that escaped separator.
