# Pass-11 focused exact-archive acceptance recorded

Claude's original ledger and checkpoint are preserved at
`/home/john/Code/rfile-qa-claude/pass11/`. Codex read both, inspected the installer
assertion log and rehashed the independently downloaded ZIP. No production,
installer or packaged documentation was changed in this recording step.

## Accepted identity

```text
commit        ca9465f5f93eca75c80352a18a5de108e20762a0
tree          92b34ed5c86ea5737a4eb1f00e3ce085fe59dca5
packaging run 41326583263 (attempt 1)
required CI   41326558761 (attempt 1; linux/msrv passed)
artifact ID   13091831221
ZIP SHA256    d7555442445044b8952ac3f3b9d38d963a815e0b6d84ed48bcc1020483945dd2
```

The original [CI handoff](ci-ca9465f-handoff.md) preserves all artifact hashes,
run URLs, provenance records and logs. Claude independently verified the exact
source/tree in both run records and all six original artifact-file hashes.

## Acceptance and dispositions

- Provenance, archive hygiene and all 129 installed-file manifest hashes/modes:
  PASS. INSTALL.md matches the committed packaging/INSTALL.md and source-archive
  copy, including both intentional directory-cleanup policies.
- Source archive versus git archive: PASS, with only the declared vendoring,
  Cargo configuration and source-manifest additions.
- Packaged executable and installer: byte-identical to pass 10. Focused matrix,
  F-04/CDF/output/FIFO/encoding/TZ regressions: PASS within the recorded scope.
  A new full golden sweep was not warranted; the identical binary already passed
  that scope in pass 10.
- Installer: 19 explicit assertions passed, zero FAIL markers in the inspected
  log. **I-1 and I-2 are resolved as documented-intentional for these archive
  bytes.** Uninstall may prune pre-existing empty installation-path parents while
  preserving the prefix, unrelated paths and non-empty directories; handled
  failure rollback leaves created directories.
- Fresh-cache offline source rebuild: PASS. Its binary is behavior-equivalent
  in the 324-row matrix; cross-path/linker byte reproducibility is not promised.

No defects require a Codex fix. This records independent focused exact-archive
acceptance on the reported host and pinned minimal Ubuntu container. It does not
establish broader platform coverage, a completed security review or sustained
fuzz qualification. Original evidence remains unchanged.

Evidence reference hashes checked during this review:

```text
9feeebf64c1aa046397b92eaf3f3e4d9113c1b781043e6af6e6463f8c642da59  LEDGER-pass11.md
c029d24b7dfe66b3277527cbf16faac6c825010d32c4b5e3cda41de6b8516427  CHECKPOINT.md
59dabf7c227e1bff1aa21779f6de45d419246e11d7fe77e69e52ac8e0d951b2e  SHA256-INVENTORY
e261a0189dc1349ddf6e20792c3a40068d39b125689c184b69d80f84318d32fc  logs/installer.txt
```

## Next work, with the candidate kept fixed

1. Obtain decisions on [remaining maintainer dispositions](remaining-maintainer-dispositions.md).
   Recommendations in that table are not approvals.
2. Have Claude produce the [independent security-review scope brief](claude-security-review-scope-prompt.md).
   This is planning, not execution of the review or a campaign.
3. After review scope is agreed, perform the independent review and route defects
   to Codex for fixes and independent retesting. Coverage/resource evidence,
   sustained campaigns and final-candidate replay remain separate later gates.

These record/brief files are outside the frozen archive snapshot. They do not
retarget ca9465f or require rebuilding its already accepted bytes. A subsequent
code/dependency/packaging change must receive its own candidate identity and
affected verification. No CI build, publication, tags, draft-asset change or fuzz
campaign was started in this step.
