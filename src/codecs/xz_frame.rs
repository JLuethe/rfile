// SPDX-License-Identifier: GPL-3.0-only
//! Validate a complete first XZ envelope before either backend API allocates.
//! LZMA2 chunk headers locate block boundaries without decoding their payload.
use super::*;
const C: CodecId = CodecId::Xz;
fn invalid(at: usize, detail: &str) -> Error {
    failure(C, DecodeErrorKind::Corrupt, at, detail)
}
fn crc(bytes: &[u8], budget: &InspectionBudget) -> Result<u32> {
    let mut value = !0u32;
    for chunk in bytes.chunks(4096) {
        budget.bytes(chunk.len())?;
        for &b in chunk {
            value ^= b as u32;
            for _ in 0..8 {
                value = (value >> 1) ^ (0xedb88320u32.wrapping_mul(value & 1));
            }
        }
    }
    Ok(!value)
}
fn check_crc(data: &[u8], start: usize, end: usize, budget: &InspectionBudget) -> Result<()> {
    let expected = u32::from_le_bytes(field(data, end, 4, C)?.try_into().expect("width"));
    if crc(&data[start..end], budget)? != expected {
        return Err(failure(
            C,
            DecodeErrorKind::Checksum,
            end,
            "XZ envelope CRC32",
        ));
    }
    Ok(())
}
fn vli(data: &[u8], at: &mut usize) -> Result<u64> {
    let mut value = 0;
    for i in 0..9 {
        let b = field(data, *at, 1, C)?[0];
        *at += 1;
        value |= u64::from(b & 127) << (i * 7);
        if b & 128 == 0 {
            if i != 0 && b == 0 {
                return Err(invalid(*at, "noncanonical VLI"));
            }
            return Ok(value);
        }
    }
    Err(invalid(*at, "oversized VLI"))
}
pub(super) fn preflight(data: &[u8], budget: &InspectionBudget) -> Result<usize> {
    let header = field(data, 0, 12, C)?;
    if &header[..6] != b"\xfd7zXZ\0" || header[6] != 0 {
        return Err(invalid(0, "XZ stream header"));
    }
    let check = match header[7] {
        0 => 0,
        1 => 4,
        4 => 8,
        10 => 32,
        _ => {
            return Err(failure(
                C,
                DecodeErrorKind::UnsupportedMethod,
                7,
                "XZ check type",
            ));
        }
    };
    check_crc(data, 6, 8, budget)?;
    let mut at = 12;
    let mut records = Vec::new();
    let _memory = budget.reserve(budget.limits.codec_members.saturating_mul(32))?;
    while field(data, at, 1, C)?[0] != 0 {
        budget.step()?;
        if records.len() >= budget.limits.codec_members {
            return Err(Error::Limit("XZ blocks"));
        }
        let start = at;
        let size = (usize::from(data[at]) + 1) * 4;
        let block = field(data, at, size, C)?;
        check_crc(block, 0, size - 4, budget)?;
        let flags = block[1];
        if flags & 0x3c != 0 {
            return Err(invalid(at + 1, "XZ block flags"));
        }
        let mut p = 2;
        let compressed = if flags & 64 != 0 {
            Some(vli(block, &mut p)?)
        } else {
            None
        };
        let expanded = if flags & 128 != 0 {
            Some(vli(block, &mut p)?)
        } else {
            None
        };
        for i in 0..=flags & 3 {
            let id = vli(block, &mut p)?;
            let n = usize::try_from(vli(block, &mut p)?)
                .map_err(|_| invalid(at, "XZ filter length"))?;
            let props = field(block, p, n, C)?;
            p = p
                .checked_add(n)
                .ok_or_else(|| invalid(at, "XZ filter range"))?;
            if i == flags & 3 {
                if id != 0x21 || props.len() != 1 || props[0] > 40 {
                    return Err(invalid(at, "XZ final LZMA2 filter"));
                }
                let dict = if props[0] == 40 {
                    u32::MAX as u64
                } else {
                    (2u64 | u64::from(props[0] & 1)) << (props[0] / 2 + 11)
                };
                if dict > budget.limits.codec_window as u64 {
                    return Err(Error::Limit("codec dictionary"));
                }
            }
        }
        if p > size - 4 || block[p..size - 4].iter().any(|&b| b != 0) {
            return Err(invalid(at, "XZ block header padding"));
        }
        at += size;
        let payload = at;
        let mut uncompressed = 0u64;
        loop {
            budget.step()?;
            let control = field(data, at, 1, C)?[0];
            at += 1;
            if control == 0 {
                break;
            }
            let (input, output) = if control <= 2 {
                let n = usize::from(u16::from_be_bytes(
                    field(data, at, 2, C)?.try_into().expect("width"),
                )) + 1;
                at += 2;
                (n, n as u64)
            } else if control >= 128 {
                let h = field(data, at, 4, C)?;
                let output = ((u64::from(control & 31) << 16)
                    | u64::from(u16::from_be_bytes([h[0], h[1]])))
                    + 1;
                let n = usize::from(u16::from_be_bytes([h[2], h[3]])) + 1;
                at += 4;
                if control >= 192 {
                    field(data, at, 1, C)?;
                    at += 1;
                }
                (n, output)
            } else {
                return Err(invalid(at - 1, "LZMA2 control"));
            };
            field(data, at, input, C)?;
            budget.bytes(input)?;
            at += input;
            uncompressed = uncompressed
                .checked_add(output)
                .ok_or_else(|| invalid(at, "XZ expanded size overflow"))?;
            if uncompressed > budget.limits.bytes as u64 {
                return Err(Error::Limit("decompression output"));
            }
        }
        let payload_size = (at - payload) as u64;
        if compressed.is_some_and(|n| n != payload_size)
            || expanded.is_some_and(|n| n != uncompressed)
        {
            return Err(invalid(at, "XZ block sizes"));
        }
        let unpadded = size as u64 + payload_size + check as u64;
        while !(at - start).is_multiple_of(4) {
            if field(data, at, 1, C)?[0] != 0 {
                return Err(invalid(at, "XZ block padding"));
            }
            at += 1;
        }
        field(data, at, check, C)?;
        at += check;
        records.push((unpadded, uncompressed));
    }
    let index = at;
    at += 1;
    let count = vli(data, &mut at)?;
    if count != records.len() as u64 {
        return Err(invalid(at, "XZ index record count"));
    }
    for (unpadded, expanded) in records {
        budget.step()?;
        if vli(data, &mut at)? != unpadded || vli(data, &mut at)? != expanded {
            return Err(invalid(at, "XZ index sizes"));
        }
    }
    while !(at - index).is_multiple_of(4) {
        if field(data, at, 1, C)?[0] != 0 {
            return Err(invalid(at, "XZ index padding"));
        }
        at += 1;
    }
    check_crc(data, index, at, budget)?;
    at += 4;
    let index_size = at - index;
    let footer = field(data, at, 12, C)?;
    if crc(&footer[4..10], budget)? != u32::from_le_bytes(footer[..4].try_into().expect("width")) {
        return Err(failure(C, DecodeErrorKind::Checksum, at, "XZ footer CRC32"));
    }
    let backward = u32::from_le_bytes(footer[4..8].try_into().expect("width")) as u64;
    if (backward + 1) * 4 != index_size as u64
        || footer[8..10] != header[6..8]
        || &footer[10..] != b"YZ"
    {
        return Err(invalid(at, "XZ footer"));
    }
    Ok(at + 12)
}
