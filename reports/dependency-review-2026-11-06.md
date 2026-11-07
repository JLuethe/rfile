# Post-release dependency review: lzma-rust2 0.25 and Jiff 0.2.39

Reviewed on 2026-11-06 (America/Los_Angeles), against `main` at
`a5ca8554f35a92715d7fa577dd4c245e1dfd6a2a`. This is development-branch
evidence. The published `v0.2.0-rc.1` tag and archives are unchanged.

## Decision and package repair

Integrate the updates proposed by #27 and #28 together with the required
package metadata. The Dependabot heads changed only `Cargo.toml` and the
workspace `Cargo.lock`; their `package-linux` jobs failed when `cargo vendor
--locked --sync fuzz/Cargo.toml` encountered the stale fuzz lockfile. This
review updates both lockfiles, `reports/dependencies.json` and the retained
license directories. The new upstream license texts are byte-identical to the
copied files. `cargo vendor --locked --sync fuzz/Cargo.toml vendor` succeeds.

The resolved changes are exactly lzma-rust2 0.24.0 to 0.25.0, Jiff 0.2.38 to
0.2.39, jiff-core 0.1.1 to 0.1.2 and jiff-static 0.2.38 to 0.2.39. The
feature selection remains lzma-rust2 `std,xz,lzip` without defaults, and Jiff
`std,tz-fat` without defaults; no new native decoder or system timezone lookup
is enabled. Jiff's published minimum Rust version remains below rfile's 1.87
minimum; lzma-rust2 declares 1.85.

## Source and boundary review

- lzma-rust2 0.25 adds stream memory-usage/limit accessors and permits retry
  after a memory-limit error. It also tightens XZ block/index padding and
  LZMA2 truncation checks, and fixes fragmented/interrupted reader handling.
  rfile constructs each stream with a finite `new_mem_limit`, never raises that
  limit, charges every drive step and consumed/output byte to its shared
  inspection budget, and returns no successful decoded result after a decoder
  error. Its XZ reader fallback retains the same finite memory limit and
  budgeted source adapter. The changed reader behavior may alter malformed
  error prose, which is outside the declared codec compatibility profile.
- Jiff 0.2.39 fixes several large-value panics/incorrect span results. The
  jiff-core timestamp change makes out-of-range `from_nanosecond` return an
  error; rfile uses checked `from_second`. The Jiff timezone-source changes
  visible in the inspected diff are documentation examples and unrelated
  formatting/arithmetic paths. rfile still loads bounded, caller-selected TZif
  bytes or validated POSIX rules and holds an immutable `TimeContext`.

## Executed verification

On Linux x86_64, rustc/cargo 1.99.1:

- `cargo test --workspace --all-features --locked`: passed.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`: passed.
- `cargo build --release --workspace --all-features --locked`: passed.
- `cargo check --manifest-path fuzz/Cargo.toml --locked --bins`: passed.
- `cargo vendor --locked --sync fuzz/Cargo.toml vendor`: passed; generated
  vendor directory removed after the check.
- Fresh `cargo audit --file Cargo.lock` and `cargo audit --file fuzz/Cargo.lock`:
  both passed with zero reported advisories or warnings (1,422 loaded advisories).
- Pinned codec classification: 3,268 comparisons; 2,444 exact, 824 existing
  named decisions, zero unexpected. The independent lzip fixtures also passed
  the all-feature test suite. The generated current-run report was inspected;
  the previously committed historical report was then restored.
- Pinned date comparison: 2,850 cases; 2,820 exact defined results and 30
  oracle-undefined FILETIME results with the required safe marker, zero
  unexpected. The generated current-run report was inspected; the historical
  report was restored.

The two full differentials compare current release builds to the clean pinned
file 5.48 oracle at `41b1123083ab0bbdfe32f6d4ec14f6aec9a91e59`.
Neither passing matrix proves arbitrary malformed streams, all timezones, or
an independent security audit. Exact-package checks and hosted CI are recorded
in the pull request for this update.
