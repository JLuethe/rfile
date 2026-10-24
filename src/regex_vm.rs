// SPDX-License-Identifier: GPL-3.0-only
//! Capture-aware ERE matching with finite instructions, work and live history.
//! States retain capture values in the memo key: (pc,offset) alone is unsound
//! when a later backreference can distinguish two paths.
use crate::{Error, InspectionBudget, Result, regex_dialect::Pattern};
use regex_syntax::hir::{Class, Hir, HirKind, Look};
use std::{collections::HashSet, sync::Arc};
#[derive(Debug)]
enum Inst {
    Accept,
    Bytes([u64; 4], usize),
    Split(usize, usize),
    Save(usize, usize),
    Assert(Look, usize),
    Reference(usize, usize),
    Jump(usize),
}
#[derive(Debug)]
pub(crate) struct Program {
    code: Vec<Inst>,
    entry: usize,
    slots: usize,
    insensitive: bool,
    pinned_ambiguous: bool,
}
impl Program {
    fn emit(&mut self, inst: Inst) -> std::result::Result<usize, String> {
        if self.code.len() >= 65536 {
            return Err("regex VM instruction limit".into());
        }
        let i = self.code.len();
        self.code.push(inst);
        Ok(i)
    }
    fn expression(
        &mut self,
        hir: &Hir,
        next: usize,
        pattern: &Pattern,
    ) -> std::result::Result<usize, String> {
        match hir.kind() {
            HirKind::Empty => Ok(next),
            HirKind::Literal(lit) => {
                let mut at = next;
                for &b in lit.0.iter().rev() {
                    let mut bits = [0u64; 4];
                    bits[b as usize / 64] |= 1 << (b % 64);
                    at = self.emit(Inst::Bytes(bits, at))?;
                }
                Ok(at)
            }
            HirKind::Class(Class::Bytes(class)) => {
                let mut bits = [0u64; 4];
                for r in class.ranges() {
                    for b in r.start()..=r.end() {
                        bits[b as usize / 64] |= 1 << (b % 64);
                    }
                }
                self.emit(Inst::Bytes(bits, next))
            }
            HirKind::Class(Class::Unicode(class)) => {
                let mut bits = [0u64; 4];
                for range in class.ranges() {
                    if range.end() > '\x7f' {
                        return Err("non-ASCII Unicode regex node in byte dialect".into());
                    }
                    for b in range.start() as u8..=range.end() as u8 {
                        bits[b as usize / 64] |= 1 << (b % 64);
                    }
                }
                self.emit(Inst::Bytes(bits, next))
            }
            HirKind::Look(look) => {
                if matches!(
                    look,
                    Look::WordUnicode
                        | Look::WordUnicodeNegate
                        | Look::WordStartUnicode
                        | Look::WordEndUnicode
                        | Look::WordStartHalfUnicode
                        | Look::WordEndHalfUnicode
                ) {
                    return Err("Unicode look in byte dialect".into());
                }
                self.emit(Inst::Assert(*look, next))
            }
            HirKind::Concat(parts) => {
                let mut at = next;
                for part in parts.iter().rev() {
                    at = self.expression(part, at, pattern)?;
                }
                Ok(at)
            }
            HirKind::Alternation(parts) => {
                let mut entries = Vec::new();
                for part in parts {
                    entries.push(self.expression(part, next, pattern)?);
                }
                let mut at = entries.pop().expect("nonempty HIR alternation");
                for first in entries.into_iter().rev() {
                    at = self.emit(Inst::Split(first, at))?;
                }
                Ok(at)
            }
            HirKind::Capture(cap) => {
                let id = cap.index as usize;
                if let Some(g) = pattern.references.get(id).copied().flatten() {
                    return self.emit(Inst::Reference(g, next));
                }
                let g = pattern
                    .capture_map
                    .get(id)
                    .copied()
                    .flatten()
                    .ok_or_else(|| "regex capture mapping".to_string())?;
                let close = self.emit(Inst::Save(g * 2 + 1, next))?;
                let body = self.expression(&cap.sub, close, pattern)?;
                self.emit(Inst::Save(g * 2, body))
            }
            HirKind::Repetition(rep) => {
                if rep.min > 4096 || rep.max.is_some_and(|n| n > 4096) {
                    return Err("regex VM repetition expansion limit".into());
                }
                let mut at = next;
                if let Some(max) = rep.max {
                    for _ in rep.min..max {
                        let body = self.expression(&rep.sub, at, pattern)?;
                        at = self.emit(Inst::Split(body, at))?;
                    }
                } else {
                    let jump = self.emit(Inst::Jump(0))?;
                    let body = self.expression(&rep.sub, jump, pattern)?;
                    self.code[jump] = Inst::Split(body, next);
                    at = jump;
                }
                for _ in 0..rep.min {
                    at = self.expression(&rep.sub, at, pattern)?;
                }
                Ok(at)
            }
        }
    }
    pub fn compile(pattern: &Pattern, insensitive: bool) -> std::result::Result<Self, String> {
        let hir = regex_syntax::ParserBuilder::new()
            .unicode(false)
            .utf8(false)
            .multi_line(true)
            .case_insensitive(insensitive)
            .nest_limit(250)
            .build()
            .parse(&pattern.text)
            .map_err(|e| e.to_string())?;
        let mut program = Self {
            code: vec![Inst::Accept],
            entry: 0,
            slots: (pattern.captures + 1) * 2,
            insensitive,
            pinned_ambiguous: pinned_ambiguity(&hir, pattern, &mut Vec::new()),
        };
        program.entry = program.expression(&hir, 0, pattern)?;
        Ok(program)
    }
    pub fn memory_usage(&self) -> usize {
        self.code.capacity() * std::mem::size_of::<Inst>()
    }
    pub fn find(
        &self,
        data: &[u8],
        budget: &InspectionBudget,
        profile: crate::SemanticProfile,
    ) -> Result<Option<(usize, usize)>> {
        if self.pinned_ambiguous && profile == crate::SemanticProfile::Pinned548 {
            return Err(Error::Unsupported("pinned glibc quantified backreference semantics; use corrected-v1 for exhaustive capture histories".into()));
        }
        if budget.limits.regex_states == 0 {
            return Err(Error::Limit("regex states"));
        }
        let mut fuel = 0usize;
        // Count all transitions and backreference comparisons, including failed
        // starts, against one regex budget rather than restarting at each byte.
        let mut charge = |n: usize| -> Result<()> {
            fuel = fuel.checked_add(n).ok_or(Error::Limit("regex steps"))?;
            if fuel > budget.limits.regex_steps {
                return Err(Error::Limit("regex steps"));
            }
            budget.steps(n)?;
            budget.checkpoint()
        };
        for start in 0..=data.len() {
            charge(1)?;
            let mut pending = Vec::new();
            let mut visited = HashSet::new();
            let mut leases = Vec::new();
            let bytes_per_state = self
                .slots
                .saturating_mul(std::mem::size_of::<Option<usize>>())
                .saturating_mul(4)
                .saturating_add(512);
            let initial = budget.reserve(bytes_per_state)?;
            let state = State {
                pc: self.entry,
                pos: start,
                captures: vec![None; self.slots].into(),
            };
            pending.push(state.clone());
            visited.insert(state);
            leases.push(initial);
            let mut best = None;
            while let Some(mut state) = pending.pop() {
                charge(1)?;
                budget.bytes(self.slots)?;
                let mut branch = None;
                match &self.code[state.pc] {
                    Inst::Accept => {
                        best = Some(best.unwrap_or(0).max(state.pos));
                        continue;
                    }
                    Inst::Bytes(bits, next) => {
                        let Some(&b) = data.get(state.pos) else {
                            continue;
                        };
                        if bits[b as usize / 64] & (1 << (b % 64)) == 0 {
                            continue;
                        }
                        state.pos += 1;
                        state.pc = *next;
                    }
                    Inst::Split(a, b) => {
                        let mut other = state.clone();
                        other.pc = *b;
                        branch = Some(other);
                        state.pc = *a;
                    }
                    Inst::Save(slot, next) => {
                        // Reserve before copy-on-write creates a new capture
                        // history. The state lease accounts for old/new copies.
                        let lease = budget.reserve(bytes_per_state)?;
                        let captures = Arc::make_mut(&mut state.captures);
                        captures[*slot] = Some(state.pos);
                        if slot.is_multiple_of(2) {
                            captures[*slot + 1] = None;
                        }
                        state.pc = *next;
                        leases.push(lease);
                    }
                    Inst::Assert(look, next) => {
                        if !assertion(*look, data, state.pos) {
                            continue;
                        }
                        state.pc = *next;
                    }
                    Inst::Reference(g, next) => {
                        let (Some(a), Some(b)) = (state.captures[g * 2], state.captures[g * 2 + 1])
                        else {
                            continue;
                        };
                        let n = b - a;
                        let Some(value) = data.get(state.pos..state.pos.saturating_add(n)) else {
                            continue;
                        };
                        charge(n)?;
                        budget.bytes(n)?;
                        let equal = if self.insensitive {
                            data[a..b].eq_ignore_ascii_case(value)
                        } else {
                            &data[a..b] == value
                        };
                        if !equal {
                            continue;
                        }
                        state.pos += n;
                        state.pc = *next;
                    }
                    Inst::Jump(next) => state.pc = *next,
                }
                for next in std::iter::once(state).chain(branch) {
                    if visited.contains(&next) {
                        continue;
                    }
                    if visited.len() >= budget.limits.regex_states {
                        return Err(Error::Limit("regex states"));
                    }
                    let lease = budget.reserve(bytes_per_state)?;
                    visited
                        .try_reserve(1)
                        .map_err(|_| Error::Limit("regex state allocation"))?;
                    pending
                        .try_reserve(1)
                        .map_err(|_| Error::Limit("regex state allocation"))?;
                    visited.insert(next.clone());
                    pending.push(next);
                    leases.push(lease);
                }
            }
            if let Some(end) = best {
                return Ok(Some((start, end)));
            }
        }
        Ok(None)
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
struct State {
    pc: usize,
    pos: usize,
    captures: Arc<[Option<usize>]>,
}
fn assertion(look: Look, data: &[u8], pos: usize) -> bool {
    let left = pos.checked_sub(1).and_then(|p| data.get(p)).copied();
    let right = data.get(pos).copied();
    let word = |b: Option<u8>| b.is_some_and(|b| b.is_ascii_alphanumeric() || b == b'_');
    let (l, r) = (word(left), word(right));
    match look {
        Look::Start => pos == 0,
        Look::End => pos == data.len(),
        Look::StartLF => pos == 0 || left == Some(b'\n'),
        Look::EndLF => pos == data.len() || right == Some(b'\n'),
        Look::StartCRLF => {
            pos == 0 || left == Some(b'\n') || (left == Some(b'\r') && right != Some(b'\n'))
        }
        Look::EndCRLF => {
            pos == data.len()
                || right == Some(b'\r')
                || (right == Some(b'\n') && left != Some(b'\r'))
        }
        Look::WordAscii => l != r,
        Look::WordAsciiNegate => l == r,
        Look::WordStartAscii => !l && r,
        Look::WordEndAscii => l && !r,
        Look::WordStartHalfAscii => !l,
        Look::WordEndHalfAscii => !r,
        _ => false, // Unsupported Unicode looks are rejected during compilation.
    }
}

/// Do not silently substitute mathematical capture enumeration for known host
/// glibc pruning/interval quirks. Corrected mode is explicit; pinned mode rejects
/// this conservative construct class until its native history rules are ported.
fn pinned_ambiguity(hir: &Hir, pattern: &Pattern, ancestors: &mut Vec<(bool, bool)>) -> bool {
    match hir.kind() {
        HirKind::Repetition(rep) => {
            let interval = rep.max.is_some_and(|max| max > 1 && max != rep.min);
            ancestors.push((interval, rep.max.is_none() && rep.min > 0));
            let risk = pinned_ambiguity(&rep.sub, pattern, ancestors);
            ancestors.pop();
            risk
        }
        HirKind::Capture(cap) => {
            if let Some(group) = pattern
                .capture_map
                .get(cap.index as usize)
                .copied()
                .flatten()
            {
                if pattern.references.contains(&Some(group)) {
                    let variable = cap.sub.properties().minimum_len()
                        != cap.sub.properties().maximum_len()
                        || contains_reference(&cap.sub, pattern);
                    if ancestors
                        .iter()
                        .any(|&(interval, looping)| interval || (looping && variable))
                    {
                        return true;
                    }
                }
            }
            pinned_ambiguity(&cap.sub, pattern, ancestors)
        }
        _ => hir
            .kind()
            .subs()
            .iter()
            .any(|sub| pinned_ambiguity(sub, pattern, ancestors)),
    }
}

fn contains_reference(hir: &Hir, pattern: &Pattern) -> bool {
    if let HirKind::Capture(cap) = hir.kind() {
        if pattern
            .references
            .get(cap.index as usize)
            .copied()
            .flatten()
            .is_some()
        {
            return true;
        }
    }
    hir.kind()
        .subs()
        .iter()
        .any(|sub| contains_reference(sub, pattern))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ExecutionControl, Limits, SemanticProfile as Profile};
    fn compile(pattern: &str, insensitive: bool) -> Program {
        Program::compile(
            &crate::regex_dialect::normalize(pattern.as_bytes(), insensitive, false).unwrap(),
            insensitive,
        )
        .unwrap()
    }
    fn budget(limits: &Limits) -> InspectionBudget {
        InspectionBudget::new(limits, ExecutionControl::default()).unwrap()
    }
    #[test]
    fn captures_keep_histories_unset_empty_and_longest_separate() {
        for (pattern, input, expected) in [
            (r"(a|ab)\1", b"abab".as_slice(), Some((0, 4))),
            (r"(ab|a)\1", b"aaaa", Some((0, 2))),
            (r"((a)|b)*\2", b"aba", Some((0, 3))),
            (r"(a)?\1", b"b", None),
            (r"(a)?\1", b"", None),
            (r"()\1", b"", Some((0, 0))),
            (r"(a|aa)+\1", b"aaa", Some((0, 3))),
            (r"(a){0,2}\1", b"aa", Some((0, 2))),
            (r"(a*)*\1", b"aaa", Some((0, 3))),
            (r"(a*)\1", b"aaa", Some((0, 2))),
            (r"(a)(b)\2\1", b"xxabba", Some((2, 6))),
            (r"(a)\1{2}", b"aaa", Some((0, 3))),
            (r"()\1+", b"", Some((0, 0))),
            (r"(a+)((\1))+\2", b"aaaa", Some((0, 4))),
        ] {
            let p = compile(pattern, false);
            let b = budget(&Limits::default());
            assert_eq!(
                p.find(input, &b, Profile::CorrectedV1).unwrap(),
                expected,
                "{pattern}"
            );
            assert_eq!(b.metrics().tracked_memory_live, 0);
        }
        assert_eq!(
            compile(r"(ab)\1", true)
                .find(b"ABab", &budget(&Limits::default()), Profile::Pinned548)
                .unwrap(),
            Some((0, 4))
        );
    }
    #[test]
    fn pinned_quantifier_boundary_is_visible_and_conservative() {
        for pattern in [r"(a*)+\1", r"(a|aa)+\1", r"(a){0,2}\1", r"(a+)((\1))+\2"] {
            assert!(
                matches!(
                    compile(pattern, false).find(
                        b"aaaa",
                        &budget(&Limits::default()),
                        Profile::Pinned548
                    ),
                    Err(Error::Unsupported(_))
                ),
                "{pattern}"
            );
        }
        // Ordinary fixed-width + and zero-minimum * retain tested C behavior.
        for pattern in [r"(a)+\1", r"(a*)*\1", r"(a|aa)*\1"] {
            assert!(
                compile(pattern, false)
                    .find(b"aaaa", &budget(&Limits::default()), Profile::Pinned548)
                    .unwrap()
                    .is_some()
            );
        }
    }
    #[test]
    fn states_steps_memory_and_work_exhaustion_never_return_a_partial_match() {
        let p = compile(r"((a|aa)*)\1", false);
        for limits in [
            Limits {
                regex_steps: 1,
                ..Limits::default()
            },
            Limits {
                regex_states: 0,
                ..Limits::default()
            },
            Limits {
                regex_states: 2,
                ..Limits::default()
            },
            Limits {
                memory: 1,
                ..Limits::default()
            },
            Limits {
                work_bytes: 1,
                ..Limits::default()
            },
            Limits {
                steps: 1,
                ..Limits::default()
            },
        ] {
            let b = budget(&limits);
            let r = p.find(b"aaaaaaaaaaaaaaaaaaaa", &b, Profile::CorrectedV1);
            assert!(r.unwrap_err().is_limit());
            assert_eq!(b.metrics().tracked_memory_live, 0);
        }
    }
    #[test]
    fn compile_expansion_and_capture_limits_precede_evaluation() {
        assert!(
            crate::regex_dialect::normalize(
                ("(".repeat(33) + &")".repeat(33)).as_bytes(),
                false,
                false
            )
            .is_err()
        );
        for pattern in [r"(a){100000}\1", r"(a{4096}){4096}\1"] {
            let parsed = crate::regex_dialect::normalize(pattern.as_bytes(), false, false).unwrap();
            assert!(
                Program::compile(&parsed, false)
                    .unwrap_err()
                    .contains("limit")
            );
        }
    }
}
