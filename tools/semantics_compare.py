#!/usr/bin/env python3
"""Small custom magic programs, including truncation and numeric/string boundaries."""
import ctypes,importlib.util,json,pathlib,subprocess,tempfile,struct,sys
from oracle_boundaries import filetime_has_uninitialized_seconds
ROOT=pathlib.Path(__file__).resolve().parents[1];spec=importlib.util.spec_from_file_location('d',ROOT/'tools/differential.py');d=importlib.util.module_from_spec(spec);spec.loader.exec_module(d);lib=d.library()
cases=[]
for t in ['byte','ubyte','short','ushort','beshort','leshort','long','ulong','belong','lelong','melong','quad','uquad','bequad','lequad']:
 for rel in ['x','<0','>0','!0','=0','&1','^3']:
  cases.append((t+'-'+rel,f'0 {t} {rel} value=%lld\n'if'quad'in t else f'0 {t} {rel} value=%d\n'))
for t in ['float','befloat','lefloat','double','bedouble','ledouble']:
 cases.append((t,f'0 {t} x value=%g\n'))
for t in ['date','ldate','bedate','ledate','qdate','qldate','leqdate','beqdate','qwdate','leqwdate','beqwdate','lemsdosdate','lemsdostime']:
 cases.append((t,f'0 {t} x value=%s\n'))
for t in ['string','pstring','pstring/H','pstring/h','pstring/J','lestring16','bestring16','search/8','search/8/c','regex/b','octal']:
 cases.append((t,f'0 {t} x value=%s\n'if t!='octal'else'0 octal 012 value=%s\n'))
cases.extend([('regex-longest','0 regex/b a|ab value=%s\n'),('parent','0 string AB parent\n>&0 byte x child=%d\n'),('indirect','0 string AB root\n>(2.b) string XY child\n'),('negative','-0 offset x size=%llu\n'),('mask','0 byte&0x80 x value=%d\n'),('mask-div','0 short/2 x value=%d\n'),('quad-past-eof','40 lequad x value=%lld\n'),('quad-past-eof-negation','40 lequad !0 value=%lld\n'),('quad-child','0 lequad x root=%lld\n>&0 byte x child=%d\n'),('neg-short-mask','0 beshort&0xf0 !0 value=%d\n')])
rows=[];flags=0xffb000
with tempfile.TemporaryDirectory(prefix='rfile-semantics-')as temp:
 temp=pathlib.Path(temp)
 inputs=[]
 for base in [b'\0'*32,b'\xff'*32,bytes(range(32)),b'AB\x04\0XY'+b'\0'*26,b'\x03abc\n'+b' '*27,b'ab\n'+b' '*29]:
  for n in [0,1,2,3,4,7,8,15,16,32]:inputs.append(base[:n])
 inputs.extend(struct.pack('<q',x)for x in [0,864000000000,30*864000000000,31*864000000000,59*864000000000,-1,-864000000000,-10**18,10**18])
 inputs.extend(struct.pack('<d',x)for x in [float('nan'),float('inf'),-0.0,1.0/3,1e-6,1e30])
 files=[]
 for i,b in enumerate(inputs):p=temp/f'input-{i}';p.write_bytes(b);files.append(p)
 for index,(label,source)in enumerate(cases):
  magic=temp/f'case-{index}.magic';magic.write_text(source)
  c=subprocess.run([str(d.UP/'src/file'),'-C','-m',str(magic)],cwd=temp,env=d.ENV,capture_output=True)
  if c.returncode:rows.append({'case':label,'source':source,'compile_error':c.stderr.decode(errors='replace')});continue
  database=temp/(magic.name+'.mgc');h=lib.magic_open(flags);lib.magic_load(h,str(database).encode())
  proc=subprocess.run([str(ROOT/'target/release/rfile-probe'),str(database),f'{flags:x}','buffer'],input=''.join(str(p)+'\n'for p in files).encode(),env=d.ENV,capture_output=True,timeout=30)
  if proc.returncode:raise RuntimeError(proc.stderr.decode(errors='replace'))
  actual=[json.loads(x)for x in proc.stdout.splitlines()]
  for b,row in zip(inputs,actual):
   ref=lib.magic_buffer(h,b,len(b));out=bytes.fromhex(row['hex'])if'hex'in row else None;equal=ref is not None and out is not None and ref==out
   undefined=False
   if label in ('qwdate','leqwdate','beqwdate'):
    order='big'if label.startswith('be')else'little'if label.startswith('le')else sys.byteorder
    undefined=filetime_has_uninitialized_seconds(int.from_bytes(b[:8].ljust(8,b'\0'),order))
   if undefined:
    rows.append(dict(case=label,source=source,input_hex=b.hex(),equal=False,raw_equal=equal,
      disposition='native-filetime-uninitialized-seconds',oracle_defined=False,
      reference_hex=ref.hex()if ref is not None else None,rust_hex=out.hex()if out is not None else None,
      rust_error=row.get('error'),safe_result_validated=out==b'value=*Invalid datetime*'))
   else:
    rows.append({'case':label,'source':source,'input_hex':b.hex(),'equal':equal,**({}if equal else{'reference':ref.decode('utf8','backslashreplace')if ref is not None else None,'rust':out.decode('utf8','backslashreplace')if out is not None else row})})
  lib.magic_close(h)
report={'schema_version':2,'flags':flags,'total':sum('equal'in r for r in rows),'equal':sum(r.get('equal',False)for r in rows),'oracle_undefined':sum(r.get('oracle_defined')is False for r in rows),'oracle_undefined_validated':sum(r.get('safe_result_validated',False)for r in rows),'compiler_errors':sum('compile_error'in r for r in rows),'cases':rows};report['unexpected']=report['total']-report['equal']-report['oracle_undefined_validated'];(ROOT/'reports/semantics.json').write_text(json.dumps(report,indent=2)+'\n');print({k:v for k,v in report.items()if k!='cases'})
for r in [r for r in rows if r.get('equal')is False][:30]:print(json.dumps(r))
if report['unexpected'] or report['compiler_errors']:raise SystemExit('Unexpected custom rule mismatch: inspect reports/semantics.json')
