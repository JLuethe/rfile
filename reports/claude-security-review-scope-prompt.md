# Prompt for Claude: scope the independent security review

You are Claude, independent of Codex's production implementation. Prepare a
security-review scope brief for Rfile. **This assignment is scoping only:** do
not start the review execution, fuzz campaigns, sanitizers, long-running tests,
background workers, deployments or publication. Do not edit production code or
the shared checkout. Preserve every earlier ledger and artifact.

## Frozen subject

```text
commit        ca9465f5f93eca75c80352a18a5de108e20762a0
tree          92b34ed5c86ea5737a4eb1f00e3ce085fe59dca5
packaging run 41326583263
artifact ID   13091831221
ZIP SHA256    d7555442445044b8952ac3f3b9d38d963a815e0b6d84ed48bcc1020483945dd2
```

The default CLI archives passed focused independent acceptance in pass 11.
Runtime/build/installer code is unchanged from 8325cef. Later handoff/brief
commits are not new review targets; record the exact frozen identity above.

Read first:

1. `/home/john/Code/rfile-qa-claude/pass11/LEDGER-pass11.md` and CHECKPOINT.md.
2. `/home/john/Code/rfile/reports/claude-pass11-codex-review.md` and
   `reports/remaining-maintainer-dispositions.md`.
3. At the frozen source: SECURITY.md, docs/remaining/06-hardening.md (SEC-01
   through SEC-10), relevant sandbox/codec contracts, crates/rfile-platform/README.md,
   crates/rfile-ffi/README.md, fuzz/Cargo.toml and existing review/coverage reports.
4. Read docs/release-scope.md for historical scope, but distinguish its old e64
   draft identity from the accepted ca9465f candidate. Flag stale statements;
   do not silently replace identities or change the frozen snapshot.

## Required brief

1. **Claims and boundaries.** Separate the shipped default x86_64 Linux/glibc CLI
   and engine from optional source codecs, sandbox and C ABI. Identify what an
   independent agent source review can establish, its limitations, and what would
   require a separate external audit. Do not label conformance tests an audit.
2. **Threat model and trust boundaries.** Map untrusted bytes/rules/databases,
   file offsets, compressed sizes, filenames/symlinks/races, caller configuration,
   environment/locale/TZ, output and FFI pointer/descriptor ownership. Distinguish
   heuristic classification from validation and privilege/sandbox guarantees.
3. **Prioritized manual-review work packages.** List relevant source files and
   invariants, reviewer work, expected evidence and acceptance criteria. Include
   parsing/offset arithmetic; compiled/source validation; recursion and shared
   CPU/memory/IO/output budgets; codec expansion/windows; CDF/ELF random access;
   regex/formatter/encoding behavior; filesystem and installer authority; caches,
   concurrency and error unwinding; unsafe platform/locale/FFI wrappers; optional
   Landlock/seccomp enforcement; dependencies/build scripts and native linkage.
   Identify separate work for opt-in features without implying they ship in the
   accepted binary.
4. **Evidence gap matrix.** For each invariant, distinguish current evidence
   from missing review, coverage, resource, sanitizer/Miri and replay work.
   Historical reports and smoke runs retain their actual commits/profiles.
   A successful build, green verifier, unsafe inventory or broad coverage percent
   is not proof of security. Explain how bounded harness reachability will be
   assessed without starting those runs in this assignment.
5. **Dependency/unsafe review plan.** Specify provenance and advisory-database
   revisions to record, enabled-feature accounting, safe-wrapper invariants and
   applicable versus unsupported analysis. Do not infer present advisory status
   from an old report. Recommend evidence/commands for later authorized execution.
6. **Review protocol.** Define severity, deterministic bounded local reproducer
   expectations, ownership and independent closure. Codex fixes; Claude retests.
   Confidential security findings follow SECURITY.md rather than public exploit
   disclosure. Review findings must distinguish product, harness, oracle and
   infrastructure faults, and maintainer-accepted differences from new defects.
7. **Campaign prerequisite plan.** Explain when the source can be frozen after
   fixes, which checks/archives need renewed acceptance, and how the existing
   isolated supervisor, target/configuration identities, two seeds, CPU accounting,
   coverage/resource measurements and replay would be used later. The repository's
   full gate is 25 targets × 24 measured CPU-hours = 600 CPU-hours; do not reduce
   or claim that scope without an explicit scoped decision. Do not launch it.
8. **Effort, sequencing and decisions.** Give estimated effort per work package,
   dependencies, proposed stopping criteria and a concise list of maintainer
   decisions. Put manual review before expensive campaigns so fixes can enter
   the frozen candidate. Estimates are not executed evidence or authorization.

Read-only source/evidence inspection and writing the brief in a new private QA
directory are in scope. If you notice a potential defect while scoping, record
its location, suspected invariant and required follow-up; do not begin an
unbounded investigation or implement a fix.

Write a new `SECURITY-REVIEW-BRIEF.md`, CHECKPOINT.md and SHA256-INVENTORY under
an unused `/home/john/Code/rfile-qa-claude/security-scope/` directory (select a new
name if occupied). Record reviewer identity, subject commit/tree, sources read,
assumptions, exclusions and which recommendations are unexecuted. End with a
short handoff for the maintainer and Codex. No findings or qualification gates
are closed merely by producing the scope brief.
