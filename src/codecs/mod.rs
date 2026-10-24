// SPDX-License-Identifier: GPL-3.0-only
//! Bounded, in-process decoding. No native codec bindings or helper processes.
use crate::{Error, InspectionBudget, Result, SemanticProfile, input::InspectionBytes};
use std::io::{self, BufRead, Read};
#[cfg(feature = "codec-bzip2")]
mod bzip2;
#[cfg(feature = "codec-compress")]
mod compress;
#[cfg(feature = "codec-lz4")]
mod lz4;
#[cfg(feature = "codec-lzma")]
mod lzma;
#[cfg(feature = "codec-lzma")]
mod xz_frame;
#[cfg(feature = "codec-zip")]
mod zip;
#[cfg(feature = "codec-zstd")]
mod zstd;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodecId {
    Gzip,
    Zlib,
    Bzip2,
    Xz,
    Lzma,
    Lzip,
    Zstd,
    Lz4,
    Compress,
    Zip,
    Frozen,
    ScoLzh,
    Pack,
    Lrzip,
}
impl CodecId {
    pub fn name(self) -> &'static str {
        match self {
            Self::Gzip => "gzip",
            Self::Zlib => "zlib",
            Self::Bzip2 => "bzip2",
            Self::Xz => "xz",
            Self::Lzma => "lzma",
            Self::Lzip => "lzip",
            Self::Zstd => "zstd",
            Self::Lz4 => "lz4",
            Self::Compress => "compress",
            Self::Zip => "zip",
            Self::Frozen => "frozen",
            Self::ScoLzh => "sco-lzh",
            Self::Pack => "pack",
            Self::Lrzip => "lrzip",
        }
    }
    // Preserve the feature checks even when all features constant-fold to true.
    #[allow(clippy::match_like_matches_macro)]
    pub fn compiled(self) -> bool {
        match self {
            Self::Gzip | Self::Zlib => true,
            Self::Bzip2 => cfg!(feature = "codec-bzip2"),
            Self::Xz | Self::Lzma | Self::Lzip => cfg!(feature = "codec-lzma"),
            Self::Zstd => cfg!(feature = "codec-zstd"),
            Self::Lz4 => cfg!(feature = "codec-lz4"),
            Self::Compress => cfg!(feature = "codec-compress"),
            Self::Zip => cfg!(feature = "codec-zip"),
            _ => false,
        }
    }
}
pub const COMPILED: &[&str] = &[
    "gzip",
    "zlib",
    #[cfg(feature = "codec-bzip2")]
    "bzip2",
    #[cfg(feature = "codec-lzma")]
    "xz",
    #[cfg(feature = "codec-lzma")]
    "lzma",
    #[cfg(feature = "codec-lzma")]
    "lzip",
    #[cfg(feature = "codec-zstd")]
    "zstd",
    #[cfg(feature = "codec-lz4")]
    "lz4",
    #[cfg(feature = "codec-compress")]
    "compress",
    #[cfg(feature = "codec-zip")]
    "zip",
];
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecodeErrorKind {
    InvalidHeader,
    Truncated,
    Corrupt,
    Checksum,
    DictionaryRequired,
    UnsupportedMethod,
    TrailingData,
    NoProgress,
}
#[derive(Debug)]
pub struct DecodeError {
    pub codec: CodecId,
    pub kind: DecodeErrorKind,
    pub offset: u64,
    pub detail: String,
}
impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} decompression {:?} at {}: {}",
            self.codec.name(),
            self.kind,
            self.offset,
            self.detail
        )
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChecksumState {
    Verified,
    Absent,
}
#[derive(Clone, Debug)]
pub struct DecodeFacts {
    pub codec: CodecId,
    pub input_used: usize,
    pub members: usize,
    pub complete: bool,
    pub checksum: ChecksumState,
}
#[derive(Debug)]
pub struct DecodeOutcome {
    pub bytes: InspectionBytes,
    pub facts: DecodeFacts,
}
fn failure(
    codec: CodecId,
    kind: DecodeErrorKind,
    offset: usize,
    detail: impl Into<String>,
) -> Error {
    Error::Decode(DecodeError {
        codec,
        kind,
        offset: offset as u64,
        detail: detail.into(),
    })
}
fn field(data: &[u8], at: usize, n: usize, codec: CodecId) -> Result<&[u8]> {
    data.get(
        at..at
            .checked_add(n)
            .ok_or_else(|| failure(codec, DecodeErrorKind::InvalidHeader, at, "range overflow"))?,
    )
    .ok_or_else(|| failure(codec, DecodeErrorKind::Truncated, at, "short field"))
}
pub fn probe(data: &[u8]) -> Option<CodecId> {
    // Same order as the pinned dispatch, including its weak LZMA/zlib probes.
    for (magic, id) in [
        (b"\x1f\x9d".as_slice(), CodecId::Compress),
        (b"\x1f\x8b", CodecId::Gzip),
        (b"\x1f\x9e", CodecId::Frozen),
        (b"\x1f\xa0", CodecId::ScoLzh),
        (b"\x1f\x1e", CodecId::Pack),
        (b"PK\x03\x04", CodecId::Zip),
        (b"BZh", CodecId::Bzip2),
        (b"LZIP", CodecId::Lzip),
        (b"\xfd7zXZ\0", CodecId::Xz),
        (b"LRZI", CodecId::Lrzip),
        (b"\x04\x22M\x18", CodecId::Lz4),
        (b"\x28\xb5\x2f\xfd", CodecId::Zstd),
    ] {
        if data.starts_with(magic) {
            return Some(id);
        }
    }
    if data.len() >= 13 && data[..3] == [0x5d, 0, 0] && (data[12] == 0 || data[12] == 255) {
        return Some(CodecId::Lzma);
    }
    if data.len() >= 2
        && (data[0] & 15) == 8
        && data[0] & 128 == 0
        && u16::from_be_bytes([data[0], data[1]]).is_multiple_of(31)
    {
        return Some(CodecId::Zlib);
    }
    None
}
fn append(out: &mut InspectionBytes, data: &[u8], budget: &InspectionBudget) -> Result<()> {
    budget.checkpoint()?;
    if out.len().saturating_add(data.len()) > budget.limits.bytes {
        return Err(Error::Limit("decompression output"));
    }
    budget.bytes(data.len())?;
    out.extend(data)
}
/// A bounded memory source gives blocking backends cancellation/work checkpoints
/// while retaining the actual consumed position. Memory reads are not syscalls.
struct Source<'a> {
    data: &'a [u8],
    pos: usize,
    budget: &'a InspectionBudget,
    failure: Option<Error>,
}
impl<'a> Source<'a> {
    fn new(data: &'a [u8], budget: &'a InspectionBudget) -> Self {
        Self {
            data,
            pos: 0,
            budget,
            failure: None,
        }
    }
    fn charge(&mut self, n: usize) -> io::Result<()> {
        if let Err(e) = self.budget.bytes(n) {
            self.failure = Some(e);
            return Err(io::Error::other("inspection control"));
        }
        if n == 0 {
            if let Err(e) = self.budget.step() {
                self.failure = Some(e);
                return Err(io::Error::other("inspection control"));
            }
        }
        Ok(())
    }
}
impl Read for Source<'_> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        let n = out.len().min(16 * 1024).min(self.data.len() - self.pos);
        self.charge(n)?;
        out[..n].copy_from_slice(&self.data[self.pos..self.pos + n]);
        self.pos += n;
        Ok(n)
    }
}
impl BufRead for Source<'_> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        let n = (self.data.len() - self.pos).min(16 * 1024);
        self.charge(n)?;
        Ok(&self.data[self.pos..self.pos + n])
    }
    fn consume(&mut self, n: usize) {
        self.pos = self.pos.saturating_add(n).min(self.data.len());
    }
}
fn read_output(
    reader: &mut impl Read,
    out: &mut InspectionBytes,
    budget: &InspectionBudget,
) -> io::Result<()> {
    let mut chunk = [0u8; 16 * 1024];
    loop {
        budget.checkpoint().map_err(io::Error::other)?;
        let remaining = budget.limits.bytes - out.len();
        let request = chunk.len().min(remaining.saturating_add(1));
        let n = reader.read(&mut chunk[..request])?;
        if n > request {
            return Err(io::Error::other("decoder over-report"));
        }
        if n == 0 {
            return Ok(());
        }
        append(out, &chunk[..n], budget).map_err(io::Error::other)?;
    }
}
fn io_failure(codec: CodecId, source: &mut Source<'_>, error: io::Error) -> Error {
    if let Some(e) = source.failure.take() {
        return e;
    }
    // Preserve cancellation/resource errors carried through a Read adapter.
    if error.get_ref().is_some_and(|e| e.is::<Error>()) {
        return *error
            .into_inner()
            .expect("checked inner")
            .downcast::<Error>()
            .expect("checked type");
    }
    let kind = if error.kind() == io::ErrorKind::UnexpectedEof {
        DecodeErrorKind::Truncated
    } else {
        DecodeErrorKind::Corrupt
    };
    failure(codec, kind, source.pos, error.to_string())
}
fn gzip_header(data: &[u8]) -> Result<usize> {
    let c = CodecId::Gzip;
    let header = field(data, 0, 10, c)?;
    if header[2] != 8 || header[3] & 0xe0 != 0 {
        return Err(failure(
            c,
            DecodeErrorKind::InvalidHeader,
            2,
            "method or reserved flags",
        ));
    }
    let flags = header[3];
    let mut at = 10;
    if flags & 4 != 0 {
        let len = u16::from_le_bytes(field(data, at, 2, c)?.try_into().expect("width")) as usize;
        at += 2;
        field(data, at, len, c)?;
        at += len;
    }
    for flag in [8, 16] {
        if flags & flag != 0 {
            let bytes = data
                .get(at..)
                .ok_or_else(|| failure(c, DecodeErrorKind::Truncated, at, "header"))?;
            let len = bytes
                .iter()
                .take(65537)
                .position(|&b| b == 0)
                .ok_or_else(|| {
                    if bytes.len() > 65536 {
                        Error::Limit("gzip header string")
                    } else {
                        failure(
                            c,
                            DecodeErrorKind::Truncated,
                            at,
                            "unterminated header string",
                        )
                    }
                })?;
            at += len + 1;
        }
    }
    if flags & 2 != 0 {
        field(data, at, 2, c)?;
        at += 2;
    }
    Ok(at)
}
fn deflate_one(
    codec: CodecId,
    data: &[u8],
    out: &mut InspectionBytes,
    budget: &InspectionBudget,
) -> Result<(usize, ChecksumState)> {
    let header = if codec == CodecId::Gzip {
        gzip_header(data)?
    } else {
        field(data, 0, 2, codec)?;
        if data[1] & 0x20 != 0 {
            return Err(failure(
                codec,
                DecodeErrorKind::DictionaryRequired,
                1,
                "preset dictionary",
            ));
        }
        2
    };
    if budget.limits.codec_window < 32768 {
        return Err(Error::Limit("codec window"));
    }
    let _memory = budget.reserve(128 * 1024 + header.saturating_mul(4))?;
    let mut source = Source::new(data, budget);
    let result = if codec == CodecId::Gzip {
        let mut decoder = flate2::bufread::GzDecoder::new(&mut source);
        read_output(&mut decoder, out, budget)
    } else {
        let mut decoder = flate2::bufread::ZlibDecoder::new(&mut source);
        read_output(&mut decoder, out, budget)
    };
    if let Err(e) = result {
        return Err(io_failure(codec, &mut source, e));
    }
    Ok((source.pos, ChecksumState::Verified))
}
pub fn probe_with_profile(data: &[u8], profile: SemanticProfile) -> Option<CodecId> {
    if profile == SemanticProfile::Pinned548 {
        return probe(data);
    }
    let mut at = 0usize;
    for _ in 0..1024 {
        let bytes = data.get(at..at.checked_add(4)?)?;
        let magic = u32::from_le_bytes(bytes.try_into().ok()?);
        if magic & 0xfffffff0 != 0x184d2a50 {
            let found = probe(&data[at..]);
            return if at == 0 {
                found
            } else {
                found.filter(|id| matches!(id, CodecId::Zstd | CodecId::Lz4))
            };
        }
        let n = u32::from_le_bytes(data.get(at + 4..at + 8)?.try_into().ok()?) as usize;
        at = at.checked_add(8)?.checked_add(n)?;
        if at >= data.len() {
            return None;
        }
    }
    // The prefix is still ambiguous between LZ4 and Zstd; never guess a codec.
    None
}
pub fn decode(
    data: &[u8],
    codec: CodecId,
    budget: &InspectionBudget,
    profile: SemanticProfile,
) -> Result<DecodeOutcome> {
    if !codec.compiled() {
        return Err(Error::Unsupported(format!(
            "{} decompression; compile the matching codec feature",
            codec.name()
        )));
    }
    let mut output = InspectionBytes::new(budget)?;
    let mut at = 0;
    let mut members = 0;
    let mut checksum = ChecksumState::Verified;
    let mut payload_members = 0usize;
    loop {
        budget.checkpoint()?;
        if members >= budget.limits.codec_members {
            return Err(Error::Limit("compression members"));
        }
        members += 1;
        budget.step()?;
        if matches!(codec, CodecId::Zstd | CodecId::Lz4) && data.len() - at >= 4 {
            let magic = u32::from_le_bytes(data[at..at + 4].try_into().expect("checked width"));
            if magic & 0xfffffff0 == 0x184d2a50 {
                let size =
                    u32::from_le_bytes(field(data, at + 4, 4, codec)?.try_into().expect("width"))
                        as usize;
                field(data, at + 8, size, codec)?;
                budget.bytes(size.saturating_add(8))?;
                at += 8 + size;
                if at == data.len() {
                    break;
                }
                continue;
            }
        }
        payload_members += 1;

        let (used, check) = match codec {
            CodecId::Gzip | CodecId::Zlib => deflate_one(codec, &data[at..], &mut output, budget)?,
            #[cfg(feature = "codec-bzip2")]
            CodecId::Bzip2 => bzip2::one(&data[at..], &mut output, budget)?,
            #[cfg(feature = "codec-lzma")]
            CodecId::Xz | CodecId::Lzma | CodecId::Lzip => {
                lzma::one(codec, &data[at..], &mut output, budget)?
            }
            #[cfg(feature = "codec-zstd")]
            CodecId::Zstd => zstd::one(&data[at..], &mut output, budget)?,
            #[cfg(feature = "codec-lz4")]
            CodecId::Lz4 => lz4::one(&data[at..], &mut output, budget)?,
            #[cfg(feature = "codec-compress")]
            CodecId::Compress => compress::one(&data[at..], &mut output, budget)?,
            #[cfg(feature = "codec-zip")]
            CodecId::Zip => zip::one(&data[at..], &mut output, budget)?,
            _ => {
                return Err(Error::Unsupported(format!(
                    "{} decompression",
                    codec.name()
                )));
            }
        };
        if used == 0 || used > data.len() - at {
            return Err(failure(
                codec,
                DecodeErrorKind::NoProgress,
                at,
                "invalid consumption",
            ));
        }
        at += used;
        if check == ChecksumState::Absent {
            checksum = ChecksumState::Absent;
        }
        if profile == SemanticProfile::Pinned548
            || matches!(codec, CodecId::Zip | CodecId::Compress | CodecId::Lzma)
        {
            break;
        }
        if codec == CodecId::Xz {
            let before = at;
            while data.get(at) == Some(&0) {
                if (at - before).is_multiple_of(4096) {
                    budget.bytes(4096.min(data.len() - at))?;
                }
                at += 1;
            }
            if !(at - before).is_multiple_of(4) {
                return Err(failure(
                    codec,
                    DecodeErrorKind::TrailingData,
                    before,
                    "XZ stream padding",
                ));
            }
        }
        if at == data.len() {
            break;
        }
        let skippable = matches!(codec, CodecId::Zstd | CodecId::Lz4)
            && data.get(at..at + 4).is_some_and(|b| {
                u32::from_le_bytes(b.try_into().expect("width")) & 0xfffffff0 == 0x184d2a50
            });
        if !skippable && probe(&data[at..]) != Some(codec) {
            return Err(failure(
                codec,
                DecodeErrorKind::TrailingData,
                at,
                "trailing non-member bytes",
            ));
        }
    }
    Ok(DecodeOutcome {
        bytes: output,
        facts: DecodeFacts {
            codec,
            input_used: at,
            members,
            complete: true,
            checksum: if payload_members == 0 {
                ChecksumState::Absent
            } else {
                checksum
            },
        },
    })
}
/// Drive raw DEFLATE explicitly to StreamEnd; EOF alone does not prove that a
/// ZIP member's final block existed. At most one output lookahead byte is used.
#[cfg(feature = "codec-zip")]
fn raw_deflate(data: &[u8], out: &mut InspectionBytes, budget: &InspectionBudget) -> Result<usize> {
    let codec = CodecId::Zip;
    let mut decoder = flate2::Decompress::new(false);
    let mut at = 0usize;
    let mut chunk = [0u8; 16 * 1024];
    loop {
        budget.step()?;
        let end = data.len().min(at.saturating_add(16 * 1024));
        let request = chunk
            .len()
            .min((budget.limits.bytes - out.len()).saturating_add(1));
        let before_in = decoder.total_in();
        let before_out = decoder.total_out();
        let status = decoder
            .decompress(
                &data[at..end],
                &mut chunk[..request],
                flate2::FlushDecompress::None,
            )
            .map_err(|e| failure(codec, DecodeErrorKind::Corrupt, at, e.to_string()))?;
        let consumed = (decoder.total_in() - before_in) as usize;
        let produced = (decoder.total_out() - before_out) as usize;
        budget.bytes(consumed)?;
        at += consumed;
        append(out, &chunk[..produced], budget)?;
        if status == flate2::Status::StreamEnd {
            return Ok(at);
        }
        if consumed == 0 && produced == 0 {
            return Err(failure(
                codec,
                if at == data.len() {
                    DecodeErrorKind::Truncated
                } else {
                    DecodeErrorKind::NoProgress
                },
                at,
                "raw DEFLATE did not finish",
            ));
        }
    }
}
