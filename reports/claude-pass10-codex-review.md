# Pass-10 artifact acceptance and installer dispositions

Reviewed the original ledger/checkpoint at
`/home/john/Code/rfile-qa-claude/pass10/`, the installer scenario log, the
installer implementation and its packaging path. Earlier ledgers, raw evidence
and downloaded artifacts were not modified.

## Accepted artifact identity

Claude independently downloaded and accepted CI artifact **13069527384** from
packaging run **41272131089**, with required Rust checks run **41272111258**:

```text
commit 3f9af8d89dcf2990a842cf6e4e6e9c1b28a0c228
tree   f08995301414e2976b70f6e289fc58f00a05b62d
ZIP    c4b8cf35e16a6bdf5523d7f7c4ddccf3d644ab0f3a7bc645f5838ff4fb5290f7
```

Codex rehashed Claude's downloaded ZIP; it matches GitHub's digest and the
previous CI handoff. Claude's acceptance covers provenance, archive hygiene,
manifest hashes/modes, exact packaged-binary behavior, installation/uninstallation
and fresh-cache offline source rebuilding. The packaged binary matches the
pass-9 default binary's behavior on all 14,088 golden comparison rows. Acceptance
is scoped to the supplied archives, tested host and pinned minimal Ubuntu
container; it is not security qualification or cross-platform certification.

Evidence references:

- Ledger SHA256:
  `13203d00e56b0797d814f7c8f59d1c2f515dc9e2e9ef85967f4094a333f28e03`.
- Installer log SHA256:
  `3110b2086a475fb59a1b4bca7c9db8872aaec01708f9f2fd57b969e798cc6fac`.
- Original inventory SHA256:
  `ba13aa5343c3a37a3a05f0347e747f2fd00bc16d81b5de45042396cfe94519d0`.

## I-1 and I-2 disposition: document existing behavior

The user authorized fixing or documenting these optional observations. Both are
documented as intentional in packaging/INSTALL.md, which becomes INSTALL.md in
the CLI archive. No installer or Rust code was changed.

- **I-1:** uninstall may remove empty installation-path parent directories below
  the prefix even if they predate installation. The receipt records file
  ownership, not directory ownership. Cleanup uses rmdir, preserves non-empty
  directories and does not remove the prefix itself. It does not promise exact
  restoration of the original empty-directory snapshot.
- **I-2:** handled installation-failure rollback unlinks files created by that
  attempt but does not remove directories it created. Empty bin/share or a newly
  created prefix can remain. INSTALL.md states this and advises checking which
  empty directories were newly created before manual cleanup.

This preserves the tested receipt/cleanup contract rather than adding directory
ownership bookkeeping and receipt migration for these nonblocking observations.
The wording was checked against install.py and pass-10 installer scenarios T6
and T11. No new runtime tests are needed for this documentation-only disposition.
Other maintainer dispositions are not implicitly accepted by this decision.

## Packaging consequence and next gates

Pass-10 acceptance remains valid for the exact 3f9af8d archives. Those archives
do not contain this new INSTALL.md wording; they were not altered. Since
package_cli.py copies packaging/INSTALL.md into the binary archive and includes
the committed tree in the source archive, shipping the new wording changes both
archives and their source identities. Do not attribute pass-10 acceptance to
future bytes or silently edit the old downloads.

Next work:

1. Record the outstanding maintainer dispositions for the documented F-03,
   F-07, F-04, F-10/string16 and mixed-flag/directory/FIFO/empty-file contracts.
   Finish intended documentation before another artifact freeze.
2. Build the final docs-inclusive candidate through CI and have Claude perform
   focused acceptance of its exact archives: provenance and hashes, presence of
   the I-1/I-2 wording, installer scenarios and offline source rebuild, with
   behavior checks appropriate to any binary/toolchain differences. A docs-only
   change does not by itself require repeating all source-level suites.
3. For the stronger security-qualified endpoint, obtain an independent security
   review before the long campaign, then complete coverage/resource evidence,
   the 600 CPU-hour campaign and final-candidate replay on the frozen source.
   These were not performed by functional/archive acceptance. Experimental
   publication and any stronger security claim still require the maintainer's
   scoped decision.

No workflow was dispatched in this documentation pass. No publication, tags,
draft updates or release-asset replacements were made.
