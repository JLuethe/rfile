// SPDX-License-Identifier: GPL-3.0-only
//! First physical ZIP member only. No extraction, path resolution or helpers.
use super::*;
fn u16at(b: &[u8], p: usize) -> Result<u16> {
    Ok(u16::from_le_bytes(
        field(b, p, 2, CodecId::Zip)?.try_into().expect("width"),
    ))
}
fn u32at(b: &[u8], p: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(
        field(b, p, 4, CodecId::Zip)?.try_into().expect("width"),
    ))
}
fn crc32(bytes: &[u8], budget: &InspectionBudget) -> Result<u32> {
    let mut crc = !0u32;
    for chunk in bytes.chunks(4096) {
        budget.bytes(chunk.len())?;
        for &b in chunk {
            crc ^= b as u32;
            for _ in 0..8 {
                crc = (crc >> 1) ^ (0xedb88320u32.wrapping_mul(crc & 1));
            }
        }
    }
    Ok(!crc)
}
pub(super) fn one(
    data: &[u8],
    out: &mut InspectionBytes,
    budget: &InspectionBudget,
) -> Result<(usize, ChecksumState)> {
    let codec = CodecId::Zip;
    field(data, 0, 30, codec)?;
    if &data[..4] != b"PK\x03\x04" {
        return Err(failure(
            codec,
            DecodeErrorKind::InvalidHeader,
            0,
            "ZIP local header",
        ));
    }
    let flags = u16at(data, 6)?;
    let method = u16at(data, 8)?;
    if flags & 1 != 0 || flags & 0x40 != 0 {
        return Err(failure(
            codec,
            DecodeErrorKind::UnsupportedMethod,
            6,
            "encrypted ZIP entry",
        ));
    }
    if !matches!(method, 0 | 8) {
        return Err(failure(
            codec,
            DecodeErrorKind::UnsupportedMethod,
            8,
            format!("ZIP method {method}"),
        ));
    }
    let mut compressed = u32at(data, 18)? as u64;
    let mut expanded = u32at(data, 22)? as u64;
    let mut zip64 = compressed == 0xffffffff || expanded == 0xffffffff;
    let name = u16at(data, 26)? as usize;
    let extra = u16at(data, 28)? as usize;
    let at = 30usize
        .checked_add(name)
        .and_then(|n| n.checked_add(extra))
        .ok_or_else(|| failure(codec, DecodeErrorKind::InvalidHeader, 26, "ZIP lengths"))?;
    field(data, 30, name + extra, codec)?;
    let mut pos = 30 + name;
    while pos + 4 <= at {
        let id = u16at(data, pos)?;
        let size = u16at(data, pos + 2)? as usize;
        pos += 4;
        let entry = field(data, pos, size, codec)?;
        if pos + size > at {
            return Err(failure(
                codec,
                DecodeErrorKind::InvalidHeader,
                pos,
                "extra field range",
            ));
        }
        if id == 1 {
            zip64 = true;
            let mut cursor = 0;
            if expanded == 0xffffffff {
                expanded =
                    u64::from_le_bytes(field(entry, cursor, 8, codec)?.try_into().expect("width"));
                cursor += 8;
            }
            if compressed == 0xffffffff {
                compressed =
                    u64::from_le_bytes(field(entry, cursor, 8, codec)?.try_into().expect("width"));
            }
        }
        pos += size;
    }
    if expanded > budget.limits.bytes as u64 && flags & 8 == 0 {
        return Err(Error::Limit("decompression output"));
    }
    let start = out.len();
    let used;
    if method == 0 {
        if flags & 8 != 0 {
            return Err(failure(
                codec,
                DecodeErrorKind::UnsupportedMethod,
                6,
                "stored entry with unknown data descriptor boundary",
            ));
        }
        let size = usize::try_from(compressed).map_err(|_| Error::Limit("ZIP entry size"))?;
        append(out, field(data, at, size, codec)?, budget)?;
        used = size;
    } else {
        if budget.limits.codec_window < 32768 {
            return Err(Error::Limit("codec window"));
        }
        let _memory = budget.reserve(128 * 1024)?;
        let input = if flags & 8 != 0 {
            &data[at..]
        } else {
            field(
                data,
                at,
                usize::try_from(compressed).map_err(|_| Error::Limit("ZIP entry size"))?,
                codec,
            )?
        };
        used = raw_deflate(input, out, budget)?;
        if flags & 8 == 0 && used as u64 != compressed {
            return Err(failure(
                codec,
                DecodeErrorKind::Corrupt,
                at + used,
                "compressed size mismatch",
            ));
        }
    }
    let mut end = at + used;
    let mut expected = u32at(data, 14)?;
    if flags & 8 != 0 {
        if u32at(data, end)? == 0x08074b50 {
            end += 4;
        }
        expected = u32at(data, end)?;
        let (actual_compressed, actual_expanded) = if zip64 {
            (
                u64::from_le_bytes(field(data, end + 4, 8, codec)?.try_into().expect("width")),
                u64::from_le_bytes(field(data, end + 12, 8, codec)?.try_into().expect("width")),
            )
        } else {
            (u32at(data, end + 4)? as u64, u32at(data, end + 8)? as u64)
        };
        end += if zip64 { 20 } else { 12 };
        if actual_compressed != used as u64 || actual_expanded != (out.len() - start) as u64 {
            return Err(failure(
                codec,
                DecodeErrorKind::Corrupt,
                end,
                "data descriptor size",
            ));
        }
        expanded = actual_expanded;
    }
    if expanded != (out.len() - start) as u64 {
        return Err(failure(
            codec,
            DecodeErrorKind::Corrupt,
            end,
            "uncompressed size mismatch",
        ));
    }
    budget.bytes(out.len() - start)?;
    if crc32(&out[start..], budget)? != expected {
        return Err(failure(codec, DecodeErrorKind::Checksum, end, "ZIP CRC32"));
    }
    Ok((end, ChecksumState::Verified))
}
