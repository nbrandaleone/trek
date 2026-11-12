//! Input tokenizer (`scan`, `ja`) matching Tom Almy's C `scan()` semantics exactly.

use crate::io::{prout, IoHandle};
use crate::state::GameState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScanToken {
    Eol,
    Alpha,
    Real,
}

/// Parse the longest leading C `%lf` float prefix from `s`, returning `(value, bytes_consumed)`.
pub fn parse_c_double_prefix(s: &str) -> Option<(f64, usize)> {
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut pos = 0;

    if pos < len && (bytes[pos] == b'+' || bytes[pos] == b'-') {
        pos += 1;
    }

    let mut digits_before = 0usize;
    while pos < len && bytes[pos].is_ascii_digit() {
        pos += 1;
        digits_before += 1;
    }

    let mut digits_after = 0usize;
    if pos < len && bytes[pos] == b'.' {
        pos += 1;
        while pos < len && bytes[pos].is_ascii_digit() {
            pos += 1;
            digits_after += 1;
        }
    }

    if digits_before == 0 && digits_after == 0 {
        return None;
    }

    // Optional exponent
    let exp_start = pos;
    if pos < len && (bytes[pos] == b'e' || bytes[pos] == b'E') {
        let mut epos = pos + 1;
        if epos < len && (bytes[epos] == b'+' || bytes[epos] == b'-') {
            epos += 1;
        }
        let exp_digits_start = epos;
        while epos < len && bytes[epos].is_ascii_digit() {
            epos += 1;
        }
        if epos > exp_digits_start {
            pos = epos;
        } else {
            pos = exp_start;
        }
    }

    let slice = &s[..pos];
    let val = slice.parse::<f64>().ok()?;
    Some((val, pos))
}

pub async fn scan(state: &mut GameState, io: &IoHandle) -> ScanToken {
    state.linecount = 0;
    state.aaitem = 0.0;
    state.citem.clear();

    if state.line_pos >= state.line.len() {
        if state.consumed_on_line {
            state.chew();
            return ScanToken::Eol;
        }
        let mut raw = io.read_line(state).await;
        while raw.ends_with('\n') || raw.ends_with('\r') {
            raw.pop();
        }
        state.line = raw;
        state.line_pos = 0;
        state.consumed_on_line = false;
    }

    let bytes = state.line.as_bytes();
    while state.line_pos < bytes.len() && bytes[state.line_pos] == b' ' {
        state.line_pos += 1;
    }

    if state.line_pos >= state.line.len() {
        state.chew();
        return ScanToken::Eol;
    }

    let first = state.line.as_bytes()[state.line_pos];
    if first.is_ascii_digit() || first == b'+' || first == b'-' || first == b'.' {
        let rest = &state.line[state.line_pos..];
        if let Some((val, consumed)) = parse_c_double_prefix(rest) {
            state.aaitem = val;
            state.line_pos += consumed;
            state.consumed_on_line = true;
            return ScanToken::Real;
        } else {
            state.chew();
            return ScanToken::Eol;
        }
    }

    // Read non-space word, lowercasing ASCII and storing up to 9 chars in `citem`.
    let rest = &state.line[state.line_pos..];
    let mut char_count = 0usize;
    let mut byte_advance = 0usize;
    for ch in rest.chars() {
        if ch == ' ' {
            break;
        }
        if char_count < 9 {
            state.citem.push(ch.to_ascii_lowercase());
            char_count += 1;
        }
        byte_advance += ch.len_utf8();
    }
    state.line_pos += byte_advance;
    state.consumed_on_line = true;
    ScanToken::Alpha
}

pub async fn ja(state: &mut GameState, io: &IoHandle) -> bool {
    state.chew();
    loop {
        scan(state, io).await;
        state.chew();
        if state.citem.starts_with('y') {
            return true;
        }
        if state.citem.starts_with('n') {
            return false;
        }
        state.proutn("Please answer with \"Y\" or \"N\":");
    }
}

/// Helper to print a prompt and scan a single token if needed.
pub async fn prout_prompt_scan(state: &mut GameState, io: &IoHandle, msg: &str) -> ScanToken {
    prout(state, io, msg).await;
    scan(state, io).await
}
