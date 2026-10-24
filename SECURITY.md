# Security policy

rfile parses untrusted files and rule databases. Version 0.1.0 is experimental:
compatibility, resource limits and security hardening are still being reviewed.
The current development branch receives fixes; no stable security support or
response-time guarantee is offered for released versions yet.

## Reporting a vulnerability

Use GitHub's private vulnerability reporting form:
https://github.com/JLuethe/rfile/security/advisories/new

Please report suspected memory-safety issues, excessive CPU/memory consumption,
stack exhaustion, panics, filesystem races, decompression problems and security-
relevant misclassification privately. Avoid public issues containing exploit
samples until maintainers have assessed the report.

Include the commit/version, platform, flags, rule database identity, configured
limits, a minimized input or rule reproducer, expected/observed behavior and any
sanitizer or resource measurements. Do not include private documents, credentials
or personal data in a sample. A script generating a synthetic reproducer is ideal.

## Current boundaries

First-party Rust code forbids unsafe code, but std and dependencies may use it.
There is no kernel sandbox and no completed independent security audit or prolonged
coverage-guided fuzz campaign. Input sampling and classification are heuristic;
a result does not validate a complete file or establish that it is safe to execute.
See docs/06-implementation-status.md for known compatibility and security gaps.
