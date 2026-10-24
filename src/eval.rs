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
use crate::{
    Error, Flags, Result,
    database::*,
    format::{self, Value},
    limits::Budget,
};
use std::cmp::Ordering;

pub(crate) struct Input<'a> {
    pub head: &'a [u8],
    pub tail: Option<&'a [u8]>,
}
#[derive(Default, Debug)]
pub(crate) struct MatchOutput {
    pub bytes: Vec<u8>,
    pub found: bool,
    pub leading_nospace: bool,
    pub annotation_stop: bool,
    pub mime: Option<Vec<u8>>,
    pub extension: Option<Vec<u8>>,
    pub apple: Option<Vec<u8>>,
}
struct Hit {
    value: Value,
    end: usize,
    position: usize,
    der_parent_end: Option<usize>,
}
#[derive(Clone, Copy)]
struct PrintState {
    first: bool,
    printed: bool,
    space: bool,
}
impl Default for PrintState {
    fn default() -> Self {
        Self {
            first: true,
            printed: false,
            space: false,
        }
    }
}
pub(crate) struct Evaluator<'a> {
    db: &'a Database,
    flags: Flags,
    budget: Budget<'a>,
    text: bool,
    executable: bool,
    indirect_count: usize,
    print: PrintState,
}
impl<'a> Evaluator<'a> {
    pub fn new(
        db: &'a Database,
        flags: Flags,
        limits: &'a crate::Limits,
        text: bool,
        executable: bool,
    ) -> Self {
        Self {
            db,
            flags,
            budget: Budget::new(limits),
            text,
            executable,
            indirect_count: 0,
            print: PrintState::default(),
        }
    }
    pub fn run(&mut self, input: &Input<'_>, mode: u16) -> Result<MatchOutput> {
        self.print = PrintState::default();
        self.groups(input, mode, 0)
    }
    fn append(&self, out: &mut Vec<u8>, part: &[u8]) -> Result<()> {
        if out
            .len()
            .checked_add(part.len())
            .is_none_or(|n| n > self.budget.limits.output)
        {
            return Err(Error::Limit("output"));
        }
        out.extend_from_slice(part);
        Ok(())
    }
    fn groups(&mut self, input: &Input<'_>, mode: u16, depth: usize) -> Result<MatchOutput> {
        let mut result = MatchOutput::default();
        let mut skip_database = false;
        for g in &self.db.ordinary {
            if skip_database {
                if g.database_end {
                    skip_database = false;
                }
                continue;
            }
            let r = &g.rules[0];
            if r.flags & mode != mode {
                continue;
            }
            if string_kind(r.kind) {
                let test = r.string_flags & ((1 << 5) | (1 << 6));
                if self.text && test == 1 << 6 || !self.text && test == 1 << 5 {
                    continue;
                }
            }
            if r.kind == 5
                && r.relation == b'='
                && r.flags & (INDIR | OFFNEGATIVE) == 0
                && r.string_flags == 0
                && r.range == 0
            {
                let start = r.offset as usize;
                let length = r.vallen as usize;
                let actual = start
                    .checked_add(length)
                    .and_then(|end| input.head.get(start..end));
                if actual != Some(r.bytes()) {
                    self.budget.step()?;
                    self.budget.bytes(length)?;
                    continue;
                }
            }
            let matched = self.group(g, input, 0, mode, false, depth)?;
            if matched.found {
                if !matched.bytes.is_empty() {
                    if !result.bytes.is_empty() && !self.flags.has(Flags::CONTINUE) {
                        self.append(&mut result.bytes, b"\n- ")?;
                    }
                    self.append(&mut result.bytes, &matched.bytes)?;
                }
                result.found = true;
                if matched.annotation_stop && !g.database_end {
                    skip_database = true;
                }
                if matched.mime.is_some() {
                    result.mime = matched.mime;
                }
                if matched.extension.is_some() {
                    result.extension = matched.extension;
                }
                if matched.apple.is_some() {
                    result.apple = matched.apple;
                }
                if !self.flags.has(Flags::CONTINUE) {
                    break;
                }
            }
        }
        Ok(result)
    }
    fn group(
        &mut self,
        g: &Group,
        input: &Input<'_>,
        base: usize,
        _mode: u16,
        flip: bool,
        depth: usize,
    ) -> Result<MatchOutput> {
        if depth >= self.budget.limits.names {
            return Err(Error::Limit("name use count"));
        }
        let mut out = MatchOutput::default();
        let mut active = [false; 256];
        let mut ends = [0usize; 256];
        let mut got = [false; 256];
        let mut tail_mode = false;
        let mut eof_base = 0usize;
        let mut annotation_done = false;
        for r in &g.rules {
            self.budget.step()?;
            let level = r.level as usize;
            if level > 0 && !active[level - 1] {
                active[level] = false;
                continue;
            }
            active[level] = false;
            if level < 255 {
                active[level + 1] = false;
            }
            if r.kind == 3 && got[level] {
                continue;
            }
            let parent = if level == 0 { 0 } else { ends[level - 1] };
            let mut offset;
            if r.flags & OFFNEGATIVE != 0 && r.flags & (OFFADD | INDIROFFADD) == 0 {
                if base != 0 {
                    continue;
                }
                let Some(magnitude) = usize::try_from(r.offset).ok() else {
                    continue;
                };
                let b = input.tail.unwrap_or(input.head);
                let Some(pos) = b.len().checked_sub(magnitude) else {
                    continue;
                };
                tail_mode = input.tail.is_some();
                eof_base = pos;
                offset = pos;
            } else {
                if r.flags & OFFPOSITIVE != 0 || level == 0 {
                    tail_mode = false;
                    eof_base = 0;
                }
                let signed_offset = if r.flags & OFFNEGATIVE != 0 {
                    -(r.offset as i64)
                } else {
                    r.offset as i64
                };
                let start = if r.flags & OFFADD != 0 {
                    parent
                } else if r.flags & OFFPOSITIVE != 0 || level == 0 {
                    0
                } else {
                    eof_base
                };
                let Some(pos) = add_signed(start, signed_offset) else {
                    continue;
                };
                offset = pos;
            }
            let bytes = if tail_mode {
                input.tail.unwrap_or(input.head)
            } else {
                input.head
            };
            let mut data_position = match offset.checked_add(base) {
                Some(p) => p,
                None => continue,
            };
            if r.flags & INDIR != 0 {
                let t = flip_kind(r.in_kind, flip);
                let Some(lhs) = read_number(bytes, data_position, t).map(|(v, w)| {
                    if r.in_op & 0x20 != 0 {
                        signed(v, w) as i128
                    } else {
                        v as i128
                    }
                }) else {
                    continue;
                };
                let operand = if r.in_op & 0x80 != 0 {
                    let Some(at) = add_signed(offset, r.in_offset as i64) else {
                        continue;
                    };
                    let Some((v, w)) = read_number(bytes, at, t) else {
                        continue;
                    };
                    if r.in_op & 0x20 != 0 {
                        signed(v, w) as i128
                    } else {
                        v as i128
                    }
                } else {
                    r.in_offset as i128
                };
                if lhs >= u32::MAX as i128
                    || lhs <= i32::MIN as i128
                    || operand >= u32::MAX as i128
                    || operand <= i32::MIN as i128
                {
                    continue;
                }
                let Some(mut target) = indirect_op(lhs, operand, r.in_op) else {
                    continue;
                };
                if r.in_op & 0x40 != 0 {
                    target = !target;
                }
                if target < 0 || target >= u32::MAX as i128 {
                    continue;
                }
                offset = target as usize;
                if r.flags & INDIROFFADD != 0 {
                    let Some(n) = offset.checked_add(parent) else {
                        continue;
                    };
                    offset = n;
                    if offset == 0 {
                        continue;
                    }
                }
                data_position = if r.kind == 46 {
                    match offset.checked_add(base) {
                        Some(p) => p,
                        None => continue,
                    }
                } else {
                    offset
                };
            }
            let mut nested: Option<MatchOutput> = None;
            let hit = if r.kind == 46 {
                if data_position > bytes.len() {
                    None
                } else {
                    let name = r.name();
                    let (name, call_flip) = if name.first() == Some(&b'^') {
                        (&name[1..], !flip)
                    } else {
                        (name, flip)
                    };
                    let Some(&index) = self.db.names.get(name) else {
                        return Err(Error::Database(format!(
                            "cannot find entry `{}`",
                            String::from_utf8_lossy(name)
                        )));
                    };
                    let call_input = Input {
                        head: bytes,
                        tail: None,
                    };
                    if self.flags.has(Flags::CONTINUE) && r.flags & NOSPACE != 0 {
                        self.print.space = false;
                    }
                    let called = self.group(
                        &self.db.named[index],
                        &call_input,
                        data_position,
                        _mode,
                        call_flip,
                        depth + 1,
                    )?;
                    let found = called.found || out.found;
                    nested = Some(called);
                    if found {
                        Some(Hit {
                            value: Value::None,
                            end: offset,
                            position: offset,
                            der_parent_end: None,
                        })
                    } else {
                        None
                    }
                }
            } else if r.kind == 41 {
                let mut start = offset;
                if r.string_flags & 1 != 0 {
                    start = start.saturating_add(base);
                }
                if start == 0 || start > bytes.len() {
                    None
                } else {
                    self.indirect_count += 1;
                    if self.indirect_count >= self.budget.limits.indirect {
                        return Err(Error::Limit("indirect count"));
                    }
                    let call = Input {
                        head: &bytes[start..],
                        tail: None,
                    };
                    let mut called = self.groups(&call, BINTEST, depth + 1)?;
                    if !self.flags.has(Flags::CONTINUE)
                        && !self.flags.annotation()
                        && !called.bytes.is_empty()
                    {
                        called.bytes.insert(0, b' ');
                    }
                    if called.found {
                        if !self.flags.annotation() && !r.description.is_empty() {
                            let mut text = format::render(
                                &r.description,
                                &Value::Number {
                                    n: start as u64,
                                    signed: false,
                                    width: 4,
                                },
                                self.executable,
                            )?;
                            text.extend(called.bytes);
                            called.bytes = text;
                        }
                        nested = Some(called);
                        Some(Hit {
                            value: Value::None,
                            end: offset,
                            position: offset,
                            der_parent_end: None,
                        })
                    } else {
                        None
                    }
                }
            } else {
                self.fetch_compare(r, bytes, data_position, offset, base, flip)?
            };
            let hit = match hit {
                Some(hit) => hit,
                None if r.relation == b'!'
                    && r.kind != 46
                    && !matches!(r.kind, 24..=32 | 42..=44 | 55..=60)
                    && (data_position > bytes.len()
                        || width(r.kind)
                            .filter(|_| !matches!(r.kind, 24..=32 | 42..=44 | 55..=60))
                            .is_some_and(|w| w > bytes.len().saturating_sub(data_position))
                        || matches!(r.kind, 5 | 13 | 18 | 19 | 20 | 61)
                            && r.vallen as usize > bytes.len().saturating_sub(data_position)) =>
                {
                    let value = if let Some(w) = width(r.kind).filter(|&w| w <= 8) {
                        let native = match w {
                            1 => 1,
                            2 => 2,
                            4 => 4,
                            _ => 24,
                        };
                        let n = padded_number(bytes, data_position, native).map_or(0, |x| x.0);
                        if matches!(r.kind, 33..=38) {
                            Value::Float(if w == 4 {
                                f32::from_bits(n as u32) as f64
                            } else {
                                f64::from_bits(n)
                            })
                        } else {
                            number_value(r.kind, n, r.unsigned(), w)
                        }
                    } else if matches!(r.kind, 5 | 13 | 18 | 19 | 61) {
                        Value::Bytes(format::printable(r.name(), self.flags.has(Flags::RAW)))
                    } else {
                        Value::Bytes(Vec::new())
                    };
                    Hit {
                        value,
                        end: offset.saturating_add(width(r.kind).unwrap_or(r.vallen as usize)),
                        position: data_position,
                        der_parent_end: None,
                    }
                }
                None => continue,
            };
            got[level] = r.kind != 47;
            if let Some(parent_end) = hit.der_parent_end {
                if level > 0 {
                    ends[level - 1] = parent_end;
                }
            }
            active[level] = hit.end <= bytes.len();
            ends[level] = hit.end;
            if level < 255 {
                got[level + 1] = false;
            }
            if let Some(called) = nested {
                if called.found {
                    out.found = true;
                }
                if !called.bytes.is_empty() {
                    if out.bytes.is_empty() {
                        out.leading_nospace = called.leading_nospace || r.flags & NOSPACE != 0;
                    }
                    if !self.flags.has(Flags::CONTINUE)
                        && !out.bytes.is_empty()
                        && r.flags & NOSPACE == 0
                        && !called.leading_nospace
                    {
                        self.append(&mut out.bytes, b" ")?;
                    }
                    self.append(&mut out.bytes, &called.bytes)?;
                }
                if called.mime.is_some() {
                    out.mime = called.mime;
                }
                if called.extension.is_some() {
                    out.extension = called.extension;
                }
                if called.apple.is_some() {
                    out.apple = called.apple;
                }
                if !self.flags.has(Flags::CONTINUE)
                    && self.flags.annotation()
                    && !out.bytes.is_empty()
                {
                    annotation_done = true;
                }
            }
            let annotation = if self.flags.has(Flags::APPLE) && !r.apple.is_empty() {
                out.apple = Some(r.apple.clone());
                Some(r.apple.clone())
            } else if self.flags.has(Flags::EXTENSION) && !r.extension.is_empty() {
                out.extension = Some(r.extension.clone());
                Some(r.extension.clone())
            } else if self.flags.has(Flags::MIME_TYPE) && !r.mime.is_empty() {
                let value = format::expand(&r.mime, self.executable);
                out.mime = Some(value.clone());
                Some(value)
            } else {
                None
            };
            if let Some(value) = annotation {
                if if self.flags.has(Flags::CONTINUE) {
                    !self.print.first
                } else {
                    !out.bytes.is_empty()
                } {
                    self.append(&mut out.bytes, b"\n- ")?;
                }
                self.append(&mut out.bytes, &value)?;
                if self.flags.has(Flags::CONTINUE) {
                    self.print = PrintState {
                        first: false,
                        printed: true,
                        space: true,
                    };
                }
                out.annotation_stop = true;
                out.found = true;
                annotation_done = true;
            }
            if !r.mime.is_empty() && out.mime.is_none() {
                out.mime = Some(format::expand(&r.mime, self.executable));
            }
            if !r.extension.is_empty() && out.extension.is_none() {
                out.extension = Some(r.extension.clone());
            }
            if !r.apple.is_empty() && out.apple.is_none() {
                out.apple = Some(r.apple.clone());
            }
            if !r.description.is_empty() {
                out.found = true;
                if !self.flags.annotation() {
                    let keep = self.flags.has(Flags::CONTINUE);
                    if keep {
                        if r.kind == 45 {
                            self.append(&mut out.bytes, &r.description)?;
                        }
                        if level == 0 {
                            self.print.printed = true;
                            self.print.space = true;
                            if !self.print.first {
                                self.append(&mut out.bytes, b"\n- ")?;
                            }
                        } else {
                            if !self.print.printed {
                                self.print.printed = true;
                                if !self.print.first {
                                    self.append(&mut out.bytes, b"\n- ")?;
                                }
                            }
                            if self.print.space && r.flags & NOSPACE == 0 {
                                self.append(&mut out.bytes, b" ")?;
                            }
                        }
                        if !matches!(r.kind, 41 | 45 | 46) {
                            let rendered = if matches!(r.kind, 3 | 47) {
                                r.description.clone()
                            } else {
                                format::render(&r.description, &hit.value, self.executable)?
                            };
                            self.append(&mut out.bytes, &rendered)?;
                        }
                        self.print.space = true;
                    } else if !matches!(r.kind, 41 | 46) {
                        if out.bytes.is_empty() {
                            out.leading_nospace = r.flags & NOSPACE != 0;
                        }
                        if !out.bytes.is_empty() && r.flags & NOSPACE == 0 {
                            self.append(&mut out.bytes, b" ")?;
                        }
                        let rendered = if matches!(r.kind, 3 | 45 | 47) {
                            r.description.clone()
                        } else {
                            format::render(&r.description, &hit.value, self.executable)?
                        };
                        self.append(&mut out.bytes, &rendered)?;
                    }
                }
            }
            if annotation_done {
                break;
            }
            if self.flags.has(Flags::DEBUG) {
                eprintln!(
                    "[rfile line={} type={} offset={} matched]",
                    r.line, r.kind, hit.position
                );
            }
        }
        if self.flags.has(Flags::CONTINUE) && !annotation_done {
            if self.print.printed {
                self.print.first = false;
            }
            if out.found {
                self.print.printed = false;
                self.print.first = false;
            }
        }
        Ok(out)
    }
    fn fetch_compare(
        &mut self,
        r: &Rule,
        b: &[u8],
        position: usize,
        offset: usize,
        base: usize,
        flip: bool,
    ) -> Result<Option<Hit>> {
        let kind = flip_kind(r.kind, flip);
        let hit = |value, end| {
            Some(Hit {
                value,
                end,
                position,
                der_parent_end: None,
            })
        };
        if matches!(kind, 3 | 47 | 45) {
            return Ok(hit(Value::None, offset));
        }
        if kind == 52 {
            let actual = position as u64;
            return Ok(
                if compare(actual, r.number(), r.relation, r.unsigned(), 8) {
                    hit(
                        Value::Number {
                            n: actual,
                            signed: !r.unsigned(),
                            width: 8,
                        },
                        offset,
                    )
                } else {
                    None
                },
            );
        }
        if kind == 48 {
            let Some((value, child, parent_end)) =
                crate::detectors::der_match(b, position, r.name())
            else {
                return Ok(None);
            };
            return Ok(Some(Hit {
                value: Value::Bytes(format::printable(&value, self.flags.has(Flags::RAW))),
                end: child,
                position,
                der_parent_end: Some(parent_end),
            }));
        }
        if matches!(kind, 49..=51) {
            let Some(bytes) = b.get(position..position.saturating_add(16)) else {
                return Ok(None);
            };
            let actual: [u8; 16] = bytes.try_into().expect("checked GUID width");
            let ordering = actual.as_slice().cmp(&r.value[..16]);
            let pass = compare_order(ordering, r.relation);
            return Ok(if pass {
                hit(Value::Guid(actual, kind == 51), offset.saturating_add(16))
            } else {
                None
            });
        }
        if matches!(kind, 5 | 13 | 18 | 19 | 20 | 17 | 61) {
            let required = if kind == 17 { 0 } else { r.vallen as usize };
            if position > b.len() || required > b.len() - position {
                return Ok(None);
            }
            let bytes = &b[position..];
            if kind == 17 {
                let mut length = if r.string_flags & (1 << 11) != 0 {
                    (r.range as usize).saturating_mul(80)
                } else {
                    r.range as usize
                };
                if length == 0 || length > bytes.len() {
                    length = bytes.len();
                }
                length = length.min(self.budget.limits.regex);
                if r.string_flags & (1 << 11) != 0 && r.range > 0 {
                    let mut lines = 0;
                    for (i, &c) in bytes[..length].iter().enumerate() {
                        if c == b'\n' {
                            lines += 1;
                            if lines >= r.range {
                                length = (i + 1).min(length);
                                break;
                            }
                        }
                    }
                }
                let mut text = &bytes[..length.saturating_sub(1)];
                if let Some(n) = text.iter().position(|&c| c == 0) {
                    text = &text[..n];
                }
                self.budget.bytes(text.len())?;
                let regex = self.db.regex(r)?;
                self.budget
                    .bytes(text.len().saturating_mul(regex.work_factor()))?;
                let found = regex.find(text).map_err(Error::Unsupported)?;
                let pass = if r.relation == b'!' {
                    found.is_none()
                } else if r.relation == b'x' {
                    true
                } else {
                    found.is_some()
                };
                if !pass {
                    return Ok(None);
                }
                let (start, len) = found.map_or((0, 0), |(start, end)| (start, end - start));
                let end = position
                    + start
                    + if r.string_flags & (1 << 4) == 0 {
                        len
                    } else {
                        0
                    };
                let mut value = text.get(start..start + len).unwrap_or_default();
                if r.string_flags & (1 << 13) != 0 {
                    value = trim(value);
                }
                return Ok(hit(
                    Value::Bytes(format::printable(value, self.flags.has(Flags::RAW))),
                    end.saturating_sub(base),
                ));
            }
            if kind == 20 {
                let mut found = None;
                let pattern = r.bytes();
                let max = if r.range == 0 {
                    bytes.len()
                } else {
                    r.range as usize + usize::from(r.string_flags == 0)
                };
                for i in 0..max.min(bytes.len().saturating_sub(pattern.len()).saturating_add(1)) {
                    self.budget.bytes(1)?;
                    if string_cmp(pattern, &bytes[i..], r.string_flags, &mut self.budget)?
                        == Ordering::Equal
                    {
                        found = Some(i);
                        break;
                    }
                }
                let pass = match r.relation {
                    b'!' => found.is_none(),
                    b'x' => true,
                    _ => found.is_some(),
                };
                if !pass {
                    return Ok(None);
                }
                let i = found.unwrap_or(0);
                let end = (position
                    + i
                    + if r.string_flags & 16 == 0 {
                        pattern.len()
                    } else {
                        0
                    })
                .saturating_sub(base);
                // Upstream search changes offset/rm_len, but leaves its string cursor at region start.
                let mut value = &bytes[..bytes.len().saturating_sub(i).min(bytes.len())];
                if r.string_flags & (1 << 13) != 0 {
                    value = trim(value);
                }
                return Ok(hit(
                    Value::Bytes(format::printable(value, self.flags.has(Flags::RAW))),
                    end,
                ));
            }
            let mut scratch = vec![0u8; 128];
            let mut prefix = 0;
            if kind == 18 || kind == 19 {
                let start = usize::from(kind == 18);
                for (i, at) in (start..bytes.len()).step_by(2).take(127).enumerate() {
                    let low = bytes[at];
                    let high = if kind == 18 {
                        bytes[at - 1]
                    } else {
                        bytes.get(at + 1).copied().unwrap_or(0)
                    };
                    scratch[i] = if low == 0 && high != 0 { b' ' } else { low };
                }
            } else {
                let n = if r.range != 0 {
                    (r.range as usize).min(128).min(bytes.len())
                } else {
                    bytes.len().min(128)
                };
                scratch[..n].copy_from_slice(&bytes[..n]);
                scratch[127] = 0;
                if kind == 13 {
                    prefix = crate::parser::pstring_width(r.string_flags);
                    let mut length = match prefix {
                        1 => scratch[0] as usize,
                        2 => {
                            if r.string_flags & (1 << 8) != 0 {
                                u16::from_be_bytes([scratch[0], scratch[1]]) as usize
                            } else {
                                u16::from_le_bytes([scratch[0], scratch[1]]) as usize
                            }
                        }
                        _ => {
                            if r.string_flags & (1 << 10) != 0 {
                                u32::from_be_bytes(scratch[..4].try_into().expect("pstring"))
                                    as usize
                            } else {
                                u32::from_le_bytes(scratch[..4].try_into().expect("pstring"))
                                    as usize
                            }
                        }
                    };
                    if r.string_flags & (1 << 12) != 0 {
                        let Some(n) = length.checked_sub(prefix) else {
                            return Ok(None);
                        };
                        length = n;
                    }
                    length = length.min(128 - prefix);
                    scratch.copy_within(prefix..prefix + length, 0);
                    scratch[length] = 0;
                }
            }
            let ordering = string_cmp(r.bytes(), &scratch, r.string_flags, &mut self.budget)?;
            if !compare_order(ordering, r.relation) {
                return Ok(None);
            }
            let equality = matches!(r.relation, b'=' | b'!');
            let mut value = if equality {
                crate::database::cstr(&r.value).to_vec()
            } else {
                crate::database::cstr(&scratch).to_vec()
            };
            if !equality && r.value[0] == 0 {
                if let Some(n) = value.iter().position(|&c| c == b'\r' || c == b'\n') {
                    value.truncate(n);
                }
            }
            let end = if equality {
                offset + r.vallen as usize
            } else {
                offset + value.len() + prefix
            };
            if r.string_flags & (1 << 13) != 0 {
                value = trim(&value).to_vec();
            }
            if kind == 61 {
                let mut p = 0;
                value = crate::parser::integer(r.name(), &mut p)
                    .unwrap_or(0)
                    .to_string()
                    .into_bytes();
            }
            return Ok(hit(
                Value::Bytes(format::printable(&value, self.flags.has(Flags::RAW))),
                end,
            ));
        }
        if matches!(kind, 39 | 40 | 53 | 54) {
            return Err(Error::Unsupported(format!(
                "magic type {kind} (also incomplete in pinned evaluator)"
            )));
        }
        let numeric = if matches!(kind,24..=32|42..=44|55..=60) {
            padded_number(b, position, kind)
        } else {
            read_number(b, position, kind)
        };
        let Some((mut n, width)) = numeric else {
            return Ok(None);
        };
        self.budget.bytes(width)?;
        if matches!(kind, 33..=38) {
            let mut actual = if width == 4 {
                f32::from_bits(n as u32) as f64
            } else {
                f64::from_bits(n)
            };
            if r.mask != 0 {
                let rhs = r.mask as f64;
                actual = match r.mask_op & 7 {
                    3 => actual + rhs,
                    4 => actual - rhs,
                    5 => actual * rhs,
                    6 => actual / rhs,
                    _ => actual,
                };
            }
            let expected = if width == 4 {
                f32::from_bits(r.number() as u32) as f64
            } else {
                f64::from_bits(r.number())
            };
            let yes = match r.relation {
                b'x' => true,
                b'=' => actual == expected,
                b'!' => actual != expected,
                b'>' => actual > expected,
                b'<' => actual < expected,
                _ => false,
            };
            return Ok(if yes {
                hit(Value::Float(actual), offset + width)
            } else {
                None
            });
        }
        if let Some(value) = numeric_op(n, r.mask, r.mask_op, r.unsigned(), width) {
            n = value;
        } else {
            return Err(Error::Database(format!(
                "invalid numeric operation at line {}",
                r.line
            )));
        }
        if !compare(n, r.number(), r.relation, r.unsigned(), width) {
            return Ok(None);
        }
        let value = number_value(kind, n, r.unsigned(), width);
        Ok(hit(
            value,
            if matches!(kind, 42..=44) {
                0
            } else {
                offset.saturating_add(width)
            },
        ))
    }
}
fn number_value(kind: u8, n: u64, unsigned: bool, width: usize) -> Value {
    match kind {
        6 | 9 | 12 | 14 | 15 | 16 | 21 | 22 | 27..=32 | 42..=44 => {
            Value::Bytes(format::datetime(n, matches!(kind, 42..=44)).into_bytes())
        }
        55..=57 => Value::Bytes(format::dos_date(n as u16).into_bytes()),
        58..=60 => Value::Bytes(format::dos_time(n as u16).into_bytes()),
        _ => Value::Number {
            n,
            signed: !unsigned,
            width,
        },
    }
}
fn padded_number(b: &[u8], position: usize, kind: u8) -> Option<(u64, usize)> {
    let w = width(kind)?;
    if w > 8 {
        return None;
    }
    let mut bytes = [0; 8];
    if let Some(tail) = b.get(position..) {
        let count = tail.len().min(w);
        bytes[..count].copy_from_slice(&tail[..count]);
    }
    read_number(&bytes[..w], 0, kind)
}
fn trim(b: &[u8]) -> &[u8] {
    let a = b
        .iter()
        .position(|c| !c.is_ascii_whitespace())
        .unwrap_or(b.len());
    let z = b
        .iter()
        .rposition(|c| !c.is_ascii_whitespace())
        .map_or(a, |i| i + 1);
    &b[a..z]
}
fn add_signed(a: usize, b: i64) -> Option<usize> {
    if b < 0 {
        a.checked_sub(b.unsigned_abs().try_into().ok()?)
    } else {
        a.checked_add(b.try_into().ok()?)
    }
}
fn indirect_op(a: i128, b: i128, op: u8) -> Option<i128> {
    if b == 0 {
        return Some(a);
    }
    Some(match op & 7 {
        0 => a & b,
        1 => a | b,
        2 => a ^ b,
        3 => a.checked_add(b)?,
        4 => a.checked_sub(b)?,
        5 => a.checked_mul(b)?,
        6 => a.checked_div(b)?,
        7 => a.checked_rem(b)?,
        _ => return None,
    })
}
fn numeric_op(n: u64, rhs: u64, op: u8, unsigned: bool, width: usize) -> Option<u64> {
    let mask = if width >= 8 {
        u64::MAX
    } else {
        (1u64 << (width * 8)) - 1
    };
    let lhs = n & mask;
    let apply = rhs != 0;
    let rhs = rhs & mask;
    let value = if !apply {
        lhs
    } else {
        match op & 7 {
            0 => lhs & rhs,
            1 => lhs | rhs,
            2 => lhs ^ rhs,
            3 => lhs.wrapping_add(rhs),
            4 => lhs.wrapping_sub(rhs),
            5 => lhs.wrapping_mul(rhs),
            6 | 7 => {
                if rhs == 0
                    || !unsigned
                        && width == 4
                        && signed(lhs, width) == i32::MIN as i64
                        && signed(rhs, width) == -1
                {
                    return None;
                }
                if unsigned {
                    if op & 7 == 6 { lhs / rhs } else { lhs % rhs }
                } else {
                    let a = signed(lhs, width);
                    let b = signed(rhs, width);
                    if op & 7 == 6 {
                        a.checked_div(b)? as u64
                    } else {
                        a.checked_rem(b)? as u64
                    }
                }
            }
            _ => return None,
        }
    };
    Some(if op & 0x40 != 0 {
        !value & mask
    } else {
        value & mask
    })
}
fn compare(n: u64, e: u64, rel: u8, unsigned: bool, width: usize) -> bool {
    let n = if unsigned { n } else { signed(n, width) as u64 };
    match rel {
        b'x' => true,
        b'=' => n == e,
        b'!' => n != e,
        b'&' => n & e == e,
        b'^' => n & e != e,
        b'>' | b'<' => {
            let cmp = if unsigned {
                n.cmp(&e)
            } else {
                (n as i64).cmp(&(e as i64))
            };
            if rel == b'>' {
                cmp == Ordering::Greater
            } else {
                cmp == Ordering::Less
            }
        }
        _ => false,
    }
}
fn compare_order(c: Ordering, r: u8) -> bool {
    match r {
        b'x' => true,
        b'=' => c == Ordering::Equal,
        b'!' => c != Ordering::Equal,
        b'>' => c == Ordering::Greater,
        b'<' => c == Ordering::Less,
        _ => false,
    }
}
fn string_cmp(
    pattern: &[u8],
    actual: &[u8],
    flags: u32,
    budget: &mut Budget<'_>,
) -> Result<Ordering> {
    let mut p = 0;
    let mut i = 0;
    while p < pattern.len() {
        budget.bytes(1)?;
        let a = pattern[p];
        let Some(&b) = actual.get(i) else {
            return Ok(Ordering::Greater);
        };
        if flags & 4 != 0 && a.is_ascii_lowercase() {
            let cmp = b.to_ascii_lowercase().cmp(&a);
            if cmp != Ordering::Equal {
                return Ok(cmp);
            }
            i += 1;
        } else if flags & 8 != 0 && a.is_ascii_uppercase() {
            let cmp = b.to_ascii_uppercase().cmp(&a);
            if cmp != Ordering::Equal {
                return Ok(cmp);
            }
            i += 1;
        } else if flags & 1 != 0 && a.is_ascii_whitespace() {
            if !b.is_ascii_whitespace() {
                return Ok(Ordering::Greater);
            }
            i += 1;
            if !pattern.get(p + 1).is_some_and(u8::is_ascii_whitespace) {
                while actual.get(i).is_some_and(u8::is_ascii_whitespace) {
                    budget.bytes(1)?;
                    i += 1;
                }
            }
        } else if flags & 2 != 0 && a.is_ascii_whitespace() {
            while actual.get(i).is_some_and(u8::is_ascii_whitespace) {
                budget.bytes(1)?;
                i += 1;
            }
        } else {
            let cmp = b.cmp(&a);
            if cmp != Ordering::Equal {
                return Ok(cmp);
            }
            i += 1;
        }
        p += 1;
    }
    Ok(
        if flags & (1 << 14) != 0
            && actual
                .get(i)
                .is_some_and(|&b| b != 0 && !b.is_ascii_whitespace())
        {
            Ordering::Greater
        } else {
            Ordering::Equal
        },
    )
}
pub(crate) fn flip_kind(t: u8, flip: bool) -> u8 {
    if !flip {
        return t;
    }
    match t {
        7 => 10,
        10 => 7,
        8 => 11,
        11 => 8,
        9 => 12,
        12 => 9,
        15 => 16,
        16 => 15,
        25 => 26,
        26 => 25,
        28 => 29,
        29 => 28,
        31 => 32,
        32 => 31,
        43 => 44,
        44 => 43,
        34 => 35,
        35 => 34,
        37 => 38,
        38 => 37,
        56 => 57,
        57 => 56,
        59 => 60,
        60 => 59,
        _ => t,
    }
}
pub(crate) fn read_number(b: &[u8], p: usize, t: u8) -> Option<(u64, usize)> {
    if t == 61 {
        let data = b.get(p..)?;
        let mut i = 0;
        let mut value = 0u64;
        while let Some(&c @ b'0'..=b'7') = data.get(i) {
            value = value.checked_mul(8)?.checked_add((c - b'0') as u64)?;
            i += 1;
        }
        return Some((value, i));
    }
    let w = width(t)?;
    if w > 8 {
        return None;
    }
    let s = b.get(p..p.checked_add(w)?)?;
    let big = matches!(
        t,
        7 | 8 | 9 | 15 | 26 | 29 | 32 | 34 | 37 | 39 | 44 | 57 | 60
    );
    let little = matches!(
        t,
        10 | 11 | 12 | 16 | 25 | 28 | 31 | 35 | 38 | 40 | 43 | 56 | 59
    );
    let value = if matches!(t, 21..=23) {
        u32::from_be_bytes([s[1], s[0], s[3], s[2]]) as u64
    } else if big || !little && cfg!(target_endian = "big") {
        s.iter().fold(0, |a, &c| (a << 8) | u64::from(c))
    } else {
        s.iter().rev().fold(0, |a, &c| (a << 8) | u64::from(c))
    };
    let value = if matches!(t, 39 | 40) {
        (value & 127)
            | ((value >> 8) & 127) << 7
            | ((value >> 16) & 127) << 14
            | ((value >> 24) & 127) << 21
    } else {
        value
    };
    Some((value, w))
}
