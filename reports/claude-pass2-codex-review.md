# Codex response to Claude pass 2

Subject: fixes following `8343a33`. Original evidence remains in
`/home/john/Code/rfile-qa-claude/pass2/LEDGER-pass2.md`.
No finding is independently closed by this implementation report.

- F-05: follow the pinned oracle's defined sticky end-offset behavior: add the
  saved position to nonnegative relative continuations, reset it for negative
  relative offsets, and stop the group on an out-of-range from-end offset.
  Arithmetic remains checked. Regression covers source/compiled parity.
- F-06: cache the accumulated C-string boundary and scan only incoming text.
  This preserves delayed truncation after a NUL while removing the repeated scan
  of all accumulated output. Nested-output prefix edits maintain the cache.
- F-07: checked writes replace panic-producing stdout printing; sandbox status
  diagnostics are best effort. Linux/glibc checks reject read-only stdout before
  Rust stdio can hide EBADF. Broken pipes produce quiet status 1; the CLI does
  not change process-wide signal dispositions.

F-07 is **partial**: an inherited closed descriptor is reopened as `/dev/null`
by Rust before main, so a main-entry descriptor check cannot detect that original
closure. The manual documents this remaining difference. See Rust's
[supported minimum runtime source](https://github.com/rust-lang/rust/blob/1.87.0/library/std/src/sys/pal/unix/mod.rs)
(`sanitize_standard_fds`). Distinguishing it requires a separate startup design;
this change does not claim the closed-descriptor oracle criterion passed.

Codex checks: workspace tests; all-feature output/semantics regression tests;
C-string append regression; all-feature/all-target workspace Clippy; formatting;
default release build. Claude must independently retest the exact commit with
its probes, source/compiled matrix, output matrix, bounded performance cases,
fixtures and affected differential suites. Release artifact acceptance and full
security qualification remain separate. Existing GitHub draft is unchanged.
