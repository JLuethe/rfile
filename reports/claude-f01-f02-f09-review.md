# Codex response: F-01, F-02 and F-09

These changes follow the pass-2 ledger at
`/home/john/Code/rfile-qa-claude/pass2/LEDGER-pass2.md`. Independent retest is
required; this implementation record does not close findings.

- **F-01:** ELF64 positions with the sign bit set cannot be valid signed file
  positions. Render the pinned signed diagnostic for program-note reads instead
  of passing the position to InputView and turning enrichment into a whole-file
  error. Diagnostic conversion is separate from IO; no invalid-position read is
  attempted. Core MIME output remains its existing identification. Executable
  program-section reads get the corresponding native diagnostic. Other IO and
  resource failures still propagate normally.
- **F-09:** expected filesystem/path/manifest refusal errors are caught at the
  installer entrypoint and printed as a concise status-1 diagnostic. Existing
  exclusive creation, preflight checks, rollback and uninstall protections are
  unchanged. Unexpected programming exceptions are not generally swallowed.
- **F-02 decision:** preserve the complete-decode hard error, explicitly document
  the default 7 MiB cap, missing partial/wrapper output on failure, the `-P bytes=N`
  workaround and independent budgets. A partial decoded prefix cannot establish
  the unread tail's integrity; adding sampling would require an explicitly
  incomplete result contract. This is a deliberate bounded compatibility
  difference. No decoder limits are loosened and no checksum verification is
  removed. Documentation appears in the manual, compatibility policy and release
  notes. Gzip and zlib tests cover cap minus one, exact, plus one, and raised cap
  in both semantic profiles.

Codex validation: ELF and codec integration suites (22 tests), three installer
refusal regressions, all-feature/all-target workspace Clippy and formatting.
Claude should independently check the exact fix commit, original F-01 four
inputs and nearby signed boundaries/modes, all ELF comparisons, gzip/zlib limits
and workarounds/bombs, and new exact-package installer refusal paths. Existing
GitHub draft remains the older e64 artifact; no replacement/publication is
performed by this work.
