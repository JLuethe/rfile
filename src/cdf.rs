/*-
 * Copyright (c) 2008 Christos Zoulas
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 *
 * THIS SOFTWARE IS PROVIDED BY THE NETBSD FOUNDATION, INC. AND CONTRIBUTORS
 * ``AS IS'' AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED
 * TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR
 * PURPOSE ARE DISCLAIMED.  IN NO EVENT SHALL THE FOUNDATION OR CONTRIBUTORS
 * BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
 * CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
 * SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
 * INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
 * CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
 * ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
 * POSSIBILITY OF SUCH DAMAGE.
 */
// rfile contributions: Copyright (C) 2026 rfile contributors.
// SPDX-License-Identifier: GPL-3.0-only

// Independent Rust implementation; upstream behavior and retained data notices credited in NOTICE.
//! Bounded Compound File Binary Format reader. No extraction or external processes.
use crate::{
    Error, InspectionBudget, Result, SemanticProfile, TimeContext,
    input::{InputView, InspectionBytes, read_range},
};
use std::collections::HashSet;
const SIGNATURE: &[u8] = b"\xd0\xcf\x11\xe0\xa1\xb1\x1a\xe1";
const END: u32 = 0xfffffffe;
const RESERVED: u32 = 0xfffffffa;
const PROPERTY_LIMIT: usize = 10_000;
const SUMMARY_FMTID: [u8; 16] = [
    0xe0, 0x85, 0x9f, 0xf2, 0xf9, 0x4f, 0x68, 0x10, 0xab, 0x91, 0x08, 0, 0x2b, 0x27, 0xb3, 0xd9,
];
fn bad(stage: &'static str, offset: usize) -> Error {
    Error::Malformed {
        format: "CDF",
        stage,
        offset: offset as u64,
    }
}
fn range<'a>(b: &'a [u8], p: usize, n: usize, stage: &'static str) -> Result<&'a [u8]> {
    b.get(p..p.checked_add(n).ok_or_else(|| bad(stage, p))?)
        .ok_or_else(|| bad(stage, p))
}
fn u16le(b: &[u8], p: usize) -> Result<u16> {
    Ok(u16::from_le_bytes(
        range(b, p, 2, "field")?.try_into().expect("checked width"),
    ))
}
fn u32le(b: &[u8], p: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(
        range(b, p, 4, "field")?.try_into().expect("checked width"),
    ))
}
fn u64le(b: &[u8], p: usize) -> Result<u64> {
    Ok(u64::from_le_bytes(
        range(b, p, 8, "field")?.try_into().expect("checked width"),
    ))
}
#[derive(Clone, Debug)]
pub struct Directory {
    pub name: Vec<u16>,
    pub index: u32,
    pub kind: u8,
    pub first_sector: u32,
    pub size: u64,
    pub clsid: [u8; 16],
    pub created: u64,
    pub modified: u64,
    pub left: u32,
    pub right: u32,
    pub child: u32,
}
impl Directory {
    pub fn name_lossy(&self) -> String {
        String::from_utf16_lossy(&self.name)
    }
    fn named(&self, text: &str) -> bool {
        self.name.iter().copied().eq(text.encode_utf16())
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum PropertyValue {
    Empty,
    Null,
    I16(i16),
    I32(i32),
    U32(u32),
    I64(i64),
    U64(u64),
    Bool(i32),
    F32(u32),
    F64(u64),
    Bytes(Vec<u8>),
    Utf16(Vec<u16>),
    FileTime(i64),
    Vector(Vec<PropertyValue>),
    Opaque { offset: usize, len: usize },
}
#[derive(Clone, Debug)]
pub struct Property {
    pub id: u32,
    pub wire_type: u32,
    pub value: PropertyValue,
}
#[derive(Clone, Debug)]
pub struct PropertySet {
    pub stream: String,
    pub fmtid: [u8; 16],
    pub byte_order: u16,
    pub system: u16,
    pub system_version: u16,
    pub properties: Vec<Property>,
}
#[derive(Clone, Debug)]
pub struct CatalogEntry {
    pub number: u32,
    pub timestamp: u64,
    pub name: Vec<u16>,
    pub header_bytes: Option<Vec<u8>>,
}
#[derive(Clone, Debug, Default)]
pub struct CompoundDocument {
    pub major: u16,
    pub sector_bytes: usize,
    pub directories: Vec<Directory>,
    pub property_sets: Vec<PropertySet>,
    pub catalog: Vec<CatalogEntry>,
    pub diagnostics: Vec<String>,
    pub failure_stage: Option<&'static str>,
}
pub(crate) struct Detection {
    pub description: Vec<u8>,
    pub mime: &'static str,
    pub facts: CompoundDocument,
}
struct Reader<'a, 'v> {
    prefix: &'a [u8],
    view: Option<&'v mut dyn InputView>,
    budget: &'a InspectionBudget,
    sector: usize,
    mini: usize,
    fat: Vec<u32>,
    minifat: Vec<u32>,
    root: InspectionBytes,
    physical: u64,
    profile: SemanticProfile,
}
impl Reader<'_, '_> {
    fn sector(&mut self, id: u32) -> Result<InspectionBytes> {
        if id >= RESERVED {
            return Err(bad("sector sentinel", id as usize));
        }
        let offset = (u64::from(id) + 1)
            .checked_mul(self.sector as u64)
            .ok_or_else(|| bad("sector range", 0))?;
        let end = offset
            .checked_add(self.sector as u64)
            .ok_or_else(|| bad("sector range", 0))?;
        if end > self.physical {
            return Err(bad("sector EOF", offset as usize));
        }
        if end <= self.prefix.len() as u64 {
            self.budget.step()?;
            self.budget.bytes(self.sector)?;
            let mut out = InspectionBytes::new(self.budget)?;
            out.extend(&self.prefix[offset as usize..end as usize])?;
            return Ok(out);
        }
        let view = self
            .view
            .as_deref_mut()
            .ok_or_else(|| bad("sector EOF", offset as usize))?;
        let out = read_range(view, offset, self.sector, self.budget)?;
        if out.len() != self.sector {
            return Err(bad("sector short read", offset as usize));
        }
        Ok(out)
    }
    fn chain(&mut self, first: u32, small: bool) -> Result<InspectionBytes> {
        let mut out = InspectionBytes::new(self.budget)?;
        let mut seen = HashSet::new();
        let mut id = first;
        let available = if small {
            self.root.len() / self.mini
        } else {
            (self.physical / self.sector as u64).min(usize::MAX as u64) as usize
        };
        while id != END {
            self.budget.step()?;
            if id >= RESERVED || seen.len() >= available.min(250_000) {
                return Err(bad("chain link", id as usize));
            }
            // Charge a conservative hash-table growth envelope before insert.
            self.budget.charge_retained(128)?;
            if !seen.insert(id) {
                return Err(bad("chain cycle", id as usize));
            }
            if small {
                let pos = (id as usize)
                    .checked_mul(self.mini)
                    .ok_or_else(|| bad("mini range", 0))?;
                let bytes = range(&self.root, pos, self.mini, "mini EOF")?;
                self.budget.bytes(bytes.len())?;
                out.extend(bytes)?;
            } else {
                let bytes = self.sector(id)?;
                out.extend(&bytes)?;
            }
            let table = if small { &self.minifat } else { &self.fat };
            id = *table
                .get(id as usize)
                .ok_or_else(|| bad("chain table", id as usize))?;
        }
        Ok(out)
    }
    fn stream(&mut self, dir: &Directory, cutoff: u64) -> Result<InspectionBytes> {
        if dir.size == 0 {
            return InspectionBytes::new(self.budget);
        }
        let mut bytes = self.chain(dir.first_sector, dir.size < cutoff)?;
        let size = usize::try_from(dir.size).map_err(|_| bad("stream size", 0))?;
        if size > bytes.len() {
            return Err(bad("stream truncated", size));
        }
        bytes.truncate(size);
        Ok(bytes)
    }
    fn tables(&mut self, header: &[u8]) -> Result<Vec<Directory>> {
        let declared_count = u32le(header, 44)? as usize;
        let mut ids = Vec::new();
        let mut unique = HashSet::new();
        for p in (76..512).step_by(4) {
            let id = u32le(header, p)?;
            if id >= 0x80000000 {
                break;
            }
            if id < RESERVED {
                self.budget.charge_retained(128)?;
                if !unique.insert(id) {
                    return Err(bad("Can't read SAT", p));
                }
                ids.push(id);
            }
        }
        let mut next = u32le(header, 68)?;
        let difat_count = u32le(header, 72)? as usize;
        if difat_count as u64 > self.physical / self.sector as u64 {
            return Err(bad("Can't read SAT", 72));
        }
        let mut seen = HashSet::new();
        'extensions: for _ in 0..difat_count {
            if next >= 0x80000000 {
                break;
            }
            self.budget.step()?;
            self.budget.charge_retained(128)?;
            if !seen.insert(next) {
                return Err(bad("Can't read SAT", 68));
            }
            let bytes = self.sector(next).map_err(|e| stage(e, "Can't read SAT"))?;
            for p in (0..bytes.len() - 4).step_by(4) {
                let id = u32le(&bytes, p)?;
                if id >= 0x80000000 {
                    break 'extensions;
                }
                if id < RESERVED {
                    self.budget.charge_retained(128)?;
                    if !unique.insert(id) {
                        return Err(bad("Can't read SAT", p));
                    }
                    ids.push(id);
                }
            }
            next = u32le(&bytes, bytes.len() - 4)?;
        }
        let count = ids.len();
        if self.profile == SemanticProfile::CorrectedV1 && count != declared_count {
            return Err(bad("Can't read SAT", 44));
        }
        if count > 250_000 {
            return Err(Error::Limit("CDF FAT sectors"));
        }
        let entries = count
            .checked_mul(self.sector / 4)
            .ok_or(Error::Limit("CDF FAT entries"))?;
        self.budget.charge_retained(entries.saturating_mul(8))?;
        self.fat
            .try_reserve_exact(entries)
            .map_err(|_| Error::Limit("CDF FAT allocation"))?;
        for id in ids.into_iter().take(count) {
            let bytes = self.sector(id).map_err(|e| stage(e, "Can't read SAT"))?;
            self.fat.extend(
                bytes
                    .chunks_exact(4)
                    .map(|b| u32::from_le_bytes(b.try_into().expect("entry width"))),
            );
        }
        if u32le(header, 64)? > 0 {
            let bytes = self
                .chain(u32le(header, 60)?, false)
                .map_err(|e| stage(e, "Can't read SSAT"))?;
            self.budget.charge_retained(bytes.len().saturating_mul(2))?;
            self.minifat = bytes
                .chunks_exact(4)
                .map(|b| u32::from_le_bytes(b.try_into().expect("entry width")))
                .collect();
        }
        let bytes = self
            .chain(u32le(header, 48)?, false)
            .map_err(|e| stage(e, "Can't read directory"))?;
        self.budget.charge_retained(bytes.len().saturating_mul(8))?;
        let mut dirs = Vec::new();
        for (index, d) in bytes.chunks_exact(128).enumerate() {
            self.budget.step()?;
            let n = u16le(d, 64)? as usize;
            if d[66] == 0 {
                continue;
            }
            if !(2..=64).contains(&n) || !n.is_multiple_of(2) {
                return Err(bad("Can't read directory", 64));
            }
            let size = u64le(d, 120)?;
            dirs.push(Directory {
                index: index as u32,
                name: d[..n - 2]
                    .chunks_exact(2)
                    .map(|b| u16::from_le_bytes([b[0], b[1]]))
                    .collect(),
                kind: d[66],
                first_sector: u32le(d, 116)?,
                size: if self.sector == 512 {
                    size & 0xffffffff
                } else {
                    size
                },
                clsid: d[80..96].try_into().expect("directory width"),
                created: u64le(d, 100)?,
                modified: u64le(d, 108)?,
                left: u32le(d, 68)?,
                right: u32le(d, 72)?,
                child: u32le(d, 76)?,
            });
        }
        if let Some(root) = dirs.iter().find(|d| d.kind == 5) {
            if root.size > 0 && root.first_sector < RESERVED {
                self.root = self
                    .chain(root.first_sector, false)
                    .map_err(|e| stage(e, "Cannot read short stream"))?;
                let size =
                    usize::try_from(root.size).map_err(|_| bad("Cannot read short stream", 0))?;
                if size > self.root.len() {
                    return Err(bad("Cannot read short stream", 0));
                }
                self.root.truncate(size);
            }
        }
        Ok(dirs)
    }
}
fn stage(error: Error, name: &'static str) -> Error {
    match error {
        Error::Malformed { offset, .. } => Error::Malformed {
            format: "CDF",
            stage: name,
            offset,
        },
        e => e,
    }
}
struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
    budget: &'a InspectionBudget,
}
impl Cursor<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8]> {
        let bytes = range(self.bytes, self.pos, n, "property bounds")?;
        self.pos += n;
        self.budget.bytes(n)?;
        Ok(bytes)
    }
    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(
            self.take(4)?.try_into().expect("field width"),
        ))
    }
    fn align(&mut self) -> Result<()> {
        let next = self
            .pos
            .checked_add(3)
            .ok_or_else(|| bad("property alignment", self.pos))?
            & !3;
        self.take(next - self.pos)?;
        Ok(())
    }
    fn value(&mut self, wire: u32, count: &mut usize, depth: usize) -> Result<PropertyValue> {
        self.budget.step()?;
        *count = count
            .checked_add(1)
            .filter(|&n| n <= PROPERTY_LIMIT)
            .ok_or(Error::Limit("CDF properties"))?;
        self.budget.charge_retained(256)?;
        if depth > 16 {
            return Err(Error::Limit("CDF property depth"));
        }
        if wire & !0x1fff != 0 {
            return Ok(PropertyValue::Opaque {
                offset: self.pos,
                len: self.bytes.len() - self.pos,
            });
        }
        if wire & 0x1000 != 0 {
            let n = self.u32()? as usize;
            if n > PROPERTY_LIMIT.saturating_sub(*count) {
                return Err(Error::Limit("CDF vector elements"));
            }
            let mut values = Vec::new();
            for _ in 0..n {
                let base = wire & 0xfff;
                let kind = if base == 12 { self.u32()? } else { base };
                values.push(self.value(kind, count, depth + 1)?);
                if matches!(base, 12 | 30 | 31 | 71) {
                    self.align()?;
                }
            }
            self.align()?;
            return Ok(PropertyValue::Vector(values));
        }
        Ok(match wire & 0xfff {
            0 => PropertyValue::Empty,
            1 => PropertyValue::Null,
            2 => PropertyValue::I16(i16::from_le_bytes(self.take(2)?.try_into().expect("width"))),
            3 => PropertyValue::I32(self.u32()? as i32),
            4 => PropertyValue::F32(self.u32()?),
            5 => PropertyValue::F64(u64::from_le_bytes(self.take(8)?.try_into().expect("width"))),
            11 => PropertyValue::Bool(
                i16::from_le_bytes(self.take(2)?.try_into().expect("width")) as i32
            ),
            19 => PropertyValue::U32(self.u32()?),
            20 => PropertyValue::I64(
                u64::from_le_bytes(self.take(8)?.try_into().expect("width")) as i64
            ),
            21 => PropertyValue::U64(u64::from_le_bytes(self.take(8)?.try_into().expect("width"))),
            30 | 31 => {
                let n = self.u32()? as usize;
                let size = n
                    .checked_mul(if wire & 0xfff == 31 { 2 } else { 1 })
                    .ok_or_else(|| bad("property string size", self.pos))?;
                self.budget.charge_retained(size.saturating_mul(2))?;
                let bytes = self.take(size)?;
                if wire & 0xfff == 31 {
                    PropertyValue::Utf16(
                        bytes
                            .chunks_exact(2)
                            .map(|b| u16::from_le_bytes([b[0], b[1]]))
                            .collect(),
                    )
                } else {
                    PropertyValue::Bytes(bytes.to_vec())
                }
            }
            64 => PropertyValue::FileTime(u64::from_le_bytes(
                self.take(8)?.try_into().expect("width"),
            ) as i64),
            71 => {
                let n = self.u32()? as usize;
                let offset = self.pos;
                self.take(n)?;
                PropertyValue::Opaque { offset, len: n }
            }
            _ => PropertyValue::Opaque {
                offset: self.pos,
                len: self.bytes.len() - self.pos,
            },
        })
    }
}
fn properties(
    bytes: &[u8],
    name: &str,
    budget: &InspectionBudget,
    total: &mut usize,
) -> Result<Vec<PropertySet>> {
    let count = u32le(bytes, 24)? as usize;
    if count > 64 {
        return Err(Error::Limit("CDF property sets"));
    }
    range(
        bytes,
        28,
        count
            .checked_mul(20)
            .ok_or_else(|| bad("section table", 28))?,
        "section table",
    )?;
    let mut sets = Vec::new();
    for i in 0..count {
        budget.step()?;
        let declaration = 28 + i * 20;
        let offset = u32le(bytes, declaration + 16)? as usize;
        let length = u32le(bytes, offset)? as usize;
        if offset < 28 + 20 * count || length < 8 {
            return Err(bad("section overlap", offset));
        }
        let section = range(bytes, offset, length, "section bounds")?;
        let n = u32le(section, 4)? as usize;
        if n > PROPERTY_LIMIT.saturating_sub(*total) {
            return Err(Error::Limit("CDF properties"));
        }
        let table_end = 8usize
            .checked_add(
                n.checked_mul(8)
                    .ok_or_else(|| bad("property table", offset))?,
            )
            .ok_or_else(|| bad("property table", offset))?;
        range(section, 0, table_end, "property table")?;
        budget.charge_retained(n.saturating_mul(16))?;
        let mut offsets = (0..n)
            .map(|j| u32le(section, 12 + j * 8).map(|v| v as usize))
            .collect::<Result<Vec<_>>>()?;
        budget.steps(n.saturating_mul(16))?;
        offsets.sort_unstable();
        let mut values = Vec::new();
        for j in 0..n {
            let id = u32le(section, 8 + j * 8)?;
            let start = u32le(section, 12 + j * 8)? as usize;
            if start < table_end {
                return Err(bad("property overlap", offset + start));
            }
            let end = offsets
                .get(offsets.partition_point(|&p| p <= start))
                .copied()
                .unwrap_or(section.len())
                .min(section.len());
            let property_bytes = &section[..end];
            let wire = u32le(property_bytes, start)?;
            let mut cursor = Cursor {
                bytes: property_bytes,
                pos: start + 4,
                budget,
            };
            let value = cursor.value(wire, total, 0)?;
            values.push(Property {
                id,
                wire_type: wire,
                value,
            });
        }
        budget.charge_retained(512)?;
        sets.push(PropertySet {
            stream: name.into(),
            fmtid: bytes[declaration..declaration + 16]
                .try_into()
                .expect("declaration bounds"),
            byte_order: u16le(bytes, 0)?,
            system: u16le(bytes, 6)?,
            system_version: u16le(bytes, 4)?,
            properties: values,
        });
    }
    Ok(sets)
}
fn catalog(bytes: &[u8], budget: &InspectionBudget) -> Result<Vec<CatalogEntry>> {
    let mut pos = 0;
    let mut entries = Vec::new();
    while pos + 2 <= bytes.len() {
        budget.step()?;
        let len = u16le(bytes, pos)? as usize;
        if len == 0 {
            break;
        }
        if entries.len() >= PROPERTY_LIMIT {
            return Err(Error::Limit("CDF catalog entries"));
        }
        let entry = range(bytes, pos, len, "catalog record")?;
        if entries.is_empty() && len == 12 {
            budget.charge_retained(128)?;
            entries.push(CatalogEntry {
                number: 0,
                timestamp: 0,
                name: Vec::new(),
                header_bytes: Some(entry.to_vec()),
            });
            pos += len;
            continue;
        }
        if len < 16 || !(len - 16).is_multiple_of(2) {
            return Err(bad("catalog record", pos));
        }
        budget.charge_retained(len.saturating_mul(4))?;
        entries.push(CatalogEntry {
            number: u32le(entry, 4)?,
            timestamp: u64le(entry, 8)?,
            header_bytes: None,
            name: entry[16..]
                .chunks_exact(2)
                .map(|b| u16::from_le_bytes([b[0], b[1]]))
                .take_while(|&c| c != 0)
                .collect(),
        });
        pos += len;
    }
    Ok(entries)
}
fn validate_tree(
    dirs: &[Directory],
    budget: &InspectionBudget,
    diagnostics: &mut Vec<String>,
) -> Result<()> {
    // Validate references without recursively following attacker-controlled trees.
    budget.charge_retained(dirs.len().saturating_mul(256))?;
    let indexes: std::collections::HashMap<_, _> =
        dirs.iter().enumerate().map(|(i, d)| (d.index, i)).collect();
    let mut colors = vec![0u8; dirs.len()];
    for start in 0..dirs.len() {
        if colors[start] != 0 {
            continue;
        }
        let mut stack = vec![(start, false)];
        while let Some((index, exit)) = stack.pop() {
            budget.step()?;
            if exit {
                colors[index] = 2;
                continue;
            }
            if colors[index] == 2 {
                continue;
            }
            if colors[index] == 1 {
                diagnostics.push("cyclic directory tree".into());
                return Ok(());
            }
            colors[index] = 1;
            stack.push((index, true));
            let dir = &dirs[index];
            for link in [dir.child, dir.right, dir.left] {
                if link == 0xffffffff {
                    continue;
                }
                if let Some(&next) = indexes.get(&link) {
                    stack.push((next, false));
                } else {
                    diagnostics.push(format!(
                        "directory {} references absent record {link}",
                        dir.index
                    ));
                }
            }
        }
    }
    Ok(())
}
/// Parse a bounded property stream independently of its container. Offsets in
/// opaque values are relative to the owning section; property order/duplicates
/// and exact floating-point bits are retained.
pub fn parse_property_stream(
    bytes: &[u8],
    name: &str,
    budget: &InspectionBudget,
) -> Result<Vec<PropertySet>> {
    if name.len() > 256 {
        return Err(Error::Limit("CDF stream name"));
    }
    properties(bytes, name, budget, &mut 0)
}
/// Inspect a caller-owned positional source with the same aggregate budget as
/// classification. No paths, native pointers or extraction processes are used.
pub fn inspect(
    view: &mut dyn InputView,
    budget: &InspectionBudget,
    profile: SemanticProfile,
    time: &TimeContext,
) -> Result<Option<CompoundDocument>> {
    let prefix = read_range(view, 0, 512, budget)?;
    Ok(detect(&prefix, Some(view), budget, profile, time)?.map(|d| d.facts))
}
fn dir_type(dirs: &[Directory]) -> Option<(&'static str, &'static str)> {
    // readcdf.c sectioninfo order, scoped to physical directory record order.
    for d in dirs {
        for (name, kind, desc, mime) in [
            ("EncryptedPackage", 2, "Encrypted", "application/encrypted"),
            ("EncryptedSummary", 2, "Encrypted", "application/encrypted"),
            ("mfbu_header", 2, "QuickBooks", "application/quickbooks"),
            ("Book", 2, "Microsoft Excel", "application/vnd.ms-excel"),
            ("Workbook", 2, "Microsoft Excel", "application/vnd.ms-excel"),
            ("WordDocument", 2, "Microsoft Word", "application/msword"),
            (
                "PowerPoint Document",
                2,
                "Microsoft PowerPoint",
                "application/vnd.ms-powerpoint",
            ),
            (
                "__properties_version1.0",
                2,
                "Microsoft Outlook Message",
                "application/vnd.ms-outlook",
            ),
            (
                "__recip_version1.0_#00000000",
                1,
                "Microsoft Outlook Message",
                "application/vnd.ms-outlook",
            ),
        ] {
            if d.kind == kind && d.named(name) {
                return Some((desc, mime));
            }
        }
    }
    None
}
fn app_type(app: &str) -> Option<&'static str> {
    let app = app.to_ascii_lowercase();
    for (pattern, mime) in [
        ("word", "application/msword"),
        ("excel", "application/vnd.ms-excel"),
        ("powerpoint", "application/vnd.ms-powerpoint"),
        ("crystal reports", "application/x-rpt"),
        ("advanced installer", "application/vnd.ms-msi"),
        ("installshield", "application/vnd.ms-msi"),
        ("microsoft patch compiler", "application/vnd.ms-msi"),
        ("nant", "application/vnd.ms-msi"),
        ("windows installer", "application/vnd.ms-msi"),
    ] {
        if app.contains(pattern) {
            return Some(mime);
        }
    }
    None
}
const LABELS: [&str; 19] = [
    "Code page",
    "Title",
    "Subject",
    "Author",
    "Keywords",
    "Comments",
    "Template",
    "Last Saved By",
    "Revision Number",
    "Total Editing Time",
    "Last Printed",
    "Create Time/Date",
    "Last Saved Time/Date",
    "Number of Pages",
    "Number of Words",
    "Number of Characters",
    "Thumbnail",
    "Name of Creating Application",
    "Security",
];
fn property_name(id: u32) -> String {
    if id == 0 {
        return "Dictionary".into();
    }
    if id == 0x80000000 {
        return "Locale ID".into();
    }
    if (1..=19).contains(&id) {
        LABELS[id as usize - 1].into()
    } else {
        format!("{id:#x}")
    }
}
fn decode_string(bytes: &[u8], codepage: u16) -> Result<String> {
    let bytes = crate::database::cstr(bytes);
    match codepage {
        65001 => String::from_utf8(bytes.to_vec()).map_err(|_| bad("UTF8 property", 0)),
        20127 if bytes.is_ascii() => Ok(String::from_utf8_lossy(bytes).into_owned()),
        1252 => {
            const EXT: [char; 32] = [
                '€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{8d}',
                'Ž', '\u{8f}', '\u{90}', '‘', '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›',
                'œ', '\u{9d}', 'ž', 'Ÿ',
            ];
            Ok(bytes
                .iter()
                .map(|&b| {
                    if (128..160).contains(&b) {
                        EXT[(b - 128) as usize]
                    } else {
                        char::from(b)
                    }
                })
                .collect())
        }
        _ => Err(Error::Unsupported(format!("CDF codepage {codepage}"))),
    }
}
fn property_text(
    value: &PropertyValue,
    codepage: u16,
    time: &TimeContext,
) -> Result<Option<String>> {
    Ok(match value {
        PropertyValue::Empty | PropertyValue::Null | PropertyValue::Opaque { .. } => None,
        PropertyValue::I16(n) => Some(n.to_string()),
        PropertyValue::I32(n) | PropertyValue::Bool(n) => Some(n.to_string()),
        PropertyValue::U32(n) => Some(n.to_string()),
        PropertyValue::I64(n) => Some(n.to_string()),
        PropertyValue::U64(n) => Some(n.to_string()),
        PropertyValue::F32(n) => Some(
            String::from_utf8_lossy(&crate::format::render(
                b"%g",
                &crate::format::Value::Float(f32::from_bits(*n) as f64),
                false,
            )?)
            .into_owned(),
        ),
        PropertyValue::F64(n) => Some(
            String::from_utf8_lossy(&crate::format::render(
                b"%g",
                &crate::format::Value::Float(f64::from_bits(*n)),
                false,
            )?)
            .into_owned(),
        ),
        PropertyValue::Bytes(bytes) => Some(decode_string(bytes, codepage)?),
        PropertyValue::Utf16(units) => Some(
            String::from_utf16(
                &units
                    .iter()
                    .copied()
                    .take_while(|&c| c != 0)
                    .collect::<Vec<_>>(),
            )
            .map_err(|_| bad("UTF16 property", 0))?,
        ),
        PropertyValue::FileTime(0) => None,
        PropertyValue::FileTime(n) => {
            if *n < 1_000_000_000_000_000 {
                let s = n / 10_000_000;
                let d = s / 86400;
                let h = s / 3600 % 24;
                let m = s / 60 % 60;
                Some(format!(
                    "{}{}{m:02}:{:02}",
                    if d != 0 {
                        format!("{d}d+")
                    } else {
                        String::new()
                    },
                    if d != 0 || h != 0 {
                        format!("{h:02}:")
                    } else {
                        String::new()
                    },
                    s % 60
                ))
            } else {
                Some(crate::format::datetime(
                    *n as u64,
                    true,
                    true,
                    time,
                    SemanticProfile::CorrectedV1,
                ))
            }
        }
        PropertyValue::Vector(values) => {
            let mut out = Vec::new();
            for value in values {
                if let Some(text) = property_text(value, codepage, time)? {
                    out.push(text);
                }
            }
            Some(out.join("; "))
        }
    })
}
fn text_bound(value: &PropertyValue) -> usize {
    match value {
        PropertyValue::Bytes(b) => b.len().saturating_mul(4),
        PropertyValue::Utf16(b) => b.len().saturating_mul(4),
        PropertyValue::Vector(v) => v.iter().fold(0usize, |n, x| {
            n.saturating_add(text_bound(x)).saturating_add(2)
        }),
        _ => 256,
    }
}
fn corrected_summary(
    facts: &mut CompoundDocument,
    time: &TimeContext,
    budget: &InspectionBudget,
) -> Result<Option<(Vec<u8>, &'static str)>> {
    let Some(set) = facts.property_sets.first() else {
        return Ok(None);
    };
    let mut text = String::from("Composite Document File V2 Document");
    text.push_str(if set.byte_order == 0xfffe {
        ", Little Endian"
    } else {
        ", Big Endian"
    });
    let (major, minor) = (set.system_version & 255, set.system_version >> 8);
    match set.system {
        2 => text.push_str(&format!(", Os: Windows, Version {major}.{minor}")),
        1 => text.push_str(&format!(", Os: MacOS, Version {minor}.{major}")),
        n => text.push_str(&format!(", Os {n}, Version: {major}.{minor}")),
    }
    let root_msi = facts.directories.iter().any(|d| {
        d.kind == 5
            && u64::from_le_bytes(d.clsid[..8].try_into().expect("CLSID width")) == 0xc1084
            && u64::from_le_bytes(d.clsid[8..].try_into().expect("CLSID width"))
                == 0x46000000000000c0
    });
    if root_msi {
        text.push_str(", Microsoft Installer");
    }
    let mut mime = root_msi.then_some("application/vnd.ms-msi");
    for set in &facts.property_sets {
        let codepage = set
            .properties
            .iter()
            .find_map(|p| {
                if p.id == 1 {
                    match p.value {
                        PropertyValue::I16(n) => Some(n as u16),
                        PropertyValue::U32(n) => Some(n as u16),
                        _ => None,
                    }
                } else {
                    None
                }
            })
            .unwrap_or(1252);
        for property in &set.properties {
            budget.step()?;
            let bound = text_bound(&property.value);
            if bound > budget.limits.output {
                return Err(Error::Limit("CDF property output"));
            }
            let _render_memory = budget.reserve(bound.saturating_mul(32))?;
            match property_text(&property.value, codepage, time) {
                Ok(Some(value)) if !value.is_empty() => {
                    if set.stream == "\u{5}SummaryInformation"
                        && set.fmtid == SUMMARY_FMTID
                        && property.id == 18
                        && mime.is_none()
                    {
                        mime = app_type(&value);
                    }
                    budget.charge_retained(value.len().saturating_mul(8).saturating_add(256))?;
                    if text.len().saturating_add(value.len()).saturating_add(256)
                        > budget.limits.output
                    {
                        return Err(Error::Limit("CDF output"));
                    }
                    let label =
                        if set.stream == "\u{5}SummaryInformation" && set.fmtid == SUMMARY_FMTID {
                            property_name(property.id)
                        } else {
                            format!("{} {:#x}", set.stream.escape_default(), property.id)
                        };
                    text.push_str(&format!(", {label}: {value}"));
                }
                Ok(_) => {}
                Err(Error::Unsupported(message)) => facts.diagnostics.push(message),
                Err(Error::Malformed { stage, .. }) => {
                    facts.diagnostics.push(format!("invalid {stage}"))
                }
                Err(e) => return Err(e),
            }
        }
    }
    let mime = mime
        .or_else(|| dir_type(&facts.directories).map(|x| x.1))
        .unwrap_or("application/vnd.ms-office");
    Ok(Some((text.into_bytes(), mime)))
}
pub(crate) fn detect(
    prefix: &[u8],
    view: Option<&mut dyn InputView>,
    budget: &InspectionBudget,
    profile: SemanticProfile,
    time: &TimeContext,
) -> Result<Option<Detection>> {
    if prefix.len() < 512 || !prefix.starts_with(SIGNATURE) {
        return Ok(None);
    }
    let shift = u16le(prefix, 30)?;
    let mini = u16le(prefix, 32)?;
    if shift > 20 || mini > 20 {
        return Ok(None);
    }
    let mut facts = CompoundDocument {
        major: u16le(prefix, 26)?,
        sector_bytes: 1 << shift,
        ..CompoundDocument::default()
    };
    let physical = view
        .as_ref()
        .and_then(|v| v.known_len())
        .unwrap_or(prefix.len() as u64);
    let result = (|| -> Result<(Vec<u8>, &'static str)> {
        if shift < 7 || mini < 2 || mini > shift {
            return Err(bad("Can't read SAT", 30));
        }
        if profile == SemanticProfile::CorrectedV1
            && !((facts.major == 3 && shift == 9) || (facts.major == 4 && shift == 12))
        {
            return Err(bad("header version", 26));
        }
        if profile == SemanticProfile::CorrectedV1 && (u16le(prefix, 28)? != 0xfffe || mini != 6) {
            return Err(bad("header layout", 28));
        }
        let mut reader = Reader {
            prefix,
            view,
            budget,
            sector: 1 << shift,
            mini: 1 << mini,
            fat: Vec::new(),
            minifat: Vec::new(),
            root: InspectionBytes::new(budget)?,
            physical,
            profile,
        };
        facts.directories = reader.tables(prefix)?;
        validate_tree(&facts.directories, budget, &mut facts.diagnostics)?;
        let cutoff = u32le(prefix, 56)? as u64;
        if let Some(file) = facts
            .directories
            .iter()
            .find(|d| d.kind == 2 && d.named("FileHeader"))
        {
            match reader.stream(file, cutoff) {
                Ok(bytes) if bytes.starts_with(b"HWP Document File") => {
                    return Ok((
                        b"Hancom HWP (Hangul Word Processor) file, version 5.0".to_vec(),
                        "application/x-hwp",
                    ));
                }
                Ok(_) | Err(Error::Malformed { .. }) => {}
                Err(e) => return Err(e),
            }
        }
        let mut total = 0;
        for name in ["\u{5}SummaryInformation", "\u{5}DocumentSummaryInformation"] {
            if let Some(dir) = facts
                .directories
                .iter()
                .find(|d| d.kind == 2 && d.named(name))
            {
                match reader
                    .stream(dir, cutoff)
                    .and_then(|bytes| properties(&bytes, name, budget, &mut total))
                {
                    Ok(sets) => facts.property_sets.extend(sets),
                    Err(Error::Malformed { stage, .. }) => {
                        facts.diagnostics.push(format!("{name:?}: {stage}"))
                    }
                    Err(e) => return Err(e),
                }
            }
        }
        if profile == SemanticProfile::CorrectedV1 {
            if let Some(summary) = corrected_summary(&mut facts, time, budget)? {
                return Ok(summary);
            }
            if let Some(dir) = facts
                .directories
                .iter()
                .find(|d| d.kind == 2 && d.named("Catalog"))
            {
                match reader
                    .stream(dir, cutoff)
                    .and_then(|bytes| catalog(&bytes, budget))
                {
                    Ok(entries) => facts.catalog = entries,
                    Err(Error::Malformed { stage, .. }) => {
                        facts.diagnostics.push(format!("Catalog: {stage}"))
                    }
                    Err(e) => return Err(e),
                }
                if !facts.catalog.is_empty() {
                    let names: Vec<_> = facts
                        .catalog
                        .iter()
                        .skip(1)
                        .map(|e| String::from_utf16_lossy(&e.name))
                        .collect();
                    budget
                        .charge_retained(names.iter().map(|n| n.len().saturating_mul(8)).sum())?;
                    return Ok((
                        format!("Microsoft Thumbs.db [{}]", names.join(", ")).into_bytes(),
                        "application/CDFV2",
                    ));
                }
            }
        }
        if let Some((name, mime)) = dir_type(&facts.directories) {
            return Ok((format!("CDFV2 {name}").into_bytes(), mime));
        }
        // The pin's property stream bounds check uses a stack copy's address.
        // Reproduce its observed fallback with checked data, never bad pointers.
        Err(bad("Cannot read section info", 0))
    })();
    let (description, mime) = match result {
        Ok(value) => value,
        Err(Error::Malformed { stage, .. }) => {
            facts.failure_stage = Some(stage);
            (
                format!("Composite Document File V2 Document, {stage}").into_bytes(),
                "application/x-ole-storage",
            )
        }
        Err(e) => return Err(e),
    };
    Ok(Some(Detection {
        description,
        mime,
        facts,
    }))
}
