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
// Independent Rust implementation; upstream behavior and retained data notices credited in NOTICE.
//! Checked ELF metadata enrichment for descriptor/path entry points.
use crate::{Error, Limits, Result};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
};
pub(crate) struct Info {
    pub suffix: Vec<u8>,
    pub executable: bool,
}
struct Elf<'a> {
    file: &'a mut File,
    big: bool,
    is64: bool,
    read_bytes: usize,
    limits: &'a Limits,
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
    fn read(&mut self, offset: u64, size: usize) -> Result<Vec<u8>> {
        self.read_bytes = self
            .read_bytes
            .checked_add(size)
            .ok_or(Error::Limit("ELF read budget"))?;
        if size > self.limits.elf_shsize || self.read_bytes > self.limits.work_bytes {
            return Err(Error::Limit("ELF read bytes"));
        }
        self.file.seek(SeekFrom::Start(offset))?;
        let mut out = Vec::new();
        self.file.take(size as u64).read_to_end(&mut out)?;
        Ok(out)
    }
}
fn notes(
    e: &Elf<'_>,
    b: &[u8],
    out: &mut String,
    used: &mut usize,
    did_bid: &mut bool,
    did_os: &mut bool,
) -> Result<()> {
    let mut p = 0usize;
    while p.checked_add(12).is_some_and(|x| x <= b.len()) {
        *used += 1;
        if *used >= e.limits.elf_notes {
            return Err(Error::Limit("ELF notes"));
        }
        let names = e.u32(b, p).unwrap_or(0) as usize;
        let desc = e.u32(b, p + 4).unwrap_or(0) as usize;
        let kind = e.u32(b, p + 8).unwrap_or(0);
        p += 12;
        let Some(name_end) = p.checked_add(names) else {
            break;
        };
        let Some(name) = b.get(p..name_end) else {
            break;
        };
        let Some(desc_start) = name_end.checked_add(3).map(|n| n & !3) else {
            break;
        };
        let Some(desc_end) = desc_start.checked_add(desc) else {
            break;
        };
        let Some(data) = b.get(desc_start..desc_end) else {
            break;
        };
        p = match desc_end.checked_add(3) {
            Some(n) => n & !3,
            None => break,
        };
        let name = crate::database::cstr(name);
        if name == b"GNU" && kind == 3 && (4..=20).contains(&desc) && !*did_bid {
            let typ = match desc {
                8 => "xxHash",
                16 => "md5/uuid",
                20 => "sha1",
                _ => "unknown",
            };
            out.push_str(&format!(", BuildID[{typ}]="));
            for c in data {
                out.push_str(&format!("{c:02x}"));
            }
            *did_bid = true;
        } else if name == b"Go" && kind == 4 && desc < 128 && !*did_bid {
            out.push_str(", Go BuildID=");
            out.push_str(&String::from_utf8_lossy(&crate::format::printable(
                data, false,
            )));
            *did_bid = true;
        }
        if !*did_os {
            if name == b"GNU" && kind == 1 && desc == 16 {
                let os = match e.u32(data, 0).unwrap_or(99) {
                    0 => "Linux",
                    1 => "Hurd",
                    2 => "Solaris",
                    3 => "kFreeBSD",
                    4 => "kNetBSD",
                    _ => "<unknown>",
                };
                out.push_str(&format!(
                    ", for GNU/{os} {}.{}.{}",
                    e.u32(data, 4).unwrap_or(0),
                    e.u32(data, 8).unwrap_or(0),
                    e.u32(data, 12).unwrap_or(0)
                ));
                *did_os = true;
            } else if name == b"OpenBSD" && kind == 1 && desc == 4 {
                out.push_str(", for OpenBSD");
                *did_os = true;
            } else if name == b"SuSE" && kind == 1 && desc == 2 {
                out.push_str(&format!(", for SuSE {}.{}", data[0], data[1]));
                *did_os = true;
            } else if name == b"Android" && kind == 1 && desc >= 4 {
                out.push_str(&format!(", for Android {}", e.u32(data, 0).unwrap_or(0)));
                *did_os = true;
            }
        }
    }
    Ok(())
}
pub(crate) fn analyze(
    file: &mut File,
    head: &[u8],
    limits: &Limits,
    executable: bool,
    mime_only: bool,
) -> Result<Option<Info>> {
    if head.len() < 16 || !head.starts_with(b"\x7fELF") {
        return Ok(None);
    }
    let class = head[4];
    if !matches!(class, 1 | 2) {
        return Ok(Some(Info {
            suffix: format!(", unknown class {class}").into_bytes(),
            executable,
        }));
    }
    let mut e = Elf {
        file,
        big: head[5] == 2,
        is64: class == 2,
        read_bytes: 0,
        limits,
    };
    let header_size = if e.is64 { 64 } else { 52 };
    if head.len() <= header_size {
        return Ok(None);
    }
    let kind = e.u16(head, 16).unwrap_or(0);
    let mut mode = executable;
    let mut out = String::new();
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
        Some(phnum),
        Some(shsize),
        Some(shnum),
        Some(strtab),
    ) = (phoff, shoff, phsize, phnum, shsize, shnum, strtab)
    else {
        return Ok(None);
    };
    if phnum > limits.elf_phnum {
        return Err(Error::Limit("ELF program headers"));
    }
    if shnum > limits.elf_shnum {
        return Err(Error::Limit("ELF section headers"));
    }
    if !matches!(kind, 1..=4) {
        return Ok(Some(Info {
            suffix: Vec::new(),
            executable: mode,
        }));
    }
    let mut notes_used = 0;
    let mut did_bid = false;
    let mut did_os = false;
    if matches!(kind, 2 | 3) {
        if phnum == 0 {
            out.push_str(", no program header");
        } else if phsize != if e.is64 { 56 } else { 32 } {
            out.push_str(", corrupted program header size");
        } else {
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
                    out.push_str(&format!(", can't read elf program headers at {offset}"));
                    break;
                }
                let typ = e.u32(&ph, 0).unwrap_or(0);
                let poff = e.word(&ph, if e.is64 { 8 } else { 4 }).unwrap_or(0);
                let psize = e.word(&ph, if e.is64 { 32 } else { 16 }).unwrap_or(0);
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
                                    pie = value & 0x08000000 != 0;
                                    mode = pie;
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
                            notes(&e, &b, &mut out, &mut notes_used, &mut did_bid, &mut did_os)?
                        }
                        _ => {}
                    }
                }
            }
            if !mime_only {
                out.push_str(if dynamic {
                    if pie && needed == 0 {
                        ", static-pie linked"
                    } else {
                        ", dynamically linked"
                    }
                } else {
                    ", statically linked"
                });
                if !interp.is_empty() {
                    out.push_str(", interpreter ");
                    out.push_str(&String::from_utf8_lossy(&crate::format::printable(
                        &interp, false,
                    )));
                }
            }
        }
    }
    if kind == 4 {
        return Ok(Some(Info {
            suffix: out.into_bytes(),
            executable: mode,
        }));
    } // Core-note metadata is a separate compatibility milestone.
    if !mime_only {
        if shnum == 0 {
            out.push_str(", no section header");
        } else if shsize != if e.is64 { 64 } else { 40 } {
            out.push_str(", corrupted section header size");
        } else {
            let Some(name_header_offset) = shoff.checked_add((strtab * shsize) as u64) else {
                return Err(Error::Limit("ELF section offset"));
            };
            let name_header = e.read(name_header_offset, shsize)?;
            if name_header.len() < shsize {
                out.push_str(&format!(
                    ", missing section headers at {name_header_offset}"
                ));
            } else {
                let name_offset = e
                    .word(&name_header, if e.is64 { 24 } else { 16 })
                    .unwrap_or(0);
                let names_size = e
                    .word(&name_header, if e.is64 { 32 } else { 20 })
                    .unwrap_or(0)
                    .min(1024 * 1024) as usize;
                let names = e.read(name_offset, names_size)?;
                let mut stripped = true;
                let mut debug = false;
                let mut complete = true;
                for i in 0..shnum {
                    let Some(offset) = shoff.checked_add((i * shsize) as u64) else {
                        return Err(Error::Limit("ELF section offset"));
                    };
                    let sh = e.read(offset, shsize)?;
                    if sh.len() < shsize {
                        out.push_str(&format!(", can't read elf section at {offset}"));
                        complete = false;
                        break;
                    }
                    let name = e.u32(&sh, 0).unwrap_or(0) as usize;
                    let typ = e.u32(&sh, 4).unwrap_or(0);
                    if typ == 2 {
                        stripped = false;
                    }
                    if names
                        .get(name..)
                        .is_some_and(|b| crate::database::cstr(b) == b".debug_info")
                    {
                        debug = true;
                        stripped = false;
                    }
                    if typ == 7 {
                        let offset = e.word(&sh, if e.is64 { 24 } else { 16 }).unwrap_or(0);
                        let size = e.word(&sh, if e.is64 { 32 } else { 20 }).unwrap_or(0);
                        let size =
                            usize::try_from(size).map_err(|_| Error::Limit("ELF note size"))?;
                        let b = e.read(offset, size)?;
                        notes(&e, &b, &mut out, &mut notes_used, &mut did_bid, &mut did_os)?;
                    }
                }
                if complete {
                    if debug {
                        out.push_str(", with debug_info");
                    }
                    out.push_str(if stripped {
                        ", stripped"
                    } else {
                        ", not stripped"
                    });
                }
            }
        }
    }
    Ok(Some(Info {
        suffix: out.into_bytes(),
        executable: mode,
    }))
}
