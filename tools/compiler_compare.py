#!/usr/bin/env python3
"""Compile the exact upstream fragment list with both engines and compare wire records."""
import collections, hashlib, json, os, pathlib, re, shutil, struct, subprocess, tempfile
ROOT=pathlib.Path(__file__).resolve().parents[1];UP=ROOT/'upstream';ENV=dict(os.environ,LC_ALL='C',TZ='UTC');ENV.pop('MAGIC',None)
text=(UP/'magic/Makefile.am').read_text();fragment_text=text.split('MAGIC_FRAGMENTS =',1)[1].split('EXTRA_DIST',1)[0]
files=re.findall(r'\$\(MAGIC_DIR\)/([^\s\\]+)|\$\(MAGIC_FRAGMENT_DIR\)/([^\s\\]+)',fragment_text)
paths=[UP/'magic'/a if a else UP/'magic/Magdir'/b for a,b in files]
def records(b,normalize=False):
 out=[]
 for p in range(432,len(b),432):
  r=bytearray(b[p:p+432])
  if normalize:
   r[20:24]=b'\0'*4
   if r[160]==0:r[161:224]=b'\0'*63
  out.append(bytes(r))
 return out
with tempfile.TemporaryDirectory(prefix='rfile-compiler-')as d:
 d=pathlib.Path(d);magic=d/'magic';magic.mkdir()
 for p in paths:shutil.copy2(p,magic/p.name)
 compiled=[]
 for binary in [UP/'src/file',ROOT/'target/release/rfile']:
  result=subprocess.run([str(binary),'-C','-m','magic'],cwd=d,env=ENV,capture_output=True,timeout=60)
  if result.returncode:raise RuntimeError(result.stderr.decode(errors='replace'))
  compiled.append((d/'magic.mgc').read_bytes())
 a,b=compiled;aa=records(a,True);bb=records(b,True)
 report={'upstream_commit':subprocess.check_output(['git','-C',str(UP),'rev-parse','HEAD'],text=True).strip(),'fragments':len(paths),'reference_header':struct.unpack_from('<4I',a),'rust_header':struct.unpack_from('<4I',b),'raw_equal':a==b,'reference_sha256':hashlib.sha256(a).hexdigest(),'rust_sha256':hashlib.sha256(b).hexdigest(),'normalization':'line numbers zero; bytes after an empty description NUL zero (debug path only)','normalized_order_equal':aa==bb,'normalized_position_equal':sum(x==y for x,y in zip(aa,bb)),'reference_only_records':sum((collections.Counter(aa)-collections.Counter(bb)).values()),'rust_only_records':sum((collections.Counter(bb)-collections.Counter(aa)).values())}
 (ROOT/'reports/compiler.json').write_text(json.dumps(report,indent=2)+'\n');(ROOT/'reports/rust-compiled.mgc').write_bytes(b)
 differences=[]
 ca=collections.Counter(aa)-collections.Counter(bb);cb=collections.Counter(bb)-collections.Counter(aa)
 for rec,n in ca.most_common(8):differences.append({'reference_only':rec.hex(),'count':n})
 for rec,n in cb.most_common(8):differences.append({'rust_only':rec.hex(),'count':n})
 (ROOT/'reports/compiler-differences.json').write_text(json.dumps(differences,indent=2)+'\n');print(json.dumps(report,indent=2))
