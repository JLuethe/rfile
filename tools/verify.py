#!/usr/bin/env python3
"""Reproducible local verification. The C checkout is a test oracle only."""
import argparse,hashlib,json,os,pathlib,re,shutil,subprocess,time
ROOT=pathlib.Path(__file__).resolve().parents[1]
CARGO=os.environ.get('CARGO',shutil.which('cargo')or str(pathlib.Path.home()/'.cargo/bin/cargo'))
ENV=dict(os.environ,LC_ALL='C',TZ='UTC');ENV.pop('POSIXLY_CORRECT',None)
ap=argparse.ArgumentParser();ap.add_argument('--all-features',action='store_true');ap.add_argument('--benchmark',action='store_true');ap.add_argument('--no-oracle',action='store_true');args=ap.parse_args()
features=["--all-features"]if args.all_features else []
steps=[];started=time.perf_counter()
def run(name,command):
 print(f'Running {name}',flush=True);t=time.perf_counter()
 with (ROOT/(name+'.log')).open('wb')as log:r=subprocess.run(command,cwd=ROOT,env=ENV,stdout=log,stderr=subprocess.STDOUT)
 steps.append({'name':name,'status':r.returncode,'seconds':time.perf_counter()-t,'command':command})
 if r.returncode:raise SystemExit(f'{name} failed; see {name}.log')
run('format',[CARGO,'fmt','--all','--','--check'])
run('clippy',[CARGO,'clippy','--all-targets','--locked']+features+['--','-D','warnings'])
run('cargo-test',[CARGO,'test','--locked']+features)
run('cargo-release-test',[CARGO,'test','--release','--locked']+features)
run('cargo-build',[CARGO,'build','--release','--locked']+features)
run('database-rebuild',['python3','tools/rebuild_database.py'])
run('fuzz-smoke',[CARGO,'test','--release','--locked','--test','hardening']+features+['--','--ignored','--nocapture'])
run('fuzz-format',[CARGO,'fmt','--manifest-path','fuzz/Cargo.toml','--','--check'])
run('fuzz-build',[CARGO,'check','--manifest-path','fuzz/Cargo.toml','--locked','--bins'])
if not args.no_oracle:
 expected=json.loads((ROOT/'data/UPSTREAM.json').read_text())['commit'];actual=subprocess.check_output(['git','-C',str(ROOT/'upstream'),'rev-parse','HEAD'],text=True).strip()
 if actual!=expected:raise SystemExit(f'oracle revision mismatch: {actual} != {expected}')
 run('upstream-check',['make','-C','upstream','check'])
 run('compiler',['python3','tools/compiler_compare.py'])
 if not json.loads((ROOT/'reports/compiler.json').read_text())['raw_equal']:raise SystemExit('compiler wire bytes differ')
 run('fixtures',['python3','tools/fixture_compare.py'])
 fixtures=json.loads((ROOT/'reports/fixtures.json').read_text())
 if fixtures['total']!=fixtures['equal']:raise SystemExit('upstream fixture mismatch')
 run('differential-broad',['python3','tools/differential.py','--limit','3000'])
 run('extended',['python3','tools/extended_compare.py'])
 for report in ['differential','extended']:
  data=json.loads((ROOT/('reports/'+report+'.json')).read_text())
  if not all(x['equal']for x in data['comparisons']):raise SystemExit(report+' mismatch; inspect reports')
 run('semantics-differential',['python3','tools/semantics_compare.py'])
 run('regex-differential',['python3','tools/regex_compare.py'])
 semantics=json.loads((ROOT/'reports/semantics.json').read_text())
 if semantics['total']!=semantics['equal'] or semantics['compiler_errors']:raise SystemExit('custom rule differential mismatch')
 run('elf-differential',['python3','tools/elf_compare.py'])
 if args.all_features:run('compression-differential',['python3','tools/compression_compare.py'])
 run('cdf-differential',['python3','tools/cdf_compare.py'])
 run('locale-differential',['python3','tools/locale_compare.py'])
 run('float-differential',['python3','tools/float_compare.py'])
 run('time-differential',['python3','tools/time_compare.py'])
 run('component-differential',['python3','tools/component_compare.py'])
 run('cli-differential',['python3','tools/cli_compare.py'])
 cli=json.loads((ROOT/'reports/cli.json').read_text())
 if any(not r['equal']and not r.get('intentional_difference')for r in cli['cases']):raise SystemExit('unexpected CLI discrepancy')
 if args.all_features:run('sandbox-differential',['python3','tools/sandbox_compare.py'])
 if args.benchmark:run('benchmark',['python3','tools/benchmark.py'])
 if args.benchmark and not json.loads((ROOT/'reports/benchmark.json').read_text())['same_output_checksum']:raise SystemExit('benchmark output checksums differ')
# Linux loader evidence distinguishes the production program from the test oracle.
linked=subprocess.run(['ldd','target/release/rfile'],cwd=ROOT,capture_output=True,text=True)
(ROOT/'reports/runtime-linkage.txt').write_text(linked.stdout+linked.stderr)
if 'libmagic' in linked.stdout:raise SystemExit('production executable unexpectedly links libmagic')
smoke=[]
for line in (ROOT/'fuzz-smoke.log').read_text().splitlines():
 if line.startswith('{'):smoke.append(json.loads(line))
report={'features':'all-features'if args.all_features else 'default','status':'passed','elapsed_seconds':time.perf_counter()-started,'steps':steps,'cargo':subprocess.check_output([CARGO,'--version'],text=True).strip(),'rustc':subprocess.check_output([str(pathlib.Path(CARGO).with_name('rustc')),'--version'],text=True).strip(),'binary_sha256':hashlib.sha256((ROOT/'target/release/rfile').read_bytes()).hexdigest(),'database_sha256':hashlib.sha256((ROOT/'data/magic.mgc').read_bytes()).hexdigest(),'mutation_smoke':smoke,'scope':'Linux supported profile; known CLI deviations allowed by explicit case labels; remaining release gaps documented'}
(ROOT/('reports/verification-extended.json'if args.all_features else 'reports/verification.json')).write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({k:v for k,v in report.items()if k!='steps'},indent=2))
