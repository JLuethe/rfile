# ELF: executable enrichment and core-dump fidelity

Normative shared requirements and baseline: [parent specification](../07-remaining-work-spec.md). ELF-01 through ELF-09 extend the current [elf.rs](../../src/elf.rs); this document does not implement a debugger, loader or DWARF parser.

## Evidence and integration contract

Current `analyze` handles common program/section tables, interpreter/dynamic information, build IDs, selected OS notes and stripped/debug descriptions. The larger reference is `upstream/src/readelf.c`: `dophn_core:365`, OS versions `:440/:489`, build IDs `:565`, OS notes `:611`, PaX/memtag `:728/:767`, core `:806`, virtual-address mapping `:1019`, AUXV `:1087`, note dispatch `:1231`, sections/capabilities `:1429`, executable headers `:1737`, `file_tryelf:1895`; `src/elfclass.h:31–79` determines class-specific scheduling. Tables in `src/readelf.h` are part of the contract, not host ABI structs.

ELF-01: softmagic remains responsible for the basic ELF signature, class, endianness, machine and file-type description. Enrichment runs only where the original entry point has file-descriptor/random-access evidence. A `magic_buffer` comparison must not gain arbitrary path-based details. `NO_ELF`, annotation modes and MIME rules must preserve their current precedence. ELF decisions can affect PIE MIME classification even when human suffix rendering is suppressed. Enrichment must be inserted at the existing softmagic stage before keep-going fallbacks, not appended blindly after final output.

Use `ElfFacts { class, order, machine, type, linkage, executable_override, evidence: Vec<ElfEvidence> }`. Evidence carries source offset, exact ordering index and a typed variant; output rendering preserves discovery order and legacy suppression flags. Avoid a HashSet of printed strings: two identical-looking notes can legitimately print twice if the pin does not mark that note family as handled.

## ELF-02: headers and bounded tables

| Layout | ELF32 | ELF64 |
|---|---|---|
| ELF header / program header / section header bytes | 52 / 32 / 40 | 64 / 56 / 64 |
| class/order | ident bytes 4/5 | same |
| type/machine | u16 at 16/18 | same |
| entry point | u32 at 24 | u64 at 24 |
| program/section offsets | u32 at 28/32 | u64 at 32/40 |
| flags / header size | u32 at 36 / u16 at 40 | u32 at 48 / u16 at 52 |
| program entry size/count | u16 at 42/44 | u16 at 54/56 |
| section entry size/count/string index | u16 at 46/48/50 | u16 at 58/60/62 |

Interpret file fields using ident byte order, independently of host/compiled database endian. Validate checked `table_offset + count * entry_size`, field-sized reads and file length before requesting data. Compare declared entry size to the relevant layout; preserve descriptive corruption diagnostics where the reference emits them. ET_EXEC/ET_DYN inspect program headers then sections; ET_REL inspects sections; ET_CORE inspects core program notes; unknown types leave basic magic results. The pin requires more than a complete ELF header before enrichment (`elfclass.h:31`); test exact header-size boundaries.

Keep current ELF count/section-size limits plus aggregate read/memory budgets. Program-note read windows must reflect the pin's bounded buffer sizes rather than automatically reading a whole giant segment. Record actual bytes inspected. File truncation after metadata is a short read; no zero-filled invented note content.

Extended numbering is a deliberate profile distinction: the pin uses raw u16 counts and does not resolve PN_XNUM/SHN_XINDEX through section zero. Pinned548 retains that behavior. CorrectedV1 may resolve standard extended numbering through a checked section-zero record, then enforce effective-count caps before iteration. Do not call that enhancement exact pinned compatibility.

## ELF-03: linkage, executable state and section facts

Process PT_DYNAMIC entries in encounter order at class width. Count DT_NEEDED; DF_1_PIE in DT_FLAGS_1 determines PIE/executable state for ET_DYN. PT_INTERP also increments the need count. Preserve source reset of executable mode when encountering PT_DYNAMIC; filesystem executable bits alone are insufficient. Link text is static, dynamic or static-PIE according to the pin's combination of dynamic table, PIE bit and need count. Preserve the exact interpreter truncation/empty marker and placement after linkage text.

For sections, use the designated string table to read bounded names. SHT_SYMTAB makes a file not stripped; SHT_DYNSYM alone does not. A `.debug_info` name produces debug information and not-stripped state. Test section/name-read scheduling: `doshn` uses header state in an unusual order, so avoid silently normalizing a legacy off-by-one observable. If that behavior differs from a standards-correct iterator, isolate a golden and use the corrected profile for the repair.

Missing section tables, wrong entry sizes, bad offsets and unreadable notes have distinct text versus fatal IO outcomes. Create a source-to-diagnostic table covering each return from `doshn`/`dophn_exec`; the acceptance matrix must include these, not just valid binaries.

## ELF-04: note framing and dispatch

A note starts with three file-endian u32 values: name length, description length, type. Name starts immediately after the 12-byte header; description and next-note positions use checked alignment appropriate to the program/section source (normally 4, with validated program alignment). Reject invalid alignment before arithmetic. Length high-bit and zero/zero terminator behaviors have explicit legacy diagnostics. Bounds and monotonic advancement are checked even if the note family is unknown.

Maintain the pin's per-family flags and dispatch order: OS note, build ID, NetBSD PaX, Android memtag, core information, AUXV, then remaining NetBSD annotations. Share the remaining-note counter across all program/section sources and named passes for one ELF analysis. Preserve the exact boundary where consuming the last allowed note produces the reference's too-many-notes outcome. Duplicate locations discovered via both a program header and section are not globally deduplicated unless the source's family flag suppresses them.

| Family | Required decode/render behavior |
|---|---|
| GNU build ID | type 3, descriptor 4..20 bytes; lengths 8/16/20 label xxHash/md5-uuid/sha1 using exact source spelling; lowercase hex; one handled GNU build ID |
| Go build ID | owner/name-length predicate and descriptor <128; bounded string rendering. The source Go branch returns success without setting the GNU handled bit: test repeated Go notes rather than assuming suppression. |
| GNU ABI | four u32 fields; OS enum Linux/Hurd/Solaris/kFreeBSD/kNetBSD/unknown then three version fields |
| SuSE/OpenBSD | exact owner, type and size checks; SuSE two-byte version, OpenBSD marker |
| Android ABI | API level, then two bounded 64-byte NDK strings when descriptor permits; preserve order and return/handled semantics |
| FreeBSD/NetBSD/DragonFly | source version encodings and suffix text; see formulas below |
| NetBSD PaX | bit labels and delimiter order from `do_pax_note`; unknown bits do not become guessed names |
| Android memtag | source bit positions for none/async/sync/heap/stack; zero-value handling and comma joining tested |
| NetBSD miscellaneous | architecture, compiler model, emulation; bounded 100-byte data; unknown note number printed at most once |

NetBSD version decode uses major=`v/100000000`, minor=`v/1000000%100`, release=`v/10000%100`, patch=`v/100%100`; for major >=9 fold `100*release` into patch and suppress letter suffixes. Older release counters use the source's repeated-Z alphabet rule. FreeBSD has the 460002 special case, pre-460100 and pre-500000 branches and modern major/minor/revision formatting (`readelf.c:489–563`); transcribe these predicates into a small pure formatting function with boundary vectors at every branch, not a contemporary OS-version lookup. DragonFly uses major=`v/100000`, minor=`v/10000%10`, final=`v%10000`.

## ELF-05: core information without executing or loading binaries

Recognize SVR4 CORE ownership including the historical four-byte non-NUL form, FreeBSD, and NetBSD-CORE prefixes. Set and print core-style state before dependent AUXV processing. Unknown owners remain unhandled. Only parse process information under the same file-type/owner/type gates as the pin.

The generic PRPSINFO search uses ordered candidate offsets relative to the description:

| Class | Default offset order | Conditional prefix when USE_NT_PSINFO is enabled |
|---|---|---|
| 32 | 100,84,44,28,48,32,8 | 104,88 |
| 64 | 136,120,56,40,16 | 152,136 |

For each candidate, inspect up to 16 bytes, reject empty, nonprintable or quote-containing names, then apply the source's earlier-offset adjustment when a candidate lies inside a longer printable region; preserve its 16-byte nonterminated-name exception. Collect the bounded printable command string and trim trailing whitespace before quoting. Reference build features determine the conditional offset table; record them in the fixture manifest.

FreeBSD PRPSINFO argument offset is `4+4+17` for 32-bit and `4+4+8+17` for 64-bit; collect up to 80 characters when the 81-byte field fits, then PID at `argument_offset+81+2`. Read PID with file-endian byte decoding, never an unaligned native pointer cast. NetBSD procinfo is a versioned field layout copied from the pinned header into offset/width descriptors, with name, PID/effective UID/GID, LWP count/ID and signal/code. Truncated fields follow safe, explicit fallback; do not cast to host structs.

Some legacy string scans are bounded by the enclosing note buffer rather than the description. Pinned548 can retain that defined, in-buffer behavior using checked ranges; CorrectedV1 constrains process fields to their descriptor. Add a cross-note printable sentinel test so the difference cannot be hidden.

## ELF-06: AUXV and virtual strings

Only supported core styles and NT_AUXV activate this stage; BSD AUXV branches compiled out by the pin remain outside pinned output. Walk class-sized `(type,value)` pairs, maximum 50 elements as in the source, with exact terminator/error behavior established by fixtures. Print execfn/platform strings and real/effective UID/GID tags in encounter order. Numeric C casts are emulated at the output width.

Resolve a virtual string address against program headers using `vaddr <= address < vaddr + file_size`; checked addition, not mem_size. Read from `file_offset + address - vaddr`, maximum 256 bytes, bounded to available file data, with the legacy final-byte-NUL behavior. Never read the host process at a virtual address from a core file. On an unmapped address, CorrectedV1 returns no string; the pin can return file offset zero on lookup failure, so any compatibility emulation must have its own golden and a checked file read. It must not become arbitrary pointer access.

## ELF-07: Solaris capabilities

Handle SHT_SUNW_cap only for the machine families allowed by `doshn`. Entries are class-sized tag/value pairs. Accumulate HW_1 and SF_1 by OR; retain source handling of NULL/unknown tags, bounded bad-capability warnings, and the special leading-'A' path. Hardware names are emitted in the source table order after debug/stripped evidence, clearing recognized bits before reporting the remaining mask. SF_1 uses FPUSED/FPKNWN to distinguish frame-pointer descriptions, then reports unknown remaining bits. No CPU-instruction detection or execution is performed.

Deliver generated immutable Rust tables for names/masks with upstream file/line/hash attribution and a reviewable extraction diff. The test suite exercises every named bit, combinations, unknown masks, unsupported machines and table order. Machine and note type numeric constants come from the pinned `readelf.h`, not the build host's `/usr/include/elf.h`.

## ELF-08 and ELF-09: verification and definition of done

Build deterministic synthetic ELF generators independent of the parser. Matrix: ELF32/64, LE/BE, EXEC/DYN/REL/CORE, zero/normal/extended counts, PT/SHT note sources, alignments 4/8/invalid, stripped/debug/linkage/interpreter combinations, each OS/build-ID/PaX/memtag family, each core-offset candidate and overlap rule, AUXV mapping/miss/50-entry boundary, and every Solaris capability branch. Test duplicate/out-of-order notes and wrong owner/type/size predicates.

Compare path and borrowed-descriptor enrichment against their corresponding C APIs; compare buffers separately without fabricated file details. Include MIME/type/encoding, Apple/extensions, no-ELF, keep-going, sparse >4 GiB offsets, tables beyond the input sample, changing files and broken read injection. Cap-limit errors and corruption text are compared as well as classifications.

Add ELF header/note and full seekable-core fuzz targets. Inputs must reach interior note handlers through structure-aware mutation, including counts, virtual mappings and note-length arithmetic. Generate synthetic core data; do not check in real crash dumps containing memory, credentials or personal information. Full fidelity means all enabled source branches have an exact golden or named corrected/safety outcome, not simply that `/usr/bin` binaries agree.
