# Compatibility matrix and explicit decisions

Target: upstream `41b1123083ab0bbdfe32f6d4ec14f6aec9a91e59`, file 5.48, magic v21. “Exact” means bytes, channel and status under the same locale, rules and build features. This is the required compatibility policy, **not a statement of completed implementation**. Actual results and remaining gaps are recorded in `reports/` and [implementation status](06-implementation-status.md).

| Surface | Required target | Stage | Intentional difference or gap policy | Verification |
|---|---|---|---|---|
| CLI defaults / ordered options | no implicit stdin; flags/order/default symlink policy | B | name/version `rfile`; never impersonate upstream version | argc permutations, POSIXLY_CORRECT |
| stdin `-` | consume prefix, label `/dev/stdin` | A/B | reader cap explicit; no implicit tempfiles | pipe/regular redirected stdin, repeated `-` |
| file list `-f` | trim/parse order, width and immediate action | B | arbitrary NUL filenames unavailable just as source | options before/after -f; `-n` |
| output framing | brief, -bb quirk, -N, -F, -0/-00 | B | C bytes plus certified Linux/glibc C.UTF-8/en_US.UTF-8 widths; other locales/platforms need separate evidence | byte capture and invalid UTF8 paths |
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
| regex | POSIX ERE C-locale leftmost-longest, last-byte truncation | B/C | bounded capture VM plus regular fast path; pinned quantified-history restrictions and multicharacter collation remain explicit gaps; corrected histories are opt-in; no regex FFI | alternation, NUL, $, classes, limits |
| dates/float/GUID/DER/octal/varints | typed matching and exact display | B/C | varint/ID3 at pin may have broken evaluator support; track suspected defects | cross-endian vectors, TZ, NaN |
| metadata/symlinks/special files | Unix lstat/stat and MIME inode results | B/C | no reading devices without -s; unsupported platforms declared | fifo/socket/link/device/permissions |
| text/encoding | ordered recognizers, character tables, normalized rules, suffixes | B/C | unsupported encodings must be listed, not guessed as UTF8 | BOM, invalid/truncated Unicode, EBCDIC |
| tar/JSON/CSV/SIMH | upstream heuristic, not just format validity | B/C | dedicated bounded heuristic recognizer; retain malformed-prefix behavior | existing+generated fixtures |
| ELF/CDF | metadata ordering and bounded malformed handling | C | generic identification is not equivalent to detailed output | real files, core notes, sector cycles |
| compression | -z/-Z one layer, wrapper MIME and error descriptions | B/C | default gzip/zlib; opt-in additional codecs and named helper/empty-frame output decisions; no production helper; see [codec decisions](remaining/03-compression.md#r5-implemented-adapter-decisions) | same codec feature set, bombs and truncation |
| limits | upstream defaults plus safe global budgets | A+ | new limit errors are intentional, never masquerade as data match | tiny/zero/absurd limit values |
| preserve atime | platform restoration behavior | C | reject flag if not implemented rather than no-op | timestamp/fd ownership tests |
| sandbox | Linux seccomp/Landlock enabled features reported | C | safe parser does not imply kernel sandbox; missing sandbox declared | strace/denial process tests |
| Rust API | owned results, immutable DB, concurrent calls | A/B | deliberately idiomatic, no magic_t aliasing | API integration and multithread determinism |
| C ABI | all 19 exported libmagic entry points/constants | experimental preview | distinct rfile-ffi library; ownership/state tests, 49/49 original and 440/440 expanded diagnostic scenarios exact; broader source-action and qualification gaps remain; see [ABI plan](remaining/07-c-abi.md) | C/C++ clients under sanitizers, pointer lifetimes, separate-process state matrix |
| packaging | deterministic versioned rule distribution | B/D | embedded pinned fallback, binary `rfile` | build without upstream tree or C compiler |
| Windows/BSD/macOS/big endian | target-specific metadata and filename handling | C/D | Linux tested first; portable core alone is not platform certification | native CI/QEMU |

Intentional changes are restricted to: memory-safe rejection of malformed/overflow states; finite total work/output/database budgets; immutable shareable database and owned Rust results; no external decompression helpers; distinct executable identity; pinned embedded data fallback; optional strict source mode and portable binary output. Every other deviation remains a compatibility gap until explicitly approved as a design change. Do not count a MIME-only match as an exact human-output match, or a buffer match as fd API parity.

Potential upstream defects should be represented as separate expected observations, with minimal reproducer and exact flags/build attached. Preserve defined, safe quirks in compatibility mode; do not reproduce out-of-bounds access, UB or denial of service. A corrected semantic profile must be named/versioned, with paired legacy and corrected tests.

Observed delivered profile: all 103 upstream goldens, the standard differential corpus and the selected extended flag profile pass in the reports. CLI has six documented differences on this sandboxed host (four native gzip guard cases and two FIFO SIGSYS cases). These figures do not certify untested platforms or the remaining ELF/CDF/locale/compiler-diagnostic surfaces. Local-time date rendering currently uses UTC; source warnings, native sort on other endian targets, strict CRC errors, parameter quirks and oversized buffer sampling remain gaps or declared changes.


### R7 confinement compatibility decisions (2026-10-25)

The optional sandbox feature is an intentional process-behavior extension,
not emulation of C SIGSYS failures. Auto preserves tested CLI outputs while
restricting writes, execution and networking. Strict adds bounded preparation
and input descriptor confinement. Its unsupported dynamic/streamed file lists,
preflight-error ordering, 128-entry cap, and separate compile/check/list action
scopes are explicit in [SBX](remaining/05-sandbox.md). `-S` disables restrictions;
last mode option wins. Compiled support is distinct from enforced runtime status.
The optional Linux compile backend uses private descriptor-relative publication;
atomic replacement intentionally changes symlink/inode/mode behavior relative to
in-place writes. This is recorded behavior, not an unreported compatibility fix.


Database CLI actions now have 243 exact explicit-path scenarios in each of off,
auto and strict sandbox modes. Check dumps, compiled preference, partial path
lists, warning caps and compiled artifact hashes are compared; only the executable
diagnostic prefix is normalized. Regular-file-only source IO and finite aggregate
quotas are intentional bounds. Default action discovery, DEBUG and wider source
warning/IO semantics remain open; see [the R8c contract](remaining/07-c-abi.md#r8c-cli-action-integration).

### Claude F-02: bounded decompression (2026-10-31)

The decoder retains its complete-stream contract. Above `Limits.bytes` (default
7,340,032 decoded bytes), gzip/zlib and the other enabled decoders report an
explicit limit error instead of sampling a prefix. Classification returns no
partial inner/wrapper result on this error. A short sampled prefix cannot
establish the integrity of an unread trailer or later members; resource errors
must not turn into confident fallback descriptions. The cap is configurable via
`-P bytes=N` within the independent memory/work/window budgets. Identifying the
wrapper without `-z`/`-Z` avoids decoding. Prefix sampling would require a
separate, explicitly incomplete result contract and is deferred. Boundary
regressions cover 7 MiB minus one, exact, plus one, and raising the configured cap.

### Claude pass 3: truncated inputs and filesystem diagnostics

For truncated inputs, the pinned oracle can evaluate `use` block reads at the
sum of the use offset and block-relative offset while checking only the latter
against the input length. Past-end data can therefore be zero-filled and match.
Rfile checks the absolute position and omits those fragments. This is not generic
large-buffer sampling or a rule limited to end-relative offsets: explicit use
positions also exhibit the oracle quirk. Observed families include gzip, ZIP
with -k, PNG/APNG, PGP and zstd; reported examples include missing `was ""`,
`from FAT filesystem`, `made by v0.0`, PNG geometry, PGP creation and zstd dictionary
fragments. The pass-3 short-input --mime-type sweep matched; extensions and other
inputs are not thereby certified. Rfile does not emulate the omitted bounds check.
Plain UTF-8 selection requires a completed decoded non-ASCII character; an
incomplete trailing sequence alone does not identify UTF-8 (F-04b). Incomplete
tails after a completed character remain tolerated, and BOM handling is separate.

A narrow safe compatibility exception synthesizes an empty value for explicit
lestring16/bestring16 wildcard reads beyond EOF, preserving the native padding
without reading memory out of bounds or relaxing other types/use-block reads.
Invalid CDF sector shifts (0,1,7,8) violate the format's geometry; earlier rejection
may select a different failure stage/message. No unsafe stage parity is promised.

Size-zero regular files are identified as empty from metadata before opening,
in normal/MIME modes even with -E; MIME output uses inode/x-empty. With -s or
annotation-only --apple/--extension that shortcut is disabled and unreadable
files produce the permission diagnostic. Other
unreadable regular files return their permission description in every mode,
including -E, with real-user writable/executable access prefixes on Linux/glibc.
This follows the pinned metadata result; actual stat/other open errors retain
normal -E behavior. For FIFOs, -s content inspection is explicitly unsupported
and returns an error before open, independent of queued data or writer presence.
It never represents uninspected content as empty. The pinned oracle's FIFO -s
error was also reproduced in an unsandboxed distro build; the earlier general
seccomp-only attribution is retracted. Previously recorded annotation-only FIFO
SIGSYS observations are separate, narrowly identified cases.

### Pass-5 remaining annotation/string16 differences

Annotation flags follow symlinks to regular-file targets without -L, including
mixed MIME/annotation requests, and permission descriptions use target metadata.
Annotation requests on symlinks to non-regular targets fail with an explicit
unsupported diagnostic before content open. For a directory target this differs
from the pinned `cannot read ... (Is a directory)` diagnostic; it is a controlled,
documented wording difference, not exact native output.

On direct empty regular files, mixed -i --apple emits the MIME result once.
The pinned oracle appends an annotation result to the metadata MIME result:
an empty-content MIME result if readable, or a permission description if not.
For a regular symlink target, including readable, empty and chained targets,
mixed -i --apple emits only the target result; for an unreadable target it emits
the permission description. The oracle prefixes either result with
inode/symlink; charset=binary. Rfile deliberately does not concatenate these
outputs (F-08d). This differs from 8e33173, which emitted only the symlink MIME
result for mixed flags. Annotation flags still request symlink-target inspection
even when combined with MIME flags.

String16 wildcard compatibility remains deliberately narrow. Pre-existing
non-wildcard comparisons such as =\0 and <z on two-byte inputs can differ where
the oracle compares a synthesized empty value past EOF; rfile does not extend
the wildcard exception to these relations without a justified rule. This
residual stays open. F-03, F-07 inherited-closed stdout, invalid CDF geometry and
shifted-use bounds remain documented maintainer-disposition items, not closed.
