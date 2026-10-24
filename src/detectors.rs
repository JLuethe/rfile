/*
 * Copyright (c) Ian F. Darwin 1986-1995.
 * Software written by Ian F. Darwin and others;
 * maintained 1995-present by Christos Zoulas and others.
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

/*-
 * Copyright (c) 2018 Christos Zoulas
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
/*-
 * Copyright (c) 2016 Christos Zoulas
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
// Independent Rust implementation; upstream behavior and retained data notices credited in NOTICE.

pub(crate) fn tar(b: &[u8]) -> Option<&'static str> {
    let header = b.get(..512)?;
    if crate::database::cstr(&header[..100]).len() >= 8
        && crate::database::cstr(&header[..100]).ends_with(b"/gpkg-1")
    {
        return None;
    }
    let checksum = std::str::from_utf8(crate::database::cstr(&header[148..156]))
        .ok()?
        .trim();
    let expected = u64::from_str_radix(checksum, 8).ok()?;
    let actual: u64 = header
        .iter()
        .enumerate()
        .map(|(i, &c)| {
            if (148..156).contains(&i) {
                32
            } else {
                u64::from(c)
            }
        })
        .sum();
    if actual != expected {
        return None;
    }
    Some(if &header[257..265] == b"ustar  \0" {
        "POSIX tar archive (GNU)"
    } else if &header[257..263] == b"ustar\0" {
        "POSIX tar archive"
    } else {
        "tar archive"
    })
}
pub(crate) fn csv(b: &[u8]) -> bool {
    let (mut p, mut fields, mut total, mut lines) = (0, 0, 0, 0);
    while p < b.len() {
        let c = b[p];
        p += 1;
        match c {
            b'"' => {
                let mut quote = false;
                while p < b.len() {
                    let c = b[p];
                    p += 1;
                    if c != b'"' {
                        if quote {
                            p -= 1;
                            break;
                        }
                    } else {
                        quote = !quote;
                    }
                }
            }
            b',' => fields += 1,
            b'\n' => {
                lines += 1;
                if lines == 10 {
                    return total > 1 && total == fields;
                }
                if total == 0 {
                    if fields == 0 {
                        return false;
                    }
                    total = fields;
                } else if fields != total {
                    return false;
                }
                fields = 0;
            }
            _ => {}
        }
    }
    total >= 1 && lines >= 2
}
pub(crate) fn simh(b: &[u8]) -> bool {
    let (mut p, mut records, mut marks) = (0usize, 0, 0);
    fn length(b: &[u8], p: &mut usize) -> Option<u32> {
        let end = p.checked_add(4)?;
        let a: [u8; 4] = b.get(*p..end)?.try_into().ok()?;
        *p = end;
        let n = u32::from_le_bytes(a);
        if n == u32::MAX {
            return Some(n);
        }
        if n & 0x0f000000 != 0 {
            return None;
        }
        let n = n & 0x00ffffff;
        Some(n + (n & 1))
    }
    while p + 4 <= b.len() {
        let Some(n) = length(b, &mut p) else {
            return false;
        };
        if n == u32::MAX && (records > 0 || marks > 0) {
            break;
        }
        if n == 0 {
            marks += 1;
            if marks == 10 {
                break;
            }
            continue;
        }
        let Some(next) = p.checked_add(n as usize) else {
            return false;
        };
        p = next;
        if p + 4 > b.len() {
            break;
        }
        let Some(trailer) = length(b, &mut p) else {
            return false;
        };
        if trailer != n {
            return false;
        }
        records += 1;
    }
    records > 0
}
struct Json<'a> {
    b: &'a [u8],
    p: usize,
    compound: bool,
}
impl Json<'_> {
    fn space(&mut self) {
        while self
            .b
            .get(self.p)
            .is_some_and(|c| matches!(c, b' ' | b'\t' | b'\r' | b'\n'))
        {
            self.p += 1;
        }
    }
    fn string(&mut self) -> bool {
        while let Some(&c) = self.b.get(self.p) {
            self.p += 1;
            match c {
                0 => return false,
                b'"' => return true,
                b'\\' => {
                    let Some(&x) = self.b.get(self.p) else {
                        return false;
                    };
                    self.p += 1;
                    match x {
                        b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't' => {}
                        b'u' => {
                            for _ in 0..4 {
                                if !self.b.get(self.p).is_some_and(u8::is_ascii_hexdigit) {
                                    return false;
                                }
                                self.p += 1;
                            }
                        }
                        _ => return false,
                    }
                }
                _ => {}
            }
        }
        false
    }
    fn number(&mut self) -> bool {
        if self.b.get(self.p) == Some(&b'-') {
            self.p += 1;
        }
        let mut got = false;
        while self.b.get(self.p).is_some_and(u8::is_ascii_digit) {
            got = true;
            self.p += 1;
        }
        if self.b.get(self.p) == Some(&b'.') {
            self.p += 1;
        }
        while self.b.get(self.p).is_some_and(u8::is_ascii_digit) {
            got = true;
            self.p += 1;
        }
        if got && matches!(self.b.get(self.p), Some(b'e' | b'E')) {
            self.p += 1;
            got = false;
            if matches!(self.b.get(self.p), Some(b'+' | b'-')) {
                self.p += 1;
            }
            while self.b.get(self.p).is_some_and(u8::is_ascii_digit) {
                got = true;
                self.p += 1;
            }
        }
        got
    }
    fn value(&mut self, depth: usize) -> bool {
        if depth > 500 {
            return false;
        }
        self.space();
        let Some(&c) = self.b.get(self.p) else {
            return false;
        };
        self.p += 1;
        let ok = match c {
            b'"' => self.string(),
            b'[' => {
                self.compound = true;
                loop {
                    self.space();
                    if self.b.get(self.p) == Some(&b']') {
                        self.p += 1;
                        break true;
                    }
                    if !self.value(depth + 1) {
                        break false;
                    }
                    match self.b.get(self.p) {
                        Some(b',') => self.p += 1,
                        Some(b']') => {
                            self.p += 1;
                            break true;
                        }
                        _ => break false,
                    }
                }
            }
            b'{' => {
                self.compound = true;
                loop {
                    self.space();
                    if self.b.get(self.p) == Some(&b'}') {
                        self.p += 1;
                        break true;
                    }
                    if self.b.get(self.p) != Some(&b'"') {
                        break false;
                    }
                    self.p += 1;
                    if !self.string() {
                        break false;
                    }
                    self.space();
                    if self.b.get(self.p) != Some(&b':') {
                        break false;
                    }
                    self.p += 1;
                    if !self.value(depth + 1) {
                        break false;
                    }
                    match self.b.get(self.p) {
                        Some(b',') => self.p += 1,
                        Some(b'}') => {
                            self.p += 1;
                            break true;
                        }
                        _ => break false,
                    }
                }
            }
            b't' | b'f' | b'n' => {
                let rest: &[u8] = match c {
                    b't' => b"rue",
                    b'f' => b"alse",
                    _ => b"ull",
                };
                let available = rest.len().min(self.b.len() - self.p);
                let ok = self.b[self.p..self.p + available] == rest[..available];
                self.p += available;
                ok
            }
            _ => {
                self.p -= 1;
                self.number()
            }
        };
        self.space();
        ok
    }
}
pub(crate) fn json(b: &[u8]) -> Option<bool> {
    let mut s = Json {
        b,
        p: 0,
        compound: false,
    };
    s.space();
    let first = *b.get(s.p)?;
    if !s.value(0) || !s.compound {
        return None;
    }
    if s.p == b.len() {
        return Some(false);
    }
    if b.get(s.p) == Some(&first) && s.value(1) {
        Some(true)
    } else {
        None
    }
}
const DER_NAMES: &[&str] = &[
    "eoc",
    "bool",
    "int",
    "bit_str",
    "octet_str",
    "null",
    "obj_id",
    "obj_desc",
    "ext",
    "real",
    "enum",
    "embed",
    "utf8_str",
    "rel_oid",
    "time",
    "res2",
    "seq",
    "set",
    "num_str",
    "prt_str",
    "t61_str",
    "vid_str",
    "ia5_str",
    "utc_time",
    "gen_time",
    "gr_str",
    "vis_str",
    "gen_str",
    "univ_str",
    "char_str",
    "bmp_str",
    "date",
    "tod",
    "datetime",
    "duration",
    "oid-iri",
    "rel-oid-iri",
];
pub(crate) fn der_match(
    b: &[u8],
    position: usize,
    pattern: &[u8],
) -> Option<(Vec<u8>, usize, usize)> {
    let data = b.get(position..)?;
    let mut p = 1;
    let mut tag = (data.first()? & 31) as usize;
    if tag == 31 {
        while let Some(&c) = data.get(p) {
            p += 1;
            tag = tag.checked_mul(128)?.checked_add((c & 127) as usize)?;
            if c & 128 == 0 {
                break;
            }
            if p > 8 {
                return None;
            }
        }
    }
    let mut length = *data.get(p)? as usize;
    p += 1;
    if length & 128 != 0 {
        let count = length & 127;
        if count == 0 || count > 4 {
            return None;
        }
        length = 0;
        for &c in data.get(p..p.checked_add(count)?)? {
            length = length.checked_mul(256)?.checked_add(c as usize)?;
        }
        p += count;
    }
    let end = p.checked_add(length)?;
    let content = data.get(p..end)?;
    let name = DER_NAMES.get(tag)?.as_bytes();
    if !pattern.starts_with(name) {
        return None;
    }
    let mut expected = &pattern[name.len()..];
    if expected.first().is_some_and(u8::is_ascii_digit) {
        let mut i = 0;
        let size = crate::parser::integer(expected, &mut i)? as usize;
        if size != length {
            return None;
        }
        expected = &expected[i..];
    }
    let value = if matches!(tag, 12 | 19 | 22) {
        crate::database::cstr(content).to_vec()
    } else if tag == 23 && content.len() >= 12 {
        format!(
            "20{}-{}-{} {}:{}:{} GMT",
            String::from_utf8_lossy(&content[0..2]),
            String::from_utf8_lossy(&content[2..4]),
            String::from_utf8_lossy(&content[4..6]),
            String::from_utf8_lossy(&content[6..8]),
            String::from_utf8_lossy(&content[8..10]),
            String::from_utf8_lossy(&content[10..12])
        )
        .into_bytes()
    } else {
        content
            .iter()
            .take(63)
            .flat_map(|c| format!("{c:02x}").into_bytes())
            .collect()
    };
    if !expected.is_empty()
        && (expected[0] != b'=' || &expected[1..] != b"x" && expected[1..] != value)
    {
        return None;
    }
    Some((value, position.checked_add(p)?, position.checked_add(end)?))
}
