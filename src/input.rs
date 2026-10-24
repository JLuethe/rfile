// SPDX-License-Identifier: GPL-3.0-only
//! Checked positional IO and owned buffers with per-inspection reservations.
use crate::{Error, InspectionBudget, MemoryReservation, Result};
use std::{
    fs::File,
    io::{self, Read},
    ops::{Deref, DerefMut},
};

pub trait InputView {
    fn known_len(&self) -> Option<u64>;
    /// Primitive positional read; core range drivers charge attempts, including
    /// interruptions, before invoking adapters. This prevents an adapter's error
    /// path from accidentally bypassing the request cap.
    fn read_at_unaccounted(&mut self, offset: u64, out: &mut [u8]) -> Result<usize>;
    fn read_at(&mut self, offset: u64, out: &mut [u8], budget: &InspectionBudget) -> Result<usize> {
        offset
            .checked_add(out.len() as u64)
            .ok_or(Error::Malformed {
                format: "input",
                stage: "range",
                offset,
            })?;
        budget.record_read(out.len())?;
        self.read_at_unaccounted(offset, out)
    }
}
pub struct SliceView<'a> {
    bytes: &'a [u8],
}
impl<'a> SliceView<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }
}
impl InputView for SliceView<'_> {
    fn known_len(&self) -> Option<u64> {
        Some(self.bytes.len() as u64)
    }
    fn read_at_unaccounted(&mut self, offset: u64, out: &mut [u8]) -> Result<usize> {
        let start = usize::try_from(offset)
            .ok()
            .and_then(|o| self.bytes.get(o..));
        let Some(bytes) = start else {
            return Ok(0);
        };
        let n = bytes.len().min(out.len());
        out[..n].copy_from_slice(&bytes[..n]);
        Ok(n)
    }
}
pub struct FileView<'a> {
    file: &'a File,
    len: Option<u64>,
}
impl<'a> FileView<'a> {
    pub fn new(file: &'a File) -> Result<Self> {
        let meta = file.metadata()?;
        Ok(Self {
            file,
            len: meta.is_file().then_some(meta.len()),
        })
    }
}
impl InputView for FileView<'_> {
    fn known_len(&self) -> Option<u64> {
        self.len
    }
    fn read_at_unaccounted(&mut self, offset: u64, out: &mut [u8]) -> Result<usize> {
        offset
            .checked_add(out.len() as u64)
            .ok_or(Error::Malformed {
                format: "input",
                stage: "range",
                offset,
            })?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::FileExt;
            Ok(self.file.read_at(out, offset)?)
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::FileExt;
            Ok(self.file.seek_read(out, offset)?)
        }
        #[cfg(not(any(unix, windows)))]
        {
            Err(Error::Unsupported(
                "positional file IO on this platform".into(),
            ))
        }
    }
}
#[derive(Debug)]
pub struct InspectionBytes {
    bytes: Vec<u8>,
    reservation: MemoryReservation,
    budget: InspectionBudget,
}
impl InspectionBytes {
    pub fn new(budget: &InspectionBudget) -> Result<Self> {
        Ok(Self {
            bytes: Vec::new(),
            reservation: budget.reserve(0)?,
            budget: budget.clone(),
        })
    }
    pub fn zeroed(len: usize, budget: &InspectionBudget) -> Result<Self> {
        let mut b = Self::new(budget)?;
        b.ensure_capacity(len)?;
        b.bytes.resize(len, 0);
        Ok(b)
    }
    fn ensure_capacity(&mut self, required: usize) -> Result<()> {
        if required <= self.bytes.capacity() {
            return Ok(());
        }
        // Reserve both old and replacement capacity during a possible realloc.
        let lease = self.budget.reserve(required)?;
        self.bytes
            .try_reserve_exact(required - self.bytes.len())
            .map_err(|e| Error::Io(io::Error::new(io::ErrorKind::OutOfMemory, e.to_string())))?;
        self.reservation = lease;
        Ok(())
    }
    pub fn extend(&mut self, bytes: &[u8]) -> Result<()> {
        let required = self
            .bytes
            .len()
            .checked_add(bytes.len())
            .ok_or(Error::Limit("buffer length"))?;
        if required > self.bytes.capacity() {
            let capacity = required.max(self.bytes.capacity().saturating_mul(2));
            self.ensure_capacity(capacity)?;
        }
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
    pub fn truncate(&mut self, len: usize) {
        self.bytes.truncate(len);
    }
}
impl Deref for InspectionBytes {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &self.bytes
    }
}
impl DerefMut for InspectionBytes {
    fn deref_mut(&mut self) -> &mut [u8] {
        &mut self.bytes
    }
}

pub fn read_prefix(
    reader: &mut impl Read,
    limit: usize,
    budget: &InspectionBudget,
) -> Result<InspectionBytes> {
    let mut out = InspectionBytes::new(budget)?;
    let mut chunk = [0u8; 4096];
    while out.len() < limit {
        let n = chunk.len().min(limit - out.len());
        budget.record_read(n)?;
        let got = match reader.read(&mut chunk[..n]) {
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            r => r?,
        };
        if got > n {
            return Err(Error::Malformed {
                format: "input",
                stage: "reader over-report",
                offset: out.len() as u64,
            });
        }
        if got == 0 {
            break;
        }
        out.extend(&chunk[..got])?;
    }
    Ok(out)
}
pub fn read_range(
    view: &mut (impl InputView + ?Sized),
    offset: u64,
    len: usize,
    budget: &InspectionBudget,
) -> Result<InspectionBytes> {
    offset.checked_add(len as u64).ok_or(Error::Malformed {
        format: "input",
        stage: "range",
        offset,
    })?;
    let available = view.known_len().map(|n| n.saturating_sub(offset));
    let size = available.map_or(len, |n| len.min(n.min(usize::MAX as u64) as usize));
    let mut out = InspectionBytes::zeroed(size, budget)?;
    let mut pos = 0;
    while pos < size {
        let end = size.min(pos + 4096);
        budget.record_read(end - pos)?;
        let got = match view.read_at_unaccounted(offset + pos as u64, &mut out[pos..end]) {
            Err(Error::Io(e)) if e.kind() == io::ErrorKind::Interrupted => continue,
            r => r?,
        };
        if got > end - pos {
            return Err(Error::Malformed {
                format: "input",
                stage: "reader over-report",
                offset: offset + pos as u64,
            });
        }
        if got == 0 {
            break;
        }
        pos += got;
    }
    out.truncate(pos);
    Ok(out)
}
