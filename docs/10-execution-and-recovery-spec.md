# Remaining-work specification coverage and recoverable execution

Prepared 2026-10-24 against main `f431a53e8935348cb261b783c97a83a929740c72`.
This adds execution requirements; it does not replace the existing implementation
specifications, implement a supervisor, or activate an unattended job.

## Existing specifications remain authoritative

| Remaining work | Existing implementation contract | Execution order |
|---|---|---|
| Float/date and remaining regex/locale semantics | [SEM-01–10](remaining/04-semantics.md), including delivered R6a/R6b boundaries | R6c, then R6d |
| CDF/ELF qualification and residual fidelity | [CDF](remaining/01-cdf.md), [ELF](remaining/02-elf.md) | Focused residual cases, then final candidate replay |
| Codec gaps and feature activation | [CMP requirements](remaining/03-compression.md) | Explicit supported scope; optional defaults remain gated |
| Linux confinement | [SBX-01–09](remaining/05-sandbox.md) | R7 |
| All 19 C ABI functions and packaging | [ABI-01–12](remaining/07-c-abi.md) | R8; source diagnostic/list compatibility is a prerequisite |
| Sustained campaigns, resource guarantees, independent review | [SEC-01–10](remaining/06-hardening.md) | R9, with early targeted checks |
| Release qualification and platform boundaries | [Parent specification](07-remaining-work-spec.md), [wrap-up plan](09-wrap-up-plan.md) | R10 |

Original baseline paragraphs describe the revisions named in those documents.
Use the delivery ledger and wrap-up review for what is implemented now. A new
specification of the entire system would duplicate contracts and risk divergence.

## R6c execution refinement

Start from `feat/format-time-edges`, research commit
`463de0953b61b62a7f3b317ee1df2a3af95c5c13`, and incorporate current main without
discarding local edits. Its 6,630-case baseline has 2,552 differences and is not
passing evidence. Review the harness before promoting it to an acceptance gate.

1. Preserve exact IEEE input bits, source type, format, source compile outcome,
   compiled-template hash, flags, profile, locale, rounding environment and raw
   result/error bytes. Distinguish native source rejection from runtime behavior.
2. Repair `src/format.rs` precision handling without silent clamping. Preflight
   requested width/precision and output capacity with checked arithmetic. A
   resource refusal is an explicit error, never an approximate shorter result.
3. Verify sign placement before zero fill; space/plus/left/zero interactions;
   alternate decimal points and trailing zeros; exponent spelling and rounded
   `%g` notation selection; signed zero, NaN/infinity spelling and padding.
   Preserve integer/string/character behavior when changing shared padding.
4. Keep C type information and source location available at the evaluator's
   diagnostic boundary. The pin's `%G`/`%F` float rejection requires an explicit
   pinned/corrected decision and golden error tests, not a blanket formatter
   exception or unsafe printf FFI. Corrected uppercase support, if chosen, must
   be documented. Do not change both profiles silently.
5. Extend date vectors for perpetual DST, unusual TZif transitions/footers,
   extreme ranges and standard-time FILETIME normalization. Compare before
   choosing a fix. Keep immutable contexts and checked calendar arithmetic;
   do not mutate process TZ/locale to emulate the oracle.
6. Define C ABI behavior for caller rounding mode and numeric/time locale before
   claiming equivalence. Tests under C/UTC/default rounding prove only that
   environment. Unsupported configurations need a visible contract.
7. Close each difference with a focused regression or individually documented
   profile/safety decision. Run source and v21 routes, default/all-feature/MSRV,
   then the full verification matrix. Record newly added suite counts rather
   than retaining a stale expectation of exactly 24 stages.

Do not mark full R6 native fidelity complete while quantified-history restrictions,
uncertified locale behavior or other declared semantic gaps remain.

## Model selection

Recommendation: **GPT-6 Astra, High reasoning**, for this project's specification,
implementation and compatibility analysis. This is a task-specific judgment,
not a measured rfile model benchmark. Official documentation identifies Astra
for demanding reasoning/coding and supports High; its effort guidance recommends
High for difficult planning/debugging and reserving higher settings for measured
benefit. There is no need to change models solely because this is Rust.
[Model reference](https://developers.openai.com/api/docs/models/gpt-6-astra),
[effort guidance](https://developers.openai.com/api/docs/guides/deployment-checklist).

Use a bounded milestone with explicit tests. Another pass by the same model is
self-review, not the independent review required by SEC-10. Higher effort or
a model switch does not grant access to restricted requests. Actual account
entitlements must be checked through the supported product controls.

## REC-01: recovery objective and limits

Recover repository work after an ordinary client/process/network interruption:
inspect what actually completed, reconcile changes and resume the next authorized
milestone. The supervisor must live outside the agent turn; an ended/blocked
turn cannot wake itself. Reopening a UI is distinct from successfully submitting
a new permitted turn. No documented safety-notice auto-resume hook was established
by this review, and the current GUI thread's CLI resume identity is unverified.

Use documented `codex exec --json` events and explicit session IDs for a future
CLI supervisor. Local CLI `0.164.0` help confirms `codex resume SESSION_ID PROMPT`
and `codex exec resume SESSION_ID PROMPT`. Official documentation describes JSONL
events and saved-session continuation; it does not promise recovery through a
policy restriction. [Non-interactive documentation](https://learn.chatgpt.com/docs/non-interactive-mode).

## REC-02: persistent state

A future implementation stores machine state in an ignored, private per-repository
directory, separate from tracked `HANDOFF.md`. Use atomic write/rename and one
exclusive process lock. Store schema version, canonical repository/worktree,
branch/HEAD, dirty-file hashes, explicit session ID, model/config identity,
milestone, last completed acceptance step, permitted next action, process ID
plus start identity, heartbeat, retry count, job/artifact identities and limits.
No authentication tokens, private inputs or full confidential logs enter Git.

Append events with sequence numbers and completion markers. Record a command
start before dispatch and its observed result afterward. A missing completion
is UNKNOWN, not pass/fail. Preserve partial logs. Commit coherent source changes
at milestone boundaries; checkpointing must not blindly stage private artifacts
or declare failing research ready for main.

## REC-03: state transitions

| Event/state | Required action | Automatic continuation? |
|---|---|---|
| Running, worker identity alive | Observe bounded heartbeat; do not start another worker | No duplicate |
| Turn completed | Validate worktree, artifacts and acceptance results; choose next authorized task | Yes, within configured scope/budget |
| Confirmed transient connection failure or dead client | Ensure old worker is gone; reconcile repository/jobs, then retry saved session | At most three attempts with 10/30/60-second backoff |
| Security/content restriction or model refusal | Preserve evidence; mark `needs_review`; show reason locally; do not resend blocked request | No |
| Authentication, access or billing failure | Preserve state and identify required account action | No |
| Test failure | Save reproduction and continue ordinary debugging within task limits | Yes; never label failed checks successful |
| Unknown failure, conflicting edits or unclear live worker | Preserve edits and stop mutation pending review | No |
| Completed scope, exhausted budget or user stop | Record final state and terminate supervisor | No |

Classify explicit structured error codes where available; retain raw events.
Do not assume every failure has an exit code dedicated to policy, or infer
transience solely from a nonzero exit. Text detection may add a stop condition
but cannot prove an unrecognized error is safe to retry. Validate against the
installed CLI version. Do not rotate models/accounts, alter prompts or start new
threads automatically to route around a security restriction.

After a restriction, a user reviews the task/access, reports a suspected mistake
where appropriate, and explicitly resumes permitted work. The state then moves
to `reconcile`, not straight to a blind replay. Ordinary local builds already
launched may still run; inspect them independently of the agent's request state.
[Official safety-notice guidance](https://help.openai.com/en/articles/20001326).

## REC-04: reconcile before resuming

1. Acquire lock; verify repository identity, branch/HEAD and worktree changes.
   Preserve other edits. No hard reset, clean, force push or silent branch switch.
2. Check live child/worker identity and finished artifacts before running duplicate
   tests or campaigns. Silence alone is not evidence of a hung worker. PID reuse
   must not cause killing an unrelated process.
3. Compare checkpoint with Git, current reports, source/binary/database hashes and
   actual GitHub PR state. A lost response after a successful push/merge must not
   trigger a second publication. Query before retrying side effects.
4. Supply the saved milestone, relevant specification and changed-file summary
   to the explicit saved session. Never use `--last` in unattended scheduling;
   it can select a different task. Avoid dumping entire historical transcripts.
5. Re-run incomplete/relevant checks, update `HANDOFF.md` and atomically checkpoint
   state. Claims of completion require observed acceptance evidence.

The supervisor inherits normal approved permissions; recovery does not justify
disabling sandboxing, approval review or hooks. Require explicit maximum elapsed
time and usage allowance at setup. If reliable usage accounting is unavailable,
enforce bounded turns/time and do not claim a token-spend guarantee. Notifications
are local unless the user separately authorizes messages to other people.

## REC-05: acceptance before enabling unattended recovery

Test with a fake CLI and a temporary Git repository: client death before/after
an edit, missing command result, already-live worker, PID reuse, interrupted
checkpoint write, lock contention, transient retry exhaustion, security notice,
unknown error, changed branch, user edits, already-pushed commit, already-merged
PR, corrupt artifact, failed test and completed scope. Verify zero duplicate
mutating workers, no dropped edits, finite retries, and no model invocation
after `needs_review` without an explicit user resume.

Then run one supervised, bounded permitted session against the installed CLI
to confirm event schema, session identity and continuation behavior. Document
whether it resumes the intended CLI session or a separately created task;
do not claim that it revives an unrelated GUI thread. Keep the supervisor off
until these checks pass. No runner/service is implemented or enabled by this spec.

## Manual continuation available now

From this repository, use the existing `HANDOFF.md` prompt in the current app,
or select the appropriate saved CLI session with `codex resume`. If its identity
is known and accessible, an explicit invocation is:

```sh
codex resume SESSION_ID "Read HANDOFF.md and docs/10-execution-and-recovery-spec.md. Reconcile repository and running-job state, then resume the next permitted R6c task."
```

`SESSION_ID` is a placeholder, not an identifier discovered for this conversation.
Do not run it as-is. A request may still be restricted; resuming is not a guarantee
of access. The supported response to repeated mistaken restrictions is review,
feedback/Support and the official access process, not an infinite retry loop.


### Local test workers are separate from agent recovery

The implemented `tools/fuzz_queue.py` can continue a bounded local test workload
outside a chat turn. It records real worker identities and results, and stops on
failure, candidate change or deadline. It never invokes a model, retries a blocked
request or restarts the GUI thread. Its scope and validation are in the
[hardening specification](remaining/06-hardening.md#bounded-overnight-queue).
An actual started service is recorded separately in HANDOFF and its manifest;
shipping this runner does not itself mean a background job is active.
