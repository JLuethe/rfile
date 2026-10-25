# Delivered Rust implementation: status and evidence

Version **0.1.0** is built and runnable in this workspace. The architecture/specification was committed first as `fc385f1`; implementation followed. The final user instruction authorized the build after the spec, superseding the earlier instruction to stop at specification.

This is an independent Rust detector, source parser/compiler, CLI and library. Production code neither links libmagic nor invokes `file` or decompression helpers. The C checkout, ctypes harness and small C benchmark driver are test oracles only. No system utility was replaced or globally installed. The project is published at https://github.com/JLuethe/rfile as an experimental GPL-3.0-only source release; no crates.io package is published.

## Run the delivered build

```sh
cargo build --release --locked
./target/release/rfile --version
./target/release/rfile README.md
./target/release/rfile -i /usr/bin/ls
printf 'hello\n' | ./target/release/rfile -b -
cargo test --locked
```

On this machine Cargo is `/home/john/.cargo/bin/cargo`. The release binary is `target/release/rfile`; rules are embedded. Normal builds do not require the gitignored `upstream/` directory or a C compiler. See the [README](../README.md) for the Rust API and [manual](rfile.1) for CLI usage.

## What is implemented

| Area | Delivered behavior / source |
|---|---|
| Database | Checked little/big-endian v21 loading, little-endian export, source directories/lists, ordering and named indices; [database.rs](../src/database.rs), [parser.rs](../src/parser.rs). Rust recompilation of the 361 selected source inputs produces exactly the upstream database bytes. |
| Evaluator | Integer/endian/mask/relation rules, continuations, default/clear, offsets/indirection/tail reads, names and endian flip, strings/pstrings/string16, searches, GNU byte-ERE normalization, bounded backreference histories and overall longest matching, annotations and strength ranking; [eval.rs](../src/eval.rs), [regex_engine.rs](../src/regex_engine.rs), [format.rs](../src/format.rs). |
| Detection | Ordered text/encoding recognizers, EBCDIC conversion, tar, JSON/NDJSON, CSV, SIMH, DER, gzip/zlib and opt-in additional codecs, checked CDF property/catalog metadata, expanded ELF/core/AUXV/OS/Solaris enrichment; [encoding.rs](../src/encoding.rs), [detectors.rs](../src/detectors.rs), [cdf.rs](../src/cdf.rs), [elf.rs](../src/elf.rs). |
| CLI/IO | Filename framing, brief/NUL/raw/MIME/Apple/extensions, file lists, stdin, ordered options/exclusions/parameters, metadata, symlinks, timestamp restoration and bounded reads; [cli.rs](../src/cli.rs), [filesystem.rs](../src/filesystem.rs). |
| Rust API | Owned output/results; immutable shared database with bounded synchronized lazy regex caches; buffer/path/reader/borrowed-file entry points, restored seek position; [lib.rs](../src/lib.rs). No C ABI. |
| Hardening | Core `forbid(unsafe_code)` plus an inventoried platform locale boundary, checked reads/arithmetic, bounded allocations/recursion/regex/work/output/decompression; [limits.rs](../src/limits.rs), [hardening tests](../tests/hardening.rs), twenty-three [fuzz targets](../fuzz/README.md). Transitive std/dependency unsafe is outside the first-party prohibition. |
| Distribution | Embedded Rust-built rules, unmodified selected source fragments, separate upstream-excluded `etf`, hashes and notice bundle; [data](../data/), [NOTICE](../NOTICE), locked dependencies, Linux CI configuration. |

## Executed evidence

The pinned oracle is file 5.48 at `41b1123083ab0bbdfe32f6d4ec14f6aec9a91e59`, built locally with the features in [baseline.json](../reports/baseline.json). Runs use `LC_ALL=C`, `TZ=UTC`, explicit rules/flags, and matching bounded bytes. These figures describe this corpus and host; they are not universal compatibility claims.

| Check | Observed result | Evidence |
|---|---|---|
| Original build/tests | Autotools build and `make check` pass; 103 fixtures | [baseline](../reports/baseline.json), verification log steps |
| Rust formatting/lints/tests | 120 ordinary tests pass in debug and release; Clippy warnings-as-errors passes | [all-feature verification](../reports/verification-extended.json) |
| Direct upstream goldens | 103 / 103 exact | [fixtures.json](../reports/fixtures.json) |
| Broad API differential | 22,544 / 22,544 exact; 2,818 files × buffer/path × description/MIME/type/charset | [differential.json](../reports/differential.json), [hashed corpus manifest](../reports/corpus-manifest.json) |
| Additional flag profiles | 11,000 / 11,000 exact; balanced 500-file corpus × 11 flags/combinations × 2 entry points | [extended.json](../reports/extended.json) |
| Custom language/truncation programs | 10,875 / 10,875 exact; 145 programs × 75 inputs, builtin detectors disabled | [semantics.json](../reports/semantics.json) |
| CLI bytes/channels/status | 102 / 108 exact; six explicitly recorded differences below | [cli.json](../reports/cli.json) |
| Source compiler | Byte-identical 10,876,464-byte v21 image; all 25,176 records and source ordering match | [compiler.json](../reports/compiler.json), [Rust-only rebuild](../tools/rebuild_database.py) |
| Stable mutation campaign | 10,000 database mutations + 10,000 source mutations + 10,000 random buffers + 1,339 fixture truncations; no panic | verification mutation summary; deterministic seed `4f6c655275737421` |
| Fuzz infrastructure | Twenty-three targets include per-codec adapters, captures and locale conversion; sanitizer smoke evidence is separate from the unexecuted sustained gate | [fuzz README](../fuzz/README.md), verification steps |
| Runtime independence | Production loader has no libmagic dependency | [runtime-linkage.txt](../reports/runtime-linkage.txt) |

Database SHA-256: `8eab57da91fd9cc4c7af3b8a4369a64d32124aa55dc376207a81e69abd4a5ac8`. [SOURCES.json](../data/SOURCES.json) maps the exact vendored inputs. The byte comparison is raw equality, not merely normalized equality.

The informational release benchmark uses 110 inputs, 10,820,044 bytes, three sweeps per pass and ten warm passes; both engines produce the same checksum. [benchmark.json](../reports/benchmark.json) records native-process cold load, median/p95 warm timings and peak RSS. Rust is approximately 1.7× C's warm runtime on this corpus; Rust validates and owns the database while C maps it. This run measured 39.9 MiB Rust peak RSS versus 29.9 MiB C; cold load was 7.1 ms versus 96 µs (one sample each). This is a smoke measurement, not an adversarial latency or exhaustive format benchmark.

Reproduce the complete checks with `python3 tools/verify.py --benchmark`; `bash tools/build_reference.sh` prepares the pinned oracle if needed. Individual scripts write complete mismatches and do not silently change expected output. System corpus files are hashed and referenced, not redistributed. GitHub CI runs on pushes and pull requests; its remote results are available in the repository Actions tab. The machine-readable reports here describe the recorded local baseline.

## Explicit differences and compatibility gaps

Six CLI comparisons intentionally differ on this host: four gzip `-z/-Z` cases decode in Rust while the sandboxed C CLI reports a no-fork error even for its native decoder; two annotation-only FIFO cases return a controlled Rust error while C terminates with SIGSYS. The four gzip cases with C sandbox disabled match. Source evidence and suspected-defect distinctions are in [architecture observations](01-architecture.md#additional-observations-from-differential-work); raw stdout/stderr/status remain in the report.

Remaining gaps are concrete release work:

1. **Structured formats:** CDF and ELF/core milestones are merged (#7/#8), including checked metadata and named profile decisions. CDF unknown codepages/opaque property types remain explicit; exhaustive host IO failure prose, other-platform behavior and sustained security certification remain open. See the [delivery ledger](08-delivery-progress.md) for the exact supported notes/properties and evidence.
2. **Regex/date/display:** the bounded capture VM and GNU C-byte lexer now support ordinary/nested/empty/optional/repeated references. Pinned548 explicitly rejects the conservatively identified quantified-history construct class; CorrectedV1 enumerates those histories. Unknown-escape/ICASE quirks are profiled, classes are parsed, and source/v21 comparisons are separate. Full glibc history fidelity, multicharacter collation, uncertified locales/encodings, caller rounding modes and exceptional custom timezone cases remain R6 work. The formatter/date increment has 15,447 exact float and 2,850 exact date comparisons.
3. **Grammar/diagnostics:** ordinary upstream sources compile identically, but strict errors replace some warning/truncation behavior. `-c/-l/-d` output differs. Parameter parsing rejects `atoi`/prefix quirks. ID3/varint and selected indirect-double evaluator behavior needs further minimal repros; affected paths report unsupported rather than silently ignore rules. Full C API state/error/parameter compatibility is not provided.
4. **Compression:** default builds include gzip/zlib; `extended-codecs` adds bzip2, XZ/LZMA/lzip, Zstd, modern LZ4, UNIX compress and first-entry stored/deflate ZIP. Randomized historical bzip2 blocks, stored ZIP entries with unknown descriptor boundaries, external dictionaries, legacy frames, lrzip/frozen/pack/SCO remain explicit gaps. Strict checksum/truncation/output-cap handling intentionally differs from some C paths. These features remain experimental and off by default. No production helpers or kernel sandbox are implemented.
5. **Sampling/limits/filesystems:** buffer input is sampled at `Limits.bytes`, unlike unrestricted C `magic_buffer`; truncation is exposed. Additional finite budgets intentionally reject workloads C may attempt. Reader truncation at exactly the cap is conservative. Socket permission reporting approximates Unix mode bits rather than complete ACL/access policy. Special device reads may block when explicitly enabled.
6. **Platforms/security:** x86_64 Linux/glibc is the only executed platform. macOS/BSD/Windows, musl, 32-bit and actual big-endian machines remain unverified; swapped database vectors do not certify a big-endian host. No 24-hour coverage-guided campaign, Miri campaign, historical CVE coverage claim or independent security audit has been completed. Do not deploy this 0.1 build as a fully certified libmagic replacement in hostile production pipelines yet.

The [compatibility matrix](03-compatibility.md) states the target and policy; this document states delivered scope. See the [prioritized plan](04-verification-and-delivery.md#milestones-and-acceptance) and [risk register](05-open-questions-and-risks.md) for closure criteria. P0 and the measured P1 profile are delivered. P2 fidelity/security/platform certification and P3 C ABI/release remain open; the follow-on request makes the C ABI planned scope.

The [remaining-work implementation specification](07-remaining-work-spec.md) defines closure contracts. Its original CDF discrepancy is resolved in the pinned profile; corrected property rendering is a separate deliberate profile choice. The delivery ledger distinguishes code delivery from pending security/release gates. Historical timing and original corpus totals above are not a claim of universal conformance.
