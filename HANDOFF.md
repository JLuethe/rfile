# rfile restart checkpoint

Updated 2026-10-30, late evening PDT (2026-10-31 UTC). The user authorized
continuing release preparation overnight and explicitly requested independent
GPT-6 Luna subagents after Claude's quota was exhausted. Codex implements fixes;
the testing agents test/retest. Keep this work recoverable; do not publish the
release without the maintainer's final approval.

Start with [release scope](docs/release-scope.md) and the
[release plan](docs/11-github-release-plan.md). The endpoint is an experimental
0.2.0-rc.1 default-feature x86_64 Linux/glibc CLI plus complete vendored source,
not completed security qualification or universal libmagic compatibility.

## Current GitHub / working state

- Repo: https://github.com/JLuethe/rfile. PR #19 passed independent Luna queue QA
  and was squash-merged into main as `5316cb814b2f5e349425f8349ae508cc0f162750`.
- Release PR #23: https://github.com/JLuethe/rfile/pull/23, branch
  `fix/release-readiness-baseline`, now based on main. The feature branch merged
  main to reconcile squash ancestry; that merge changed no source files.
- PR #20 was closed (incorrect MSRV update); PR #1 was closed as superseded by
  the tested pinned checkout upgrade. Optional dependency PRs #21/#22 are
  explicitly deferred. Both `linux` and `msrv` are required branch checks.
- No new release is published. Existing v0.1.0 is unchanged. Do not tag or
  substitute final artifacts until independent acceptance of their exact hashes.
- Packaging, installer, complete vendored source, checksums, release notes,
  release-mode CI and bounded pinned-oracle CI are implemented. Artifacts are
  built from clean immutable Git snapshots. Current binary requires GLIBC 2.34;
  Ubuntu 22.04 is the intended oldest executed runtime, not a claim for every
  glibc 2.34 distribution. Rust 1.87 remains the source-build minimum.

## Independent reviews and open acceptance work

- Initial Claude baseline/retests: `reports/release-review-2026-10-30/` and
  `reports/release-readiness-2026-10-30/`. Correct fixture coverage, fail-closed
  verification reporting and actual sandbox coverage passed. The queued Claude
  retry was cancelled when the user requested Luna; do not assume it is active.
- Luna queue QA: `reports/release-0.2.0-rc.1/queue-qa/`, passed bounded locking,
  duplicate rejection, partial accounting and deadline cleanup. It used explicitly
  identified prebuilt binaries and makes no sustained qualification claim.
- Default full oracle CI passed on `0c04563`. New Ubuntu CI exposed the known
  native FILETIME uninitialized-seconds path. The precise harness correction
  passed Luna's 37 direct vectors, 10,875 semantic cases (10,848 exact + 27
  separately validated undefined-reference cases), and 2,850 date cases (2,820
  exact + 30 separately validated). No production date semantics changed.
- The all-feature CI then exposed 64 ZIP64 native gzip-helper text variants.
  `dc103ea` preserves exact supplemental whole-observation hashes, requires
  unchanged Rust results/status and records the helper version/hash. Luna independently accepted the exact 64-case replay, rejected mutated Rust/native
  observations, and passed the local 3,268-case codec matrix. Fresh full CI remains
  required.
- Artifact QA found the fresh-cache checker could lose rustc when the caller's
  PATH omitted ~/.cargo/bin. `9f4d20f` preserves the selected Cargo toolchain
  directory; a follow-up bounds default fresh-build parallelism to one job.
  Luna is retesting the original failure and exact archives.
- Artifact QA also found the default CLI failed in minimal Ubuntu without tzdata.
  `09f2c36` adds UTC for an absent implicit system zone and built-in `TZ=UTC`;
  explicit invalid zones and malformed/unreadable existing files still fail.
  Luna accepted the fix through 13 constrained-container checks. The reusable
  checker is integrated into candidate CI; final exact assets remain pending.
- Fresh 25-target sanitizer smoke and two FFI Miri tests ran in an isolated
  clone. Read its actual manifests/logs and final reviewer report before using
  the result. No target met its 24-CPU-hour gate.

## Active agents / durable scratch

Use the live collaboration inventory if this thread resumes; never assume an
agent or process survived an interruption. Current testing tasks are:

- `luna_independent_qa`: queue + FILETIME + ZIP64 comparison review. Evidence in
  `/tmp/rfile-queue-review-qa/` and `/tmp/rfile-night-review/`; isolated source
  `/home/john/Code/rfile/.reference/release-acceptance-source`. Its reusable
  Python regression is integrated as `tools/test_oracle_boundaries.py`.
- `luna_artifact_qa`: exact CI archives from run `41176219039`, source
  `0f1054d2404d8049d2d2357e22af97c5e8f22d3c`. Scratch/evidence under
  `/home/john/Code/rfile/.reference/package-qa-0f/`. It must retest final assets.
- `luna_sanitizer_qa`: isolated clone
  `/home/john/Code/rfile/.reference/release-rc1-sanitizers`, build commit
  `408c940fcc37e938d6f9e47ef43eddd2641872af`. Read `qa-evidence/` and the actual
  `.reference/fuzz-queues/latest.json` within that clone. Final-candidate replay
  is still required before attaching this evidence to a different source SHA.

## Next release actions

1. Finish independent ZIP64 and artifact retests. Integrate the portable Luna
   regression and invoke report-contract tests only against fresh oracle reports.
   Archive raw findings and retests with truthful model/source identities.
2. Reconcile docs/PR body, commit remaining fixes/evidence, and require all CI
   lanes to pass. Merge PR #23 only after independent blockers are closed.
3. Freeze the final commit, build exact CLI/source archives (prefer Ubuntu 22.04
   CI), and have agents run the complete final oracle/strict-sandbox matrix,
   artifact checks, source rebuild and bounded sanitizer/Miri replay against
   that identity. Compare two clean builds for any reproducibility statement.
4. Prepare an unpublished GitHub release draft with exact assets, checksums,
   evidence, supported scope and remaining limitations. Keep main protection,
   existing tags and the clean primary oracle intact. Final publication is for
   the maintainer to approve in the morning.
5. Preserve future fidelity/security work separately. The full 25-target minimum
   is 600 measured CPU-hours plus branch/resource evidence and independent review;
   neither overnight smoke nor a green CI run closes that gate.

## Reference / resources

The primary oracle is `/home/john/Code/rfile/.reference/file-clean`, clean commit
`41b1123083ab0bbdfe32f6d4ec14f6aec9a91e59`. Always set RFILE_REFERENCE explicitly.
The old `upstream/` retains a local seccomp patch; never reset it or call it clean.
`.reference/cdf-pointer` is an explicitly patched auxiliary oracle only.
Cargo is `/home/john/.cargo/bin/cargo`. Default/all-feature runs must be serialized
within a checkout. Use separate checkouts and CARGO_BUILD_JOBS=1 while other builds
run; /tmp is RAM-backed, so large temporary builds should use scratch on disk.
Rootless Podman works for disposable unprivileged offline runtime checks; the
Docker socket does not. Do not change global configuration to work around that.

## Historical 2026-10-25 validation

- All 33 stages of `tools/verify.py --all-features --benchmark` passed:
  145 ordinary debug/release tests; 143 isolated default tests; Rust 1.87;
  unchanged v21 database and matching benchmark checksum (warm ratio 1.607).
- ABI state: 49/49 original scenarios and 440/440 expanded diagnostic scenarios
  exact. The six previously frozen diagnostic differences are now resolved.
  Strict Rust source constructors retain their existing first-error behavior.
- 582/582 valid source-check dumps pass. Seven new ordinary source regressions
  include stale continuation state and rejecting oversized Pascal-string records
  before compilation could emit an unimportable record.
- Two applicable FFI Miri tests pass with `LC_ALL=C TZ=UTC0` and
  `MIRIFLAGS='-Zmiri-env-forward=LC_ALL -Zmiri-env-forward=TZ'`.
  Miri does not establish host timezone/locale/kernel correctness.
- 25 ASan fuzz targets build. A resource-constrained all-target smoke completed
  81.238734 measured CPU seconds, separately archived in fuzz-queue-all-smoke.json.
  Duplicate-job rejection, partial-run status and deadline cleanup pass.
  No sustained target has met its 24-CPU-hour gate. Review actual live manifests
  and process identities before launching duplicate work.
- Historical isolated packages passed dynamic/static clients, default discovery
  and an offline vendored-source rebuild. The latest source-provenance report
  records `1198c0e` and 966 matching files. Those artifacts must be rebuilt and
  retested for a later release candidate.
  cargo-audit had no findings at its recorded RustSec revision; independent
  review has not occurred. No global installation/system replacement/new release.

## Interruption recovery

[Recovery specification](docs/10-execution-and-recovery-spec.md) remains the
contract. No permanent agent auto-restart service is installed. Check the live
agent/process inventory and each isolated checkout's actual queue manifest before
restarting work. A surviving process is not proof of a completed test; retain its
exit status and logs. Do not duplicate a queue or combine campaign accounting
across changed source/binary/supervisor identities.

Continuation: read this checkpoint, release scope and independent QA reports;
continue the experimental release through fixes, independent retests, final
artifact acceptance and an unpublished draft. Codex fixes; independent testing
agents test/retest. Preserve all uncompleted security gates explicitly.
