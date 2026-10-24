#!/usr/bin/env python3
"""Pinned libmagic oracle vs independent Rust path/buffer API. Never used by detection."""
import argparse, ctypes, hashlib, json, locale, os, pathlib, subprocess, time
ROOT=pathlib.Path(__file__).resolve().parents[1]
UP=ROOT/'upstream'; DB=UP/'magic/magic.mgc'
ENV=dict(os.environ,LC_ALL='C',TZ='UTC',MAGIC=str(DB));ENV.pop('POSIXLY_CORRECT',None)
locale.setlocale(locale.LC_ALL,'C');os.environ['TZ']='UTC';time.tzset()

def library():
 lib=ctypes.CDLL(str(UP/'src/.libs/libmagic.so'))
 lib.magic_open.argtypes=[ctypes.c_int];lib.magic_open.restype=ctypes.c_void_p
 for f in ('magic_load','magic_file'):
  getattr(lib,f).argtypes=[ctypes.c_void_p,ctypes.c_char_p]
 lib.magic_file.restype=ctypes.c_char_p
 lib.magic_buffer.argtypes=[ctypes.c_void_p,ctypes.c_void_p,ctypes.c_size_t];lib.magic_buffer.restype=ctypes.c_char_p
 lib.magic_descriptor.argtypes=[ctypes.c_void_p,ctypes.c_int];lib.magic_descriptor.restype=ctypes.c_char_p
 lib.magic_close.argtypes=[ctypes.c_void_p];lib.magic_error.argtypes=[ctypes.c_void_p];lib.magic_error.restype=ctypes.c_char_p
 return lib

def corpus(limit):
 files=list(sorted((UP/'tests').glob('*.testfile')))
 tracked=subprocess.check_output(['git','-C',str(UP),'ls-files'],text=True).splitlines()
 files.extend(UP/x for x in tracked if (UP/x).is_file() and not x.endswith(('.result','.testfile')))
 for base,patterns,count in [('/usr/bin',['*'],700),('/usr/share/icons',['**/*.png','**/*.svg'],1100),('/usr/share/fonts',['**/*.ttf','**/*.otf'],100),('/usr/share/mime',['**/*.xml'],400)]:
  group=[]
  for pattern in patterns:group.extend(sorted(pathlib.Path(base).glob(pattern)))
  files.extend(p for p in group[:count] if p.is_file() and not p.is_symlink())
 unique=[p for p in dict.fromkeys(files) if os.access(p,os.R_OK)];return unique[:limit]

def run_group(lib,files,db,flags,mode,extra=0):
 requested_flags=flags;flags|=extra
 proc=subprocess.run([str(ROOT/'target/release/rfile-probe'),str(db),f'{flags:x}',mode],input=''.join(str(p)+'\n' for p in files).encode(),env=ENV,capture_output=True,timeout=180)
 if proc.returncode:raise RuntimeError(proc.stderr.decode(errors='replace'))
 rows=[json.loads(s) for s in proc.stdout.splitlines()]
 if len(rows)!=len(files):raise RuntimeError(f'probe records {len(rows)} != {len(files)}')
 h=lib.magic_open(flags)
 if lib.magic_load(h,str(db).encode())!=0:raise RuntimeError(lib.magic_error(h))
 comparisons=[]
 for path,row in zip(files,rows):
  if mode=='buffer':
   with path.open('rb')as source:data=source.read(7*1024*1024)
   result=lib.magic_buffer(h,data,len(data))
  elif mode=='descriptor':
   with path.open('rb')as descriptor:result=lib.magic_descriptor(h,descriptor.fileno())
  else:result=lib.magic_file(h,os.fsencode(path))
  expected=result if result is not None else None
  actual=bytes.fromhex(row['hex']) if 'hex'in row else None
  equal=actual is not None and expected is not None and actual==expected
  comparisons.append({'path':str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path),'mode':mode,'flags':requested_flags,'effective_flags':flags,'database':str(db.relative_to(ROOT)),'equal':equal,**({} if equal else {'reference':expected.decode('utf-8','backslashreplace') if expected is not None else None,'rust':actual.decode('utf-8','backslashreplace') if actual is not None else None,'rust_error':row.get('error'),'reference_error':lib.magic_error(h).decode(errors='replace') if expected is None and lib.magic_error(h) else None})})
 lib.magic_close(h);return comparisons

def main():
 ap=argparse.ArgumentParser();ap.add_argument('--limit',type=int,default=3000);ap.add_argument('--fixtures-only',action='store_true');args=ap.parse_args()
 lib=library();files=list(sorted((UP/'tests').glob('*.testfile'))) if args.fixtures_only else corpus(args.limit)
 manifest=[{'path':str(p),'size':p.stat().st_size,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}for p in files]
 (ROOT/'reports/corpus-manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
 comparisons=[];started=time.perf_counter();summaries=[]
 for mode in ['buffer','path']:
  for flags in [0,0x410,0x10,0x400]:
   ordinary=[];special={}
   for p in files:
    if p.parent==UP/'tests':
     extra=sorted(p.parent.glob(p.name.removesuffix('.testfile')+'*.magic'))
     if extra:special.setdefault((':'.join(str(x) for x in extra), 32 if p.with_suffix('.flags').exists() and 'k' in p.with_suffix('.flags').read_text() else 0),[]).append(p);continue
    ordinary.append(p)
   comparisons.extend(run_group(lib,ordinary,DB,flags,mode))
   for (names,extra),group in special.items():
    # List overrides are exercised by CLI separately; merge via source for the special multiple case.
    paths=names.split(':')
    comparisons.extend(run_group(lib,group,pathlib.Path(names),flags,mode,extra))
   selected=[r for r in comparisons if r['mode']==mode and r['flags']==flags];summaries.append({'mode':mode,'flags':flags,'total':len(selected),'equal':sum(r['equal']for r in selected),'errors':sum(r.get('rust_error')is not None for r in selected)})
   print(json.dumps(summaries[-1]),flush=True)
 report={'upstream_commit':subprocess.check_output(['git','-C',str(UP),'rev-parse','HEAD'],text=True).strip(),'database_sha256':hashlib.sha256(DB.read_bytes()).hexdigest(),'environment':{'LC_ALL':'C','TZ':'UTC','POSIXLY_CORRECT':None},'elapsed_seconds':time.perf_counter()-started,'summary':summaries,'comparisons':comparisons}
 (ROOT/'reports/differential.json').write_text(json.dumps(report,indent=2)+'\n')
 print(f"wrote {len(comparisons)} comparisons in {report['elapsed_seconds']:.2f}s")
if __name__=='__main__':main()
