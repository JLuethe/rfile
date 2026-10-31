import hashlib, json, os, shutil, subprocess, sys, tarfile
from pathlib import Path
base=Path(__file__).resolve().parents[1]
archive=base/'candidate/rfile-0.2.0-rc.1-x86_64-unknown-linux-gnu.tar.gz'
work=base/'tmp/installer-independent'; shutil.rmtree(work,ignore_errors=True); work.mkdir(parents=True)
with tarfile.open(archive) as t: t.extractall(work,filter='data')
root=work/'rfile-0.2.0-rc.1-x86_64-unknown-linux-gnu'; script=root/'install.py'; results=[]
env=dict(os.environ, LC_ALL='C', TZ='UTC', TMPDIR=str(base/'tmp'))
env.pop('MAGIC',None)
def run(name, action, prefix, expected=0):
 p=subprocess.run([sys.executable,str(script),action,'--prefix',str(prefix)],env=env,capture_output=True,text=True)
 results.append({'name':name,'status':p.returncode,'stdout':p.stdout,'stderr':p.stderr})
 assert (p.returncode==0)==(expected==0),(name,p.returncode,p.stdout,p.stderr)
 return p
# absolute prefix and spaces, then unrelated file and changed payload protection
prefix=work/'absolute prefix with spaces'; run('absolute-space-prefix-install','install',prefix)
assert (prefix/'bin/rfile').is_file() and (prefix/'share/man/man1/rfile.1').is_file()
(prefix/'unrelated').write_text('keep')
original=(prefix/'bin/rfile').read_bytes(); (prefix/'bin/rfile').write_bytes(original+b'changed')
run('changed-payload-uninstall-refused','uninstall',prefix,1)
assert (prefix/'share/man/man1/rfile.1').exists() and (prefix/'unrelated').read_text()=='keep'
(prefix/'bin/rfile').write_bytes(original); run('uninstall-clean','uninstall',prefix)
assert not (prefix/'bin/rfile').exists() and (prefix/'unrelated').read_text()=='keep'
# relative prefix rejected
run('relative-prefix-rejected','install',Path('relative-prefix'),2)
# Pre-existing destination preserved, no other payload or receipt created
pre=work/'preexisting'; (pre/'bin').mkdir(parents=True); (pre/'bin/rfile').write_text('owner')
run('pre-existing-destination-refused','install',pre,1)
assert (pre/'bin/rfile').read_text()=='owner' and not (pre/'share/rfile/install-receipt.json').exists()
# Symlinked destination component must not redirect writes
sym=work/'symlink-prefix'; outside=work/'outside'; sym.mkdir(); outside.mkdir(); (sym/'bin').symlink_to(outside,target_is_directory=True)
run('symlink-component-refused','install',sym,1); assert not list(outside.iterdir())
# Payload byte tampering rejected and no prefix writes
bad=work/'bad-payload'; shutil.copytree(root,bad); (bad/'share/licenses/rfile/LICENSE').write_bytes(b'changed')
badscript=bad/'install.py'; p=subprocess.run([sys.executable,str(badscript),'install','--prefix',str(work/'tampered-prefix')],env=env,capture_output=True,text=True)
results.append({'name':'tampered-payload-refused','status':p.returncode,'stdout':p.stdout,'stderr':p.stderr}); assert p.returncode!=0 and not (work/'tampered-prefix').exists()
# Manifest digest tampering rejected as mismatch
badm=work/'bad-manifest'; shutil.copytree(root,badm); mp=badm/'MANIFEST.json'; md=json.loads(mp.read_text()); md['files']['bin/rfile']['sha256']='0'*64; mp.write_text(json.dumps(md))
p=subprocess.run([sys.executable,str(badm/'install.py'),'install','--prefix',str(work/'manifest-prefix')],env=env,capture_output=True,text=True)
results.append({'name':'manifest-checksum-tampering-refused','status':p.returncode,'stdout':p.stdout,'stderr':p.stderr}); assert p.returncode!=0 and not (work/'manifest-prefix').exists()
# Prefix itself may be reached by symlink: resolved destination should remain explicit real path
real=work/'real-prefix'; real.mkdir(); alias=work/'prefix-alias'; alias.symlink_to(real,target_is_directory=True)
run('prefix-alias-resolved','install',alias); assert (real/'bin/rfile').is_file(); run('prefix-alias-uninstall','uninstall',alias); assert not (real/'bin/rfile').exists()
(base/'evidence/independent-installer-results.json').write_text(json.dumps({'checks':results,'count':len(results),'status':'passed'},indent=2)+'\n')
print(json.dumps({'status':'passed','checks':len(results)}))
