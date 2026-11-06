# Docs-inclusive CI candidate: ca9465f

Status: **required CI and packaging passed; artifacts are downloadable**.
Claude focused acceptance is pending.

## Exact build target

```text
repository JLuethe/rfile
branch     ci/rc1-final-ca9465f
commit     ca9465f5f93eca75c80352a18a5de108e20762a0
tree       92b34ed5c86ea5737a4eb1f00e3ce085fe59dca5
```

The remote branch and both run records bind this commit/tree. The branch stays
pinned; this handoff and its later documentation commit are not build targets.

| Workflow | Run / attempt | Result | URL |
|---|---|---|---|
| Rust checks, push | 41326558761 / 1 | linux and msrv passed | [Run](https://github.com/JLuethe/rfile/actions/runs/41326558761) |
| Release candidate, workflow_dispatch | 41326583263 / 1 | package-linux passed | [Run](https://github.com/JLuethe/rfile/actions/runs/41326583263) |

Required branch checks are linux and msrv. Packaging built the default-feature
Linux/glibc CLI and vendored source on Ubuntu 22.04. CI package/offline-rebuild
checks passed 14/14; minimal-runtime checks passed 13/13. Independent acceptance
is not inferred from these self-checks.

Final workflow metadata and check-run records confirm linux, msrv and package-linux
all succeeded on the candidate. Both run records contain the exact head_sha and
head_commit.tree_id above; the remote branch was checked again after completion.
No workflow retries were needed. Rust checks uploaded no artifacts; packaging
uploaded the single artifact below.

## Artifact identity and download

- [Download artifact 13091831221](https://github.com/JLuethe/rfile/actions/runs/41326583263/artifacts/13091831221).
- Name: `rfile-candidate-ca9465f5f93eca75c80352a18a5de108e20762a0`.
- Expires: **2026-11-15T16:30:30Z** (UTC).
- Downloaded ZIP hash matches GitHub's digest:

```text
d7555442445044b8952ac3f3b9d38d963a815e0b6d84ed48bcc1020483945dd2  13091831221.zip
```

Every original file inside the ZIP:

```text
7aac3b790ecb4b0bd770afade2a51d5782836dee0501a7bafbac60370ab8c705  SHA256SUMS
aa00cb3a64af4002ca14d3937bca9c6be71699d9332495c7196741955c00b860  minimal-runtime-qa.json
0b2c58f9d4e56f1aaf6473677974615350df8e71e4b86275d18e7ee2b1905b30  package-qa.json
04b131b400a2fd8e7a0c6b42ec54a8ba675935f4b1a9c1e53714bdf243888ef0  release-manifest.json
59d5a890d1142ccb7929f81458888f41b8bf535bfbff8068ff24840586b761c7  rfile-0.2.0-rc.1-source.tar.gz
b0f47032f80abd8b29c92b8229c25b15f46d90a6812f8cd18fc54c6d5c74ba65  rfile-0.2.0-rc.1-x86_64-unknown-linux-gnu.tar.gz
```

Independent download commands (choose a new directory):

```bash
gh api repos/JLuethe/rfile/actions/artifacts/13091831221/zip > 13091831221.zip
sha256sum 13091831221.zip
gh run download 41326583263 --repo JLuethe/rfile \
  --name rfile-candidate-ca9465f5f93eca75c80352a18a5de108e20762a0 \
  --dir /path/to/new/acceptance-directory
```

## Documentation and runtime binding

No code under src/crates/data/tools/tests/fuzz, dependency manifests, workflows
or packaging/install.py differs from the verified 8325cef baseline. Only
documentation changed. The empty runtime-and-build-code-diff.txt preserves the
git comparison result.

- New packaged executable SHA256: `3bc6b059a37fd0f057c3e573dcd661f082c67cc8668eef2f761b4ef34aa0dca2`.
- It is byte-identical to the packaged executable independently accepted in
  pass 10. No source-level changes occurred; focused acceptance applies.
- Packaged install.py SHA256: `09cfe0c39c033162d967538981d9f808071e75e66086c0eb9baeb1ed5dfc12da`; unchanged.
- Packaged INSTALL.md SHA256: `3e156e3123cba293dc6a751399a2d78e9eb74ef2e4116db3c3bcacfe030a7f75`.
  It exactly equals committed packaging/INSTALL.md and the same file in the
  vendored source archive, including both I-1/I-2 intentional-behavior paragraphs.
- Source RELEASE-SOURCE.json git_archive_sha256: `95f839b01a29d5b6242304e2a7c7997a3156baf18f8d313e2392dbb51d8f690a`.
  It matches a local git archive of this exact candidate. Both archive manifests
  bind the candidate source commit, and tarball hashes match SHA256SUMS.

INSTALL.md and install.py are archive-root files, not installed-file entries in
MANIFEST.json. Their integrity comes from the archive checksum and comparison
with committed files. Check the 129 installed-file entries against MANIFEST.json
separately; do not require a new INSTALL.md manifest entry.

## Preserved evidence

Root: `/home/john/Code/rfile/.reference/ci-final-ca9465f/`.

- `artifact-zips/13091831221.zip`: exact downloaded artifact ZIP.
- `artifacts/rfile-candidate-ca9465f5f93eca75c80352a18a5de108e20762a0/`: original payload plus a local checksum list.
- `artifact-verification.json` and `SHA256SUMS.artifacts`: ZIP/payload hashes,
  manifest bindings, documentation equality and binary identity.
- `expected-INSTALL.sha256`, `expected-installer.sha256` and
  `git-source-tar.sha256`: hashes from the exact Git candidate.
- `run-*-initial.json`, `jobs-*-initial.json` and final metadata: source and run identities.
- `logs/run-41326583263-attempt1.zip`: full packaging logs and extracted copy.
- `logs/run-41326558761-attempt1.zip`: full required-check logs and extracted copy.
- `check-runs-final.json`, `run-*-final.json`, `jobs-*-final.json` and
  `remote-candidate-branch-final.json`: final required-check/source/run bindings.
- `SHA256-INVENTORY`: hashes of retained evidence, including copies of this
  handoff and the Claude prompt. Initial snapshots are preserved separately.

Full log ZIP SHA256 values:

```text
84607955ceb17e40015b319c3b27f8681ffe923083df6d1d99348b32efeec909  run-41326558761-attempt1.zip
42404105e17999fc1650ccd276f02e54d1d77dd44c9c78c2b6bfc9b08dab7978  run-41326583263-attempt1.zip
```

## Claude next task

Use [the focused pass-11 prompt](claude-pass11-test-prompt.md). Download the new
archives independently; verify provenance, hashes and INSTALL.md; compare source
with git archive; repeat the behavior matrix and installer scenarios (including
I-1/I-2); rebuild offline with a fresh cache. Preserve evidence in a new QA pass.
Pass-10 acceptance remains attached to its old artifact bytes; do not silently
apply it to these new archives even though the executable is identical.

Maintainer dispositions and independent security qualification remain open.
No publication, tags, draft-asset replacement or security campaign was performed.
