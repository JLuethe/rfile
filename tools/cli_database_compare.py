#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Database CLI actions: compare raw channels/status and compiled artifacts.

Only the executable diagnostic prefix file: is normalized to rfile:. All other
bytes, including paths and line numbers, remain exact. No mismatch allowlist.
"""
import argparse,hashlib,json,os,subprocess,tempfile
from pathlib import Path
from reference import ROOT,UP,manifest
from abi_diagnostic_compare import CASES

def main():
 ap=argparse.ArgumentParser();ap.add_argument('--binary',default='target/debug/rfile');ap.add_argument('--sandbox-matrix',action='store_true');args=ap.parse_args()
 binary=(ROOT/args.binary).resolve();env=dict(os.environ,LC_ALL='C',TZ='UTC')
 for key in ['MAGIC','POSIXLY_CORRECT','LD_PRELOAD']:env.pop(key,None)
 rows=[]
 with tempfile.TemporaryDirectory(prefix='rfile-cli-db-')as td:
  tmp=Path(td)
  def run(label,paths,options,compiled=None):
   for action in ['c','C','l']:
    observed=[]
    for program,mode in [(UP/'src/file','-S'),(binary,'-S')]+([(binary,'--sandbox=auto'),(binary,'--sandbox=strict')]if args.sandbox_matrix else []):
     for p in tmp.glob('*.mgc'):p.unlink()
     if compiled is not None:(tmp/'probe.mgc').write_bytes(compiled)
     command=[str(program),mode,'-'+action,'-m',paths,*options]
     p=subprocess.run(command,cwd=tmp,env=env,capture_output=True,timeout=20)
     # Only a diagnostic program prefix at the beginning of a line is mapped.
     stderr=b'\n'.join(b'rfile: '+line[6:] if line.startswith(b'file: ')else line for line in p.stderr.split(b'\n'))
     observed.append(dict(status=p.returncode,stdout_hex=p.stdout.hex(),stderr_hex=stderr.hex(),artifacts={p.name:hashlib.sha256(p.read_bytes()).hexdigest()for p in tmp.glob('*.mgc')}))
    row=dict(label=label,action=action,paths=paths,options=options,exact=all(observed[0]==v for v in observed[1:]))
    if not row['exact']:row.update(reference=observed[0],rust=observed[1:])
    rows.append(row)
  for name,data in CASES.items():
   (tmp/'probe').unlink(missing_ok=True)
   if data is not None:(tmp/'probe').write_bytes(data)
   for options in [[],['-P','magwarn=0'],['-i']]:run(name,'probe',options)
  (tmp/'probe').write_bytes(b'0 string ABC first\n>3 byte 1 child\n!:mime text/example\n')
  (tmp/'other').write_bytes(b'0 string XYZ second\n')
  (tmp/'bad').write_bytes(b'0 bogus x invalid\n')
  for paths in ['probe:other','missing:probe','probe:missing','bad:probe','probe:bad',':probe','probe::other','missing','']:
   run('paths',paths,[])
  for content in [b'',b'wrong header', (ROOT/'data/magic.mgc').read_bytes()]:run('compiled-preference','probe',[],content)
  directory=tmp/'rules';directory.mkdir()
  (directory/'a').write_bytes(b'0 string ABC first\n')
  (directory/'b').write_bytes(b'0 string XYZ second\n')
  (directory/'.hidden').write_bytes(b'broken\n')
  run('directory-valid','rules',[])
  (directory/'b').write_bytes(b'0 bogus x invalid\n')
  run('directory-invalid','rules',[])
  (tmp/'probe').write_bytes(b'0 bogus x bad\n'*70)
  run('warning-cap','probe',[])
  # A deliberate safe IO boundary, not native equivalence (C may block here).
  fifo=tmp/'fifo';os.mkfifo(fifo)
  bounded=subprocess.run([str(binary),'-S','-c','-m','fifo'],cwd=tmp,env=env,capture_output=True,timeout=5)
  if bounded.returncode!=1 or b'database must be a regular file'not in bounded.stderr:raise SystemExit('FIFO database was not safely rejected')
 report=dict(schema_version=1,sandbox_modes=['off','auto','strict']if args.sandbox_matrix else ['off'],reference=manifest(),binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),fifo_database_rejected=True,scope='Explicit database actions only; normalize diagnostic executable prefix, compare all remaining bytes and status/artifact hashes. Default discovery, DEBUG and other platforms remain separate.',total=len(rows),exact=sum(r['exact']for r in rows),cases=rows)
 (ROOT/'reports/cli-database.json').write_text(json.dumps(report,indent=2)+'\n')
 print({k:report[k]for k in ['total','exact']})
 if report['total']!=report['exact']:raise SystemExit('Database CLI mismatch: reports/cli-database.json')
if __name__=='__main__':main()
