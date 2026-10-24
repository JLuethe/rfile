// SPDX-License-Identifier: GPL-3.0-only
use super::*;
use lzma_rust2::{Action, LzipStream, LzmaStream, Status, StreamResult, XzStream};
trait Stream {
    fn process(
        &mut self,
        input: &[u8],
        output: &mut [u8],
        action: Action,
    ) -> io::Result<StreamResult>;
    fn unused(&self) -> usize {
        0
    }
}
impl Stream for LzmaStream {
    fn process(&mut self, i: &[u8], o: &mut [u8], a: Action) -> io::Result<StreamResult> {
        self.process(i, o, a)
    }
    fn unused(&self) -> usize {
        self.unused_input().len()
    }
}
impl Stream for LzipStream {
    fn process(&mut self, i: &[u8], o: &mut [u8], a: Action) -> io::Result<StreamResult> {
        self.process(i, o, a)
    }
    fn unused(&self) -> usize {
        self.unused_input().len()
    }
}
impl Stream for XzStream {
    fn process(&mut self, i: &[u8], o: &mut [u8], a: Action) -> io::Result<StreamResult> {
        self.process(i, o, a)
    }
}
fn drive(
    stream: &mut impl Stream,
    codec: CodecId,
    data: &[u8],
    out: &mut InspectionBytes,
    budget: &InspectionBudget,
) -> Result<usize> {
    let mut at = 0usize;
    let mut chunk = [0u8; 16 * 1024];
    loop {
        budget.step()?;
        budget.checkpoint()?;
        let end = data.len().min(at.saturating_add(16 * 1024));
        let action = if end == data.len() {
            Action::Finish
        } else {
            Action::Run
        };
        let request = chunk
            .len()
            .min((budget.limits.bytes - out.len()).saturating_add(1));
        let result = stream
            .process(&data[at..end], &mut chunk[..request], action)
            .map_err(|e| {
                failure(
                    codec,
                    if e.kind() == io::ErrorKind::UnexpectedEof {
                        DecodeErrorKind::Truncated
                    } else if e.kind() == io::ErrorKind::Unsupported
                        || e.to_string()
                            == "multiple non-LZMA2 filters not supported yet for stream API"
                    {
                        DecodeErrorKind::UnsupportedMethod
                    } else if e.kind() == io::ErrorKind::OutOfMemory {
                        return Error::Limit("codec dictionary");
                    } else {
                        DecodeErrorKind::Corrupt
                    },
                    at,
                    e.to_string(),
                )
            })?;
        if result.bytes_consumed > end - at || result.bytes_produced > request {
            return Err(failure(
                codec,
                DecodeErrorKind::Corrupt,
                at,
                "backend over-report",
            ));
        }
        budget.bytes(result.bytes_consumed)?;
        at += result.bytes_consumed;
        append(out, &chunk[..result.bytes_produced], budget)?;
        if result.status == Status::StreamEnd {
            return at.checked_sub(stream.unused()).ok_or_else(|| {
                failure(
                    codec,
                    DecodeErrorKind::Corrupt,
                    at,
                    "backend unused-input range",
                )
            });
        }
        if result.bytes_consumed == 0 && result.bytes_produced == 0 {
            return Err(failure(
                codec,
                if at == data.len() {
                    DecodeErrorKind::Truncated
                } else {
                    DecodeErrorKind::NoProgress
                },
                at,
                "incomplete stream",
            ));
        }
    }
}
pub(super) fn one(
    codec: CodecId,
    data: &[u8],
    out: &mut InspectionBytes,
    budget: &InspectionBudget,
) -> Result<(usize, ChecksumState)> {
    let window = budget.limits.codec_window;
    let data = if codec == CodecId::Xz {
        &data[..super::xz_frame::preflight(data, budget)?]
    } else {
        data
    };
    if codec == CodecId::Lzma {
        let header = field(data, 0, 13, codec)?;
        let dict = u32::from_le_bytes(header[1..5].try_into().expect("width")) as usize;
        if dict > window {
            return Err(Error::Limit("codec dictionary"));
        }
        let size = u64::from_le_bytes(header[5..13].try_into().expect("width"));
        if size != u64::MAX && size > budget.limits.bytes as u64 {
            return Err(Error::Limit("decompression output"));
        }
    }
    if codec == CodecId::Lzip {
        let header = field(data, 0, 6, codec)?;
        let exponent = header[5] & 31;
        if &header[..4] != b"LZIP" || header[4] != 1 || !(12..=29).contains(&exponent) {
            return Err(failure(
                codec,
                DecodeErrorKind::InvalidHeader,
                0,
                "LZIP version/dictionary",
            ));
        }
        let size = (1usize << exponent) - ((1usize << exponent) / 16 * (header[5] as usize >> 5));
        if size > window {
            return Err(Error::Limit("codec dictionary"));
        }
    }
    let _memory = budget.reserve(window.saturating_add(8 * 1024 * 1024))?;
    // The backend cap includes decoder overhead, conservatively stricter than
    // the dictionary ceiling. No encoder/unchecked optimization is enabled.
    let kib = (window / 1024).min(u32::MAX as usize) as u32;
    let checksum =
        if codec == CodecId::Lzma || (codec == CodecId::Xz && field(data, 6, 2, codec)?[1] == 0) {
            ChecksumState::Absent
        } else {
            ChecksumState::Verified
        };
    let start = out.len();
    let result = match codec {
        CodecId::Lzma => drive(
            &mut LzmaStream::new_mem_limit(kib, None),
            codec,
            data,
            out,
            budget,
        ),
        CodecId::Lzip => drive(
            &mut LzipStream::new_mem_limit(false, kib),
            codec,
            data,
            out,
            budget,
        ),
        CodecId::Xz => drive(
            &mut XzStream::new_mem_limit(false, kib),
            codec,
            data,
            out,
            budget,
        ),
        _ => unreachable!(),
    };
    match result {
        Err(Error::Decode(ref e))
            if codec == CodecId::Xz && e.kind == DecodeErrorKind::UnsupportedMethod =>
        {
            // The push API permits one prefilter. The bounded reader supports
            // valid chains of BCJ/Delta filters; retry only this explicit case.
            out.truncate(start);
            budget.step()?;
            let mut source = Source::new(data, budget);
            let result = {
                let mut decoder = lzma_rust2::XzReader::new_mem_limit(&mut source, false, kib);
                read_output(&mut decoder, out, budget)
            };
            if let Err(e) = result {
                return Err(io_failure(codec, &mut source, e));
            }
            Ok((source.pos, checksum))
        }
        Ok(n) => Ok((n, checksum)),
        Err(e) => Err(e),
    }
}
