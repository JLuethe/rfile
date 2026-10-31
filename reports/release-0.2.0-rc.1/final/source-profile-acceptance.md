# Independent source-profile acceptance

Reviewer: GPT-6 Luna. Candidate: `a781c017b3495121a4203053cbabe55225cb0130`, tree `91506a94437571b6fc3257f1d2e860824e19b727`, source SHA-256 `64ed7072cc801c52b00aedda06c616ad3ae7141337df3c455aa8077f2bb0304d`. Final main `e64ce034bee6a6c0959872d9b8f632d692dbfaa3` is tree-identical. Primary oracle: clean pinned `41b1123083ab0bbdfe32f6d4ec14f6aec9a91e59`; upstream digest `ecf3f799246ee2f02d18e5ca8d935a636aca98a01f538adffffe3b3eacf7cb75`. Offline, one build job, `LC_ALL=C`, `TZ=UTC`.

## Verdict

PASS for this source-profile acceptance. Default verification passed 32 stages and all-feature required-sandbox verification passed 34 stages. Python discovery passed 15 tests; Rust 1.87 workspace/all-target/all-feature check passed; source dump comparison returned 582/582 valid exact rows and separately identified two malformed inputs (584 total). This is source-profile acceptance, not sustained fuzz/security qualification.

## Profile results

- Default `python3 tools/verify.py --benchmark`: run `20261031T064125Z-a3042b1ba8174e46bbf595049ca064f1`, 32 stages, exit 0; benchmark output checksum matched.
- All features `python3 tools/verify.py --all-features --require-sandbox --benchmark`: run `20261031T065741Z-18f18ef5938f4b7da66d4f0e6c9850b0`, 34 stages, exit 0; benchmark checksum matched. Strict CLI and kernel sandbox coverage executed in debug and release; kernel child cases 8/profile. Fuzz smoke: fixture files 103, truncations 1339; fixture matrix 103/103.
- Time comparison: 2,850 rows; 2,820 exact, 30 narrowly classified native undefined/Rust-validated, zero unexpected. Semantics: 10,875 rows; 10,848 exact, 27 undefined validated, zero unexpected, zero compiler errors. Compression: 3,268 rows; 2,444 exact, 824 policy-classified intentional differences, zero unexpected.
- Python: `python3 -m unittest discover -s tools -p 'test_*.py' -v`, 15 passed. Rust MSRV: `cargo +1.87.0 check --workspace --all-targets --all-features --locked`, passed. Source check: built `source_check` with all features and compared 584 cases: 582 valid exact, 2 malformed separately.

## Preserved infrastructure failures

The first default sandbox attempt failed at Rust FFI clients because LeakSanitizer refuses to run under ptrace; the same focused stage passed 3/3 outside ptrace (20 exports), and the full default retry passed. The first all-feature run failed from `/tmp` quota exhaustion (run report remained `running`); the next run hit cached test binaries embedding the old `/tmp` manifest path after relocation. A symlink to the same checkout corrected the path and the full all-feature retry passed. These failures and logs are retained and are not counted as passes.

## Raw evidence

Raw VerificationRun directories, including both successful profiles and all three failed attempts, are in `verification-runs/`. Supporting command logs, focused retest JSON, checkpoint, and runner scripts are in `rfile-final-acceptance-a781c01-qa/`. Source dump output is `source-dump.json`. File hashes are indexed in `evidence-file-hashes.json`; run report hashes and statuses are in `run-index.json`. All 80 stage logs with a recorded expected SHA-256 match; the quota-truncated partial run had one cargo-build log without a recorded expected hash. Build caches are excluded.
