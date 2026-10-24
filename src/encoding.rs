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
include!("ebcdic_table.rs");

#[derive(Clone, Debug)]
pub(crate) struct Encoding {
    pub label: &'static str,
    pub mime: &'static str,
    pub chars: Vec<u32>,
    pub text: bool,
}
impl Encoding {
    pub fn binary() -> Self {
        Self {
            label: "unknown",
            mime: "binary",
            chars: Vec::new(),
            text: false,
        }
    }
    pub fn utf8(&self) -> Vec<u8> {
        let mut s = String::new();
        for &c in &self.chars {
            if let Some(c) = char::from_u32(c) {
                s.push(c);
            }
        }
        s.into_bytes()
    }
}
pub(crate) fn ascii_text(c: u8) -> bool {
    matches!(c,7..=15|26|27|32..=126|133)
}
pub(crate) fn looks_utf8(b: &[u8]) -> bool {
    utf8(b).is_some()
}
fn utf8(b: &[u8]) -> Option<Vec<u32>> {
    let mut end = b.len();
    let s = match std::str::from_utf8(b) {
        Ok(s) => s,
        Err(e) if e.error_len().is_none() => {
            end = e.valid_up_to();
            std::str::from_utf8(&b[..end]).ok()?
        }
        Err(_) => return None,
    };
    if b[..end].iter().any(|&c| c < 128 && !ascii_text(c)) {
        return None;
    }
    Some(s.chars().map(u32::from).collect())
}
fn text_code(c: u32) -> bool {
    c >= 128 || c == 0 || ascii_text(c as u8)
}
fn make(label: &'static str, mime: &'static str, chars: Vec<u32>) -> Encoding {
    Encoding {
        label,
        mime,
        chars,
        text: true,
    }
}
pub(crate) fn detect(buf: &[u8], limit: usize) -> Encoding {
    let b = &buf[..buf.len().min(limit)];
    if b.iter().all(|&c| ascii_text(c)) {
        if b.len() > 4 && b.starts_with(b"+/v") && matches!(b[3], b'8' | b'9' | b'+' | b'/') {
            return make("Unicode text, UTF-7", "utf-7", Vec::new());
        }
        return make(
            "ASCII",
            "us-ascii",
            b.iter().map(|&c| u32::from(c)).collect(),
        );
    }
    if b.len() > 3 && b.starts_with(&[239, 187, 191]) {
        if let Some(chars) = utf8(&b[3..]) {
            return make("Unicode text, UTF-8 (with BOM)", "utf-8", chars);
        }
    }
    if let Some(chars) = utf8(b) {
        if b.iter().any(|c| *c >= 128) {
            return make("Unicode text, UTF-8", "utf-8", chars);
        }
    }
    if b.starts_with(&[255, 254, 0, 0]) || b.starts_with(&[0, 0, 254, 255]) {
        let le = b[0] == 255;
        let chars: Vec<u32> = b[4..]
            .chunks_exact(4)
            .map(|c| {
                if le {
                    u32::from_le_bytes(c.try_into().expect("chunk"))
                } else {
                    u32::from_be_bytes(c.try_into().expect("chunk"))
                }
            })
            .collect();
        if chars.iter().all(|&c| c != 0xfffe && text_code(c)) {
            return if le {
                make("Unicode text, UTF-32, little-endian", "utf-32le", chars)
            } else {
                make("Unicode text, UTF-32, big-endian", "utf-32be", chars)
            };
        }
    }
    if b.starts_with(&[255, 254]) || b.starts_with(&[254, 255]) {
        let le = b[0] == 255;
        let units = b[2..].chunks_exact(2).map(|c| {
            if le {
                u16::from_le_bytes([c[0], c[1]])
            } else {
                u16::from_be_bytes([c[0], c[1]])
            }
        });
        let chars: Option<Vec<u32>> = char::decode_utf16(units)
            .map(|c| c.ok().map(u32::from))
            .collect();
        if let Some(chars) = chars {
            if chars.iter().all(|&c| {
                text_code(c) && c != 0xfffe && c != 0xffff && !(0xfdd0..=0xfdef).contains(&c)
            }) {
                return if le {
                    make("Unicode text, UTF-16, little-endian", "utf-16le", chars)
                } else {
                    make("Unicode text, UTF-16, big-endian", "utf-16be", chars)
                };
            }
        }
    }
    if b.iter().all(|&c| ascii_text(c) || c >= 160) {
        return make(
            "ISO-8859",
            "iso-8859-1",
            b.iter().map(|&c| u32::from(c)).collect(),
        );
    }
    if b.iter().all(|&c| ascii_text(c) || c >= 128) {
        return make(
            "Non-ISO extended-ASCII",
            "unknown-8bit",
            b.iter().map(|&c| u32::from(c)).collect(),
        );
    }
    let translated: Vec<_> = b.iter().map(|&c| EBCDIC[c as usize]).collect();
    if translated.iter().all(|&c| ascii_text(c)) {
        return make(
            "EBCDIC",
            "ebcdic",
            translated.into_iter().map(u32::from).collect(),
        );
    }
    if translated.iter().all(|&c| ascii_text(c) || c >= 160) {
        return make(
            "International EBCDIC",
            "ebcdic",
            translated.into_iter().map(u32::from).collect(),
        );
    }
    Encoding::binary()
}
pub(crate) fn for_text(buf: &[u8], limit: usize) -> Encoding {
    let mut n = buf.len();
    while n > 1 && buf[n - 1] == 0 {
        n -= 1;
    }
    if n & 1 != 0 && buf.len() & 1 == 0 {
        n += 1;
    }
    detect(&buf[..n], limit)
}
pub(crate) fn text_description(prefix: &[u8], encoding: &Encoding) -> Vec<u8> {
    let mut out = String::from_utf8_lossy(prefix).into_owned();
    let mut executable = false;
    if !out.is_empty() {
        if out.ends_with(" text executable") {
            out.truncate(out.len() - 16);
            out.push_str(", ");
            executable = true;
        } else if out.ends_with(" text") {
            out.truncate(out.len() - 5);
            out.push_str(", ");
        } else {
            out.push_str(", ");
        }
    }
    out.push_str(encoding.label);
    out.push_str(" text");
    if executable {
        out.push_str(" executable");
    }
    let (mut crlf, mut cr, mut lf, mut nel) = (0, 0, 0, 0);
    let mut seen_cr = false;
    let mut last_line_end: Option<usize> = None;
    let mut longest = 0;
    for (i, &c) in encoding.chars.iter().enumerate() {
        if c == 10 {
            if seen_cr {
                crlf += 1;
            } else {
                lf += 1;
            }
            last_line_end = Some(i);
        } else if seen_cr {
            cr += 1;
        }
        seen_cr = c == 13;
        if seen_cr {
            last_line_end = Some(i);
        }
        if c == 133 {
            nel += 1;
            last_line_end = Some(i);
        }
        let length = last_line_end.map_or(i + 1, |n| i - n);
        if length > 300 {
            longest = longest.max(length);
        }
    }
    if seen_cr && cr == 0 && crlf == 0 {
        cr += 1;
    }
    if longest > 0 {
        out.push_str(&format!(", with very long lines ({longest})"));
    }
    if crlf + cr + lf + nel == 0 {
        out.push_str(", with no line terminators");
    } else if crlf + cr + nel > 0 {
        let mut endings = Vec::new();
        if crlf > 0 {
            endings.push("CRLF");
        }
        if cr > 0 {
            endings.push("CR");
        }
        if lf > 0 {
            endings.push("LF");
        }
        if nel > 0 {
            endings.push("NEL");
        }
        out.push_str(&format!(", with {} line terminators", endings.join(", ")));
    }
    if encoding.chars.contains(&27) {
        out.push_str(", with escape sequences");
    }
    if encoding.chars.contains(&8) {
        out.push_str(", with overstriking");
    }
    out.into_bytes()
}
