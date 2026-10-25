// SPDX-License-Identifier: GPL-3.0-only
//! Small immutable representation of already Jiff-validated POSIX TZ rules.
//! Pinned GNU behavior evaluates both transitions in the UTC calendar year,
//! and anchors years <= 1970 at epoch zero. Corrected mode keeps Jiff's rules.
use jiff::{civil::Date, tz::TimeZone};

#[derive(Clone, Debug)]
pub(crate) struct PosixTime {
    pub standard: i32,
    daylight: i32,
    rules: Option<(Rule, Rule)>,
}
#[derive(Clone, Debug)]
struct Rule {
    day: Day,
    seconds: i32,
}
#[derive(Clone, Debug)]
enum Day {
    Julian(u16),
    Ordinal(u16),
    Week { month: i8, week: u16, weekday: u16 },
}
struct Cursor<'a>(&'a [u8]);
impl Cursor<'_> {
    fn take(&mut self, c: u8) -> bool {
        if self.0.first() == Some(&c) {
            self.0 = &self.0[1..];
            true
        } else {
            false
        }
    }
    fn name(&mut self) -> Option<()> {
        if self.take(b'<') {
            let end = self.0.iter().position(|&b| b == b'>')?;
            self.0 = &self.0[end + 1..];
        } else {
            let n = self
                .0
                .iter()
                .take_while(|b| b.is_ascii_alphabetic())
                .count();
            if n == 0 {
                return None;
            }
            self.0 = &self.0[n..];
        }
        Some(())
    }
    fn number(&mut self) -> Option<u16> {
        let mut n = 0u16;
        let mut any = false;
        while let Some(&b @ b'0'..=b'9') = self.0.first() {
            n = n.checked_mul(10)?.checked_add((b - b'0') as u16)?;
            self.0 = &self.0[1..];
            any = true;
        }
        any.then_some(n)
    }
    fn clock(&mut self) -> Option<i32> {
        let sign = if self.take(b'-') {
            -1
        } else {
            self.take(b'+');
            1
        };
        let hours = self.number()? as i32;
        let minutes = if self.take(b':') {
            self.number()? as i32
        } else {
            0
        };
        let seconds = if self.take(b':') {
            self.number()? as i32
        } else {
            0
        };
        Some(sign * (hours * 3600 + minutes * 60 + seconds))
    }
    fn rule(&mut self) -> Option<Rule> {
        let day = if self.take(b'J') {
            Day::Julian(self.number()?)
        } else if self.take(b'M') {
            let month = i8::try_from(self.number()?).ok()?;
            if !self.take(b'.') {
                return None;
            }
            let week = self.number()?;
            if !self.take(b'.') {
                return None;
            }
            Day::Week {
                month,
                week,
                weekday: self.number()?,
            }
        } else {
            Day::Ordinal(self.number()?)
        };
        let seconds = if self.take(b'/') { self.clock()? } else { 7200 };
        Some(Rule { day, seconds })
    }
}
impl PosixTime {
    pub fn parse(validated: &str) -> Option<Self> {
        let mut c = Cursor(validated.as_bytes());
        c.name()?;
        let standard = -c.clock()?;
        if c.0.is_empty() {
            return Some(Self {
                standard,
                daylight: standard,
                rules: None,
            });
        }
        c.name()?;
        let daylight = if c.0.first() == Some(&b',') {
            standard + 3600
        } else {
            -c.clock()?
        };
        if !c.take(b',') {
            return None;
        }
        let start = c.rule()?;
        if !c.take(b',') {
            return None;
        }
        let end = c.rule()?;
        c.0.is_empty().then_some(Self {
            standard,
            daylight,
            rules: Some((start, end)),
        })
    }
    pub fn offset(&self, year: i16, timestamp: i64) -> Option<i32> {
        self.info(year, timestamp).map(|(offset, _)| offset)
    }
    fn info(&self, year: i16, timestamp: i64) -> Option<(i32, bool)> {
        let Some((start, end)) = &self.rules else {
            return Some((self.standard, false));
        };
        let a = start.transition(year, self.standard)?;
        let b = end.transition(year, self.daylight)?;
        let dst = if a > b {
            timestamp < b || timestamp >= a
        } else {
            timestamp >= a && timestamp < b
        };
        Some((if dst { self.daylight } else { self.standard }, dst))
    }
    // The native mktime standard-time request searches at fixed bounded
    // intervals, then assumes one hour when no standard-time probe is found.
    // This matters for deliberately short standard periods and non-hour DST.
    pub fn legacy_standard(&self, wall: i64) -> Option<i64> {
        let info = |t| {
            let ts = jiff::Timestamp::from_second(t).ok()?;
            self.info(TimeZone::UTC.to_datetime(ts).year(), t)
        };
        let mut t = wall;
        let mut previous = None;
        let mut converged = false;
        for _ in 0..6 {
            let (offset, _) = info(t)?;
            let next = wall.checked_sub(offset as i64)?;
            if next == t {
                converged = true;
                break;
            }
            if previous == Some(next) {
                // Standard-time interpretation across a spring-forward gap.
                return wall.checked_sub(self.standard as i64);
            }
            previous = Some(t);
            t = next;
        }
        if !converged {
            return None;
        }
        if !info(t)?.1 {
            return Some(t);
        }
        for distance in (601_200..229_222_804i64).step_by(601_200) {
            for direction in [-1, 1] {
                let probe = t.checked_add(distance * direction)?;
                let (offset, dst) = info(probe)?;
                if !dst {
                    return wall.checked_sub(offset as i64);
                }
            }
        }
        t.checked_add(3600)
    }
}
impl Rule {
    fn transition(&self, year: i16, prior_offset: i32) -> Option<i64> {
        let jan = Date::new(year, 1, 1).ok()?;
        let leap = jan.in_leap_year();
        let ordinal = match self.day {
            Day::Ordinal(n) => n as i64,
            Day::Julian(n) => n as i64 - 1 + i64::from(leap && n >= 60),
            Day::Week {
                month,
                week,
                weekday,
            } => {
                let first = Date::new(year, month, 1).ok()?;
                let start =
                    (weekday as i16 - first.weekday().to_sunday_zero_offset() as i16).rem_euclid(7);
                let mut day = start + 7 * (week as i16 - 1);
                if day >= first.days_in_month() as i16 {
                    day -= 7;
                }
                first.day_of_year() as i64 - 1 + day as i64
            }
        };
        let base = if year <= 1970 {
            0
        } else {
            TimeZone::UTC
                .to_timestamp(jan.at(0, 0, 0, 0))
                .ok()?
                .as_second()
        };
        base.checked_add(ordinal * 86400)?
            .checked_add(self.seconds as i64)?
            .checked_sub(prior_offset as i64)
    }
}
