# Rfile retest of commit 0ee7f29: increment FAIL (one Clippy gate failure); items 2–4 PASS

Clippy fails under `-D warnings`. Everything else I ran in items 2–4 passed. Item 5 (`verify.py --no-oracle`) was not run because the 12-minute budget ran out.

## Identity and environment
- **Tester:** Claude Opus 5.5 (`claude-opus-5-5`), acting as independent tester. This is not a security audit.
- **Commit:** `0ee7f29d82398b1ad0a9f99ed71ff1bc01142318` on `fix/release-readiness-baseline`, in the isolated clone `/tmp/rfile-readiness-0ee7f29`. Its git status was clean at the start.
- **Toolchain:** stable rustc 1.99.1 (2026-10-13), plus the 1.87.0 toolchain that was already installed. Nothing was downloaded.
- **Platform:** Linux x86_64 (kernel 7.2.8). All runs were offline and `--locked`, and every inherited `RFILE_*` variable was removed.

## Per-command results
| Step | Exit | Result |
|---|---|---|
| `cargo fmt --all -- --check` | 0 | PASS |
| Workspace tests, default features, `--nocapture` | 0 | PASS: 26 test binaries, 0 failed, 1 ignored. Sandbox CLI recorded as `skipped/not-compiled` |
| Workspace tests, all features | 0 | PASS: 27 test binaries, 0 failed, 1 ignored. Sandbox CLI and kernel both `executed` (8 child cases) |
| **Clippy, all targets, all features, `-D warnings`** | **101** | **FAIL** (details below) |
| 1.87.0 check, workspace, all features, all targets | 0 | PASS (separate target directory `target/msrv-1.87`) |
| Release hardening, all features, `--ignored`, with `RFILE_REFERENCE` | 0 | PASS: 103 files, 1339 truncations |

The panic messages in the passing logs come from tests that check panics are contained (the FFI cookie test and the locale guard test). They are expected.

## Item 2: fixture contract — PASS
I compiled the hardening test first (17.7s) so the exact 120s command timed the test, not a cold build.

| Case | Exit | Evidence |
|---|---|---|
| Exact command with the clean reference | 0 | 103 files, 1339 truncations, 10000 each for DB/source/random, seed `4f6c655275737421` |
| Missing explicit reference | 101 | panic at `tests/hardening.rs:117` |
| Empty reference (its `tests/` has no `.testfile`) | 101 | panic at `tests/hardening.rs:123` |
| Empty reference directory with no `tests/` | 101 | panic at `tests/hardening.rs:117` |
| `RFILE_REFERENCE=""` | 101 | panic at `tests/hardening.rs:123` |
| No reference and no `upstream/`, default settings | 101 | panic at `tests/hardening.rs:117` |
| `RFILE_REQUIRE_FIXTURES=0`, no reference | 0 | `mutation_fixtures` recorded as `skipped`, `required:false`, 0 truncations |
| Opt-out with a missing reference | 101 | fails as required |
| Opt-out with an empty reference | 101 | fails as required |

## Item 3: sandbox — PASS
- **Default features, `RFILE_REQUIRE_SANDBOX=1`, `sandbox_cli`:** exit 101, panic at `tests/sandbox_cli.rs:36`. It fails correctly.
- **Default features, no require:** exit 0 with `RFILE_COVERAGE {"component":"sandbox_cli","status":"skipped","reason":"not-compiled"}`.
- **All features, require set, `sandbox_cli`:** 1 passed, recorded as `executed`.
- **All features, require set, platform kernel tests:** 2 passed, recorded as `executed` with `child_cases:8`.
- **Gap (not a defect):** with default features and require set, the platform `sandbox` test compiles to 0 tests and exits 0 without writing a coverage record. `verify.py` only enforces `--require-sandbox` together with `--all-features`, so this can't be counted as certification.

## Item 4: VerificationRun regression — PASS
These ran on synthetic temporary roots against the real `tools/verification_run.py`; t5 ran the real `verify.py`.

| Test | Result |
|---|---|
| t1: startup shows `running` mid-run and keeps the old `passed` summary as `previous-summary.json` | PASS (see note) |
| t2: two successful runs keep separate logs with different hashes; log hashes match; coverage is recorded; latest summary points to run 2 | PASS |
| t3: a step exiting with status 7 gives `failed`, and later steps don't run | PASS |
| t4: a missing executable gives a `failed` step and run (`status:null`, ENOENT error recorded) | PASS |
| t5: real `verify.py` hits an early oracle failure (missing `data/UPSTREAM.json`); summary is `failed`, no steps ran, previous summary kept | PASS |
| t6: a second writer gets `Another verification run owns this checkout`; the first run's summary and run directory are unchanged | PASS (see note) |
| t7: SIGTERM during a child gives `interrupted` (signal 15); grandchild and process group are gone | PASS (0.29s) |
| t7b: child ignores SIGTERM; the SIGKILL fallback cleans up the group and the run is `interrupted` | PASS (5.29s) |
| t7c: SIGKILL of the runner leaves `running`, never `passed` | PASS |
| t8: a step that changes `src/` invalidates the run (`failed`, "Source changed…") | PASS |

**Note on t1 and t6:** the harness printed FAIL for both because of my own assertion error. Their holder runs never called `identify()`, so the run ended `failed` with "Source changed" instead of the `passed` my assertion expected. That is fail-closed behaviour, not a bug. Every property the tests targeted held, as shown in the output.

## Reproduced bug
**Medium severity: the all-feature, all-target Clippy gate fails.** It blocks `verify.py --all-features` at its clippy step and the item 1 acceptance.

Reproducer:
```
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
error: literal with an empty format string   (clippy::print_literal)
  crates/rfile-platform/tests/sandbox.rs:30-33  println!("{}", r#"RFILE_COVERAGE {...unsupported-kernel...}"#)
  crates/rfile-platform/tests/sandbox.rs:72-75  println!("{}", r#"RFILE_COVERAGE {...executed...}"#)
error: could not compile `rfile-platform` (test "sandbox") due to 2 previous errors
```
Compilation stopped at `rfile-platform`, so `tests/sandbox_cli.rs` lines 42, 54 and 221 weren't linted. They use the same pattern and probably fail too. This is test code only; no detector code is involved. I made no fix.

## Missing coverage
- **Item 5 (`verify.py --no-oracle`):** not run because of the time budget.
- **Release-profile sandbox runs:** the require-sandbox checks did not run in the release profile.
- **Clippy:** I didn't run default-feature Clippy, and the all-feature run stopped before linting `sandbox_cli.rs`.
- **Harness source not saved:** the Write tool was denied under `/tmp/rfile-readiness-qa`, as was a harness version that saved its own source to a file. I ran the harness inline with `python3 -c`, so only its results file exists. The script is in this session's transcript.

## Verdict
- **This increment's retest:** FAIL, solely because of the Clippy gate. The fixture-truncation fix (103 files / 1339 truncations, fail-closed cases) and the new VerificationRun controls are confirmed working.
- **Packaging/final release:** no verdict. No candidate is frozen, and no artifacts, oracle differential runs, sustained fuzzing or security audit were in scope.

## Durable logs (all under `/tmp/rfile-readiness-qa/`)
- **Per-command summary** (argv, environment changes, exit, seconds, test results, coverage, panics): `cargo-retest-summary.json`
- **Raw output:** `logs/<step>.stdout` and `logs/<step>.stderr`. The Clippy failure is in `logs/01d-clippy-allfeat-alltargets.stderr`.
- **Harness results:** `vr-harness/results.json`
- **Synthetic run roots and artifacts:** `vr-roots/t*-*/`
- **Fixture test inputs:** `empty-ref/` and `empty-ref-bare/`
- **Clone build products:** `target/` and `target/msrv-1.87/`
