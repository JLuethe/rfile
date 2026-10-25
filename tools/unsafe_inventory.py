#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Locate first-party unsafe declarations/blocks for review, not a soundness proof."""
import hashlib,json,re
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
rows=[]
for path in sorted([*ROOT.glob('src/**/*.rs'),*ROOT.glob('crates/**/*.rs')]):
    lines=path.read_text().splitlines()
    for index,line in enumerate(lines):
        if re.search(r'\bunsafe\s*(?:\{|impl\b|extern\b|fn\b|\()',line):
            rows.append(dict(path=str(path.relative_to(ROOT)),line=index+1,code=line.strip(),context='\n'.join(lines[max(0,index-4):index]),source_sha256=hashlib.sha256(path.read_bytes()).hexdigest()))
report=dict(schema_version=1,scope='Lexical first-party inventory, includes test code. Not reachability analysis, a soundness proof, or an independent security review. Std/third-party unsafe is separately inventoried.',core_forbids_unsafe=all('#![forbid(unsafe_code)]'in (ROOT/p).read_text()for p in ['src/lib.rs','src/main.rs','src/bin/probe.rs']),permitted_crates=['crates/rfile-platform','crates/rfile-ffi'],items=rows)
(ROOT/'reports/unsafe.json').write_text(json.dumps(report,indent=2)+'\n');print(dict(items=len(rows),core_forbids_unsafe=report['core_forbids_unsafe']))
if any(not row['path'].startswith(('crates/rfile-platform/','crates/rfile-ffi/'))for row in rows):raise SystemExit('Unsafe outside the current permitted adapter')
