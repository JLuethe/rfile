/*
 * Copyright (c) Ian F. Darwin 1986-1995.
 * Software written by Ian F. Darwin and others;
 * maintained 1995-present by Christos Zoulas and others.
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
use crate::{Error, Result};

#[derive(Clone, Debug)]
pub(crate) enum Value {
    Number { n: u64, signed: bool, width: usize },
    Float(f64),
    Bytes(Vec<u8>),
    Guid([u8; 16], bool),
    None,
}
impl Value {
    fn as_bytes(&self) -> Vec<u8> {
        match self {
            Self::Number { n, signed, width } => {
                if *signed {
                    crate::database::signed(*n, *width).to_string().into_bytes()
                } else {
                    n.to_string().into_bytes()
                }
            }
            Self::Float(f) => general(*f, 6, false, false).into_bytes(),
            Self::Bytes(b) => b.clone(),
            Self::Guid(b, big) => guid(b, *big).into_bytes(),
            Self::None => Vec::new(),
        }
    }
}
pub(crate) fn escape(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    for &c in bytes {
        if (32..=126).contains(&c) {
            out.push(c);
        } else {
            out.extend_from_slice(format!("\\{c:03o}").as_bytes());
        }
    }
    out
}
pub(crate) fn printable(bytes: &[u8], raw: bool) -> Vec<u8> {
    let mut out = Vec::new();
    for &c in crate::database::cstr(bytes) {
        if raw || (32..=126).contains(&c) {
            if out.len() == 511 {
                break;
            }
            out.push(c);
        } else {
            if out.len() > 507 {
                break;
            }
            out.extend_from_slice(format!("\\{c:03o}").as_bytes());
        }
    }
    out
}

pub(crate) fn expand(bytes: &[u8], executable: bool) -> Vec<u8> {
    let mut out = Vec::new();
    let mut p = 0;
    while p < bytes.len() {
        if bytes[p..].starts_with(b"${x?") {
            if let Some(colon) = bytes[p + 4..]
                .iter()
                .position(|&b| b == b':')
                .map(|x| p + 4 + x)
            {
                if let Some(end) = bytes[colon + 1..]
                    .iter()
                    .position(|&b| b == b'}')
                    .map(|x| colon + 1 + x)
                {
                    out.extend_from_slice(if executable {
                        &bytes[p + 4..colon]
                    } else {
                        &bytes[colon + 1..end]
                    });
                    p = end + 1;
                    continue;
                }
            }
            return bytes.to_vec();
        }
        out.push(bytes[p]);
        p += 1;
    }
    out
}
fn general(x: f64, precision: usize, upper: bool, alternate: bool) -> String {
    let p = precision.clamp(1, 64);
    if !x.is_finite() {
        let s = if x.is_nan() {
            if x.is_sign_negative() { "-nan" } else { "nan" }
        } else if x.is_sign_negative() {
            "-inf"
        } else {
            "inf"
        };
        return if upper { s.to_uppercase() } else { s.into() };
    }
    let mut exp = if x == 0.0 {
        0
    } else {
        x.abs().log10().floor() as i32
    };
    // Round before selecting %g's exponent representation.
    let scientific = format!("{:.prec$e}", x, prec = p - 1);
    if let Some((_, e)) = scientific.split_once('e') {
        exp = e.parse().unwrap_or(exp);
    }
    let mut s = if exp < -4 || exp >= p as i32 {
        let (mantissa, exponent) = scientific.split_once('e').unwrap_or((&scientific, "0"));
        let mut mantissa = mantissa.to_owned();
        if !alternate && mantissa.contains('.') {
            while mantissa.ends_with('0') {
                mantissa.pop();
            }
            if mantissa.ends_with('.') {
                mantissa.pop();
            }
        }
        let e: i32 = exponent.parse().unwrap_or(0);
        format!("{mantissa}e{e:+03}")
    } else {
        let decimals = (p as i32 - exp - 1).max(0) as usize;
        let mut s = format!("{x:.decimals$}");
        if !alternate && s.contains('.') {
            while s.ends_with('0') {
                s.pop();
            }
            if s.ends_with('.') {
                s.pop();
            }
        }
        s
    };
    if upper {
        s.make_ascii_uppercase();
    }
    s
}
pub(crate) fn render(description: &[u8], value: &Value, executable: bool) -> Result<Vec<u8>> {
    let desc = expand(description, executable);
    let mut out = Vec::new();
    let mut p = 0;
    let mut conversions = 0;
    while p < desc.len() {
        if desc[p] != b'%' {
            out.push(desc[p]);
            p += 1;
            continue;
        }
        p += 1;
        if desc.get(p) == Some(&b'%') {
            out.push(b'%');
            p += 1;
            continue;
        }
        conversions += 1;
        if conversions > 1 {
            return Err(Error::Unsupported(
                "multiple format conversions in one description".into(),
            ));
        }
        let mut left = false;
        let mut zero = false;
        let mut plus = false;
        let mut blank = false;
        let mut alternate = false;
        while let Some(&c) = desc.get(p) {
            match c {
                b'-' => left = true,
                b'0' => zero = true,
                b'+' => plus = true,
                b' ' => blank = true,
                b'#' => alternate = true,
                _ => break,
            }
            p += 1;
        }
        let mut width = 0usize;
        while let Some(&c @ b'0'..=b'9') = desc.get(p) {
            width = width
                .checked_mul(10)
                .and_then(|x| x.checked_add((c - b'0') as usize))
                .ok_or(Error::Limit("format width"))?;
            if width > 4096 {
                return Err(Error::Limit("format width"));
            }
            p += 1;
        }
        let precision = if desc.get(p) == Some(&b'.') {
            p += 1;
            let mut n = 0usize;
            while let Some(&c @ b'0'..=b'9') = desc.get(p) {
                n = n
                    .checked_mul(10)
                    .and_then(|x| x.checked_add((c - b'0') as usize))
                    .ok_or(Error::Limit("format precision"))?;
                if n > 4096 {
                    return Err(Error::Limit("format precision"));
                }
                p += 1;
            }
            Some(n)
        } else {
            None
        };
        let length_start = p;
        while desc.get(p).is_some_and(|c| b"hljztL".contains(c)) {
            p += 1;
        }
        let long = &desc[length_start..p];
        let conversion = *desc
            .get(p)
            .ok_or_else(|| Error::Unsupported("incomplete format specifier".into()))?;
        p += 1;
        let mut b = match conversion {
            b's' => {
                let mut v = value.as_bytes();
                if let Some(n) = precision {
                    v.truncate(n);
                }
                v
            }
            b'c' => {
                let n = if let Value::Number { n, .. } = value {
                    *n
                } else {
                    0
                };
                vec![n as u8]
            }
            b'd' | b'i' | b'u' | b'o' | b'x' | b'X' => {
                let (n, signed_value, nwidth) = if let Value::Number { n, signed, width } = value {
                    (*n, *signed, *width)
                } else {
                    return Err(Error::Unsupported(
                        "numeric format for nonnumeric value".into(),
                    ));
                };
                let wide = long == b"ll"
                    || long == b"j"
                    || long == b"z"
                    || long == b"t"
                    || long == b"l" && std::mem::size_of::<usize>() == 8;
                let bits = if wide { 64 } else { 32 };
                let promoted = if signed_value {
                    crate::database::signed(n, nwidth) as u64
                } else {
                    n
                };
                let n = if bits == 32 {
                    promoted as u32 as u64
                } else {
                    promoted
                };
                let is_signed = matches!(conversion, b'd' | b'i');
                let signed_n = if bits == 32 {
                    n as i32 as i64
                } else {
                    n as i64
                };
                let negative = is_signed && signed_n < 0;
                let absolute = if negative { signed_n.unsigned_abs() } else { n };
                let mut digits = match conversion {
                    b'x' => format!("{absolute:x}"),
                    b'X' => format!("{absolute:X}"),
                    b'o' => format!("{absolute:o}"),
                    _ => absolute.to_string(),
                };
                if precision == Some(0) && absolute == 0 {
                    digits.clear();
                }
                if let Some(n) = precision {
                    if n > digits.len() {
                        digits = format!("{}{}", "0".repeat(n - digits.len()), digits);
                    }
                }
                let prefix = if negative {
                    "-"
                } else if is_signed && plus {
                    "+"
                } else if is_signed && blank {
                    " "
                } else if alternate && conversion == b'x' && absolute != 0 {
                    "0x"
                } else if alternate && conversion == b'X' && absolute != 0 {
                    "0X"
                } else if alternate && conversion == b'o' && !digits.starts_with('0') {
                    "0"
                } else {
                    ""
                };
                let mut text = format!("{prefix}{digits}");
                if zero && !left && precision.is_none() && width > text.len() {
                    text = format!(
                        "{prefix}{}{digits}",
                        "0".repeat(width - prefix.len() - digits.len())
                    );
                }
                text.into_bytes()
            }
            b'e' | b'E' | b'f' | b'F' | b'g' | b'G' => {
                let Value::Float(x) = value else {
                    return Err(Error::Unsupported("float format for non-float".into()));
                };
                let prec = precision.unwrap_or(6).min(64);
                let mut s = if !x.is_finite() {
                    general(*x, prec, conversion.is_ascii_uppercase(), alternate)
                } else {
                    match conversion {
                        b'g' | b'G' => general(*x, prec, conversion == b'G', alternate),
                        b'e' | b'E' => {
                            let s = format!("{x:.prec$e}");
                            if let Some((m, e)) = s.split_once('e') {
                                let e: i32 = e.parse().unwrap_or(0);
                                format!("{m}e{e:+03}")
                            } else {
                                s
                            }
                        }
                        _ => format!("{x:.prec$}"),
                    }
                };
                if !s.starts_with('-') {
                    if plus {
                        s.insert(0, '+');
                    } else if blank {
                        s.insert(0, ' ');
                    }
                }
                if conversion.is_ascii_uppercase() {
                    s.make_ascii_uppercase();
                }
                s.into_bytes()
            }
            _ => {
                return Err(Error::Unsupported(format!(
                    "format conversion %{}",
                    conversion as char
                )));
            }
        };
        if width > b.len() {
            let pad = vec![
                if zero && !left && conversion != b's' {
                    b'0'
                } else {
                    b' '
                };
                width - b.len()
            ];
            if left {
                b.extend(pad);
            } else {
                let mut padded = pad;
                padded.extend(b);
                b = padded;
            }
        }
        out.extend(b);
        if out.len() > 1024 * 1024 {
            return Err(Error::Limit("formatted output"));
        }
    }
    Ok(out)
}
pub(crate) fn parse_guid(s: &[u8]) -> Option<[u8; 16]> {
    if !s.is_ascii()
        || s.len() != 36
        || s[8] != b'-'
        || s[13] != b'-'
        || s[18] != b'-'
        || s[23] != b'-'
    {
        return None;
    }
    let text = std::str::from_utf8(s).ok()?;
    let mut b = [0; 16];
    b[..4].copy_from_slice(&u32::from_str_radix(&text[..8], 16).ok()?.to_le_bytes());
    b[4..6].copy_from_slice(&u16::from_str_radix(&text[9..13], 16).ok()?.to_le_bytes());
    b[6..8].copy_from_slice(&u16::from_str_radix(&text[14..18], 16).ok()?.to_le_bytes());
    let tail = format!("{}{}", &text[19..23], &text[24..36]);
    for i in 0..8 {
        b[8 + i] = u8::from_str_radix(&tail[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(b)
}
pub(crate) fn guid(b: &[u8; 16], big: bool) -> String {
    let a: [u8; 4] = b[..4].try_into().expect("GUID width");
    let c: [u8; 2] = b[4..6].try_into().expect("GUID width");
    let d: [u8; 2] = b[6..8].try_into().expect("GUID width");
    let (a, c, d) = if big {
        (
            u32::from_be_bytes(a),
            u16::from_be_bytes(c),
            u16::from_be_bytes(d),
        )
    } else {
        (
            u32::from_le_bytes(a),
            u16::from_le_bytes(c),
            u16::from_le_bytes(d),
        )
    };
    format!(
        "{a:08X}-{c:04X}-{d:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
        b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15]
    )
}
const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
// Preserve cdf_timestamp_to_timespec's calendar normalization, including its
// month-boundary and pre-1601 quirks. Reject its failure path deterministically:
// C's caller ignores that error and may read an uninitialized tv_sec.
fn windows_seconds(ticks: i64) -> Option<i64> {
    fn leap(y: i64) -> bool {
        y % 4 == 0 && (y % 100 != 0 || y % 400 == 0)
    }
    fn civil_days(mut y: i64, m: i64, d: i64) -> i64 {
        y -= i64::from(m <= 2);
        let era = y.div_euclid(400);
        let yoe = y - era * 400;
        let mp = m + if m > 2 { -3 } else { 9 };
        let doy = (153 * mp + 2) / 5 + d - 1;
        era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468
    }
    let seconds = ticks / 10_000_000;
    let days = seconds / 86400;
    let year = 1601 + days / 365;
    if year > 9999 {
        return None;
    }
    let elapsed = if year > 1601 {
        civil_days(year, 1, 1) - civil_days(1601, 1, 1)
    } else {
        0
    };
    let rdays = days - elapsed + 1;
    let lengths = [
        31,
        28 + i64::from(leap(year)),
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut day = rdays;
    for count in lengths {
        if day < count {
            break;
        }
        day -= count;
    }
    let mut residual = rdays;
    let mut month = 0usize;
    for count in lengths {
        residual -= count;
        if residual <= 0 {
            break;
        }
        month += 1;
    }
    let before: i64 = lengths[..month].iter().sum();
    Some((civil_days(year, 1, 1) + before + day - 1) * 86400 + seconds % 86400)
}
pub(crate) fn datetime(
    n: u64,
    windows: bool,
    local: bool,
    time: &crate::TimeContext,
    profile: crate::SemanticProfile,
) -> String {
    let mut seconds = if windows && profile == crate::SemanticProfile::CorrectedV1 {
        (n as i64)
            .div_euclid(10_000_000)
            .saturating_sub(11_644_473_600)
    } else if windows {
        let Some(seconds) = windows_seconds(n as i64) else {
            return "*Invalid datetime*".into();
        };
        let Some(seconds) = time.legacy_windows_seconds(seconds) else {
            return "*Invalid datetime*".into();
        };
        seconds
    } else {
        n as i64
    };
    if seconds > 0x3a_fff4_87cf {
        return "*Invalid datetime*".into();
    }
    if local {
        let Some(adjusted) = time.local_seconds(seconds) else {
            return "*Invalid datetime*".into();
        };
        seconds = adjusted;
    }
    let days = seconds.div_euclid(86400);
    let sec = seconds.rem_euclid(86400);
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    if !(-999..=9999).contains(&year) {
        return "*Invalid datetime*".into();
    }
    let weekday =
        ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"][(days + 4).rem_euclid(7) as usize];
    format!(
        "{weekday} {} {day:2} {:02}:{:02}:{:02} {year:04}",
        MONTHS[(month - 1) as usize],
        sec / 3600,
        (sec / 60) % 60,
        sec % 60
    )
}
pub(crate) fn dos_date(n: u16) -> String {
    let m = ((n >> 5) & 15) as usize;
    let month = if (1..=12).contains(&m) { m } else { 1 };
    format!("{} {:02} {}", MONTHS[month - 1], n & 31, (n >> 9) + 1980)
}
pub(crate) fn dos_time(n: u16) -> String {
    format!("{:02}:{:02}:{:02}", n >> 11, (n >> 5) & 63, (n & 31) * 2)
}

pub(crate) fn validate(description: &[u8], kind: u8) -> Result<()> {
    let mut i = 0;
    let mut conversions = 0;
    while i < description.len() {
        if description[i] != b'%' {
            i += 1;
            continue;
        }
        i += 1;
        if description.get(i) == Some(&b'%') {
            i += 1;
            continue;
        }
        conversions += 1;
        if conversions > 1 {
            return Err(Error::Unsupported("multiple format conversions".into()));
        }
        while description.get(i).is_some_and(|c| b"#0- +".contains(c)) {
            i += 1;
        }
        let start = i;
        while description.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        if i > start
            && std::str::from_utf8(&description[start..i])
                .ok()
                .and_then(|n| n.parse::<usize>().ok())
                .is_none_or(|n| n > 4096)
        {
            return Err(Error::Limit("format width"));
        }
        if description.get(i) == Some(&b'.') {
            i += 1;
            let start = i;
            while description.get(i).is_some_and(u8::is_ascii_digit) {
                i += 1;
            }
            if i > start
                && std::str::from_utf8(&description[start..i])
                    .ok()
                    .and_then(|n| n.parse::<usize>().ok())
                    .is_none_or(|n| n > 4096)
            {
                return Err(Error::Limit("format precision"));
            }
        }
        while description.get(i).is_some_and(|c| b"hljztL".contains(c)) {
            i += 1;
        }
        let c = *description
            .get(i)
            .ok_or_else(|| Error::Unsupported("incomplete format".into()))?;
        i += 1;
        let allowed = if matches!(kind, 33..=38) {
            b"eEfFgGs".as_slice()
        } else if matches!(
            kind,
            1 | 2 | 4 | 7 | 8 | 10 | 11 | 23..=26 | 39 | 40 | 41 | 52
        ) {
            b"cdiuoxXs".as_slice()
        } else if matches!(kind, 3 | 45 | 46 | 47) {
            b"".as_slice()
        } else {
            b"s".as_slice()
        };
        if !allowed.contains(&c) {
            return Err(Error::Unsupported(format!(
                "format %{} for magic type {kind}",
                c as char
            )));
        }
    }
    Ok(())
}
