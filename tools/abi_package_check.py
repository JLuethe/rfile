#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Validate an isolated package, including offline rebuild from its source bundle."""
import argparse,hashlib,json,os,shlex,subprocess,tarfile,tempfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
def main():
 ap=argparse.ArgumentParser();ap.add_argument('package');a=ap.parse_args();package=Path(a.package).resolve();manifest=json.loads((package/'MANIFEST.json').read_text())
 for name,digest in manifest['sha256'].items():assert hashlib.sha256((package/name).read_bytes()).hexdigest()==digest,name
 for name,target in manifest['symlinks'].items():assert os.readlink(package/name)==target,name
 assert Path(manifest['prefix'])==package,'this test needs a package whose configured prefix is its actual isolated directory'
 report=dict(schema_version=1,package_manifest_sha256=hashlib.sha256((package/'MANIFEST.json').read_bytes()).hexdigest(),git_head=manifest['git_head'],source_sha256=manifest['source_sha256'],library_sha256=manifest['sha256']['lib/librfile_magic.so.1.0.0'],checks=[])
 with tempfile.TemporaryDirectory(prefix='rfile-package-check-')as tmp:
  tmp=Path(tmp);(tmp/'input').write_bytes(b'hello\n');env=dict(os.environ,PKG_CONFIG_LIBDIR=str(package/'lib/pkgconfig'),LC_ALL='C',TZ='UTC',HOME=str(tmp/'nonexistent'))
  for key in ['MAGIC','LD_PRELOAD','PKG_CONFIG_PATH']:env.pop(key,None)
  for static in [False,True]:
   flags=shlex.split(subprocess.check_output(['pkg-config',*(['--static']if static else[]),'--cflags','--libs','rfile-magic'],env=env,text=True))
   if static:flags=[str(package/'lib/librfile_magic.a')if x=='-lrfile_magic'else x for x in flags]
   subprocess.run(['cc','-Wall','-Wextra','-Werror','-DRFILE_OWNED_DATABASES=1',str(ROOT/'crates/rfile-ffi/tests/client.c'),*flags,'-Wl,-rpath,'+str(package/'lib'),'-o',str(tmp/'client')],env=env,check=True,capture_output=True)
   r=subprocess.run([str(tmp/'client'),str(package/'share/rfile/magic.mgc'),str(tmp/'input')],env=env,capture_output=True,text=True,timeout=60);report['checks'].append(dict(name='static-client'if static else'dynamic-client',status=r.returncode,stdout=r.stdout,stderr=r.stderr));assert r.returncode==0,r.stderr
  code='''import ctypes,json,sys
l=ctypes.CDLL(sys.argv[1]);l.magic_getpath.argtypes=[ctypes.c_char_p,ctypes.c_int];l.magic_getpath.restype=ctypes.c_char_p;l.magic_open.restype=ctypes.c_void_p;l.magic_load.argtypes=[ctypes.c_void_p,ctypes.c_char_p];l.magic_close.argtypes=[ctypes.c_void_p]
p=l.magic_getpath(None,0).decode();assert p==sys.argv[2];h=l.magic_open(0);s=l.magic_load(h,None);l.magic_close(h);assert s==0;print(json.dumps(dict(default_path=p,load_status=s)))'''
  r=subprocess.run(['python3','-c',code,str(package/'lib/librfile_magic.so'),str(package/'share/rfile/magic.mgc')],env=env,capture_output=True,text=True,timeout=60);assert r.returncode==0,r.stderr;report['default_discovery']=json.loads(r.stdout)
  with tarfile.open(package/'share/rfile/source.tar.gz')as archive:archive.extractall(tmp,filter='data')
  source=tmp/'rfile-source';cargo=os.environ.get('CARGO',str(Path.home()/'.cargo/bin/cargo'));build_env=dict(os.environ,CARGO_TARGET_DIR=str(tmp/'target'))
  with(ROOT/'reports/r8-source-package-rebuild.log').open('wb')as log:r=subprocess.run([cargo,'check','--offline','--locked','--workspace','--all-targets','--all-features'],cwd=source,env=build_env,stdout=log,stderr=subprocess.STDOUT)
  report['checks'].append(dict(name='offline-source-rebuild',status=r.returncode));assert r.returncode==0
 (ROOT/'reports/abi-package.json').write_text(json.dumps(report,indent=2)+'\n');print({k:v for k,v in report.items()if k!='source_sha256'})
if __name__=='__main__':main()
