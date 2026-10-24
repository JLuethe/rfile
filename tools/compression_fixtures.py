#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Independent encoder fixtures. Native tools/Python are test-only, never runtime."""
import bz2,gzip,hashlib,io,json,lzma,random,struct,subprocess,zipfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];DEST=ROOT/'tests/fixtures/compression'
def zip_bytes(payload,method,descriptor=False,zip64=False):
    class Stream(io.BytesIO):
        def seekable(self):return False
        def seek(self,*args):raise io.UnsupportedOperation('sequential fixture')
    value=Stream()if descriptor else io.BytesIO()
    with zipfile.ZipFile(value,'w',compression=method)as archive:
        info=zipfile.ZipInfo('first',(2020,1,2,3,4,6));info.compress_type=method
        if zip64:
            with archive.open(info,'w',force_zip64=True)as entry:entry.write(payload)
        else:archive.writestr(info,payload)
        info=zipfile.ZipInfo('second',(2020,1,2,3,4,6));info.compress_type=method;archive.writestr(info,b'OTHER')
    return value.getvalue()
def lzip_bytes(payload):
    raw=lzma.compress(payload,format=lzma.FORMAT_RAW,filters=[dict(id=lzma.FILTER_LZMA1,dict_size=1<<23,lc=3,lp=0,pb=2)])
    header=b'LZIP'+bytes([1,23]);return header+raw+struct.pack('<IQQ',__import__('zlib').crc32(payload),len(payload),len(header)+len(raw)+20)
def command(args,payload):return subprocess.run(args,input=payload,capture_output=True,check=True).stdout
def main():
    import zlib
    DEST.mkdir(parents=True,exist_ok=True)
    payload=b'TEST codec payload: hello world!\\n'*4
    payloads={'payload':payload,'empty':b'','binary':bytes(range(256))*8,
              'random':random.Random(548).randbytes(180000),
              'runs':b'A'*300000+b'BCBCBCBC'*20000}
    for name,data in payloads.items():(DEST/name).write_bytes(data)
    rows=[];encoded={}
    def add(name,codec,data,expected='payload',pinned=None,corrected=None):
        (DEST/name).write_bytes(data);rows.append(dict(name=name,codec=codec,bytes=len(data),sha256=hashlib.sha256(data).hexdigest(),payload=expected,pinned_payload=pinned or expected,corrected_payload=corrected or expected))
    for key,plain in payloads.items():
        encoders={'gzip':lambda:gzip.compress(plain,mtime=0),'zlib':lambda:zlib.compress(plain),'bzip2':lambda:bz2.compress(plain,compresslevel=1),
                  'xz':lambda:lzma.compress(plain),'lzma':lambda:lzma.compress(plain,format=lzma.FORMAT_ALONE),'lzip':lambda:lzip_bytes(plain),
                  'zstd':lambda:command(['zstd','-q','-c'],plain),'lz4':lambda:command(['lz4','-q','-c','-B4'],plain),
                  'compress':lambda:command([str(ROOT/'.reference/ncompress/compress'),'-c','-f'],plain),
                  'zip-stored':lambda:zip_bytes(plain,zipfile.ZIP_STORED),'zip-deflate':lambda:zip_bytes(plain,zipfile.ZIP_DEFLATED)}
        for codec,encode in encoders.items():
            data=encode();name=codec if key=='payload'else codec+'-'+key
            add(name,'zip'if codec.startswith('zip-')else codec,data,key)
            if key=='payload':encoded[codec]=data
    second=b'SECOND member\n';(DEST/'two-members').write_bytes(payload+second)
    for codec,encoder in [('gzip',lambda:gzip.compress(second,mtime=0)),('zlib',lambda:zlib.compress(second)),('bzip2',lambda:bz2.compress(second)),('xz',lambda:lzma.compress(second)),('lzip',lambda:lzip_bytes(second)),('zstd',lambda:command(['zstd','-q','-c'],second)),('lz4',lambda:command(['lz4','-q','-c','-B4'],second))]:
        add(codec+'-concat',codec,encoded[codec]+encoder(),corrected='two-members')
    for bits in [9,10,12,16]:add(f'compress-bits-{bits}','compress',command([str(ROOT/'.reference/ncompress/compress'),'-c','-f','-b',str(bits)],payloads['random']),'random')
    for check in [lzma.CHECK_NONE,lzma.CHECK_CRC32,lzma.CHECK_CRC64,lzma.CHECK_SHA256]:add(f'xz-check-{check}','xz',lzma.compress(payload,check=check))
    for label,filters in [('delta',[dict(id=lzma.FILTER_DELTA,dist=3)]),('x86',[dict(id=lzma.FILTER_X86)]),('delta-x86',[dict(id=lzma.FILTER_DELTA,dist=2),dict(id=lzma.FILTER_X86)])]:
        add('xz-filter-'+label,'xz',lzma.compress(payloads['binary'],filters=filters+[dict(id=lzma.FILTER_LZMA2,dict_size=1<<20)]),'binary')
    add('zip-descriptor','zip',zip_bytes(payload,zipfile.ZIP_DEFLATED,descriptor=True))
    add('zip64','zip',zip_bytes(payload,zipfile.ZIP_DEFLATED,zip64=True))
    add('zip64-descriptor','zip',zip_bytes(payload,zipfile.ZIP_DEFLATED,descriptor=True,zip64=True))
    for label,options in [('linked',['-BD']),('block-check',['-BX']),('no-check',['--no-frame-crc']),('large-block',['-B7'])]:
        add('lz4-'+label,'lz4',command(['lz4','-q','-c','-B4',*options],payloads['runs']),'runs')
    add('zstd-no-check','zstd',command(['zstd','-q','-c','--no-check'],payloads['random']),'random')
    add('xz-multiblock','xz',command(['xz','-c','--block-size=32768'],payloads['random']),'random')
    ncompress=subprocess.check_output(['git','-C',str(ROOT/'.reference/ncompress'),'rev-parse','HEAD'],text=True).strip()
    report=dict(schema_version=1,encoders={'python':__import__('sys').version+'; stdlib gzip/zlib/bz2/lzma/zipfile','xz':command(['xz','--version'],b'').decode().splitlines()[0],
                'zstd':command(['zstd','--version'],b'').decode().strip(),'lz4':command(['lz4','--version'],b'').decode().strip(),'ncompress_commit':ncompress,
                'ncompress_build':'make CC="cc -Wno-error=incompatible-pointer-types"; unmodified test-only source'},
                payloads={name:dict(bytes=len(data),sha256=hashlib.sha256(data).hexdigest())for name,data in payloads.items()},cases=rows)
    (DEST/'manifest.json').write_text(json.dumps(report,indent=2)+'\n');print(f'Generated {len(rows)} independent compressed fixtures')
if __name__=='__main__':main()
