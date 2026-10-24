#!/usr/bin/env python3
import importlib.util,json,pathlib,subprocess
ROOT=pathlib.Path(__file__).resolve().parents[1];spec=importlib.util.spec_from_file_location('differential',ROOT/'tools/differential.py');d=importlib.util.module_from_spec(spec);spec.loader.exec_module(d)
rows=[]
for p in sorted((d.UP/'tests').glob('*.testfile')):
 base=p.name.removesuffix('.testfile');sources=sorted(p.parent.glob(base+'*.magic'));database=':'.join(str(x)for x in sources)if sources else str(d.DB);flags=512
 flagfile=p.parent/(base+'.flags')
 if flagfile.exists()and'k'in flagfile.read_text():flags|=32
 result=subprocess.run([str(ROOT/'target/release/rfile-probe'),database,f'{flags:x}','path'],input=(str(p)+'\n').encode(),env=d.ENV,capture_output=True,timeout=20)
 if result.returncode:raise RuntimeError(result.stderr.decode(errors='replace'))
 actual=json.loads(result.stdout);expected=p.with_suffix('.result').read_bytes();expected=expected[:-1]if expected.endswith(b'\n')else expected;out=bytes.fromhex(actual['hex'])if'hex'in actual else None
 rows.append({'fixture':p.name,'flags':flags,'equal':out==expected,**({}if out==expected else{'expected':expected.decode('utf8','backslashreplace'),'actual':out.decode('utf8','backslashreplace')if out is not None else actual})})
report={'total':len(rows),'equal':sum(x['equal']for x in rows),'cases':rows};(ROOT/'reports/fixtures.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({k:v for k,v in report.items()if k!='cases'}))
for r in rows:
 if not r['equal']:print(json.dumps(r))
