#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Type-correct printf float differential using pinned compiler and IEEE bit vectors."""
import hashlib,json,math,os,platform,random,struct,subprocess,tempfile
from pathlib import Path
from differential import library
from fuzz_campaign import source_digest
from reference import UP
ROOT=Path(__file__).resolve().parents[1]
ENV=dict(os.environ,LC_ALL='C',TZ='UTC')
def main():
    lib=library();flags=0xffb000;rows=[];programs=[];rng=random.Random(548)
    formats=['%g','%G','%e','%E','%f','%F','%.0g','%.1g','%.17g','%.64g','%.65g','%.128g','%.0f','%#.0f','%#.0e','%#.1g','%#.6g','%#g','%+g','% g','%+015.6g','%015.6f','%-15.6g','% 015.6e','%.128f','%.128e','%.1022f','%.1023f','%1024f','%.1024f',
             '%1.1024f','%lf','%hf','%Lf','%Le']
    values=[0.,-0.,1.,-1.,1.25,-1.25,1.5,2.5,9.5,9.99999,0.0001,0.0000999999,999999.,999999.9,1e-100,1e100,9.99,99.99,999.99,9999.99,99999.99,9999999.9,float('inf'),-float('inf'),float('nan'),math.nextafter(1e6,0.),math.nextafter(1e-4,0.)]
    bits64=[struct.unpack('<Q',struct.pack('<d',v))[0]for v in values]+[1,2,0x000fffffffffffff,0x0010000000000000,0x3ff0000000000001,0x7fefffffffffffff,0xffefffffffffffff,0xfff8000000000001,0x7ff0000000000001]+[rng.getrandbits(64)for _ in range(100)]
    bits32=[struct.unpack('<I',struct.pack('<f',v))[0]for v in values if abs(v)<3.4e38 or not __import__('math').isfinite(v)]+[1,2,0x007fffff,0x00800000,0x3f800001,0x7f7fffff,0xff7fffff,0xffc00001,0x7f800001]+[rng.getrandbits(32)for _ in range(100)]
    with tempfile.TemporaryDirectory(prefix='rfile-float-')as directory:
        directory=Path(directory)
        for kind,bits,width in [('lefloat',bits32,4),('ledouble',bits64,8)]:
            paths=[];payloads=[]
            for i,value in enumerate(bits):
                data=value.to_bytes(width,'little');p=directory/f'{kind}-{i}';p.write_bytes(data);paths.append(p);payloads.append(data)
            template=directory/'template';template.write_text(f'0 {kind} x %g\n')
            subprocess.run([str(UP/'src/file'),'-C','-m',str(template)],cwd=directory,env=ENV,check=True,capture_output=True)
            template_bytes=Path(str(template)+'.mgc').read_bytes()
            for fmt in formats:
                source=f'0 {kind} x {fmt}\n';magic=directory/'rules';magic.write_text(source)
                c=subprocess.run([str(UP/'src/file'),'-C','-m',str(magic)],cwd=directory,env=ENV,capture_output=True)
                # The C source validator rejects some type-correct alternate
                # forms. Runtime file_checkfmt accepts them; patch only the
                # fixed description field in its valid same-kind v21 template.
                db=directory/'rules.mgc'
                compiled=bytearray(template_bytes);desc=fmt.encode();assert len(desc)<64
                compiled[432+160:432+224]=desc+b'\0'*(64-len(desc));db.write_bytes(compiled)
                programs.append(dict(kind=kind,format=fmt,source=source,source_compiler_status=c.returncode,source_compiler_stderr=c.stderr.decode(errors='backslashreplace'),database_sha256=hashlib.sha256(compiled).hexdigest()))
                source_only=directory/'source-only'
                source_only.write_text(source)
                routes=[('compiled',db)]
                if c.returncode==0:routes.append(('source',source_only))
                for route,selected in routes:
                    h=lib.magic_open(flags);assert lib.magic_load(h,os.fsencode(selected))==0
                    run=subprocess.run([str(ROOT/'target/release/rfile-probe'),str(selected),f'{flags:x}','buffer'],input=''.join(str(p)+'\n'for p in paths).encode(),env=ENV,capture_output=True,timeout=60)
                    if run.returncode:raise RuntimeError(run.stderr)
                    actual=[json.loads(line)for line in run.stdout.splitlines()];assert len(actual)==len(bits)
                    for value,data,row in zip(bits,payloads,actual):
                        ref=lib.magic_buffer(h,data,len(data));out=bytes.fromhex(row['hex'])if'hex'in row else None
                        error=(lib.magic_error(h)or b'') if ref is None else None
                        equal=(ref==out and (ref is not None or (error is not None and error.hex()==row.get('error_hex'))))
                        rows.append(dict(route=route,kind=kind,format=fmt,bits_hex=f'{value:0{width*2}x}',equal=equal,**({}if equal else dict(reference_hex=ref.hex()if ref is not None else None,reference_error_hex=error.hex() if error is not None else None,rust_hex=out.hex()if out is not None else None,rust_error=row.get('error')))))
                    lib.magic_close(h)
    report=dict(schema_version=1,upstream_commit=json.loads((ROOT/'data/UPSTREAM.json').read_text())['commit'],source_sha256=source_digest(),binary_sha256=hashlib.sha256((ROOT/'target/release/rfile-probe').read_bytes()).hexdigest(),environment=dict(LC_ALL='C',TZ='UTC',libc=platform.libc_ver(),rounding='host default FE_TONEAREST'),formats=formats,programs=programs,total=len(rows),exact=sum(r['equal']for r in rows),cases=rows)
    (ROOT/'reports/float.json').write_text(json.dumps(report,indent=2)+'\n');print({k:v for k,v in report.items()if k not in ['cases','programs','formats']})
    for row in [r for r in rows if not r['equal']][:15]:print(row)
    if report['total']!=report['exact']:raise SystemExit('Float differences')
if __name__=='__main__':main()
