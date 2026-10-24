/*
 * Copyright (c) Christos Zoulas 2003.
 * All Rights Reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice immediately at the beginning of the file, without modification,
 *    this list of conditions, and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 *
 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED. IN NO EVENT SHALL THE AUTHOR OR CONTRIBUTORS BE LIABLE FOR
 * ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 */
// rfile contributions: Copyright (C) 2026 rfile contributors.
// SPDX-License-Identifier: GPL-3.0-only

// Independent Rust implementation; upstream behavior and retained data notices credited in NOTICE.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Flags(pub u32);
impl Flags {
    pub const DEBUG: u32 = 0x1;
    pub const SYMLINK: u32 = 0x2;
    pub const COMPRESS: u32 = 0x4;
    pub const DEVICES: u32 = 0x8;
    pub const MIME_TYPE: u32 = 0x10;
    pub const CONTINUE: u32 = 0x20;
    pub const CHECK: u32 = 0x40;
    pub const PRESERVE_ATIME: u32 = 0x80;
    pub const RAW: u32 = 0x100;
    pub const ERROR: u32 = 0x200;
    pub const MIME_ENCODING: u32 = 0x400;
    pub const MIME: u32 = Self::MIME_TYPE | Self::MIME_ENCODING;
    pub const APPLE: u32 = 0x800;
    pub const NO_COMPRESS: u32 = 0x1000;
    pub const NO_TAR: u32 = 0x2000;
    pub const NO_SOFT: u32 = 0x4000;
    pub const NO_APPTYPE: u32 = 0x8000;
    pub const NO_ELF: u32 = 0x10000;
    pub const NO_TEXT: u32 = 0x20000;
    pub const NO_CDF: u32 = 0x40000;
    pub const NO_CSV: u32 = 0x80000;
    pub const NO_TOKENS: u32 = 0x100000;
    pub const NO_ENCODING: u32 = 0x200000;
    pub const NO_JSON: u32 = 0x400000;
    pub const NO_SIMH: u32 = 0x800000;
    pub const EXTENSION: u32 = 0x1000000;
    pub const TRANSPARENT: u32 = 0x2000000;
    pub const NO_FORK: u32 = 0x4000000;
    pub fn has(self, bits: u32) -> bool {
        self.0 & bits != 0
    }
    pub fn annotation(self) -> bool {
        self.has(Self::MIME | Self::APPLE | Self::EXTENSION)
    }
    pub fn exclude(name: &str) -> Option<u32> {
        Some(match name {
            "compress" => Self::NO_COMPRESS,
            "tar" => Self::NO_TAR,
            "soft" => Self::NO_SOFT,
            "apptype" => Self::NO_APPTYPE,
            "elf" => Self::NO_ELF,
            "ascii" | "text" => Self::NO_TEXT,
            "cdf" => Self::NO_CDF,
            "csv" => Self::NO_CSV,
            "tokens" => Self::NO_TOKENS,
            "encoding" => Self::NO_ENCODING,
            "json" => Self::NO_JSON,
            "simh" => Self::NO_SIMH,
            _ => return None,
        })
    }
}
