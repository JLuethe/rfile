# Compression: independent bounded codec support

CMP-01 through CMP-09 implement the codec portion of the [parent contract](../07-remaining-work-spec.md). Identification by magic rules and inspection of decompressed contents are different capabilities. This work includes the latter without native C codecs, libmagic or external decompression commands in production.

## Baseline and scope

Current `src/detectors.rs:498–547` supplies gzip/zlib readers and signature dispatch; `src/lib.rs:198–244` coordinates inner/outer classification. Other recognized codecs produce Unsupported on `-z/-Z`. Existing output limit errors, one-layer behavior and six known CLI sandbox differences stay visible until their named contract changes.

Reference `upstream/src/compress.c:201–258` has ordered dispatch for UNIX compress, gzip, frozen, SCO LZH, pack, first ZIP member, bzip2, lzip, XZ, lrzip, LZ4, Zstd, LZMA and zlib. Native implementations are selected by build features. `file_zmagic` controls one layer, flags, outer descriptions and MIME; `uncompressbuf:1189` includes the observed no-fork/native-decoder guard defect. Compare using C `-S` or direct library calls when exercising native decoding; retain the default-CLI discrepancy separately. The pinned host lacks native lzip/lrzip; those require a separately labeled oracle, not a false feature-matched result.

## CMP-01: feature registry and release scope

| Format | Planned capability / priority | Feature and completion policy |
|---|---|---|
| gzip, zlib | retain, close checksum/truncation/member differences first | `codec-deflate`; miniz backend remains explicit |
| bzip2 | next | `codec-bzip2`; bounded blocks and member restart |
| XZ / LZMA-alone / lzip | next | `codec-lzma`; separate signatures/container policies on one backend |
| Zstandard | next | `codec-zstd`; frame/window/checksum/skippable handling |
| LZ4 frame | next | `codec-lz4`; modern frame format, checksums, linked blocks |
| UNIX compress `.Z` | included later | `codec-compress`; independent LZW/container state machine, not an assumed GIF decoder |
| ZIP first physical member | included later | `codec-zip`; stored/deflate first-entry content, no extraction, explicit encryption/other-method errors |
| lrzip, frozen, SCO LZH, pack, legacy LZ4 / old Zstd variants | explicit remaining feature gaps | no selected validated safe backend; leave disabled until separate decoder and limits/security evidence exists |

The full-popular-codecs milestone means gzip/zlib+bzip2+XZ/LZMA/lzip+Zstd+modern LZ4, not all external-helper formats. `.Z` and ZIP have separate milestones. “All codecs supported” is prohibited while any reference dispatch entry remains unsupported. Modern and historical codec versions must not be conflated merely because rule identification recognizes both.

Introduce `CodecId`, `CodecCapability`, and an ordered registry whose probe returns a match only after the minimum relevant header bytes exist. Signature dispatch order is fixed by the reference table; zlib's weak header predicate must remain after stronger signatures. Unsupported inspection is a typed error; identification without `-z` continues through ordinary rules.

## CMP-02: common decoding result and driver

```rust
struct DecodeOutcome {
    bytes: Vec<u8>, codec: CodecId, input_used: u64,
    members: u32, complete: bool, checksum: ChecksumState,
}
enum DecodeError { InvalidHeader, Corrupt, Truncated, DictionaryRequired,
    UnsupportedMethod, Limit(Resource), Cancelled, Io }
```

A streaming adapter is driven in bounded input/output steps and reports progress. Reserve compressed-input, dictionary/window, decoder tables and output independently under the aggregate budget. Read/write chunk maximum is 16 KiB; charge consumed bytes, emitted bytes and frames even when a frame emits zero data. A no-progress adapter iteration is an error, not an infinite retry. Metadata lengths never authorize allocation until validated. Release all reservations on success and failure.

Expanded output remains capped at `Limits.bytes` (default 7 MiB). Current safety policy returns Limit when more output is required; it does not classify a silently truncated expanded prefix. Checking the cap may produce at most one lookahead byte. This is an explicit difference from reference paths that stop with a usable prefix. Do not drain an enormous stream just to verify its final checksum after the output cap has been reached. Preserve an incomplete/checksum-not-checked state in diagnostic evidence.

Default maximum dictionary/window is 64 MiB; maximum frames/members is 64, counting skippable/empty frames. Nested *layers* remain one: disable COMPRESS for inner classification, while concatenated members of the same selected codec are handled by that codec's policy. One member is not the same as one layer. All child classification uses the parent's work/output allocation budget; no fresh budget per member or inner call.

A streaming `Read` interface alone does not prove bounded CPU or dictionary allocation. Prefer low-level incremental APIs where needed. Opaque decoder operations have a documented worst-case block and cancellation latency verified with hostile inputs; do not claim a hard in-process deadline. If a backend cannot respect required memory/work boundaries, keep that capability disabled. Hard wall time, blocked IO and process-wide RSS are enforced by the calling supervisor and security harness.

## CMP-03: backend candidates and exact feature choices

Versions, manifests, declared licenses and features were inspected locally using `cargo info` and downloaded source; [candidate evidence](../../reports/remaining-candidate-dependencies.json) records hashes. These are implementation candidates, not integrated or security-certified dependencies. Recheck the exact versions/advisories when adding them, lock the entire graph and retain license texts.

| Candidate | Verified capability and terms | Planned use / remaining adapter work |
|---|---|---|
| `bzip2-rs 0.1.2` | pure-Rust decoder, low-level decoder and Reader; MIT/Apache-2.0 choice | default features off; use low-level progress for accounting. Inspect allocation from BZh block-size digit and concatenation semantics before enabling. Old release age is a maintenance risk, not evidence of a vulnerability. |
| `lzma-rust2 0.23.0` | LZMA/LZMA2/XZ/lzip; Apache-2.0; MSRV 1.85 | `default-features=false`, explicitly std/xz/lzip; optimization and encoder disabled. Use `XzReader::new_mem_limit` or incremental streams with KiB limits; LZMA-alone header properties checked before construction. |
| `ruzstd 0.9.1` | pure-Rust Zstd decoding; MIT; MSRV 1.87 | explicit std/hash, no dictionary builder. `FrameDecoder::set_max_window_size` checks declared window before allocation; set before initialize/reset. Validate dictionaries, skippable frames and concatenation independently of convenience Reader behavior. |
| `lz4_flex 0.14.0` | frame Reader; MIT; MSRV 1.81 | explicit std/frame/safe-decode/checked-decode; safe-encode may remain enabled to avoid unsafe feature paths although product does not encode. Bound block size and scratch multiplier before constructing/reading a frame. |
| existing flate2/miniz | already locked and tested | retain pure-Rust backend; inspect member stopping and CRC behavior rather than changing backend to match C |

The workspace MSRV becomes **1.87 when the selected Zstd dependency lands**, with CI on 1.87 and current stable. Do not leave a misleading 1.85 manifest or silently choose an older unreviewed crate. No bzip2/xz2/zstd C binding is substituted if these adapters fail. Pure Rust dependencies may still contain unsafe or panic/resource bugs; feature-specific unsafe inventory and fuzzing are mandatory.

Primary capability references: [bzip2-rs](https://docs.rs/bzip2-rs/0.1.2/bzip2_rs/), [lzma-rust2](https://docs.rs/lzma-rust2/0.23.0/lzma_rust2/), [ruzstd](https://docs.rs/ruzstd/0.9.1/ruzstd/), [lz4_flex](https://docs.rs/lz4_flex/0.14.0/lz4_flex/). Backend recommendations do not imply their outputs/errors equal the reference libraries.

## CMP-04: format-specific allocation and framing

- **gzip/zlib:** bound optional gzip filename/comment/extra headers within sampled compressed bytes; distinguish header/data/trailer corruption. Preserve gzip versus zlib signature rules, preset-dictionary request, empty data and input consumption. Test CRC and ISIZE independently; no arbitrary dictionary lookup. Do not disable checksum validation merely to match the reference's raw-inflate behavior.
- **bzip2:** block-size digit 1..9 bounds the encoded block size to 100,000 multiples. Account for all inverse-BWT/Huffman and output scratch, not just that nominal block. CRC failure, randomized historical blocks and trailing/concatenated members need explicit corpus coverage. Reinitialize between members only with verified consumed-input position.
- **XZ/LZMA/lzip:** preflight dictionary/filter properties; cap filter count, block/member count and every block's memory before allocation. Respect XZ check type, index/footer, stream padding and BCJ/Delta filters the chosen backend supports. LZMA-alone's unknown uncompressed size is streaming, not a capacity hint. Lzip header dictionary and trailer CRC/size/member-size must be verified. Units differ: convert bytes to backend KiB with checked rounding and retain aggregate overhead reservation.
- **Zstd:** cap declared window before initialization, including single-segment frames with content size; unknown content size is not unlimited allocation permission. Dictionaries are supplied only by an explicit caller configuration with its own byte cap, never fetched by ID. Initial release has no external dictionaries and reports DictionaryRequired. Skippable frames count and are skipped within input/work limits. Check checksum and frame boundaries; do not treat every `0x28b52ffd` legacy variant as modern support.
- **LZ4:** supported modern maximum block categories are 64 KiB/256 KiB/1 MiB/4 MiB. Inspect header before allocation and reserve source plus destination/history (linked blocks can require approximately two blocks plus 64 KiB). Validate header/block/content checksums and optional content size. Independent blocks versus linked history, empty frames and dictionary IDs have distinct tests. Raw size-prepended block APIs are not interchangeable with frame decoding.

Use a table of per-codec member policies captured from native C adapters. Pinned548 follows a backend's deterministic first-member/multi-member behavior where safe. CorrectedV1 decodes valid concatenated members consistently until frame/input/output caps; it reports trailing nonframe data rather than silently inventing another member. The table must be completed by measured probes before a codec is default-enabled; unknown member policy is a release blocker for that codec.

## CMP-05: legacy formats and ZIP contract

For `.Z`, validate the three-byte header, 9..16 maximum code width, block-mode and reserved bits. Implement the UNIX-compress variant of LZW: literal base dictionary, CLEAR handling, KwKwK special case, width-growth boundaries and the format's code-group packing/alignment. Dictionary entries are bounded prefix/suffix indices, not recursively expanded strings; expansion uses a bounded iterative stack and charges each emitted byte. Use independent public ncompress/gzip-generated vectors for width growth, resets and malformed cycles. A GIF-compatible LZW crate is not accepted without demonstrating the different bitstream/reset semantics.

ZIP inspection selects the **first physical local-file entry**, matching the reference's documented first-file approach; do not silently choose a more interesting later file. Establish directory/empty-first-entry behavior against the pinned helper in a labeled test profile. Support stored and deflate methods first, bounded ZIP64 sizes/data descriptors and a finite local-header parser. Never create paths, resolve entry names, write extracted data or recursively enumerate an archive. Encryption and unsupported methods are explicit errors. A central-directory-only or damaged archive remains identified by rules without claiming successful inner inspection.

lrzip/frozen/pack/SCO/legacy frames stay declared unsupported until a separate design specifies their safe bounded decoder; absence of a suitable dependency does not authorize helpers or a success fallback. They prevent any claim of full reference-helper coverage but do not block the named popular-codecs milestone.

## CMP-06: output and error compatibility

Successful decoding reuses the shared pipeline with compression disabled. `-z` human output combines inner and outer descriptions in the same ordering/parentheses; `-Z` suppresses the outer. MIME type-only, encoding-only and combined MIME, Apple/extensions, raw and keep-going must be separately tested; do not assume all are human formatting with a different label.

Normalize internal errors by category, but preserve a table of pinned human/MIME decompression-error strings for deterministic reference conditions. Native-library version-specific prose is reported separately, not passed off as exact by whitespace normalization. Rust safety limit/cancellation errors remain explicit. Pinned no-fork guard errors and sandbox SIGSYS are intentional differences, not behavior to reintroduce.

## CMP-07 through CMP-09: verification and activation

For every codec generate empty, text, binary and structured inner payloads; each option/feature combination; all frame checksums and damage points; short prefixes at every header/trailer boundary; huge declared windows, missing dictionaries, zero-progress/empty/skippable chains, concatenation and trailing garbage, near-cap expanded sizes, nested different-codec layers and bombs. Test the complete CLI/API flag/output/error/status matrix with the same database and a capability-matched oracle. Encoder tools may generate public fixtures in tests; no encoder/helper is a production dependency.

Add one structured fuzz target per codec plus the common registry/envelope path. Measure actual allocation peaks and decode latency; prove header-cap rejection precedes expensive allocation using instrumentation or allocation-failure injection. A valid decoder-only roundtrip is insufficient for classification compatibility. Feature-off builds must still identify outer files and report explicit inspection capability errors. Activate each default feature only after its memory, correctness, license, differential and SEC campaign gates pass; publish a per-codec capability list in version output and release notes.

## R5 implemented adapter decisions

Implementation now lives in `src/codecs/`, exposed through `codecs::{probe, probe_with_profile, decode}` and `Classification.compression`. The older baseline references above describe the pre-R5 design input. `DecodeFacts` records actual consumption, members, successful completion and payload checksum presence; errors return no successful outcome. Gzip/zlib remain unconditional (there is no redundant `codec-deflate` feature). Six opt-in features or their `extended-codecs` umbrella add the other selected formats. None are enabled by default before the SEC activation gates.

The bzip2 and UNIX compress decoders are independent checked Rust state machines. The former bounds Huffman/MTF/inverse-BWT blocks before allocation, incrementally emits RLE runs and verifies block/combined CRCs. Randomized historical bzip2 blocks return UnsupportedMethod. Candidate `bzip2-rs` lacked suitable exact member consumption; the alternative `crabz` expanded a full RLE block before the adapter cap. Neither is a production dependency.

`lzma-rust2 =0.23.0` uses only std/xz/lzip; its optimization and encoder features are disabled. It forbids unsafe code under this configuration. Incremental APIs have exact consumption and memory caps. The XZ stream API rejects multiple prefilters; that exact pinned-backend condition triggers a bounded reader retry, charged to the same work budget. An independent XZ framing pass validates stream/block/index/footer CRCs, canonical VLIs, index sizes, LZMA2 chunk boundaries, filter dictionary sizes and stream boundaries before either backend. This also closes the reader API's weaker index-size validation. XZ blocks are separately capped at `codec_members` per stream; records reserve 32 bytes per allowed block. XZ CHECK_NONE and LZMA-alone report absent payload integrity, rather than inventing verification.

`ruzstd =0.9.1` std/hash requires Rust 1.87. Its ring buffer contains unsafe Rust; the dependency is not described as memory-safe merely because the adapter forbids unsafe. The adapter rejects windows above `min(codec_window, bytes)` before construction, reserves four window lengths plus 4 MiB for growth/workspace, requires completed frames and explicitly compares stored/calculated xxHash checksums (the convenience decoder alone does not). A declared content size above the expanded cap is rejected before allocation. External nonzero dictionary IDs are unsupported.

`lz4_flex =0.14.0` uses std/frame/safe-decode/safe-encode/checked-decode, which enables its crate-level unsafe prohibition. The adapter validates all block/frame ranges and a terminal block before construction because a backend EOF alone is insufficient. Reservations cover three block lengths plus history/overhead. Dictionary-flag frames, including dictionary ID zero, currently return DictionaryRequired. Modern linked/independent blocks are supported; legacy framing is not.

LZMA-family workspace reserves `codec_window + 8 MiB`; the backend KiB cap also includes its overhead and may reject a nominal dictionary close to the configured ceiling. Aggregate reservations are conservative live-buffer envelopes, not allocator-wide RSS enforcement. Backend calls operate on at most 16 KiB supplied input/output; their internal block/filter operation can delay cooperative cancellation. A supervising process is required for a hard wall/RSS limit.

Pinned548 stops at the first member for native codecs, matching the measured reference build. CorrectedV1 processes concatenated gzip/zlib/bzip2/XZ/lzip/Zstd/LZ4 members and aligned XZ padding, rejecting trailing non-member bytes. Skippable LZ4/Zstd frames count toward the member cap; a leading prefix is inferred only if a following unambiguous LZ4/Zstd frame exists. A pure skippable prefix is ambiguous and does not invent a detection. UNIX compress and LZMA-alone remain single streams; ZIP selects the first physical entry in both profiles. A stored ZIP entry whose descriptor has no safely known boundary remains unsupported. No filenames are resolved or extracted.

Strict integrity/truncation rejection and an explicit output-cap error are intentional safety differences from C paths that return a usable prefix. Malformed native-library error prose is not yet normalized to C. Optional C oracle helpers for ZIP/.Z run only in the differential harness, never in production. Lzip uses independent Python LZMA1/container vectors because this pinned reference build has no native lzip; that is labeled separately from matched-feature differential evidence.

The differential review made three explicit additional decisions for **both** profiles: emit one in-process `.Z` result instead of repeating native gzip/uncompress dispatch, inspect the first valid ZIP entry without reproducing helper-dependent warning/errors, and accept a valid empty LZ4 frame rather than the reference native adapter's `unable to decompress file`. This means Pinned548 does not promise exact output for those named cases. `upstream/src/compress.c:216–218,325–407` explains duplicate `.Z` dispatch; `:914` is the empty-LZ4 error. The helper ZIP behavior depends on installed gzip and entry layout. In this host's vectors, ncompress round-trips all four `.Z` widths; gzip rejects a valid 9-bit stream. No native helper defect is reintroduced into the Rust decoder.

`tests/fixtures/compression/compatibility-decisions.json` freezes the complete raw C/Rust observation hash for every reviewed disagreement, keyed by fixture/API/options. The harness rejects a new or changed disagreement rather than assigning a broad format-wide ignore. Exact matches, frozen decisions and unexpected results have separate totals. The fixture manifest and complete report retain actual bytes/hashes; the decision hash is not output normalization. `.Z` is not self-delimiting and cannot reliably distinguish every truncated code-group suffix from an ordinary end; its absent checksum and that limitation remain observable.
