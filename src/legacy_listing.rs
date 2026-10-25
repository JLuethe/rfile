// SPDX-License-Identifier: GPL-3.0-only
//! Byte-preserving pinned strength-list output. Classification remains separate.
use crate::{
    Database, Error, Result,
    database::{BINTEST, TEXTTEST},
};

pub(crate) fn render(database: &Database, limit: usize) -> Result<Vec<u8>> {
    if limit > 16 * 1024 * 1024 {
        return Err(Error::Limit("listing output configuration"));
    }
    let mut output = Vec::new();
    let mut append = |bytes: &[u8]| -> Result<()> {
        let n = output
            .len()
            .checked_add(bytes.len())
            .filter(|&n| n <= limit)
            .ok_or(Error::Limit("listing output"))?;
        output
            .try_reserve(n - output.len())
            .map_err(|_| Error::Limit("listing allocation"))?;
        output.extend_from_slice(bytes);
        Ok(())
    };
    for (set, groups) in [(0, &database.ordinary), (1, &database.named)] {
        append(format!("Set {set}:\n").as_bytes())?;
        for (mode, heading) in [
            (BINTEST, b"Binary patterns:\n".as_slice()),
            (TEXTTEST, b"Text patterns:\n".as_slice()),
        ] {
            append(heading)?;
            for group in groups {
                let root = &group.rules[0];
                if root.flags & mode != mode {
                    continue;
                }
                let description = group
                    .rules
                    .iter()
                    .map(|r| cstr(&r.description))
                    .find(|s| !s.is_empty())
                    .unwrap_or_default();
                let mime = group
                    .rules
                    .iter()
                    .map(|r| cstr(&r.mime))
                    .find(|s| !s.is_empty())
                    .unwrap_or_default();
                append(
                    format!(
                        "Strength = {:3}@{}: ",
                        crate::parser::strength(root),
                        root.line
                    )
                    .as_bytes(),
                )?;
                append(description)?;
                append(b" [")?;
                append(mime)?;
                append(b"]\n")?;
            }
        }
    }
    Ok(output)
}
fn cstr(bytes: &[u8]) -> &[u8] {
    &bytes[..bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len())]
}
