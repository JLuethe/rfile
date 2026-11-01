# Claude pass-11 focused exact-archive acceptance

You are Claude, the independent tester for Rfile. Test the new docs-inclusive CI
archives and record results. Codex owns production changes. Do not publish, tag,
change draft assets, edit production or overwrite any earlier evidence.

Read first:

1. `/home/john/Code/rfile/reports/ci-ca9465f-handoff.md` for the complete artifact
   hashes, download commands, logs and source/runtime bindings.
2. `/home/john/Code/rfile-qa-claude/pass10/LEDGER-pass10.md` and CHECKPOINT.md.

Exact candidate:

```text
commit        ca9465f5f93eca75c80352a18a5de108e20762a0
tree          92b34ed5c86ea5737a4eb1f00e3ce085fe59dca5
packaging run 41326583263
required CI   41326558761
artifact ID   13091831221
```

Artifact name: `rfile-candidate-ca9465f5f93eca75c80352a18a5de108e20762a0`.
ZIP SHA256: `d7555442445044b8952ac3f3b9d38d963a815e0b6d84ed48bcc1020483945dd2`.

This candidate contains the I-1/I-2 INSTALL.md wording. Runtime, installer,
dependencies, tests and build tooling are unchanged from 8325cef. Codex's archived
executable hash is identical to pass 10; verify this independently. Source code
has not changed, so the focused scope below applies; a full golden sweep is not
requested unless you find an identity/code discrepancy warranting it.

Tasks:

1. Independently download the exact artifact ZIP. Verify the GitHub digest, run
   ID/attempt, workflow, commit and tree. Verify all six original payload-file
   hashes against the handoff and both tarball hashes against SHA256SUMS.
2. Confirm binary-archive INSTALL.md contains both intentional directory policies
   and exactly matches committed packaging/INSTALL.md. Expected SHA256:
   `3e156e3123cba293dc6a751399a2d78e9eb74ef2e4116db3c3bcacfe030a7f75`.
   INSTALL.md is archive documentation, not an installed-file entry in MANIFEST.
   Verify its content through archive integrity/source comparison; separately
   verify the existing installed-file manifest hashes/modes. Do not require a
   new manifest entry or treat its absence as a regression.
3. Compare the source archive against git archive of the exact candidate, with
   only the declared vendor/config/RELEASE-SOURCE additions. Verify metadata
   hashes and source identity; compare binary and installer bytes with pass 10.
4. Re-run the pass-10 behavior matrix on the new packaged executable and compare
   exact outputs/status with the accepted baseline, retaining documented oracle
   differences separately. Source changes or identity mismatches require renewed
   scope assessment before claiming equivalence.
5. Repeat install/uninstall scenarios, especially I-1 and I-2: pre-existing empty
   installation-path parent directories may be pruned; the prefix/non-empty dirs
   and unrelated files remain; handled failure rollback removes created files
   but leaves created empty directories. Verify these agree with INSTALL.md.
6. Rebuild offline from the source archive with a fresh empty Cargo cache and
   record toolchain/commands/hash/results. Byte reproducibility across differing
   build paths/linkers is not promised.

Use bounded subprocesses and owned-process cleanup. Preserve initial failures
and harness revisions. Write a new `pass11/LEDGER-pass11.md`, CHECKPOINT.md and
SHA256-INVENTORY under `/home/john/Code/rfile-qa-claude/`; if that pass directory
already exists, preserve it and select the next unused pass number.

Report exact identities, scope, failures/skips, and disposition of I-1/I-2 only
for the tested final archive bytes. Leave other maintainer decisions open.
This task does not launch the independent security review, 600 CPU-hour fuzz
campaign, coverage/resource qualification or publication. End with a concise
handoff to Codex for any actionable defects.
