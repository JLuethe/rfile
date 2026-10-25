// SPDX-License-Identifier: GPL-3.0-only
//! Shared safe header diagnostic ordering, followed by core validation.
use rfile::{Database, Result};
pub fn load(bytes: &[u8], name: &[u8], file: bool) -> Result<Database> {
    Database::from_compiled_named(bytes, name, file)
}
