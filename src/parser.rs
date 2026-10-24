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
// Independent Rust implementation; upstream behavior and retained data notices credited in NOTICE.
use crate::{Error, Result, database::*};

const TYPES: &[&str] = &[
    "invalid",
    "byte",
    "short",
    "default",
    "long",
    "string",
    "date",
    "beshort",
    "belong",
    "bedate",
    "leshort",
    "lelong",
    "ledate",
    "pstring",
    "ldate",
    "beldate",
    "leldate",
    "regex",
    "bestring16",
    "lestring16",
    "search",
    "medate",
    "meldate",
    "melong",
    "quad",
    "lequad",
    "bequad",
    "qdate",
    "leqdate",
    "beqdate",
    "qldate",
    "leqldate",
    "beqldate",
    "float",
    "befloat",
    "lefloat",
    "double",
    "bedouble",
    "ledouble",
    "beid3",
    "leid3",
    "indirect",
    "qwdate",
    "leqwdate",
    "beqwdate",
    "name",
    "use",
    "clear",
    "der",
    "guid",
    "leguid",
    "beguid",
    "offset",
    "bevarint",
    "levarint",
    "msdosdate",
    "lemsdosdate",
    "bemsdosdate",
    "msdostime",
    "lemsdostime",
    "bemsdostime",
    "octal",
];
fn parse_error(line: usize, s: impl Into<String>) -> Error {
    Error::Parse {
        line,
        message: s.into(),
    }
}
pub(crate) fn operation(c: u8) -> Option<u8> {
    b"&|^+-*/%".iter().position(|&x| x == c).map(|x| x as u8)
}
fn space(b: u8) -> bool {
    b.is_ascii_whitespace()
}
fn skip(s: &[u8], p: &mut usize) {
    while *p < s.len() && space(s[*p]) {
        *p += 1;
    }
}

pub(crate) fn integer(s: &[u8], p: &mut usize) -> Option<u64> {
    let start = *p;
    let negative = s.get(*p) == Some(&b'-');
    if matches!(s.get(*p), Some(b'-' | b'+')) {
        *p += 1;
    }
    let radix = if s.get(*p) == Some(&b'0') && matches!(s.get(*p + 1), Some(b'x' | b'X')) {
        *p += 2;
        16
    } else if s.get(*p) == Some(&b'0') {
        8
    } else {
        10
    };
    let digits = *p;
    let mut v = 0u64;
    while let Some(&c) = s.get(*p) {
        let d = match c {
            b'0'..=b'9' => (c - b'0') as u64,
            b'a'..=b'f' => (c - b'a' + 10) as u64,
            b'A'..=b'F' => (c - b'A' + 10) as u64,
            _ => break,
        };
        if d >= radix {
            break;
        }
        v = v.checked_mul(radix)?.checked_add(d)?;
        *p += 1;
    }
    if *p == digits {
        *p = start;
        return None;
    }
    if negative {
        Some(v.wrapping_neg())
    } else {
        Some(v)
    }
}
fn numeric_suffix(s: &[u8], p: &mut usize) {
    while matches!(
        s.get(*p),
        Some(b'u' | b'U' | b'l' | b'L' | b's' | b'h' | b'b' | b'c')
    ) {
        *p += 1;
    }
}
fn string_value(s: &[u8], p: &mut usize, line: usize) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    while let Some(&b) = s.get(*p) {
        if space(b) {
            break;
        }
        *p += 1;
        let value = if b == b'\\' {
            let Some(&b) = s.get(*p) else {
                break;
            };
            *p += 1;
            match b {
                b'a' => 7,
                b'b' => 8,
                b'f' => 12,
                b'n' => 10,
                b'r' => 13,
                b't' => 9,
                b'v' => 11,
                b'0'..=b'7' => {
                    let mut v = (b - b'0') as u16;
                    for _ in 0..2 {
                        if let Some(&x @ b'0'..=b'7') = s.get(*p) {
                            v = (v << 3) | u16::from(x - b'0');
                            *p += 1;
                        } else {
                            break;
                        }
                    }
                    v as u8
                }
                b'x' => {
                    let mut v = 0;
                    let mut n = 0;
                    while n < 2 {
                        let Some(d) = s.get(*p).and_then(|x| (*x as char).to_digit(16)) else {
                            break;
                        };
                        v = (v << 4) | d as u8;
                        *p += 1;
                        n += 1;
                    }
                    if n == 0 { b'x' } else { v }
                }
                x => x,
            }
        } else {
            b
        };
        out.push(value);
        if out.len() > 127 {
            return Err(parse_error(line, "string too long (maximum 127 bytes)"));
        }
    }
    Ok(out)
}
fn modifiers(s: &[u8], r: &mut Rule, line: usize) -> Result<()> {
    let mut p = 0;
    while p < s.len() {
        let c = s[p];
        if c == b'/' {
            p += 1;
            continue;
        }
        if c.is_ascii_digit() {
            r.range = integer(s, &mut p)
                .ok_or_else(|| parse_error(line, "bad string range"))?
                .try_into()
                .map_err(|_| parse_error(line, "range overflow"))?;
            continue;
        }
        p += 1;
        let bit = match c {
            b'W' => 0,
            b'w' => 1,
            b'c' => 2,
            b'C' => 3,
            b's' => 4,
            b't' => 5,
            b'b' => 6,
            b'B' => 7,
            b'H' => 8,
            b'h' => 9,
            b'L' => 10,
            b'l' => 11,
            b'J' => 12,
            b'T' => 13,
            b'f' => 14,
            b'r' if r.kind == 41 => 0,
            _ => {
                return Err(parse_error(
                    line,
                    format!("invalid string modifier {}", c as char),
                ));
            }
        };
        if r.kind == 13 && matches!(c, b'B' | b'H' | b'h' | b'L' | b'l') {
            r.string_flags &= !0xf80;
        }
        r.string_flags |= 1 << bit;
    }
    Ok(())
}
fn offset(s: &[u8], r: &mut Rule, line: usize) -> Result<()> {
    let mut p = 0;
    if s.get(p) == Some(&b'&') {
        r.flags |= OFFADD;
        p += 1;
    }
    if s.get(p) == Some(&b'(') {
        r.flags |= INDIR;
        p += 1;
        if r.flags & OFFADD != 0 {
            r.flags = (r.flags & !OFFADD) | INDIROFFADD;
        }
        if s.get(p) == Some(&b'&') {
            r.flags |= OFFADD;
            p += 1;
        }
    }
    if r.level == 0 && r.flags & (OFFADD | INDIROFFADD) != 0 {
        return Err(parse_error(line, "relative offset at level zero"));
    }
    if let Some(&b @ (b'-' | b'+')) = s.get(p) {
        r.flags |= if b == b'-' { OFFNEGATIVE } else { OFFPOSITIVE };
        p += 1;
    }
    r.offset = integer(s, &mut p).ok_or_else(|| parse_error(line, "invalid offset"))? as i32;
    if r.flags & INDIR != 0 {
        r.in_kind = 4;
        if matches!(s.get(p), Some(b'.' | b',')) {
            if s[p] == b',' {
                r.in_op |= 0x20;
            }
            p += 1;
            r.in_kind = match s.get(p) {
                Some(b'b' | b'B' | b'c' | b'C') => 1,
                Some(b's' | b'h') => 10,
                Some(b'S' | b'H') => 7,
                Some(b'l') => 11,
                Some(b'L') => 8,
                Some(b'm') => 23,
                Some(b'q') => 25,
                Some(b'Q') => 26,
                Some(b'i') => 40,
                Some(b'I') => 39,
                Some(b'o') => 61,
                Some(b'e' | b'f' | b'g') => 38,
                Some(b'E' | b'F' | b'G') => 37,
                _ => return Err(parse_error(line, "invalid indirect type")),
            };
            p += 1;
        }
        if s.get(p) == Some(&b'~') {
            r.in_op |= 0x40;
            p += 1;
        }
        if let Some(op) = s.get(p).and_then(|&c| operation(c)) {
            r.in_op |= op;
            p += 1;
        }
        if s.get(p) == Some(&b'(') {
            r.in_op |= 0x80;
            p += 1;
        }
        if s.get(p).is_some_and(|b| b.is_ascii_digit() || *b == b'-') {
            r.in_offset = integer(s, &mut p)
                .ok_or_else(|| parse_error(line, "invalid indirect operand"))?
                as i32;
        }
        if s.get(p) != Some(&b')') {
            return Err(parse_error(line, "missing indirect closing parenthesis"));
        }
        p += 1;
        if r.in_op & 0x80 != 0 {
            if s.get(p) != Some(&b')') {
                return Err(parse_error(line, "missing nested closing parenthesis"));
            }
            p += 1;
        }
    }
    if p != s.len() {
        return Err(parse_error(line, "trailing characters in offset"));
    }
    Ok(())
}
fn type_field(s: &[u8], r: &mut Rule, line: usize) -> Result<()> {
    let mut p = 0;
    if s.starts_with(b"u") && !s.starts_with(b"use") {
        r.flags |= UNSIGNED;
        p = 1;
    }
    let rest = &s[p..];
    if let Some((t, name)) = TYPES
        .iter()
        .enumerate()
        .skip(1)
        .filter(|(_, name)| rest.starts_with(name.as_bytes()))
        .max_by_key(|(_, name)| name.len())
    {
        r.kind = t as u8;
        p += name.len();
    } else {
        let is_unsigned = r.flags & UNSIGNED != 0;
        if !is_unsigned && s.first() == Some(&b'd') {
            p = 1;
        }
        r.kind = match s.get(p) {
            Some(b'C' | b'1') => 1,
            Some(b'S' | b'2') => 2,
            Some(b'I' | b'L' | b'4') => 4,
            Some(b'Q' | b'8') => 24,
            Some(b's') if p == 0 => 5,
            _ => {
                return Err(parse_error(
                    line,
                    format!("invalid type {}", String::from_utf8_lossy(s)),
                ));
            }
        };
        p += 1;
    }
    if r.kind == 13 {
        r.string_flags = 1 << 7;
    }
    if s.get(p) == Some(&b'~') {
        r.mask_op |= 0x40;
        p += 1;
    }
    if let Some(op) = s.get(p).and_then(|&c| operation(c)) {
        p += 1;
        if string_kind(r.kind) {
            if op != 6 {
                return Err(parse_error(line, "only / modifiers allowed for strings"));
            }
            modifiers(&s[p..], r, line)?;
            p = s.len();
        } else {
            r.mask_op |= op;
            r.mask = integer(s, &mut p).ok_or_else(|| parse_error(line, "invalid numeric mask"))?;
            if !r.unsigned() {
                r.mask = signed(r.mask, width(r.kind).unwrap_or(8)) as u64;
            }
            numeric_suffix(s, &mut p);
        }
    }
    if p != s.len() {
        return Err(parse_error(line, "invalid type modifier"));
    }
    Ok(())
}
fn parse_rule(s: &[u8], line: usize, file: &str) -> Result<Rule> {
    let mut p = 0;
    while s.get(p) == Some(&b'>') {
        p += 1;
    }
    if p > 255 {
        return Err(parse_error(line, "continuation depth exceeds 255"));
    }
    let mut r = Rule {
        level: p as u8,
        line: line as u32,
        ..Rule::default()
    };
    let begin = p;
    while p < s.len() && !space(s[p]) {
        p += 1;
    }
    offset(&s[begin..p], &mut r, line)?;
    skip(s, &mut p);
    let begin = p;
    while p < s.len() && !space(s[p]) {
        p += 1;
    }
    type_field(&s[begin..p], &mut r, line)?;
    skip(s, &mut p);
    if r.kind == 45 && r.level != 0 {
        return Err(parse_error(line, "name must be at top level"));
    }
    if let Some(&c) = s.get(p) {
        if b"!><&^=".contains(&c) {
            r.relation = c;
            p += 1;
            if s.get(p) == Some(&b'=') && c != b'!' {
                if c == b'<' || c == b'>' {
                    return Err(parse_error(line, "inclusive comparison unsupported"));
                }
                p += 1;
            }
        } else if c == b'x' && s.get(p + 1).is_none_or(|x| space(*x)) {
            r.relation = b'x';
            p += 1;
        }
    } else {
        return Err(parse_error(line, "missing test"));
    }
    if r.relation != b'x' {
        if !string_kind(r.kind) && r.kind != 48 {
            skip(s, &mut p);
        }
        if string_kind(r.kind) || r.kind == 48 {
            let v = string_value(s, &mut p, line)?;
            r.vallen = v.len() as u8;
            r.value[..v.len()].copy_from_slice(&v);
            if r.kind == 13 {
                r.vallen = r
                    .vallen
                    .checked_add(pstring_width(r.string_flags) as u8)
                    .ok_or_else(|| parse_error(line, "pstring too long"))?;
            }
        } else if matches!(r.kind, 33..=38) {
            let begin = p;
            while p < s.len() && !space(s[p]) {
                p += 1;
            }
            let value = std::str::from_utf8(&s[begin..p])
                .map_err(|_| parse_error(line, "invalid float"))?
                .parse::<f64>()
                .map_err(|_| parse_error(line, "invalid float"))?;
            if r.kind <= 35 {
                r.value[..4].copy_from_slice(&(value as f32).to_le_bytes());
            } else {
                r.value[..8].copy_from_slice(&value.to_le_bytes());
            }
        } else if matches!(r.kind, 49..=51) {
            let end = p
                .checked_add(36)
                .filter(|&x| x <= s.len())
                .ok_or_else(|| parse_error(line, "short GUID"))?;
            let guid = crate::format::parse_guid(&s[p..end])
                .ok_or_else(|| parse_error(line, "invalid GUID"))?;
            r.value[..16].copy_from_slice(&guid);
            p = end;
        } else {
            let negative = s.get(p) == Some(&b'-');
            let n = integer(s, &mut p).ok_or_else(|| parse_error(line, "invalid numeric test"))?;
            let w = width(r.kind).unwrap_or(8);
            let magnitude = if negative && n != u64::MAX {
                n.wrapping_neg()
            } else {
                n
            };
            let mask = if w >= 8 {
                u64::MAX
            } else {
                (1u64 << (w * 8)) - 1
            };
            if magnitude & !mask != 0 && magnitude | mask != u64::MAX {
                return Err(parse_error(line, "numeric literal overflow"));
            }
            let n = if r.unsigned() {
                n
            } else {
                signed(n, width(r.kind).unwrap_or(8)) as u64
            };
            r.value[..8].copy_from_slice(&n.to_le_bytes());
            numeric_suffix(s, &mut p);
        }
    }
    if r.kind == 17 {
        crate::regex_engine::validate(r.name(), r.string_flags & 12 != 0)
            .map_err(|e| parse_error(line, e))?;
    }
    skip(s, &mut p);
    if s.get(p..p + 2) == Some(b"\\b") {
        r.flags |= NOSPACE;
        p += 2;
    } else if s.get(p) == Some(&8) {
        r.flags |= NOSPACE;
        p += 1;
    }
    r.description = s[p..].iter().copied().take(63).collect();
    crate::format::validate(&r.description, r.kind)
        .map_err(|e| parse_error(line, e.to_string()))?;
    if r.description.is_empty() {
        r.provenance = file.as_bytes().iter().copied().take(62).collect();
    }
    if r.kind == 17 {
        r.regex = Some(std::sync::Arc::new(std::sync::OnceLock::new()));
    }
    Ok(r)
}
pub(crate) fn pstring_width(flags: u32) -> usize {
    if flags & ((1 << 8) | (1 << 9)) != 0 {
        2
    } else if flags & ((1 << 10) | (1 << 11)) != 0 {
        4
    } else {
        1
    }
}
fn nonmagic(s: &[u8]) -> usize {
    let mut p = 0;
    let mut n = 0;
    while p < s.len() {
        match s[p] {
            b'\\' => {
                p += 1;
                n += 1;
            }
            b'?' | b'*' | b'.' | b'+' | b'^' | b'$' => {}
            b'[' => {
                while p < s.len() && s[p] != b']' {
                    p += 1;
                }
                n += 1;
            }
            b'{' => {
                while p < s.len() && s[p] != b'}' {
                    p += 1;
                }
            }
            _ => n += 1,
        }
        p += 1;
    }
    n.max(1)
}
pub(crate) fn strength(r: &Rule) -> usize {
    let mut v = 20i64;
    if r.kind == 3 {
        v = 0;
    } else {
        v += match r.kind {
            5 | 13 | 61 => i64::from(r.vallen) * 10,
            18 | 19 => i64::from(r.vallen) * 5,
            20 => {
                let n = i64::from(r.vallen);
                if n == 0 { 0 } else { n * (10 / n).max(1) }
            }
            17 => {
                let n = nonmagic(r.name()) as i64;
                n * (10 / n).max(1)
            }
            48 => 10,
            _ => width(r.kind).map_or(0, |x| x as i64 * 10),
        };
        match r.relation {
            b'x' | b'!' => v = 0,
            b'=' => v += 10,
            b'>' | b'<' => v -= 20,
            b'&' | b'^' => v -= 10,
            _ => {}
        }
    }
    let n = i64::from(r.factor);
    v = match r.factor_op {
        b'+' => v + n,
        b'-' => v - n,
        b'*' => v * n,
        b'/' if n != 0 => v / n,
        _ => v,
    };
    v.max(1) as usize + usize::from(r.description.is_empty())
}
fn root_mode(r: &mut Rule) {
    let mask = (1 << 5) | (1 << 6);
    r.flags |= match r.kind {
        5 | 13 | 18 | 19 | 17 | 20 => {
            if r.string_flags & mask != 0 {
                let mut flags = 0;
                if r.string_flags & (1 << 5) != 0 {
                    flags |= TEXTTEST;
                }
                if r.string_flags & (1 << 6) != 0 {
                    flags |= BINTEST;
                }
                flags
            } else if matches!(r.kind, 17 | 20) && crate::encoding::looks_utf8(r.bytes()) {
                TEXTTEST
            } else {
                BINTEST
            }
        }
        3 | 41 | 45 | 46 | 47 => 0,
        _ => BINTEST,
    };
}
pub(crate) fn parse_sources(sources: &[(String, Vec<u8>)]) -> Result<Database> {
    let mut groups: Vec<Group> = Vec::new();
    let mut total = 0usize;
    for (file, source) in sources {
        let mut has_root = false;
        for (index, raw) in source.split_inclusive(|&b| b == b'\n').enumerate() {
            let terminated = raw.last() == Some(&b'\n');
            let line = index + usize::from(terminated);
            let s = if terminated {
                &raw[..raw.len() - 1]
            } else {
                raw
            };
            let s = if s.last() == Some(&b'\r') {
                &s[..s.len() - 1]
            } else {
                s
            };
            if s.len() > 65536 {
                return Err(parse_error(line, "line too long"));
            }
            if s.is_empty() || s[0] == b'#' {
                continue;
            }
            if s.starts_with(b"!:") {
                if !has_root {
                    return Err(parse_error(line, "annotation without a rule"));
                }
                let g = groups
                    .last_mut()
                    .ok_or_else(|| parse_error(line, "annotation without group"))?;
                let mut p = 2;
                while p < s.len() && !space(s[p]) {
                    p += 1;
                }
                let name = &s[2..p];
                skip(s, &mut p);
                if name == b"strength" {
                    let root = &mut g.rules[0];
                    if root.factor_op != 0 || root.kind == 45 {
                        return Err(parse_error(line, "invalid/repeated strength directive"));
                    }
                    root.factor_op = *s
                        .get(p)
                        .filter(|c| b"+-*/".contains(c))
                        .ok_or_else(|| parse_error(line, "bad strength operator"))?;
                    p += 1;
                    skip(s, &mut p);
                    let n = integer(s, &mut p)
                        .filter(|&x| x <= 255)
                        .ok_or_else(|| parse_error(line, "strength out of range"))?;
                    if root.factor_op == b'/' && n == 0 {
                        return Err(parse_error(line, "strength divides by zero"));
                    }
                    root.factor = n as u8;
                } else {
                    let r = g
                        .rules
                        .last_mut()
                        .ok_or_else(|| parse_error(line, "annotation without rule"))?;
                    let (dest, limit) = match name {
                        b"mime" => (&mut r.mime, 79),
                        b"apple" => (&mut r.apple, 8),
                        b"ext" => (&mut r.extension, 119),
                        _ => return Err(parse_error(line, "unknown directive")),
                    };
                    if !dest.is_empty() {
                        return Err(parse_error(line, "duplicate annotation"));
                    }
                    let begin = p;
                    while p < s.len() && !space(s[p]) {
                        p += 1;
                    }
                    dest.extend(s[begin..p].iter().copied().take(limit));
                }
                continue;
            }
            let r =
                parse_rule(s, line, file).map_err(|e| parse_error(line, format!("{file}: {e}")))?;
            total += 1;
            if total > 250_000 {
                return Err(Error::Limit("source rules"));
            }
            if r.level == 0 {
                groups.push(Group {
                    rules: Vec::new(),
                    database_end: false,
                });
                has_root = true;
            } else if !has_root {
                return Err(parse_error(line, "continuation without root"));
            }
            let g = groups
                .last_mut()
                .ok_or_else(|| parse_error(line, "missing root"))?;
            if let Some(prev) = g.rules.last() {
                if r.level > prev.level.saturating_add(1) {
                    return Err(parse_error(line, "skipped continuation level"));
                }
            }
            g.rules.push(r);
        }
    }
    for g in &mut groups {
        root_mode(&mut g.rules[0]);
    }
    groups.sort_by(|a, b| {
        strength(&b.rules[0])
            .cmp(&strength(&a.rules[0]))
            .then_with(|| {
                let mut a = a.rules[0].record();
                let mut b = b.rules[0].record();
                a[20..24].fill(0);
                b[20..24].fill(0);
                b.cmp(&a)
            })
    });
    let (mut a, mut b) = (Vec::new(), Vec::new());
    for g in groups {
        if g.rules[0].kind == 45 {
            b.extend(g.rules);
        } else {
            a.extend(g.rules);
        }
    }
    Database::from_rules(a, b)
}
