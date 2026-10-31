# Claude pass-2 retest of Codex fixes — F-05 / F-06 / F-07

Date 2026-10-31. Tester: Claude (claude-sonnet-5-5). Not an audit: one tester, synthetic inputs, bounded runs.

## Identity
- Source: d7d0eca9d5da7f0a867e6ea95aab505840d398c8, tree 633e2278cb452d61232faea44f16d6479f822244 (isolated checkout `.reference/claude-pass2-retest-d7d0eca`; no production source or expected observation changed; only tracked `reports/*.json` churn from verify.py).
- Own binary: `target/release/rfile` (CARGO_BUILD_JOBS=1, `--release --locked`, rustc/cargo 1.99.1) sha256 `a1355b3a4c884b040c78aa00fd0a9aaf3a232bd699119621d78b867de411b60b` (`logs/binary.sha256`; version rfile-0.2.0-rc.1, features default, sandbox compiled=false). verify.py's own builds: default binary same a1355b3a…; all-features binary `dcdfb8f2998ea547…a4b`.
- Oracle for focused tests: pinned `.reference/file-clean` (41b11230…, src/file sha256 2b616d56…); untouched. Baseline for before/after: pass-2 8343 archive binary 6458750f…9323.

## Verdicts (commit d7d0eca)
| Finding | Verdict | Evidence |
|---|---|---|
| F-05 | **Retest PASS** (candidate for closure by maintainer; I do not close it) | 28/28 probes SAME (`logs/f05-probes.txt`), 284-case source+compiled matrix 0 oracle diffs, 0 src-vs-mgc mismatches (`logs/f05-matrix.txt`). Untested still: >2 GiB / int32 truncation domain, -k, --apple/--extension variants, APK zips beyond earlier 6. |
| F-06 | **Retest PASS** (candidate) | A_100000 0.07 s, D_100000 0.06 s, D_200000 0.12 s CPU (was 17.5/–/19.2 s); targets 1/0.5/1 s met. Doubling D 6250→50000: 0.00,0.01,0.02,0.04 s (~2×). RSS D_200000 124 MB = unchanged; 64 MB at 100k. Outputs byte-identical to 8343 binary on 14 families incl. A_100000 and D_200000 (`logs/f06-new-vs-old.txt`). NUL/RAW(-r)/nested-use/backspace/string%s: 14 rules × 5 option sets = 70, new==8343==oracle, NULs present in output (`logs/f06-nul-raw-nested.txt`). L_200 still hits the output limit, now in 0.03 s, same output hash. Not tested: use-heavy name-block multiplier variants, D_300000, an explicit CPU/RSS regression test in repo was not reviewed. |
| F-07 | **PARTIAL — NOT CLOSED, as expected** | No panic anywhere (84 rfile matrix rows; `--help/-v/--capabilities` to /dev/full, closed pipe, read-only fd; `--sandbox-status` with stderr /dev/full or closed → rc 0). Read-only stdout → rc 1 `Bad file descriptor` for all 11 commands incl. help/-v/capabilities. /dev/full → rc 1. EPIPE (closed reader, head -1, head -c10) → quiet rc 1, no stderr message, no signal; output bounded (≤25 B). **Unresolved**: inherited closed fd 1 (`>&-`) → rc 0, silent for all commands (oracle rc 1) — Rust runtime reopens /dev/null; documented by Codex. EPIPE is rc 1 where oracle dies by SIGPIPE (intentional policy). `-l` on embedded DB still `unsupported` rc 1 (pre-existing, outside F-07). |

New defects found in the three fixes: **none**.

## verify.py gates (sequential)
1. Run 1 default `--benchmark`, TMPDIR=evidence/tmp: **FAILED at elf-differential** (4709/5461 exact). All 752 differences are oracle `stdin` rows: `ERROR: cannot create temporary file for pipe copy (Permission denied)`; every `api`(3957) and `cli`(752) row equal. **Infrastructure, not rfile**: the oracle's Landlock sandbox (`src/landlock.c`) plus a pipe copy fails when `TMPDIR` points at the required scratch dir on this host (reproduced with both the shared and private oracle by hand; works with TMPDIR=/tmp or unset). Standalone rerun `elf_compare.py` with TMPDIR=/tmp: 5461/5461 (`logs/INFRA-elf-rerun-TMPDIR-tmp.log`). Preserved: `logs/run1-*`, `run1-verification-runs/`.
2. Attempt 0 (infra): verify.py needs `<root>/upstream`; absent → traceback (`logs/INFRA-verify-default-attempt1-no-upstream.log`).
3. Deviations (disclosed): (a) verify.py used a **private pinned oracle** built from a local clone of file-clean at 41b11230… (`oracle-private/`, `RFILE_REFERENCE`; magic.mgc sha identical to the shared one 8eab57da…) because `make check` would write into the shared oracle; (b) run 2 used **TMPDIR=/tmp (tmpfs)** instead of the .reference disk, to avoid the oracle issue above. Everything else in the run keeps CARGO_BUILD_JOBS=1.
4. Run 2 default `verify.py --benchmark`: **rc 0** (`logs/run2-verify-default.*`, 2026-10-31 11:16 PDT).
5. Run 2 `verify.py --all-features --require-sandbox --benchmark`: **rc 0** (`logs/run2-verify-allfeatures.*`, 11:29 PDT); report.json shows sandbox_cli and sandbox_kernel executed (8 child cases, debug+release), 103 required mutation fixtures executed, no skips. ASan/LSan/coverage gates were not disabled or altered (no edits to tools).

## Unexecuted / out of scope
Run 1 itself never completed past elf-differential (steps after it were covered only by run 2). Not run: pass-1 long fuzz/600 CPU-hour, musl/aarch64, >2 GiB inputs, installer (F-09), TZ matrix, release artifact acceptance, PRs. Other findings (F-01–F-04, F-08–F-10) untouched and not re-evaluated.

## Recommendation
Hand back to maintainer: F-05 and F-06 satisfy their acceptance on this commit; F-07 remains open on the inherited-closed-stdout criterion only. Suggest Codex/maintainer confirm that verify.py's oracle `stdin` rows work when TMPDIR is not the Landlock-restricted path (a harness note, not a code bug).

Files: `logs/`, `f05/`, `f07/matrix.json`, `run2-reports-final/`, `run1-verification-runs/`, `run2-verification-runs/`, `CHECKPOINT.md`, `run-verify*.sh`.
