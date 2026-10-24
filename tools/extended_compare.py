#!/usr/bin/env python3
import importlib.util,json,pathlib,subprocess,time
ROOT=pathlib.Path(__file__).resolve().parents[1];spec=importlib.util.spec_from_file_location('diff',ROOT/'tools/differential.py');mod=importlib.util.module_from_spec(spec);spec.loader.exec_module(mod)
lib=mod.library();files=[p for p in mod.corpus(4000) if p.parent!=mod.UP/'tests' or not list(p.parent.glob(p.name.removesuffix('.testfile')+'*.magic'))];files=list(dict.fromkeys(files[:250]+[p for p in files if str(p).startswith('/usr/bin/')][:150]+[p for p in files if str(p).startswith('/usr/share/')][:100]))
rows=[];summary=[]
for flags in [0x20,0x430,0x1000000,0x800,0x2000,0x4000,0x20000,0x200000,0x20020,0x40420,0x2420]:
 for mode in ['buffer','path']:
  part=mod.run_group(lib,files,mod.DB,flags,mode);rows.extend(part);s={'flags':flags,'mode':mode,'total':len(part),'equal':sum(x['equal']for x in part),'errors':sum(x.get('rust_error')is not None for x in part)};summary.append(s);print(json.dumps(s),flush=True)
(ROOT/'reports/extended.json').write_text(json.dumps({'summary':summary,'comparisons':rows},indent=2)+'\n')
