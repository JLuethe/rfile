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
    Dfa {
        first: Box<dfa::regex::Regex>,
        longest: Box<dense::DFA<Vec<u32>>>,
    },
    Nfa {
        first: PikeVM,
        longest: PikeVM,
    },
}
fn pattern(pattern: &[u8], insensitive: bool) -> (String, syntax::Config) {
    let mut text = String::new();
    for &c in pattern {
        if c >= 128 || c == 0 {
            text.push_str(&format!("\\x{c:02x}"));
        } else {
            text.push(c as char);
        }
    }
    // POSIX REG_NEWLINE excludes LF from negated classes as well as dot.
    let mut transformed = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        transformed.push(c);
        if c == '\\' {
            if let Some(next) = chars.next() {
                transformed.push(next);
            }
            continue;
        }
        if c == '[' && chars.peek() == Some(&'^') {
            transformed.push(chars.next().expect("peeked"));
            transformed.push_str("\\n");
        }
    }
    (
        transformed,
        syntax::Config::new()
            .unicode(false)
            .utf8(false)
            .multi_line(true)
            .case_insensitive(insensitive)
            .octal(true),
    )
}
pub(crate) fn validate(bytes: &[u8], insensitive: bool) -> std::result::Result<(), String> {
    let (text, syntax) = pattern(bytes, insensitive);
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
        let (transformed, syntax) = pattern(bytes, insensitive);
        let dense = dense::Config::new()
            .dfa_size_limit(Some(4 * 1024 * 1024))
            .determinize_size_limit(Some(8 * 1024 * 1024));
        let nfa = thompson::Config::new()
            .utf8(false)
            .nfa_size_limit(Some(4 * 1024 * 1024));
        // Bounded repetitions can require exponentially many DFA states. Use the
        // bounded Thompson VM directly for those patterns instead of first paying
        // the full determinization limit during an ordinary CLI invocation.
        if !bytes.contains(&b'{') {
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
            Self::Dfa { .. } => 2,
            Self::Nfa { first, .. } => first.get_nfa().states().len().saturating_mul(2),
        }
    }
    pub fn find(&self, data: &[u8]) -> std::result::Result<Option<(usize, usize)>, String> {
        match self {
            Self::Dfa { first, longest } => {
                let Some(first) = first
                    .try_search(&Input::new(data))
                    .map_err(|e| e.to_string())?
                else {
                    return Ok(None);
                };
                let start = first.start();
                let input = Input::new(data)
                    .span(start..data.len())
                    .anchored(Anchored::Yes);
                let last = longest.try_search_fwd(&input).map_err(|e| e.to_string())?;
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
