// rfile contributions: Copyright (C) 2026 rfile contributors.
// SPDX-License-Identifier: GPL-3.0-only
use crate::{Error, Result};

#[derive(Clone, Debug)]
pub struct Limits {
    pub bytes: usize,
    pub encoding: usize,
    pub indirect: usize,
    pub names: usize,
    pub regex: usize,
    pub steps: usize,
    pub work_bytes: usize,
    pub output: usize,
    pub elf_notes: usize,
    pub elf_phnum: usize,
    pub elf_shnum: usize,
    pub elf_shsize: usize,
    pub magwarn: usize,
    pub memory: usize,
    pub io_bytes: usize,
    pub io_reads: usize,
    pub codec_window: usize,
    pub codec_members: usize,
    pub regex_steps: usize,
    pub regex_states: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            bytes: 7 * 1024 * 1024,
            encoding: 64 * 1024,
            indirect: 50,
            names: 150,
            regex: 8192,
            steps: 1_000_000,
            work_bytes: 128 * 1024 * 1024,
            output: 1024 * 1024,
            elf_notes: 256,
            elf_phnum: 2048,
            elf_shnum: 32768,
            elf_shsize: 128 * 1024 * 1024,
            magwarn: 64,
            memory: 128 * 1024 * 1024,
            io_bytes: 128 * 1024 * 1024,
            io_reads: 65_536,
            codec_window: 64 * 1024 * 1024,
            codec_members: 64,
            regex_steps: 1_000_000,
            regex_states: 65_536,
        }
    }
}
impl Limits {
    pub fn validate(&self) -> Result<()> {
        if self.bytes > 256 * 1024 * 1024 || self.encoding > 256 * 1024 * 1024 {
            return Err(Error::Limit("input allocation"));
        }
        if self.indirect > 150 || self.names > 150 {
            return Err(Error::Limit("recursion configuration"));
        }
        if self.output > 16 * 1024 * 1024 {
            return Err(Error::Limit("output configuration"));
        }
        if self.memory > 512 * 1024 * 1024
            || self.io_bytes > 512 * 1024 * 1024
            || self.io_reads > 1_000_000
            || self.codec_window > 256 * 1024 * 1024
            || self.codec_members > 1024
            || self.regex_steps > 10_000_000
            || self.regex_states > 262_144
        {
            return Err(Error::Limit("inspection configuration"));
        }
        Ok(())
    }
    pub fn set(&mut self, name: &str, value: usize) -> Result<()> {
        let mut proposed = self.clone();
        let field = match name {
            "bytes" => &mut proposed.bytes,
            "encoding" => &mut proposed.encoding,
            "indir" => &mut proposed.indirect,
            "name" => &mut proposed.names,
            "regex" => &mut proposed.regex,
            "elf_notes" => &mut proposed.elf_notes,
            "elf_phnum" => &mut proposed.elf_phnum,
            "elf_shnum" => &mut proposed.elf_shnum,
            "elf_shsize" => &mut proposed.elf_shsize,
            "magwarn" => &mut proposed.magwarn,
            "steps" => &mut proposed.steps,
            "work" => &mut proposed.work_bytes,
            "output" => &mut proposed.output,
            "memory" => &mut proposed.memory,
            "io_bytes" => &mut proposed.io_bytes,
            "io_reads" => &mut proposed.io_reads,
            "codec_window" => &mut proposed.codec_window,
            "codec_members" => &mut proposed.codec_members,
            "regex_steps" => &mut proposed.regex_steps,
            "regex_states" => &mut proposed.regex_states,
            _ => return Err(Error::Unsupported(format!("parameter {name}"))),
        };
        *field = value;
        proposed.validate()?;
        *self = proposed;
        Ok(())
    }
}

pub(crate) use crate::inspection::InspectionBudget as Budget;
