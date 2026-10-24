# CDF: complete compound-document classification

Parent contract, baseline, profiles and shared budgets: [remaining-work specification](../07-remaining-work-spec.md). Requirements CDF-01 through CDF-09 are prospective. This document covers CFB/CDF containers used by legacy Office and related applications; it does not implement document editing or extraction.

## Existing behavior and source evidence

Current [cdf.rs](../../src/cdf.rs) reads FAT/DIFAT, mini-FAT, directory and selected streams from a bounded slice, identifies HWP and several directory-name subtypes, and falls back to generic OLE storage. It does not read property sets or Thumbs.db catalogs and cannot retrieve a summary beyond the initial file sample.

Pinned upstream anchors: `src/cdf.c:395` header, `:472` SAT, `:564` chain counting, `:610/:659` large/small chains, `:714` directory, `:817` root short stream, `:861/:870/:879` document-summary/summary/user streams, `:1009` properties, `:1167` property-set header, `:1206` catalog; `src/readcdf.c:148` property rendering, `:271` catalog, `:302` summary prefix, `:392` summary/type fallback, `:440` section subtype table, `:562` orchestration; `src/cdf_time.c:99` FILETIME conversion. All refer to the immutable parent baseline.

The [new probe](../../reports/remaining-spec-observations.json) is reproducible from its stored 5,632-byte fixture hex. An independent olefile 0.47 reader at its DEFECT_INCORRECT threshold reads both complete properties (1252 and Microsoft Word); its result is recorded with the fixture. The probe exposes a human-description mismatch absent from the old corpus. The CDF property parser checks stream containment using `&sh`, a copied stack object, then calculates its end from that same stack object (`cdf.c:1034`, `:1047`). This is suspicious independently of file validity. Do not claim successful summary-output parity merely because the pinned reader rejects summaries.

## CDF-01: bounded random-access container

Use the shared InputView and aggregate allocation/read budgets. Keep file offsets as u64; a sector ID is u32 interpreted by explicit sentinel rules, never cast to a pointer or signed offset.

| Header field | Byte offset / width | Required interpretation |
|---|---|---|
| signature | 0 / 8 | `d0 cf 11 e0 a1 b1 1a e1` |
| minor, major, byte order | 24,26,28 / u16 each | retain values; normal v3/v4 containers use 0xfffe and LE fields |
| sector / mini-sector shifts | 30,32 / u16 | v3 normally 9, v4 normally 12; mini normally 6; validate before shifting |
| directory sector count | 40 / u32 | v4 count; v3 reserved zero does not mean directory absent |
| FAT count / directory first | 44,48 / u32 | table count and chain head |
| mini cutoff | 56 / u32 | compare declared stream length with this value |
| mini-FAT first/count | 60,64 / u32 | ordinary-sector chain holding mini-FAT |
| DIFAT first/count | 68,72 / u32 | extension chain containing FAT sector IDs |
| inline DIFAT | 76 / 109 u32 | ordered FAT sector IDs |

Regular sector position is `(u64(id)+1) * sector_bytes`; for v4, sector zero starts at 4096, not 512. Require sufficient header padding. Mini-sector position is `id * mini_sector_bytes` within the root mini-stream, not the original file. Reserved IDs: FREE=0xffffffff, END=0xfffffffe, FAT=0xfffffffd, DIFAT=0xfffffffc; other reserved IDs are invalid chain links. EOF/overflow must be detected before allocation or IO.

Accept standard v3/v4 in both profiles. The legacy source permits some nonstandard shifts up to 20; safe compatibility acceptance requires a fixture proving observable behavior and satisfying physical bounds. CorrectedV1 rejects inconsistent major/shift/byte-order combinations. Record that difference instead of broadly accepting nonsensical headers.

## CDF-02: chains and directories

Read inline DIFAT then extension sectors in order. Each extension's final u32 is its successor; remaining entries are FAT IDs. Check counts, EOF, reserved IDs and revisits. Duplicate FAT references must not multiply allocation; distinguish a malformed table from a reference-count disagreement. FAT memory is bounded by both declared FAT count and the global cap.

Implement a chain iterator parameterized by FAT, sector size and logical address space. Each yielded sector consumes one step and read reservation. A visited set or bounded generation-mark vector detects cycles; maximum links is the smaller of address-space capacity and configured chain limit. Stop at END. FREE/FAT/DIFAT cannot substitute for END. A declared nonempty stream with an early END is truncated/malformed; an empty stream with END is allowed. Keep physical chain bytes separate from declared useful length.

Directory records are 128 bytes. Name bytes 0..64, LE u16 name length at 64 (includes terminator), kind at 66, tree links at 68/72/76, CLSID 80..96, creation/modification FILETIME 100/108, first sector 116, size 120. Preserve directory record order for legacy subtype scanning. For v3 only low 32 stream-size bits are used as the pinned implementation does. Validate name length 2..64, evenness and UTF16 bounds before decoding. Empty slots are not directories. Preserve raw u16 name units for the legacy name comparator; lossy Unicode conversion cannot alter matching.

Find the root storage, then expose the mini-stream through bounded views. Do not materialize every user stream. Directory sibling/child cycles must not hang tree traversal. The pinned fallback can scan physical records without reconstructing a balanced tree; keep that legacy order distinct from any validated tree view. Missing root/short-stream behavior must follow a golden, since the C implementation can tolerate an absent root mini-stream when all requested data is in ordinary sectors.

## CDF-03: ordered detector state machine

Perform these stages in the same order as `file_trycdf`:

1. Apple/extension-only mode bypasses this detector; `NO_CDF` bypasses it in the outer pipeline. Validate header; a non-CDF header is NoMatch.
2. Read SAT, SSAT, directory, root short stream, preserving which stage failed.
3. Search the user stream named `FileHeader`; matching `HWP Document File` wins before summaries. Keep existing human/MIME strings.
4. Try `\x05SummaryInformation`. If absent, distinguish absence from a corrupt present stream. Apply summary rendering/type inference; if unhandled or failed, try `\x05DocumentSummaryInformation`.
5. Apply application-name/CLSID and directory-name inference in source order. Thumbs.db catalog is only attempted at its designated summary/type fallback point, not unconditionally before HWP/Office detection.
6. Scan directory subtype records in physical record order: encrypted streams, QuickBooks, Excel, Word, PowerPoint, Outlook. Match kind and name, not name alone. Existing table in `readcdf.c:440–515` is the normative ordered list; transcribe with its source hash and original notices.
7. Render the generic CDF fallback and stage reason only when the corresponding legacy failure state requests it. A successful header alone cannot create a specific Office MIME type.

Typed `CdfOutcome` retains `stage`, `reason`, optional root CLSID, optional Summary and ordered evidence. Separate malformed-format fallback from Limit/Io/Cancelled. Human prefix and MIME path run different rendering logic; do not derive MIME by parsing the human description. MIME charset is appended by the outer classifier once. Keep-going and flags use the existing outer scheduling contracts.

## CDF-04: property-set parser

Read the property-set stream header: byte-order/version, OS version/system ID, 16-byte CLSID, section count at 24 and section declarations beginning at 28. Each declaration is a 16-byte FMTID plus u32 stream-relative section offset. A section starts with u32 byte length and property count, followed by ordered `(property_id, section-relative offset)` u32 pairs. Offset origin is the section start, including its eight-byte header. Check table multiplication and every range against both section and stream boundaries.

Use distinct types for StreamOffset, SectionOffset and byte count. The parser must not reproduce the stack-pointer bug. Preserve property-table order and duplicate IDs in a Vec; using a map would reorder output and overwrite duplicates. Parse the first declared property set for Pinned548, matching `cdf_unpack_summary_info`; CorrectedV1 can parse the additional DocumentSummary/UserDefined set but must retain separate FMTID/labels rather than treating every ID as a SummaryInformation ID. Apply an explicit maximum of 64 sets and 10,000 total expanded properties.

```rust
struct PropertySet { fmtid: [u8;16], system: SystemId, values: Vec<Property> }
struct Property { id: u32, wire_type: u32, value: PropertyValue }
enum PropertyValue {
    Empty, Null, I16(i16), I32(i32), U32(u32), I64(i64), U64(u64),
    Bool(i32), F32(u32), F64(u64), Bytes(Vec<u8>), Utf16(Vec<u16>),
    FileTime(i64), Vector(Vec<PropertyValue>), Opaque { range: Range<u64> },
}
```

Keep floating bits to preserve NaN/sign formatting. Scalar base type IDs are 0 empty, 1 null, 2 i16, 3 i32, 4 float, 5 double, 0x0b bool, 0x13 u32, 0x14 i64, 0x15 u64, 0x1e length-prefixed bytes, 0x1f length-prefixed UTF16, 0x40 FILETIME, 0x47 clipboard. Flags 0x1000 vector, 0x2000 array, 0x4000 byref, 0x8000 reserved are evaluated before the low-12-bit type. Never interpret a byref value as an address. Unsupported types are bounded opaque values or a legacy render failure, not uninitialized zero structs.

Scalar lengths and vector counts are checked before stepping. Byte-string length is bytes; wide-string length is UTF16 units, checked times two. Respect four-byte alignment for standard property values, including vector elements. The pin's vector-string indexing is unusual (`cdf.c:1106–1148`); Pinned548 uses only deterministic, in-bounds observed behavior, with a per-vector golden. CorrectedV1 follows the property-set wire layout. Clipboard payload is skipped only after validating its declared length. Unknown bounded properties are retained in structured evidence; legacy rendering may reject a type that the parser can read.

## CDF-05: rendering and inference

Property IDs 1..19 in SummaryInformation correspond to codepage, title, subject, author, keywords, comments, template, last-saved-by, revision, total editing time, last printed, creation, last saved, pages, words, characters, thumbnail, application and security. Locale ID is 0x80000000. Copy the exact label table from `cdf_print_property_name` with notice/source attribution; unknown IDs use its numeric fallback. Prefix includes byte order, OS/version and recognized root CLSID text, in that order.

The legacy renderer accepts a narrower set than the parser: null/clipboard print nothing; i16/i32/u32/float/double print named fields; strings print only when the filtered value is nonempty; FILETIME zero prints nothing; unsupported/render-unhandled types produce summary failure. Preserve this distinction for Pinned548. C legacy narrow strings retain printable bytes; wide strings sample low bytes, not full Unicode. The bounded buffer is 1024 bytes including NUL. CorrectedV1 decodes supported codepages/UTF16 and applies the display policy; codepage unavailable is an explicit diagnostic with original bytes retained. It does not guess UTF8 from arbitrary property bytes.

For FILETIME below signed 1,000,000,000,000,000 ticks, the pin treats the value as elapsed editing time; larger values use its calendar conversion and local date display. Preserve this threshold and label formatting in the pinned renderer. CorrectedV1 uses checked real FILETIME/elapsed conversion, never uninitialized `timespec`. Use the shared time module, not a second date implementation.

MIME precedence is recognized root CLSID, then application property if no CLSID MIME, then directory-name mapping, then the generic source fallback. The application matcher in `readcdf.c:111` is C-locale case-insensitive substring matching, not arbitrary Unicode case folding. Human and MIME mappings are distinct tables. Thumbs.db catalog validates record lengths and names before rendering the ordered filename list; it does not extract thumbnails.

## CDF-06: intentional corrections and profiles

| Case | Pinned548 | CorrectedV1 |
|---|---|---|
| Summary section hits the pin's invalid bounds path | checked emulation of the observed fallback, including final `Cannot read section info` where appropriate | parse using stream/section integer offsets and render properties |
| Additional property set | legacy first-set behavior | separate supported FMTIDs, no accidental label reuse |
| Wide-string display | bounded printable low-byte projection when legacy path succeeds | checked UTF16/codepage decoding |
| Invalid FILETIME conversion | deterministic invalid/failure diagnostic; never consume undefined state | checked conversion and explicit invalid value |
| Huge/cyclic structure | Limit or malformed fallback according to stage | same hard safety boundary |

No blanket claim that all malformed inputs match C. An auxiliary C patch correcting only section pointer origin can help validate property order; any further vector/string/time fixes must be separate patches with separate results. Format-valid synthetic fixtures and an independent CFB/property reader establish corrected truth; agreement with a patched oracle alone is insufficient.

## CDF-07 through CDF-09: verification and delivery

Deliver a deterministic fixture generator with source/hash manifests. Cover v3/v4; ordinary/mini streams; more than 109 FAT sectors; valid/absent/truncated root pool; every property type and flag combination; duplicate/unordered IDs; two property sets; all application/CLSID inference priorities; HWP/Thumbs.db; missing/corrupt summary versus document-summary; MIME-only/encoding-only/combined, `-k`, skip flags, and directory subtype collisions. Place metadata beyond 7 MiB to distinguish buffer from file IO; include sparse files and short-read injection.

For each fixture compare the full human and MIME bytes, stderr/status, pinned/corrected profile, buffer/path/descriptor result and resource outcome. Preserve the exact new probe's mismatch until fixed under the pinned profile. Malformed vectors include FAT/mini-FAT/DIFAT loops, sentinel misuse, overflow counts, duplicate/reused sectors, offsets into the property table, sections beyond stream, UTF16 truncation, excessive vector counts and overlapping properties. No panic, invalid read, unbounded allocation, or reset budget is acceptable.

Add dedicated CDF-chain and property-set fuzz targets; corpus must reach every state/variant, not mostly fail the signature check. Completion requires the SEC campaign plus reviewed evidence that every source branch is covered, intentionally corrected or explicitly unsupported. A generic MIME result alone is not full CDF fidelity.

Format references: [Microsoft MS-CFB](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-cfb/53989ce4-7b05-4f8d-829b-d08d6148375b), [MS-OLEPS](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-oleps/), and its [property-set stream definition](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-oleps/6e65d6fa-6044-4e23-ae71-d65d1e3b1249). Wire-format correctness and pinned implementation behavior are separately evaluated.
