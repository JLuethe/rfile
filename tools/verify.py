#!/usr/bin/env python3
"""Reproducible local verification. The C checkout is a test oracle only."""
import argparse,hashlib,json,os,pathlib,shutil,subprocess
from verification_run import VerificationRun
ROOT=pathlib.Path(__file__).resolve().parents[1]
CARGO=os.environ.get('CARGO',shutil.which('cargo')or str(pathlib.Path.home()/'.cargo/bin/cargo'))

def main():
 ap=argparse.ArgumentParser(description=__doc__)
 ap.add_argument('--all-features',action='store_true')
 ap.add_argument('--benchmark',action='store_true')
 ap.add_argument('--no-oracle',action='store_true')
 ap.add_argument('--require-sandbox',action='store_true',help='Fail unless CLI and kernel sandbox tests execute (requires --all-features)')
 args=ap.parse_args()
 if args.require_sandbox and not args.all_features:ap.error('--require-sandbox requires --all-features')
 features=['--all-features']if args.all_features else []
 env=dict(os.environ,LC_ALL='C',TZ='UTC',RFILE_REQUIRE_FIXTURES='0'if args.no_oracle else'1',RFILE_REQUIRE_SANDBOX='1'if args.require_sandbox else'0')
 env.pop('POSIXLY_CORRECT',None)
 with VerificationRun(ROOT,'all-features'if args.all_features else'default',dict(oracle=not args.no_oracle,sandbox=args.require_sandbox))as evidence:
  evidence.identify()
  if not args.no_oracle:
   from reference import UP,manifest
   evidence.report['oracle']=manifest()
   env['RFILE_REFERENCE']=str(UP)
   evidence.save()
  def run(name,command):evidence.run(name,command,env)
  run('format',[CARGO,'fmt','--all','--','--check'])
  run('clippy',[CARGO,'clippy','--workspace','--all-targets','--locked']+features+['--','-D','warnings'])
  run('cargo-test',[CARGO,'test','--workspace','--locked']+features+['--','--nocapture'])
  run('cargo-release-test',[CARGO,'test','--workspace','--release','--locked']+features+['--','--nocapture'])
  run('cargo-build',[CARGO,'build','--release','--locked']+features)
  run('ffi-build',[CARGO,'build','-p','rfile-ffi','--locked']+features)
  run('ffi-rust-clients',['python3','tools/abi_clients.py','--rust-only'])
  run('unsafe-inventory',['python3','tools/unsafe_inventory.py'])
  run('database-rebuild',['python3','tools/rebuild_database.py'])
  run('fuzz-smoke',[CARGO,'test','--release','--locked','--test','hardening']+features+['--','--ignored','--nocapture'])
  run('fuzz-format',[CARGO,'fmt','--manifest-path','fuzz/Cargo.toml','--','--check'])
  run('fuzz-build',[CARGO,'check','--manifest-path','fuzz/Cargo.toml','--locked','--bins'])
  if not args.no_oracle:
   expected=json.loads((ROOT/'data/UPSTREAM.json').read_text())['commit'];actual=subprocess.check_output(['git','-C',str(UP),'rev-parse','HEAD'],text=True).strip()
   if actual!=expected:raise SystemExit(f'oracle revision mismatch: {actual} != {expected}')
   run('ffi-clients',['python3','tools/abi_clients.py'])
   run('ffi-state',['python3','tools/abi_compare.py'])
   run('ffi-diagnostics',['python3','tools/abi_diagnostic_compare.py'])
   run('upstream-check',['make','-C',str(UP),'check'])
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
   if semantics['unexpected'] or semantics['compiler_errors']:raise SystemExit('custom rule differential mismatch')
   run('elf-differential',['python3','tools/elf_compare.py'])
   if args.all_features:run('compression-differential',['python3','tools/compression_compare.py'])
   run('cdf-differential',['python3','tools/cdf_compare.py'])
   run('locale-differential',['python3','tools/locale_compare.py'])
   run('float-differential',['python3','tools/float_compare.py'])
   run('time-differential',['python3','tools/time_compare.py'])
   run('component-differential',['python3','tools/component_compare.py'])
   run('cli-differential',['python3','tools/cli_compare.py'])
   run('cli-database-differential',['python3','tools/cli_database_compare.py','--binary','target/release/rfile']+(['--sandbox-matrix']if args.all_features else []))
   cli=json.loads((ROOT/'reports/cli.json').read_text())
   if any(not r['equal']and not r.get('intentional_difference')for r in cli['cases']):raise SystemExit('unexpected CLI discrepancy')
   if args.all_features:run('sandbox-differential',['python3','tools/sandbox_compare.py'])
   if args.benchmark:run('benchmark',['python3','tools/benchmark.py'])
   if args.benchmark and not json.loads((ROOT/'reports/benchmark.json').read_text())['same_output_checksum']:raise SystemExit('benchmark output checksums differ')
  # Linux loader evidence distinguishes the production program from the test oracle.
  linked=subprocess.run(['ldd','target/release/rfile'],cwd=ROOT,capture_output=True,text=True)
  (ROOT/'reports/runtime-linkage.txt').write_text(linked.stdout+linked.stderr)
  if linked.returncode:raise SystemExit('Runtime linkage inspection failed')
  if 'libmagic' in linked.stdout:raise SystemExit('production executable unexpectedly links libmagic')
  smoke=[]
  for line in (evidence.directory/'logs/fuzz-smoke.log').read_text().splitlines():
   if line.startswith('{'):smoke.append(json.loads(line))
  if not args.no_oracle and (len(smoke)!=1 or smoke[0].get('fixture_truncations',0)==0):
   raise SystemExit('Required mutation fixture coverage is absent')
  if args.require_sandbox:
   for stage in ['cargo-test','cargo-release-test']:
    executed={row['component']for row in evidence.report['coverage']if row['step']==stage and row['status']=='executed'}
    if not {'sandbox_cli','sandbox_kernel'}<=executed:
     raise SystemExit(f'Required sandbox coverage is absent in {stage}')
  evidence.archive(ROOT/'reports/runtime-linkage.txt')
  evidence.report.update(cargo=subprocess.check_output([CARGO,'--version'],text=True).strip(),rustc=subprocess.check_output([str(pathlib.Path(CARGO).with_name('rustc')),'--version'],text=True).strip(),binary_sha256=hashlib.sha256((ROOT/'target/release/rfile').read_bytes()).hexdigest(),database_sha256=hashlib.sha256((ROOT/'data/magic.mgc').read_bytes()).hexdigest(),mutation_smoke=smoke,scope='Linux supported profile; coverage skips recorded separately; not sustained security qualification')
 print(json.dumps({k:v for k,v in evidence.report.items()if k not in ('steps','artifacts')},indent=2))

if __name__=='__main__':main()
