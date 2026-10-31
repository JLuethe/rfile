# Rfile final CI artifact acceptance report

**Verdict: ACCEPTED for the proposed experimental `x86_64-unknown-linux-gnu` candidate assets**, subject to the scope limits below. This is the exact final CI artifact from run `41178070592`; no interim/local binary was substituted. This is artifact/runtime QA, not a full security review or evidence for other platforms or every glibc 2.34 host.

## Identity and hashes

- Repository/run: `JLuethe/rfile`, release-candidate workflow run `41178070592`, status `completed/success`.
- Source commit: `e64ce034bee6a6c0959872d9b8f632d692dbfaa3`.
- Frozen source tree: `91506a94437571b6fc3257f1d2e860824e19b727`, exactly the same tree as `a781`.
- Candidate directory: `/home/john/Code/rfile/.reference/package-qa-e64/candidate`.
- Source tarball `rfile-0.2.0-rc.1-source.tar.gz`: SHA256 `f7d2356b2699908cfe765c433e7d99e73eaffb4c9d979fbc99e658081d913bd1`.
- Linux binary tarball `rfile-0.2.0-rc.1-x86_64-unknown-linux-gnu.tar.gz`: SHA256 `d2b01ffb9988a7797e9467c9a3c2efee3fa28128a70d35dcb0db7d4b791cd379`.
- Packaged executable `bin/rfile`: SHA256 `64ebbaf7153cd557b6cb27af4616e441f59fab9d6bf2dc0dfb52556b00b0c8f6`.
- `SHA256SUMS` passes for both archives. The release manifest and `package-qa.json` identify the same source commit and archive hashes. `RELEASE-SOURCE.json` identifies e64; its recorded committed-tree archive hash `d371abe9119ea89074f4e6e9ba320d8b66c197ca940b1ce9b31cc8f0233cd065` matches independent `git archive e64...` output. The binary manifest’s 129 file hashes/modes all match; only expected non-installed root support files are `INSTALL.md` and `install.py`.
- Manifest claims glibc 2.34; ELF version imports confirm `GLIBC_2.34` as the maximum. The binary depends on `libc.so.6` and `libgcc_s.so.1`, and does not link `libmagic`.
- Source archive contains 5,058 files, 64 vendored crates, 120 dependency/license files, and the project/upstream notices (`LICENSE`, `NOTICE`, `LICENSES/README.md`, `data/UPSTREAM-COPYING`, `data/UPSTREAM.json`, `data/SOURCES.json`). The pinned upstream identity is file 5.48, commit `41b1123083ab0bbdfe32f6d4ec14f6aec9a91e59`.

Detailed computed provenance and payload data are in [`provenance-payload.json`](provenance-payload.json); the exact downloaded archive hashes are in [`downloaded-file-hashes.txt`](downloaded-file-hashes.txt).

## Checks and outcomes

- The candidate’s own `tools/check_cli_package.py`: **PASS, 14 checks**, with `CARGO` unset, the caller PATH lacking `~/.cargo/bin`, `TMPDIR` under the e64 scratch directory, and `CARGO_BUILD_JOBS=1`. This includes checksum/manifest checks, version/capabilities, text file/stdin/MIME/gzip/errors, install/uninstall, and its fresh-cache offline all-target/all-features check. Raw JSON and invocation log: [`provided-checker.json`](provided-checker.json), [`provided-checker-command.log`](provided-checker-command.log).
- Non-UTF-8 positional filename handling: **PASS** when raw bytes reach the exact packaged binary directly on the host, and when an ASCII shell glob resolves the mounted raw-byte filename inside Ubuntu before `exec`. An exploratory test passing invalid UTF-8 directly through Podman’s command argv produced a U+FFFD replacement; it does not reproduce with the actual binary via direct byte argv or in-container shell expansion, so it is retained as an OCI harness transport observation. Evidence: [`nonutf8-filename-check.json`](nonutf8-filename-check.json), [`nonutf8-isolation.json`](nonutf8-isolation.json), [`nonutf8-isolation-command.log`](nonutf8-isolation-command.log), and the original transport observation [`nonutf8-filename-command.log`](nonutf8-filename-command.log).
- Independent adversarial installer matrix: **PASS, 10 checks**. Covers absolute and space-containing prefix, relative prefix rejection, pre-existing destination protection, symlink protection, payload and manifest tampering, changed-file uninstall refusal, cleanup atomicity, unrelated data preservation, and symlinked prefix handling. Results: [`independent-installer-results.json`](independent-installer-results.json); command/status: [`independent-installer-command.log`](independent-installer-command.log).
- Independent release-mode CLI source build: **PASS** with the extracted source archive, a fresh empty Cargo home, vendored dependencies, no network (`--offline`), `--locked`, one job, and `x86_64-unknown-linux-gnu` target. The built binary reports version `0.2.0-rc.1`, embedded magic v21, and the pinned upstream commit. This local build used the installed stable toolchain and is not claimed to be byte-identical to the CI binary. Log/hash/version: [`offline-release-build-final.log`](offline-release-build-final.log), [`offline-release-binary.sha256`](offline-release-binary.sha256), [`offline-release-binary-version.txt`](offline-release-binary-version.txt).
- Independent minimal-container runtime matrix against the **exact extracted final archive binary** using rootless Podman: **PASS, 13 checks**, all container cleanup statuses 0. Image was `docker.io/library/ubuntu@sha256:076e7bf82319619f28dd0aa9c02ab34b055ad37100395b4f49a2ffba04fb5e11`, image config `a7b0f6af325461862a99c677cf3229ca5241d9f4f4289bd00cc2cce91a3aff23`. Container used network none, read-only root and input mounts, all caps dropped, no-new-privileges, uid/gid 65534, 256 MiB, 32 process limit, and one CPU. It exercised absent implicit `/etc/localtime`, embedded-default path/stdin/gzip classification, `TZ=UTC`, `UTC0`, empty TZ, explicit invalid/missing zones, malformed/unreadable existing localtime, and a valid UTC TZif. Image glibc is 2.35; this establishes behavior in that image only. Raw per-command report/log: [`ubuntu22-runtime-qa.json`](ubuntu22-runtime-qa.json), [`ubuntu22-runtime-command.log`](ubuntu22-runtime-command.log). The self-contained checker used for this run is [`timezone-container-regression.py`](timezone-container-regression.py), SHA256 in [`timezone-container-regression.sha256`](timezone-container-regression.sha256).
- CI also uploaded [`minimal-runtime-qa.json`](../candidate/minimal-runtime-qa.json), reporting 13/13 pass in the pinned Ubuntu image using Docker. Independent Podman results above match.

## Commands and preserved evidence

- Status: `gh run view 41178070592 --repo JLuethe/rfile --json status,conclusion,headSha,workflowName,url`; saved response: [`run-status.json`](run-status.json).
- Artifact: `gh run download --repo JLuethe/rfile --name rfile-candidate-e64ce034bee6a6c0959872d9b8f632d692dbfaa3 --dir .reference/package-qa-e64/candidate` (exit 0).
- Archive check: `cd .reference/package-qa-e64/candidate && sha256sum -c SHA256SUMS` (exit 0).
- Package checker invocation: `python3 <extracted-source>/tools/check_cli_package.py <candidate-dir> --report <evidence>/provided-checker.json` (exit 0, 14 checks).
- Podman checker invocation:

  ```sh
  python3 .reference/package-qa-e64/evidence/timezone-container-regression.py \
    --engine podman \
    --image docker.io/library/ubuntu@sha256:076e7bf82319619f28dd0aa9c02ab34b055ad37100395b4f49a2ffba04fb5e11 \
    --binary .reference/package-qa-e64/binary-extracted/rfile-0.2.0-rc.1-x86_64-unknown-linux-gnu/bin/rfile \
    --report .reference/package-qa-e64/evidence/ubuntu22-runtime-qa.json \
    --scratch-root .reference/package-qa-e64/tmp
  ```

## Scope and limits

The result accepts the exact source and binary archives for this experimental x86_64 Linux/glibc candidate based on integrity, source identity, installer safety checks, offline source build, and the tested Ubuntu 22.04/glibc 2.35 runtime. It does not certify every glibc 2.34 system, any other OS/architecture, libmagic compatibility as a whole, or a completed security review. The manifest itself reports that independent security review and sustained fuzz gates remain incomplete.
