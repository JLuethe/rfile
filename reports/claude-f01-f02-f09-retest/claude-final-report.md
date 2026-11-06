# Claude retest — exact commit 2bed7f803c5ca1fbcab2ae7b6feea2ec20adfdb2 (tree dd43679f5e7dda58846301c209bb2d17c6b23f7f)

Execution commit: 2bed7f8 (clean checkout `.reference/claude-2bed7f8-retest`, 0 dirty files). Product, shared checkout, oracle and earlier evidence not modified. Nothing pushed/published.

## Identities (identity.txt)
- CLI archive sha256 cd8d9845…e67509; source archive decea907…710cf; archive binary 570df533…ed3 (bin/rfile).
- Source-built (non-remapped) rfile 4e8b49d1…9970 / rfile-probe 689cbb1c…0295 (cargo 1.99.1, 1 job, disk scratch). Differ from archive binary only in embedded paths; ELF 5461 passed with both.
- packaging/install.py sha256 09cfe0c3…c12da — byte-identical to ec687fc.

## Results
| Gate | Result |
|---|---|
| package_cli --offline build, one job | OK |
| check_cli_package exact-package | passed, 14 checks, source_commit 2bed7f8 |
| F01 matrix (copies of ec gen_f01/f01_cli, archive binary) | 5880 rows, **0 differences**, 0 nonzero/ERROR (ec had 1800) |
| elf_compare.py (RFILE_REFERENCE=file-clean, TMPDIR=/tmp) | 5461/5461 exact, source-built and archive binary |
| Rust `--test elf --test codecs` (9+14 tests) | dev, release, and extended-codecs: all pass |
| Python test_installer (4) | OK |
| Installer cases, ORIGINAL expectations (unmodified copy) | 1 FAIL: `uninstall receipt-files-list` rc=1 (expected rc0) — harness error, see below |
| Installer cases, corrected copy | 0 FAIL; no traceback, no residue, unrelated files preserved |
| Installer supplemental (26 bad manifests x fresh/preexisting prefix, 12 bad receipts) | 0 FAIL; precious data preserved; final uninstall rc0 |
| F02 gzip/zlib boundaries/-z/-Z/-P/checksums/truncation/bombs | 0 FAIL (corrected predicates) |

Corrected expectation justification: receipt `files=[]` is an invalid receipt; the installer prints `rfile installer: Manifest files must be a nonempty object`, rc1, state byte-identical before/after (state_unchanged=True, installer_results.json). Old expectation (rc0 + removal) was a harness bug from the pre-validation era. Original-expectation log retained: logs/installer_cases_original_expectations.log.

F02 harness note: first run (logs/f02_cases_v1_harness_predicate_error.log, 24 FAIL) assumed errors on stderr/empty stdout. rfile prints `ERROR: ...` on stdout with rc1 (same as file's convention); the product output was identical in both runs. v2 predicate: rc1, stdout starts `ERROR:`, no ASCII/wrapper description. Product unchanged.

F02 facts verified on exact binary (gzip and zlib, path and stdin): decoded 7340031/7340032 OK with -z (inner + wrapper) and -Z (inner only); 7340033 -> `ERROR: decompression output limit exceeded`, rc1, no partial/wrapper text; no -z identifies wrapper for all sizes; `-P bytes=7340033|16777216` succeeds; `-P bytes=n-1` fails, `=n` passes (cap exact); with raised cap bad CRC, bad ISIZE, bad Adler, truncated streams still error; 1 GiB bombs error under default and 16 MiB caps. Docs (man page, 03-compatibility, release notes) match: status 1, default 7340032, no partial output, -P bytes workaround, other budgets still apply.
Another Claude's full F02 report at ec was not available to me; this is a focused independent run only, not a duplication of verify.py.

## Verdicts
- **F-01 (signed ELF positions/read spans): PASS** at 2bed7f8 for tested scope (original 4 core inputs, signed boundaries, exec/dyn PT_NOTE/DYNAMIC/INTERP, LE/BE, 7 modes, path+stdin; ELF 5461). The two ec residuals (missing comma; read span crossing i64::MAX) are fixed.
- **F-09 (installer refusals/manifest+receipt validation): PASS** (packaged installer; script unchanged from ec).
- **F-02 (gzip/zlib 7 MiB cap): PASS as a documented deliberate hard-error difference**, not sampling equivalence with libmagic.

## Not tested / limits
No full verify.py, no sanitizer/fuzz, no clippy/fmt rerun, no other Rust suites beyond elf+codecs, no Ubuntu CI/final artifact, no whole security audit claim. Tests ran only on this host's x86_64 Linux. F02 data is repeated text; other payload shapes not covered.
