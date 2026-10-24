/*
 * Copyright (c) Christos Zoulas 2003.
 * All Rights Reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice immediately at the beginning of the file, without modification,
 *    this list of conditions, and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 *
 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED. IN NO EVENT SHALL THE AUTHOR OR CONTRIBUTORS BE LIABLE FOR
 * ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 */
// rfile contributions: Copyright (C) 2026 rfile contributors.
// SPDX-License-Identifier: GPL-3.0-only

// Independent Rust implementation; upstream behavior and retained data notices credited in NOTICE.
//! Checked ELF metadata enrichment for descriptor/path entry points.
use crate::{
    Error, InspectionBudget, Limits, Result,
    input::{self, FileView, InputView, InspectionBytes},
};
use std::fs::File;
#[derive(Clone, Debug, Default)]
pub struct NoteEvidence {
    pub owner: Vec<u8>,
    pub owner_bytes: usize,
    pub kind: u32,
    pub description_bytes: usize,
    pub source_offset: u64,
}
#[derive(Clone, Debug, Default)]
pub struct Info {
    pub suffix: Vec<u8>,
    pub executable: bool,
    pub class: u8,
    pub data_encoding: u8,
    pub machine: u16,
    pub file_type: u16,
    pub notes: Vec<NoteEvidence>,
}
struct Elf<'a> {
    view: &'a mut dyn InputView,
    budget: &'a InspectionBudget,
    big: bool,
    is64: bool,
    read_bytes: usize,
    limits: &'a Limits,
    profile: crate::SemanticProfile,
    raw: bool,
}
impl Elf<'_> {
    fn integer(&self, b: &[u8], p: usize, n: usize) -> Option<u64> {
        let b = b.get(p..p.checked_add(n)?)?;
        Some(if self.big {
            b.iter().fold(0, |n, &c| (n << 8) | u64::from(c))
        } else {
            b.iter().rev().fold(0, |n, &c| (n << 8) | u64::from(c))
        })
    }
    fn u16(&self, b: &[u8], p: usize) -> Option<usize> {
        Some(self.integer(b, p, 2)? as usize)
    }
    fn u32(&self, b: &[u8], p: usize) -> Option<u64> {
        self.integer(b, p, 4)
    }
    fn word(&self, b: &[u8], p: usize) -> Option<u64> {
        self.integer(b, p, if self.is64 { 8 } else { 4 })
    }
    fn read(&mut self, offset: u64, size: usize) -> Result<InspectionBytes> {
        self.read_bytes = self
            .read_bytes
            .checked_add(size)
            .ok_or(Error::Limit("ELF read budget"))?;
        if size > self.limits.elf_shsize || self.read_bytes > self.limits.work_bytes {
            return Err(Error::Limit("ELF read bytes"));
        }
        input::read_range(self.view, offset, size, self.budget)
    }
}
#[path = "elf_capabilities.rs"]
mod capabilities;
fn hex(value: u64) -> String {
    if value == 0 {
        "0".into()
    } else {
        format!("{value:#x}")
    }
}
fn render_capabilities(
    out: &mut Output<'_>,
    machine: usize,
    mut hw: u64,
    mut sf: u64,
) -> Result<()> {
    if hw != 0 {
        out.push_str(", uses")?;
        let table = match machine {
            2 | 18 | 43 => capabilities::CAP_SPARC,
            3 | 50 | 62 => capabilities::CAP_386,
            _ => &[],
        };
        for &(mask, name) in table {
            if hw & mask != 0 {
                out.push_str(&format!(" {name}"))?;
                hw &= !mask;
            }
        }
        if hw != 0 {
            out.push_str(&format!(
                " {}hardware capability {hw:#x}",
                if table.is_empty() { "" } else { "unknown " }
            ))?;
        }
    }
    if sf & 2 != 0 {
        out.push_str(if sf & 1 != 0 {
            ", uses frame pointer"
        } else {
            ", not known to use frame pointer"
        })?;
    }
    sf &= !3;
    if sf != 0 {
        out.push_str(&format!(", with unknown software capability {sf:#x}"))?;
    }
    Ok(())
}
struct Output<'a> {
    bytes: InspectionBytes,
    budget: &'a InspectionBudget,
}
impl<'a> Output<'a> {
    fn new(budget: &'a InspectionBudget) -> Result<Self> {
        Ok(Self {
            bytes: InspectionBytes::new(budget)?,
            budget,
        })
    }
    fn raw(&mut self, bytes: &[u8]) -> Result<()> {
        if self.bytes.len().saturating_add(bytes.len()) > self.budget.limits.output {
            return Err(Error::Limit("ELF output"));
        }
        self.bytes.extend(bytes)
    }
    fn printf(&mut self, parts: &[&[u8]]) -> Result<()> {
        let len = parts.iter().fold(0usize, |n, p| n.saturating_add(p.len()));
        if len > 1024 {
            return Err(Error::LegacyOutput {
                attempted: len,
                previous: self.len(),
            });
        }
        for part in parts {
            self.raw(part)?;
        }
        Ok(())
    }
    fn push_str(&mut self, text: &str) -> Result<()> {
        self.printf(&[text.as_bytes()])
    }
    fn push(&mut self, c: char) -> Result<()> {
        self.raw(c.encode_utf8(&mut [0u8; 4]).as_bytes())
    }
    fn len(&self) -> usize {
        self.bytes.len()
    }
    fn into_bytes(self) -> Vec<u8> {
        self.bytes.into_retained_vec()
    }
}
#[derive(Default)]
struct NoteState {
    used: usize,
    bid: bool,
    os: bool,
    pax: bool,
    memtag: bool,
    core: bool,
    style: Option<u8>,
    auxv: bool,
    netbsd: u8,
    is_core: bool,
    phoff: u64,
    phnum: usize,
    evidence: Vec<NoteEvidence>,
}
fn freebsd_version(n: u32) -> String {
    let mut out = String::from(", for FreeBSD");
    if n == 460002 {
        out.push_str(" 4.6.2");
    } else if n < 460100 {
        out.push_str(&format!(" {}.{}", n / 100000, n / 10000 % 10));
        if n / 1000 % 10 > 0 {
            out.push_str(&format!(".{}", n / 1000 % 10));
        }
        if n % 1000 > 0 || n % 100000 == 0 {
            out.push_str(&format!(" ({})", n as i32));
        }
    } else if n < 500000 {
        out.push_str(&format!(
            " {}.{}",
            n / 100000,
            n / 10000 % 10 + n / 1000 % 10
        ));
        if n / 100 % 10 > 0 {
            out.push_str(&format!(" ({})", n as i32));
        } else if n / 10 % 10 > 0 {
            out.push_str(&format!(".{}", n / 10 % 10));
        }
    } else {
        out.push_str(&format!(" {}.{}", n / 100000, n / 1000 % 100));
        if n / 100 % 10 > 0 || n % 100000 / 100 == 0 {
            out.push_str(&format!(" ({})", n as i32));
        } else if n / 10 % 10 > 0 {
            out.push_str(&format!(".{}", n / 10 % 10));
        }
    }
    out
}
fn netbsd_version(n: u32) -> String {
    let mut out = String::from(", for NetBSD");
    if n > 100000000 {
        let major = n / 100000000;
        let minor = n / 1000000 % 100;
        let mut release = n / 10000 % 100;
        let mut patch = n / 100 % 100;
        out.push_str(&format!(" {major}.{minor}"));
        if major >= 9 {
            patch += 100 * release;
            release = 0;
        }
        if release == 0 && patch != 0 {
            out.push_str(&format!(".{patch}"));
        } else if release != 0 {
            while release > 26 {
                out.push('Z');
                release -= 26;
            }
            out.push((b'A' + release as u8 - 1) as char);
        }
    }
    out
}
fn printable_bytes(bytes: &[u8], capacity: usize, raw: bool) -> Vec<u8> {
    let mut out = Vec::new();
    for &c in crate::database::cstr(bytes) {
        if out.len() >= capacity.saturating_sub(1) {
            break;
        }
        if raw || (32..=126).contains(&c) {
            out.push(c);
        } else {
            if out.len().saturating_add(4) >= capacity {
                break;
            }
            out.extend_from_slice(format!("\\{c:03o}").as_bytes());
        }
    }
    out
}
fn core_note(
    e: &Elf<'_>,
    region: (&[u8], usize),
    data: &[u8],
    name: &[u8],
    kind: u64,
    out: &mut Output<'_>,
    state: &mut NoteState,
) -> Result<bool> {
    let (b, start) = region;
    let style = if name == b"CORE" || name == b"CORE\0" {
        Some(0)
    } else if name == b"FreeBSD\0" {
        Some(1)
    } else if name.starts_with(b"NetBSD-CORE") {
        Some(2)
    } else {
        None
    };
    if let Some(style) = style {
        if state.style.is_none() {
            out.push_str([", SVR4-style", ", FreeBSD-style", ", NetBSD-style"][style as usize])?;
            state.style = Some(style);
        }
    }
    if style == Some(2) && kind == 1 {
        let mut pi = [0u8; 160];
        let n = data.len().min(pi.len());
        pi[..n].copy_from_slice(&data[..n]);
        let name = printable_bytes(&pi[124..156], 512, e.raw);
        out.push_str(", from '")?;
        out.raw(&name[..name.len().min(31)])?;
        out.push_str(&format!(
            "', pid={}, uid={}, gid={}, nlwps={}, lwp={} (signal {}/code {})",
            e.u32(&pi, 80).unwrap_or(0),
            e.u32(&pi, 100).unwrap_or(0),
            e.u32(&pi, 112).unwrap_or(0),
            e.u32(&pi, 120).unwrap_or(0),
            e.u32(&pi, 156).unwrap_or(0),
            e.u32(&pi, 8).unwrap_or(0),
            e.u32(&pi, 12).unwrap_or(0)
        ))?;
        state.core = true;
        return Ok(true);
    }
    if !state.is_core || kind != 3 {
        return Ok(false);
    }
    if style == Some(1) {
        let args = if e.is64 { 33 } else { 25 };
        let enclosing = if e.profile == crate::SemanticProfile::Pinned548 {
            &b[start..]
        } else {
            data
        };
        if let Some(bytes) = enclosing.get(args..args + 81) {
            out.push_str(", from '")?;
            out.raw(crate::database::cstr(&bytes[..80]))?;
            out.push_str("'")?;
        }
        let pid = args + 83;
        if let Some(value) = e.u32(enclosing, pid) {
            out.push_str(&format!(", pid={value}"))?;
        }
        state.core = true;
        return Ok(false);
    }
    let offsets = if e.is64 {
        &[136usize, 120, 56, 40, 16][..]
    } else {
        &[100usize, 84, 44, 28, 48, 32, 8][..]
    };
    for i in 0..offsets.len() {
        e.budget.step()?;
        let at = offsets[i];
        let mut length = 0;
        let mut valid = true;
        for j in 0..16 {
            let Some(&c) = data.get(at + j) else {
                valid = false;
                break;
            };
            if c == 0 {
                if j == 0 {
                    valid = false;
                }
                break;
            }
            if !(32..=126).contains(&c) || b"'\"`".contains(&c) {
                valid = false;
                break;
            }
            length += 1;
        }
        if !valid {
            continue;
        }
        let mut chosen = i;
        for k in i + 1..offsets.len() {
            if offsets[k] >= offsets[chosen] || offsets[k] + 16 == offsets[chosen] && length == 16 {
                continue;
            }
            if data
                .get(offsets[k]..offsets[chosen])
                .is_some_and(|s| s.iter().all(|c| (32..=126).contains(c)))
            {
                chosen = k;
            }
        }
        let enclosing = if e.profile == crate::SemanticProfile::Pinned548 {
            &b[start..]
        } else {
            data
        };
        let tail = enclosing.get(offsets[chosen]..).unwrap_or(&[]);
        let n = tail
            .iter()
            .position(|c| !(32..=126).contains(c))
            .unwrap_or(tail.len());
        let mut bytes = &tail[..n];
        while bytes.last().is_some_and(u8::is_ascii_whitespace) {
            bytes = &bytes[..bytes.len() - 1];
        }
        out.push_str(", from '")?;
        out.raw(crate::database::cstr(&bytes[..bytes.len().min(255)]))?;
        out.push_str("'")?;
        state.core = true;
        return Ok(true);
    }
    Ok(false)
}
fn virtual_string(
    e: &mut Elf<'_>,
    address: u64,
    state: &NoteState,
    out: &mut Output<'_>,
) -> Result<Option<String>> {
    let mut mapped = None;
    let size = if e.is64 { 56 } else { 32 };
    for i in 0..state.phnum {
        e.budget.step()?;
        let offset = state
            .phoff
            .checked_add((i * size) as u64)
            .ok_or(Error::Limit("ELF virtual mapping"))?;
        let ph = e.read(offset, size)?;
        if ph.len() < size {
            out.push_str(&format!(", can't read elf program header at {offset}"))?;
            break;
        }
        let fileoff = e.word(&ph, if e.is64 { 8 } else { 4 }).unwrap_or(0);
        let base = e.word(&ph, if e.is64 { 16 } else { 8 }).unwrap_or(0);
        let len = e.word(&ph, if e.is64 { 32 } else { 16 }).unwrap_or(0);
        if address >= base && address.checked_sub(base).is_some_and(|delta| delta < len) {
            mapped = fileoff.checked_add(address - base);
            break;
        }
    }
    let offset = match mapped {
        Some(n) => n,
        None if e.profile == crate::SemanticProfile::Pinned548 => 0,
        None => return Ok(None),
    };
    let bytes = e.read(offset, 256)?;
    if bytes.is_empty() {
        out.push_str(&format!(", can't read elf string at {offset}"))?;
        return Ok(None);
    }
    let bytes = crate::database::cstr(&bytes[..bytes.len() - 1]);
    if bytes.is_empty() || !bytes.iter().all(|c| (32..=126).contains(c)) {
        return Ok(None);
    }
    Ok(Some(
        String::from_utf8(bytes.to_vec()).expect("ASCII string"),
    ))
}
fn notes(
    e: &mut Elf<'_>,
    b: &[u8],
    align: usize,
    out: &mut Output<'_>,
    state: &mut NoteState,
    origin: u64,
) -> Result<()> {
    let mut p = 0usize;
    while p < b.len() {
        e.budget.step()?;
        let record_offset = p;
        if state.used >= e.limits.elf_notes {
            break;
        }
        state.used += 1;
        let Some(header) = b.get(p..p.saturating_add(12)) else {
            break;
        };
        let names = e.u32(header, 0).unwrap_or(0) as usize;
        let desc = e.u32(header, 4).unwrap_or(0) as usize;
        let kind = e.u32(header, 8).unwrap_or(0);
        p += 12;
        if names == 0 && desc == 0 {
            break;
        }
        if names & 0x80000000 != 0 {
            out.push_str(&format!(", bad note name size {names:#x}"))?;
            break;
        }
        if desc & 0x80000000 != 0 {
            out.push_str(&format!(", bad note description size {desc:#x}"))?;
            break;
        }
        let round = |n: usize| n.checked_add(align - 1).map(|v| v / align * align);
        let Some(name_end) = p.checked_add(names) else {
            break;
        };
        let Some(name) = b.get(p..name_end) else {
            break;
        };
        let Some(start) = round(name_end) else {
            break;
        };
        let Some(end) = start.checked_add(desc) else {
            break;
        };
        let Some(data) = b.get(start..end) else {
            break;
        };
        let Some(next) = round(end) else {
            break;
        };
        if next <= p {
            break;
        }
        p = next;
        e.budget.charge_retained(1024)?;
        state.evidence.push(NoteEvidence {
            owner: name[..name.len().min(128)].to_vec(),
            owner_bytes: name.len(),
            kind: kind as u32,
            description_bytes: desc,
            source_offset: origin
                .checked_add(record_offset as u64)
                .ok_or(Error::Limit("ELF note offset"))?,
        });
        if !state.os {
            let version = e.u32(data, 0).unwrap_or(0) as u32;
            let recognized = if name == b"GNU\0" && kind == 1 && desc == 16 {
                let os = match version {
                    0 => "Linux",
                    1 => "Hurd",
                    2 => "Solaris",
                    3 => "kFreeBSD",
                    4 => "kNetBSD",
                    _ => "<unknown>",
                };
                out.push_str(&format!(
                    ", for GNU/{os} {}.{}.{}",
                    e.u32(data, 4).unwrap_or(0) as u32 as i32,
                    e.u32(data, 8).unwrap_or(0) as u32 as i32,
                    e.u32(data, 12).unwrap_or(0) as u32 as i32
                ))?;
                true
            } else if name == b"SuSE\0" && kind == 1 && desc == 2 {
                out.push_str(&format!(", for SuSE {}.{}", data[0], data[1]))?;
                true
            } else if name == b"OpenBSD\0" && kind == 1 && desc == 4 {
                out.push_str(", for OpenBSD")?;
                true
            } else if name == b"FreeBSD\0" && kind == 1 && desc == 4 {
                out.push_str(&freebsd_version(version))?;
                true
            } else if name == b"NetBSD\0" && kind == 1 && desc == 4 {
                out.push_str(&netbsd_version(version))?;
                true
            } else if name == b"DragonFly\0" && kind == 1 && desc == 4 {
                out.push_str(&format!(
                    ", for DragonFly {}.{}.{}",
                    version / 100000,
                    version / 10000 % 10,
                    version % 10000
                ))?;
                true
            } else if name == b"Android\0" && kind == 1 && desc >= 4 {
                out.push_str(&format!(", for Android {}", version as i32))?;
                if desc >= 132 {
                    out.push_str(", built by NDK ")?;
                    out.raw(crate::database::cstr(&data[4..68]))?;
                    out.push_str(" (")?;
                    out.raw(crate::database::cstr(&data[68..132]))?;
                    out.push_str(")")?;
                }
                state.os = true;
                false
            } else {
                false
            };
            if recognized {
                state.os = true;
                continue;
            }
        }
        if !state.bid {
            if name == b"GNU\0" && kind == 3 && (4..=20).contains(&desc) {
                let typ = match desc {
                    8 => "xxHash",
                    16 => "md5/uuid",
                    20 => "sha1",
                    _ => "unknown",
                };
                out.push_str(&format!(", BuildID[{typ}]="))?;
                for c in data {
                    out.push_str(&format!("{c:02x}"))?;
                }
                state.bid = true;
                continue;
            }
            if names == 4 && name.starts_with(b"Go\0") && kind == 4 && desc < 128 {
                out.push_str(", Go BuildID=")?;
                out.raw(crate::database::cstr(&data[..data.len().min(255)]))?;
                continue;
            }
        }
        let mut handled_flags = false;
        for (owner, nt, done, labels, prefix) in [
            (
                b"PaX\0".as_slice(),
                3,
                &mut state.pax,
                &[
                    "+mprotect",
                    "-mprotect",
                    "+segvguard",
                    "-segvguard",
                    "+ASLR",
                    "-ASLR",
                ][..],
                ", PaX: ",
            ),
            (
                b"Android\0".as_slice(),
                4,
                &mut state.memtag,
                &["none", "async", "sync", "heap", "stack"][..],
                ", Android Memtag: ",
            ),
        ] {
            if !*done && name == owner && kind == nt && desc == 4 {
                *done = true;
                handled_flags = true;
                let bits = e.u32(data, 0).unwrap_or(0);
                if bits != 0 {
                    out.push_str(prefix)?;
                }
                let mut first = true;
                for (i, label) in labels.iter().enumerate() {
                    if bits & (1 << i) != 0 {
                        if !first {
                            out.push(',')?;
                        }
                        out.push_str(label)?;
                        first = false;
                    }
                }
            }
        }
        if handled_flags {
            continue;
        }
        if !state.core && core_note(e, (b, start), data, name, kind, out, state)? {
            continue;
        }
        if !state.auxv && state.is_core && state.style == Some(0) && kind == 6 {
            state.auxv = true;
            let width = if e.is64 { 16 } else { 8 };
            for (i, entry) in data.chunks_exact(width).enumerate() {
                if i >= 50 {
                    out.push_str(" Too many ELF Auxv elements")?;
                    e.budget.charge_retained(out.len())?;
                    return Err(Error::LegacyLimit(out.bytes.to_vec()));
                }
                e.budget.step()?;
                let tag = e.word(entry, 0).unwrap_or(0);
                let value = e.word(entry, width / 2).unwrap_or(0);
                match tag {
                    31 | 15 => {
                        if let Some(text) = virtual_string(e, value, state, out)? {
                            out.push_str(&format!(
                                ", {}: '{text}'",
                                if tag == 31 { "execfn" } else { "platform" }
                            ))?;
                        }
                    }
                    11..=14 => out.push_str(&format!(
                        ", {}: {}",
                        match tag {
                            11 => "real uid",
                            12 => "effective uid",
                            13 => "real gid",
                            _ => "effective gid",
                        },
                        value as u32 as i32
                    ))?,
                    _ => {}
                }
            }
            continue;
        }
        if name == b"NetBSD\0" && kind != 1 {
            let (bit, tag) = match kind {
                5 => (1, "compiled for"),
                6 => (2, "compiler model"),
                2 => (4, "emulation:"),
                _ => (8, ""),
            };
            if state.netbsd & bit == 0 {
                state.netbsd |= bit;
                if bit == 8 {
                    out.push_str(&format!(", note={kind}"))?;
                } else {
                    out.push_str(&format!(", {tag}: "))?;
                    out.raw(crate::database::cstr(&data[..desc.min(100)]))?;
                }
            }
        }
        if out.len() > e.limits.output {
            return Err(Error::Limit("ELF output"));
        }
    }
    Ok(())
}
pub(crate) fn analyze(
    file: &mut File,
    head: &[u8],
    executable: bool,
    mime_only: bool,
    budget: &InspectionBudget,
    profile: crate::SemanticProfile,
    raw: bool,
) -> Result<Option<Info>> {
    inspect(
        &mut FileView::new(file)?,
        head,
        Options {
            executable,
            mime_only,
            profile,
            raw,
        },
        budget,
    )
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Options {
    pub executable: bool,
    pub mime_only: bool,
    pub profile: crate::SemanticProfile,
    pub raw: bool,
}
/// Descriptor-style ELF inspection over a caller-owned positional view. Buffer
/// classification does not invoke this API or invent filesystem evidence.
pub fn inspect(
    view: &mut dyn InputView,
    head: &[u8],
    options: Options,
    budget: &InspectionBudget,
) -> Result<Option<Info>> {
    let Options {
        executable,
        mime_only,
        profile,
        raw,
    } = options;
    let limits = &budget.limits;
    if head.len() < 16 || !(head.starts_with(b"\x7fELF") || head.starts_with(b"\x7fOLF")) {
        return Ok(None);
    }
    let _format_workspace = budget.reserve(4096)?;
    let class = head[4];
    if !matches!(class, 1 | 2) {
        return Ok(Some(Info {
            suffix: format!(", unknown class {class}").into_bytes(),
            executable,
            class,
            ..Info::default()
        }));
    }
    let mut e = Elf {
        view,
        budget,
        big: head[5] != 1,
        is64: class == 2,
        read_bytes: 0,
        limits,
        profile,
        raw,
    };
    let header_size = if e.is64 { 64 } else { 52 };
    if head.len() <= header_size {
        return Ok(None);
    }
    let kind = e.u16(head, 16).unwrap_or(0);
    let mut mode = executable;
    let mut out = Output::new(budget)?;
    let (phoff, shoff, phsize, phnum, shsize, shnum, strtab) = if e.is64 {
        (
            e.word(head, 32),
            e.word(head, 40),
            e.u16(head, 54),
            e.u16(head, 56),
            e.u16(head, 58),
            e.u16(head, 60),
            e.u16(head, 62),
        )
    } else {
        (
            e.word(head, 28),
            e.word(head, 32),
            e.u16(head, 42),
            e.u16(head, 44),
            e.u16(head, 46),
            e.u16(head, 48),
            e.u16(head, 50),
        )
    };
    let (
        Some(phoff),
        Some(shoff),
        Some(phsize),
        Some(mut phnum),
        Some(shsize),
        Some(mut shnum),
        Some(mut strtab),
    ) = (phoff, shoff, phsize, phnum, shsize, shnum, strtab)
    else {
        return Ok(None);
    };
    if profile == crate::SemanticProfile::CorrectedV1
        && shoff != 0
        && (phnum == 0xffff || shnum == 0 || strtab == 0xffff)
    {
        let expected = if e.is64 { 64 } else { 40 };
        if shsize != expected {
            return Err(Error::Malformed {
                format: "ELF",
                stage: "extended section header size",
                offset: shoff,
            });
        }
        let zero = e.read(shoff, expected)?;
        if zero.len() != expected {
            return Err(Error::Malformed {
                format: "ELF",
                stage: "extended section zero",
                offset: shoff,
            });
        }
        if phnum == 0xffff {
            phnum = e.u32(&zero, if e.is64 { 44 } else { 28 }).unwrap_or(0) as usize;
        }
        if shnum == 0 {
            shnum = usize::try_from(e.word(&zero, if e.is64 { 32 } else { 20 }).unwrap_or(0))
                .map_err(|_| Error::Limit("ELF extended sections"))?;
        }
        if strtab == 0xffff {
            strtab = e.u32(&zero, if e.is64 { 40 } else { 24 }).unwrap_or(0) as usize;
        }
    }
    let excess = match kind {
        4 if phnum > limits.elf_phnum => Some(("program headers", phnum)),
        2 | 3 if phnum > limits.elf_phnum => Some(("program", phnum)),
        2 | 3 if shnum > limits.elf_shnum => Some(("section", shnum)),
        1 if shnum > limits.elf_shnum => Some(("section headers", shnum)),
        _ => None,
    };
    if let Some((label, count)) = excess {
        if !mime_only {
            out.push_str(&format!(", too many {label} ({count})"))?;
        }
        return Ok(Some(Info {
            suffix: out.into_bytes(),
            executable: mode,
            class,
            data_encoding: head[5],
            machine: e.u16(head, 18).unwrap_or(0) as u16,
            file_type: kind as u16,
            ..Info::default()
        }));
    }
    if !matches!(kind, 1..=4) {
        return Ok(Some(Info {
            suffix: Vec::new(),
            executable: mode,
            class,
            data_encoding: head[5],
            machine: e.u16(head, 18).unwrap_or(0) as u16,
            file_type: kind as u16,
            ..Info::default()
        }));
    }
    let mut state = NoteState {
        is_core: kind == 4,
        phoff,
        phnum,
        ..NoteState::default()
    };
    if matches!(kind, 2 | 3) {
        if phnum == 0 {
            out.push_str(", no program header")?;
        } else if phsize != if e.is64 { 56 } else { 32 } {
            out.push_str(", corrupted program header size")?;
        } else {
            let mut program_complete = true;
            let mut dynamic = false;
            let mut pie = false;
            let mut needed = 0;
            let mut interp = Vec::new();
            for i in 0..phnum {
                let Some(offset) = phoff.checked_add((i * phsize) as u64) else {
                    return Err(Error::Limit("ELF program offset"));
                };
                let ph = e.read(offset, phsize)?;
                if ph.len() < phsize {
                    out.push_str(&format!(", can't read elf program headers at {offset}"))?;
                    program_complete = false;
                    break;
                }
                let typ = e.u32(&ph, 0).unwrap_or(0);
                let poff = e.word(&ph, if e.is64 { 8 } else { 4 }).unwrap_or(0);
                let psize = e.word(&ph, if e.is64 { 32 } else { 16 }).unwrap_or(0);
                let mut alignment = 4usize;
                if typ == 4 {
                    if shnum != 0 {
                        continue;
                    }
                    let declared = e.word(&ph, if e.is64 { 48 } else { 28 }).unwrap_or(0);
                    let declared = if declared == 0 { 4 } else { declared };
                    if declared & 0x80000000 != 0 || declared < 4 {
                        if !mime_only {
                            out.push_str(&format!(", invalid note alignment {declared:#x}"))?;
                        }
                    } else {
                        alignment =
                            usize::try_from(declared).map_err(|_| Error::Limit("ELF alignment"))?;
                    }
                }
                if matches!(typ, 2..=4) {
                    let b = e.read(poff, psize.min(2024) as usize)?;
                    match typ {
                        2 => {
                            dynamic = true;
                            mode = false;
                            let size = if e.is64 { 16 } else { 8 };
                            for d in b.chunks_exact(size) {
                                let tag = e.word(d, 0).unwrap_or(0);
                                let value = e.word(d, size / 2).unwrap_or(0);
                                if tag == 0x6ffffffb {
                                    if value & 0x08000000 != 0 {
                                        pie = true;
                                        mode = true;
                                    } else {
                                        mode = false;
                                    }
                                }
                                if tag == 1 {
                                    needed += 1;
                                }
                            }
                        }
                        3 => {
                            needed += 1;
                            interp = if b.is_empty() || b[0] == 0 {
                                b"*empty*".to_vec()
                            } else {
                                crate::database::cstr(&b[..b.len().saturating_sub(1)]).to_vec()
                            };
                        }
                        4 if shnum == 0 && !mime_only => {
                            notes(&mut e, &b, alignment, &mut out, &mut state, poff)?
                        }
                        4 if mime_only => break,
                        _ => {}
                    }
                }
            }
            if !mime_only && program_complete {
                out.push_str(if dynamic {
                    if pie && needed == 0 {
                        ", static-pie linked"
                    } else {
                        ", dynamically linked"
                    }
                } else {
                    ", statically linked"
                })?;
                if !interp.is_empty() {
                    out.printf(&[b", interpreter ", &printable_bytes(&interp, 2024, raw)])?;
                }
            }
        }
    }
    if kind == 4 && !mime_only {
        if phnum == 0 {
            out.push_str(", no program header")?;
        } else if phsize != if e.is64 { 56 } else { 32 } {
            out.push_str(", corrupted program header size")?;
        } else {
            for i in 0..phnum {
                budget.step()?;
                let offset = phoff
                    .checked_add((i * phsize) as u64)
                    .ok_or(Error::Limit("ELF program offset"))?;
                let ph = e.read(offset, phsize)?;
                if ph.len() < phsize {
                    out.push_str(&format!(", can't read elf program headers at {offset}"))?;
                    break;
                }
                if e.u32(&ph, 0) != Some(4) {
                    continue;
                }
                let offset = e.word(&ph, if e.is64 { 8 } else { 4 }).unwrap_or(0);
                let size = e
                    .word(&ph, if e.is64 { 32 } else { 16 })
                    .unwrap_or(0)
                    .min(2024) as usize;
                let bytes = e.read(offset, size)?;
                notes(&mut e, &bytes, 4, &mut out, &mut state, offset)?;
            }
        }
    }
    if kind == 4 {
        if !mime_only && state.used >= limits.elf_notes {
            out.push_str(&format!(", too many notes ({})", limits.elf_notes))?;
        }
        return Ok(Some(Info {
            suffix: out.into_bytes(),
            executable: mode,
            class,
            data_encoding: head[5],
            machine: e.u16(head, 18).unwrap_or(0) as u16,
            file_type: kind as u16,
            notes: state.evidence,
        }));
    }
    if !mime_only {
        if shnum == 0 {
            out.push_str(", no section header")?;
        } else if shsize != if e.is64 { 64 } else { 40 } {
            out.push_str(", corrupted section header size")?;
        } else {
            let Some(name_header_offset) = shoff.checked_add((strtab * shsize) as u64) else {
                return Err(Error::Limit("ELF section offset"));
            };
            let name_header = e.read(name_header_offset, shsize)?;
            if name_header.len() < shsize {
                out.push_str(&format!(
                    ", missing section headers at {name_header_offset}"
                ))?;
            } else {
                let name_offset = e
                    .word(&name_header, if e.is64 { 24 } else { 16 })
                    .unwrap_or(0);
                let mut previous_name = e.u32(&name_header, 0).unwrap_or(0) as usize;
                let mut cap_hw = 0u64;
                let mut cap_sf = 0u64;
                let mut bad_caps = 0usize;
                if e.view.known_len().is_some_and(|size| name_offset > size) {
                    out.push_str(&format!(", too large section header offset {name_offset}"))?;
                    return Ok(Some(Info {
                        suffix: out.into_bytes(),
                        executable: mode,
                        class,
                        data_encoding: head[5],
                        machine: e.u16(head, 18).unwrap_or(0) as u16,
                        file_type: kind as u16,
                        notes: state.evidence,
                    }));
                }
                let mut stripped = true;
                let mut debug = false;
                let mut complete = true;
                for i in 0..shnum {
                    let Some(offset) = shoff.checked_add((i * shsize) as u64) else {
                        return Err(Error::Limit("ELF section offset"));
                    };
                    let sh = e.read(offset, shsize)?;
                    if sh.len() < shsize {
                        out.push_str(&format!(", can't read elf section at {offset}"))?;
                        complete = false;
                        break;
                    }
                    let current_name = e.u32(&sh, 0).unwrap_or(0) as usize;
                    let name = if profile == crate::SemanticProfile::Pinned548 {
                        previous_name
                    } else {
                        current_name
                    };
                    previous_name = current_name;
                    let typ = e.u32(&sh, 4).unwrap_or(0);
                    if typ == 2 {
                        stripped = false;
                    }
                    let name_position = name_offset
                        .checked_add(name as u64)
                        .ok_or(Error::Limit("ELF section name offset"))?;
                    let name_bytes = e.read(name_position, 49)?;
                    if crate::database::cstr(&name_bytes) == b".debug_info" {
                        debug = true;
                        stripped = false;
                    }
                    if typ == 7 {
                        let offset = e.word(&sh, if e.is64 { 24 } else { 16 }).unwrap_or(0);
                        let size = e.word(&sh, if e.is64 { 32 } else { 20 }).unwrap_or(0);
                        if e.view.known_len().is_some_and(|file_size| {
                            offset.checked_add(size).is_none_or(|end| end > file_size)
                        }) {
                            let file_size = e.view.known_len().unwrap_or(0);
                            out.push_str(&format!(
                                ", note offset/size {}+{} exceeds file size {}",
                                hex(offset),
                                hex(size),
                                hex(file_size)
                            ))?;
                            complete = false;
                            break;
                        }
                        let size =
                            usize::try_from(size).map_err(|_| Error::Limit("ELF note size"))?;
                        let b = e.read(offset, size)?;
                        if b.len() != size {
                            out.push_str(&format!(", can't read elf note at {offset}"))?;
                            complete = false;
                            break;
                        }
                        notes(&mut e, &b, 4, &mut out, &mut state, offset)?;
                    }
                    if typ == 0x6ffffff5
                        && matches!(e.u16(head, 18), Some(2 | 3 | 43 | 50 | 62))
                        && bad_caps <= 5
                    {
                        let offset = e.word(&sh, if e.is64 { 24 } else { 16 }).unwrap_or(0);
                        let size = e.word(&sh, if e.is64 { 32 } else { 20 }).unwrap_or(0);
                        let width = if e.is64 { 16 } else { 8 };
                        let size = usize::try_from(size)
                            .map_err(|_| Error::Limit("ELF capability size"))?;
                        let data = e.read(offset, size)?;
                        if data.len() != size {
                            return Err(Error::Malformed {
                                format: "ELF",
                                stage: "capability short read",
                                offset,
                            });
                        }
                        for cap in data.chunks_exact(width) {
                            budget.step()?;
                            if cap[0] == b'A' {
                                break;
                            }
                            let tag = e.word(cap, 0).unwrap_or(0);
                            let value = e.word(cap, width / 2).unwrap_or(0);
                            match tag {
                                0 => {}
                                1 => cap_hw |= value,
                                2 => cap_sf |= value,
                                _ => {
                                    out.push_str(&format!(
                                        ", with unknown capability {} = {}",
                                        hex(tag),
                                        hex(value)
                                    ))?;
                                    let old = bad_caps;
                                    bad_caps += 1;
                                    if old > 2 {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
                if complete {
                    if debug {
                        out.push_str(", with debug_info")?;
                    }
                    out.push_str(if stripped {
                        ", stripped"
                    } else {
                        ", not stripped"
                    })?;
                    render_capabilities(&mut out, e.u16(head, 18).unwrap_or(0), cap_hw, cap_sf)?;
                }
            }
        }
    }
    if !mime_only && state.used >= limits.elf_notes {
        out.push_str(&format!(", too many notes ({})", limits.elf_notes))?;
    }
    Ok(Some(Info {
        suffix: out.into_bytes(),
        executable: mode,
        class,
        data_encoding: head[5],
        machine: e.u16(head, 18).unwrap_or(0) as u16,
        file_type: kind as u16,
        notes: state.evidence,
    }))
}
