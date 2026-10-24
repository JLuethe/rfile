# Linux kernel isolation and CLI lifecycle

SBX-01 through SBX-09 implement the isolation portion of the [parent specification](../07-remaining-work-spec.md). Kernel restrictions supplement safe parsing and finite budgets; they do not establish a complete security boundary by themselves. No ordinary Rust/C library call installs restrictions in its host process.

## Baseline, primary sources and limits of the claim

There is currently no Rust sandbox; `-S` is accepted and version output says none. The pinned CLI initializes Landlock then seccomp (`upstream/src/file.c:380–412`). `landlock.c:97` allows filesystem reads under `/`, some temporary writes for pipes and skips build actions. `seccomp.c:78` installs an allowlist and default kill action; it also contains read/write-open distinctions. LD_PRELOAD disables C seccomp automatically. The existing annotation-only FIFO SIGSYS and native-decoder no-fork failure are observed mismatches, not behavior to reproduce.

The [kernel Landlock documentation](https://docs.kernel.org/userspace-api/landlock.html) describes self-restriction and runtime ABI negotiation. The [seccomp documentation](https://docs.kernel.org/userspace-api/seccomp_filter.html) explains syscall/argument filtering and no-new-privileges; pointer contents such as pathnames cannot be inspected by a filter. Therefore a path allowlist is not implemented with seccomp string comparisons. Landlock restrictions also do not cover every metadata operation or revoke all authority on already-open descriptors. Report those limits explicitly.

## SBX-01: public modes and actual status

Add `--sandbox=auto|strict|off`; `-S` means off and last option wins. `auto` is the default on certified Linux builds with the sandbox feature. `strict` requires both a tested seccomp policy and the minimum Landlock rights for its requested mode; inability to install is a setup error before untrusted engine parsing. `off` never claims protection.

`auto` attempts supported restrictions. A genuinely unavailable kernel feature may degrade with one precise stderr warning and an accurate status record; invalid policy/permission/resource errors are fatal, not silently treated as lack of kernel support. Unsupported builds/architectures report that distinction. If a required restriction cannot be installed, no input classification is attempted under a reported strict policy. Installation is irreversible; partial success followed by fatal failure exits rather than pretending to undo it.

Expose compiled/available/enforced seccomp and Landlock ABI/rights, selected CLI action policy, degraded reasons and whether read access is input-confined. `--version` before installation reports build capabilities only. Add `--sandbox-status` for actual runtime status without conflating it with the preexisting version feature string. Status output is explicit and must not contaminate ordinary classification stdout.

Do not silently disable the policy merely because LD_PRELOAD is present. Auto mode reports its presence and tests the normal policy; callers can request off for compatibility. Strict mode rejects configurations known to need unapproved runtime behavior. No policy can undo preloaded code that executed before main; document that limitation.

## SBX-02: parsing/action ordering and installation point

Current file-list execution can classify while CLI options are still being parsed. Installing only after final argument parsing would leave `-f` inputs unprotected. Refactor into an ordered event plan that preserves all visible parse-order contracts, including frozen cookie flags/database at first `-f`, streamed versus buffered lists, later flag changes, and outputs preceding a later option error. A planner records an error event in order; it must not eagerly fail before earlier list events that the pin would have executed.

Lifecycle:

1. Parse trusted command-line syntax into the bounded event plan without evaluating magic or classifying inputs. Determine classification versus compile/check/list privileges. Initialize runtime randomness and policy structures; prepare locale/time configuration and bounded raw resources as needed. Defer untrusted TZif/config parsing until the baseline policy is installed; loading a raw bounded buffer is not permission to parse it before the claimed boundary.
2. Establish no-new-privileges and a baseline syscall/filesystem policy **before** source/compiled database parsing and first classification/list execution. Classification baseline permits read-only filesystem access, existing stdout/stderr writes and necessary runtime/metadata IO, while denying new writes, execution and network operations.
3. Load/validate rules and enumerate/preopen allowed operands under that baseline. Strict input confinement requires all paths/file-list entries and selected outputs to be known. Strict mode rejects an unbounded/dynamic filename stream instead of granting `/` reads and claiming confinement. Bounded file-list planning is permitted; preserve its parsing/output semantics through the event model.
4. Tighten strict Landlock to required inputs and prepared resources, and restrict descriptors/syscalls further where safe. Run events with original labels, metadata and symlink semantics attached to the opened resources. Do not reopen a different target by path after preflight.
5. Flush, restore requested timestamps only through the allowed descriptor path, and exit. No helper process or lazy codec loading is introduced after restriction.

If an action's requirements cannot preserve both CLI semantics and the chosen strict policy, fail with a named unsupported combination. Do not read an untrusted file first and install the claimed policy afterward.

## SBX-03: three action policies

| Policy | Filesystem/read scope | Write scope | Execution/network |
|---|---|---|---|
| classification auto | read-only filesystem tree, including future list paths; explicitly **not input-confined** | existing output/error descriptors; optional requested timestamp operation | denied |
| classification strict | preopened inputs, rules/time/locale resources already loaded; no later path discovery | approved output/error descriptors; approved input timestamp descriptors when `-p` | denied |
| compile | source reads; selected output parent(s) and private staging directory | compiled database creation/write/atomic replacement in allowed directory scope only | denied |

Check/list are read-only action policies, not compile writes. Stdin content needs no temporary file in the current Rust design. No broad `/tmp` or `$TMPDIR` write grant is added merely because stdin is a pipe. Any future spool is opt-in with bounded size and a private preopened directory; it requires a separate policy test.

Compile mode may need create/rename/unlink rights in an output directory; Landlock cannot guarantee only a particular basename within a granted directory. State this directory-level scope accurately. A read-only classification policy must not acquire compile privileges from a dormant/invalid action later in the event plan. Reject inconsistent mixed actions or run isolated action phases without trying to loosen an installed restriction.

## SBX-04: Landlock rights and file identity

Use runtime ABI discovery and the intersection of tested compiled rights with available kernel rights. Deny handled write/create/remove/rename/truncate/execute rights by default, permit required reads, and add only the rights an action requests. Do not infer that a high kernel ABI means the selected crate models every new right. Strict requires the configured minimum rights, not merely any ABI >=1. Record actual handled rights and degraded rights by name.

For auto read-only mode, reads under `/` reproduce a broad upstream capability but do not prevent reading other same-user files after a bug. For strict mode, use stable descriptors/allowed file identities, not string-prefix path validation. Symlinks, directory traversal, bind mounts, rename races, `/proc/self/fd`, deleted-but-open inputs and open descriptor inheritance need explicit tests. A stat of an outside path may still reveal metadata; do not describe Landlock as complete information-flow isolation.

Close unneeded inherited descriptors before restricted execution, with a documented policy for caller-provided stdin/stdout/stderr. Restrictions do not revoke previously granted descriptor authority. Preopened writable output is legitimate; it must not be confused with permission to open arbitrary new outputs. Keep CLOEXEC even though execution is denied.

## SBX-05: seccomp policy

Policies are typed, generated from reviewed Rust tables, not untrusted runtime JSON. The filter validates architecture before syscall number; unsupported ABI or x32 syscall encodings cannot bypass the rule set. First certified target is x86_64 Linux/glibc; aarch64/musl require their own traced and tested policy.

Permit only needed runtime operations: bounded reads/preads/seeks, fstat/stat/readlink/access as appropriate to action, close, memory mapping/allocation/protection used by the Rust runtime, futex/signal/exit, and necessary non-network terminal IO. Initialize randomness before filtering, or permit getrandom deliberately with a reason. Do not blindly copy the C allowlist: Rust/libc startup, allocator and TLS behavior differ.

For read-only opens, reject write modes **and** create/truncate/append/tmpfile flags. Filtering only O_WRONLY/O_RDWR is insufficient because O_RDONLY|O_CREAT can still create. Disallow openat2 unless the action needs it and the pathname/flags policy is enforced by another layer; seccomp cannot dereference its open_how pointer. In strict preopened mode, forbid new opens once initialization is finished.

Allow writes only to intended output/error/staging descriptors when the strict finite descriptor set permits it; protect against descriptor-number reuse by denying later opens/duplication or updating design before any reuse. Auto mode must disclose any broader write-on-existing-fd permission. Restrict ioctls to the tested terminal queries; no broad ioctl allowance. `-p` uses the specific descriptor timestamp operation, not broad chmod/chown/path-write rights.

Deny socket/connect/bind/listen, execve/execveat, fork/clone process creation, ptrace, mount, module loading, bpf, io_uring and unnecessary privileged/process-management calls. No multithreaded codec backend is enabled behind this policy; permitting clone for threads would need a new argument/filter contract. Unknown syscalls return EPERM in release policy for controlled failure; architecture mismatch kills the process. Tests must verify expected failures rather than assume all denial is SIGSYS. A debug trap policy can be opt-in for diagnosis, never the release default.

## SBX-06: adapter and candidate dependencies

Keep a safe `SandboxPlan::prepare` / `install` boundary in `crates/rfile-platform`; core keeps forbid(unsafe_code). Audited unsafe is limited to OS calls and pointer/length lifetimes in that adapter. Never expose raw filters or unconstrained syscalls to untrusted rule data. Installation returns an owned immutable status; it cannot be cloned as proof that another thread is sandboxed.

Inspected candidates: `landlock 0.4.7` (MIT OR Apache-2.0, MSRV 1.71), `seccompiler 0.5.0` (Apache-2.0 OR BSD-3-Clause), `rustix 1.1.5` (MIT/Apache alternatives, MSRV 1.65). See [manifest evidence](../../reports/remaining-candidate-dependencies.json). Seccompiler provides typed syscall rules/filter application; disable its optional JSON feature. Landlock supplies rule/compatibility state APIs; validate actual enforcement, not just builder success. Rustix supplies safe OS operations with its own reviewed backend. These are not detector FFI and are not yet product dependencies.

Do not install seccomp from library classification calls. In a multithreaded host, a per-thread policy is not process-wide protection and TSYNC has its own constraints. An application requiring process isolation should use an explicitly launched supervised worker; designing a general worker service is outside this change. The C ABI preserves the same no-surprise rule.

## SBX-07 through SBX-09: tests and acceptance

Every policy is tested in a disposable child process because restrictions cannot be undone. The parent captures status/signal, stderr, actual kernel feature state and elapsed resources. No test depends on privilege or weakens the developer's shell. Native-kernel tests skipped for lack of support are reported as skips and do not satisfy strict-policy certification.

Positive cases: ordinary and malformed inputs, shared database/cache use, MIME/raw/keep-going, invalid-byte paths, relative paths, symlinks/broken links, FIFO/socket metadata, bounded stdin/list input, `-f` before later options/errors, home/MAGIC rules, local timezone/locale output, `-p`, custom source compilation and atomic replacement, every in-process codec, terminal and redirected output. Trace and freeze required syscalls for each supported libc/architecture; a new unexpected syscall requires review, not automatic allowlisting.

Negative cases: read/write opens with every risky flag combination, create/truncate in an unauthorized directory, rename/unlink/link, outside strict input read including `/proc` aliases, use of an inherited extra writable descriptor, network socket, exec of a harmless test stub, process creation, unapproved ioctl and forbidden metadata mutation. The tests must assert an operation was denied; a success classification alone proves nothing. Test kernel ABI unavailable, partial rights, already restricted container, installation failure and LD_PRELOAD. Failure-injection must prove strict mode never begins classification after an installation error.

Completion requires action/flag matrices, actual denied-operation evidence, documented read-scope limits, reviewed unsafe boundary and certified x86_64 Linux behavior. Other targets stay visibly unsupported/degraded until their own policy gate passes. Kernel restrictions do not replace the SEC fuzz/resource review.
