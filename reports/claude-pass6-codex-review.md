# Pass-6 Codex fixes and Claude retest handoff

Date: 2026-10-31. Original evidence:
`/home/john/Code/rfile-qa-claude/pass6/LEDGER-pass6.md`.
Claude's ledgers, binaries and raw evidence were not modified.

## Exact candidate

`git rev-parse 8325cefd5d3c8c7d22015d8a97c948196330c435 8325cefd5d3c8c7d22015d8a97c948196330c435^{tree}`:

```text
8325cefd5d3c8c7d22015d8a97c948196330c435
b6e431e70d286914acc5d52417982e99b36151de
```

Local branch: `fix/claude-timezone-diagnostic`. This report and the HANDOFF update
are a subsequent documentation-only commit. Build/retest the candidate above;
no archive is supplied or claimed accepted.

## Changes and dispositions

- **F-04b: narrow code fix, awaiting independent retest.** Plain UTF-8 selection
  now checks for a non-ASCII character in the decoded prefix, rather than a high
  byte in the raw input. An incomplete tail alone no longer selects UTF-8. The
  decoder, incomplete-tail tolerance after a completed character, BOM branch,
  parser `looks_utf8` behavior and one-byte classification path were not changed.
  Regression cases cover the two-byte ARJ/PDF prefixes, 11-byte matilde `-k`,
  partial two/three/four-byte sequences, completed characters plus incomplete
  tails, ASCII, invalid UTF-8, BOM-only/BOM-plus-ASCII and one-byte controls.
- **F-08d: documented deliberate difference.** Compatibility notes, man page and
  release notes now explicitly include readable, empty and chained regular
  symlink targets without `-L`: mixed `-i --apple` emits the target result only
  (a permission description if unreadable). The oracle prefixes it with
  `inode/symlink; charset=binary`. The compatibility/release notes acknowledge
  that 8e33173 emitted only the symlink MIME result. No filesystem or mixed-flag
  output behavior was changed in this candidate. Maintainer acceptance remains
  separate from documenting the behavior.
- **F-08c annotation-only unreadable symlinks:** Claude pass 6 verified the fixed
  subset at e8cdcb4 across default/all-feature profiles. This candidate retains
  it. That verification does not close the separate mixed-flag, directory-wording
  or FIFO contracts.

## Codex local validation

Passed on the candidate source before committing:

- `cargo test --locked --test semantics`: 29 tests.
- `cargo test --all-features --locked --test semantics`: 29 tests.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`.
- `cargo fmt --all -- --check` and `git diff --check`.

Compared all 24 files in Claude's pass-6 `f04b/` directory using default-feature
debug rfile and the pinned oracle, with `-b`, `-k -b`, and `-b --mime-encoding`:
**72/72 exact matches**, comparing status, stdout and stderr. This includes all
reported fixture prefixes, complete-character/incomplete-tail controls, BOM
controls and the one-byte very-short cases. Environment: `LC_ALL=C TZ=UTC0`;
oracle pin `41b1123083ab0bbdfe32f6d4ec14f6aec9a91e59` with explicit
`.reference/file-clean/magic/magic.mgc`. The oracle was not modified.

Local JSON: `.reference/pass6-codex-validation/f04b-default.json`, SHA256
`21824f3179790d880361370006129bc072c336b5be9d6cf526ff2d87615be141`.
These are Codex checks, not Claude's independent acceptance, release builds,
packaged artifacts or a full compatibility sweep.

## Next Claude retest

Rebuild both profiles from the exact candidate in an isolated clone and record
commit, tree, commands and binary hashes. Rerun the pass-6 commands, particularly
the F-04b probe set and ARJ@2, PDF@2 and matilde@11 with `-k`. Expect plain UTF-8
fallbacks to newly agree and the completed-character/incomplete-tail, BOM and
one-byte controls to stay identical. Check the F-08d docs against readable,
empty, chained and unreadable symlink controls; the deliberate concatenation
difference remains. Retain the filesystem matrix and adjacent bounded regression
suites from pass 6. Write a new ledger rather than rewriting earlier evidence;
close only what is independently verified.

Other maintainer items remain open: mixed-flag output, directory wording, FIFO
and `-E`-on-empty contracts, F-03 explicit TZ, F-07 inherited-closed stdout, F-04
shifted-use bounds, F-10 invalid geometry stages and non-wildcard string16
residuals. Neither full verifier profile, CI/archive or exact-asset acceptance,
the 600 CPU-hour fuzz campaign, coverage/resource evidence nor independent
security review was performed here. No tags, draft, release assets or pinned
oracle changes; nothing was published.
