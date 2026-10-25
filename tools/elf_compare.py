#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Pinned ELF path/buffer and CLI comparisons over independent synthetic input."""
import json,os,subprocess,tempfile,hashlib,struct
from pathlib import Path
from differential import library,run_group
from elf_fixtures import cases,elf,note
from reference import UP
ROOT=Path(__file__).resolve().parents[1]
ENV=dict(os.environ,LC_ALL='C',TZ='UTC')
def main():
    pin=json.loads((ROOT/'data/UPSTREAM.json').read_text())['commit']
    if subprocess.check_output(['git','-C',str(UP),'rev-parse','HEAD'],text=True).strip()!=pin:raise SystemExit('Reference revision drift')
    api=[];cli=[];stdin=[];manifest=[]
    with tempfile.TemporaryDirectory(prefix='rfile-elf-')as directory:
        paths=[]
        for name,data in cases():
            path=Path(directory)/(name+'.elf');path.write_bytes(data);paths.append(path)
            manifest.append(dict(name=name,bytes=len(data),sha256=hashlib.sha256(data).hexdigest()))
        lib=library()
        for mode in ['path','buffer','descriptor']:
            for flags in [0,0x410,0x10,0x400,0x20,0x10000,0x100]:
                rows=run_group(lib,paths,ROOT/'data/magic.mgc',flags,mode)
                for row in rows:
                    row['case']=Path(row['path']).stem
                    if row.get('reference') is None and row.get('rust') is None and row.get('reference_error_hex') is not None and row.get('reference_error_hex')==row.get('rust_error_hex'):row['equal']=True
                    row['error_comparison']=row.get('reference_error') is not None
                api+=rows
        for path in paths:
            for flags in [[],['-i'],['-k'],['-r']]:
                results=[]
                for binary in [UP/'src/file',ROOT/'target/release/rfile']:
                    r=subprocess.run([str(binary),'-b','-m',str(ROOT/'data/magic.mgc'),*flags,str(path)],env=ENV,capture_output=True,timeout=10)
                    results.append(dict(status=r.returncode,stdout_hex=r.stdout.hex(),stderr_hex=r.stderr.hex()))
                cli.append(dict(case=path.stem,flags=flags,reference=results[0],rust=results[1],equal=results[0]==results[1]))
        for path in paths:
            for flags in [[],['-i'],['-k'],['-r']]:
                results=[]
                for binary in [UP/'src/file',ROOT/'target/release/rfile']:
                    r=subprocess.run([str(binary),'-b','-m',str(ROOT/'data/magic.mgc'),*flags,'-'],input=path.read_bytes(),env=ENV,capture_output=True,timeout=10)
                    results.append(dict(status=r.returncode,stdout_hex=r.stdout.hex(),stderr_hex=r.stderr.hex()))
                stdin.append(dict(case=path.stem,flags=flags,reference=results[0],rust=results[1],equal=results[0]==results[1]))
        # A sparse >4 GiB note location exercises u64 offsets without allocating
        # or reading the hole as a Python bytes object.
        large=Path(directory)/'sparse-note-4gib.elf'
        image=bytearray(elf([(4,note(b'GNU\0',3,b'12345678'),4,0)]));offset=(1<<32)+4096
        payload=bytes(image[120:]);struct.pack_into('<Q',image,72,offset)
        with large.open('wb')as stream:stream.write(image[:120]);stream.seek(offset);stream.write(payload)
        manifest.append(dict(name=large.stem,bytes=large.stat().st_size,sparse=True,note_offset=offset,header_sha256=hashlib.sha256(image[:120]).hexdigest(),note_sha256=hashlib.sha256(payload).hexdigest()))
        for mode in ['path','buffer','descriptor']:
            for flags in [0,0x410,0x20]:
                rows=run_group(lib,[large],ROOT/'data/magic.mgc',flags,mode)
                for row in rows:row['case']=large.stem
                api+=rows
    report=dict(schema_version=1,upstream_commit=pin,environment=dict(LC_ALL='C',TZ='UTC'),fixtures=manifest,api=api,cli=cli,stdin=stdin,total=len(api)+len(cli)+len(stdin),exact=sum(r['equal']for r in api+cli+stdin))
    (ROOT/'reports/elf.json').write_text(json.dumps(report,indent=2)+'\n');print(dict(total=report['total'],exact=report['exact']))
    if report['total']!=report['exact']:raise SystemExit('ELF differences: inspect reports/elf.json')
if __name__=='__main__':main()
