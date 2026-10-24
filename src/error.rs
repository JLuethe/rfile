// rfile contributions: Copyright (C) 2026 rfile contributors.
// SPDX-License-Identifier: GPL-3.0-only
use std::fmt;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Database(String),
    Parse { line: usize, message: String },
    Limit(&'static str),
    Unsupported(String),
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => e.fmt(f),
            Self::Database(s) => write!(f, "invalid magic database: {s}"),
            Self::Parse { line, message } => write!(f, "magic line {line}: {message}"),
            Self::Limit(s) => write!(f, "{s} limit exceeded"),
            Self::Unsupported(s) => write!(f, "unsupported: {s}"),
        }
    }
}
impl std::error::Error for Error {}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
pub type Result<T> = std::result::Result<T, Error>;
