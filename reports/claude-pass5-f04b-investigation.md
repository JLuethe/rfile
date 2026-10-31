# F-04b root-cause investigation (before encoding changes)

Investigated at repository HEAD f5e4199 (code candidate 8e33173217dd89b20c227d7665319681114ee9cb).
No encoding production code was changed. Local Codex observations are under
`.reference/pass5-codex-investigation/f04b.json`; they are not independent acceptance.
Oracle: clean pin 41b1123083ab0bbdfe32f6d4ec14f6aec9a91e59, explicitly selected
magic database, LC_ALL=C and TZ=UTC0.

| Case | Bytes | Rfile | Pinned oracle |
|---|---|---|---|
| arj first 2 | 60ea | Unicode UTF-8 | ISO-8859 |
| pdf-utf8-bom first 2 | efbb | Unicode UTF-8 | ISO-8859 |
| matilde.arm first 11, -k | 2321414d520a3c085b64de | AMR script + UTF-8 text | AMR script + ISO-8859 text |
| ASCII + incomplete tail | 41e282 | UTF-8 | Non-ISO extended ASCII |
| Complete character + incomplete tail | c3a9e282 | UTF-8 | UTF-8 |
| Complete character | c3a9 | UTF-8 | UTF-8 |
| One-byte control | ef | very short file | very short file |

`src/encoding.rs::utf8` accepts an incomplete tail by decoding only the prefix
before `Utf8Error.valid_up_to()`. `detect` then selects UTF-8 if *any raw input
byte* has its high bit set, including the incomplete tail that was discarded.
There need not be any complete non-ASCII character in the decoded prefix.

Pinned `src/encoding.c::file_looks_utf8` instead sets `gotone` only after all
continuation bytes of a multibyte character are consumed. An incomplete tail
jumps to `done` without setting it. Plain UTF-8 detection requires a return >1
(gotone), whereas BOM handling uses a distinct >0 condition. The reported
matilde prefix has acceptable ASCII/control bytes followed by incomplete DE;
ARJ/PDF likewise contain no complete multibyte character. The Latin-1 fallback
therefore wins in the oracle.

Proposed later fix: for the plain UTF-8 branch, require at least one decoded
non-ASCII character rather than any high byte in the original input. Preserve
incomplete-tail tolerance after completed characters, BOM-specific behavior,
ASCII controls, and parser `looks_utf8` semantics. Do not simply reject every
incomplete tail. This report precedes any encoding fix; F-04b remains OPEN.
