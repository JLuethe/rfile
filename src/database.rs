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

// Independent Rust implementation; upstream behavior and retained data notices credited in NOTICE.
use crate::regex_engine::CompiledRegex;
use crate::{Error, Result};
use std::{
    collections::HashMap,
    path::Path,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicUsize, Ordering},
    },
};

pub const UPSTREAM_COMMIT: &str = "41b1123083ab0bbdfe32f6d4ec14f6aec9a91e59";
pub const FORMAT_VERSION: u32 = 21;
pub const RECORD_SIZE: usize = 432;
pub(crate) const MAX_DB: usize = 128 * 1024 * 1024;
pub(crate) const INDIR: u16 = 1;
pub(crate) const OFFADD: u16 = 2;
pub(crate) const INDIROFFADD: u16 = 4;
pub(crate) const UNSIGNED: u16 = 8;
pub(crate) const NOSPACE: u16 = 16;
pub(crate) const BINTEST: u16 = 32;
pub(crate) const TEXTTEST: u16 = 64;
pub(crate) const OFFNEGATIVE: u16 = 128;
pub(crate) const OFFPOSITIVE: u16 = 256;

type RegexCell = Option<Arc<OnceLock<std::result::Result<Box<CompiledRegex>, String>>>>;

#[derive(Clone, Debug)]
pub(crate) struct Rule {
    pub flags: u16,
    pub level: u8,
    pub factor: u8,
    pub relation: u8,
    pub vallen: u8,
    pub kind: u8,
    pub in_kind: u8,
    pub in_op: u8,
    pub mask_op: u8,
    pub condition: u8,
    pub factor_op: u8,
    pub offset: i32,
    pub in_offset: i32,
    pub line: u32,
    pub mask: u64,
    pub range: u32,
    pub string_flags: u32,
    pub value: [u8; 128],
    pub description: Vec<u8>,
    pub mime: Vec<u8>,
    pub apple: Vec<u8>,
    pub extension: Vec<u8>,
    pub regex: RegexCell,
    pub provenance: Vec<u8>,
}
impl Default for Rule {
    fn default() -> Self {
        Self {
            flags: 0,
            level: 0,
            factor: 0,
            relation: b'=',
            vallen: 0,
            kind: 0,
            in_kind: 0,
            in_op: 0,
            mask_op: 0,
            condition: 0,
            factor_op: 0,
            offset: 0,
            in_offset: 0,
            line: 0,
            mask: 0,
            range: 0,
            string_flags: 0,
            value: [0; 128],
            description: Vec::new(),
            mime: Vec::new(),
            apple: Vec::new(),
            extension: Vec::new(),
            regex: None,
            provenance: Vec::new(),
        }
    }
}
impl Rule {
    pub fn number(&self) -> u64 {
        u64::from_le_bytes(self.value[..8].try_into().expect("fixed value"))
    }
    pub fn bytes(&self) -> &[u8] {
        &self.value[..usize::from(self.vallen).min(128)]
    }
    pub fn name(&self) -> &[u8] {
        cstr(&self.value)
    }
    pub fn unsigned(&self) -> bool {
        self.flags & UNSIGNED != 0
    }
    pub fn record(&self) -> [u8; RECORD_SIZE] {
        let mut b = [0; RECORD_SIZE];
        b[0..2].copy_from_slice(&self.flags.to_le_bytes());
        b[2..12].copy_from_slice(&[
            self.level,
            self.factor,
            self.relation,
            self.vallen,
            self.kind,
            self.in_kind,
            self.in_op,
            self.mask_op,
            self.condition,
            self.factor_op,
        ]);
        b[12..16].copy_from_slice(&self.offset.to_le_bytes());
        b[16..20].copy_from_slice(&self.in_offset.to_le_bytes());
        b[20..24].copy_from_slice(&self.line.to_le_bytes());
        if string_kind(self.kind) {
            b[24..28].copy_from_slice(&self.range.to_le_bytes());
            b[28..32].copy_from_slice(&self.string_flags.to_le_bytes());
        } else {
            b[24..32].copy_from_slice(&self.mask.to_le_bytes());
        }
        b[32..160].copy_from_slice(&self.value);
        for (off, size, value) in [
            (160, 64, &self.description),
            (224, 80, &self.mime),
            (304, 8, &self.apple),
            (312, 120, &self.extension),
        ] {
            let len = value.len().min(if off == 304 { size } else { size - 1 });
            b[off..off + len].copy_from_slice(&value[..len]);
        }
        if self.description.is_empty() {
            let n = self.provenance.len().min(62);
            b[161..161 + n].copy_from_slice(&self.provenance[..n]);
        }
        b
    }
}
pub(crate) fn cstr(bytes: &[u8]) -> &[u8] {
    &bytes[..bytes.iter().position(|&x| x == 0).unwrap_or(bytes.len())]
}
pub(crate) fn string_kind(t: u8) -> bool {
    matches!(t, 5 | 13 | 17 | 18 | 19 | 20 | 41 | 45 | 46 | 61)
}
pub(crate) fn width(t: u8) -> Option<usize> {
    Some(match t {
        1 => 1,
        2 | 7 | 10 | 55..=60 => 2,
        4 | 6 | 8 | 9 | 11 | 12 | 14 | 15 | 16 | 21 | 22 | 23 | 33 | 34 | 35 | 39 | 40 => 4,
        24..=32 | 36..=38 | 42..=44 | 52..=54 => 8,
        49..=51 => 16,
        _ => return None,
    })
}
pub(crate) fn signed(v: u64, n: usize) -> i64 {
    match n {
        1 => (v as i8) as i64,
        2 => (v as i16) as i64,
        4 => (v as i32) as i64,
        _ => v as i64,
    }
}
#[derive(Clone, Debug)]
pub(crate) struct Group {
    pub rules: Vec<Rule>,
    pub database_end: bool,
}
#[derive(Debug, Default)]
struct RegexBudget(AtomicUsize);
#[derive(Clone, Debug)]
pub struct Database {
    regex_budget: Arc<RegexBudget>,
    pub(crate) ordinary: Vec<Group>,
    pub(crate) named: Vec<Group>,
    pub(crate) names: HashMap<Vec<u8>, usize>,
}
impl Database {
    pub fn builtin() -> Result<Self> {
        Self::from_compiled(include_bytes!("../data/magic.mgc"))
    }
    pub fn rule_count(&self) -> usize {
        self.ordinary
            .iter()
            .chain(&self.named)
            .map(|g| g.rules.len())
            .sum()
    }
    pub fn from_compiled(bytes: &[u8]) -> Result<Self> {
        let bad = |s: &str| Error::Database(s.to_owned());
        if bytes.len() > MAX_DB {
            return Err(Error::Limit("database size"));
        }
        if bytes.len() < RECORD_SIZE || !bytes.len().is_multiple_of(RECORD_SIZE) {
            return Err(bad("size is not a multiple of 432"));
        }
        let little = match bytes.get(..4) {
            Some([0x1c, 0x04, 0x1e, 0xf1]) => true,
            Some([0xf1, 0x1e, 0x04, 0x1c]) => false,
            _ => return Err(bad("bad header magic")),
        };
        let u16at = |b: &[u8], p: usize| {
            let a = [b[p], b[p + 1]];
            if little {
                u16::from_le_bytes(a)
            } else {
                u16::from_be_bytes(a)
            }
        };
        let u32at = |b: &[u8], p: usize| {
            let a: [u8; 4] = b[p..p + 4].try_into().expect("checked record");
            if little {
                u32::from_le_bytes(a)
            } else {
                u32::from_be_bytes(a)
            }
        };
        let u64at = |b: &[u8], p: usize| {
            let a: [u8; 8] = b[p..p + 8].try_into().expect("checked record");
            if little {
                u64::from_le_bytes(a)
            } else {
                u64::from_be_bytes(a)
            }
        };
        let version = u32at(bytes, 4);
        if version != FORMAT_VERSION {
            return Err(bad(&format!("version {version}; supported version is 21")));
        }
        let a = u32at(bytes, 8) as usize;
        let n = u32at(bytes, 12) as usize;
        if a.checked_add(n).and_then(|x| x.checked_add(1)) != Some(bytes.len() / RECORD_SIZE)
            || a + n > 250_000
        {
            return Err(bad("inconsistent rule counts"));
        }
        let mut sets = [Vec::new(), Vec::new()];
        for (idx, b) in bytes[RECORD_SIZE..].chunks_exact(RECORD_SIZE).enumerate() {
            let t = b[6];
            if !(1..=61).contains(&t) {
                return Err(bad("invalid rule type"));
            }
            if !b"=x!><&^".contains(&b[4]) {
                return Err(bad("invalid relation"));
            }
            if b[5] > 128 {
                return Err(bad("invalid string length"));
            }
            if u16at(b, 0) & !0x1ff != 0 || b[8] & !0xe7 != 0 || b[9] & !0x47 != 0 {
                return Err(bad("reserved rule flags"));
            }
            if ![0, b'+', b'-', b'*', b'/'].contains(&b[11]) || b[11] == b'/' && b[3] == 0 {
                return Err(bad("invalid strength operation"));
            }
            if b[10] != 0 {
                return Err(Error::Unsupported("conditional magic".into()));
            }
            for (start, end) in [(160, 224), (224, 304), (312, 432)] {
                if !b[start..end].contains(&0) {
                    return Err(bad("unterminated annotation"));
                }
            }
            if matches!(t, 17 | 45 | 46) && !b[32..160].contains(&0) {
                return Err(bad("unterminated pattern/name"));
            }
            let mut value: [u8; 128] = b[32..160].try_into().expect("record length");
            if !little && !string_kind(t) && t != 48 {
                match t {
                    33..=35 => value[..4].reverse(),
                    49..=51 => {}
                    _ => value[..8].reverse(),
                }
            }
            let rule = Rule {
                flags: u16at(b, 0),
                level: b[2],
                factor: b[3],
                relation: b[4],
                vallen: b[5],
                kind: t,
                in_kind: b[7],
                in_op: b[8],
                mask_op: b[9],
                condition: b[10],
                factor_op: b[11],
                offset: u32at(b, 12) as i32,
                in_offset: u32at(b, 16) as i32,
                line: u32at(b, 20),
                mask: u64at(b, 24),
                range: u32at(b, 24),
                string_flags: u32at(b, 28),
                value,
                description: cstr(&b[160..224]).to_vec(),
                mime: cstr(&b[224..304]).to_vec(),
                apple: cstr(&b[304..312]).to_vec(),
                extension: cstr(&b[312..432]).to_vec(),
                regex: if t == 17 {
                    Some(Arc::new(OnceLock::new()))
                } else {
                    None
                },
                provenance: if b[160] == 0 {
                    cstr(&b[161..224]).to_vec()
                } else {
                    Vec::new()
                },
            };
            sets[usize::from(idx >= a)].push(rule);
        }
        Self::from_rules(std::mem::take(&mut sets[0]), std::mem::take(&mut sets[1]))
    }
    pub(crate) fn from_rules(a: Vec<Rule>, b: Vec<Rule>) -> Result<Self> {
        fn groups(rules: Vec<Rule>) -> Result<Vec<Group>> {
            let mut out: Vec<Group> = Vec::new();
            for r in rules {
                if r.level == 0 {
                    if r.flags & (OFFADD | INDIROFFADD) != 0 {
                        return Err(Error::Database("relative offset at root".into()));
                    }
                    out.push(Group {
                        rules: Vec::new(),
                        database_end: false,
                    });
                } else if out.is_empty() {
                    return Err(Error::Database("invalid continuation hierarchy".into()));
                }
                out.last_mut()
                    .ok_or_else(|| Error::Database("missing root".into()))?
                    .rules
                    .push(r);
            }
            if let Some(last) = out.last_mut() {
                last.database_end = true;
            }
            Ok(out)
        }
        let ordinary = groups(a)?;
        let named = groups(b)?;
        let mut names = HashMap::new();
        for (i, g) in named.iter().enumerate() {
            let root = &g.rules[0];
            if root.kind != 45 {
                return Err(Error::Database("named set root must be name".into()));
            }
            names.entry(root.name().to_vec()).or_insert(i);
        }
        let db = Self {
            ordinary,
            named,
            names,
            regex_budget: Arc::new(RegexBudget::default()),
        };
        Ok(db)
    }
    pub(crate) fn regex<'r>(
        &self,
        rule: &'r Rule,
        budget: &crate::InspectionBudget,
    ) -> Result<&'r CompiledRegex> {
        let cell = rule
            .regex
            .as_ref()
            .ok_or_else(|| Error::Database("regex cache absent".into()))?;
        // Compilation is bounded separately from the shared 64 MiB cache.
        // Reserve a conservative transient envelope before entering the compiler.
        let _compilation = if cell.get().is_none() {
            Some(budget.reserve(64 * 1024 * 1024)?)
        } else {
            None
        };
        let cached = cell.get_or_init(|| {
            let compiled = CompiledRegex::compile(rule.name(), rule.string_flags & 12 != 0)?;
            let size = compiled.memory_usage();
            let mut used = self.regex_budget.0.load(Ordering::Relaxed);
            loop {
                let next = used
                    .checked_add(size)
                    .filter(|&n| n <= 64 * 1024 * 1024)
                    .ok_or_else(|| "regex database memory budget".to_string())?;
                match self.regex_budget.0.compare_exchange_weak(
                    used,
                    next,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => break,
                    Err(n) => used = n,
                }
            }
            Ok(Box::new(compiled))
        });
        cached.as_ref().map(Box::as_ref).map_err(|e| {
            if e == "regex database memory budget" {
                Error::Limit("regex database memory")
            } else {
                Error::Unsupported(format!("regex at magic line {}: {e}", rule.line))
            }
        })
    }
    pub fn from_source(source: &str) -> Result<Self> {
        if source.len() > MAX_DB {
            return Err(Error::Limit("source database"));
        }
        crate::parser::parse_sources(&[("<memory>".into(), source.as_bytes().to_vec())])
    }
    /// Parse an owned snapshot of source fragments, sorting all fragments as one
    /// source database. Names are diagnostic provenance, never opened as paths.
    pub fn from_source_fragments(sources: &[(String, Vec<u8>)]) -> Result<Self> {
        Self::validate_source_fragments(sources)?;
        crate::parser::parse_sources(sources)
    }
    pub(crate) fn validate_source_fragments(sources: &[(String, Vec<u8>)]) -> Result<()> {
        if sources.len() > 4096 {
            return Err(Error::Limit("magic source files"));
        }
        sources.iter().try_fold(0usize, |n, (name, bytes)| {
            if name.len() > 65536 {
                return Err(Error::Limit("source name"));
            }
            n.checked_add(bytes.len())
                .filter(|&n| n <= MAX_DB)
                .ok_or(Error::Limit("source database"))
        })?;
        Ok(())
    }
    /// Render parse-order check records while parsing a single source set.
    pub fn check_source_fragments(
        sources: &[(String, Vec<u8>)],
        time: &crate::TimeContext,
        output_limit: usize,
    ) -> crate::SourceCheck {
        crate::legacy_source::check_fragments(sources, time, output_limit)
    }
    /// Parse one source set with bounded native diagnostic events and check dumps.
    /// Unmapped diagnostics remain explicit Rust errors; this does not silently
    /// treat every malformed-source failure as native-compatible.
    pub fn source_action(
        sources: &[crate::SourceFragment],
        time: &crate::TimeContext,
        options: crate::SourceActionOptions,
    ) -> crate::SourceAction {
        crate::legacy_source::action(sources, time, options)
    }
    /// Native header diagnostic order followed by bounded structural validation.
    #[doc(hidden)]
    pub fn from_compiled_named(bytes: &[u8], name: &[u8], file: bool) -> Result<Self> {
        crate::legacy_compiled::load(bytes, name, file)
    }
    pub fn load(path: &Path) -> Result<Self> {
        use std::io::Read;
        if !path.to_string_lossy().ends_with(".mgc") {
            let mut name = path.as_os_str().to_os_string();
            name.push(".mgc");
            if let Ok(b) = read_limited(Path::new(&name)) {
                if let Ok(db) = Self::from_compiled(&b) {
                    return Ok(db);
                }
            }
        }
        if path.is_dir() {
            return Self::load_source(path);
        }
        fn read_limited(p: &Path) -> Result<Vec<u8>> {
            let mut b = Vec::new();
            std::fs::File::open(p)?
                .take((MAX_DB + 1) as u64)
                .read_to_end(&mut b)?;
            if b.len() > MAX_DB {
                return Err(Error::Limit("database size"));
            }
            Ok(b)
        }
        let b = read_limited(path)?;
        if b.starts_with(&[0x1c, 0x04, 0x1e, 0xf1])
            || b.starts_with(&[0xf1, 0x1e, 0x04, 0x1c])
            || path.to_string_lossy().ends_with(".mgc")
        {
            Self::from_compiled(&b)
        } else {
            crate::parser::parse_sources(&[(path.to_string_lossy().into_owned(), b)])
        }
    }
    pub fn load_source(path: &Path) -> Result<Self> {
        use std::io::Read;
        if path.is_dir() {
            let mut files = Vec::new();
            for (index, entry) in std::fs::read_dir(path)?.enumerate() {
                if index >= 16_384 {
                    return Err(Error::Limit("magic directory entries"));
                }
                let entry = entry?;
                if !entry.file_name().to_string_lossy().starts_with('.') && entry.path().is_file() {
                    if files.len() >= 4096 {
                        return Err(Error::Limit("magic source files"));
                    }
                    files.push(entry.path());
                }
            }
            files.sort();
            let mut sources = Vec::new();
            let mut total = 0;
            for p in files {
                let mut b = Vec::new();
                std::fs::File::open(&p)?
                    .take((MAX_DB - total + 1) as u64)
                    .read_to_end(&mut b)?;
                total += b.len();
                if total > MAX_DB {
                    return Err(Error::Limit("source database"));
                }
                sources.push((p.to_string_lossy().into_owned(), b));
            }
            crate::parser::parse_sources(&sources)
        } else {
            let mut bytes = Vec::new();
            std::fs::File::open(path)?
                .take((MAX_DB + 1) as u64)
                .read_to_end(&mut bytes)?;
            if bytes.len() > MAX_DB {
                return Err(Error::Limit("source database"));
            }
            crate::parser::parse_sources(&[(path.to_string_lossy().into_owned(), bytes)])
        }
    }
    pub fn merge(databases: Vec<Self>) -> Result<Self> {
        let total = databases.iter().try_fold(0usize, |n, db| {
            n.checked_add(db.rule_count())
                .filter(|&n| n <= 250_000)
                .ok_or(Error::Limit("merged database rules"))
        })?;
        let _ = total;
        let mut db = Self {
            ordinary: Vec::new(),
            named: Vec::new(),
            names: HashMap::new(),
            regex_budget: Arc::new(RegexBudget::default()),
        };
        for d in databases {
            let base = db.named.len();
            for (n, i) in d.names {
                db.names.entry(n).or_insert(base + i);
            }
            db.ordinary.extend(d.ordinary);
            db.named.extend(d.named);
        }
        if db.rule_count() > 250_000 {
            return Err(Error::Limit("merged database rules"));
        }
        for group in db.ordinary.iter_mut().chain(&mut db.named) {
            for r in &mut group.rules {
                r.regex = if r.kind == 17 {
                    Some(Arc::new(OnceLock::new()))
                } else {
                    None
                };
            }
        }
        Ok(db)
    }
    pub fn compile(&self) -> Result<Vec<u8>> {
        let a: usize = self.ordinary.iter().map(|g| g.rules.len()).sum();
        let b: usize = self.named.iter().map(|g| g.rules.len()).sum();
        let size = (1 + a + b)
            .checked_mul(RECORD_SIZE)
            .ok_or(Error::Limit("compiled database"))?;
        if size > MAX_DB {
            return Err(Error::Limit("compiled database"));
        }
        let mut out = Vec::with_capacity(size);
        let mut header = [0; RECORD_SIZE];
        header[..4].copy_from_slice(&0xf11e041cu32.to_le_bytes());
        header[4..8].copy_from_slice(&FORMAT_VERSION.to_le_bytes());
        header[8..12].copy_from_slice(&(a as u32).to_le_bytes());
        header[12..16].copy_from_slice(&(b as u32).to_le_bytes());
        out.extend(header);
        for g in self.ordinary.iter().chain(&self.named) {
            for r in &g.rules {
                out.extend(r.record());
            }
        }
        Ok(out)
    }
    /// Pinned libmagic strength listing, preserving non-UTF-8 description bytes.
    /// The caller supplies an output cap, at most 16 MiB. This does not emit IO.
    pub fn legacy_list(&self, output_limit: usize) -> Result<Vec<u8>> {
        crate::legacy_listing::render(self, output_limit)
    }
    /// Parse one source and retain the bounded legacy check dump even on error.
    /// This primitive does not perform file discovery or render parser warnings.
    pub fn check_source_dump(
        source: &[u8],
        name: &str,
        time: &crate::TimeContext,
        output_limit: usize,
    ) -> crate::SourceCheck {
        crate::legacy_source::check(source, name, time, output_limit)
    }
    pub fn list(&self) -> String {
        let mut out = String::new();
        for (set, groups) in [(0, &self.ordinary), (1, &self.named)] {
            out.push_str(&format!("Set {set}:\n"));
            for g in groups {
                let r = &g.rules[0];
                out.push_str(&format!(
                    "Strength = {:3}@{}: {} [{}]\n",
                    crate::parser::strength(r),
                    r.line,
                    String::from_utf8_lossy(&r.description),
                    String::from_utf8_lossy(&r.mime)
                ));
            }
        }
        out
    }
}
