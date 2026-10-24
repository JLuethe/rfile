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
use std::collections::HashSet;
const SIGNATURE: &[u8] = &[0xd0, 0xcf, 0x11, 0xe0, 0xa1, 0xb1, 0x1a, 0xe1];
fn u16le(b: &[u8], p: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        b.get(p..p.checked_add(2)?)?.try_into().ok()?,
    ))
}
fn u32le(b: &[u8], p: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        b.get(p..p.checked_add(4)?)?.try_into().ok()?,
    ))
}
fn u64le(b: &[u8], p: usize) -> Option<u64> {
    Some(u64::from_le_bytes(
        b.get(p..p.checked_add(8)?)?.try_into().ok()?,
    ))
}
struct Reader<'a> {
    b: &'a [u8],
    sector: usize,
    mini: usize,
    fat: Vec<u32>,
    minifat: Vec<u32>,
    root: Vec<u8>,
}
struct Directory {
    name: String,
    kind: u8,
    first: u32,
    size: usize,
}
impl Reader<'_> {
    fn sector(&self, id: u32) -> Option<&[u8]> {
        let start = (id as usize).checked_add(1)?.checked_mul(self.sector)?;
        self.b.get(start..start.checked_add(self.sector)?)
    }
    fn chain(&self, first: u32, small: bool) -> Option<Vec<u8>> {
        let (table, sector, available) = if small {
            (&self.minifat, self.mini, self.root.len() / self.mini)
        } else {
            (&self.fat, self.sector, self.b.len() / self.sector)
        };
        let mut out = Vec::new();
        let mut id = first;
        let mut seen = HashSet::new();
        while id != 0xfffffffe {
            if id >= 0xfffffffa || !seen.insert(id) || seen.len() > available.min(250_000) {
                return None;
            }
            let bytes = if small {
                let offset = (id as usize).checked_mul(sector)?;
                self.root.get(offset..offset.checked_add(sector)?)?
            } else {
                self.sector(id)?
            };
            if out.len().checked_add(bytes.len())? > 128 * 1024 * 1024 {
                return None;
            }
            out.extend_from_slice(bytes);
            id = *table.get(id as usize)?;
        }
        Some(out)
    }
    fn stream(&self, dir: &Directory, cutoff: usize) -> Option<Vec<u8>> {
        let mut b = self.chain(dir.first, dir.size < cutoff)?;
        if dir.size > b.len() {
            return None;
        }
        b.truncate(dir.size);
        Some(b)
    }
}
fn inspect(b: &[u8]) -> std::result::Result<(Vec<u8>, &'static str), &'static str> {
    let shift = u16le(b, 30).ok_or("Can't read SAT")?;
    let mini_shift = u16le(b, 32).ok_or("Can't read SAT")?;
    if shift > 20 || mini_shift > 20 || shift < 7 || mini_shift < 2 || mini_shift > shift {
        return Err("Can't read SAT");
    }
    let mut r = Reader {
        b,
        sector: 1 << shift,
        mini: 1 << mini_shift,
        fat: Vec::new(),
        minifat: Vec::new(),
        root: Vec::new(),
    };
    let fat_count = u32le(b, 44).ok_or("Can't read SAT")? as usize;
    if fat_count > 250_000 || fat_count > b.len() / r.sector {
        return Err("Can't read SAT");
    }
    let mut fat_ids = Vec::new();
    for p in (76..512).step_by(4) {
        let id = u32le(b, p).ok_or("Can't read SAT")?;
        if id < 0xfffffffa {
            fat_ids.push(id);
        }
    }
    let mut next = u32le(b, 68).ok_or("Can't read SAT")?;
    let difat_count = u32le(b, 72).ok_or("Can't read SAT")? as usize;
    let mut seen = HashSet::new();
    if difat_count > b.len() / r.sector {
        return Err("Can't read SAT");
    }
    for _ in 0..difat_count {
        if !seen.insert(next) {
            return Err("Can't read SAT");
        }
        let sector = r.sector(next).ok_or("Can't read SAT")?;
        for p in (0..sector.len() - 4).step_by(4) {
            let id = u32le(sector, p).ok_or("Can't read SAT")?;
            if id < 0xfffffffa {
                fat_ids.push(id);
            }
        }
        next = u32le(sector, sector.len() - 4).ok_or("Can't read SAT")?;
    }
    if fat_ids.len() < fat_count {
        return Err("Can't read SAT");
    }
    for id in fat_ids.into_iter().take(fat_count) {
        let bytes = r.sector(id).ok_or("Can't read SAT")?;
        let entries: Vec<_> = bytes
            .chunks_exact(4)
            .map(|b| u32::from_le_bytes(b.try_into().expect("FAT entry")))
            .collect();
        r.fat.extend(entries);
    }
    let minifat_first = u32le(b, 60).ok_or("Can't read SSAT")?;
    let minifat_count = u32le(b, 64).ok_or("Can't read SSAT")?;
    if minifat_count > 0 {
        let bytes = r.chain(minifat_first, false).ok_or("Can't read SSAT")?;
        r.minifat = bytes
            .chunks_exact(4)
            .map(|b| u32::from_le_bytes(b.try_into().expect("mini FAT entry")))
            .collect();
    }
    let first_dir = u32le(b, 48).ok_or("Can't read directory")?;
    let directory = r.chain(first_dir, false).ok_or("Can't read directory")?;
    let mut dirs = Vec::new();
    for d in directory.chunks_exact(128) {
        let n = u16le(d, 64).ok_or("Can't read directory")? as usize;
        if d[66] == 0 {
            continue;
        }
        if !(2..=64).contains(&n) || n % 2 != 0 {
            return Err("Can't read directory");
        }
        let name = String::from_utf16_lossy(
            &d[..n - 2]
                .chunks_exact(2)
                .map(|b| u16::from_le_bytes([b[0], b[1]]))
                .collect::<Vec<_>>(),
        );
        let size = u64le(d, 120).ok_or("Can't read directory")?;
        let size = if shift == 9 { size & 0xffffffff } else { size };
        let size = usize::try_from(size).map_err(|_| "Can't read directory")?;
        dirs.push(Directory {
            name,
            kind: d[66],
            first: u32le(d, 116).ok_or("Can't read directory")?,
            size,
        });
    }
    let root = dirs
        .iter()
        .find(|d| d.kind == 5)
        .ok_or("Cannot read short stream")?;
    if root.size > 0 {
        r.root = r
            .chain(root.first, false)
            .ok_or("Cannot read short stream")?;
        if root.size > r.root.len() {
            return Err("Cannot read short stream");
        }
        r.root.truncate(root.size);
    }
    let cutoff = u32le(b, 56).ok_or("Cannot read short stream")? as usize;
    if let Some(file) = dirs.iter().find(|d| d.kind == 2 && d.name == "FileHeader") {
        if let Some(stream) = r.stream(file, cutoff) {
            if stream.starts_with(b"HWP Document File") {
                return Ok((
                    b"Hancom HWP (Hangul Word Processor) file, version 5.0".to_vec(),
                    "application/x-hwp",
                ));
            }
        }
    }
    // Distinguish known storage types only from validated directory entries.
    for d in &dirs {
        if d.kind != 2 && d.kind != 1 {
            continue;
        }
        let identified = match (d.name.as_str(), d.kind) {
            ("EncryptedPackage" | "EncryptedSummary", 2) => Some(("Encrypted", "encrypted")),
            ("mfbu_header", 2) => Some(("QuickBooks", "quickbooks")),
            ("Book" | "Workbook", 2) => Some(("Microsoft Excel", "vnd.ms-excel")),
            ("WordDocument", 2) => Some(("Microsoft Word", "msword")),
            ("PowerPoint Document", 2) => Some(("Microsoft PowerPoint", "vnd.ms-powerpoint")),
            ("__properties_version1.0", 2) | ("__recip_version1.0_#00000000", 1) => {
                Some(("Microsoft Outlook Message", "vnd.ms-outlook"))
            }
            _ => None,
        };
        if let Some((name, mime)) = identified {
            return Ok((
                format!("CDFV2 {name}").into_bytes(),
                match mime {
                    "encrypted" => "application/encrypted",
                    "quickbooks" => "application/quickbooks",
                    "vnd.ms-excel" => "application/vnd.ms-excel",
                    "msword" => "application/msword",
                    "vnd.ms-powerpoint" => "application/vnd.ms-powerpoint",
                    _ => "application/vnd.ms-outlook",
                },
            ));
        }
    }
    Err("")
}
pub(crate) fn detect(b: &[u8]) -> Option<(Vec<u8>, &'static str)> {
    if b.len() < 512 || !b.starts_with(SIGNATURE) {
        return None;
    }
    if u16le(b, 30)? > 20 || u16le(b, 32)? > 20 {
        return None;
    }
    Some(match inspect(b) {
        Ok(x) => x,
        Err(reason) => {
            let mut description = b"Composite Document File V2 Document".to_vec();
            if !reason.is_empty() {
                description.extend_from_slice(b", ");
                description.extend_from_slice(reason.as_bytes());
            }
            (description, "application/x-ole-storage")
        }
    })
}
