#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Compile C/C++ clients against isolated native and Rust libraries; never install."""
import argparse,hashlib,json,os,subprocess,tempfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]

def main():
    ap=argparse.ArgumentParser();ap.add_argument('--library',default='target/debug/librfile_magic.so');ap.add_argument('--rust-only',action='store_true');args=ap.parse_args();library=(ROOT/args.library).resolve()
    if not args.rust_only:
        from reference import UP,manifest
        reference=manifest();db=UP/'magic/magic.mgc'
    else:reference=None;db=ROOT/'data/magic.mgc'
    report=dict(reference=reference,library_sha256=hashlib.sha256(library.read_bytes()).hexdigest(),scope='Header, linkage, C/C++ client ownership/state/concurrency, C ASan/UBSan (Rust allocations are not ASan-instrumented).',checks=[])
    with tempfile.TemporaryDirectory(prefix='rfile-abi-clients-')as tmp:
        tmp=Path(tmp);(tmp/'input').write_bytes(b'hello\n');(tmp/'librfile_magic.so.1').symlink_to(library)
        env=dict(os.environ,LC_ALL='C',TZ='UTC',ASAN_OPTIONS='detect_leaks=1:abort_on_error=1',UBSAN_OPTIONS='halt_on_error=1')
        for key in ['MAGIC','LD_PRELOAD']:env.pop(key,None)
        variants=[('rust',ROOT/'crates/rfile-ffi/include/rfile',library,tmp)]
        if not args.rust_only:variants.insert(0,('native',UP/'src',UP/'src/.libs/libmagic.so',UP/'src/.libs'))
        for name,header,lib,rpath in variants:
            for lang,sanitize in [('c',False),('c',True),('cpp',False)]:
                command=['c++'if lang=='cpp'else'cc','-Wall','-Wextra','-Werror','-pthread','-I'+str(header),str(ROOT/f'crates/rfile-ffi/tests/client.{lang}'),str(lib),'-Wl,-rpath,'+str(rpath),'-o',str(tmp/'client')]
                if name=='rust':command+=['-DRFILE_OWNED_DATABASES=1']
                if sanitize:command+=['-fsanitize=address,undefined','-fno-omit-frame-pointer']
                subprocess.run(command,check=True,capture_output=True)
                r=subprocess.run([str(tmp/'client'),str(db),str(tmp/'input')],capture_output=True,env=env,timeout=60)
                report['checks'].append(dict(library=name,language=lang,sanitizers=sanitize,status=r.returncode,stdout=r.stdout.decode(),stderr=r.stderr.decode()))
        symbols=subprocess.check_output(['nm','-D','--defined-only',str(library)],text=True);exports=[line.split()[-1]for line in symbols.splitlines()]
        report['exports']=exports
        report['dynamic']=subprocess.check_output(['readelf','-d',str(library)],text=True)
        report['expected_exports']=sorted(['magic_'+n for n in ['open','close','getpath','file','descriptor','buffer','error','getflags','setflags','version','load','load_buffers','compile','check','list','errno','setparam','getparam','getmaxparam']]+['rfile_implementation_version'])
    (ROOT/('reports/abi-clients-rust.json'if args.rust_only else'reports/abi-clients.json')).write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(dict(checks=len(report['checks']),passed=sum(c['status']==0 for c in report['checks']),exports=len(exports)),indent=2))
    assert all(c['status']==0 for c in report['checks']), 'client failed: see report'
    assert sorted(exports)==report['expected_exports'],'unexpected exports'
    assert 'librfile_magic.so.1'in report['dynamic'] and 'libmagic.so.'not in report['dynamic'],'bad SONAME/native detector dependency'
if __name__=='__main__':main()
