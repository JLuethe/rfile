// SPDX-License-Identifier: GPL-3.0-only
//! Native header diagnostic order, followed by the core's stronger validation.
use crate::{Database, Error, Result};
pub(crate) fn load(bytes: &[u8], name: &[u8], file: bool) -> Result<Database> {
    let fail = |prefix: &[u8], suffix: &[u8]| {
        let mut b = prefix.to_vec();
        b.extend_from_slice(name);
        b.extend_from_slice(suffix);
        Error::LegacyFormat(b)
    };
    if file && bytes.len() < 8 {
        return Err(fail(b"file `", b"' is too small"));
    }
    let entries = bytes.len() / 432;
    if entries < 2 {
        return Err(fail(
            format!("Too few magic entries {entries} in `").as_bytes(),
            b"'",
        ));
    }
    if !bytes.len().is_multiple_of(432) {
        return Err(fail(
            b"Size of `",
            format!("' {} is not a multiple of 432", bytes.len()).as_bytes(),
        ));
    }
    let little = match &bytes[..4] {
        [0x1c, 0x04, 0x1e, 0xf1] => true,
        [0xf1, 0x1e, 0x04, 0x1c] => false,
        _ => return Err(fail(b"bad magic in `", b"'")),
    };
    let number = |offset: usize| {
        let b = bytes[offset..offset + 4]
            .try_into()
            .expect("header length checked");
        if little {
            u32::from_le_bytes(b)
        } else {
            u32::from_be_bytes(b)
        }
    };
    let version = number(4);
    if version != 21 {
        return Err(fail(
            b"File 5.48 supports only version 21 magic files. `",
            format!("' is version {}", version as i32).as_bytes(),
        ));
    }
    let mut n = 0u64;
    for i in 0..2 {
        let count = number(8 + i * 4);
        if count as usize > entries {
            return Err(fail(format!("nmagic[{i}] too large in `").as_bytes(), b"'"));
        }
        n += u64::from(count);
    }
    if entries as u64 != n + 1 {
        return Err(fail(
            b"Inconsistent entries in `",
            format!("' {entries} != {}", n + 1).as_bytes(),
        ));
    }
    Database::from_compiled(bytes)
}
