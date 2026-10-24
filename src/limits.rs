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
        Ok(())
    }
    pub fn set(&mut self, name: &str, value: usize) -> Result<()> {
        let field = match name {
            "bytes" => &mut self.bytes,
            "encoding" => &mut self.encoding,
            "indir" => &mut self.indirect,
            "name" => &mut self.names,
            "regex" => &mut self.regex,
            "elf_notes" => &mut self.elf_notes,
            "elf_phnum" => &mut self.elf_phnum,
            "elf_shnum" => &mut self.elf_shnum,
            "elf_shsize" => &mut self.elf_shsize,
            "magwarn" => &mut self.magwarn,
            "steps" => &mut self.steps,
            "work" => &mut self.work_bytes,
            "output" => &mut self.output,
            _ => return Err(Error::Unsupported(format!("parameter {name}"))),
        };
        let old = *field;
        *field = value;
        if let Err(e) = self.validate() {
            self.set(name, old)?;
            return Err(e);
        }
        Ok(())
    }
}

pub(crate) struct Budget<'a> {
    pub limits: &'a Limits,
    steps: usize,
    work: usize,
}
impl<'a> Budget<'a> {
    pub fn new(limits: &'a Limits) -> Self {
        Self {
            limits,
            steps: 0,
            work: 0,
        }
    }
    pub fn step(&mut self) -> Result<()> {
        self.steps = self.steps.checked_add(1).ok_or(Error::Limit("steps"))?;
        if self.steps > self.limits.steps {
            return Err(Error::Limit("steps"));
        }
        Ok(())
    }
    pub fn bytes(&mut self, n: usize) -> Result<()> {
        self.work = self.work.checked_add(n).ok_or(Error::Limit("work bytes"))?;
        if self.work > self.limits.work_bytes {
            return Err(Error::Limit("work bytes"));
        }
        Ok(())
    }
}
