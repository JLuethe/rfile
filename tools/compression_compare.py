#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Compare codec classification with the pinned oracle, preserving raw results.

Independent encoder vectors cover lzip when this reference has no native codec.
No production helper is used. The oracle's .Z/ZIP paths do invoke test helpers.
"""
import hashlib,json,os,subprocess,argparse,shutil
from pathlib import Path
from differential import library,run_group
from reference import UP
ROOT=Path(__file__).resolve().parents[1]
ENV=dict(os.environ,LC_ALL='C',TZ='UTC');ENV.pop('POSIXLY_CORRECT',None)
def row_key(section,row):
    name=row.get('case',row.get('path','').split('/')[-1])
    return json.dumps([section,name,row.get('mode'),row.get('flags'),row.get('options')],separators=(',',':'))
def signature(row):
    return hashlib.sha256(json.dumps(row,sort_keys=True,separators=(',',':')).encode()).hexdigest()
def main():
    ap=argparse.ArgumentParser();ap.add_argument('--record-decisions',action='store_true');args=ap.parse_args()
    pin=json.loads((ROOT/'data/UPSTREAM.json').read_text())['commit']
    assert subprocess.check_output(['git','-C',str(UP),'rev-parse','HEAD'],text=True).strip()==pin
    gzip_path=shutil.which('gzip')
    if not gzip_path:raise SystemExit('The pinned oracle ZIP/.Z matrix requires its gzip test helper')
    helper=dict(path=gzip_path,sha256=hashlib.sha256(Path(gzip_path).read_bytes()).hexdigest(),
                version=subprocess.check_output([gzip_path,'--version'],text=True).splitlines()[0])
    manifest=json.loads((ROOT/'tests/fixtures/compression/manifest.json').read_text())
    # Lzip has independent Python LZMA1/container vectors, but is not native in
    # this oracle build; never report it as capability-matched differential.
    paths=[ROOT/'tests/fixtures/compression'/r['name']for r in manifest['cases']if r['codec']!='lzip']
    lib=library();api=[];cli=[];stdin=[]
    flags=[4,0x2000004,0x414,0x14,0x404,0x24,0x804,0x1000004,0x1004]
    for mode in ['path','buffer','descriptor']:
        for flag in flags:api+=run_group(lib,paths,ROOT/'data/magic.mgc',flag,mode)
    for path in paths:
        for options in [['-z'],['-Z'],['-z','-i'],['-z','--mime-type'],['-z','--mime-encoding'],['-z','-k'],['-z','--apple'],['-z','--extension']]:
            for sequential,rows in [(False,cli),(True,stdin)]:
                results=[]
                for binary in [UP/'src/file',ROOT/'target/release/rfile']:
                    r=subprocess.run([str(binary),'-S','-b','-m',str(ROOT/'data/magic.mgc'),*options,'-'if sequential else str(path)],input=path.read_bytes()if sequential else None,env=ENV,capture_output=True,timeout=10)
                    results.append(dict(status=r.returncode,stdout_hex=r.stdout.hex(),stderr_hex=r.stderr.hex()))
                rows.append(dict(case=path.name,options=options,reference=results[0],rust=results[1],equal=results[0]==results[1]))
    report=dict(schema_version=1,upstream_commit=pin,environment=dict(LC_ALL='C',TZ='UTC'),database_sha256=hashlib.sha256((ROOT/'data/magic.mgc').read_bytes()).hexdigest(),binary_sha256=hashlib.sha256((ROOT/'target/release/rfile').read_bytes()).hexdigest(),features='extended-codecs',api=api,cli=cli,stdin=stdin,total=len(api)+len(cli)+len(stdin),exact=sum(r['equal']for r in api+cli+stdin),independent_only=['lzip'],oracle_helpers=['gzip (.Z/ZIP)'],scope='valid independently encoded payloads; no malformed-error equivalence claim')
    decisions_path=ROOT/'tests/fixtures/compression/compatibility-decisions.json'
    decisions={} if args.record_decisions else json.loads(decisions_path.read_text())['cases']
    additional={}
    if not args.record_decisions:
        observations=json.loads((ROOT/'tests/fixtures/compression/compatibility-observations.json').read_text())
        if observations['upstream_commit']!=pin or observations['base_decisions_sha256']!=hashlib.sha256(decisions_path.read_bytes()).hexdigest():
            raise SystemExit('Supplemental codec observations do not match the reviewed base decisions')
        additional=observations['cases']
    unexpected=[];counts={}
    for section in ['api','cli','stdin']:
        for row in report[section]:
            if row['equal']:
                row['disposition']='exact';continue
            name=row.get('case',row.get('path','').split('/')[-1]);key=row_key(section,row)
            reason=('helper-Z-single-result'if name.startswith('compress')else
                    'helper-ZIP-first-entry'if name.startswith('zip')else
                    'native-LZ4-valid-empty'if name=='lz4-empty'else None)
            value=dict(observation_sha256=signature(row),decision=reason)
            if args.record_decisions and reason:decisions[key]=value
            if reason and (decisions.get(key)==value or additional.get(key)==value):
                row['disposition']=reason;counts[reason]=counts.get(reason,0)+1
                if decisions.get(key)!=value:row['observation_set']='supplemental'
            else:
                row['disposition']='unexpected';unexpected.append(key)
    if args.record_decisions:
        decisions_path.write_text(json.dumps(dict(schema_version=1,upstream_commit=pin,policy='Explicit single in-process result for .Z; inspect first ZIP entry without reproducing helper diagnostics; accept valid empty LZ4. Both profiles. Raw result hashes freeze each observed case; new or changed disagreements fail.',cases=decisions),indent=2)+'\n')
    report['intentional_differences']=counts;report['unexpected']=unexpected;report['gzip_helper']=helper
    report['supplemental_observations_accepted']=sum(row.get('observation_set')=='supplemental'for section in ['api','cli','stdin']for row in report[section])
    (ROOT/'reports/compression.json').write_text(json.dumps(report,indent=2)+'\n')
    print({k:v for k,v in report.items()if k not in {'api','cli','stdin'}})
    if unexpected:raise SystemExit('Unexpected codec classification disagreements: inspect reports/compression.json')
if __name__=='__main__':main()
