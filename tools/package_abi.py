#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Build an isolated C ABI package tree. Does not install at --prefix or publish."""
import argparse,ctypes,hashlib,json,os,shutil,subprocess,tempfile,tarfile,gzip,tomllib
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
def main():
    ap=argparse.ArgumentParser();ap.add_argument('--output',required=True);ap.add_argument('--prefix',required=True);ap.add_argument('--extended-codecs',action='store_true');a=ap.parse_args()
    output=Path(a.output).resolve();prefix=Path(a.prefix)
    if not prefix.is_absolute() or '\n'in str(prefix) or '\0'in str(prefix):raise SystemExit('Prefix must be an absolute path without line/NUL bytes')
    if output.exists():raise SystemExit('Output already exists; choose a new package directory')
    if subprocess.check_output(['git','status','--porcelain','--untracked-files=all'],cwd=ROOT):
        raise SystemExit('Release packaging requires a clean committed checkout; use an isolated clone')
    version=tomllib.loads((ROOT/'Cargo.toml').read_text())['package']['version']
    cargo=os.environ.get('CARGO',str(Path.home()/'.cargo/bin/cargo'))
    features=['--features','extended-codecs']if a.extended_codecs else[]
    with tempfile.TemporaryDirectory(prefix='rfile-abi-package-')as tmp:
        target=Path(tmp)/'target';env=dict(os.environ,RFILE_MAGIC_PATH=str(prefix/'share/rfile/magic.mgc'),CARGO_TARGET_DIR=str(target))
        subprocess.run([cargo,'build','-p','rfile-ffi','--release','--locked',*features],cwd=ROOT,env=env,check=True)
        stage=Path(tmp)/'package'
        for folder in ['lib/pkgconfig','include/rfile','share/rfile','share/licenses/rfile']:(stage/folder).mkdir(parents=True,exist_ok=True)
        shutil.copy2(target/'release/librfile_magic.so',stage/'lib/librfile_magic.so.1.0.0')
        (stage/'lib/librfile_magic.so.1').symlink_to('librfile_magic.so.1.0.0');(stage/'lib/librfile_magic.so').symlink_to('librfile_magic.so.1')
        shutil.copy2(target/'release/librfile_magic.a',stage/'lib/librfile_magic.a')
        shutil.copy2(ROOT/'crates/rfile-ffi/include/rfile/magic.h',stage/'include/rfile/magic.h')
        shutil.copy2(ROOT/'data/magic.mgc',stage/'share/rfile/magic.mgc')
        for name in ['UPSTREAM.json','UPSTREAM-COPYING','SOURCES.json']:shutil.copy2(ROOT/'data'/name,stage/'share/rfile'/name)
        shutil.copytree(ROOT/'data/magic-source',stage/'share/rfile/magic-source')
        for name in ['LICENSE','NOTICE','THIRD_PARTY_LICENSES.md']:
            if(ROOT/name).exists():shutil.copy2(ROOT/name,stage/'share/licenses/rfile'/name)
        for name in ['README.md'] :shutil.copy2(ROOT/'crates/rfile-ffi'/name,stage/'share/rfile'/name)
        shutil.copytree(ROOT/'LICENSES',stage/'share/licenses/rfile/LICENSES')
        shutil.copy2(ROOT/'docs/remaining/07-c-abi.md',stage/'share/rfile/C-ABI-SPEC.md')
        (stage/'lib/pkgconfig/rfile-magic.pc').write_text(f'''prefix={prefix}
exec_prefix=${{prefix}}
libdir=${{prefix}}/lib
includedir=${{prefix}}/include/rfile

Name: rfile-magic
Description: Experimental independent Rust file classifier C interface
Version: {version}
Libs: -L${{libdir}} -lrfile_magic
Libs.private: -ldl -lpthread -lm
Cflags: -I${{includedir}}
''')
        lib=ctypes.CDLL(str(stage/'lib/librfile_magic.so'));lib.rfile_implementation_version.restype=ctypes.c_char_p
        capability=json.loads(lib.rfile_implementation_version());capability.update(extended_codecs=a.extended_codecs,configured_database=str(prefix/'share/rfile/magic.mgc'))
        (stage/'share/rfile/capabilities.json').write_text(json.dumps(capability,indent=2)+'\n')
        # Include a self-contained build source snapshot and vendored crates,
        # with deterministic archive ownership/timestamps. No ignored checkout,
        # credentials, host corpus, logs or build artifacts enter the archive.
        source=Path(tmp)/'source';source.mkdir()
        paths=subprocess.check_output(['git','ls-files','--cached','-z'],cwd=ROOT).split(b'\0')
        accepted=[]
        for raw in paths:
            if not raw:continue
            name=os.fsdecode(raw);path=Path(name)
            allowed_dirs={'src','crates','tests','examples','data','tools','docs','LICENSES','.github','fuzz','packaging'}
            allowed_files={'Cargo.toml','Cargo.lock','LICENSE','NOTICE','README.md','SECURITY.md','HANDOFF.md','.gitignore','reports/remaining-spec-observations.json'}
            if path.parts[0]not in allowed_dirs and name not in allowed_files:continue
            if not(ROOT/path).is_file() or (ROOT/path).is_symlink():continue
            dest=source/path;dest.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(ROOT/path,dest);accepted.append(path)
        subprocess.run([cargo,'vendor','--locked','--sync','fuzz/Cargo.toml',str(source/'vendor')],cwd=ROOT,check=True,stdout=subprocess.DEVNULL)
        (source/'.cargo').mkdir(exist_ok=True)
        (source/'.cargo/config.toml').write_text('[source.crates-io]\nreplace-with = "vendored-sources"\n[source.vendored-sources]\ndirectory = "vendor"\n')
        source_hashes={str(p):hashlib.sha256((source/p).read_bytes()).hexdigest()for p in sorted(accepted)}
        epoch=int(subprocess.check_output(['git','show','-s','--format=%ct','HEAD'],cwd=ROOT,text=True))
        with(stage/'share/rfile/source.tar.gz').open('wb')as output_archive:
            with gzip.GzipFile(filename='',mode='wb',fileobj=output_archive,mtime=0)as gz:
                with tarfile.open(fileobj=gz,mode='w')as archive:
                    for p in sorted(source.rglob('*')):
                        info=archive.gettarinfo(str(p),arcname='rfile-source/'+str(p.relative_to(source)));info.uid=info.gid=0;info.uname=info.gname='';info.mtime=epoch
                        if p.is_file():
                            with p.open('rb')as f:archive.addfile(info,f)
                        else:archive.addfile(info)
        files={str(p.relative_to(stage)):hashlib.sha256(p.read_bytes()).hexdigest()for p in sorted(stage.rglob('*'))if p.is_file()and not p.is_symlink()}
        manifest=dict(schema_version=1,source_sha256=source_hashes,git_head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),tracked_worktree_dirty=bool(subprocess.check_output(['git','status','--porcelain','--untracked-files=no'],cwd=ROOT)),prefix=str(prefix),features=features,sha256=files,symlinks={str(p.relative_to(stage)):os.readlink(p)for p in stage.rglob('*')if p.is_symlink()},rustc=subprocess.check_output([str(Path(cargo).with_name('rustc')),'--version'],text=True).strip(),status='experimental package tree; not installed or published')
        (stage/'MANIFEST.json').write_text(json.dumps(manifest,indent=2)+'\n');output.parent.mkdir(parents=True,exist_ok=True);shutil.copytree(stage,output,symlinks=True)
    print(output)
if __name__=='__main__':main()
