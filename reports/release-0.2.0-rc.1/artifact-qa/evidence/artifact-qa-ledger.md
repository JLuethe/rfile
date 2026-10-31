# Rfile candidate artifact QA ledger

Prepared by GPT-6 Luna as the independent artifact tester, per the parent assignment. This ledger records preparation results only. **The original `0f` candidate is not release-accepted** because default CLI initialization failed in the minimal Ubuntu image without a system timezone file. The follow-up `09f2c36` binary passes the timezone regression, but it is a local build, not a frozen exact release archive; final asset acceptance remains pending.

## Original exact CI candidate

- Repository/run: `JLuethe/rfile`, run `41176219039`; CI status `completed/success`.
- Source: `0f1054d2404d8049d2d2357e22af97c5e8f22d3c`.
- Download command: `gh run download --repo JLuethe/rfile --name rfile-candidate-0f1054d2404d8049d2d2357e22af97c5e8f22d3c --dir .reference/package-qa-0f/candidate`.
- Download log: [`download.log`](download.log). Run metadata was obtained with `gh run view 41176219039 --repo JLuethe/rfile --json status,conclusion,headSha,workflowName,url`.
- Archive hashes, all passing `SHA256SUMS`:
  - Source archive `rfile-0.2.0-rc.1-source.tar.gz`: `45461255d5c6aa3c04083bfce939d43953bc04cadc6b72840fe8d46c0abf4c41`.
  - Linux CLI archive `rfile-0.2.0-rc.1-x86_64-unknown-linux-gnu.tar.gz`: `67a61daa94bbcacc3e0bd8dc15cdc97cd1b1a5e21b34c3de4cfa6491442e1c72`.
  - `release-manifest.json` claims source commit `0f1054d2404d8049d2d2357e22af97c5e8f22d3c` and glibc minimum `2.34`; imported GLIBC symbols confirm max `GLIBC_2.34`. Dynamic dependencies include `libc.so.6` and `libgcc_s.so.1`; no `libmagic` linkage.
- Source identity: source `RELEASE-SOURCE.json` commit and version match. Its committed-tree tar hash `0ba60fbbaafb029175dfd7a1b352e9283c018f941952be464f174f66ad6c8814` matches `git archive` of that commit from the workspace repository.
- Source/notices inspection: 5,013 files, 1,106 directories, 64 vendored crates, 120 files under `LICENSES/`; source includes `LICENSE`, `NOTICE`, `LICENSES/README.md`, `data/UPSTREAM-COPYING`, `data/UPSTREAM.json`, and `data/SOURCES.json`. `UPSTREAM.json` records upstream `file` commit `41b1123083ab0bbdfe32f6d4ec14f6aec9a91e59` / version 5.48. The binary archive carries all listed licenses/notices.
- Independently hashed and mode-checked all 129 declared binary payload entries: pass. `INSTALL.md` and `install.py` are expected root support files outside the installed-file manifest. Initial scratch assertion mistakenly treated those support files as unexpected; retained failure evidence is [`independent-manifest-check.json`](independent-manifest-check.json); corrected scope passes in [`independent-manifest-check-corrected.json`](independent-manifest-check-corrected.json).
- Installer adversarial checks: **10/10 pass** in [`independent-installer-results.json`](independent-installer-results.json): absolute prefix with spaces, relative prefix rejection, existing destination preservation, internal symlink refusal, payload byte tampering, manifest checksum tampering, changed installed payload refusal, no partial uninstall, unrelated-file preservation, and explicit prefix symlink handling.
- Executable reports version `0.2.0-rc.1`, embedded pinned magic v21, upstream commit above, and `safe-rust regex miniz_oxide`; default embedded classification works on the host. `--version`, capability output, benign text file/stdin/MIME/gzip and missing-file behavior are covered by the package checker and container logs below.

## Checker and offline build evidence

- Original candidate `tools/check_cli_package.py` from the source archive was executed from an extracted source tree, with `TMPDIR` under `.reference/package-qa-0f/tmp`. First run failed at its final offline source check (exit 101): fresh `CARGO_HOME` hid rustup proxies, so it could not execute `rustc -vV`. Preserved at [`provided-checker-command.log`](provided-checker-command.log) and [`provided-checker.json`](provided-checker.json).
- Retried that original checker with explicit stable-toolchain `CARGO` and `PATH`; **14 checks pass**, including install, uninstall, and empty-cache offline workspace check. Evidence: [`provided-checker-retry-command.log`](provided-checker-retry-command.log), [`provided-checker-retry.json`](provided-checker-retry.json).
- Parent’s local checker fix commit `9f4d20f4e7dd19249421b71bcc847f615fd5607c` was tested separately against the same exact `0f` archive with caller PATH lacking `~/.cargo/bin` and `CARGO` unset. The toolchain path regression was fixed. First retry with unrestricted Cargo parallelism hit `SIGKILL` compiling `hybrid-array` (resource/infrastructure failure; original output retained in [`fixed-checker-original-path-command.log`](fixed-checker-original-path-command.log)). With `CARGO_BUILD_JOBS=1`, all **14 checks pass**: [`fixed-checker-jobs1-command.log`](fixed-checker-jobs1-command.log), [`fixed-checker-jobs1.json`](fixed-checker-jobs1.json).
- Independent release-mode CLI link from the exact source archive with empty Cargo home and offline vendor directory passed:

  ```sh
  CARGO_HOME="$PWD/.reference/package-qa-0f/empty-cargo-home-release" \
  CARGO_TARGET_DIR="$PWD/.reference/package-qa-0f/release-source-target" \
  CARGO_BUILD_JOBS=1 \
  PATH="/home/john/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH" \
  cargo build -p rfile --bin rfile --release --locked --no-default-features \
    --target x86_64-unknown-linux-gnu --offline
  ```

  Exit 0; log: [`offline-release-build.log`](offline-release-build.log). The package checker’s fresh-cache check is `cargo check --workspace --all-targets --all-features --locked --offline`, distinct from the release CLI link above.

## Original candidate Ubuntu runtime: release-blocking finding

- Image: `ubuntu:22.04`; inspected config ID `a7b0f6af325461862a99c677cf3229ca5241d9f4f4289bd00cc2cce91a3aff23`; repo digest used: `docker.io/library/ubuntu@sha256:076e7bf82319619f28dd0aa9c02ab34b055ad37100395b4f49a2ffba04fb5e11`. Runtime tests used rootless Podman, `--network=none --read-only --cap-drop=ALL --security-opt=no-new-privileges --user=65534:65534 --memory=256m --pids-limit=32 --cpus=1`, with only candidate binary and synthetic fixture bind-mounted read-only.
- Image reports glibc 2.35 and lacks `/etc/localtime` and `/usr/share/zoneinfo/Etc/UTC`. Host inputs were readable inside the container (`cat /fixtures/plain.txt` succeeded).
- On original candidate `0f`, with TZ absent, both `--capabilities` and `-b /fixtures/plain.txt` fail with `No such file or directory (os error 2)`, exit 1. `TZ=UTC` also fails because this minimal image has no named zoneinfo data. `TZ=UTC0` succeeds (capabilities says `time_zone: UTC0`; plain text classifies `ASCII text`). Explicit missing `TZ=:/nonexistent` fails. This isolates an implicit missing-localtime default failure; it is **why the original exact archive is not accepted**. Evidence: [`ubuntu22-timezone-matrix.log`](ubuntu22-timezone-matrix.log), [`ubuntu22-runtime.log`](ubuntu22-runtime.log), [`ubuntu22-runtime-utc0.log`](ubuntu22-runtime-utc0.log).

## Follow-up timezone fix candidate (local build only)

- Source commit: `09f2c368a32bb10855bf80183e49dcbe813e4e9a` (`Use UTC when an unconfigured system timezone is absent`). Built from isolated `git archive` tree with vendored dependencies, empty Cargo home, one job, offline, release profile.
- Build command/log: [`build-command.txt`](../timezone-retest-09f2c36/build-command.txt), [`build.log`](../timezone-retest-09f2c36/build.log), exit 0.
- Binary: `/home/john/Code/rfile/.reference/package-qa-0f/timezone-retest-09f2c36/target/x86_64-unknown-linux-gnu/release/rfile`; SHA256 `69660224b249c6f8220e929f681372c3876d35954d641892d9570fb3d514fd4f`.
- A reusable constrained-container regression helper was authored in scratch and hardened per parent feedback. Script: [`timezone-container-regression.py`](../timezone-container-regression.py); SHA256 `37ef1811ef1830e4f5e839de51e6358da8a1fd757e950afb081d46981c21e5ce`. It takes binary path, `podman`/`docker`, a digest-pinned image, and report path; uses bounded subprocesses, unique named containers, explicit cleanup, per-command logs/statuses, and writes the binary SHA. It requires a valid host UTC TZif fixture and fails on cleanup errors/timeouts.
- Hardened script: **13/13 assertions pass** in the pinned Ubuntu image. Missing implicit `/etc/localtime` yields UTC for capabilities, path classification, stdin, and gzip; explicit `TZ=UTC`, `UTC0`, and empty TZ are accepted with expected zone identity; explicit missing and invalid timezone names fail; malformed and unreadable existing `/etc/localtime` fail; valid existing UTC TZif passes. Every cleanup status is 0. Report: [`timezone-regression-09f2c36-hardened.json`](timezone-regression-09f2c36-hardened.json); command log: [`timezone-regression-09f2c36-hardened-command.log`](timezone-regression-09f2c36-hardened-command.log). Exact checker invocation: `python3 .reference/package-qa-0f/timezone-container-regression.py --engine podman --image docker.io/library/ubuntu@sha256:076e7bf82319619f28dd0aa9c02ab34b055ad37100395b4f49a2ffba04fb5e11 --binary .reference/package-qa-0f/timezone-retest-09f2c36/target/x86_64-unknown-linux-gnu/release/rfile --report .reference/package-qa-0f/evidence/timezone-regression-09f2c36-hardened.json --scratch-root .reference/package-qa-0f/tmp` (exit 0).

## Acceptance status

- Original exact `0f` CI archives: **NOT ACCEPTED** for release due default runtime timezone failure on a minimal Ubuntu 22.04/glibc 2.35 image. This is one tested environment; no claim is made about every glibc 2.34 system or other platforms.
- Checker toolchain-path fix: independently verified; its unrestricted parallel build first hit a SIGKILL, then one-job retry passed.
- Timezone fix `09f2c36`: local binary passes the hardened 13-case container suite. It has not yet been re-packaged into frozen source/binary archives, so final exact-asset acceptance is pending parent follow-up.
- No production files, system configuration, global tools, releases, or installations were modified by this tester. QA scratch and evidence are under `/home/john/Code/rfile/.reference/package-qa-0f/`.
