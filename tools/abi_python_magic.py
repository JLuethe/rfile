#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Exercise pinned python-magic in a separate process with an isolated loader alias."""
import argparse,hashlib,json,os,subprocess,tempfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
PROGRAM=r'''
import ctypes,hashlib,importlib.metadata,json,magic,os,sys
assert importlib.metadata.version('python-magic')=='0.4.27'
# This symbol proves the loader selected Rust, rather than system libmagic.
magic.libmagic.rfile_implementation_version.restype=ctypes.c_char_p
implementation=json.loads(magic.libmagic.rfile_implementation_version())
assert implementation['implementation']=='rfile'
rows=[]
for mime in [False,True]:
 m=magic.Magic(magic_file=sys.argv[1],mime=mime)
 rows.append(dict(mime=mime,buffer=m.from_buffer(b'hello\n'),file=m.from_file(sys.argv[2])))
 with open(sys.argv[2],'rb')as f:
  f.seek(1);out=m.from_descriptor(f.fileno());assert f.tell()==1
  rows[-1]['descriptor']=out
 assert rows[-1]['buffer']==rows[-1]['file']==rows[-1]['descriptor']
 if mime:assert rows[-1]['buffer']=='text/plain'
 del m
print(json.dumps(dict(package_version=importlib.metadata.version('python-magic'),implementation=implementation,rows=rows,module_sha256=hashlib.sha256(open(magic.__file__,'rb').read()).hexdigest())))
'''
def main():
 ap=argparse.ArgumentParser();ap.add_argument('--python',default='.reference/abi-python/bin/python');ap.add_argument('--library',default='target/debug/librfile_magic.so');a=ap.parse_args();library=(ROOT/a.library).resolve()
 with tempfile.TemporaryDirectory(prefix='rfile-python-client-')as tmp:
  tmp=Path(tmp);(tmp/'libmagic.so.1').symlink_to(library);(tmp/'input').write_bytes(b'hello\n')
  env=dict(os.environ,LD_LIBRARY_PATH=str(tmp),LC_ALL='C',TZ='UTC');env.pop('LD_PRELOAD',None);env.pop('MAGIC',None)
  r=subprocess.run([str((ROOT/a.python).absolute()),'-c',PROGRAM,str(ROOT/'data/magic.mgc'),str(tmp/'input')],env=env,capture_output=True,text=True,timeout=60)
  report=dict(scope='Pinned python-magic consumer using local loader alias; no system install. Consumer tests only, not universal client compatibility.',library_sha256=hashlib.sha256(library.read_bytes()).hexdigest(),status=r.returncode,stderr=r.stderr,result=json.loads(r.stdout)if r.returncode==0 else r.stdout)
 (ROOT/'reports/abi-python-magic.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2));assert r.returncode==0
if __name__=='__main__':main()
