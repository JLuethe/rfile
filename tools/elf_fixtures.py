#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Synthetic ELF writer; never executes or loads generated programs/core data."""
import struct

def note(name,kind,data,big=False,align=4):
    order='>' if big else '<'
    value=struct.pack(order+'III',len(name),len(data),kind)+name
    value+=b'\0'*(-len(value)%align)
    value+=data
    return value+b'\0'*(-len(value)%align)

def elf(programs=(),sections=(),is64=True,big=False,kind=2,machine=None):
    order='>' if big else '<';machine=machine or (62 if is64 else 3)
    eh,ph,sh=(64,56,64)if is64 else(52,32,40)
    image=bytearray(eh+ph*len(programs));headers=[]
    for typ,data,align,vaddr in programs:
        offset=len(image);image+=data
        if is64:header=struct.pack(order+'IIQQQQQQ',typ,4,offset,vaddr,vaddr,len(data),len(data),align)
        else:header=struct.pack(order+'IIIIIIII',typ,offset,vaddr,vaddr,len(data),len(data),4,align)
        headers.append(header)
    names=bytearray(b'\0');section_headers=[]
    if sections:
        section_headers.append(b'\0'*sh)
        for name,typ,data in sections:
            index=len(names);names+=name.encode()+b'\0';offset=len(image);image+=data
            if is64:header=struct.pack(order+'IIQQQQIIQQ',index,typ,0,0,offset,len(data),0,0,4,0)
            else:header=struct.pack(order+'IIIIIIIIII',index,typ,0,0,offset,len(data),0,0,4,0)
            section_headers.append(header)
        index=len(names);names+=b'.shstrtab\0';offset=len(image);image+=names
        fmt=order+('IIQQQQIIQQ'if is64 else'IIIIIIIIII')
        section_headers.append(struct.pack(fmt,index,3,0,0,offset,len(names),0,0,1,0))
    shoff=len(image)if sections else 0
    image+=b''.join(section_headers)
    ident=b'\x7fELF'+bytes([2 if is64 else 1,2 if big else 1,1,0])+b'\0'*8
    fields=(kind,machine,1,0,eh if programs else 0,shoff,0,eh,ph,len(programs),sh,len(section_headers),len(section_headers)-1 if sections else 0)
    header=ident+struct.pack(order+('HHIQQQIHHHHHH'if is64 else'HHIIIIIHHHHHH'),*fields)
    image[:eh]=header
    for i,header in enumerate(headers):image[eh+i*ph:eh+(i+1)*ph]=header
    if len(image)<=eh:image+=b'\0'
    return bytes(image)

def cases():
    result=[]
    for is64 in [False,True]:
      for big in [False,True]:
        order='>' if big else '<';word='Q'if is64 else'I';label=f'{64 if is64 else 32}-'+('be'if big else'le')
        versions=[('gnu',b'GNU\0',1,struct.pack(order+'IIII',0,6,8,3)),
                  ('freebsd-460002',b'FreeBSD\0',1,struct.pack(order+'I',460002)),
                  ('freebsd-460100',b'FreeBSD\0',1,struct.pack(order+'I',460100)),
                  ('freebsd-491000',b'FreeBSD\0',1,struct.pack(order+'I',491000)),
                  ('freebsd-1302500',b'FreeBSD\0',1,struct.pack(order+'I',1302500)),
                  ('netbsd-old',b'NetBSD\0',1,struct.pack(order+'I',800300000)),
                  ('netbsd-new',b'NetBSD\0',1,struct.pack(order+'I',1000150200)),
                  ('dragonfly',b'DragonFly\0',1,struct.pack(order+'I',600123)),
                  ('android',b'Android\0',1,struct.pack(order+'I',34)+b'r26d\0'.ljust(64,b'\0')+b'12345\0'.ljust(64,b'\0')),
                  ('pax',b'PaX\0',3,struct.pack(order+'I',63)),
                  ('memtag',b'Android\0',4,struct.pack(order+'I',31)),
                  ('netbsd-march',b'NetBSD\0',5,b'amd64\0'),
                  ('netbsd-cmodel',b'NetBSD\0',6,b'small\0'),
                  ('netbsd-emulation',b'NetBSD\0',2,b'linux\0')]
        for name,owner,typ,data in versions:
            result.append((f'{label}-{name}',elf([(4,note(owner,typ,data,big),4,0)],is64=is64,big=big)))
        duplicate=note(b'Go\0\0',4,b'first',big)+note(b'Go\0\0',4,b'second',big)
        result.append((f'{label}-duplicate-go',elf([(4,duplicate,4,0)],is64=is64,big=big)))
        args=bytearray(180);at=56 if is64 else 44;args[at:at+15]=b'rfile --probe \0'
        aux=b''.join(struct.pack(order+word*2,t,v)for t,v in [(31,0x400000),(15,0x400000+len(b'/usr/bin/rfile\0\0\0')),(11,1000),(12,1001),(13,1002),(14,1003),(0,0)])
        core=note(b'CORE\0',3,args,big)+note(b'CORE\0',6,aux,big)
        result.append((f'{label}-core-auxv',elf([(4,core,4,0),(1,b'/usr/bin/rfile\0\0\0x86_64\0',4,0x400000)],is64=is64,big=big,kind=4)))
        args=bytearray(160);at=33 if is64 else 25;args[at:at+6]=b'hello\0';struct.pack_into(order+'I',args,at+83,1234)
        result.append((f'{label}-freebsd-core',elf([(4,note(b'FreeBSD\0',3,args,big),4,0)],is64=is64,big=big,kind=4)))
        args=bytearray(160)
        for at,value in [(0,1),(4,160),(8,11),(12,2),(80,1234),(100,1000),(112,1001),(120,4),(156,3)]:struct.pack_into(order+'I',args,at,value)
        args[124:130]=b'hello\0'
        result.append((f'{label}-netbsd-core',elf([(4,note(b'NetBSD-CORE\0',1,args,big),4,0)],is64=is64,big=big,kind=4)))
        for align in [0,4,6,8,0x80000000]:
            data=note(b'GNU\0',3,bytes(range(20)),big,align if 4<=align<100 else 4)
            result.append((f'{label}-align-{align}',elf([(4,data,align,0)],is64=is64,big=big)))
        cap=struct.pack(order+word*2,1,0x1801)+struct.pack(order+word*2,2,3)
        result.append((f'{label}-sun-capabilities',elf(sections=[('.SUNW_cap',0x6ffffff5,cap)],is64=is64,big=big,kind=1)))
        result.append((f'{label}-section-notes',elf(sections=[('.note',7,note(b'GNU\0',3,bytes(range(20)),big)),('.debug_info',1,b'debug')],is64=is64,big=big,kind=1)))
    # Branch boundaries and hostile-but-bounded records supplement ordinary files.
    for version in [199905,100000000,100000001,899990000,900000100,0xffffffff]:
        result.append((f'netbsd-version-{version}',elf([(4,note(b'NetBSD\0',1,struct.pack('<I',version)),4,0)])))
    for version in [0,460001,460099,499999,500000,500100,500110,501000,2147483648,0xffffffff]:
        result.append((f'freebsd-version-{version}',elf([(4,note(b'FreeBSD\0',1,struct.pack('<I',version)),4,0)])))
    for desc in [4,8,16,20]:
        result.append((f'build-id-size-{desc}',elf([(4,note(b'GNU\0',3,bytes(range(desc))),4,0)])))
    result.append(('go-raw-bytes',elf([(4,note(b'Go\0\0',4,b'A\xff\tB'),4,0)])))
    result.append(('netbsd-raw-bytes',elf([(4,note(b'NetBSD\0',5,b'A\xff\tB'),4,0)])))
    result.append(('interpreter-long',elf([(3,b'/'+b'a'*1600+b'\0',4,0)])))
    result.append(('interpreter-controls',elf([(3,b'/ld\xff\t\0',4,0)])))
    dynamic=struct.pack('<QQQQ',0x6ffffffb,0x8000000,0x6ffffffb,0)
    result.append(('sticky-pie',elf([(2,dynamic,8,0)],kind=3)))
    result.append(('note-before-pie',elf([(4,note(b'GNU\0',3,b'12345678'),4,0),(2,struct.pack('<QQ',0x6ffffffb,0x8000000),8,0)],kind=3)))
    for machine,arch in [(62,'386'),(2,'sparc')]:
        import re
        header=(__import__('pathlib').Path(__file__).resolve().parents[1]/'upstream/src/readelf.h').read_text()
        constants=[(name,int(value,16))for name,value in re.findall(r'^#define\s+(AV_'+('386'if arch=='386'else'SPARC')+r'_\w+)\s+(0x[0-9a-fA-F]+)',header,re.M)]
        for name,value in constants:
            result.append(('cap-'+name,elf(sections=[('.SUNW_cap',0x6ffffff5,struct.pack('<QQ',1,value))],kind=1,machine=machine)))
    for name,entries in [('unknown',[(99,0),(98,1),(97,2),(96,3),(95,4)]),('leading-A',[(65,1)]),('null-not-terminator',[(0,0),(1,1)]),('sf-unused',[(2,2)]),('sf-unknown',[(2,8)]),('hw-unknown',[(1,1<<63)])]:
        result.append(('cap-'+name,elf(sections=[('.SUNW_cap',0x6ffffff5,b''.join(struct.pack('<QQ',*entry)for entry in entries))],kind=1)))
    for n in [255,256,257]:
        result.append((f'note-count-{n}',elf(sections=[('.note',7,note(b'Unknown\0',99,b'')*n)],kind=1)))
    for field,value in [('name',0x80000001),('description',0x80000001)]:
        data=struct.pack('<III',value if field=='name'else 4,value if field=='description'else 4,99)+b'abc\0data'
        result.append(('bad-note-'+field,elf([(4,data,4,0)])))
    for field,offset,value in [('phnum',56,65535),('shnum',60,65535)]:
        data=bytearray(elf([(4,note(b'GNU\0',3,b'12345678'),4,0)],sections=[('.debug_info',1,b'debug')]))
        struct.pack_into('<H',data,offset,value);result.append(('excess-'+field,bytes(data)))
    data=bytearray(elf([(4,note(b'GNU\0',3,b'12345678'),4,0)],sections=[('.debug_info',1,b'debug')]))
    shoff=struct.unpack_from('<Q',data,40)[0];count=struct.unpack_from('<H',data,60)[0];strings=struct.unpack_from('<H',data,62)[0]
    struct.pack_into('<H',data,56,65535);struct.pack_into('<H',data,60,0);struct.pack_into('<H',data,62,65535)
    struct.pack_into('<Q',data,shoff+32,count);struct.pack_into('<I',data,shoff+40,strings);struct.pack_into('<I',data,shoff+44,1)
    result.append(('extended-counts',bytes(data)))
    base=elf([(4,note(b'GNU\0',3,b'12345678'),4,0)])
    for name,field,fmt,value in [('program-short',32,'Q',4096),('program-size',54,'H',23)]:
        data=bytearray(base);struct.pack_into('<'+fmt,data,field,value);result.append((name,bytes(data)))
    base=elf(sections=[('.note',7,note(b'GNU\0',3,b'12345678')),('.debug_info',1,b'debug')],kind=1)
    shoff=struct.unpack_from('<Q',base,40)[0];strings=struct.unpack_from('<H',base,62)[0]
    for name,offset,fmt,value in [('section-short',40,'Q',4096),('section-size',58,'H',23),('names-offset',shoff+strings*64+24,'Q',4096),('note-size',shoff+64+32,'Q',4096)]:
        data=bytearray(base);struct.pack_into('<'+fmt,data,offset,value);result.append((name,bytes(data)))
    data=bytearray(elf(sections=[('.debug_info',1,b'debug')],kind=1));shoff=struct.unpack_from('<Q',data,40)[0]
    first=bytes(data[shoff+64:shoff+128]);last=bytes(data[shoff+128:shoff+192]);data[shoff+64:shoff+128]=last;data[shoff+128:shoff+192]=first;struct.pack_into('<H',data,62,1)
    result.append(('debug-last-header',bytes(data)))
    for name,magic in [('olf',b'\x7fOLF'),('invalid-class',b'\x7fELF')]:
        data=bytearray(base);data[:4]=magic
        if name=='invalid-class':data[4]=9
        result.append((name,bytes(data)))
    duplicate=note(b'GNU\0',1,struct.pack('<IIII',0,1,2,3))+note(b'FreeBSD\0',1,struct.pack('<I',1302500))
    result.append(('duplicate-os',elf([(4,duplicate,4,0)])))
    misc=note(b'NetBSD\0',99,b'a')+note(b'NetBSD\0',100,b'b')+note(b'NetBSD\0',2,b'first')+note(b'NetBSD\0',2,b'second')
    result.append(('duplicate-netbsd',elf([(4,misc,4,0)])))
    aux=struct.pack('<QQ',11,1000)*51
    result.append(('auxv-limit',elf([(4,note(b'CORE\0',6,aux),4,0)],kind=4)))
    return result
if __name__=='__main__':
    from pathlib import Path
    import json,hashlib
    root=Path(__file__).resolve().parents[1];directory=root/'tests/fixtures/elf';directory.mkdir(parents=True,exist_ok=True)
    manifest=[]
    for name,data in cases():
        (directory/(name+'.elf')).write_bytes(data);manifest.append(dict(name=name,bytes=len(data),sha256=hashlib.sha256(data).hexdigest()))
    (directory/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
    print(f'Generated {len(manifest)} ELF fixtures')
