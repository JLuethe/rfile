// SPDX-License-Identifier: GPL-3.0-only
//! GNU ERE lexical normalization in the pinned regex C locale. Magic source
//! unescaping happens earlier; unknown regex escapes quote the following byte.
pub(crate) struct Pattern {
    pub text: String,
    pub captures: usize,
    pub capture_map: Vec<Option<usize>>,
    pub references: Vec<Option<usize>>,
}
fn literal(text: &mut String, b: u8) {
    text.push_str(&format!("\\x{b:02x}"));
}
fn predicate(name: &[u8], b: u8) -> Option<bool> {
    Some(match name {
        b"alnum" => b.is_ascii_alphanumeric(),
        b"alpha" => b.is_ascii_alphabetic(),
        b"blank" => b == b' ' || b == b'\t',
        b"cntrl" => b.is_ascii_control(),
        b"digit" => b.is_ascii_digit(),
        b"graph" => (33..=126).contains(&b),
        b"lower" => b.is_ascii_lowercase(),
        b"print" => (32..=126).contains(&b),
        b"punct" => b.is_ascii_punctuation(),
        b"space" => matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 11 | 12),
        b"upper" => b.is_ascii_uppercase(),
        b"xdigit" => b.is_ascii_hexdigit(),
        _ => return None,
    })
}
#[derive(Clone)]
enum Item {
    Byte(u8),
    Set(Box<[bool; 256]>),
}
fn class(bytes: &[u8], at: &mut usize, insensitive: bool, text: &mut String) -> Result<(), String> {
    let start = *at;
    *at += 1;
    let negate = bytes.get(*at) == Some(&b'^');
    if negate {
        *at += 1;
    }
    let mut items = Vec::new();
    if bytes.get(*at) == Some(&b']') {
        items.push(Item::Byte(b']'));
        *at += 1;
    }
    loop {
        let b = *bytes
            .get(*at)
            .ok_or_else(|| format!("unterminated bracket at byte {start}"))?;
        if b == b']' {
            *at += 1;
            break;
        }
        if b == b'[' && bytes.get(*at + 1).is_some_and(|b| b":.=".contains(b)) {
            let marker = bytes[*at + 1];
            let begin = *at + 2;
            let mut end = begin;
            while end + 1 < bytes.len() && !(bytes[end] == marker && bytes[end + 1] == b']') {
                end += 1;
            }
            if end + 1 >= bytes.len() {
                return Err(format!("unterminated bracket token at byte {}", *at));
            }
            let token = &bytes[begin..end];
            if marker == b':' {
                let mut set = [false; 256];
                for b in 0..=255u8 {
                    set[b as usize] = predicate(token, b)
                        .ok_or_else(|| format!("unknown POSIX class at byte {}", *at))?;
                }
                items.push(Item::Set(Box::new(set)));
            } else if token.len() == 1 {
                items.push(Item::Byte(token[0]));
            } else {
                return Err(format!(
                    "unsupported multibyte collating element at byte {}",
                    *at
                ));
            }
            *at = end + 2;
        } else {
            items.push(Item::Byte(b));
            *at += 1;
        }
    }
    let mut set = [false; 256];
    let mut i = 0;
    while i < items.len() {
        if let (Some(Item::Byte(a)), Some(Item::Byte(b'-')), Some(Item::Byte(b))) =
            (items.get(i), items.get(i + 1), items.get(i + 2))
        {
            if a > b {
                return Err(format!("reversed bracket range at byte {start}"));
            }
            for b in *a..=*b {
                set[b as usize] = true;
            }
            i += 3;
        } else {
            if matches!(items.get(i + 1), Some(Item::Byte(b'-'))) && items.get(i + 2).is_some() {
                return Err(format!("invalid bracket range at byte {start}"));
            }
            match &items[i] {
                Item::Byte(b) => set[*b as usize] = true,
                Item::Set(other) => {
                    for (a, b) in set.iter_mut().zip(other.iter()) {
                        *a |= *b;
                    }
                }
            }
            i += 1;
        }
    }
    if insensitive {
        for b in b'A'..=b'Z' {
            let lower = b.to_ascii_lowercase();
            let yes = set[b as usize] || set[lower as usize];
            set[b as usize] = yes;
            set[lower as usize] = yes;
        }
    }
    if negate {
        for b in &mut set {
            *b = !*b;
        }
        set[b'\n' as usize] = false;
    }
    // Emit only byte predicates, avoiding Rust class-set syntax and its escape
    // interpretation. Case folding is already applied before negation.
    text.push_str("(?-i:[");
    if !set.iter().any(|&x| x) {
        text.push_str("\\x00&&\\x01");
    } else {
        let mut b = 0;
        while b < 256 {
            if !set[b] {
                b += 1;
                continue;
            }
            let first = b;
            while b + 1 < 256 && set[b + 1] {
                b += 1;
            }
            literal(text, first as u8);
            if b > first {
                text.push('-');
                literal(text, b as u8);
            }
            b += 1;
        }
    }
    text.push_str("])");
    Ok(())
}
pub(crate) fn normalize(bytes: &[u8], insensitive: bool, pinned: bool) -> Result<Pattern, String> {
    let bytes = &bytes[..bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len())];
    let mut p = Pattern {
        text: String::new(),
        captures: 0,
        capture_map: vec![None],
        references: vec![None],
    };
    let mut closed = Vec::new();
    let mut stack = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        match bytes[at] {
            b'[' => class(bytes, &mut at, insensitive, &mut p.text)?,
            b'(' => {
                if bytes.get(at + 1) == Some(&b'?') {
                    return Err(format!("non-ERE group extension at byte {at}"));
                }
                p.captures += 1;
                if p.captures > 32 {
                    return Err("regex capture group limit (32)".into());
                }
                closed.push(false);
                stack.push(p.captures);
                p.capture_map.push(Some(p.captures));
                p.references.push(None);
                p.text.push('(');
                at += 1;
            }
            b')' => {
                let g = stack
                    .pop()
                    .ok_or_else(|| format!("unmatched closing group at byte {at}"))?;
                closed[g - 1] = true;
                p.text.push(')');
                at += 1;
            }
            b'\\' => {
                let b = *bytes
                    .get(at + 1)
                    .ok_or_else(|| format!("trailing regex escape at byte {at}"))?;
                match b {
                    b'1'..=b'9' => {
                        let g = (b - b'0') as usize;
                        if !closed.get(g - 1).copied().unwrap_or(false) {
                            return Err(format!("invalid backreference at byte {at}"));
                        }
                        // A nonempty sentinel prevents HIR's empty-repetition
                        // optimizations from deleting dynamic references. The
                        // VM replaces the entire marked capture with Ref and
                        // never executes its sentinel literal.
                        let internal = p.capture_map.len();
                        p.text
                            .push_str(&format!("(?P<__rfile_ref_{internal}>\\x00)"));
                        p.capture_map.push(None);
                        p.references.push(Some(g));
                    }
                    b'w' | b'W' | b's' | b'S' | b'b' | b'B' => {
                        p.text.push('\\');
                        p.text.push(b as char);
                    }
                    b'<' => p.text.push_str("\\b{start}"),
                    b'>' => p.text.push_str("\\b{end}"),
                    b'`' => p.text.push_str("\\A"),
                    b'\'' => p.text.push_str("\\z"),
                    _ if pinned && insensitive && b.is_ascii_lowercase() => {
                        p.text.push_str("[\\x00&&\\x01]")
                    }
                    _ => literal(&mut p.text, b),
                }
                at += 2;
            }
            b if b >= 128 => {
                literal(&mut p.text, b);
                at += 1;
            }
            b => {
                p.text.push(b as char);
                at += 1;
            }
        }
    }
    if !stack.is_empty() {
        return Err("unclosed regex capture".into());
    }
    Ok(p)
}
