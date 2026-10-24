// SPDX-License-Identifier: GPL-3.0-only
//! UNIX compress LZW: little-bit-order codes in width-byte groups. Width
//! changes and CLEAR align to the old group boundary, unlike GIF LZW.
use super::*;
pub(super) fn one(
    data: &[u8],
    out: &mut InspectionBytes,
    budget: &InspectionBudget,
) -> Result<(usize, ChecksumState)> {
    let codec = CodecId::Compress;
    let header = field(data, 0, 3, codec)?;
    let max = header[2] & 31;
    let block = header[2] & 128 != 0;
    if &header[..2] != b"\x1f\x9d" || !(9..=16).contains(&max) || header[2] & 0x60 != 0 {
        return Err(failure(
            codec,
            DecodeErrorKind::InvalidHeader,
            2,
            "compress flags/width",
        ));
    }
    let capacity = 1usize << max;
    if capacity.saturating_mul(3) > budget.limits.codec_window {
        return Err(Error::Limit("codec dictionary"));
    }
    let _memory = budget.reserve(capacity.saturating_mul(8))?;
    let mut prefix = vec![0u16; capacity];
    let mut suffix = vec![0u8; capacity];
    let mut stack = Vec::with_capacity(capacity);
    let mut width = 9usize;
    let mut next = if block { 257 } else { 256 };
    let mut old = None;
    let mut first = 0u8;
    let mut bit = 24usize;
    let mut group = bit;
    while bit + width <= data.len().saturating_mul(8) {
        budget.step()?;
        if next > (1 << width) - 1 && width < max as usize {
            let n = width * 8;
            bit = group + (bit - group).div_ceil(n) * n;
            group = bit;
            width += 1;
            if bit + width > data.len() * 8 {
                break;
            }
        }
        let mut code = 0usize;
        for i in 0..width {
            code |= (((data[(bit + i) / 8] >> ((bit + i) % 8)) & 1) as usize) << i;
        }
        bit += width;
        if block && code == 256 {
            let n = width * 8;
            bit = group + (bit - group).div_ceil(n) * n;
            group = bit;
            width = 9;
            next = 257;
            old = None;
            continue;
        }
        let Some(previous) = old else {
            if code >= 256 {
                return Err(failure(
                    codec,
                    DecodeErrorKind::Corrupt,
                    bit / 8,
                    "nonliteral initial code",
                ));
            }
            first = code as u8;
            append(out, &[first], budget)?;
            old = Some(code);
            continue;
        };
        let current = code;
        stack.clear();
        if code == next {
            stack.push(first);
            code = previous;
        } else if code > next {
            return Err(failure(
                codec,
                DecodeErrorKind::Corrupt,
                bit / 8,
                "invalid dictionary code",
            ));
        }
        while code >= 256 {
            if code >= next || stack.len() >= capacity {
                return Err(failure(
                    codec,
                    DecodeErrorKind::Corrupt,
                    bit / 8,
                    "dictionary chain",
                ));
            }
            stack.push(suffix[code]);
            code = prefix[code] as usize;
            budget.step()?;
        }
        first = code as u8;
        stack.push(first);
        stack.reverse();
        append(out, &stack, budget)?;
        if next < capacity {
            prefix[next] = previous as u16;
            suffix[next] = first;
            next += 1;
        }
        old = Some(current);
    }
    Ok((data.len(), ChecksumState::Absent))
}
