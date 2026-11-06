# Pass-8 source-verifier evidence review

Reviewed Claude's pass-8 ledger, checkpoint, structured reports, Cargo test logs,
environment/working-tree records and SHA256 inventory. Original evidence remains
at `/home/john/Code/rfile-qa-claude/pass8/` and was not modified.

## Accepted scope and identity

The source-verifier criterion passes for these two local Linux profiles on the
reported host. There is no product or infrastructure defect requiring a fix from
this pass. The source candidate remains:

```text
commit 8325cefd5d3c8c7d22015d8a97c948196330c435
tree   b6e431e70d286914acc5d52417982e99b36151de
```

| Profile | Completed steps | Duration | Debug / release test results |
|---|---:|---:|---|
| Default | 31/31 | 399 s | Each: 159 passed, 0 failed, 1 ignored |
| All features, required sandbox | 33/33 | 454 s | Each: 161 passed, 0 failed, 1 ignored |

Execution records, dated 2026-11-01 UTC:

- Default: `20261101T010551Z-24638e88927843708a76eba9d13dfd45`.
- All features: `20261101T011230Z-d4bf02972ec44e7497e65476c9156977`.

Both structured reports record the exact candidate HEAD and identical source
digest `d5b49cb1e5cbec6feafb8c642cc95e2b9293fd069e38537434c8cdfec2b29e04`.
All step statuses are zero and states are passed. Cargo test counts were
independently summed from each stage's logs during this review.

Default release binary:
`12dab56e7cea9d6064d23201184ed6c6f805911f440a6e2a7e94daa8149c1188`.
All-feature release binary:
`a8228f19ef9dde8f77b661613e34166bd50f2ff4b6d7abc0bc68729caa29756a`.
These agree with pass-7 recorded hashes. They are local builds, not CI archives.

## Coverage, provenance and integrity

All-feature coverage records show both sandbox_cli and sandbox_kernel executed
in debug and release, with eight kernel child cases in each stage. The default
profile's sandbox_cli not-compiled skips are expected. Mutation-fixture coverage
executed with 103 files in each profile; this remains smoke evidence.

The private oracle is pinned to
`41b1123083ab0bbdfe32f6d4ec14f6aec9a91e59` with a clean tracked worktree.
Both reports bind its database/configuration/executable hashes. The database
hash is `8eab57da91fd9cc4c7af3b8a4369a64d32124aa55dc376207a81e69abd4a5ac8`.
The original pinned oracle was preserved; make check ran in the private copy.

All **133 inventory entries** were rehashed and matched during Codex review.
Inventory SHA256:
`4be5864d3f990f336cd36937b4dbff1fb2720f74b267763ae92aa4abbd5f28a0`.

The clone initially had no status entries. Verifier startup writes its summary
before recording git_status, so generated reports appear dirty in the execution
records. The final saved status lists 23 modified files, all under reports/;
this includes runtime-linkage.txt, not only JSON files. No source changes or
commits are recorded in that clone.

## Reporting correction: intentional caught panics

The original ledger says there were no panics in any log. Each debug/release
Cargo test stage actually contains two intentional panic-hook messages:

- `panic_is_contained_and_cookie_is_poisoned`: controlled C ABI boundary test.
- `guard_restores_on_success_error_and_unwind`: controlled locale unwind test.

Both tests report ok. These are expected caught test panics, not failing tests
or newly discovered product crashes. This correction preserves the original
ledger unchanged and does not alter the passing verdict. Do not repeat a claim
that the logs contain no panic messages.

## Next work

Claude's pass 7 independently verified F-04b fixed and F-08d documentation
accurate. F-08c's annotation-only unreadable subset was verified in pass 6.
Maintainer acceptance of documented differences is still separate.

Next independent test batch: full golden-fixture truncation sweep, deferred
string16 matrix and F-03 minimal-container repro on this same source candidate.
See [the pass-9 prompt](claude-pass9-test-prompt.md). The legacy sweep normalizes
stdout with strip() and saves only differing rows; the next pass should retain
byte-exact paired output/status evidence as well. The legacy F-03 script's
comment records a digest but its command uses a mutable image tag; use a new
test-only copy pinned to the actual image digest, preserving the original script.

CI-built archive and exact-release-asset acceptance, full golden-sweep acceptance,
the 600 CPU-hour fuzz campaign, coverage/resource evidence and independent
security review remain open. Test coverage markers and an unsafe inventory are
not substitutes for security qualification. No publication, tags, draft assets
or packaging changes were made in this review; subsequent documentation commits
do not replace the tested candidate identity above.
