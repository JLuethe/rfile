#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""R8 primitive check-dump gate. Discovery warnings/failed-source diagnostics
are recorded separately and are not claimed as completed action/ABI parity.
Build the research driver: cargo build --all-features --example source_check.
"""
import argparse,hashlib,json,os,re,subprocess,tempfile
from pathlib import Path
from reference import UP,manifest
ROOT=Path(__file__).resolve().parents[1]
ENV=dict(os.environ,LC_ALL='C',TZ='UTC')
for key in ['MAGIC','POSIXLY_CORRECT','LD_PRELOAD']:ENV.pop(key,None)
def cases():
    types=['byte','ubyte','short','long','beshort','belong','leshort','lelong','melong','quad','lequad','bequad','offset','float','double','befloat','ledouble','date','ldate','qdate','qldate','qwdate','msdosdate','msdostime','bevarint','levarint']
    for kind in types:
        for value in ['x','0','1','-1','127','128','255','256']:
            yield f'{kind}:{value}',f'0 {kind} {value} test\n'.encode()
    for kind,value in [('string',r'A\000B\n\xff'),('string/WwCcTfb','AB'),('pstring/H','AB'),('pstring/L','AB'),('regex/b','a+b'),('search/25/c','Ab'),('name','named'),('use','named'),('indirect','x'),('der','seq'),('octal','0123'),('guid','00112233-4455-6677-8899-aabbccddeeff')]:
        yield kind,f'0 {kind} {value} test\n'.encode()
    yield 'empty-description',b'0 string AB\n>2 byte 1 child\n'
    yield 'indirection',b'0 string AB root\n>(2.b+1) byte 1 child\n'
    yield 'inverted',b'0 byte~&0x80 =1 test\n'
def compare(driver,path,source,name,cwd):
    c=subprocess.run([str(UP/'src/file'),'-S','-c','-m',str(path)],capture_output=True,env=ENV,cwd=cwd,timeout=20)
    r=subprocess.run([str(driver),str(path)],input=source,capture_output=True,env=ENV,timeout=20)
    fallback=b"Warning: using regular magic file `"+os.fsencode(path)+b"'\n"
    if not c.stderr.startswith(fallback):raise SystemExit('Unexpected reference source-load prefix: '+repr(c.stderr[:200]))
    body=c.stderr[len(fallback):]
    equal=(c.returncode,c.stdout,body)==(r.returncode,r.stdout,r.stderr)
    row=dict(name=name,source_sha256=hashlib.sha256(source).hexdigest(),reference_status=c.returncode,rust_status=r.returncode,check_dump_equal=equal,reference_dump_sha256=hashlib.sha256(body).hexdigest(),rust_dump_sha256=hashlib.sha256(r.stderr).hexdigest())
    if not equal:row.update(reference_dump_hex=body.hex(),rust_dump_hex=r.stderr.hex())
    return row
def main():
    ap=argparse.ArgumentParser();ap.add_argument('--driver',default='target/debug/examples/source_check');args=ap.parse_args();driver=(ROOT/args.driver).resolve()
    rows=[]
    with tempfile.TemporaryDirectory(prefix='rfile-check-dump-')as tmp:
        tmp=Path(tmp)
        for name,source in cases():
            (tmp/'magic').write_bytes(source)
            rows.append(compare(driver,Path('magic'),source,name,tmp))
    text=(UP/'magic/Makefile.am').read_text().split('MAGIC_FRAGMENTS =',1)[1].split('EXTRA_DIST',1)[0]
    for a,b in re.findall(r'\$\(MAGIC_DIR\)/([^\s\\]+)|\$\(MAGIC_FRAGMENT_DIR\)/([^\s\\]+)',text):
        path=UP/'magic'/a if a else UP/'magic/Magdir'/b
        rows.append(compare(driver,path,path.read_bytes(),f'fragment:{path.name}',ROOT))
    valid=[r for r in rows if r['reference_status']==0]
    report=dict(schema_version=1,scope='Source-check dump primitive only: one exact discovery warning is separated from each native CLI invocation. Failed-source diagnostics/state remain open; this is not full CLI or C ABI conformance.',reference=manifest(),driver_sha256=hashlib.sha256(driver.read_bytes()).hexdigest(),total=len(rows),valid=len(valid),valid_exact=sum(r['check_dump_equal']for r in valid),malformed=len(rows)-len(valid),cases=rows)
    (ROOT/'reports/source-dump.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({k:v for k,v in report.items()if k not in ['cases','reference']},indent=2))
    if any(not r['check_dump_equal']for r in valid):raise SystemExit('valid source-check dump mismatch')
if __name__=='__main__':main()
