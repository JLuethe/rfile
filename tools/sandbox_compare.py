#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Same-binary off/auto/strict replay; primary oracle comparisons are separate.
Run after cargo build --release --all-features. No sandbox is applied to this
parent. The report records actual policy state and exact output/status equality.
"""
import hashlib,json,os,pathlib,platform,subprocess,tempfile,time
ROOT=pathlib.Path(__file__).resolve().parents[1]
BIN=ROOT/'target/release/rfile'
ENV=dict(os.environ,LC_ALL='C',TZ='UTC')
for key in ['MAGIC','POSIXLY_CORRECT','LD_PRELOAD']: ENV.pop(key,None)
start=time.monotonic();rows=[]
def invoke(mode,args,cwd,data=b''):
    return subprocess.run([str(BIN),'--sandbox='+mode,*args],input=data,capture_output=True,cwd=cwd,env=ENV,timeout=30)
with tempfile.TemporaryDirectory(prefix='rfile-sandbox-') as tmp:
    tmp=pathlib.Path(tmp);ENV['HOME']=str(tmp)
    status=invoke('strict',['--sandbox-status','-b',str(ROOT/'README.md')],tmp)
    if status.returncode: raise SystemExit('Strict policy qualification unavailable or failed: '+status.stderr.decode(errors='replace'))
    actual=json.loads(status.stderr)['sandbox']
    if not all(actual[k] for k in ['seccomp_enforced','landlock_enforced','input_content_confined']): raise SystemExit('strict policy did not enforce all required layers')
    fixtures=sorted(p for p in (ROOT/'tests/fixtures').rglob('*') if p.is_file())
    fixtures += sorted((ROOT/'upstream/tests').glob('*.testfile'))
    fixtures += [pathlib.Path('/usr/bin/ls'),pathlib.Path('/usr/bin/bash'),ROOT/'README.md']
    # Stable fixture batches keep setup overhead realistic and exercise the 128-entry cap.
    for offset in range(0,len(fixtures),64):
        paths=fixtures[offset:offset+64]
        for flags in [[],['-i'],['-z'],['-Z'],['-k'],['--extension'],['-p']]:
            args=[*flags,*map(str,paths)]
            reference=invoke('off',args,tmp)
            for mode in ['auto','strict']:
                got=invoke(mode,args,tmp)
                equal=(got.stdout,got.stderr,got.returncode)==(reference.stdout,reference.stderr,reference.returncode)
                rows.append(dict(mode=mode,flags=flags,fixtures=[str(p.relative_to(ROOT)) if p.is_relative_to(ROOT) else str(p) for p in paths],equal=equal,status=got.returncode,stdout_sha256=hashlib.sha256(got.stdout).hexdigest(),stderr_sha256=hashlib.sha256(got.stderr).hexdigest()))
                if not equal: raise SystemExit(f'sandbox mismatch: {mode} batch {offset} {flags}\n{got.stderr!r} vs {reference.stderr!r}')
    for mode in ['auto','strict']:
        for flags in [[],['-i'],['-z']]:
            got=invoke(mode,[*flags,'-'],tmp,b'hello sandbox stdin\n');ref=invoke('off',[*flags,'-'],tmp,b'hello sandbox stdin\n')
            equal=(got.stdout,got.stderr,got.returncode)==(ref.stdout,ref.stderr,ref.returncode)
            rows.append(dict(mode=mode,flags=flags,fixtures=['<stdin>'],equal=equal,status=got.returncode))
            if not equal: raise SystemExit('stdin mismatch')
report=dict(schema_version=1,scope='Same executable output/stdout/stderr/status under off, auto and strict; not an independent security audit or separate C comparison.',platform=platform.platform(),libc=platform.libc_ver(),binary_sha256=hashlib.sha256(BIN.read_bytes()).hexdigest(),database_sha256=hashlib.sha256((ROOT/'data/magic.mgc').read_bytes()).hexdigest(),actual_strict=actual,fixture_count=len(fixtures),comparisons=len(rows),equal=sum(r['equal'] for r in rows),operand_comparisons=sum(len(r['fixtures']) for r in rows),seconds=time.monotonic()-start,cases=rows)
(ROOT/'reports/sandbox.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({k:v for k,v in report.items() if k!='cases'},indent=2))
