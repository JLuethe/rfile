#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Shared test-only oracle identity. Never imported by production code."""
import hashlib,json,os,subprocess
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
UP=Path(os.environ.get('RFILE_REFERENCE',str(ROOT/'upstream'))).resolve()
PIN=json.loads((ROOT/'data/UPSTREAM.json').read_text())['commit']
def verify_reference():
    actual=subprocess.check_output(['git','-C',str(UP),'rev-parse','HEAD'],text=True).strip()
    if actual != PIN: raise SystemExit(f'Reference revision drift: {actual} != {PIN}')
    dirty=subprocess.check_output(['git','-C',str(UP),'status','--porcelain','--untracked-files=no'],text=True)
    if dirty: raise SystemExit(f'Reference tracked worktree is modified: {UP}\n{dirty}Preserve it; build a clean pinned checkout and set RFILE_REFERENCE.')
    return actual

def manifest():
    verify_reference()
    files=['src/.libs/file','src/.libs/libmagic.so','magic/magic.mgc','config.h','config.status','src/file']
    hashes={name:hashlib.sha256((UP/name).read_bytes()).hexdigest() for name in files if (UP/name).is_file()}
    required=['src/.libs/file','src/.libs/libmagic.so','magic/magic.mgc','config.h']
    if any(name not in hashes for name in required): raise SystemExit('Clean reference must be configured and built before verification')
    version=subprocess.run([str(UP/'src/file'),'--version'],capture_output=True,text=True,check=True)
    return dict(path=str(UP),commit=PIN,tracked_worktree_clean=True,sha256=hashes,version_stdout=version.stdout,version_stderr=version.stderr)
verify_reference()
