# Experimental 0.2.0-rc.1 final acceptance

Frozen release source: `e64ce034bee6a6c0959872d9b8f632d692dbfaa3`.
Git tree: `91506a94437571b6fc3257f1d2e860824e19b727`.
Independent source/sanitizer execution used `a781c017b3495121a4203053cbabe55225cb0130`,
which has the exact same Git tree. Reports retain their executed identities.

The downloadable profile is the default x86_64 Linux/glibc CLI with gzip/zlib;
additional codecs, confinement and C ABI remain experimental source options.
No release is published by these checks. Existing v0.1.0 remains unchanged.

## Artifact identity

[CI run 41178070592](https://github.com/JLuethe/rfile/actions/runs/41178070592)
built the frozen e64 commit on Ubuntu 22.04 using Rust 1.99.1.
The binary needs GLIBC 2.34 symbols and passed on Ubuntu 22.04 (glibc 2.35).
This does not certify every glibc 2.34 distribution.

| Exact CI archive | SHA-256 |
|---|---|
| Binary | `d2b01ffb9988a7797e9467c9a3c2efee3fa28128a70d35dcb0db7d4b791cd379` |
| Vendored source | `f7d2356b2699908cfe765c433e7d99e73eaffb4c9d979fbc99e658081d913bd1` |

Binary payload SHA-256: `64ebbaf7153cd557b6cb27af4616e441f59fab9d6bf2dc0dfb52556b00b0c8f6`.
The [release manifest](release-manifest.json) records every payload hash and mode.

## Verification status

The independent GPT-6 Luna source matrix passed 32 default stages and 34
all-feature stages, both with benchmarks. The latter required actual CLI and
kernel sandbox execution in debug and release (eight kernel child cases each).
Both mutation runs executed 103 fixtures / 1,339 truncations and 30,000
deterministic mutation cases. Rust 1.87 all-feature/all-target checks, all 15
Python regressions, and 582/582 valid source dumps passed; two malformed source
cases were separately recorded. The [independent source verdict](source-profile-acceptance.md) records the
executed identities and preserved failures. See [default](verification-default.json) and
[all-feature](verification-all-features.json) run reports.

The [independent artifact verdict](artifact-qa/evidence/final-acceptance-report.md)
accepts the exact proposed archives: 14 package checks, 10 adversarial installer
checks, 13 constrained Ubuntu runtime cases, fresh-cache offline source builds,
and 129 payload hashes/modes passed. Raw-byte filenames passed directly and
inside the container; invalid-byte arguments passed through Podman's JSON
transport were isolated as a launcher observation.

Prerequisite failures from ptrace/LeakSanitizer, temporary storage quota and
cached relocation paths are retained separately from passing retries. They were
not silently converted to passes. Initial Claude testing and Luna retests remain
attributed to their actual agents and source snapshots.

The [release draft](https://github.com/JLuethe/rfile/releases/tag/untagged-c161acd97cb00e0a1c09)
is unpublished, targets e64, and contains the accepted CI bytes. Maintainer
review and publication remain the final human action.

## Reproducibility and security scope

Two fresh offline builds of e64 on the same host/toolchain produced byte-identical
binary/source archives, manifests and checksums. Their source archive also matches
CI. Their binary archive differs from CI; no cross-host binary reproducibility is
claimed. The published candidate assets must be the accepted Ubuntu CI bytes.
See [local reproducibility](reproducibility.json) and
[CI comparison](reproducibility-ci-comparison.json).

Bounded ASan replay built and exercised all 25 targets successfully, totaling
75.771719 measured CPU seconds. Two FFI Miri tests passed. Every sustained
campaign gate remains false. Those checks are not independent security review.
Both lockfiles had no reported vulnerabilities/warnings at the recorded RustSec
revision in the parent directory's audit reports; neither lock changed afterward.

The stronger qualification endpoint still requires at least 600 measured CPU-hours
across 25 targets, coverage/resource evidence, qualified final replay and independent
unsafe/resource review. Other-platform certification, optional binary variants,
and dependency PRs #21/#22 remain deferred.

## Raw evidence

The draft includes `rfile-0.2.0-rc.1-qa-evidence.tar.gz`, `release-evidence.json`
and `EVIDENCE-SHA256SUMS`. The bundle retains raw matrices, all five source run
directories, logs, initial failures, artifact QA, ASan/Miri commands, reproducibility
records and CI/audit evidence. Relative raw paths named by the source verdict and
hash inventory refer to the `source-qa/` directory inside that bundle.
