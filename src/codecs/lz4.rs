// SPDX-License-Identifier: GPL-3.0-only
use super::*;
pub(super) fn one(
    data: &[u8],
    out: &mut InspectionBytes,
    budget: &InspectionBudget,
) -> Result<(usize, ChecksumState)> {
    let codec = CodecId::Lz4;
    let header = field(data, 0, 7, codec)?;
    let flags = header[4];
    let bd = header[5];
    if &header[..4] != b"\x04\x22M\x18" || flags >> 6 != 1 || flags & 2 != 0 || bd & 0x8f != 0 {
        return Err(failure(
            codec,
            DecodeErrorKind::InvalidHeader,
            0,
            "modern LZ4 frame",
        ));
    }
    let block = match bd >> 4 {
        4 => 64 * 1024,
        5 => 256 * 1024,
        6 => 1024 * 1024,
        7 => 4 * 1024 * 1024,
        _ => {
            return Err(failure(
                codec,
                DecodeErrorKind::InvalidHeader,
                5,
                "block category",
            ));
        }
    };
    if block > budget.limits.codec_window {
        return Err(Error::Limit("codec window"));
    }
    let mut at = 6;
    if flags & 8 != 0 {
        let size = u64::from_le_bytes(field(data, at, 8, codec)?.try_into().expect("width"));
        at += 8;
        if size > budget.limits.bytes as u64 {
            return Err(Error::Limit("decompression output"));
        }
    }
    if flags & 1 != 0 {
        return Err(failure(
            codec,
            DecodeErrorKind::DictionaryRequired,
            at,
            "LZ4 dictionary flag",
        ));
    }
    field(data, at, 1, codec)?;
    at += 1;
    loop {
        budget.step()?;
        budget.bytes(4)?;
        let encoded = u32::from_le_bytes(field(data, at, 4, codec)?.try_into().expect("width"));
        at += 4;
        if encoded == 0 {
            break;
        }
        let n = (encoded & 0x7fffffff) as usize;
        if n > block {
            return Err(failure(
                codec,
                DecodeErrorKind::Corrupt,
                at,
                "block size exceeds frame category",
            ));
        }
        field(data, at, n, codec)?;
        at += n;
        if flags & 16 != 0 {
            field(data, at, 4, codec)?;
            at += 4;
        }
    }
    if flags & 4 != 0 {
        field(data, at, 4, codec)?;
        at += 4;
    }
    let _memory = budget.reserve(
        block
            .saturating_mul(3)
            .saturating_add(64 * 1024 + 256 * 1024),
    )?;
    let mut source = Source::new(&data[..at], budget);
    let result = {
        let mut decoder = lz4_flex::frame::FrameDecoder::new(&mut source);
        read_output(&mut decoder, out, budget)
    };
    if let Err(e) = result {
        return Err(io_failure(codec, &mut source, e));
    }
    if source.pos != at {
        return Err(failure(
            codec,
            DecodeErrorKind::Truncated,
            source.pos,
            "incomplete LZ4 consumption",
        ));
    }
    Ok((
        at,
        if flags & 20 != 0 {
            ChecksumState::Verified
        } else {
            ChecksumState::Absent
        },
    ))
}
