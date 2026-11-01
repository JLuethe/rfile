# Claude pass-9 independent test prompt

You are the independent tester for Rfile. Claude tests and reports; Codex owns
all production fixes and documentation changes. Preserve earlier ledgers and raw
evidence. This pass covers the full golden truncation sweep and the deferred
string16 and F-03 checks.

## Exact candidate and baseline

```text
commit 8325cefd5d3c8c7d22015d8a97c948196330c435
tree   b6e431e70d286914acc5d52417982e99b36151de
oracle 41b1123083ab0bbdfe32f6d4ec14f6aec9a91e59
```

Read first:

1. `/home/john/Code/rfile-qa-claude/pass8/LEDGER-pass8.md` and CHECKPOINT.md.
2. `/home/john/Code/rfile/reports/claude-pass8-codex-review.md`.
3. Pass-7 ledger, pass-3 ledger section 3 and pass-4 ledger's string16 matrix.
4. Candidate docs/03-compatibility.md, docs/rfile.1 and release notes.

Pass 8's full verifiers passed on this exact commit in both local profiles.
Do not rerun them solely to reproduce that result. The test logs include two
intentional caught panics per Cargo stage; both associated tests pass.

Record exact source/tree, feature profiles, toolchain, binary hashes, environment
and oracle identity. Use an isolated clone at the candidate. Reuse hash-verified
pass-8 binaries/private oracle if suitable, or rebuild both profiles and record
new hashes and commands. Local builds remain separate from CI/archive acceptance.

## 1. Full golden truncation sweep

Repeat the full pass-3 scope: all 103 pinned oracle *.testfile goldens, each
available prefix length from 0 through min(64, original length), with -b,
-k -b and --mime-type -b. Record the actual fixture, derived-input and comparison
counts; do not assume "6,700" is an exact count or a count of original fixtures.
Run the complete scope on default and all-feature binaries; use --sandbox=strict
for the latter on the supported host and verify enforcement.

The earlier harness is:
`/home/john/Code/rfile-qa-claude/pass3/tools/f04_sweep.py`.
It strips stdout and saves only differences. Preserve it unchanged; create any
needed test-only extension under pass9/ so the new evidence also captures exact
stdout/stderr bytes, exit status, command, input hash and profile for every row.
Use bounded subprocess timeouts and preserve infrastructure failures distinctly.

Compare against the pass-3 full sweep and pass-7 bounded sweeps. Expect documented
shifted-use differences to remain. F-04b probe/fixture differences should be fixed.
Characterize new differences rather than modifying expectations to hide them.
The recorded pass-5 correction is 16/27 differences in the separate paired F-04
case suite, not the earlier pass-4 ledger's erroneous 17/27.

## 2. String16 matrix

Repeat the pass-4 1,200-case matrix: five lengths, two string16 types, six
relations, ten offsets and two rule levels. Recover the exact historical axes
from retained evidence; if a harness is missing, reconstruct it in pass9/ and
document the reconstruction and any scope difference explicitly. Run both
feature profiles and compare case-by-case with retained pass-4 results.

Verify the wildcard past-EOF empty-string exception and adjacent controls.
The documented non-wildcard =\0 and <z residuals remain maintainer items; do not
change them or call them new regressions solely because they differ from the
oracle. Record actual counts, not an assumed 208 residuals. Check source/compiled
rule parity at least for the minimal wildcard and residual reproducers, recording
exact compiler commands and database hashes.

## 3. F-03 minimal-runtime repro

Inspect `/home/john/Code/rfile-qa-claude/repro/repro_f03_container.sh`.
Its comment records an Ubuntu image digest, but the command uses ubuntu:22.04.
Preserve that script and create a test-only copy that runs the explicitly pinned
image digest. Record the actual container image identity, libc version and absence
of tzdata/localtime. Stage a hash-verified binary copy under pass9/ for container
mounts, preserving earlier evidence. Use rootless, bounded execution and read-only
inputs.

Test the exact candidate binary with TZ=UTC, America/New_York and EST5EDT,
covering date-bearing gzip and plain text, including MIME-only output. Record
both streams and status. Verify that UTC succeeds and unavailable explicit zones
fail with the documented TZ diagnostic, including non-date and MIME-only inputs.
This verifies a documented limitation, not UTC-fallback compatibility or
maintainer approval. If runtime compatibility or container infrastructure blocks
execution, preserve that evidence and report the check incomplete.

## Evidence and boundaries

Write new evidence under `/home/john/Code/rfile-qa-claude/pass9/`:

- LEDGER-pass9.md: identities, commands, complete scope/counts, results, minimized
  new findings, intentional differences, infrastructure failures and skips.
- CHECKPOINT.md: completed/pending work, restart commands and remaining processes.
- SHA256-INVENTORY: hashes of preserved harnesses, inputs, logs and result files.

Preserve initial failed attempts rather than overwriting them. Use owned child
process cleanup, deadlines and incremental checkpoints. Do not edit production
code, the shared checkout, original pinned oracle or any earlier evidence. Do not
publish, tag, replace draft/release assets or launch the 600 CPU-hour campaign.

Close only criteria independently verified. Maintainer dispositions, CI/archive
and exact-asset acceptance, sustained fuzzing, coverage/resource evidence and
independent security review remain separate. End with a concise handoff of any
actionable defect for Codex; no production edits are requested from Claude.
