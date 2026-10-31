# Independent FILETIME oracle harness review

**Reviewer:** GPT-6 Luna, independent tester. **Candidate:** `0c045636947e21ce3a1b1997529dc8ff7ff75a9d` in `/home/john/Code/rfile/.reference/release-acceptance-source`. **Oracle:** clean pinned source `41b1123083ab0bbdfe32f6d4ec14f6aec9a91e59` at `/home/john/Code/rfile/.reference/file-clean`.

## Verdict

**Accept the FILETIME harness correction. No harness blocker found.** The documented exception is limited to the native failure branch that leaves `tv_sec` uninitialized. Both comparison scripts retain native output bytes but do not count them as exact truth. They require the Rust result to be precisely the successful `*Invalid datetime*` marker, and other mismatches remain failures.

## Source and boundary review

In pinned `src/cdf_time.c`, `cdf_timestamp_to_timespec` sets `tv_nsec`, divides signed ticks into days, and computes the approximate year as `1601 + floor(days / 365)`. When this is greater than 9999 it jumps to `out` before assigning `tv_sec`. `src/print.c:file_fmtdatetime` ignores that error return and then reads `ts.tv_sec`. The undefined range for unsigned FILETIME bits is therefore `((10000 - 1601) * 365 * 86400 * 10,000,000) <= value < 2^63`, i.e. `0x24c21a7980b54000` through `INT64_MAX`. Values with the high bit set are negative signed ticks and do not take this positive-year branch.

The helper uses exactly that inclusive lower and exclusive upper bound. The semantics comparator applies it only to Windows date rules and only after the native 0/1-byte `file_buffer` shortcut. For inputs of length 2 through 7, pinned `mget` zero-pads its 8-byte scratch value; those cases can reach the same date conversion, so `len > 1` is appropriate. The time comparator preserves `reference` output and records `raw_equal`, while exact accounting excludes the undefined records. Both scripts validate the Rust status, stdout, and stderr against the specific expected invalid marker.

## Executed checks

- Release `rfile` and `rfile-probe` builds: exit 0, `CARGO_NET_OFFLINE=true`, `LC_ALL=C`, `TZ=UTC`, `RFILE_REFERENCE` set to the clean oracle.
- Standard `tools/time_compare.py`: 2,850 rows, 2,820 exact, 30 undefined and safely validated, 0 unexpected.
- Standard `tools/semantics_compare.py`: 10,875 rows, 10,848 exact, 27 undefined and safely validated, 0 unexpected, 0 compiler errors.
- Independent direct vector replay: 37 rows spanning native/little/big endian rules, threshold minus one/at/plus one, signed maximum, high-bit and all-ones negative patterns, ordinary dates, and short inputs. Ten values landed in the undefined range; every Rust invalid marker was exact and no defined comparison mismatched. Raw C and Rust results are retained in `luna-filetime-vectors.json`.
- Existing `tools.test_verification_run`: all 10 tests passed.
- Scratch regression `luna_test_oracle_boundary.py`: all five tests passed. A scratch mutation changing inclusive `>= MIN` to strict `> MIN` failed the boundary test as intended.

## Nonblocking documentation observations

The candidate's `docs/releases/0.2.0-rc.1.md` currently promises a final independent verdict from “Claude”; this review is by GPT-6 Luna. `docs/release-scope.md` also still describes PR #19 as merge-pending, though it has since been merged. These are release-document freshness issues, not harness acceptance failures; update them before freeze.

## Evidence and workspace state

Raw logs, reports, test code, and mutant evidence are under `/tmp/rfile-night-review/luna-*`. The generated `reports/time.json` and `reports/semantics.json` in the candidate checkout were copied to scratch then restored to the committed versions. The candidate worktree and clean oracle worktree are clean after cleanup; the oracle revision remains unchanged. The fresh release binaries are ignored build outputs. No source, expected result, oracle, or release artifact was edited.
