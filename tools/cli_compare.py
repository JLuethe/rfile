#!/usr/bin/env python3
"""Capture stdout, stderr and status for generated CLI cases against the pinned C binary."""
import itertools, json, os, pathlib, subprocess, tempfile, time, gzip, socket
from reference import UP
ROOT=pathlib.Path(__file__).resolve().parents[1];DB=UP/'magic/magic.mgc';ENV=dict(os.environ,LC_ALL='C',TZ='UTC');ENV.pop('MAGIC',None);ENV.pop('POSIXLY_CORRECT',None)
def invoke(binary,flags,args,data=None,env=None):
 r=subprocess.run([str(binary),'-m',str(DB),*flags,*map(str,args)],input=data,capture_output=True,env=env or ENV,timeout=20)
 return {'stdout':r.stdout.hex(),'stderr':r.stderr.hex(),'status':r.returncode}
rows=[];start=time.perf_counter()
with tempfile.TemporaryDirectory(prefix='rfile-cli-')as temp:
 d=pathlib.Path(temp);text=d/'text';text.write_bytes(b'hello\n');empty=d/'empty';empty.touch();binary=d/'binary';binary.write_bytes(b'\0\xff\0\xff');long=d/'longer-name';long.write_bytes(b'hi\r\n');missing=d/'missing';link=d/'link';link.symlink_to('text');broken=d/'broken';broken.symlink_to('absent');fifo=d/'fifo';os.mkfifo(fifo);sockpath=d/'socket';sock=socket.socket(socket.AF_UNIX);sock.bind(str(sockpath))
 weird=[]
 for suffix in [' trailing ', 'carriage\r', 'line\n', 'tab\t', 'utf8-é', 'invalid-\udcff']:
  p=d/suffix;p.write_bytes(b'hello\n');weird.append(p)
 weirdlist=d/'weird-list';weirdlist.write_bytes(b''.join(os.fsencode(p)+b'\n' for p in weird if '\n' not in p.name))
 archive=d/'compressed.gz';archive.write_bytes(gzip.compress(b'hello\n',mtime=0));listing=d/'list';listing.write_text(str(text)+'\n'+str(long)+'\n');blank=d/'blank';blank.touch()
 cases=[]
 for flags in [[],['-b'],['-bb'],['-bbb'],['-N'],['-F','|'],['-0'],['-00'],['-b','-00'],['-n'],['-i'],['--mime-type'],['--mime-encoding'],['--extension'],['--apple'],['-r'],['-k'],['-ki'],['-e','soft'],['-e','text'],['-e','encoding'],['--exclude-quiet','bogus']]:cases.append((flags,[text,long],None,None))
 for name in [empty,binary,missing,link,broken,fifo,sockpath,d]:
  for flags in [[],['-b'],['-i'],['-E'],['-L'],['--extension'],['--apple']]:cases.append((flags,[name],None,None))
 for flags in [[],['-i'],['-0'],['-00'],['-b'],['-P','bytes=1']]:cases.append((flags,['-'],b'hello\n',None))
 for flags in [[],['-n'],['-i'],['-0']]:cases.append(([*flags,'-f',str(listing)],[],None,None))
 cases.extend([(['-f',str(blank)],[],None,None),(['-f','-'],[],(str(text)+'\n').encode(),None),(['-z','-b'],[archive],None,None),(['-Z','-b'],[archive],None,None),(['-zi','-b'],[archive],None,None),(['-Zi','-b'],[archive],None,None),(['-S','-z','-b'],[archive],None,None),(['-S','-Z','-b'],[archive],None,None),(['-S','-zi','-b'],[archive],None,None),(['-S','-Zi','-b'],[archive],None,None),(['-b'],[link],None,dict(ENV,POSIXLY_CORRECT='1')),(['-p'],[text],None,None)])
 for flags in [[],['-r'],['-0'],['-00'],['-b']]:cases.append((flags,weird,None,None))
 for flags in [[],['-n'],['-r']]:cases.append(([*flags,'-f',str(weirdlist)],[],None,None))
 for index,(flags,args,data,env)in enumerate(cases):
  a=invoke(UP/'src/file',flags,args,data,env);b=invoke(ROOT/'target/release/rfile',flags,args,data,env);equal=a==b
  intentional = 'Rust native gzip decoding is independent of the C sandbox/native-decoder guard' if archive in args and '-S' not in flags else 'C annotation-only FIFO reaches a sandbox-killed syscall; Rust rejects content inspection' if fifo in args and any(x in flags for x in ['--apple','--extension']) else None
  rows.append({'intentional_difference':intentional,'case':index,'flags':flags,'args':[str(x).replace(str(d),'<TEMP>')for x in args],'equal':equal,**({}if equal else{'reference':{k:bytes.fromhex(v).decode('utf8','backslashreplace')if k!='status'else v for k,v in a.items()},'rust':{k:bytes.fromhex(v).decode('utf8','backslashreplace')if k!='status'else v for k,v in b.items()}})})
 sock.close()
report={'total':len(rows),'equal':sum(r['equal']for r in rows),'elapsed_seconds':time.perf_counter()-start,'environment':{'LC_ALL':'C','TZ':'UTC'},'cases':rows};(ROOT/'reports/cli.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({k:v for k,v in report.items()if k!='cases'}))
for r in rows:
 if not r['equal']:print(json.dumps(r))
