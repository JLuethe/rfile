# Independent codec observation QA

**Reviewer:** GPT-6 Luna. **Candidate:** `dc103ea7fe3b7a71b6d6ea348a48995b0a176576`, isolated at `/tmp/rfile-night-review-codec`. **Oracle:** clean pinned source `41b1123083ab0bbdfe32f6d4ec14f6aec9a91e59` at `/home/john/Code/rfile/.reference/file-clean`.

## Verdict

**Accept the 64 supplemental ZIP64 helper observations.** The exception is exact, tied to the unchanged base decision file and upstream pin, and still checks all Rust and non-reference fields. The 64 records replay against the original Ubuntu 24.04 CI report. Scratch mutations to Rust output/status or the native helper diagnostic are rejected by the candidate harness's actual `row_key` and `signature` functions.

## Preserved CI replay

`/tmp/rfile-oracle-ci-all-0c0/compression.json` contains all 64 newly observed ZIP64 / ZIP64-descriptor rows. For every supplemental key:

- The signature of the CI row before its classification field is added matches the supplemental SHA-256.
- Every comparison field other than the native `reference` text matches the prior accepted row in `/tmp/rfile-oracle-ci-all-3fd/compression.json`; that includes Rust bytes/status, equality, fixture path, mode, flags, and errors. The only other differing field is the harness `disposition`, expected to change from previously accepted to unexpected before supplemental policy is applied.
- Replacing Rust bytes/status causes the key to match neither base nor supplemental decision. Replacing the gzip diagnostic with an unapproved variant does the same.

All 64 matched. The supplement's declared base decision hash is `0c66ab2e97ffc416bc6c29bc40aa96fff6633b2f3826d693c67a7939e89275f9`, which matches the actual decision file; the pinned upstream hash also matches.

## Local run

`cargo build --release --locked --all-features` succeeded serially with `CARGO_BUILD_JOBS=1`, offline dependencies, UTC and the clean oracle. The bounded `tools/compression_compare.py` matrix exited 0 with 3,268 observations, 2,444 exact, 824 existing explicitly accepted differences, and no unexpected cases. This host ran `/usr/bin/gzip` 1.15 (SHA-256 `b1f9e749ee0809d6e90100218e6bc13cdf3f26f23c243d5f62aae39162297cb1`). It matched the base helper decisions and accepted zero supplemental records; therefore the preserved CI replay above is the exercise of those 64 new observations.

The initial parallel all-feature build was killed by SIGKILL while compiling `crc32fast` during concurrent build load. A later serial retry succeeded. This is retained as a resource failure, not counted as a test result.

## Files

Raw report: `/tmp/rfile-night-review/luna-local-compression.json`. Replay script and result: `/tmp/rfile-night-review/luna_codec_replay.py` and `luna-codec-replay.json`. Build and matrix logs are in `/tmp/rfile-night-review-codec/luna-*.log`. The generated candidate `reports/compression.json` was copied to scratch and restored to the committed version; the codec checkout is clean after testing. No source, decision, supplemental observation, or oracle file was changed.
