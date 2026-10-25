# Sustained fuzzing, resource assurance and security review

SEC-01 through SEC-10 are the hardening gates for the [remaining-work plan](../07-remaining-work-spec.md). This document specifies work to execute; it does not claim a completed campaign or audit. The original 31,339 deterministic mutation cases are smoke evidence, not the final security gate. The R8 branch now has 24 buildable targets, including stateful C ABI ownership/error transitions; current campaign manifests determine actual measured work.

## SEC-01: threat model and invariants

Attacker control includes file contents, source/compiled rules when a caller accepts them, compressed headers/dictionaries, format offsets/counts, filenames/symlinks and racing file changes. A caller controls its configuration and allowed IO; hostile input must not grant additional authority. The C ABI additionally has a documented foreign-caller memory/lifetime contract; arbitrary dangling C pointers are not made safe by Rust.

Review these independent outcomes: memory corruption; panic/abort; excess memory/CPU/IO; recursion/stack exhaustion; descriptor/resource leak; out-of-scope IO; wrong MIME/evidence; cache/state leakage between calls; output/control-byte injection beyond declared raw behavior. A classification is heuristic and does not validate the full file. Rust's first-party unsafe prohibition does not prove absence of logical or resource bugs, and dependencies may use unsafe.

Mandatory invariants: every read lies in a checked slice or requested file range; arithmetic cannot wrap into a valid-looking range; shared work/memory budgets include recursive/codec/format children; limit errors never become confident fallback; no process spawning/network/extraction in engine; output escaping fits the final cap; descriptor ownership and restoration hold on every return; successful compiled decode satisfies structural invariants; concurrent engines cannot share mutable match state. Each invariant maps to unit/property/fuzz/review evidence.

## SEC-02: targets and structure-aware reachability

| Target family | Input / seed strategy | Essential oracle or invariant |
|---|---|---|
| compiled database | tiny valid v21 images, header/count/endian/type/string mutations, imported pinned rules | validate/compile/decode invariants, no panic or unchecked allocation; preserve accepted record semantics |
| magic source/compiler | grammar-generated rules plus raw bytes, escaped regex/offset/format variants | successful parse/compile/import equivalence; strict error provenance |
| fixed DB classification | public fixture corpus plus random and near-limit data | output/resource bounds; primary-oracle differential in isolated driver |
| combined rules+data | bounded generated trees, named recursion and indirect offsets | finite depth/work, no cache/input lifetime coupling |
| formatting/encoding | typed printf values, IEEE bits, Unicode boundaries, EBCDIC | finite output and deterministic declared-locale behavior |
| CDF container | valid v3/v4 sector maps, streams, chains and cycles | checked sector traversal and finite IO; no signature-only coverage illusion |
| CDF properties/catalog | valid section/property/type/vector constructors | per-section bounds, complete initialization, profile differences registered |
| ELF header/note | ELF32/64 LE/BE, all owner/type/size families | every dispatch and length/alignment boundary reached |
| ELF seekable/core | virtual file images with program/section maps and short-read faults | positional reads, virtual-address mapping and restored borrowed descriptor state |
| one per enabled codec | generated valid frames plus structured header/block/member corruption | pre-allocation cap, bounded expansion, progress and checksum status |
| codec envelope | dispatch collisions, nested layers, concatenation | correct registry ordering, no budget reset, correct inner/outer flags |
| regex AST and VM | ERE/backreference grammar and ambiguous capture histories | no silent dialect change; correct earliest/longest result or explicit Limit |
| timezone/locale | bounded TZif and character sequences; separate platform adapters | no global mutation, guard restoration, bounded calendar/decoder state |
| C ABI stateful | valid allocated pointers/handles, operations generated in sequences | lifetime/error transitions, panic boundary and ownership contract |
| IO/fault injection | deterministic short reads/errors/cancellation/allocation failures | cleanup, atomic state transitions and correct error category |

Existing `classify` and encoding targets consume bounded buffers; they do not exercise real random-access CDF/ELF code paths by themselves. Add internal test interfaces or synthetic InputView rather than using arbitrary real filesystem paths from fuzzer bytes. Structure-aware builders are independent enough to avoid reproducing a parser's arithmetic bug in its generator. Raw byte mutation supplements, rather than replaces, those seeds.

Historical vulnerability regressions must have an identified public source, license/redistribution review, CVE/commit linkage, minimizing notes and expected safe outcome. Existing filenames mentioning a CVE do not establish coverage of every variant. Real core dumps and private documents cannot be fuzz seeds committed to the public repository. Use synthetic equivalents or locally held corpora with hashes only.

## SEC-03: differential isolation and discrepancy handling

Run the pinned C oracle in a separate supervised process, not in the Rust fuzzer's address space. A C crash/hang must be distinguishable from a Rust failure. Use explicit timeouts/process limits and preserve either side's partial output/status/signal. Compare only matching capabilities, profiles, API modes, input bytes and rules; never count both NULL/error results as equality without comparing error contract.

For regular matching and parser/compiler cases, compare against the pinned primary build. For corrected CDF behavior use the separately identified patched oracle and independent format fixtures. A crashing or undefined reference path is labeled suspected defect and excluded from exact deterministic truth, while still retaining Rust bounded-safety expectations. Minimize discrepancies, record a category and keep them in the ledger. Do not add broad ignore patterns to reach a green percentage.

Save stdout/stderr and library result as bytes, process and `magic_errno` states, flags/limits, source/compiled identity and order, locale/TZ/tzdata, feature set and entrypoint. Generated tests must isolate MIME type-only, encoding-only and combined output, not just successful human classification.

## SEC-04: campaign protocol

Use a pinned nightly/toolchain and cargo-fuzz/libFuzzer version, with address sanitizer where supported. Track the target binary, compiler flags, dependencies/features, seed, corpus hashes and actual target command. Example for a future implemented target:

```sh
cargo +nightly fuzz run cdf_properties -- \
  -max_total_time=86400 -timeout=5 -rss_limit_mb=768 -max_len=1048576 -seed=712548
```

This is a template, not a command executed in this documentation change. Wall time in that command is not a measurement of CPU-hours. A campaign supervisor aggregates CPU usage across all workers, records interruptions and completed work, and enforces memory/process limits. Run in disposable containers/VMs with no credentials/network access and a private writable artifact directory. Never expose the development host's real files to arbitrary path bytes.

Release gate: **at least 24 measured CPU-hours per mandatory enabled target**, including each codec and the ABI target once present; at least two deterministic seeds per target; replay all discovered regressions against the final candidate; and rerun affected campaigns after fixes or material dependency changes. The manifest enumerates targets and worker-hours, so a single campaign cannot be credited to all codecs. Twenty-four hours is a minimum engineering gate, not a proof of security.

Coverage must show successful header/parser progress and each implemented high-risk variant/dispatch branch. Publish line/region coverage and reachable-branch inventory; a high aggregate percentage made of common encoding paths is insufficient. Mark unreachable or platform-conditional branches with a reviewed explanation. Prioritize new mutators/seeds where coverage stalls instead of merely extending an unproductive run.

Per-input five-second watchdog and 768 MiB fuzz-process ceiling are initial harness values; lower them per target after measuring normal cost. These are harness limits, not product guarantees. Keep 64 KiB fast input targets plus separate larger-input targets reaching block, section and allocation boundaries up to declared product caps. A global max_len too small to cross a parser boundary is not adequate coverage.

## SEC-05: campaign manifest and failure ledger

Each campaign writes JSON with schema version, Rust/oracle/patch/rules hashes, target and binary hash, toolchain/sanitizer, features/profile/limits, environment/kernel/architecture, start/end, wall/user/system CPU, workers, seeds, corpus hashes before/after, executions, coverage, peak RSS, artifact counts, interruptions and final disposition. A successful target build is recorded separately from a successful run. Failed/skipped/aborted jobs cannot be reported as passing campaign hours.

Each finding has a stable ID, minimized input/generator, reproduction command, expected invariant, affected range, symptom, severity/rationale, root cause, fix commit, tests, rerun evidence and closure reviewer. Include timeouts, memory explosions and misclassification/security-boundary errors, not only crashes. Confidential reports stay in private security reporting until coordinated disclosure; public reports contain only approved synthetic/minimized artifacts.

## SEC-06: property, fault and concurrency tests

Use deterministic property generators for checked offset algebra, compiled endian roundtrips, directory/FAT invariants, ELF virtual mappings, regex histories and typed formatting. Compare at least one independent representation or oracle so tests do not simply mirror implementation. Preserve flags/profile/config through source and compiled database roundtrips.

Inject allocation reservation failures, short reads at every structural boundary, EINTR/IO errors, file replacement/truncation, deadline expiry, codec error after partial output, regex cache compilation failure and output-write failure. Check that no path leaks descriptors, invalidates owned Rust results or installs partially reported security state. Bound native-stack use under the maximum configured recursion on a small worker stack; if a new frame layout exceeds it, migrate to an explicit heap call stack rather than raising the cap.

Stress shared immutable databases with concurrent cold regex compilation and different flags/locales/zones/limits. Check aggregate cache reservation races, failed-cache retry policy, poison/panic behavior, output pointer lifetime in the FFI crate, and no global environment/locale changes. Use race sanitization where supported; do not label an unsupported sanitizer job as executed coverage.

## SEC-07: manual review and unsafe/dependency inventory

Review parser boundary arithmetic and budget ownership independently of conformance work. The source review covers all untrusted lengths/counts, recursive/iterative progression, dictionary/window allocation before construction, time/locale FFI, error unwinding and filesystem authority. Track every unsafe block and safe wrapper invariant in platform/FFI crates; core must retain forbid(unsafe_code). Include first-party test-only unsafe in the inventory with separate production status.

Inventory transitive dependencies and enabled features, license/source checksums, native linkage/build scripts, unsafe hot paths and current advisories. Run cargo-audit or an equivalent reviewed RustSec check against a recorded advisory-database revision. Dependabot is supplemental notification, not the complete supply-chain audit. Dependency replacement or security patch requires feature-aware differential and fuzz reruns. Do not claim safety merely because the selected codec is written in Rust.

Use Miri for applicable core/FFI ownership tests and sanitizers for native test clients/platform adapters. Miri cannot model all OS syscalls, kernel policies or external library behavior; maintain an explicit applicable/skipped test list. Hardened release builds retain overflow/bounds policies and panic containment at FFI; optimization must not introduce unchecked parsing.

## SEC-08: CI and release evidence

Pull requests run formatting, Clippy, deterministic tests, stable mutation smoke, source/database rebuild, targeted regressions and short fuzz smoke for affected targets. A nightly/scheduled workflow runs bounded longer campaigns and feature/platform matrices, with read-only credentials and artifact retention; it must not automatically run privileged workflows or expose secrets to fork pull requests. Resource-heavy runs can use isolated dedicated workers with recorded configuration.

Release candidates run the full campaigns, independent security review and per-component acceptance matrices. Native Linux/glibc sandbox tests are mandatory; musl/aarch64/32-bit/big-endian and other platforms are separate declared gates. If a platform cannot execute in CI, retain the limitation rather than infer support from a cross-compile.

SEC-09: acceptance requires no unresolved critical/high findings; lower findings need an explicit maintainer disposition and scope limitation. Every detected panic/hang/resource violation has a regression and replay result. Publish measured limits, actual campaign coverage/hours, skipped environments and remaining gaps with the release. “Memory safe” must be scoped to first-party safe parsing and audited boundaries, not presented as complete security certification.

SEC-10: independent review is a separate deliverable. Record the reviewed commit, reviewer scope and findings. Self-review plus green tests cannot be labeled an independent audit. Until that gate is satisfied, keep the experimental security statement and recommend supervised deployment for hostile workloads without claiming completed certification.

Tool references: [Rust Fuzz Book](https://rust-fuzz.github.io/book/cargo-fuzz.html), [Miri](https://github.com/rust-lang/miri). These describe tool capabilities, not the outcome of this project's future runs.


### R8 harness refinement

The campaign supervisor now requires bubblewrap isolation: private namespaces,
no network or host home, a private temporary directory, read-only instrumented
binary/system libraries, and only the selected corpus/artifact directories
writable. Absence or failure of isolation is a failed run, not permission for an
unsandboxed fallback. A small Python parent **inside** the namespace uses wait4
on the actual fuzzer to measure user/system CPU and peak RSS. Waiting for the
outer namespace supervisor did not include its descendant CPU on this host;
that preliminary smoke receives no campaign-hour credit. Logs use unique run
directories so fresh campaigns do not overwrite earlier chunk logs. Invalid or
missing accounting cannot satisfy the sustained gate. Source/binary/tool hashes
must match on resume; candidate changes stop the campaign before mixing work.

The ABI target allocates and owns all pointers/handles itself, exercises valid
and malformed v21 loads, parameters/flags, classification, NULL/zero/count
failures and state queries, then closes each cookie exactly once. It never
interprets bytes as arbitrary pointers or host filenames. This supplements C
ASan/UBSan clients and panic-containment regressions, not a claim that passing
one target proves full foreign-caller correctness.
