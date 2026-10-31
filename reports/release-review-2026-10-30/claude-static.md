# Independent QA Report: rfile, initial release-readiness review (static only)

**Bottom line:** I ran no commands, so there is no fresh build or test evidence. Bash was denied for every call (`don't ask` mode), including a read-only `git rev-parse HEAD`. I did not try to get around the denial. Everything below comes from reading the code. All gates are **NOT RUN**, and you can't decide on a release from this report.

## Identity and environment
- **Agent:** Claude (Claude Agent SDK), model `claude-opus-5-5` (Opus 5.5).
- **Source:** `/tmp/rfile-review-7a3ea75/source`. The SHA you stated is `7a3ea756e79838e4c3ca252d23c3e9ff7ab8fe76`, but I couldn't confirm it because git couldn't run.
- **Host:** Linux 7.2.8-9-omarchy, as reported by the harness. I couldn't check the toolchain version, whether `cargo` is on PATH, or whether the offline registry is complete.
- **Date:** 2026-10-30.

## Commands and exit statuses
| Command | Status |
|---|---|
| `git rev-parse HEAD` | **Blocked** (Bash permission denied) |
| `cargo fmt --all -- --check` | **NOT RUN** (blocked) |
| `cargo test --workspace --locked --offline` | **NOT RUN** (blocked) |
| `cargo test --workspace --all-features --locked --offline` | **NOT RUN** (blocked) |
| `cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings` | **NOT RUN** (blocked) |
| `cargo build --release --locked --offline` | **NOT RUN** (blocked) |
| CLI smoke checks against the binary | **NOT RUN** (no binary) |

- **Test counts:** none observed.
- **Reproduced failures:** none, because nothing ran.
- **Not fresh evidence:** HANDOFF.md says "145 ordinary tests / 33 stages". The delivery ledger says "105" and "113 tests / 24 stages". These are older records and I did not verify any of them.

## Concerns from reading the code (not observed failures)
1. **Sandbox tests pass even when they skip.**
   - `crates/rfile-platform/tests/sandbox.rs:25-28` prints `SKIP` and returns early when the kernel doesn't support the policy. `tests/sandbox_cli.rs:40` does the same.
   - On `ubuntu-latest` CI, or any kernel without Landlock ABI ≥5, these tests count as passes without testing anything.
   - CI does not check whether a skip happened.
2. **The mutation hardening test never runs in CI.**
   - `tests/hardening.rs:96` is marked `#[ignore]`, and CI never passes `--ignored`.
   - Its fixture phase (`hardening.rs:147-149`) reads `upstream/tests`. That directory isn't in this archive, and the `if let Ok(...)` silently leaves `fixture_cases` at 0.
3. **The C ABI builds an absolute path into the binary.** `crates/rfile-ffi/src/state.rs:12-15` sets the default magic path to `concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/magic.mgc")` unless `RFILE_MAGIC_PATH` is set at build time. A packaged `librfile_magic` would point at the build machine's source tree, so this matters for packaging.
4. **CI misses several things** (`.github/workflows/check.yml`):
   - The MSRV job (1.87) only runs `cargo check`. Tests are never run on MSRV.
   - CI never runs with `--offline`, so it doesn't prove the build works from vendored or offline sources.
   - `tools/verify.py`, the differential and oracle comparisons, the C/C++ ABI clients (`crates/rfile-ffi/tests/client.{c,cpp}`) and fuzz builds are not in CI. Only `abi_clients.py --rust-only` runs.
   - CI is Linux x86_64 only, and the sandbox tests are further limited to `gnu` + `x86_64` by their `cfg`.
   - The `fuzz/` crate has its own `Cargo.lock` and isn't part of the workspace, so CI never builds it.
5. **Release statements are stale or contradict each other.**
   - `README.md:52` says "The GitHub release is experimental", which implies a release exists. HANDOFF.md:45 says "No … new release".
   - HANDOFF.md (dated 2026-10-25) names `feat/bounded-fuzz-queue` as the active branch and `51a31af` as the latest reviewed main. Neither matches the snapshot under review.
   - HANDOFF.md:44 says the ABI package (`reports/abi-package.json`, code 62d7b13) is historical and has to be rebuilt.
   - HANDOFF.md:39 says no fuzz target has met its 24-CPU-hour gate.
6. **Two tests need care when run.**
   - Default and all-feature CLI tests share `CARGO_BIN_EXE_rfile` (HANDOFF.md:63-65). They have to run one after the other or with separate target directories, and CI does run them one after the other.
   - `sandbox.rs:278` runs `/bin/true`, which assumes that file exists on the host.

## Release blockers
- **For any release:** there are no fresh passing results for fmt, test (default and all-features), clippy or the release build on this SHA. A rerun with Bash allowed is required.
- **For a fully qualified replacement:**
  - The project's own fuzz gate (24 CPU-hours per target, at least 600 CPU-hours in total) is unmet.
  - No independent security review has been done.
  - ABI and diagnostics gaps remain open (`data/abi-known-gaps.json`).
  - The FFI default path is a build-time absolute path.
  - The project says itself that it can't claim to be a drop-in replacement.

## Recommended gates
**Experimental Linux-only release**
1. On the exact tagged SHA, run fmt, default and all-features clippy with `-D warnings`, default and all-features tests (one after the other), and the release build. Run all of them `--locked`, and also once `--offline` to confirm the vendored build works.
2. Run tests on MSRV 1.87, not just `cargo check`.
3. Treat a sandbox `SKIP` as a failure on the release runner, or require a runner with Landlock ABI ≥5 and record the enforcement JSON from `--sandbox-status`.
4. Run CLI smoke checks against the release binary:
   - `--help` and `--capabilities`
   - stdin with `-b -`
   - `-i` on known fixtures
   - a missing file and a directory
   - `-m custom.magic`
   - `-z` on a gzip file
   - exit codes for each of these
5. Rebuild the database and confirm its pinned hash (`tools/rebuild_database.py`).
6. Correct README.md:52 and HANDOFF.md to match the actual release state, and state the Linux/x86_64-only and codec/locale/regex limitations.

**Fully qualified replacement**
- All of the experimental gates, plus:
  - `tools/verify.py --all-features --benchmark` against the clean oracle `41b1123`.
  - Full upstream `tests/*.testfile` parity.
  - C/C++ ABI clients, and a rebuilt package that runs from a relocatable path.
  - The `--ignored` mutation campaign with fixtures present.
  - The full fuzz CPU-hour gate on the frozen candidate, Miri/ASan, and `cargo-audit` at the current RustSec revision.
  - An independent security review.
  - Coverage beyond x86_64 glibc, or explicit exclusions for other platforms.

## Limitations
- This was a static review of Cargo.toml, check.yml, HANDOFF.md, README.md, part of the delivery ledger, and grep-selected parts of the tests.
- I did not read the full source or every test file.
- No command ran, so nothing here confirms a failure. Everything above is a concern for a rerun with Bash allowed to confirm or rule out.
