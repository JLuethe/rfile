# CI candidate handoff: 3f9af8d

Status: **required CI and packaging passed; artifacts are downloadable**.
Independent archive acceptance is pending.

## Candidate and runs

```text
repository JLuethe/rfile
branch     ci/rc1-pass9-3f9af8d
commit     3f9af8d89dcf2990a842cf6e4e6e9c1b28a0c228
tree       f08995301414e2976b70f6e289fc58f00a05b62d
```

The remote branch and both workflow records bind the exact commit/tree above.
The candidate branch remains pinned. This handoff is a later documentation file;
its eventual commit is not the build target.

| Workflow | Run / attempt | Jobs | URL |
|---|---|---|---|
| Rust checks (push) | 41272111258 / 1 | linux and msrv passed | [Run](https://github.com/JLuethe/rfile/actions/runs/41272111258) |
| Release candidate (workflow_dispatch) | 41272131089 / 1 | package-linux passed | [Run](https://github.com/JLuethe/rfile/actions/runs/41272131089) |

Main branch protection requires linux and msrv. Packaging used Ubuntu 22.04
and built the default-feature x86_64-unknown-linux-gnu CLI and vendored source.
CI package checks passed 14/14, including offline source rebuild; the exact
binary passed 13/13 minimal-runtime checks. Those are CI self-checks, not the
maintainer/Claude independent acceptance.

Both final run records contain head_sha and head_commit.tree_id matching the
candidate. GitHub check-run records independently show linux, msrv and
package-linux completed successfully on this SHA. The remote candidate branch
was checked again after completion and still points to the same commit/tree.
Rust checks completed at 2026-11-01T04:16:28Z; packaging completed at
2026-11-01T04:13:12Z. No retries were needed. Rust checks uploaded no artifacts;
packaging uploaded the single artifact listed below.

## Download and SHA256 identities

- [Download the CI artifact ZIP](https://github.com/JLuethe/rfile/actions/runs/41272131089/artifacts/13069527384).
- Artifact ID: `13069527384`; name: `rfile-candidate-3f9af8d89dcf2990a842cf6e4e6e9c1b28a0c228`.
- Expiry: **2026-11-15T04:13:06Z** (UTC). Preserve a copy before expiry.
- Downloaded ZIP SHA256 equals GitHub's artifact digest:

```text
c4b8cf35e16a6bdf5523d7f7c4ddccf3d644ab0f3a7bc645f5838ff4fb5290f7  13069527384.zip
```

Every original file inside that ZIP:

```text
2f78682794b2ea87cf415b1015ba47a08f2f6fb05bd0d478df24d3ce081ab88f  SHA256SUMS
ecabced147c68c1ad2dce3f6745d93f9604d6f4864712c5647da62e503fcd6f0  minimal-runtime-qa.json
9205072dc3524b79a7bec790b9e2b2a379df525a030f57a7e219507a47a2f27d  package-qa.json
9173d90032644aac4d1d62cfac32a7c226cc94c39a9097b7fd8c52188d0a6849  release-manifest.json
d2142bda6cf0673254d67cdfa58ff85fa3abdac1ab4b000b1d314ef9f861d274  rfile-0.2.0-rc.1-source.tar.gz
425c19a7e3a1e9c89ce26d8c1478a68b2ab1990f73f3ee0236bdd32efbb012bb  rfile-0.2.0-rc.1-x86_64-unknown-linux-gnu.tar.gz
```

Packaged `bin/rfile` SHA256: `3bc6b059a37fd0f057c3e573dcd661f082c67cc8668eef2f761b4ef34aa0dca2`.

Both archive manifests identify the candidate source commit. The vendored source
RELEASE-SOURCE.json git_archive_sha256 matches a local git archive of the exact
candidate: `9f177e45f0aef2e49c3d61e4d989226ea4e539e714706bc477242e15c72e4a3b`.
Archive hashes also match release-manifest.json and the supplied SHA256SUMS.
Codex inspected identity/checksum metadata; no additional installer execution
or independent product acceptance is claimed here.

Authenticated command to download/extract the same artifact into a new directory:

```bash
gh run download 41272131089 --repo JLuethe/rfile \
  --name rfile-candidate-3f9af8d89dcf2990a842cf6e4e6e9c1b28a0c228 \
  --dir /path/to/new/acceptance-directory
```

To retain the ZIP itself for comparison with the GitHub digest:

```bash
gh api repos/JLuethe/rfile/actions/artifacts/13069527384/zip > 13069527384.zip
sha256sum 13069527384.zip
```

## Preserved evidence and logs

Raw evidence root:
`/home/john/Code/rfile/.reference/ci-pass9-3f9af8d/`.

- `artifact-zips/13069527384.zip`: exact GitHub download.
- `artifacts/rfile-candidate-3f9af8d89dcf2990a842cf6e4e6e9c1b28a0c228/`: original downloaded payload, plus a local checksum file.
- `SHA256SUMS.artifacts`: ZIP and every original payload-file hash.
- `artifact-verification.json`: digest, archive-manifest and source binding checks.
- `logs/run-41272131089-attempt1.zip`: full packaging logs and an extracted copy.
- `logs/run-41272111258-attempt1.zip`: full linux/msrv logs and an extracted copy.
- `run-*-final.json`, `jobs-*-final.json`, `check-runs-final.json` and
  `artifacts-*.json`: final run/job/check/artifact metadata; initial snapshots
  are retained separately.
- `SHA256-INVENTORY`: hashes of all retained evidence, including a copy of this
  handoff. No artifact or evidence directory is overwritten for a future test.

Full log archive SHA256 values:

```text
4c31a733767677ef80d86db831c0218a945a3cb4f9c616cd808ca1580028c4db  run-41272111258-attempt1.zip
337f491ed5813451081dccc2945762d2c425f746828cf93cab550a44fd36e56b  run-41272131089-attempt1.zip
```

## Independent acceptance next

Test the exact CI archives above and report results with their hashes. Preserve
new findings and raw evidence in a new QA pass. Do not attribute local pass-7/8
binaries to this CI build: CI used Rust/Cargo 1.99.1 on Ubuntu 22.04, and the
packaged binary records minimum glibc 2.34. Source behavior was independently
verified at 8325cef; subsequent candidate changes are documentation only.

Maintainer dispositions and security qualification remain open. Nothing was
published, tagged or installed into a release; existing draft assets were not
replaced.
