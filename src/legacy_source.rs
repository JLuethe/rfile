// SPDX-License-Identifier: GPL-3.0-only
//! Bounded source-check records in parse order, before annotations and sorting.
use crate::{
    Database, Error, Result, SemanticProfile, TimeContext,
    database::{INDIR, Rule},
    format,
};
#[derive(Debug)]
pub struct SourceCheck {
    pub database: Result<Database>,
    pub output: Vec<u8>,
}
pub(crate) fn check(source: &[u8], name: &str, time: &TimeContext, limit: usize) -> SourceCheck {
    if source.len() > crate::database::MAX_DB || name.len() > 65536 {
        return SourceCheck {
            database: Err(Error::Limit("source database")),
            output: Vec::new(),
        };
    }
    let mut owned = Vec::new();
    if owned.try_reserve_exact(source.len()).is_err() {
        return SourceCheck {
            database: Err(Error::Limit("source allocation")),
            output: Vec::new(),
        };
    }
    owned.extend_from_slice(source);
    check_fragments(&[(name.into(), owned)], time, limit)
}
pub(crate) fn check_fragments(
    sources: &[(String, Vec<u8>)],
    time: &TimeContext,
    limit: usize,
) -> SourceCheck {
    let mut output = Vec::new();
    let database = (|| -> Result<Database> {
        Database::validate_source_fragments(sources)?;
        if limit > 16 * 1024 * 1024 {
            return Err(Error::Limit("source diagnostic configuration"));
        }
        append(
            &mut output,
            b"cont\toffset\ttype\topcode\tmask\tvalue\tdesc\n",
            limit,
        )?;
        crate::parser::parse_sources_observed(sources, &mut |rule| {
            append(&mut output, &dump(rule, time)?, limit)
        })
    })();
    SourceCheck { database, output }
}
/// An owned source snapshot with byte-preserving diagnostic provenance.
#[derive(Debug)]
pub struct SourceFragment {
    pub name: Vec<u8>,
    pub bytes: Vec<u8>,
}
#[derive(Clone, Copy, Debug)]
pub struct SourceActionOptions {
    pub check: bool,
    pub warning_count: usize,
    pub warning_limit: usize,
    pub output_limit: usize,
}
#[derive(Debug)]
pub struct SourceAction {
    pub database: Result<Database>,
    pub output: Vec<u8>,
    pub warning_count: usize,
    pub last_source: Option<(Vec<u8>, usize)>,
    /// True only when every parse failure had a verified native warning mapping.
    /// The caller then uses native action failure state, not Rust Display prose.
    pub native_failure: bool,
    pub native_error: Option<Vec<u8>>,
}
pub(crate) fn action(
    sources: &[SourceFragment],
    time: &TimeContext,
    options: SourceActionOptions,
) -> SourceAction {
    struct Diagnostics<'a> {
        time: &'a TimeContext,
        options: SourceActionOptions,
        output: Vec<u8>,
        last: Option<(Vec<u8>, usize)>,
        known_failure: bool,
        unmapped_failure: bool,
        native_error: Option<Vec<u8>>,
    }
    impl crate::parser::Observer for Diagnostics<'_> {
        fn legacy(&self) -> bool {
            true
        }
        fn begin(&mut self, file: &[u8]) -> Result<()> {
            self.last = Some((file.to_vec(), 1));
            Ok(())
        }
        fn line(&mut self, _file: &[u8], physical: usize) -> Result<bool> {
            if let Some((_, line)) = &mut self.last {
                *line = physical;
            }
            Ok(self.options.warning_count < self.options.warning_limit)
        }
        fn end(&mut self, _file: &[u8], next: usize) -> Result<()> {
            if let Some((_, line)) = &mut self.last {
                *line = next;
            }
            Ok(())
        }
        fn rule(&mut self, rule: &Rule) -> Result<()> {
            if self.options.check {
                append(
                    &mut self.output,
                    &dump(rule, self.time)?,
                    self.options.output_limit,
                )?;
            }
            Ok(())
        }
        fn failure(
            &mut self,
            failure: &crate::parser::ParseFailure,
            file: &[u8],
            physical: usize,
            line: &[u8],
        ) -> Result<bool> {
            let Some(diagnostic) = &failure.diagnostic else {
                self.unmapped_failure = true;
                return Ok(false);
            };
            self.known_failure = true;
            self.emit_diagnostic(diagnostic, file, physical, line)?;
            Ok(true)
        }
        fn notice(
            &mut self,
            diagnostic: &crate::parser::ParseDiagnostic,
            file: &[u8],
            physical: usize,
            line: &[u8],
        ) -> Result<()> {
            self.emit_diagnostic(diagnostic, file, physical, line)
        }
    }
    impl Diagnostics<'_> {
        fn emit_diagnostic(
            &mut self,
            diagnostic: &crate::parser::ParseDiagnostic,
            file: &[u8],
            physical: usize,
            line: &[u8],
        ) -> Result<()> {
            use crate::parser::ParseDiagnostic;
            if let ParseDiagnostic::CookieError(bytes) = diagnostic {
                if self.native_error.is_none() {
                    if bytes.len() > self.options.output_limit.saturating_sub(self.output.len()) {
                        return Err(Error::Limit("source diagnostic error"));
                    }
                    self.options.output_limit -= bytes.len();
                    self.native_error = Some(bytes.clone());
                }
                return Ok(());
            }
            self.options.warning_count = self.options.warning_count.saturating_add(1);
            let limit = self.options.output_limit;
            if self.options.warning_count == self.options.warning_limit {
                for message in [
                    format!(
                        ", {physical}: Maximum number of warnings ({}) exceeded.\n",
                        self.options.warning_limit
                    ),
                    format!(", {physical}: Additional warnings are suppressed.\n"),
                ] {
                    append(&mut self.output, cstr(file), limit)?;
                    append(&mut self.output, message.as_bytes(), limit)?;
                }
            } else if self.options.warning_count < self.options.warning_limit {
                append(&mut self.output, cstr(file), limit)?;
                append(
                    &mut self.output,
                    format!(", {physical}: Warning: ").as_bytes(),
                    limit,
                )?;
                match diagnostic {
                    ParseDiagnostic::Warning(message) => append(&mut self.output, message, limit)?,
                    ParseDiagnostic::WarningBytes(message) => {
                        append(&mut self.output, message, limit)?
                    }
                    ParseDiagnostic::CookieError(_) => unreachable!("handled above"),
                    ParseDiagnostic::WarningTail {
                        prefix,
                        offset,
                        suffix,
                    } => {
                        append(&mut self.output, prefix, limit)?;
                        let tail = line.get(*offset..).ok_or_else(|| {
                            Error::Database("diagnostic position outside source line".into())
                        })?;
                        append(&mut self.output, cstr(tail), limit)?;
                        append(&mut self.output, suffix, limit)?;
                    }
                }
                append(&mut self.output, b"\n", limit)?;
            }
            Ok(())
        }
    }
    let mut diagnostics = Diagnostics {
        time,
        options,
        output: Vec::new(),
        last: None,
        known_failure: false,
        unmapped_failure: false,
        native_error: None,
    };
    let database = (|| -> Result<Database> {
        if sources.len() > 4096 {
            return Err(Error::Limit("magic source files"));
        }
        sources.iter().try_fold(0usize, |n, s| {
            if s.name.len() > 65536 {
                return Err(Error::Limit("source name"));
            }
            n.checked_add(s.bytes.len())
                .filter(|&n| n <= crate::database::MAX_DB)
                .ok_or(Error::Limit("source database"))
        })?;
        if options.output_limit > 16 * 1024 * 1024 {
            return Err(Error::Limit("source diagnostic configuration"));
        }
        if options.check {
            append(
                &mut diagnostics.output,
                b"cont\toffset\ttype\topcode\tmask\tvalue\tdesc\n",
                options.output_limit,
            )?;
        }
        let refs: Vec<_> = sources
            .iter()
            .map(|s| crate::parser::SourceRef {
                name: &s.name,
                bytes: &s.bytes,
            })
            .collect();
        crate::parser::parse_sources_events(&refs, &mut diagnostics)
    })();
    let native_failure = diagnostics.known_failure
        && !diagnostics.unmapped_failure
        && matches!(&database, Err(Error::Parse { .. }));
    SourceAction {
        database,
        output: diagnostics.output,
        warning_count: diagnostics.options.warning_count,
        last_source: diagnostics.last,
        native_failure,
        native_error: diagnostics.native_error,
    }
}
fn append(out: &mut Vec<u8>, bytes: &[u8], limit: usize) -> Result<()> {
    let n = out
        .len()
        .checked_add(bytes.len())
        .filter(|&n| n <= limit)
        .ok_or(Error::Limit("source diagnostic output"))?;
    out.try_reserve(n - out.len())
        .map_err(|_| Error::Limit("source diagnostic allocation"))?;
    out.extend_from_slice(bytes);
    Ok(())
}
fn cstr(s: &[u8]) -> &[u8] {
    &s[..s.iter().position(|&b| b == 0).unwrap_or(s.len())]
}
fn dump(r: &Rule, time: &TimeContext) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    out.extend_from_slice(if cstr(&r.description).is_empty() {
        cstr(&r.provenance)
    } else {
        b"*unknown*"
    });
    out.extend_from_slice(
        format!(
            ", {}: {} {}",
            r.line,
            ">".repeat(usize::from(r.level & 7) + 1),
            r.offset
        )
        .as_bytes(),
    );
    if r.flags & INDIR != 0 {
        out.push(b'(');
        out.extend_from_slice(
            crate::parser::TYPES
                .get(usize::from(r.in_kind))
                .unwrap_or(&"*bad in_type*")
                .as_bytes(),
        );
        out.push(b',');
        if r.in_op & 0x40 != 0 {
            out.push(b'~');
        }
        out.push(b"&|^+-*/%"[usize::from(r.in_op & 7)]);
        out.extend_from_slice(format!("{}),", r.in_offset).as_bytes());
    }
    out.push(b' ');
    if r.unsigned() {
        out.push(b'u');
    }
    out.extend_from_slice(
        crate::parser::TYPES
            .get(usize::from(r.kind))
            .unwrap_or(&"*bad type")
            .as_bytes(),
    );
    if r.mask_op & 0x40 != 0 {
        out.push(b'~');
    }
    if matches!(r.kind, 5 | 13 | 17 | 18 | 19 | 20 | 41 | 45 | 46 | 61) {
        if r.string_flags != 0 {
            out.push(b'/');
            for (bit, letter) in b"WwcCstbBHhLlJ".iter().enumerate() {
                if r.string_flags & (1 << bit) != 0 {
                    out.push(*letter);
                }
            }
        }
        if r.range != 0 {
            out.extend_from_slice(format!("/{}", r.range).as_bytes());
        }
    } else {
        out.push(b"&|^+-*/%"[usize::from(r.mask_op & 7)]);
        if r.mask != 0 {
            out.extend_from_slice(format!("{:08x}", r.mask).as_bytes());
        }
    }
    out.push(b',');
    out.push(r.relation);
    if r.relation != b'x' {
        match r.kind {
            1 | 2 | 4 | 7 | 8 | 10 | 11 | 23 | 41 => {
                out.extend_from_slice((r.number() as i32).to_string().as_bytes())
            }
            24..=26 | 52 => out.extend_from_slice((r.number() as i64).to_string().as_bytes()),
            5 | 13 | 17 | 18 | 19 | 20 => {
                for &b in r
                    .value
                    .get(..usize::from(r.vallen))
                    .ok_or(Error::Limit("legacy check string length"))?
                {
                    if (32..=126).contains(&b) {
                        out.push(b);
                    } else {
                        out.push(b'\\');
                        if let Some(ch) = match b {
                            7 => Some(b'a'),
                            8 => Some(b'b'),
                            12 => Some(b'f'),
                            10 => Some(b'n'),
                            13 => Some(b'r'),
                            9 => Some(b't'),
                            11 => Some(b'v'),
                            _ => None,
                        } {
                            out.push(ch);
                        } else {
                            out.extend_from_slice(format!("{b:03o}").as_bytes());
                        }
                    }
                }
            }
            6 | 9 | 12 | 21 | 14..=16 | 22 | 27..=32 | 42..=44 => {
                let local = matches!(r.kind,14..=16|22|30..=32);
                let windows = matches!(r.kind, 42..=44);
                let n = if matches!(r.kind, 6 | 9 | 12 | 21 | 14..=16 | 22) {
                    u64::from(r.number() as u32)
                } else {
                    r.number()
                };
                out.extend_from_slice(
                    format::datetime(n, windows, local, time, SemanticProfile::Pinned548)
                        .as_bytes(),
                );
                out.push(b',');
            }
            33..=38 => {
                let value = if r.kind <= 35 {
                    f32::from_bits(r.number() as u32) as f64
                } else {
                    f64::from_bits(r.number())
                };
                out.extend_from_slice(&format::render(b"%G", &format::Value::Float(value), false)?);
            }
            53 | 54 => {
                // The pinned debug printer includes an extra trailing shift in
                // little-endian varints. Preserve it with bounded wrapping math.
                let end = r
                    .value
                    .iter()
                    .position(|b| b & 0x80 == 0)
                    .unwrap_or(r.value.len() - 1);
                let mut n = 0u64;
                if r.kind == 54 {
                    for &b in r.value[..=end].iter().rev() {
                        n = (n | u64::from(b & 0x7f)).wrapping_shl(7);
                    }
                } else {
                    for (i, &b) in r.value[..=end].iter().enumerate() {
                        n |= u64::from(b & 0x7f);
                        if i < end {
                            n = n.wrapping_shl(7);
                        }
                    }
                }
                out.extend_from_slice((n as i64).to_string().as_bytes());
            }
            55..=57 => {
                out.extend_from_slice(format::dos_date(r.number() as u16).as_bytes());
                out.push(b',');
            }
            58..=60 => {
                out.extend_from_slice(format::dos_time(r.number() as u16).as_bytes());
                out.push(b',');
            }
            61 => {
                let value = std::str::from_utf8(cstr(&r.value))
                    .ok()
                    .and_then(|s| u64::from_str_radix(s, 8).ok());
                out.extend_from_slice(
                    value
                        .map_or_else(|| "*Invalid number*".into(), |n| n.to_string())
                        .as_bytes(),
                );
            }
            3 => {}
            45 | 46 | 48 => {
                out.push(b'\'');
                out.extend_from_slice(cstr(&r.value));
                out.push(b'\'');
            }
            49..=51 => {
                let mut guid = [0; 16];
                guid.copy_from_slice(&r.value[..16]);
                out.extend_from_slice(format::guid(&guid, r.kind == 51).as_bytes());
            }
            _ => out.extend_from_slice(format!("*bad type {}*", r.kind).as_bytes()),
        }
    }
    out.extend_from_slice(b",\"");
    out.extend_from_slice(cstr(&r.description));
    out.extend_from_slice(b"\"]\n");
    Ok(out)
}
