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
#![forbid(unsafe_code)]
pub mod cdf;
pub mod cli;
pub mod codecs;
mod database;
mod detectors;
mod display;
pub use display::DisplayContext;
pub mod elf;
mod encoding;
mod error;
mod eval;
mod filesystem;
mod flags;
mod format;
pub mod input;
mod inspection;
mod legacy_listing;
mod legacy_source;
pub use legacy_source::{SourceAction, SourceActionOptions, SourceCheck, SourceFragment};
mod limits;
mod options;
mod posix_time;
mod time_context;
pub use time_context::TimeContext;
mod parser;
mod regex_dialect;
mod regex_engine;
mod regex_vm;
pub use database::{Database, FORMAT_VERSION, UPSTREAM_COMMIT};
pub use error::{Error, LimitViolation, Result};
pub use flags::Flags;
pub use inspection::{
    CancellationToken, ExecutionControl, InspectionBudget, InspectionMetrics, MemoryReservation,
};
pub use limits::Limits;
pub use options::{Capabilities, SemanticProfile};

use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Classification {
    pub description: String,
    pub mime_type: String,
    pub encoding: String,
    pub extensions: Option<String>,
    pub apple: Option<String>,
    pub output: Vec<u8>,
    pub truncated: bool,
    pub diagnostics: Vec<String>,
    pub metrics: InspectionMetrics,
    pub compound_document: Option<cdf::CompoundDocument>,
    pub elf: Option<elf::Info>,
    pub compression: Option<codecs::DecodeFacts>,
}
impl Classification {
    fn with_compound_document(mut self, facts: Option<cdf::CompoundDocument>) -> Self {
        if let Some(ref f) = facts {
            self.diagnostics.extend(f.diagnostics.iter().cloned());
        }
        self.compound_document = facts;
        self
    }
    pub fn as_bytes(&self) -> &[u8] {
        &self.output
    }
}

/// Internal compatibility observation for C cookie diagnostic state. It does
/// not expose mutable evaluator state or extend any foreign pointer lifetime.
#[doc(hidden)]
#[derive(Debug)]
pub struct LegacyInspection {
    pub result: Result<Classification>,
    pub last_magic_line: Option<u32>,
}
#[derive(Clone, Debug)]
pub struct Engine {
    database: Arc<Database>,
    pub flags: Flags,
    pub limits: Limits,
    pub profile: SemanticProfile,
    control: ExecutionControl,
    time: TimeContext,
    display: DisplayContext,
}
impl Engine {
    pub fn new(database: Arc<Database>) -> Self {
        Self {
            database,
            flags: Flags::default(),
            limits: Limits::default(),
            profile: SemanticProfile::default(),
            control: ExecutionControl::default(),
            time: TimeContext::default(),
            display: DisplayContext::default(),
        }
    }
    pub fn builtin() -> Result<Self> {
        static BUILTIN: std::sync::OnceLock<std::result::Result<Arc<Database>, String>> =
            std::sync::OnceLock::new();
        let db =
            BUILTIN.get_or_init(|| Database::builtin().map(Arc::new).map_err(|e| e.to_string()));
        Ok(Self::new(
            db.as_ref().map_err(|e| Error::Database(e.clone()))?.clone(),
        ))
    }
    pub fn database(&self) -> &Database {
        &self.database
    }
    pub fn with_profile(mut self, profile: SemanticProfile) -> Self {
        self.profile = profile;
        self
    }
    pub fn with_control(mut self, control: ExecutionControl) -> Self {
        self.control = control;
        self
    }
    pub fn with_time_context(mut self, time: TimeContext) -> Self {
        self.time = time;
        self
    }
    pub fn with_display_context(mut self, display: DisplayContext) -> Self {
        self.display = display;
        self
    }
    pub fn display_context(&self) -> &DisplayContext {
        &self.display
    }
    pub fn capabilities(&self) -> Capabilities {
        let mut capabilities = Capabilities::current(self.profile);
        capabilities.time_zone = self.time.source().to_owned();
        capabilities.display_locale = self.display.name().to_owned();
        capabilities
    }
    fn budget(&self) -> Result<InspectionBudget> {
        InspectionBudget::new(&self.limits, self.control.clone())
    }
    pub fn classify(&self, data: &[u8]) -> Result<Classification> {
        let budget = self.budget()?;
        self.check_output(
            self.classify_input(data, None, false, None, None, &budget),
            &budget,
        )
    }
    /// Inspect an entire borrowed C-API buffer, independently of the file-read
    /// sampling parameter, while preserving all other shared inspection limits.
    /// This entry point remains safe Rust and accepts at most 256 MiB.
    #[doc(hidden)]
    pub fn classify_complete_buffer(&self, data: &[u8]) -> Result<Classification> {
        self.inspect_complete_buffer(data).result
    }
    #[doc(hidden)]
    pub fn inspect_complete_buffer(&self, data: &[u8]) -> LegacyInspection {
        if data.len() > 256 * 1024 * 1024 {
            return LegacyInspection {
                result: Err(Error::Limit("ABI buffer input")),
                last_magic_line: None,
            };
        }
        self.observe_legacy(|budget| {
            self.classify_input_complete(data, None, false, None, None, budget, false)
        })
    }
    pub fn classify_path(&self, path: &std::path::Path) -> Result<Classification> {
        self.inspect_path(path).result
    }
    #[doc(hidden)]
    pub fn inspect_path(&self, path: &std::path::Path) -> LegacyInspection {
        self.observe_legacy(|b| filesystem::classify_path(self, path, b))
    }
    fn observe_legacy(
        &self,
        inspect: impl FnOnce(&InspectionBudget) -> Result<Classification>,
    ) -> LegacyInspection {
        let budget = match self.budget() {
            Ok(b) => b,
            Err(e) => {
                return LegacyInspection {
                    result: Err(e),
                    last_magic_line: None,
                };
            }
        };
        let result = self.check_output(inspect(&budget), &budget);
        LegacyInspection {
            result,
            last_magic_line: budget.last_magic_line(),
        }
    }
    pub fn classify_reader(&self, reader: &mut impl std::io::Read) -> Result<Classification> {
        let budget = self.budget()?;
        let mut bytes = input::read_prefix(reader, self.limits.bytes, &budget)?;
        let sampled = bytes.len() == self.limits.bytes;
        let seekable_elf = (bytes.starts_with(b"\x7fELF") || bytes.starts_with(b"\x7fOLF"))
            && !self
                .flags
                .has(Flags::NO_ELF | Flags::APPLE | Flags::EXTENSION);
        let mut elf = None;
        if seekable_elf {
            if bytes.len() > self.limits.stream_bytes {
                return Err(Error::Limit("ELF stream spool"));
            }
            if sampled {
                let mut chunk = [0u8; 4096];
                loop {
                    let remaining = self.limits.stream_bytes - bytes.len();
                    let request = chunk.len().min(remaining.saturating_add(1));
                    budget.record_read(request)?;
                    let n = match reader.read(&mut chunk[..request]) {
                        Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                        r => r?,
                    };
                    if n > request {
                        return Err(Error::Malformed {
                            format: "input",
                            stage: "reader over-report",
                            offset: bytes.len() as u64,
                        });
                    }
                    if n == 0 {
                        break;
                    }
                    if n > remaining {
                        return Err(Error::Limit("ELF stream spool"));
                    }
                    bytes.extend(&chunk[..n])?;
                }
            }
            elf = elf::inspect(
                &mut input::SliceView::new(&bytes),
                &bytes[..bytes.len().min(self.limits.bytes)],
                elf::Options {
                    executable: false,
                    mime_only: self.flags.has(Flags::MIME),
                    raw: self.flags.has(Flags::RAW),
                    profile: self.profile,
                },
                &budget,
            )?;
        }
        let mut result = self.classify_input(
            &bytes,
            None,
            elf.as_ref().is_some_and(|e| e.executable),
            elf.as_ref().map(|e| e.suffix.as_slice()),
            None,
            &budget,
        )?;
        result.elf = elf;
        result.truncated = sampled;
        self.check_output(Ok(result), &budget)
    }
    pub fn classify_file(&self, file: &mut std::fs::File) -> Result<Classification> {
        self.inspect_file(file).result
    }
    #[doc(hidden)]
    pub fn inspect_file(&self, file: &mut std::fs::File) -> LegacyInspection {
        self.observe_legacy(|b| filesystem::classify_file(self, file, b))
    }
    fn check_output(
        &self,
        result: Result<Classification>,
        budget: &InspectionBudget,
    ) -> Result<Classification> {
        let mut classified = result?;
        budget.checkpoint()?;
        if classified.output.len() > self.limits.output {
            return Err(Error::Limit("output"));
        }
        let raw = classified.output;
        let _display_memory = budget.reserve(raw.len().saturating_mul(4))?;
        budget.bytes(raw.len())?;
        classified.output =
            self.display
                .result(&raw, self.flags.has(Flags::RAW), self.limits.output)?;
        budget.charge_retained(classified.output.len())?;
        classified.metrics = budget.metrics();
        Ok(classified)
    }
    pub(crate) fn basic(
        &self,
        description: Vec<u8>,
        mime: &str,
        encoding: &str,
        truncated: bool,
    ) -> Classification {
        let raw = if self.flags.has(Flags::MIME) {
            let mut out = Vec::new();
            if self.flags.has(Flags::MIME_TYPE) {
                out.extend_from_slice(mime.as_bytes());
            }
            if self.flags.has(Flags::MIME_ENCODING) {
                if self.flags.has(Flags::MIME_TYPE) {
                    out.extend_from_slice(b"; charset=");
                }
                out.extend_from_slice(encoding.as_bytes());
            }
            out
        } else if self.flags.has(Flags::APPLE | Flags::EXTENSION) {
            Vec::new()
        } else {
            description.clone()
        };
        Classification {
            description: String::from_utf8_lossy(&description).into_owned(),
            mime_type: mime.into(),
            encoding: encoding.into(),
            extensions: None,
            apple: None,
            output: raw,
            truncated,
            diagnostics: Vec::new(),
            metrics: InspectionMetrics::default(),
            compound_document: None,
            elf: None,
            compression: None,
        }
    }
    pub(crate) fn classify_input(
        &self,
        data: &[u8],
        tail: Option<&[u8]>,
        executable: bool,
        elf_suffix: Option<&[u8]>,
        view: Option<&mut dyn input::InputView>,
        budget: &InspectionBudget,
    ) -> Result<Classification> {
        let truncated = data.len() > self.limits.bytes;
        let data = &data[..data.len().min(self.limits.bytes)];
        self.classify_input_complete(data, tail, executable, elf_suffix, view, budget, truncated)
    }
    #[allow(clippy::too_many_arguments)]
    fn classify_input_complete(
        &self,
        data: &[u8],
        tail: Option<&[u8]>,
        executable: bool,
        elf_suffix: Option<&[u8]>,
        view: Option<&mut dyn input::InputView>,
        budget: &InspectionBudget,
        truncated: bool,
    ) -> Result<Classification> {
        if data.len() <= 1 {
            let mut c = self.basic(
                if data.is_empty() {
                    b"empty".to_vec()
                } else {
                    b"very short file (no magic)".to_vec()
                },
                if data.is_empty() {
                    "application/x-empty"
                } else {
                    "application/octet-stream"
                },
                "binary",
                truncated,
            );
            if !self.flags.has(Flags::MIME) {
                if self.flags.has(Flags::APPLE) {
                    c.output = b"UNKNUNKN".to_vec();
                } else if self.flags.has(Flags::EXTENSION) {
                    c.output = b"???".to_vec();
                }
            }
            return Ok(c);
        }
        budget.checkpoint()?;
        let encoding_bytes = data.len().min(self.limits.encoding);
        let _encoding_memory = budget.reserve(encoding_bytes.saturating_mul(8))?;
        let enc = if self.flags.has(Flags::NO_ENCODING) {
            encoding::Encoding::binary()
        } else {
            encoding::detect(data, self.limits.encoding)
        };
        if self.flags.has(Flags::COMPRESS)
            && !self.flags.has(Flags::NO_COMPRESS)
            && codecs::probe_with_profile(data, self.profile).is_some()
        {
            let decoded = codecs::decode(
                data,
                codecs::probe_with_profile(data, self.profile).expect("checked probe"),
                budget,
                self.profile,
            )?;
            let expanded = &decoded.bytes;
            let mut inner = self.clone();
            inner.flags.0 &= !Flags::COMPRESS;
            let mut result = inner.classify_input(expanded, None, false, None, None, budget)?;
            if !self.flags.has(Flags::TRANSPARENT)
                && (!self.flags.has(Flags::MIME) || self.flags.0 & Flags::MIME == Flags::MIME)
            {
                let outer =
                    inner.classify_input_complete(data, None, false, None, None, budget, false)?;
                let mut output = result.output.clone();
                append_output(
                    &mut output,
                    if self.flags.has(Flags::MIME) {
                        b" compressed-encoding="
                    } else {
                        b" ("
                    },
                    budget,
                )?;
                append_output(&mut output, database::cstr(&outer.output), budget)?;
                if !self.flags.has(Flags::MIME) {
                    append_output(&mut output, b")", budget)?;
                }
                if output.len() > self.limits.output {
                    return Err(Error::Limit("output"));
                }
                result.output = output;
            }
            result.compression = Some(decoded.facts);
            return Ok(result);
        }
        let mut evaluator = eval::Evaluator::new(
            &self.database,
            self.flags,
            budget.clone(),
            enc.text,
            executable,
            eval::Context {
                time: &self.time,
                display: &self.display,
                profile: self.profile,
            },
        );
        let mut cdf_facts = None;
        let mut output = Vec::new();
        let mut description = Vec::new();
        let mut mime = String::new();
        let mut extensions = None;
        let mut apple = None;
        let mut stage_matched = false;
        if !self.flags.has(Flags::APPLE | Flags::EXTENSION) {
            let mut builtins: Vec<(Vec<u8>, &str)> = Vec::new();
            if !self.flags.has(Flags::NO_TAR) {
                let detected = detectors::tar(data);
                stage_matched = detected.is_some();
                if let Some(t) = detected {
                    builtins.push((t.as_bytes().to_vec(), "application/x-tar"));
                }
            }
            if (builtins.is_empty() || self.flags.has(Flags::CONTINUE))
                && !self.flags.has(Flags::NO_JSON)
            {
                let detected = detectors::json(data);
                stage_matched = detected.is_some();
                if let Some(nd) = detected {
                    builtins.push((
                        if nd {
                            b"New Line Delimited JSON text data".to_vec()
                        } else {
                            b"JSON text data".to_vec()
                        },
                        if nd {
                            "application/x-ndjson"
                        } else {
                            "application/json"
                        },
                    ));
                }
            }
            if (builtins.is_empty() || self.flags.has(Flags::CONTINUE))
                && !self.flags.has(Flags::NO_CSV)
            {
                stage_matched = enc.text && detectors::csv(data);
                if stage_matched {
                    builtins.push((format!("CSV {} text", enc.label).into_bytes(), "text/csv"));
                }
            }
            if (builtins.is_empty() || self.flags.has(Flags::CONTINUE))
                && !self.flags.has(Flags::NO_SIMH)
            {
                stage_matched = detectors::simh(data);
                if stage_matched {
                    builtins.push((b"SIMH tape data".to_vec(), "application/SIMH-tape-data"));
                }
            }
            if (builtins.is_empty() || self.flags.has(Flags::CONTINUE))
                && !self.flags.has(Flags::NO_CDF)
            {
                let detected = cdf::detect(data, view, budget, self.profile, &self.time)?;
                stage_matched = detected.is_some();
                if let Some(detected) = detected {
                    cdf_facts = Some(detected.facts);
                    builtins.push((detected.description, detected.mime));
                }
            }
            for (desc, typ) in builtins {
                if !output.is_empty() && !output.ends_with(b"\n- ") {
                    append_output(&mut output, b"\n- ", budget)?;
                }
                let part: &[u8] = if self.flags.has(Flags::MIME_TYPE) {
                    typ.as_bytes()
                } else if self.flags.has(Flags::MIME_ENCODING) {
                    b""
                } else {
                    &desc
                };
                append_output(&mut output, part, budget)?;
                description = desc;
                mime = typ.into();
                if !self.flags.has(Flags::CONTINUE) {
                    return Ok(self
                        .finish(output, description, mime, enc.mime, (None, None), truncated)
                        .with_compound_document(cdf_facts));
                }
                append_output(&mut output, b"\n- ", budget)?;
            }
        }
        if !self.flags.has(Flags::NO_SOFT) {
            let matched = evaluator.run(&eval::Input { head: data, tail }, database::BINTEST)?;
            stage_matched =
                matched.found && (!self.flags.annotation() || !matched.bytes.is_empty());
            if stage_matched {
                if !output.is_empty() && !output.ends_with(b"\n- ") {
                    append_output(&mut output, b"\n- ", budget)?;
                }
                append_output(&mut output, &matched.bytes, budget)?;
                if !self.flags.annotation() {
                    description = matched.bytes;
                    if let Some(suffix) = elf_suffix {
                        append_output(&mut output, suffix, budget)?;
                        description.extend_from_slice(suffix);
                    }
                }
                if let Some(v) = matched.mime {
                    mime = String::from_utf8_lossy(&v).into_owned();
                }
                extensions = matched
                    .extension
                    .map(|x| String::from_utf8_lossy(&x).into_owned());
                apple = matched
                    .apple
                    .map(|x| String::from_utf8_lossy(&x).into_owned());
                if !self.flags.has(Flags::CONTINUE)
                    && (!self.flags.annotation() || !output.is_empty())
                {
                    return Ok(self
                        .finish(
                            output,
                            description,
                            if mime.is_empty() {
                                "application/octet-stream".into()
                            } else {
                                mime
                            },
                            enc.mime,
                            (extensions, apple),
                            truncated,
                        )
                        .with_compound_document(cdf_facts));
                }
                if self.flags.has(Flags::CONTINUE) {
                    append_output(&mut output, b"\n- ", budget)?;
                }
            }
        }
        let mut text_found = false;
        if !self.flags.has(Flags::NO_TEXT) {
            let _text_memory = budget.reserve(encoding_bytes.saturating_mul(16))?;
            let text_enc = encoding::for_text(data, self.limits.encoding);
            if text_enc.text && data.len() > 1 {
                let utf8 = text_enc.utf8();
                let matched = if !self.flags.has(Flags::NO_SOFT) && !utf8.is_empty() {
                    evaluator.run(
                        &eval::Input {
                            head: &utf8,
                            tail: None,
                        },
                        database::TEXTTEST,
                    )?
                } else {
                    eval::MatchOutput::default()
                };
                if self.flags.has(Flags::APPLE | Flags::EXTENSION) {
                    if matched.found && !matched.bytes.is_empty() {
                        output = matched.bytes;
                        text_found = true;
                    }
                } else if self.flags.has(Flags::MIME) {
                    if !matched.bytes.is_empty() {
                        if !output.is_empty() && !output.ends_with(b"\n- ") {
                            append_output(&mut output, b"\n- ", budget)?;
                        }
                        append_output(&mut output, &matched.bytes, budget)?;
                        if let Some(v) = matched.mime {
                            mime = String::from_utf8_lossy(&v).into_owned();
                        }
                    }
                    if (self.flags.has(Flags::CONTINUE) || output.is_empty())
                        && self.flags.has(Flags::MIME_TYPE)
                    {
                        if !output.is_empty() && !output.ends_with(b"\n- ") {
                            append_output(&mut output, b"\n- ", budget)?;
                        }
                        append_output(&mut output, b"text/plain", budget)?;
                        mime = "text/plain".into();
                    }
                    text_found = true;
                } else {
                    if !matched.bytes.is_empty() {
                        if !output.is_empty() && !output.ends_with(b"\n- ") {
                            append_output(&mut output, b"\n- ", budget)?;
                        }
                        append_output(&mut output, &matched.bytes, budget)?;
                    }
                    output = encoding::text_description(&output, &text_enc);
                    description = output.clone();
                    if let Some(v) = matched.mime {
                        mime = String::from_utf8_lossy(&v).into_owned();
                    }
                    if mime.is_empty() {
                        mime = "text/plain".into();
                    }
                    text_found = true;
                }
            }
        }
        if !self.flags.has(Flags::NO_TEXT) {
            stage_matched = text_found;
        }
        if !stage_matched
            || output.is_empty()
                && (!self.flags.has(Flags::MIME_ENCODING) || self.flags.has(Flags::MIME_TYPE))
        {
            if self.flags.has(Flags::MIME_TYPE) {
                append_output(&mut output, b"application/octet-stream", budget)?;
                mime = "application/octet-stream".into();
            } else if self.flags.has(Flags::APPLE) {
                append_output(&mut output, b"UNKNUNKN", budget)?;
            } else if self.flags.has(Flags::EXTENSION) {
                append_output(&mut output, b"???", budget)?;
            } else if !self.flags.has(Flags::MIME_ENCODING) {
                append_output(&mut output, b"data", budget)?;
                description = b"data".to_vec();
            }
        }
        if output.len() > self.limits.output {
            return Err(Error::Limit("output"));
        }
        if mime.is_empty() {
            mime = "application/octet-stream".into();
        }
        Ok(self
            .finish(
                output,
                description,
                mime,
                enc.mime,
                (extensions, apple),
                truncated,
            )
            .with_compound_document(cdf_facts))
    }
    fn finish(
        &self,
        mut output: Vec<u8>,
        description: Vec<u8>,
        mime_type: String,
        encoding: &str,
        annotations: (Option<String>, Option<String>),
        truncated: bool,
    ) -> Classification {
        let (extensions, apple) = annotations;
        if output.len() >= 4 && output.ends_with(b"\n- ") {
            output.truncate(output.len() - 3);
        }
        if self.flags.has(Flags::MIME_ENCODING) {
            if self.flags.has(Flags::MIME_TYPE) {
                output.extend_from_slice(b"; charset=");
            }
            output.extend_from_slice(encoding.as_bytes());
        }
        Classification {
            description: String::from_utf8_lossy(&description).into_owned(),
            mime_type,
            encoding: encoding.into(),
            extensions,
            apple,
            output,
            truncated,
            diagnostics: Vec::new(),
            metrics: InspectionMetrics::default(),
            compound_document: None,
            elf: None,
            compression: None,
        }
    }
}

fn append_output(out: &mut Vec<u8>, bytes: &[u8], budget: &InspectionBudget) -> Result<()> {
    // file_vprintf reuses the old buffer through %s on every later append.
    // Bytes beyond an embedded NUL cannot survive another print operation.
    out.truncate(database::cstr(out).len());
    let n = out
        .len()
        .checked_add(bytes.len())
        .ok_or(Error::Limit("output"))?;
    if n > budget.limits.output {
        return Err(Error::Limit("output"));
    }
    budget.charge_retained(bytes.len().saturating_mul(4))?;
    out.extend_from_slice(bytes);
    Ok(())
}
