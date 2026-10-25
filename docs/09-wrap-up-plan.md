# Wrap-up plan and repository review

Historical review: 2026-10-24. Current execution state is in [HANDOFF](../HANDOFF.md) and [the delivery ledger](08-delivery-progress.md). This checkpoint distinguishes implementation delivery from
complete compatibility and security qualification. Detailed contracts remain in
[the remaining-work specification](07-remaining-work-spec.md).

## GitHub versus local state

- Public repository: [JLuethe/rfile](https://github.com/JLuethe/rfile).
  Project license is GPL-3.0-only; upstream and dependency notices remain.
- Reviewed main: `25f4135f1144528cca89a4cfc5954e2ccacf067f`, locale PR #11.
  The active local implementation checkout matched `origin/main` exactly before
  checkpointing research. No tracked source modification was waiting to push.
- Oracle: file 5.48, commit `41b1123083ab0bbdfe32f6d4ec14f6aec9a91e59`.
- PRs #2 and #4–#11 are merged. Latest main CI passed. Hosted Linux CI runs
  formatting, default/all-feature lint/tests, a release build and rule rebuild;
  MSRV checks Rust 1.87. It does not run the entire local differential matrix.
- Protected main requires an up-to-date `linux` check and PR, applies to admins,
  and disallows force pushes/deletion. Required reviewer count is zero. MSRV
  runs but is not a required protection context; wait for both jobs before merge.
- The only open PR before this review was Dependabot #1 (checkout v4 to v7).
  Review its runner/runtime requirements separately from detector work.
- Published release is still **v0.1.0 experimental prerelease**. That historical
  tag is not current main. No new release, crates.io package or global installation
  was made during this review.
- Local branch `main` was seven commits behind `origin/main`; the active feature
  branch was current. This is a stale branch pointer, not unpushed implementation.
  Fast-forward local main when next used.

Only `tools/float_compare.py` and `reports/float.json` were untracked at review
start. They are now committed and pushed on **`feat/format-time-edges`**, at
[`463de0953b61b62a7f3b317ee1df2a3af95c5c13`](https://github.com/JLuethe/rfile/commit/463de0953b61b62a7f3b317ee1df2a3af95c5c13),
separate from main. This is a failing research baseline, not a formatter fix.
This review validated Python syntax, JSON structure/counts and binary hashes;
it did not rerun the differential or the full test suite.

Ignored local assets include `upstream/`, auxiliary reference builds, binaries,
toolchains, corpora, artifacts and logs. A fresh clone must rebuild/reacquire
these; tracked reports are evidence, not a complete copy of the working machine.
No build or fuzz worker was running when checked during this review.

## Delivered work and evidence

| Area | On main | Evidence and qualification |
|---|---|---|
| Original specification and engine | Architecture, behavior, compatibility and implementation docs; independent Rust CLI/API and magic parser/compiler/evaluator | Specification preceded implementation; no production libmagic wrapper or decompression helpers |
| Shared controls | R1 / PR #5: positional IO, shared budgets, cancellation, profiles and capabilities | Tracked memory reservations are not an allocator-wide RSS ceiling |
| Date foundations | R2 / PR #6: immutable zones, checked local/UTC families, explicit FILETIME profiles | 1,022 exact comparisons; exceptional custom zones remain |
| CDF | R3 / PR #7: sector/mini-stream/FAT/DIFAT traversal and properties/vectors/directories/catalog evidence | 400 comparisons: 385 exact + 15 named resource-limit results; corrected metadata deliberately differs from defective pinned behavior |
| ELF/core | R4 / PR #8: checked notes, process/AUXV, OS/build IDs, Solaris capabilities, bounded stdin spooling | 5,461 exact comparisons, synthetic LE/BE 32/64-bit and sparse large-offset fixtures |
| Codecs | R5 / PR #9: opt-in bzip2, XZ/LZMA/lzip, Zstd, modern LZ4, UNIX compress and first-entry ZIP | 82 fixtures; 3,268 comparisons: 2,444 exact + 824 individually documented decisions; historical/dictionary/descriptor gaps remain |
| Regex | R6a / PR #10: GNU byte-ERE front end and bounded captures/backreferences | 27,556 comparisons: 23,396 exact + 4,160 explicit unsupported results; unsupported is not conformance |
| Locale/output | R6b / PR #11: immutable Linux/glibc display contexts, widths/escaping, C-string boundaries and diagnostic bytes | 1,660 exact comparisons across C/C.UTF-8/en_US.UTF-8; isolated unsafe locale adapter |

[verification-extended.json](../reports/verification-extended.json) records
**24 passing stages and 113 ordinary tests in debug/release**. Broad API results
are 22,544 exact comparisons; additional flags 11,000; custom rules 10,875;
upstream fixtures 103/103. The CLI matrix retains six named differences
(102/108 exact). Recompiling 361 selected source files reproduces all 25,176
v21 records, SHA-256
`8eab57da91fd9cc4c7af3b8a4369a64d32124aa55dc376207a81e69abd4a5ac8`.

These overlapping test dimensions are not a count of distinct files or universal
compatibility. Historical reports belong to their recorded source/binary versions.
There are 22 buildable fuzz targets and short sanitizer runs, but no completed
sustained campaign or independent audit. The detailed evidence and limitations
are in [the delivery ledger](08-delivery-progress.md).

Review corrections: the ledger still described merged locale work as local;
issue #3 still described merged regex work as local; SECURITY.md incorrectly
said all first-party code forbids unsafe. This review updates those statements.
The core prohibition remains; `rfile-platform` contains the inventoried locale
ABI boundary. Earlier milestone narratives are historical, not current totals.

## Execution plan and acceptance criteria

Each row is a separately reviewable PR or small PR sequence. Do not combine
formatter repairs, kernel policy and foreign-pointer code into one change.

| Order | Work and dependencies | Deliverable and acceptance |
|---|---|---|
| 0 | Durable checkpoint and accurate status | Preserve research separately, update tracker/security claims, add restart handoff; distinguish merged, local, historical and unfinished work |
| 1 / R6c | Float formatting and exceptional dates; resume research branch | Repair precision clamping, alternate decimal points, sign/zero padding and nonfinite output; extend timezone vectors. Every mismatch exact or individually justified/profiled; source-compiler errors separate from runtime; boundary/resource regressions; default/all-feature/MSRV/full-matrix checks |
| 2 / R6d | Remaining semantic decisions after R6c | Implement native quantified-capture histories or retain an explicitly restricted release profile; resolve source grammar, collation, locale/date and profile-window scope. Supported forms exact through source/v21; unsupported forms visible. Full native fidelity stays open wherever restrictions remain |
| 3 / R7 | Linux CLI confinement after setup/CLI contracts stabilize | Ordered action plan, trusted setup boundary, auto/strict/off modes, descriptor authority and actual capability reporting. Child tests prove denial and allowed behavior; strict fails before classification on installation failure; `-S` works. Library calls never restrict their host process |
| 4 / R8 | C ABI plus source diagnostics/list compatibility prerequisite | All 19 functions in `rfile-ffi`, header/pkg-config/distinct shared library. C clients compare output/error/errno/state, parameters, loads, ownership, descriptor position and pointer lifetime. Sanitizer/ownership tests and isolated-prefix packaging; no system library overwrite |
| 5 / R9 | Freeze enabled implementation/features, including ABI targets | Final manifests, coverage, fault/concurrency/resource tests, dependency/unsafe review. At least 24 measured CPU-hours per mandatory enabled target, two seeds each, final-candidate replay, no unresolved high/critical findings and an actual independent review |
| 6 / R10 | Applicable preceding gates pass | Reproducible source/binary/database packages, notices, installation tests, capability/gap matrix and same-candidate verification/benchmarks. Stable/full-replacement claims blocked by unmet gates |

R6c baseline: **6,630 comparisons, 4,078 exact, 2,552 differences**, covering
26 formats and deterministic f32/f64 bit patterns. Differences include precision
>64 clamping, alternate punctuation, sign/zero padding and native `%G`/`%F`
runtime rejection. The latter needs an explicit pinned/corrected decision,
not an assumption that standard printf behavior equals this particular oracle.
The harness patches only the description field of a same-kind compiled template
to distinguish source-validator rejection from runtime behavior. These are not
6,630 successfully compiled source programs. Review the harness before treating
it as the final acceptance gate; do not broadly allowlist its failures.

Date follow-up includes perpetual-DST standard-time interpretation, unusual TZif
transitions/footers and extreme ranges. Non-C numeric locale and caller rounding
mode require explicit contracts before claiming C ABI equivalence.

CDF/ELF implementations are delivered, but complete capability/branch matrices,
independent fixtures and final security gates remain. Historical codecs, external
dictionaries and ambiguous ZIP descriptors remain named gaps. Extra codecs stay
off by default until activation gates pass. Do not relabel gaps to close a checklist.

Only x86_64 Linux/glibc is executed. Native macOS/BSD/Windows, musl, 32-bit,
ARM and big-endian certification are separate work packages. Cross-compilation
or swapped-endian fixtures cannot establish native runtime support. The first
release can be explicitly Linux-only without claiming those gaps are resolved.

## What wrapping up means

There is substantial working code; completion is not one cleanup commit. R6c is
the next bounded task. Kernel confinement and the C ABI are two significant
implementations. Independent review requires a separate reviewer; none has been
scheduled. No reliable completion date is established.

The 24-CPU-hour gate across all 22 existing targets alone implies **528 measured
CPU-hours**, before new targets or reruns. Ideal four-worker elapsed time would
be 132 hours (5.5 days) with adequate memory and full CPU utilization. That is
a planning lower bound, not a forecast. This review started no long campaign.

Use two explicit endpoints:

1. **Feature-complete experimental candidate:** requested implementations,
   supported-profile/gap matrix, deterministic/differential checks, sanitizer
   smoke and isolated installable artifacts. Still experimental.
2. **Qualified release:** sustained campaigns, independent review, resolved
   release-blocking findings and evidence-backed platform/compatibility claims.
   A full replacement claim still requires any native fidelity gaps to close.

## Content restrictions, crashes and recovery

The reported “This content can’t be shown” notice is consistent with an automated
content restriction. Alone it does not establish an app crash, Rust crash,
out-of-memory event or lost Git work. No app crash log/request diagnostic was
available here; the exact trigger is unknown. Thread length or a particular
file cannot be identified as the cause from this notice.

OpenAI's [additional safety checks guidance](https://help.openai.com/en/articles/20001326)
recommends narrowing allowed requests to the defensive outcome and necessary
context, and using feedback/Support for repeated mistaken blocks. Daybreak is
an official access route for eligible authorized work; approval does not remove
every safeguard or guarantee an answer. Changing wording does not change whether
a request is allowed. No checkpointing practice guarantees prevention of notices.

The [official troubleshooting guidance](https://help.openai.com/en/articles/20001259-troubleshooting-openai-daybreak)
explains checking a supported app version, approved account/workspace/project
and enabled access where applicable. Report persistent unexpected restrictions
with exact notice, model/product, timestamp/time zone, request ID if available,
and a redacted description of the owned-repository maintenance task. If the app
also exits/freezes, record that separately with logs; do not assume the notice
explains a process failure.

For continuity regardless of cause:

- Use one bounded milestone per session, with repository, commit, intended
  change and acceptance tests explicit.
- Commit/push coherent checkpoints. Keep failing research labeled and off main.
- Maintain [HANDOFF.md](../HANDOFF.md): branch/commit, completed checks, known
  failures, running jobs and exact next action. A new session should not need
  the entire conversation to reconstruct the project.
- Save long-running logs/results and manifests to disk. Inspect process state
  before restarting interrupted jobs; neither completion nor termination follows
  from a chat disconnect.
- Summarize logs using counts/hashes/paths rather than dumping full artifacts
  into chat. This helps review and context management; it does not change access.
- Use a fresh task from the handoff for context management when needed, not as
  a restriction bypass. Preserve truthful ownership/authorization and scope.

No reliable lifetime conversation token total was available from this review.
Git history, file sizes and test counts do not measure token usage.
