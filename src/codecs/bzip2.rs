// SPDX-License-Identifier: GPL-3.0-only
//! Independent checked bzip2 decoder. BWT storage is bounded by BZh's block
//! digit; RLE expansion emits bounded chunks and never materializes a full bomb.
use super::*;
struct Bits<'a> {
    data: &'a [u8],
    bit: usize,
    budget: &'a InspectionBudget,
}
impl Bits<'_> {
    fn take(&mut self, n: usize) -> Result<u32> {
        self.budget.bytes(n.div_ceil(8))?;
        if n > 32
            || self
                .bit
                .checked_add(n)
                .is_none_or(|end| end > self.data.len().saturating_mul(8))
        {
            return Err(failure(
                CodecId::Bzip2,
                DecodeErrorKind::Truncated,
                self.bit / 8,
                "bit stream",
            ));
        }
        let mut value = 0;
        for _ in 0..n {
            value = (value << 1) | u32::from((self.data[self.bit / 8] >> (7 - self.bit % 8)) & 1);
            self.bit += 1;
        }
        Ok(value)
    }
}
struct Huffman {
    first: [u32; 21],
    index: [usize; 21],
    count: [usize; 21],
    symbols: Vec<usize>,
}
impl Huffman {
    fn new(lengths: &[usize]) -> Result<Self> {
        let mut count = [0usize; 21];
        for &length in lengths {
            if !(1..=20).contains(&length) {
                return Err(failure(
                    CodecId::Bzip2,
                    DecodeErrorKind::Corrupt,
                    0,
                    "Huffman length",
                ));
            }
            count[length] += 1;
        }
        let mut first = [0u32; 21];
        let mut index = [0usize; 21];
        let mut code = 0u32;
        let mut total = 0;
        let mut space = 1isize;
        for length in 1..=20 {
            space = space * 2 - count[length] as isize;
            if space < 0 {
                return Err(failure(
                    CodecId::Bzip2,
                    DecodeErrorKind::Corrupt,
                    0,
                    "oversubscribed Huffman tree",
                ));
            }
            code = (code + count[length - 1] as u32) << 1;
            first[length] = code;
            index[length] = total;
            total += count[length];
        }
        let mut symbols = Vec::with_capacity(lengths.len());
        for length in 1..=20 {
            for (symbol, &n) in lengths.iter().enumerate() {
                if n == length {
                    symbols.push(symbol);
                }
            }
        }
        Ok(Self {
            first,
            index,
            count,
            symbols,
        })
    }
    fn symbol(&self, bits: &mut Bits<'_>) -> Result<usize> {
        let mut code = 0u32;
        for length in 1..=20 {
            code = (code << 1) | bits.take(1)?;
            if code >= self.first[length] {
                let delta = (code - self.first[length]) as usize;
                if delta < self.count[length] {
                    return Ok(self.symbols[self.index[length] + delta]);
                }
            }
        }
        Err(failure(
            CodecId::Bzip2,
            DecodeErrorKind::Corrupt,
            bits.bit / 8,
            "invalid Huffman code",
        ))
    }
}
struct Symbols<'a> {
    tables: &'a [Huffman],
    selectors: &'a [usize],
    used: usize,
}
impl Symbols<'_> {
    fn next(&mut self, bits: &mut Bits<'_>) -> Result<usize> {
        let group = self.used / 50;
        self.used += 1;
        let &table = self.selectors.get(group).ok_or_else(|| {
            failure(
                CodecId::Bzip2,
                DecodeErrorKind::Corrupt,
                bits.bit / 8,
                "exhausted selectors",
            )
        })?;
        self.tables[table].symbol(bits)
    }
}
const fn crc_table() -> [u32; 256] {
    let mut out = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut n = (i as u32) << 24;
        let mut bit = 0;
        while bit < 8 {
            n = if n & 0x80000000 != 0 {
                (n << 1) ^ 0x04c11db7
            } else {
                n << 1
            };
            bit += 1;
        }
        out[i] = n;
        i += 1;
    }
    out
}
const CRC: [u32; 256] = crc_table();
fn emit(
    byte: u8,
    count: usize,
    chunk: &mut Vec<u8>,
    crc: &mut u32,
    out: &mut InspectionBytes,
    budget: &InspectionBudget,
) -> Result<()> {
    if out.len().saturating_add(chunk.len()).saturating_add(count) > budget.limits.bytes {
        return Err(Error::Limit("decompression output"));
    }
    for _ in 0..count {
        *crc = (*crc << 8) ^ CRC[(((*crc >> 24) as u8) ^ byte) as usize];
        chunk.push(byte);
        if chunk.len() == 16 * 1024 {
            append(out, chunk, budget)?;
            chunk.clear();
        }
    }
    Ok(())
}
pub(super) fn one(
    data: &[u8],
    out: &mut InspectionBytes,
    budget: &InspectionBudget,
) -> Result<(usize, ChecksumState)> {
    let codec = CodecId::Bzip2;
    let header = field(data, 0, 4, codec)?;
    if &header[..3] != b"BZh" || !(b'1'..=b'9').contains(&header[3]) {
        return Err(failure(
            codec,
            DecodeErrorKind::InvalidHeader,
            0,
            "bzip2 header",
        ));
    }
    let block_limit = (header[3] - b'0') as usize * 100_000;
    if block_limit.saturating_mul(5) > budget.limits.codec_window {
        return Err(Error::Limit("codec dictionary"));
    }
    let _memory = budget.reserve(block_limit.saturating_mul(10).saturating_add(256 * 1024))?;
    let mut bits = Bits {
        data,
        bit: 32,
        budget,
    };
    let mut combined = 0u32;
    loop {
        budget.step()?;
        let marker = (u64::from(bits.take(24)?) << 24) | u64::from(bits.take(24)?);
        if marker == 0x177245385090 {
            let expected = bits.take(32)?;
            if combined != expected {
                return Err(failure(
                    codec,
                    DecodeErrorKind::Checksum,
                    bits.bit / 8,
                    "combined CRC",
                ));
            }
            return Ok((bits.bit.div_ceil(8), ChecksumState::Verified));
        }
        if marker != 0x314159265359 {
            return Err(failure(
                codec,
                DecodeErrorKind::Corrupt,
                bits.bit / 8,
                "block marker",
            ));
        }
        let expected = bits.take(32)?;
        if bits.take(1)? != 0 {
            return Err(failure(
                codec,
                DecodeErrorKind::UnsupportedMethod,
                bits.bit / 8,
                "legacy randomized block",
            ));
        }
        let original = bits.take(24)? as usize;
        let mut groups = [false; 16];
        for group in &mut groups {
            *group = bits.take(1)? != 0;
        }
        let mut alphabet = Vec::with_capacity(256);
        for (group, &used) in groups.iter().enumerate() {
            if used {
                for i in 0..16 {
                    if bits.take(1)? != 0 {
                        alphabet.push((group * 16 + i) as u8);
                    }
                }
            }
        }
        if alphabet.is_empty() {
            return Err(failure(
                codec,
                DecodeErrorKind::Corrupt,
                bits.bit / 8,
                "empty alphabet",
            ));
        }
        let n = alphabet.len();
        let table_count = bits.take(3)? as usize;
        let selector_count = bits.take(15)? as usize;
        if !(2..=6).contains(&table_count) || selector_count == 0 || selector_count > 18002 {
            return Err(failure(
                codec,
                DecodeErrorKind::Corrupt,
                bits.bit / 8,
                "table/selector count",
            ));
        }
        let mut table_mtf: Vec<_> = (0..table_count).collect();
        let mut selectors = Vec::with_capacity(selector_count);
        for _ in 0..selector_count {
            let mut i = 0;
            while bits.take(1)? != 0 {
                i += 1;
                if i >= table_count {
                    return Err(failure(
                        codec,
                        DecodeErrorKind::Corrupt,
                        bits.bit / 8,
                        "selector MTF",
                    ));
                }
            }
            let selected = table_mtf.remove(i);
            table_mtf.insert(0, selected);
            selectors.push(selected);
        }
        let mut tables = Vec::with_capacity(table_count);
        for _ in 0..table_count {
            let mut current = bits.take(5)? as i32;
            let mut lengths = Vec::with_capacity(n + 2);
            for _ in 0..n + 2 {
                while bits.take(1)? != 0 {
                    current += if bits.take(1)? == 0 { 1 } else { -1 };
                    if !(1..=20).contains(&current) {
                        return Err(failure(
                            codec,
                            DecodeErrorKind::Corrupt,
                            bits.bit / 8,
                            "Huffman delta",
                        ));
                    }
                }
                if !(1..=20).contains(&current) {
                    return Err(failure(
                        codec,
                        DecodeErrorKind::Corrupt,
                        bits.bit / 8,
                        "Huffman length",
                    ));
                }
                lengths.push(current as usize);
            }
            tables.push(Huffman::new(&lengths)?);
        }
        let mut symbols = Symbols {
            tables: &tables,
            selectors: &selectors,
            used: 0,
        };
        let mut mtf = alphabet;
        let mut last = Vec::with_capacity(block_limit);
        let mut symbol = symbols.next(&mut bits)?;
        loop {
            if symbol == n + 1 {
                break;
            }
            if symbol <= 1 {
                let mut run = 0usize;
                let mut power = 1usize;
                while symbol <= 1 {
                    let add = power.checked_mul(symbol + 1).ok_or_else(|| {
                        failure(
                            codec,
                            DecodeErrorKind::Corrupt,
                            bits.bit / 8,
                            "RLE2 overflow",
                        )
                    })?;
                    run = run
                        .checked_add(add)
                        .filter(|&v| v <= block_limit - last.len())
                        .ok_or_else(|| {
                            failure(
                                codec,
                                DecodeErrorKind::Corrupt,
                                bits.bit / 8,
                                "RLE2 block bound",
                            )
                        })?;
                    power = power.checked_mul(2).ok_or_else(|| {
                        failure(
                            codec,
                            DecodeErrorKind::Corrupt,
                            bits.bit / 8,
                            "RLE2 overflow",
                        )
                    })?;
                    symbol = symbols.next(&mut bits)?;
                }
                budget.bytes(run)?;
                last.resize(last.len() + run, mtf[0]);
                continue;
            }
            if last.len() >= block_limit || symbol > mtf.len() {
                return Err(failure(
                    codec,
                    DecodeErrorKind::Corrupt,
                    bits.bit / 8,
                    "MTF/block bound",
                ));
            }
            let value = mtf.remove(symbol - 1);
            mtf.insert(0, value);
            last.push(value);
            symbol = symbols.next(&mut bits)?;
        }
        if original >= last.len() {
            return Err(failure(
                codec,
                DecodeErrorKind::Corrupt,
                bits.bit / 8,
                "BWT origin",
            ));
        }
        let mut counts = [0usize; 256];
        for &byte in &last {
            counts[byte as usize] += 1;
        }
        let mut sum = 0;
        for count in &mut counts {
            let n = *count;
            *count = sum;
            sum += n;
        }
        let mut next = vec![0u32; last.len()];
        for (i, &byte) in last.iter().enumerate() {
            next[counts[byte as usize]] = i as u32;
            counts[byte as usize] += 1;
        }
        let mut at = next[original] as usize;
        let mut previous = 0u8;
        let mut repeated = 0;
        let mut chunk = Vec::with_capacity(16 * 1024);
        let mut crc = !0u32;
        for i in 0..last.len() {
            if i % 4096 == 0 {
                budget.bytes(4096)?;
            }
            let byte = last[at];
            at = next[at] as usize;
            if repeated == 4 {
                emit(previous, byte as usize, &mut chunk, &mut crc, out, budget)?;
                repeated = 0;
            } else {
                emit(byte, 1, &mut chunk, &mut crc, out, budget)?;
                if repeated > 0 && byte == previous {
                    repeated += 1;
                } else {
                    previous = byte;
                    repeated = 1;
                }
            }
        }
        if repeated == 4 {
            return Err(failure(
                codec,
                DecodeErrorKind::Truncated,
                bits.bit / 8,
                "RLE1 count",
            ));
        }
        append(out, &chunk, budget)?;
        let actual = !crc;
        if actual != expected {
            return Err(failure(
                codec,
                DecodeErrorKind::Checksum,
                bits.bit / 8,
                "block CRC",
            ));
        }
        combined = combined.rotate_left(1) ^ actual;
    }
}
