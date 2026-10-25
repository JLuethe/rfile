#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Linux/glibc filename, result and capture display differential; no global Rust locale mutation."""
import ctypes,gzip,hashlib,json,locale,os,platform,subprocess,tempfile
from pathlib import Path
from differential import library
from fuzz_campaign import source_digest
from reference import UP
ROOT=Path(__file__).resolve().parents[1]
DB=ROOT/'data/magic.mgc'
LOCALES=['C','C.UTF-8','en_US.UTF-8']
def invoke(binary,db,options,args,cwd,env,stdin=None):
    r=subprocess.run([os.fsencode(binary),b'-S',b'-m',os.fsencode(db),*map(os.fsencode,options),*map(os.fsencode,args)],input=stdin,cwd=cwd,env=env,capture_output=True,timeout=10)
    return dict(status=r.returncode,stdout_hex=r.stdout.hex(),stderr_hex=r.stderr.hex())
def main():
    lib=library();cli=[];api=[];programs=[]
    base=dict(os.environ,TZ='UTC');base.pop('POSIXLY_CORRECT',None);base.pop('MAGIC',None)
    with tempfile.TemporaryDirectory(prefix='rfile-locale-')as directory:
        directory=Path(directory)
        names=[b'ascii',b'control-\x01',b'tab-\t',b'line-\n',b'del-\x7f',b'bad-\xff',b'partial-\xc3',b'partial-\xe2\x82',b'overlong-\xc0\x80']
        names += [s.encode()for s in ['café','e\u0301','漢字','😀','variation-❤\ufe0f','zero-\u200b','line-separator-\u2028','space-\u00a0']]
        names += ['café-'.encode()+b'\xff'+'-漢'.encode(),b'bad-\xff-'+'é'.encode()]
        for name in names:(directory/os.fsdecode(name)).write_bytes(b'hello\n')
        (directory/'broken').symlink_to(os.fsdecode('café-'.encode()+b'\xff'))
        (directory/'valid-link').symlink_to('café')
        listing=directory/'filenames';listing.write_bytes(b''.join(n+b'\n'for n in names if b'\n'not in n))
        for language in LOCALES:
            locale.setlocale(locale.LC_CTYPE,language) # C test oracle only, serial harness.
            env=dict(base,LC_ALL=language)
            cases=[(options,names,None)for options in [[],['-r'],['-N'],['-0'],['-00'],['-b'],['-bb'],['-i'],['-F','|']]]
            cases += [(options,[b'broken',b'valid-link'],None)for options in [[],['-r'],['-i'],['-E']]]
            cases += [(options,[b'missing-'+n],None)for n in names for options in [[],['-E'],['-r','-E']]]
            cases += [(options,[],None)for options in [['-f','filenames'],['-n','-f','filenames'],['-r','-f','filenames'],['-00','-f','filenames']]]
            for index,(options,args,stdin)in enumerate(cases):
                results=[invoke(binary,DB,options,args,directory,env,stdin)for binary in [UP/'src/file',ROOT/'target/release/rfile']]
                cli.append(dict(locale=language,case=index,options=options,args_hex=[os.fsencode(a).hex()for a in args],reference=results[0],rust=results[1],equal=results[0]==results[1]))
        # The reference initializes LC_CTYPE only. LC_ALL/LC_CTYPE/LANG
        # precedence and invalid-name policy are checked in isolated children.
        for choices in [dict(LC_ALL='C',LC_CTYPE='C.UTF-8',LANG='en_US.UTF-8'),dict(LC_ALL='',LC_CTYPE='C.UTF-8',LANG='C'),dict(LC_ALL='',LC_CTYPE='',LANG='en_US.UTF-8'),dict(LC_ALL='',LC_CTYPE='',LANG='')]:
            env=dict(base,**choices)
            results=[invoke(binary,DB,[],names,directory,env)for binary in [UP/'src/file',ROOT/'target/release/rfile']]
            cli.append(dict(environment=choices,case='precedence',reference=results[0],rust=results[1],equal=results[0]==results[1]))
        for invalid in [False,True]:
            source=b'0 string TEST caf\xc3\xa9-inner\n0 string \\037\\213 outer-'+(b'\xff'if invalid else '\u6f22'.encode())+b'\n>4 byte x %c-tail\n'
            magic=directory/('compressed-invalid'if invalid else'compressed-valid');magic.write_bytes(source)
            subprocess.run([str(UP/'src/file'),'-C','-m',str(magic)],cwd=directory,env=dict(base,LC_ALL='C'),check=True,capture_output=True)
            compiled=Path(str(magic)+'.mgc')
            compressed=directory/'payload.gz';compressed.write_bytes(gzip.compress(b'TEST hello\n',mtime=0))
            programs.append(dict(case='compressed-'+str(invalid),source_hex=source.hex(),database_sha256=hashlib.sha256(compiled.read_bytes()).hexdigest()))
            for language in LOCALES:
                env=dict(base,LC_ALL=language)
                for options in [['-z','-b'],['-Z','-b'],['-z','-r','-b'],['-z','-k','-b'],['-z','-i','-b']]:
                    results=[invoke(binary,compiled,options,['payload.gz'],directory,env)for binary in [UP/'src/file',ROOT/'target/release/rfile']]
                    cli.append(dict(locale=language,case='compressed-'+str(invalid),options=options,reference=results[0],rust=results[1],equal=results[0]==results[1]))
        template=directory/'template';template.write_bytes(b'0 byte x PLACEHOLDER\n')
        subprocess.run([str(UP/'src/file'),'-C','-m',str(template)],cwd=directory,env=dict(base,LC_ALL='C'),check=True,capture_output=True)
        image=bytearray(Path(str(template)+'.mgc').read_bytes())
        descriptions=[b'ASCII',b'line\ncontrol\t', 'café'.encode(),'漢字 😀'.encode(),'e\u0301 \u2028'.encode(),'café'.encode()+b'\xff','café'.encode()+b'\xe2\x82',b'\xff'+'漢'.encode(),b'%c',b'before%cafter','café%cafter'.encode()]
        payloads=[b'\0AAA',b'\x01AAA',b'\xc8AAA']
        paths=[]
        for i,payload in enumerate(payloads):p=directory/f'payload-{i}';p.write_bytes(payload);paths.append(p)
        variants=[]
        for index,description in enumerate(descriptions):
            assert len(description)<64
            compiled=bytearray(image);compiled[432+160:432+224]=description+b'\0'*(64-len(description))
            variants.append((index,description,compiled))
        for label,source in [('empty-string',b'0 string x %s\n'),('empty-regex',b'0 regex/b a* %s\n')]:
            source_path=directory/label;source_path.write_bytes(source)
            subprocess.run([str(UP/'src/file'),'-C','-m',str(source_path)],cwd=directory,env=dict(base,LC_ALL='C'),check=True,capture_output=True)
            variants.append((label,source,Path(str(source_path)+'.mgc').read_bytes()))
        for index,description,compiled in variants:
            db=directory/f'description-{index}.mgc';db.write_bytes(compiled)
            programs.append(dict(case=index,description_hex=description.hex(),database_sha256=hashlib.sha256(compiled).hexdigest()))
            for language in LOCALES:
                locale.setlocale(locale.LC_CTYPE,language)
                env=dict(base,LC_ALL=language)
                for flags in [0xffb000,0xffb100,0xffb020,0xffb410]:
                    h=lib.magic_open(flags);assert lib.magic_load(h,os.fsencode(db))==0
                    for mode in ['path','buffer','descriptor']:
                        run=subprocess.run([str(ROOT/'target/release/rfile-probe'),str(db),f'{flags:x}',mode,language],input=''.join(str(p)+'\n'for p in paths).encode(),env=env,capture_output=True,timeout=10)
                        if run.returncode:raise RuntimeError(f"locale={language} case={index} flags={flags:x} mode={mode}: {run.stderr!r}")
                        rows=[json.loads(line)for line in run.stdout.splitlines()];assert len(rows)==len(paths)
                        for payload,path,row in zip(payloads,paths,rows):
                            if mode=='buffer':ref=lib.magic_buffer(h,payload,len(payload))
                            elif mode=='descriptor':
                                with path.open('rb')as f:ref=lib.magic_descriptor(h,f.fileno())
                            else:ref=lib.magic_file(h,os.fsencode(path))
                            actual=bytes.fromhex(row['hex'])if'hex'in row else None
                            api.append(dict(case=index,locale=language,flags=flags,mode=mode,input_hex=payload.hex(),reference_hex=ref.hex()if ref is not None else None,rust_hex=actual.hex()if actual is not None else None,rust_error=row.get('error'),equal=ref is not None and ref==actual))
                    lib.magic_close(h)
    report=dict(schema_version=1,upstream_commit=json.loads((ROOT/'data/UPSTREAM.json').read_text())['commit'],source_sha256=source_digest(),binary_sha256=hashlib.sha256((ROOT/'target/release/rfile').read_bytes()).hexdigest(),host=platform.platform(),libc=platform.libc_ver(),locale_data={str(p):hashlib.file_digest(p.open('rb'),'sha256').hexdigest()for p in [Path('/usr/lib/locale/locale-archive'),Path('/usr/lib/locale/C.utf8/LC_CTYPE'),Path('/usr/lib/locale/en_US.utf8/LC_CTYPE')]if p.is_file()},locales=LOCALES,filename_inputs_hex=[n.hex()for n in names],programs=programs,cli=cli,api=api,total=len(cli)+len(api),exact=sum(r['equal']for r in cli+api))
    (ROOT/'reports/locale.json').write_text(json.dumps(report,indent=2)+'\n');print({k:v for k,v in report.items()if k not in ['cli','api','programs','filename_inputs_hex']})
    for section in ['cli','api']:
        for row in [r for r in report[section]if not r['equal']][:12]:print(section,row)
    if report['total']!=report['exact']:raise SystemExit('Locale differences')
if __name__=='__main__':main()
