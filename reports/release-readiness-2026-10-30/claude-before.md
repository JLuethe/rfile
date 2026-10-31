I reproduced the gap. With `RFILE_REFERENCE` pointing at a reference checkout that has real fixtures, the mutation test still passes and reports **`fixture_truncations: 0`**.

## Identity
- **Snapshot:** `/tmp/rfile-review-7a3ea75/source`, an archive of `7a3ea756e79838e4c3ca252d23c3e9ff7ab8fe76` with no `.git`. I didn't run git and didn't modify anything.
- **Reference:** `/home/john/Code/rfile/.reference/file-clean`
- **Scope:** one bounded test run plus static reading of code. This is not a security audit.

## A. Executed reproduction: the mutation test skips the fixtures

**Command (run exactly as authorized):**
```
timeout 120s env RFILE_REFERENCE=/home/john/Code/rfile/.reference/file-clean /home/john/.cargo/bin/cargo test --release --locked --offline --test hardening deterministic_mutation_campaign -- --ignored --nocapture
```

**Result:** exit 0, test passed (`1 passed; 0 failed; 9 filtered out`, 2.85 s).
```
{"database_cases":10000,"elapsed_seconds":2.853618474,"fixture_truncations":0,"random_input_cases":10000,"seed":"4f6c655275737421","source_cases":10000}
```

**Evidence:**
- **The fixtures exist.** A file search found 103 `*.testfile` entries in `.reference/file-clean/tests/` (for example `CVE-2014-1943.testfile`, `json1.testfile`, `zstd-v0.8-22.testfile`).
- **The snapshot has no `upstream/` directory,** so `upstream/tests` is missing.
- **Code:** `tests/hardening.rs:147` hardcodes the path `CARGO_MANIFEST_DIR/upstream/tests`. It never reads `RFILE_REFERENCE`. `tests/hardening.rs:149` uses `if let Ok(entries) = read_dir(...)`, so a missing directory silently means zero fixture cases, with no failure or warning.
- **The tools handle this correctly elsewhere:** `tools/reference.py:7` and `tools/build_reference.sh:4` both read `RFILE_REFERENCE`. The test is the only place that ignores it.
- **Expected count:** each fixture is truncated at 13 lengths, so 103 fixtures should give about 1,339 cases (13 × 103). A shorter length can exceed a small file's size and get clamped to it; it still counts as a case.

## B. Static findings (code reading only, not run)

1. **The sandbox certification can pass while skipping.** In `tests/sandbox_cli.rs:38-44`, `supported()` prints `SKIP strict CLI certification` and returns `false` whenever stderr contains "required confinement needs". The caller at `:73` (`if supported(&root) {...}`) then skips the strict-mode assertions, and the test still passes. Also, `:28-37` returns `false` whenever the build lacks the `sandbox` feature or isn't linux-gnu-x86_64. As a result, `--all-features` verification can report success without having certified strict confinement. I didn't run the sandbox tests.
2. **`tools/verify.py` can leave a stale "passed" report after a failure.**
   - `run()` (`:18`) and every gate (`:33`, `:39`, `:42`, `:47`, `:51`, `:62`, `:65`, `:69`) stop with `SystemExit` before `:74` writes the report. A failed run therefore leaves the earlier `reports/verification*.json` in place, and that file has `status: 'passed'` hardcoded at `:73`. Nothing deletes the old report or marks it failed. The per-step `.log` files and other `reports/*.json` files are partly overwritten, so the reports end up out of sync with each other.
   - `:71-73` copies `mutation_smoke` from `fuzz-smoke.log` without checking `fixture_truncations > 0`. Finding A would therefore show up in a "passed" report as zero fixture cases.
   - The `fuzz-smoke` step (`:28`) runs with `ENV`, which inherits `RFILE_REFERENCE`, but the test ignores it (see A).
   - Separately, `:32` uses `git -C UP rev-parse`, which needs a git reference. I didn't check that here.

## Acceptance criteria for the retest
1. Re-running the exact command above prints `fixture_truncations` > 0, about 1,339 for 103 fixtures. Ideally the JSON also includes the fixture directory used and the fixture count.
2. The test reads its fixture directory from `RFILE_REFERENCE/tests` first, then falls back to `upstream/tests`.
3. If no fixture directory can be found, or it holds zero `.testfile` entries, the test fails with a clear message. It may pass only if an explicit opt-out is set, and the opt-out must be recorded in the output. A missing directory must never mean a silent pass.
4. `verify.py` refuses to write a passing report when `mutation_smoke[*].fixture_truncations == 0`, unless that opt-out is set.
5. `verify.py` deletes the old report, or writes `status: failed` with the failing step, before stopping on any failure. No `status: passed` report may survive a failed run.
6. A strict-sandbox skip in `sandbox_cli.rs` either fails when certification is required (for example in `--all-features` verification) or is recorded as "skipped" in the verification report rather than counted as passed.
7. The other hardening tests and the 30,000 mutation cases still pass, with the seed unchanged.

No prerequisites were blocked. One combined shell command (an `ls`/`grep` pipeline) was denied by permissions, so I listed and searched the files with the file-search tools instead; the authorized cargo command ran normally.
