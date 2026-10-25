# Prioritized implementation and verification plan

All work depends on the pinned baseline in [architecture](01-architecture.md). The specification was written first; implementation follows it and must publish evidence rather than claiming automatic full parity.

## Oracle and reproducibility

Reference invocation: built `upstream/src/file -m upstream/magic/magic.mgc`, with `LC_ALL=C`, `TZ=UTC`, unset `POSIXLY_CORRECT`, explicit flags, fixed working directory and feature manifest. Never compare against unpinned `/usr/bin/file` without labeling it a second oracle. Pin reference commit, magic SHA-256, configure switches, CPU/OS/libc, toolchain versions, limits and environment in every machine-readable run. First run upstream's own `make check`; a broken oracle invalidates differential conclusions.

Capture stdout and stderr as **bytes**, return code and terminating signal, timeout, wall time and path/API entrypoint. Do not normalize whitespace, description numbers or diagnostics when counting exact matches. For list operations with different argv[0], classify intentional program-name differences separately. A missing expected-error comparison is not a pass. Keep complete mismatch records with both byte strings (hex/base64 for invalid UTF8) and reference build identity. Use subprocess timeout and input/output bounds to keep either side from stalling the harness.

Test matching semantics with identical compiled database first to isolate evaluator from compiler. Then test source-compiled databases, directory ordering and colon-separated lists. Compiler tests compare record fields, group order and classifications, with a separately labeled normalized comparison zeroing only explicitly enumerated provenance/padding fields. Never call normalized equality byte equality.

Library differential uses ctypes/C reference driver for `magic_buffer`, `magic_file`, `magic_descriptor`, repeated calls/error resets, flags/getparams/setparams, unloaded state and buffer load ownership. Rust test driver emits the same bytes/errors/config. File-descriptor tests check seek position restoration, descriptor lifetime and pipes; compare buffer vs file only within the same API mode. Owned Rust results intentionally survive next calls. Concurrent Rust calls must give deterministic results from a shared Arc database.

## Corpus and disagreement resolution

Priority corpus tiers:

1. All 103 upstream `.testfile` fixtures with their `.magic` overrides and `.flags`, expected `.result`, **and** current oracle output. Existing fixtures sometimes test API behavior rather than default command output. Keep both result channels.
2. Synthetic magic-language vectors covering every type/operation/modifier, mixed continuation levels/default/clear, text/binary classification, sort ties/strength and duplicate names, named-base/endian flip, direct and nested indirect offsets, EOF `-0`, boundary one-byte/empty input, NUL/escaped whitespace and format-width truncation.
3. Metadata matrix: regular empty/nonempty, directory, symbolic/broken/loop links, sockets, FIFO, unreadable modes as unprivileged user, char/block devices where safe, procfs links, sparse >4GiB files, invalid UTF8 filenames, atime, filename alignment, list/stdin processing and pipe errors.
4. Deterministic collection of thousands of local public/source/binary files using bytewise sorted paths and SHA-256 manifest. Bound bytes per file and avoid secrets/private home data, files that change during reading, generated target trees and pseudo-filesystems. Stratify by result/MIME/type so repeated text files do not mask rare structured failures. Never redistribute arbitrary local binaries without rights review.
5. Malformed and hostile corpus: truncate every fixture at structural boundaries; corrupt mgc counts/types/strings/version; overflowing offsets/masks/pstring lengths; recursive use/indirect cycles; pathological regex; huge JSON nesting; invalid DER length; cyclic CDF SAT/mini-SAT; ELF bad counts/offsets; tar checksum, ZIP traversal names (never extract), gzip bombs and nested layers. Historical CVE fixtures only with known provenance; do not claim full CVE coverage from naming a test.
6. Feature/platform grid: Linux glibc and musl, macOS, FreeBSD, Windows, little/big endian, 32-bit where possible; codecs compiled in/out, CLI sandbox enabled/disabled, locale C/UTF8, UTC and non-UTC. External helper output requires a separate feature profile because Rust does not invoke helpers.

On disagreement attach three labels: **documented** (manual line), **observed** (oracle trace/output and minimal reproducer), **suspected defect** (reason and safety effect). A manual does not override an observed safe quirk silently. Undefined behavior cannot be an exact deterministic oracle. Fix the spec or implementation as appropriate, keep a regression, and report unresolved disagreements. No training the engine to hardcode fixture filenames/output or delegating detection to oracle binaries.

## Fuzzing design

Separate targets: compiled database decoding; source tokenization/parser/compiler; evaluator on arbitrary data with fixed pinned database; combined arbitrary valid rules+data; description formatter; encoding; DER/JSON/CSV; compression; future ELF/CDF. Feed parser output into compile/decode/evaluate for roundtrip invariants. Truncated/mutated compiled databases must never panic; successful decode must satisfy node-count/group/field invariants. Output and work always bounded. Add deterministic mutation smoke suite runnable with stable Cargo before long libFuzzer runs.

Use cargo-fuzz/libFuzzer in an isolated optional package, stable sanitized C oracle for differential fuzzing, Miri where applicable and platform sanitizers for the optional ABI only. Seed from public regression fixtures and compact generated magic snippets. Minimize every crash or timeout, preserve seed and feature/limit manifest. Acceptance for release includes at least 24 CPU-hours/target with no unresolved crash or unbounded resource issue; smoke tests are not a substitute and must be labeled with iteration count/time.

## Performance protocol

Benchmark release builds with pinned rules, identical bytes/limits/locale/features on the same machine. Separate cold database decode/regex compilation from warm per-buffer evaluation; measure tiny unknown data, common images/PDF/ELF, text with regex, high-latency file paths, near-limit data, rule misses and adversarial bounded cases. Prevent caching/skew by interleaving engine order, repeat >=10 runs, record median/p95/p99 throughput, peak RSS, CPU, wall time and exact corpus manifest. OS process startup benchmarks must be distinct from in-process library throughput. Do not claim a speedup based on a different detector set or truncated work.

A reproducible smoke benchmark may run sooner, but optimization gates require no loss of exact matches, finite resource ceilings and a profile demonstrating the bottleneck. Prefer precomputed group boundaries, cached regex and checked slices before unsafe micro-optimization. There is no arbitrary performance target until baseline is measured; provisional goal is <=2x warm runtime/RSS on balanced corpus at equivalent scope.

## Milestones and acceptance

| Priority/milestone | Dependencies | Concrete deliverables | Acceptance and remaining gaps |
|---|---|---|---|
| P0 baseline/spec | none | commit+features+hashes, successful C build/tests, five Markdown specs | reproducible references and explicit unavailable environments; complete before Rust source |
| P0 safe core | baseline/spec | Cargo lib/bin, limits/errors, checked v21 loader, primitive rule interpreter | invalid headers/truncation never panic; primitive vectors+runtime no libmagic; C ABI absent |
| P0 evaluator/source | safe core | source lexer/annotations, sorting, continuations, masks, named/indirect, search/regex | all upstream fragments parse or fail explicitly with enumerated gaps; roundtrip and semantic vectors; POSIX regex gap measured |
| P1 useful CLI/text | core/evaluator | args/framing/stdin/list/MIME/encoding/metadata, pinned rules packaging | reproducible fixture and CLI differential reports, default input cases exact; no claimed full structured parity |
| P1 builtin formats | CLI/text | tar/CSV/JSON/SIMH/DER, gzip, ELF/CDF | malformed safety tests; detailed descriptions match supported scope, feature limitations explicit |
| P1 broad verification | prior milestones | thousands-file manifest, mode-by-mode exactness, API comparisons, mutation tests, smoke benchmark | every mismatch retained/prioritized; no crash/timeouts; percentages include denominators/modes |
| P2 fidelity closure | mismatch triage | POSIX longest regex, exact formatting/timezone, remaining codecs/platform/sandbox behavior | full stated fixture suite exact, zero unclassified mismatch in supported profile; non-goals not counted |
| P2 hardening | stable semantics | fuzz campaign, limits audit, concurrency tests, CI platform grid | 24 CPU-hours/target, no unresolved critical issue, deterministic bounded output |
| P3 optional ABI/release | fidelity+hardening | isolated ABI, headers, packages/SBOM/manpages, signed release metadata | C client symbol/lifetime tests; reproducible build without C/runtime oracle; no release unless gates met |

Do not mark P2/P3 complete because P0/P1 compiles. A working independently detecting binary is a useful deliverable; replacing libmagic in mail scanners requires the full safety and compatibility gates. If time/environment prevents finishing a gate, leave an actionable failing/mismatch artifact and state exactly what remains.


## Reference identity enforcement (2026-10-25)

Use `RFILE_REFERENCE=/absolute/path/to/clean/pinned/checkout` to select a separate
reference worktree. Otherwise verification tools use `upstream/`. The shared
`tools/reference.py` requires the exact pin and rejects tracked modifications;
`tools/build_reference.sh` refuses to build a modified reference instead of
resetting user changes. The full verification report records source cleanliness,
configuration hashes, executable/library/wrapper hashes, database hash and native
version output. Untracked generated build files alone are not source changes.

During R7 review the existing local `upstream/src/seccomp.c` was found to include
a previously unrecorded pselect6/pselect6_time64 allowance. It is preserved and
recorded in `reports/reference-audit.json` and the accompanying patch. A separate
clean worktree at the same pin was built for the final replay; no mismatch
allowlist was broadened. Historical CLI counts are observations against their
then-local executable, not proof of a pristine pinned CLI. Library detector
source was unchanged, but final evidence still reruns against the clean build.
