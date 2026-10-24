#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Independent CFB fixture writer: fixed records and sector maps, no rfile parser."""
import struct
END, FREE, FAT, DIFAT = 0xfffffffe, 0xffffffff, 0xfffffffd, 0xfffffffc
SUMMARY = bytes.fromhex('e0859ff2f94f6810ab9108002b27b3d9')
DOCSUMMARY = bytes.fromhex('02d5cdd59c2e1b10939708002b2cf9ae')

def typed(kind, value):
    return struct.pack('<I', kind) + value

def section(properties):
    payload = bytearray()
    table = bytearray()
    start = 8 + 8*len(properties)
    for number, value in properties:
        table += struct.pack('<II', number, start+len(payload))
        payload += value
        payload += b'\0' * (-len(payload) % 4)
    return struct.pack('<II', start+len(payload), len(properties))+table+payload

def property_stream(properties, extra_sets=()):
    sets = [(SUMMARY, section(properties)), *extra_sets]
    offset = 28 + len(sets)*20
    declarations, payload = bytearray(), bytearray()
    for fmtid, value in sets:
        declarations += fmtid + struct.pack('<I', offset+len(payload))
        payload += value
    return struct.pack('<HHI', 0xfffe, 0, 0x00020006) + b'\0'*16 + struct.pack('<I', len(sets)) + declarations+payload

def text(value):
    value = value.encode('cp1252')+b'\0'
    return typed(30, struct.pack('<I', len(value))+value)

def compound(streams, major=3, mini=False, gap=0, root=True, clsid=b'\0'*16):
    sector = 512 if major == 3 else 4096
    entries = []
    regular = []
    mini_data = bytearray()
    mini_links = []
    for name, value in streams:
        if mini and len(value)<4096:
            first = len(mini_links)
            chunks = (len(value)+63)//64
            for i in range(chunks): mini_links.append(first+i+1 if i+1<chunks else END)
            mini_data += value + b'\0'*(chunks*64-len(value))
            entries.append((name, 2, first if chunks else END, len(value)))
        else:
            value = value + b'\0'*max(0, 4096-len(value))
            entries.append((name, 2, len(regular), len(value)))
            regular.append(value)
    directory_count = (len(entries)+(1 if root else 0)+sector//128-1)//(sector//128)
    directory_count = max(1, directory_count)
    sectors = [bytearray(sector) for _ in range(directory_count + gap)]
    links = [FREE]*len(sectors)
    for i in range(directory_count): links[i] = i+1 if i+1<directory_count else END
    def add(data):
        first = len(sectors)
        count = (len(data)+sector-1)//sector
        for i in range(count):
            sectors.append(bytearray(data[i*sector:(i+1)*sector]).ljust(sector,b'\0'))
            links.append(first+i+1 if i+1<count else END)
        return first if count else END
    locations = [add(value) for value in regular]
    root_first = add(mini_data)
    mini_first = add(b''.join(struct.pack('<I', v) for v in mini_links))
    mini_count = (len(mini_links)*4+sector-1)//sector
    fat_count = 1
    difat_count = 0
    while True:
        next_difat = max(0, (fat_count-109+(sector//4-2))//(sector//4-1))
        next_fat = (len(sectors)+fat_count+next_difat+sector//4-1)//(sector//4)
        if (next_fat,next_difat) == (fat_count,difat_count): break
        fat_count,difat_count=next_fat,next_difat
    fat_ids = list(range(len(sectors), len(sectors)+fat_count))
    difat_ids = list(range(len(sectors)+fat_count, len(sectors)+fat_count+difat_count))
    all_links = links + [FAT]*fat_count + [DIFAT]*difat_count
    all_links += [FREE]*(fat_count*(sector//4)-len(all_links))
    for i in range(fat_count): sectors.append(bytearray(struct.pack('<'+'I'*(sector//4), *all_links[i*(sector//4):(i+1)*(sector//4)])))
    remaining = fat_ids[109:]
    for i in range(difat_count):
        ids = remaining[i*(sector//4-1):(i+1)*(sector//4-1)]
        ids += [FREE]*(sector//4-1-len(ids))
        ids.append(difat_ids[i+1] if i+1<difat_count else END)
        sectors.append(bytearray(struct.pack('<'+'I'*(sector//4), *ids)))
    records = []
    if root: records.append(('Root Entry',5,root_first,len(mini_data),clsid))
    for name,kind,first,size in entries:
        if not mini or size>=4096: first=locations[first]
        records.append((name,kind,first,size,b'\0'*16))
    for i,(name,kind,first,size,guid) in enumerate(records):
        entry=bytearray(128)
        encoded=name.encode('utf-16-le')+b'\0\0'
        assert len(encoded)<=64
        entry[:len(encoded)]=encoded
        struct.pack_into('<HBBIII',entry,64,len(encoded),kind,1,FREE,i+1 if i+1<len(records) and kind!=5 else FREE,1 if kind==5 and len(records)>1 else FREE)
        entry[80:96]=guid
        struct.pack_into('<IQ',entry,116,first,size)
        sectors[i//(sector//128)][(i%(sector//128))*128:(i%(sector//128)+1)*128]=entry
    header=bytearray(sector)
    header[:8]=bytes.fromhex('d0cf11e0a1b11ae1')
    struct.pack_into('<HHHHH',header,24,0x3e,major,0xfffe,9 if major==3 else 12,6)
    struct.pack_into('<IIIIIIIII',header,40,directory_count if major==4 else 0,fat_count,0,0,4096,mini_first,mini_count,difat_ids[0] if difat_ids else END,difat_count)
    for i in range(109): struct.pack_into('<I',header,76+4*i,fat_ids[i] if i<len(fat_ids) else FREE)
    return bytes(header+b''.join(sectors))

def cases():
    simple=property_stream([(1,typed(2,struct.pack('<h',1252))),(18,text('Microsoft Word'))])
    values=[]
    for major in (3,4):
        for mini in (False,True):
            values.append((f'summary-v{major}-'+('mini' if mini else 'regular'),compound([('\x05SummaryInformation',simple)],major=major,mini=mini)))
    values.append(('missing-root',compound([('WordDocument',b'word')],root=False)))
    values.append(('empty-root',compound([])))
    for name in ['EncryptedPackage','mfbu_header','Workbook','WordDocument','PowerPoint Document','__properties_version1.0']:
        values.append(('type-'+name,compound([(name,b'data')],mini=True)))
    values.append(('hwp-precedence',compound([('FileHeader',b'HWP Document File\0'),('\x05SummaryInformation',simple)],mini=True)))
    props=[(1,typed(2,struct.pack('<h',1252))),(2,text('caf\xe9')),(2,text('duplicate title')),
           (3,typed(31,struct.pack('<I',4)+'\u96ea\u4eba!\0'.encode('utf-16-le'))),
           (14,typed(3,struct.pack('<i',-123))),(15,typed(19,struct.pack('<I',0xffffffff))),
           (16,typed(20,struct.pack('<q',-1234567890123))),(19,typed(21,struct.pack('<Q',2**63+1))),
           (200,typed(11,b'\xff\xff\0\0')),(201,typed(4,struct.pack('<f',1.25))),
           (202,typed(5,struct.pack('<d',-2.5))),(203,typed(1,b'')),(204,typed(0,b'')),
           (10,typed(64,struct.pack('<q',12300000000))),(12,typed(64,struct.pack('<q',116444736000000000))),
           (17,typed(71,struct.pack('<I',4)+b'\0'*4)),
           (205,typed(0x1002,struct.pack('<Ihhh',3,-1,2,3))),
           (206,typed(0x1003,struct.pack('<Iii',2,-1,2))),
           (207,typed(0x101f,struct.pack('<II',2,2)+'A\0'.encode('utf-16-le')+struct.pack('<I',2)+'B\0'.encode('utf-16-le'))),
           (208,typed(0x100c,struct.pack('<I',2)+typed(3,struct.pack('<i',7))+typed(2,struct.pack('<h',8))+b'\0\0')),
           (209,typed(0x4003,b'ABCD')),(18,text('Microsoft Word'))]
    values.append(('all-properties',compound([('\x05SummaryInformation',property_stream(props))],mini=False)))
    extra=(bytes.fromhex('05d5cdd59c2e1b10939708002b2cf9ae'),section([(2,text('custom'))]))
    values.append(('two-sets',compound([('\x05SummaryInformation',property_stream([(2,text('first'))],[extra]))],mini=True)))
    values.append(('msi-precedence',compound([('\x05SummaryInformation',simple)],mini=True,clsid=bytes.fromhex('84100c0000000000c000000000000046'))))
    cp=property_stream([(1,typed(2,struct.pack('<h',932))),(2,text('original'))])
    values.append(('unsupported-codepage',compound([('\x05SummaryInformation',cp)],mini=True)))
    invalid=bytearray(simple);struct.pack_into('<I',invalid,48+12,4)
    values.append(('property-overlap',compound([('\x05SummaryInformation',bytes(invalid))],mini=True)))
    vector=property_stream([(2,typed(0x1002,struct.pack('<I',10001)))])
    values.append(('vector-limit',compound([('\x05SummaryInformation',vector)],mini=True)))
    catalog=b''.join(struct.pack('<HHIQ',16+len(name.encode('utf-16-le'))+2,0,i,0)+name.encode('utf-16-le')+b'\0\0' for i,name in enumerate(['header','first.jpg','second.png']))+b'\0\0'
    values.append(('catalog',compound([('Catalog',catalog)],mini=True)))
    catalog12=struct.pack('<HHII',12,7,2,2)+catalog[len(struct.pack('<HHIQ',30,0,0,0)+'header'.encode('utf-16-le')+b'\0\0'):]
    values.append(('catalog-header12',compound([('Catalog',catalog12)],mini=True)))
    cycle=bytearray(compound([]));fat=struct.unpack_from('<I',cycle,76)[0];struct.pack_into('<I',cycle,(fat+1)*512,0)
    values.append(('directory-chain-cycle',bytes(cycle)))
    tree=bytearray(compound([]));struct.pack_into('<I',tree,512+76,0)
    values.append(('directory-tree-cycle',bytes(tree)))
    badfat=bytearray(compound([]));struct.pack_into('<I',badfat,76,FREE)
    values.append(('missing-fat',bytes(badfat)))
    return values

if __name__ == '__main__':
    from pathlib import Path
    import hashlib,json
    root=Path(__file__).resolve().parents[1]
    target=root/'tests/fixtures/cdf';target.mkdir(parents=True,exist_ok=True)
    manifest=[]
    for name,data in cases():
        path=target/(name+'.cfb');path.write_bytes(data)
        manifest.append(dict(name=name,bytes=len(data),sha256=hashlib.sha256(data).hexdigest()))
    (target/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
    print(f'Wrote {len(manifest)} independent CFB fixtures')
