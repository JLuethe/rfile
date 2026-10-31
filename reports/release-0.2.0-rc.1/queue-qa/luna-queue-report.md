# Independent fuzz queue QA

**Scope:** `tools/fuzz_queue.py` / `tools/fuzz_campaign.py` / `tools/fuzz_queue_check.py` at isolated clone HEAD `0e9590d252383932c44c53d9d0402ff7c32fe8df` (queue files match PR #19 `7a3ea75`). This is infrastructure QA, not sanitizer source qualification. No source edits, merges, installs, network access, or long campaigns.

## Result

**PASS for experimental queue infrastructure after running the authorized bounded check with namespace permission.** Do not treat the short campaigns as a security gate or release qualification.

- First normal-sandbox attempt failed at bubblewrap startup (`Failed to create NETLINK_ROUTE socket: Operation not permitted`); no fuzzer ran. Retried the same `timeout 150s python3 tools/fuzz_queue_check.py` with escalated permission needed for isolated namespaces. It exited 0 in about 16 seconds.
- Two workers (`source_diagnostics`, `abi_state`) completed with return code 0 and valid `wait4` accounting; they recorded 10.959582 and 10.965220 CPU seconds. The reports were accepted only after matching supervisor PID, start time, source digest, and binary digest.
- Both were explicitly `partial-campaign-complete`; `completed_gate=false`. Queue status was `complete` only for successful short jobs, and `qualified_release=false`.
- A duplicate queue was rejected while the first queue held its lock (`A queue already owns this repository`); a duplicate target campaign was rejected (`Another campaign owns this target`).
- The 0.0003-hour wall-deadline run stopped with `stop_reason=wall-deadline`, queue status `stopped`; its supervisor returned 2 / report status `interrupted`, and no fuzz/queue worker processes remained afterward.
- The queue and supervisor SHA-256s, clean git status, commit, source SHA-256, and individual binary identities are recorded in the raw queue JSON and logs. The three sanitizer binaries were **prebuilt and copied** into this clone; their hashes are in `binary-hashes.txt`. No fresh sanitizer build was performed.

## Assessment and limits

Lock exclusivity, duplicate rejection, partial-versus-qualified accounting, deadline interruption, and worker cleanup all behaved as intended in this bounded check. Queue rows retain binary digests and only credit reports whose identity fields match the launched worker and pinned inputs. The review supports accepting PR #19 as experimental queue infrastructure; it does not establish sanitizer coverage, code safety, or the 24-hour campaign gate.

The low CPU budget in the smoke setup necessarily ran a full 10-second chunk per target (about 11 CPU seconds each), which is expected from chunk granularity and not evidence of precise subchunk budget enforcement. A real queue's per-target target is 24 CPU hours and its maximum chunk is 300 seconds by default.

## Evidence

Raw evidence: `luna-queue-selftest.log`, `luna-queue-selftest.json`, `luna-normal-queue.json`, `luna-deadline-queue.json`, `luna-sandbox-blocked-queue.json`, and the per-target queue/fuzzer logs. The first, sandbox-blocked attempt is retained separately; it is an environment failure and was not counted as a queue implementation failure.
