#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Separate-process native/Rust C ABI state comparisons. No detector FFI in production.
This evidence distinguishes exact observed cases from remaining semantic gaps.
"""
import argparse, ctypes as C, hashlib, json, os, subprocess, sys, tempfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
def worker(library,case,base):
    lib=C.CDLL(library,use_errno=True)
    signatures={
        'open':(C.c_void_p,[C.c_int]),'close':(None,[C.c_void_p]),
        'error':(C.c_void_p,[C.c_void_p]),'errno':(C.c_int,[C.c_void_p]),
        'getflags':(C.c_int,[C.c_void_p]),'setflags':(C.c_int,[C.c_void_p,C.c_int]),
        'getparam':(C.c_int,[C.c_void_p,C.c_int,C.c_void_p]),'setparam':(C.c_int,[C.c_void_p,C.c_int,C.c_void_p]),
        'getmaxparam':(C.c_ssize_t,[C.c_int]),'version':(C.c_int,[]),'getpath':(C.c_void_p,[C.c_void_p,C.c_int]),
        'buffer':(C.c_void_p,[C.c_void_p,C.c_void_p,C.c_size_t]),'file':(C.c_void_p,[C.c_void_p,C.c_char_p]),
        'descriptor':(C.c_void_p,[C.c_void_p,C.c_int]),'load_buffers':(C.c_int,[C.c_void_p,C.c_void_p,C.c_void_p,C.c_size_t]),
        **{n:(C.c_int,[C.c_void_p,C.c_char_p])for n in ['load','compile','check','list']}}
    f={}
    for name,(ret,args)in signatures.items():
        fn=getattr(lib,'magic_'+name);fn.restype=ret;fn.argtypes=args;f[name]=fn
    def text(p):return C.string_at(p).hex()if p else None
    h=f['open'](0); rows=[]
    def record(name,value,string=False):rows.append(dict(name=name,value=text(value)if string else value,error=text(f['error'](h)),cookie_errno=f['errno'](h),flags=f['getflags'](h)))
    def buf(data=b'ABC\n'):
        b=C.create_string_buffer(data);record('buffer',f['buffer'](h,b,len(data)),True)
    def load(name=b'magic.mgc'):record('load',f['load'](h,name))
    if case=='null':
        f['close'](None)
        for name,args in [('error',[]),('errno',[]),('getflags',[]),('setflags',[0]),('buffer',[None,0]),('file',[b'x']),('descriptor',[-1]),('load',[None]),('compile',[None]),('check',[None]),('list',[None]),('load_buffers',[None,None,0]),('getparam',[0,None]),('setparam',[0,None])]:
            C.set_errno(123);v=f[name](None,*args);rows.append(dict(name=name,value=text(v)if name in ['error','buffer','file','descriptor']else v,thread_errno=C.get_errno()))
    elif case=='params':
        rows.append(dict(version=f['version']()))
        for i in range(-1,11):
            C.set_errno(123);v=C.c_size_t(999);r=f['getparam'](h,i,C.byref(v));rows.append(dict(id=i,status=r,value=v.value,thread_errno=C.get_errno(),max=f['getmaxparam'](i)))
        for i,value in [(0,65536),(1,65537),(2,65538),(3,65539),(4,65540),(5,65541),(8,65536),(6,1024),(7,512),(9,2)]:
            v=C.c_size_t(value);record('setparam',f['setparam'](h,i,C.byref(v)));out=C.c_size_t();f['getparam'](h,i,C.byref(out));rows.append(dict(id=i,value=out.value))
    elif case=='state':
        buf();record('setflags',f['setflags'](h,16));buf();load();buf();record('zero-load',f['load_buffers'](h,None,None,0));buf();load(b'');buf();load();buf()
    elif case=='buffer-owner':
        data=Path('magic.mgc').read_bytes();storage=C.create_string_buffer(data);p=(C.c_void_p*1)(C.addressof(storage));n=(C.c_size_t*1)(len(data));record('load-buffers',f['load_buffers'](h,p,n,1));rows.append(dict(unchanged=storage.raw[:len(data)]==data));buf() # Native retains this allocation through close; Rust ownership is tested separately.
    elif case.startswith('buffer-invalid:'):
        data=bytearray(Path('magic.mgc').read_bytes());kind=case.split(':')[1]
        if kind=='empty':data=bytearray()
        elif kind=='header':data=data[:432]
        elif kind=='short':data=data[:2]
        elif kind=='trailing':data+=b'x'
        elif kind=='magic':data[0:4]=bytes(4)
        elif kind=='version':data[4:8]=(20).to_bytes(4,'little')
        elif kind=='count':data[8:12]=(999).to_bytes(4,'little')
        elif kind=='sum':data[8:12]=bytes(4)
        load();storage=C.create_string_buffer(bytes(data));p=(C.c_void_p*1)(C.addressof(storage));n=(C.c_size_t*1)(len(data));record('invalid-buffers',f['load_buffers'](h,p,n,1));buf();load();buf()
    elif case=='buffer-bytes':
        load(b'tail');v=C.c_size_t(1);record('limit',f['setparam'](h,6,C.byref(v)));buf(b'A'*20+b'TAIL');buf(b'')
    elif case.startswith('descriptor:'):
        load();offset=int(case.split(':')[1]);fd=os.open('input',os.O_RDONLY);os.lseek(fd,offset,0);record('fd',f['descriptor'](h,fd),True);rows.append(dict(position=os.lseek(fd,0,1),open=os.fstat(fd).st_size));os.close(fd)
    elif case=='descriptor-invalid':
        load();record('fd-minus1',f['descriptor'](h,-1),True);record('fd-minus2',f['descriptor'](h,-2),True);buf()
    elif case=='pipe':
        load();r,w=os.pipe();os.write(w,b'ABC\n');os.close(w);record('pipe',f['descriptor'](h,r),True);rows.append(dict(open=os.fstat(r).st_size));os.close(r)
    elif case.startswith('action:'):
        _,action,name=case.split(':',2);record('setflags',f['setflags'](h,64));record(action,f[action](h,os.fsencode(name)));buf()
    elif case.startswith('paths:'):
        load(os.fsencode(case[6:]));buf()
    elif case.startswith('warnings:'):
        v=C.c_size_t(int(case.split(':')[1]));f['setparam'](h,9,C.byref(v));f['setflags'](h,64)
        for _ in range(3):load(b'magic');buf()
    elif case=='path':
        for action in range(4):
            p=C.create_string_buffer(b'explicit');r=f['getpath'](C.addressof(p),action);rows.append(dict(identity=r==C.addressof(p)))
            os.environ['MAGIC']='';rows.append(dict(empty=text(f['getpath'](None,action))));os.environ['MAGIC']='custom:other';rows.append(dict(custom=text(f['getpath'](None,action))))
        del os.environ['MAGIC']
    elif case.startswith('file:'):
        load();record('file',f['file'](h,os.fsencode(case[5:])),True)
    else:raise ValueError(case)
    f['close'](h)
    Path(base).write_text(json.dumps(rows))
def main():
    ap=argparse.ArgumentParser();ap.add_argument('--worker',nargs=3);ap.add_argument('--library',default='target/debug/librfile_magic.so');a=ap.parse_args()
    if a.worker:return worker(*a.worker)
    from reference import UP,manifest
    library=(ROOT/a.library).resolve();reference=manifest()
    cases=[*[f'buffer-invalid:{k}'for k in ['empty','header','short','trailing','magic','version','count','sum']],'null','params','state','buffer-owner','buffer-bytes','descriptor:0','descriptor:1','descriptor-invalid','pipe','path',*[f'file:{n}'for n in ['input','empty','missing','link','directory']],*[f'action:{action}:{name}'for action in ['load','check','list','compile']for name in ['magic','magic.mgc','directory','malformed']],*[f'paths:{n}'for n in ['magic:missing','missing:magic','malformed:magic','magic:','',':magic']],*[f'warnings:{n}'for n in [0,1,2,3]]]
    rows=[]
    with tempfile.TemporaryDirectory(prefix='rfile-abi-')as tmp:
        tmp=Path(tmp);(tmp/'magic').write_bytes(b'0 string ABC sample\n!:mime application/x-sample\n');(tmp/'tail').write_bytes(b'20 string TAIL tail-signature\n');(tmp/'malformed').write_bytes(b'0 bogus x malformed\n');(tmp/'input').write_bytes(b'ABC\n');(tmp/'empty').touch();(tmp/'link').symlink_to('input');(tmp/'directory').mkdir();(tmp/'directory/a').write_bytes(b'0 string ABC sample\n');(tmp/'directory/b').write_bytes(b'0 string ABCDEFG stronger\n')
        env=dict(os.environ,LC_ALL='C',TZ='UTC',HOME=str(tmp/'home'))
        for k in ['MAGIC','POSIXLY_CORRECT','LD_PRELOAD']:env.pop(k,None)
        subprocess.run([str(UP/'src/file'),'-S','-C','-m','magic'],cwd=tmp,env=env,check=True,capture_output=True)
        original=(tmp/'magic.mgc').read_bytes()
        for case in cases:
            results=[]
            for name,lib in [('c',UP/'src/.libs/libmagic.so'),('rust',library)]:
                (tmp/'magic.mgc').write_bytes(original)
                for p in ['tail.mgc','directory.mgc','malformed.mgc']:
                    (tmp/p).unlink(missing_ok=True)
                out=tmp/(name+'.json');out.unlink(missing_ok=True)
                r=subprocess.run([sys.executable,str(Path(__file__).resolve()),'--worker',str(lib),case,str(out)],cwd=tmp,env=env,capture_output=True,timeout=30)
                artifact={p:hashlib.sha256((tmp/p).read_bytes()).hexdigest()for p in ['magic.mgc','magic.mgc.mgc','directory.mgc','malformed.mgc']if(tmp/p).exists()}
                results.append(dict(status=r.returncode,stdout_hex=r.stdout.hex(),stderr_hex=r.stderr.hex(),state=json.loads(out.read_text())if out.exists()else None,artifacts=artifact))
            row=dict(name=case,exact=results[0]==results[1]);
            if not row['exact']:row.update(reference=results[0],rust=results[1])
            rows.append(row)
    report=dict(schema_version=1,scope='Experimental ABI state/action comparison; discrepancies remain explicit, not a drop-in certification.',reference=reference,library_sha256=hashlib.sha256(library.read_bytes()).hexdigest(),total=len(rows),exact=sum(r['exact']for r in rows),cases=rows)
    known=json.loads((ROOT/'data/abi-known-gaps.json').read_text())
    unexpected=[]
    for row in rows:
        if row['exact']:continue
        observed=hashlib.sha256(json.dumps([row['reference'],row['rust']],sort_keys=True,separators=(',',':')).encode()).hexdigest()
        gap=known.get(row['name'])
        if not gap or gap['observed_sha256']!=observed:unexpected.append(row['name'])
        else:row['known_gap']=gap['reason']
    report['unexpected']=unexpected
    (ROOT/'reports/abi-state.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({k:v for k,v in report.items()if k not in ['reference','cases']},indent=2));print('Differences:',[r['name']for r in rows if not r['exact']]);
    if unexpected:raise SystemExit('Unexpected ABI discrepancy: '+repr(unexpected))
if __name__=='__main__':main()
