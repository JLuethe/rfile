// SPDX-License-Identifier: GPL-3.0-only
use super::*;
use ruzstd::decoding::StreamingDecoder;
fn little(data: &[u8], at: usize, n: usize) -> Result<u64> {
    Ok(field(data, at, n, CodecId::Zstd)?
        .iter()
        .rev()
        .fold(0, |v, &b| (v << 8) | u64::from(b)))
}
pub(super) fn one(
    data: &[u8],
    out: &mut InspectionBytes,
    budget: &InspectionBudget,
) -> Result<(usize, ChecksumState)> {
    let codec = CodecId::Zstd;
    let h = field(data, 0, 5, codec)?;
    if &h[..4] != b"\x28\xb5\x2f\xfd" || h[4] & 8 != 0 {
        return Err(failure(
            codec,
            DecodeErrorKind::InvalidHeader,
            0,
            "Zstd frame",
        ));
    }
    let flags = h[4];
    let single = flags & 32 != 0;
    let mut at = 5;
    let mut window = 0u64;
    if !single {
        let wd = field(data, at, 1, codec)?[0];
        at += 1;
        let base = 1u64 << (10 + (wd >> 3));
        window = base + (base / 8) * u64::from(wd & 7);
    }
    let dictionary = [0, 1, 2, 4][(flags & 3) as usize];
    let id = little(data, at, dictionary)?;
    at += dictionary;
    if id != 0 {
        return Err(failure(
            codec,
            DecodeErrorKind::DictionaryRequired,
            at,
            "external Zstd dictionary",
        ));
    }
    let size_length = match flags >> 6 {
        0 => usize::from(single),
        1 => 2,
        2 => 4,
        _ => 8,
    };
    if size_length != 0 {
        let mut size = little(data, at, size_length)?;
        if size_length == 2 {
            size += 256;
        }
        if single {
            window = size;
        }
        if size > budget.limits.bytes as u64 {
            return Err(Error::Limit("decompression output"));
        }
    }
    if window > budget.limits.codec_window.min(budget.limits.bytes) as u64 {
        return Err(Error::Limit("codec window"));
    }
    let window = usize::try_from(window).map_err(|_| Error::Limit("codec window"))?;
    // Ring growth may overlap old/new capacities; keep that peak reserved.
    let _memory = budget.reserve(window.saturating_mul(4).saturating_add(4 * 1024 * 1024))?;
    let mut source = Source::new(data, budget);
    budget.bytes(window)?;
    let decoded = (|| -> io::Result<_> {
        let mut decoder = StreamingDecoder::new_with_max_window_size(
            &mut source,
            budget.limits.codec_window.min(budget.limits.bytes) as u64,
        )
        .map_err(io::Error::other)?;
        read_output(&mut decoder, out, budget)?;
        Ok((
            decoder.decoder.get_checksum_from_data(),
            decoder.decoder.get_calculated_checksum(),
            decoder.decoder.is_finished(),
        ))
    })();
    let (checksum, calculated, finished) = match decoded {
        Ok(v) => v,
        Err(e) => return Err(io_failure(codec, &mut source, e)),
    };
    if !finished {
        return Err(failure(
            codec,
            DecodeErrorKind::Truncated,
            source.pos,
            "unfinished Zstd frame",
        ));
    }
    if checksum.is_some() && checksum != calculated {
        return Err(failure(
            codec,
            DecodeErrorKind::Checksum,
            source.pos,
            "Zstd xxHash64",
        ));
    }
    Ok((
        source.pos,
        if checksum.is_some() {
            ChecksumState::Verified
        } else {
            ChecksumState::Absent
        },
    ))
}
