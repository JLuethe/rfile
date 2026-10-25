#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Research/acceptance corpus for native parser diagnostic events and persistent state."""
import hashlib,json,os,subprocess,sys,tempfile
from pathlib import Path
from reference import ROOT,UP,manifest
CASES={
'missing':None, 'empty':b'', 'type':b'0 bogus x bad\n', 'offset':b'bad byte 0 bad\n',
'relative':b'&0 byte 0 bad\n','missing-test':b'0 byte\n',
'two-errors':b'0 bogus x first\nbad byte 0 second\n',
'valid-between':b'0 bogus x first\n0 string ABC middle\nbad byte 0 last\n',
'no-final-newline':b'#comment\n\n0 bogus x last',
'crlf':b'0 bogus x bad\r\n', 'nul':b'bad byte 0 bad\0ignored\n',
'unsigned-type':b'0 ubogus x bad\n', 'raw-path':b'0 bogus x bad\n',
'unknown-annotation':b'!:bogus x\n','orphan-mime':b'!:mime text/plain\n',
'duplicate-mime':b'0 string ABC sample\n!:mime text/plain\n!:mime application/x-other\n',
'strength':b'0 byte 1 sample\n!:strength z4\n',
'numeric':b'0 long nonsense bad\n', 'orphan-continuation':b'>0 byte 1 child\n',
'skipped-continuation':b'0 byte 1 root\n>>0 byte 2 child\n',
'continued-name':b'0 byte 1 root\n>0 name named\n',
'stale-descendant':b'0 string ABC root\n>0 string ABC one\n>>0 string ABC two\n>>>0 string ABC three\n>0 string XYZ miss\n>>>>0 string ABC stale\n>0 string ABC last\n',
}
def main():
 library=(ROOT/'target/debug/librfile_magic.so').resolve();rows=[]
 env=dict(os.environ,LC_ALL='C',TZ='UTC');env.pop('MAGIC',None);env.pop('LD_PRELOAD',None)
 with tempfile.TemporaryDirectory(prefix='rfile-source-events-')as tmp:
  tmp=Path(tmp)
  for label,data in CASES.items():
   name=os.fsdecode(b'probe-\xff')if label=='raw-path'else'probe'
   (tmp/name).unlink(missing_ok=True)
   if data is not None:(tmp/name).write_bytes(data)
   for action in ['load','check','list','compile']:
    for cap in [0,1,2,3,64]:
     results=[]
     for impl,lib in [('native',UP/'src/.libs/libmagic.so'),('rust',library)]:
      for p in tmp.glob('*.mgc'):p.unlink()
      result=tmp/(impl+'.json');result.unlink(missing_ok=True)
      r=subprocess.run([sys.executable,str(ROOT/'tools/abi_compare.py'),'--worker',str(lib),f'diagnostic:{action}:{cap}:{name}',str(result)],env=env,cwd=tmp,capture_output=True,timeout=20)
      artifacts={p.name:hashlib.sha256(p.read_bytes()).hexdigest()for p in tmp.glob('*.mgc')}
      results.append(dict(status=r.returncode,stdout_hex=r.stdout.hex(),stderr_hex=r.stderr.hex(),state=json.loads(result.read_text())if result.exists()else None,artifacts=artifacts))
     row=dict(fixture=label,action=action,cap=cap,exact=results[0]==results[1])
     if not row['exact']:row.update(reference=results[0],rust=results[1])
     rows.append(row)
 report=dict(schema_version=1,scope='Source diagnostic event research: mismatches remain open unless individually resolved. Successful native parse with a warning is distinct from Rust rejection.',reference=manifest(),library_sha256=hashlib.sha256(library.read_bytes()).hexdigest(),fixtures={k:dict(hex=v.hex(),sha256=hashlib.sha256(v).hexdigest())if v is not None else dict(missing=True)for k,v in CASES.items()},total=len(rows),exact=sum(r['exact']for r in rows),cases=rows)
 (ROOT/'reports/abi-diagnostics.json').write_text(json.dumps(report,indent=2)+'\n');print(dict(total=report['total'],exact=report['exact'],differences=sorted({r['fixture']for r in rows if not r['exact']})))
 if report['exact']!=report['total']:raise SystemExit('Native diagnostic mismatch: see reports/abi-diagnostics.json')
if __name__=='__main__':main()
