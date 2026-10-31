# Retest of `0e9590d`: PASS for this increment

The Clippy `print_literal` failure that blocked the last retest is fixed. Every gate I ran passed, and so did the full integrated verifier.

- **Source:** commit `0e9590d252383932c44c53d9d0402ff7c32fe8df` on branch `fix/release-readiness-baseline`, in the isolated clone `/tmp/rfile-readiness-final`. The tree was clean before the run.
- **Model:** Claude Opus 5.5 (`claude-opus-5-5`)
- **Toolchains:** stable rustc 1.99.1 and the 1.87.0 toolchain that was already installed. Nothing was installed or downloaded.

## 1. Gates
All commands ran with `--locked --offline` and `CARGO_NET_OFFLINE=true`.

| Gate | Exit | Notes |
|---|---|---|
| `cargo fmt --all -- --check` | 0 | |
| Clippy, default features, workspace, all targets, `-D warnings` | 0 | 0 warnings |
| Clippy, all features, workspace, all targets, `-D warnings` | 0 | 0 warnings |
| `cargo test --workspace` (default features) | 0 | 143 passed, 0 failed, 1 ignored, across 27 suites |
| `cargo +1.87.0 check --workspace --all-targets --all-features` (target dir `target/msrv-1.87`) | 0 | 0 warnings |

The Clippy runs finished in 3–4 seconds, so they probably used cached build output. Clippy still re-checks crates whose source changed, and none of the runs printed a warning.

## 2. Integrated verifier: exit 0, `passed`
- **Command:** `timeout 300s env -u RFILE_REFERENCE CARGO_NET_OFFLINE=true python3 tools/verify.py --all-features --require-sandbox --no-oracle` (182 s)
- **Run ID:** `20261031T053545Z-48894d7255384c7591b87c93ed44a2a4`. It is the only run directory.
- **Report:** `reports/verification-runs/<run_id>/report.json`. It is byte-identical to `reports/verification-extended.json`.
- **Identity:** the recorded git head is `0e9590d…` and `source_sha256` is `18e1ea93…ad389`. The requirements are `{oracle: false, sandbox: true}`.
- **Steps:** all 12 passed with exit 0 (format, clippy, cargo-test, cargo-release-test, cargo-build, ffi-build, ffi-rust-clients, unsafe-inventory, database-rebuild, fuzz-smoke, fuzz-format, fuzz-build).
- **Log hashes:** I recomputed the hash of every step log and all 12 match.
- **Sandbox coverage:** `sandbox_cli` and `sandbox_kernel` (8 child cases) are both recorded as `executed`, in both cargo-test (debug) and cargo-release-test (release).
- **Mutation fixtures:** recorded explicitly as `skipped`, with `required:false` and 0 truncations. The smoke still ran 10000 database, 10000 source and 10000 random-input cases with seed `4f6c655275737421`.
- **Archived artifacts:** `abi-clients-rust.json`, `unsafe.json`, `runtime-linkage.txt`.

## 3. Fixture contract
I ran the hardening test in release mode with all features and `--ignored`.

| Case | Exit | Result |
|---|---|---|
| `RFILE_REFERENCE=/home/john/Code/rfile/.reference/file-clean` | 0 | 103 files, 1339 truncations, seed `4f6c655275737421` unchanged, 10000 cases of each kind |
| `RFILE_REFERENCE` pointing at a path that doesn't exist | 101 | panic at `tests/hardening.rs:117` |
| `RFILE_REFERENCE` unset and `RFILE_REQUIRE_FIXTURES=0` | 0 | `mutation_fixtures` recorded as `skipped`, `required:false` |

I did not repeat the other boundary cases from the previous report (empty reference, `RFILE_REFERENCE=""`, opt-out with a bad reference).

## 4. Reusable VerificationRun regression test
- **File:** `/tmp/rfile-readiness-final-qa/test_verification_run.py` (sha256 `46507b46…6e6e`).
- **How it finds the module:** from `RFILE_TEST_TOOLS`, or else the script's own directory, so it also works once copied into `tools/`.
- **How it runs:** every test builds a fresh git repo in a temporary directory and drives the real `VerificationRun` with small real subprocesses. Nothing is mocked. A failed expectation makes the script exit nonzero.
- **Cases covered (10):**
  - A stale pass is replaced at startup; the test calls `identify()` before completing normally.
  - Completing without `identify()` is recorded as failed.
  - A successful run keeps its own logs, log hashes and coverage, and the summary matches the per-run report.
  - Two runs get separate directories.
  - A nonzero exit stops later steps.
  - A missing executable is recorded as failed.
  - An early failure before `identify()` is recorded as failed.
  - A change to source files during the run invalidates the result.
  - Lock contention: the holder process calls `identify()`, a second run is refused, and the summary is left untouched.
  - SIGTERM: the whole process group is cleaned up, including a child and grandchild that ignore SIGTERM, which forces the SIGKILL fallback.
- **Result against this clone:** `Ran 10 tests … OK`, exit 0.
- **Check that it catches regressions:** I ran it against a deliberately broken scratch copy of the module, with the startup save and the SIGKILL fallback removed. It reported 1 failure and 1 error and exited 1. The tracked source was not touched.

## Observations (not blocking)
- The report's `git_status` shows ` M reports/verification-extended.json`. The cause is that the run rewrites its own summary file at startup, before it records git status. Reports aren't included in `source_sha256`, so the source identity is still valid, but the recorded `git_status` can never be clean.
- The verifier rewrites tracked report files, so the clone now shows modified `reports/abi-clients-rust.json`, `runtime-linkage.txt`, `unsafe.json` and `verification-extended.json`. These are tool-generated and I left them in place.

## Limitations
- This was run with `--no-oracle`: no oracle differential matrix, no upstream `make check`, no sustained fuzzing.
- I ran no ABI clients that need the oracle.
- This is not packaging, oracle-compatibility, a security audit or release certification.

## Logs
Everything is in `/tmp/rfile-readiness-final-qa/`:
- **Runner and summary:** `gates.sh`, `gates-summary.log`
- **Gates:** `01-fmt` … `06-verify`, each with `.cmd`, `.stdout`, `.stderr` and `.exit` files
- **Fixture checks:** `fixtures.sh`, `fx1-positive.log`, `fx2-missing-ref.log`, `fx3-optout.log`
- **Regression test:** `unittest-run.log`, plus `unittest-mutant-sanity.log`, `mutant.diff` and `mutant-tools/` from the broken-copy check

Build products are in the clone's `target/` and `target/msrv-1.87/`.
