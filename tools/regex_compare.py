#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""GNU ERE differential, source/v21 and case/window/capture histories."""
import argparse,hashlib,itertools,json,os,subprocess,tempfile,platform
from pathlib import Path
from differential import library
from fuzz_campaign import source_digest
from reference import UP
ROOT=Path(__file__).resolve().parents[1]
ENV=dict(os.environ,LC_ALL='C',TZ='UTC')
def patterns():
    explicit=[r'\W+',r'\S+',r'\B',r'^a$',r'[^a]+',r'(a)\1{2}',r'()\1+',r'(a+)((\1))+\2',r'(ab)\1',r'(a|ab)\1',r'(ab|a)\1',r'(a*)\1',r'(a?)\1',r'(a)?\1',r'()\1',r'(a|)\1',r'((a)|b)*\2',r'((a)?b)*\2',r'((a|b)*)\1',r'((ab)*)\2',r'(a)+\1',r'(a?)*\1',r'(a*)*\1',r'(a|aa)*\1',r'((a)b|a(b))\2',r'(a(b)?)+\2',r'^(a*)\1$',r'(.)\1',r'([^a])\1',r'(a{0,3})\1',r'(a){2,3}\1',r'(a)(b)\2\1',r'((a)(b))\3\2\1',r'(a)\10',r'\d+',r'\0',r'[\d]+',r'[\]]+',r'[^\]]+',r'[]a]+',r'[^]a]+',r'[[:alpha:]\]+',r'[[.a.]]+',r'[[=a=]]+',r'[a-c]+',r'[^A-C]+',r'[a&&b]+',r'[a--]+',r'\<ab\>',r'\b(ab)\1\b',r'\w+\W*',r'\s+',r'\`(a)\1\x27',r'a|ab',r'[^[:digit:]]+',r'([[:lower:]])\1']
    for atom in ['a','ab','a|b','a|aa','a*','[ab]','[^x]','(a)?b']:
        for suffix in ['', '?','+','{0,2}']:explicit.append('('+atom+')'+suffix+r'\1')
    return list(dict.fromkeys(explicit))
def main():
    ap=argparse.ArgumentParser();ap.add_argument('--probe',default='target/release/rfile-probe');args=ap.parse_args()
    lib=library();flags=0xffb000
    inputs=[bytes(s)for n in range(1,6)for s in itertools.product(b'ab',repeat=n)]
    inputs += [b'',b'aa\n',b'ABAB!',b'aAaA!',b'abaaba!',b'abcabc!',b'aaa\0aaaa!',b'\n\na!',b'\\d]!!',b' ]a\\d\n!',bytes(range(32)),bytes([255,255,10]),b'0!',b'10!',b'aa0!',b'ddd!',b'xxab ab!',b'a\n\tb!',b'\n\n!',b' \t\r\v\f\n!',b'\xff\xfe!']
    rows=[];compiles=[];programs=[]
    restricted={r"(a+)((\1))+\2",r"(a*)+\1",r"(a|aa)+\1",r"(a){2,3}\1",r"((a)?b)+\1"}|{ "("+atom+"){0,2}\\1" for atom in ["a","ab","a|b","a|aa","a*","[ab]","[^x]","(a)?b"] }
    with tempfile.TemporaryDirectory(prefix='rfile-regex-')as directory:
        directory=Path(directory);files=[]
        for i,data in enumerate(inputs):p=directory/f'input-{i}';p.write_bytes(data);files.append(p)
        for index,pattern in enumerate(patterns()):
            for insensitive in [False,True]:
                literal=''.join('\\x%02x'%b for b in pattern.encode())
                source=f'0 regex/b'+('c'if insensitive else'')+f' {literal} value=%s\n'
                magic=directory/'rules.magic';magic.write_text(source)
                compiled=subprocess.run([str(UP/'src/file'),'-C','-m',str(magic)],cwd=directory,env=ENV,capture_output=True)
                if compiled.returncode:
                    if pattern != r'[a--]+':raise RuntimeError(f'Unexpected oracle compiler error: {pattern}: {compiled.stderr!r}')
                    compiles.append(dict(pattern=pattern,insensitive=insensitive,reference_error=compiled.stderr.decode(errors='replace')));continue
                database=directory/(magic.name+'.mgc')
                source_only=directory/'source-only.magic';source_only.write_text(source)
                assert not Path(str(source_only)+'.mgc').exists()
                programs.append(dict(pattern=pattern,insensitive=insensitive,source_sha256=hashlib.sha256(source.encode()).hexdigest(),compiled_database_sha256=hashlib.sha256(database.read_bytes()).hexdigest()))
                h=lib.magic_open(flags);assert lib.magic_load(h,os.fsencode(database))==0
                for dbmode,db in [('source',source_only),('v21',database)]:
                    run=subprocess.run([str(ROOT/args.probe),str(db),f'{flags:x}','buffer'],input=''.join(str(p)+'\n'for p in files).encode(),capture_output=True,env=ENV,timeout=30)
                    if run.returncode:raise RuntimeError(f'{pattern} {run.stderr.decode(errors="replace")}')
                    actual=[json.loads(line)for line in run.stdout.splitlines()];assert len(actual)==len(inputs)
                    for data,row in zip(inputs,actual):
                        ref=lib.magic_buffer(h,data,len(data));out=bytes.fromhex(row['hex'])if'hex'in row else None
                        equal=ref is not None and out is not None and ref==out
                        unsupported=pattern in restricted and row.get('error')=='unsupported: pinned glibc quantified backreference semantics; use corrected-v1 for exhaustive capture histories'
                        disposition='exact'if equal else 'explicit-unsupported'if unsupported else 'unexpected'
                        rows.append(dict(pattern=pattern,insensitive=insensitive,database_mode=dbmode,input_hex=data.hex(),equal=equal,disposition=disposition,**({}if equal else dict(reference_hex=ref.hex()if ref is not None else None,rust_hex=out.hex()if out is not None else None,rust_error=row.get('error')))))
                lib.magic_close(h)
    report=dict(schema_version=1,binary_sha256=hashlib.sha256((ROOT/args.probe).read_bytes()).hexdigest(),source_sha256=source_digest(),host=platform.platform(),libc=platform.libc_ver(),programs=programs,upstream_commit=json.loads((ROOT/'data/UPSTREAM.json').read_text())['commit'],environment=dict(LC_ALL='C',TZ='UTC'),flags=flags,patterns=len(patterns()),unique_inputs=len(inputs),total=len(rows),exact=sum(r['equal']for r in rows),compiler_errors=compiles,explicit_unsupported=sum(r['disposition']=='explicit-unsupported'for r in rows),unexpected=sum(r['disposition']=='unexpected'for r in rows),cases=rows)
    (ROOT/'reports/regex.json').write_text(json.dumps(report,indent=2)+'\n');print({k:v for k,v in report.items()if k not in {'cases','programs'}})
    for row in [r for r in rows if r['disposition']=='unexpected'][:20]:print(row)
    if report['unexpected']:raise SystemExit('Regex disagreements')
if __name__=='__main__':main()
