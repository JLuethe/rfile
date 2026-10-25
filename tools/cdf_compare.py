#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""CDF CLI/API conformance, separately labeling corrected outputs and limits."""
import ctypes
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import cdf_fixtures as fixtures
from differential import library, run_group
from reference import UP
ROOT=Path(__file__).resolve().parents[1]
ENV=dict(os.environ,LC_ALL='C',TZ='UTC')

def invoke(binary,path,flags,profile=None):
    args=[str(binary),'-b','-m',str(ROOT/'data/magic.mgc'),*flags,str(path)]
    if profile: args.insert(1,'--profile='+profile)
    r=subprocess.run(args,env=ENV,capture_output=True,timeout=10)
    return dict(status=r.returncode,stdout_hex=r.stdout.hex(),stderr_hex=r.stderr.hex())

def main():
    pin=json.loads((ROOT/'data/UPSTREAM.json').read_text())['commit']
    if subprocess.check_output(['git','-C',str(UP),'rev-parse','HEAD'],text=True).strip()!=pin: raise SystemExit('Reference revision drift')
    rows=[];corrected=[];api=[];manifest=[]
    with tempfile.TemporaryDirectory(prefix='rfile-cdf-')as directory:
        directory=Path(directory)
        cases=fixtures.cases()
        simple=fixtures.property_stream([(1,fixtures.typed(2,(1252).to_bytes(2,'little'))),(18,fixtures.text('Microsoft Word'))])
        cases.append(('beyond-seven-mib-difat',fixtures.compound([('\x05SummaryInformation',simple)],gap=16384)))
        paths=[]
        for name,data in cases:
            path=directory/(name+'.cfb');path.write_bytes(data);paths.append(path)
            manifest.append(dict(name=name,bytes=len(data),sha256=hashlib.sha256(data).hexdigest()))
            for flags in [[],['-i'],['--mime-type'],['--mime-encoding'],['-k'],['-e','cdf']]:
                c=invoke(UP/'src/file',path,flags)
                rust=invoke(ROOT/'target/release/rfile',path,flags)
                equal=c==rust
                limit=(name=='vector-limit' and flags!=['-e','cdf'] and rust==dict(status=1,stdout_hex=b'ERROR: CDF vector elements limit exceeded\n'.hex(),stderr_hex=''))
                rows.append(dict(case=name,flags=flags,reference=c,rust=rust,equal=equal,disposition='exact' if equal else 'safety-limit' if limit else 'unclassified'))
                if not flags or flags==['-i']:
                    corrected.append(dict(case=name,flags=flags,result=invoke(ROOT/'target/release/rfile',path,flags,'corrected-v1'),oracle='independent wire fixtures; primary C summary pointer defect not corrected truth'))
        lib=library()
        for mode in ['buffer','path']:
            for flags in [0,0x410,0x10,0x400,0x20]:
                comparisons=run_group(lib,paths,ROOT/'data/magic.mgc',flags,mode)
                for row in comparisons:
                    name=Path(row['path']).stem
                    row['case']=name
                    if not row['equal']:
                        row['disposition']='safety-limit' if name=='vector-limit' and row.get('rust_error')=='CDF vector elements limit exceeded' else 'unclassified'
                    else: row['disposition']='exact'
                    api.append(row)
    report=dict(schema_version=1,upstream_commit=pin,environment={'LC_ALL':'C','TZ':'UTC'},fixtures=manifest,
                cli=rows,api=api,corrected=corrected,total=len(rows)+len(api),
                exact=sum(r['equal']for r in rows+api),safety_limits=sum(r['disposition']=='safety-limit'for r in rows+api),
                unexpected=sum(r['disposition']=='unclassified'for r in rows+api))
    (ROOT/'reports/cdf.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({k:v for k,v in report.items()if k not in {'cli','api','corrected','fixtures'}}))
    if report['unexpected']:raise SystemExit('CDF mismatch: inspect reports/cdf.json')
if __name__=='__main__':main()
