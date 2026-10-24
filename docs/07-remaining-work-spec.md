# Remaining fidelity, isolation and C ABI: implementation specification

Status: **proposed implementation contract; no detector, codec, sandbox or ABI implementation is included in this change.** Prepared 2026-10-24 against rfile commit `668933f6c4edbd091ce173e230dfa780d3dcb175` and upstream file 5.48 commit `41b1123083ab0bbdfe32f6d4ec14f6aec9a91e59`. Existing delivered behavior is recorded in [implementation status](06-implementation-status.md). This specification refines the [original rebuild specification](02-rust-specification.md); it does not mark any remaining feature complete.

The requested seven areas retain their original numbers: CDF (1), ELF (2), decompression (3), locale/date/regex (4), sandboxing (6), hardening (7), C ABI (8). Other-platform support (the omitted item 5) is a cross-cutting verification dependency, not an additional platform implementation project here.

## Detailed contracts

| Request / requirements | Detailed specification | Implementation destination | Completion gate |
|---|---|---|---|
| 1 / CDF-* | [Compound documents](remaining/01-cdf.md) | `src/cdf/`, common input reader | property/stream matrix, pinned and corrected outcomes, hostile chains |
| 2 / ELF-* | [Executables and core dumps](remaining/02-elf.md) | `src/elf/`, note tables | ELF32/64 × both byte orders × note/core families, exact output order |
| 3 / CMP-* | [Compression](remaining/03-compression.md) | `src/compression/`, feature manifest | bounded decoding for each declared codec; input/error/member matrix |
| 4 / SEM-* | [Locale, dates and regex](remaining/04-semantics.md) | `src/display/`, `src/time/`, `src/regex/`, platform adapter | known silent backreference mismatch closed; locale/TZ cases exact or declared |
| 6 / SBX-* | [Linux sandbox](remaining/05-sandbox.md) | `crates/rfile-platform`, CLI lifecycle | permitted operations succeed; denied operations proved in subprocesses |
| 7 / SEC-* | [Fuzzing and security](remaining/06-hardening.md) | `fuzz/`, CI, campaign manifests | sustained campaigns, invariant tests, resource and unsafe audits |
| 8 / ABI-* | [C ABI](remaining/07-c-abi.md) | `crates/rfile-ffi`, `include/`, packaging | all 19 public functions, headers, state/lifetime/error and client tests |

Immutable source browser: [pinned upstream tree](https://github.com/file/file/tree/41b1123083ab0bbdfe32f6d4ec14f6aec9a91e59/src) and [Rust baseline](https://github.com/JLuethe/rfile/tree/668933f6c4edbd091ce173e230dfa780d3dcb175/src). All source line references in the detailed documents refer to these revisions.

MUST/MUST NOT are acceptance requirements. SHOULD requires a recorded reason to deviate. Each implementation PR names its requirement IDs, tests, evidence and outstanding gaps.

## Baseline and evidence rules

**Verified source:** current CDF is prefix-only and omits summary rendering (`src/cdf.rs:107`, `:265`); current ELF enrichment is file-backed but has a subset of note families (`src/elf.rs:80`, `:168`); only gzip/zlib decode (`src/lib.rs:200`); regex translation enables octal (`src/regex_engine.rs:45–54`); date output has no timezone argument (`src/format.rs:505`); there is no platform sandbox or FFI crate. The current `Engine`, `Flags`, `Limits` and owned `Classification` are the integration points, not a replacement C library.

**New observations, outside previous passing totals:** [research observations](../reports/remaining-spec-observations.json) preserve a complete synthetic CFB input as hex, its hash, flags and both outputs. The pinned C says `Composite Document File V2 Document, Cannot read section info`; current Rust omits the suffix. MIME agrees. Its summary includes codepage 1252 and application `Microsoft Word`, independently read with olefile 0.47 after section-length/alignment validation. The temporary validator is analysis tooling, not a product dependency. Source review finds section bounds computed from stack-local `sh` instead of stream-local `shp` in `upstream/src/cdf.c:1034` and `:1047`. This is a **suspected upstream defect**, not an upstream-confirmed finding or proof that every document takes that path. The backreference example in the semantics document is another observed discrepancy. Do not include either in old all-pass denominators.

Evidence classes are **D** documented format/API contract, **V** source-verified implementation, **O** executed observation, **I** inference, **Q** unanswered. Register every disagreement as `legacy-defined`, `rust-defect`, `reference-suspected-defect`, `intentional-safety-difference`, `feature-mismatch` or `unresolved`. A reference crash/uninitialized result is never a deterministic expected answer.

The primary oracle stays unmodified at the pinned commit and database SHA-256 `8eab57da91fd9cc4c7af3b8a4369a64d32124aa55dc376207a81e69abd4a5ac8`. An auxiliary corrected C build is allowed only under a distinct identity with checked-in patch, patch hash, compiler/features and fixture manifest. It cannot replace primary results or inflate exact-match counts. Source/table extraction must retain original notices; current rfile contributions remain GPL-3.0-only.

## Binding compatibility decisions

1. Retain safe legacy ordering, formatting and parser quirks. Add `SemanticProfile::{Pinned548, CorrectedV1}`, default `Pinned548`, with CLI `--profile=pinned-5.48|corrected-v1` and a Rust builder option. This labels intended CDF/date/decoding corrections; it is not permission to approximate every difference. Both profiles always enforce safety limits and reject undefined states. `Pinned548` means pinned detection/formatting within the declared platform/feature set; previously documented native-decoder/sandbox differences remain exceptions.
2. CDF corrected metadata must be implemented, even when the pinned reader rejects it. The pinned profile preserves the observed fallback state through safe logic; the corrected profile exposes the checked metadata. A table in the CDF document defines the differences. Never intentionally calculate invalid pointer comparisons.
3. Regex backreference-looking syntax must stop silently becoming octal. Explicit rejection is the first repair; bounded compatible execution is required before claiming the backreference capability. Host libc regex and libmagic FFI are prohibited as parsing engines.
4. Native Rust codecs stay in process; helpers and detection subprocesses remain prohibited. A codec disabled or not implemented reports capability failure, not a fictitious classification of its contents. Decoder errors and output-cap handling are typed and profile-aware.
5. Linux isolation is CLI-only by default. No ordinary Rust or C classification call installs process/thread restrictions or changes the caller's process locale/environment. No kernel feature is claimed merely because code compiled.
6. C ABI is explicitly in scope. A separate audited crate exports the pinned functions. It uses the same Rust engine and does not link libmagic. Default installation uses a distinct library identity; substituting `libmagic.so.1` is an explicit deployment option after the compatibility gate.
7. No system-wide installation, automatic database update, removal of existing reports, or stable release promotion is implied by implementing these documents. Experimental releases can continue with explicit capability/gap lists.

## Shared input, budget and error model (CORE-01 through CORE-05)

Replace format-specific arbitrary reads with a safe, bounded positional abstraction:

```rust
trait InputView {
    fn known_len(&self) -> Option<u64>;
    fn read_at(&mut self, offset: u64, out: &mut [u8], budget: &mut Budget)
        -> Result<usize>;
}
struct SliceView<'a> { bytes: &'a [u8] }
// FileView uses safe positional IO; it borrows, never closes, the caller's file.
struct InspectionContext<'a> {
    input: &'a mut dyn InputView,
    budget: Budget,
    profile: SemanticProfile,
    display: DisplayPolicy,
    time: TimeContext,
    capabilities: &'a Capabilities,
}
```

CORE-01: Preserve API distinctions. Buffers cannot read beyond their bytes. Paths/seekable descriptors may read metadata and bounded random-access regions; readers/pipes remain prefix-only unless an explicitly requested bounded spool is introduced. No implicit spool is planned. The borrowed-descriptor initial prefix begins at its current position as in C; do not silently seek it to zero or rebase every subsequent format-absolute read. Cover nonzero-position quirks with explicit goldens. Restore borrowed seek positions on success and error; prefer positional reads to avoid disturbing them. Obtain file size from the opened descriptor, not an earlier path stat. A changing/truncated file returns a bounded short read, never unchecked zero-filled structural evidence.

CORE-02: Charge every format child, codec member, regex VM branch and random-access read to the same classification budget. Independent nested budgets must not reset the work ceiling. Reserve aggregate owned bytes before allocation; release on drop. Cache bytes have a separately enforced database-wide ceiling. Use checked u64 file offsets and checked usize conversion, including on 32-bit targets.

| Proposed additional resource | Default | Hard configuration maximum | Counting rule |
|---|---:|---:|---|
| aggregate owned inspection memory | 128 MiB | 512 MiB | input/tail copies, decoded streams, tables, codec scratch, transient regex state; shared immutable DB/cache separately measured |
| cumulative positional bytes read | 128 MiB | 512 MiB | every read including repeat/overlap; avoids sparse-file seek amplification |
| positional read requests | 65,536 | 1,000,000 | zero-length requests still charge a step |
| codec dictionary/window memory | 64 MiB | 256 MiB | reject before constructing decoder; includes allocation multipliers in aggregate reservation |
| codec members/frames | 64 | 1,024 | chained members and skippable frames count; nesting remains one layer |
| CDF chain/directory/property entries | 250,000 / 65,536 / 10,000 | same | each visited link, parsed record and vector-expanded property counts |
| regex VM transitions / live states | 1,000,000 / 65,536 | 10,000,000 / 262,144 | cumulative; state includes capture history where needed |
| timezone bytes / zone cache | 1 MiB each / 8 MiB | 4 MiB / 32 MiB | checked before reading external TZif; no unbounded global cache |
| cancellation/deadline polls | every 4 KiB IO or 1,024 interpreter steps | maximum interval fixed | cooperative only; a blocked syscall/opaque decoder requires process supervision |

Existing defaults stay: 7 MiB input/expanded-output cap, 64 KiB encoding, 1 MiB output, 1,000,000 rule steps, 128 MiB byte work, named/indirect defaults 150/50 and hard 150, ELF limits, bounded regex compilation/cache. The 128 MiB aggregate budget may reject combinations that individually fit old caps; record that as a safety difference. Total process RSS includes allocator/runtime/dependency overhead and is not guaranteed by this accounting; supervisors enforce process-wide ceilings.

CORE-03: distinguish `NoMatch`, `Match { evidence, completeness }`, `Malformed { stage, offset }`, `Unsupported { capability }`, `Limit { resource, limit, observed }`, `Io`, and `Cancelled`. Format corruption can be rendered as upstream's ordinary diagnostic classification when its contract requires it; limits/cancellation are never silently downgraded to `data`. Rendering and final escaping charge output budget. Evidence order must be preserved, not sorted/deduplicated after detection.

CORE-04: `Capabilities` reports engine/rules/format/profile, compiled and enabled codecs, regex subsets, display/time backend, sandbox compiled/available/enforced state and C ABI compatibility level. Keep actual kernel state distinct from build support. Existing output APIs remain authoritative; add structured evidence without fabricating MIME by extension.

CORE-05: reorganize only when implementing: `src/cdf/{reader,properties,render}`, `src/elf/{headers,notes,core,capabilities,render}`, `src/compression/{registry,limits,codecs}`, `src/{display,time,regex}`, plus `crates/rfile-platform` and `crates/rfile-ffi`. Root becomes a Cargo workspace while retaining the current package/API entry points. The core keeps `#![forbid(unsafe_code)]`; unsafe exceptions are confined to platform/FFI adapters with a written invariant per block. Third-party unsafe/FFI is inventoried, not hidden by crate boundaries.

## Delivery sequence and release gates

| Milestone | Depends on | Deliverable | Acceptance |
|---|---|---|---|
| R0 discrepancy harness | baseline | additional CDF/regex vectors, per-stage coverage/exception manifest | current misses reproduced; untouched old results retained; all comparison errors counted |
| R1 shared contracts | R0 | common input/budget/errors/profile/capabilities | descriptor restoration, sparse/short reads, aggregate reservations and nested budget tests |
| R2 immediate semantic repair | R1 | explicit backreference detection, local dates, C-locale regressions | no silent octal interpretation; rejected capability is visible; TZ tests isolated |
| R3 CDF closure | R1, time/display subset | complete chain/property/catalog/type pipeline and profile renderers | CDF-* matrix; corrected fixture truth independently validated |
| R4 ELF closure | R1, display subset | core/AUXV/OS/capability table coverage | ELF-* branch matrix; both classes/endians; stable evidence order |
| R5 codecs | R1 | bzip2, XZ/LZMA/lzip, Zstd, LZ4; legacy formats staged | each enabled feature passes its corpus and cap/error contracts before default enablement |
| R6 full semantic closure | R2 | bounded backreference VM, C/UTF8 display and date/float edge fidelity | SEM-* matrix; host-specific libc results identified; unsupported locales explicit |
| R7 Linux isolation | R1, codec/time initialization design | platform adapter and classification/compile sandbox policies | SBX-* positive/negative subprocess tests; no parser before required-policy success |
| R8 C ABI implementation | R1, semantic/API state contract | standalone shared/static library, pinned headers and all functions | ABI-* client/state/ownership suite; no libmagic dependency; packaging remains distinct |
| R9 integrated hardening | every changed subsystem | fuzz campaigns, sanitizer/Miri/limits review and findings ledger | SEC-* evidence; no unresolved critical/high findings; independent review recorded |
| R10 compatibility release | R3–R9 plus named gaps closed | reproducible source/binary/ABI packages and expanded compatibility matrix | zero unclassified disagreement in declared profile; every capability gate evidenced |

R3/R4/R5 can be developed independently after R1; R9 starts with R0 and repeats after material changes rather than waiting until the end. R8 can ship experimentally before full detector fidelity, but cannot advertise drop-in libmagic equivalence. Estimates are intentionally not invented; work is sized by closed branch/test inventories. Optional/legacy codecs remain explicit gaps until their separate gate passes.

## Verification artifacts and reporting

Extend, do not replace, `tools/verify.py` with component manifests and isolated jobs. Reports carry Rust SHA, primary/auxiliary oracle identity, rules hash, build features, limits, locale/TZ/tzdata/libc/kernel/architecture, API/flags/profile, input generator/hash, stdout/stderr hex, status/signal, error state and output bytes. Timeout/crash/missing output is not equality, including when both implementations fail. Publish unique-input counts separately from comparison counts and group by stage/API/feature; thousands of repeated ASCII files cannot establish CDF/core coverage.

Golden data is immutable and attributable. Each new mismatch has a minimal reproducer, classification and owner/closure criterion. Run all existing 103 goldens and 44,419 API/semantic comparisons on the recorded host, then the new matrices; changing intentional-difference rules requires review. Benchmarks interleave engines, separate startup/cold/warm paths, include adversarial bounded workloads, record median/p95/p99 over enough samples (at least 100 for p99) and RSS/cgroup peaks. Provisional gate: <=2× the pinned C warm time and RSS on equivalent supported features, or an approved profile with safety/performance rationale; correctness and finite limits outrank that goal.

## Remaining decisions are explicit gates

Host-libc note-layout and locale differences, legacy codec readiness, actual big-endian/32-bit execution, tzdata versioning, GPL distribution of future C-linked applications, and codec internal cancellation require the investigations named in their detailed documents. No unresolved question can be converted into an undocumented fallback or an implemented-capability claim. [Candidate dependency evidence](../reports/remaining-candidate-dependencies.json) records inspected versions/features/licenses; none was added to Cargo.toml by this documentation change.
