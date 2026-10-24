# Compatibility matrix and explicit decisions

Target: upstream `41b1123083ab0bbdfe32f6d4ec14f6aec9a91e59`, file 5.48, magic v21. “Exact” means bytes, channel and status under the same locale, rules and build features. This is the required compatibility policy, **not a statement of completed implementation**. Actual results and remaining gaps are recorded in `reports/` and [implementation status](06-implementation-status.md).

| Surface | Required target | Stage | Intentional difference or gap policy | Verification |
|---|---|---|---|---|
| CLI defaults / ordered options | no implicit stdin; flags/order/default symlink policy | B | name/version `rfile`; never impersonate upstream version | argc permutations, POSIXLY_CORRECT |
| stdin `-` | consume prefix, label `/dev/stdin` | A/B | reader cap explicit; no implicit tempfiles | pipe/regular redirected stdin, repeated `-` |
| file list `-f` | trim/parse order, width and immediate action | B | arbitrary NUL filenames unavailable just as source | options before/after -f; `-n` |
| output framing | brief, -bb quirk, -N, -F, -0/-00 | B | C locale byte widths initially; Unicode display pending | byte capture and invalid UTF8 paths |
| errors/status | stdout per-file messages, -E, stderr setup errors | B | Rust messages only where declared; never silently swallow resource errors | absent/unreadable paths, broken output pipe |
| MIME type/charset | exact annotation timing and builtin defaults | B/C | structured result owns strings | test separately and combined with -k |
| Apple/extensions | exact annotation/fallback behavior | B | no guessed extensions from filename | missing and child annotations |
| skip flags | independent pipeline exclusions and aliases | B/C | unsupported exclusions explicit until detector exists | pairwise flag matrix |
| multiple matches | group priority, children, builtin continuation | B/C | do not deduplicate similar descriptions | -k on overlapping rules and fixtures |
| magic source | escapes, numeric aliases, operations, full type set | B/C | strict mode rejects unsafe/ambiguous forms; compatibility mode names quirks | parser corpus, diagnostics, normalized records |
| source checks/list | compile/check/list actions | B | diagnostics may differ initially; mismatch documented | stdout/stderr/status snapshot |
| source compilation order | root strength plus native tie comparator | B | portable little-endian output; deterministic debug provenance | root order and normalized binary comparisons |
| compiled v21 | read both endian orders; export LE | A/B | reject all other versions, no struct casts | C->Rust->C; truncation/count corruptions |
| source directory/list loading | sorted fragments, sequential database list | B | embedded database fallback explicit | duplicate names, failed element, empty MAGIC |
| numeric primitives | width/endian/sign/masks/relations | A/B | checked errors for UB rather than mimic memory corruption | custom magic and boundary byte vectors |
| offsets and continuations | start/tail/parent base and indirect forms | B/C | overflow mismatch or typed error, never wrap slice address | synthetic trees and EOF cases |
| name/use, endian flip | recursive context/evidence/limits | B | bounded calls and no uncontrolled stack | recursive/cyclic names and ^ uses |
| string/pstring/string16 | byte-oriented quirks and displayed values | B | no full Unicode replacement of string16 semantics | prefix/width/NUL/whitespace/case vectors |
| literal search | bounded positions and continuation base | B | memmem boundary quirk retained only when verified | range last-position tests |
| regex | POSIX ERE C-locale leftmost-longest, last-byte truncation | B/C | earliest-start/longest-overall-match implemented; unsupported backreferences/collation remain gaps; no C FFI | alternation, NUL, $, classes, limits |
| dates/float/GUID/DER/octal/varints | typed matching and exact display | B/C | varint/ID3 at pin may have broken evaluator support; track suspected defects | cross-endian vectors, TZ, NaN |
| metadata/symlinks/special files | Unix lstat/stat and MIME inode results | B/C | no reading devices without -s; unsupported platforms declared | fifo/socket/link/device/permissions |
| text/encoding | ordered recognizers, character tables, normalized rules, suffixes | B/C | unsupported encodings must be listed, not guessed as UTF8 | BOM, invalid/truncated Unicode, EBCDIC |
| tar/JSON/CSV/SIMH | upstream heuristic, not just format validity | B/C | dedicated bounded heuristic recognizer; retain malformed-prefix behavior | existing+generated fixtures |
| ELF/CDF | metadata ordering and bounded malformed handling | C | generic identification is not equivalent to detailed output | real files, core notes, sector cycles |
| compression | -z/-Z one layer, wrapper MIME and error descriptions | B/C | gzip and zlib implemented; no external helper; other codecs explicit unsupported errors | same codec feature set, bombs and truncation |
| limits | upstream defaults plus safe global budgets | A+ | new limit errors are intentional, never masquerade as data match | tiny/zero/absurd limit values |
| preserve atime | platform restoration behavior | C | reject flag if not implemented rather than no-op | timestamp/fd ownership tests |
| sandbox | Linux seccomp/Landlock enabled features reported | C | safe parser does not imply kernel sandbox; missing sandbox declared | strace/denial process tests |
| Rust API | owned results, immutable DB, concurrent calls | A/B | deliberately idiomatic, no magic_t aliasing | API integration and multithread determinism |
| C ABI | all exported libmagic entry points/constants | D, now requested | absent until separate crate and conformance suite; see [ABI plan](remaining/07-c-abi.md) | C client under sanitizers, pointer lifetimes |
| packaging | deterministic versioned rule distribution | B/D | embedded pinned fallback, binary `rfile` | build without upstream tree or C compiler |
| Windows/BSD/macOS/big endian | target-specific metadata and filename handling | C/D | Linux tested first; portable core alone is not platform certification | native CI/QEMU |

Intentional changes are restricted to: memory-safe rejection of malformed/overflow states; finite total work/output/database budgets; immutable shareable database and owned Rust results; no external decompression helpers; distinct executable identity; pinned embedded data fallback; optional strict source mode and portable binary output. Every other deviation remains a compatibility gap until explicitly approved as a design change. Do not count a MIME-only match as an exact human-output match, or a buffer match as fd API parity.

Potential upstream defects should be represented as separate expected observations, with minimal reproducer and exact flags/build attached. Preserve defined, safe quirks in compatibility mode; do not reproduce out-of-bounds access, UB or denial of service. A corrected semantic profile must be named/versioned, with paired legacy and corrected tests.

Observed delivered profile: all 103 upstream goldens, the standard differential corpus and the selected extended flag profile pass in the reports. CLI has six documented differences on this sandboxed host (four native gzip guard cases and two FIFO SIGSYS cases). These figures do not certify untested platforms or the remaining ELF/CDF/locale/compiler-diagnostic surfaces. Local-time date rendering currently uses UTC; source warnings, native sort on other endian targets, strict CRC errors, parameter quirks and oversized buffer sampling remain gaps or declared changes.
