// rfile contributions: Copyright (C) 2026 rfile contributors.
// SPDX-License-Identifier: GPL-3.0-only
//! Select the earliest start, then its longest overall match, without C regex FFI.
use regex_automata::{
    Anchored, Input, MatchKind,
    dfa::{self, Automaton, dense},
    nfa::thompson::{self, pikevm::PikeVM},
    util::syntax,
};

#[derive(Debug)]
pub(crate) enum CompiledRegex {
    Profiles {
        pinned: Box<CompiledRegex>,
        corrected: Box<CompiledRegex>,
    },
    Capture(crate::regex_vm::Program),
    Dfa {
        first: Box<dfa::regex::Regex>,
        longest: Box<dense::DFA<Vec<u32>>>,
    },
    Nfa {
        first: PikeVM,
        longest: PikeVM,
    },
}
fn pattern(
    bytes: &[u8],
    insensitive: bool,
    pinned: bool,
) -> std::result::Result<(crate::regex_dialect::Pattern, syntax::Config), String> {
    Ok((
        crate::regex_dialect::normalize(bytes, insensitive, pinned)?,
        syntax::Config::new()
            .unicode(false)
            .utf8(false)
            .multi_line(true)
            .case_insensitive(insensitive),
    ))
}
pub(crate) fn validate(bytes: &[u8], insensitive: bool) -> std::result::Result<(), String> {
    let (pattern, syntax) = pattern(bytes, insensitive, false)?;
    if pattern.references.iter().any(Option::is_some) {
        return crate::regex_vm::Program::compile(&pattern, insensitive).map(|_| ());
    }
    let text = pattern.text;
    thompson::NFA::compiler()
        .syntax(syntax)
        .configure(
            thompson::Config::new()
                .utf8(false)
                .nfa_size_limit(Some(4 * 1024 * 1024)),
        )
        .build(&text)
        .map(|_| ())
        .map_err(|e| e.to_string())
}
impl CompiledRegex {
    pub fn compile(bytes: &[u8], insensitive: bool) -> std::result::Result<Self, String> {
        let (pinned, syntax) = pattern(bytes, insensitive, true)?;
        let (corrected, _) = pattern(bytes, insensitive, false)?;
        if pinned.text != corrected.text {
            return Ok(Self::Profiles {
                pinned: Box::new(Self::compile_pattern(pinned, syntax, insensitive)?),
                corrected: Box::new(Self::compile_pattern(corrected, syntax, insensitive)?),
            });
        }
        Self::compile_pattern(pinned, syntax, insensitive)
    }
    fn compile_pattern(
        pattern: crate::regex_dialect::Pattern,
        syntax: syntax::Config,
        insensitive: bool,
    ) -> std::result::Result<Self, String> {
        if pattern.references.iter().any(Option::is_some) {
            return Ok(Self::Capture(crate::regex_vm::Program::compile(
                &pattern,
                insensitive,
            )?));
        }
        let transformed = pattern.text;
        let dense = dense::Config::new()
            .dfa_size_limit(Some(4 * 1024 * 1024))
            .determinize_size_limit(Some(8 * 1024 * 1024));
        let nfa = thompson::Config::new()
            .utf8(false)
            .nfa_size_limit(Some(4 * 1024 * 1024));
        // Bounded repetitions can require exponentially many DFA states. Use the
        // bounded Thompson VM directly for those patterns instead of first paying
        // the full determinization limit during an ordinary CLI invocation.
        if !transformed.contains('{') {
            let first = dfa::regex::Regex::builder()
                .syntax(syntax)
                .thompson(nfa.clone())
                .dense(dense.clone())
                .build(&transformed);
            if let Ok(first) = first {
                if let Ok(longest) = dense::Builder::new()
                    .syntax(syntax)
                    .thompson(nfa.clone())
                    .configure(dense.match_kind(MatchKind::All))
                    .build(&transformed)
                {
                    return Ok(Self::Dfa {
                        first: Box::new(first),
                        longest: Box::new(longest),
                    });
                }
            }
        }
        let compiled = thompson::NFA::compiler()
            .syntax(syntax)
            .configure(nfa)
            .build(&transformed)
            .map_err(|e| e.to_string())?;
        let first = PikeVM::builder()
            .build_from_nfa(compiled.clone())
            .map_err(|e| e.to_string())?;
        let longest = PikeVM::builder()
            .configure(PikeVM::config().match_kind(MatchKind::All))
            .build_from_nfa(compiled)
            .map_err(|e| e.to_string())?;
        Ok(Self::Nfa { first, longest })
    }
    pub fn memory_usage(&self) -> usize {
        match self {
            Self::Profiles { pinned, corrected } => pinned
                .memory_usage()
                .saturating_add(corrected.memory_usage()),
            Self::Capture(program) => program.memory_usage(),
            Self::Dfa { first, longest } => {
                first.forward().memory_usage()
                    + first.reverse().memory_usage()
                    + longest.memory_usage()
            }
            Self::Nfa { first, longest } => {
                first.get_nfa().memory_usage() + longest.get_nfa().memory_usage()
            }
        }
    }
    /// Conservative upper bound for two active-state tables, capture slots,
    /// epsilon stack and temporary match storage, before creating a Pike cache.
    pub fn scratch_bound(&self) -> usize {
        match self {
            Self::Profiles { pinned, corrected } => {
                pinned.scratch_bound().max(corrected.scratch_bound())
            }
            Self::Capture(_) => 0,
            Self::Dfa { .. } => 4096,
            Self::Nfa { first, .. } => {
                let nfa = first.get_nfa();
                nfa.states()
                    .len()
                    .saturating_add(1)
                    .saturating_mul(nfa.group_info().slot_len().saturating_add(16))
                    .saturating_mul(std::mem::size_of::<usize>())
                    .saturating_mul(4)
            }
        }
    }
    pub fn work_factor(&self) -> usize {
        match self {
            Self::Profiles { pinned, corrected } => {
                pinned.work_factor().max(corrected.work_factor())
            }
            Self::Capture(_) => 0,
            Self::Dfa { .. } => 2,
            Self::Nfa { first, .. } => first.get_nfa().states().len().saturating_mul(2),
        }
    }
    pub fn find(
        &self,
        data: &[u8],
        budget: &crate::InspectionBudget,
        profile: crate::SemanticProfile,
    ) -> crate::Result<Option<(usize, usize)>> {
        match self {
            Self::Profiles { pinned, corrected } => {
                if profile == crate::SemanticProfile::Pinned548 {
                    pinned.find(data, budget, profile)
                } else {
                    corrected.find(data, budget, profile)
                }
            }
            Self::Capture(program) => program.find(data, budget, profile),
            Self::Dfa { first, longest } => {
                let Some(first) = first
                    .try_search(&Input::new(data))
                    .map_err(|e| crate::Error::Unsupported(e.to_string()))?
                else {
                    return Ok(None);
                };
                let start = first.start();
                let input = Input::new(data)
                    .span(start..data.len())
                    .anchored(Anchored::Yes);
                let last = longest
                    .try_search_fwd(&input)
                    .map_err(|e| crate::Error::Unsupported(e.to_string()))?;
                Ok(Some((start, last.map_or(first.end(), |m| m.offset()))))
            }
            Self::Nfa { first, longest } => {
                let Some(found) = first.find(&mut first.create_cache(), Input::new(data)) else {
                    return Ok(None);
                };
                let start = found.start();
                let input = Input::new(data)
                    .span(start..data.len())
                    .anchored(Anchored::Yes);
                let end = longest
                    .find(&mut longest.create_cache(), input)
                    .map_or(found.end(), |m| m.end());
                Ok(Some((start, end)))
            }
        }
    }
}
