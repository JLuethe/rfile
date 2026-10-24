#!/usr/bin/env python3
import hashlib,json,os,pathlib,platform,statistics,subprocess,tempfile
ROOT=pathlib.Path(__file__).resolve().parents[1];UP=ROOT/'upstream';CARGO=os.environ.get('CARGO',str(pathlib.Path.home()/'.cargo/bin/cargo'))
ENV=dict(os.environ,LC_ALL='C',TZ='UTC');ENV.pop('POSIXLY_CORRECT',None)
subprocess.run([CARGO,'build','--release','--locked','--example','benchmark'],cwd=ROOT,check=True)
subprocess.run(['cc','-O2','-I'+str(UP/'src'),str(ROOT/'tools/oracle_bench.c'),'-L'+str(UP/'src/.libs'),'-Wl,-rpath,'+str(UP/'src/.libs'),'-lmagic','-o',str(ROOT/'reports/oracle-bench')],check=True)
paths=list(sorted((UP/'tests').glob('*.testfile')))
paths.extend([UP/'README.md',UP/'src/file.c',UP/'src/apprentice.c',pathlib.Path('/usr/bin/ls'),pathlib.Path('/usr/bin/bash')])
with tempfile.TemporaryDirectory(prefix='rfile-bench-')as tmp:
 tmp=pathlib.Path(tmp);(tmp/'text').write_bytes(b'hello, world\n'*500_000);(tmp/'unknown').write_bytes(bytes(range(256))*4096);paths.extend([tmp/'text',tmp/'unknown'])
 manifest=[{'path':str(p)if p.is_relative_to(UP)or str(p).startswith('/usr/')else'<GENERATED>/'+p.name,'bytes':min(p.stat().st_size,7*1024*1024),'sha256':hashlib.sha256(p.read_bytes()[:7*1024*1024]).hexdigest()}for p in paths]
 listing=tmp/'list';listing.write_text(''.join(str(p)+'\n'for p in paths))
 rust=json.loads(subprocess.check_output([str(ROOT/'target/release/examples/benchmark'),str(listing)],env=ENV,timeout=120))
 c=json.loads(subprocess.check_output([str(ROOT/'reports/oracle-bench'),str(listing),str(UP/'magic/magic.mgc')],env=ENV,timeout=120))
 for result in [rust,c]:
  ordered=sorted(result['warm_pass_seconds']);result['median_seconds']=statistics.median(ordered);result['p95_seconds']=ordered[-1];result['calls_per_second']=result['calls_per_pass']/result['median_seconds']
 report={'platform':platform.platform(),'locale':'C','timezone':'UTC','profile':'buffer API, default flags, same pinned database; 10 warm passes of 3 corpus sweeps; one cold process load per engine','corpus':manifest,'rust':rust,'reference':c,'same_output_checksum':rust['checksum']==c['checksum'],'warm_rust_to_c_ratio':rust['median_seconds']/c['median_seconds'],'notes':['RSS includes each native process and loaded corpus; one-process high-water mark.','p95 uses nearest-rank on 10 samples; do not infer a stable tail distribution.','Cold load has one sample; Rust validates/owns records while C maps data and compiles regex lazily.']}
 (ROOT/'reports/benchmark.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({k:v for k,v in report.items()if k!='corpus'},indent=2))
