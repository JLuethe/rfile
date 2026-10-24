// rfile contributions: Copyright (C) 2026 rfile contributors.
// SPDX-License-Identifier: GPL-3.0-only
use std::fmt;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Decode(crate::codecs::DecodeError),
    Database(String),
    Parse {
        line: usize,
        message: String,
    },
    Limit(&'static str),
    Unsupported(String),
    ResourceLimit(LimitViolation),
    LegacyLimit(Vec<u8>),
    LegacyOutput {
        attempted: usize,
        previous: usize,
    },
    Cancelled,
    DeadlineExceeded,
    Malformed {
        format: &'static str,
        stage: &'static str,
        offset: u64,
    },
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => e.fmt(f),
            Self::Decode(e) => e.fmt(f),
            Self::Database(s) => write!(f, "invalid magic database: {s}"),
            Self::Parse { line, message } => write!(f, "magic line {line}: {message}"),
            Self::Limit(s) => write!(f, "{s} limit exceeded"),
            Self::Unsupported(s) => write!(f, "unsupported: {s}"),
            Self::ResourceLimit(v) => write!(f, "{} limit exceeded", v.resource),
            Self::LegacyLimit(bytes) => f.write_str(&String::from_utf8_lossy(bytes)),
            Self::LegacyOutput {
                attempted,
                previous,
            } => write!(f, "Output buffer space exceeded {attempted}+{previous}"),
            Self::Cancelled => f.write_str("inspection cancelled"),
            Self::DeadlineExceeded => f.write_str("inspection deadline exceeded"),
            Self::Malformed {
                format,
                stage,
                offset,
            } => write!(f, "malformed {format} {stage} at offset {offset}"),
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LimitViolation {
    pub resource: &'static str,
    pub limit: usize,
    pub observed: usize,
}
impl Error {
    pub fn raw_diagnostic(&self) -> Option<&[u8]> {
        match self {
            Self::LegacyLimit(bytes) => Some(bytes),
            _ => None,
        }
    }
    pub fn is_limit(&self) -> bool {
        matches!(
            self,
            Self::Limit(_)
                | Self::ResourceLimit(_)
                | Self::LegacyOutput { .. }
                | Self::LegacyLimit(_)
        )
    }
    pub fn limit_detail(&self) -> Option<LimitViolation> {
        if let Self::ResourceLimit(v) = self {
            Some(*v)
        } else {
            None
        }
    }
}
