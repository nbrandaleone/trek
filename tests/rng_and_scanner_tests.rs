use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

use trek::{
    parse_c_double_prefix, scan, GameState, IoHandle, MacOsRng, PromptType, ScanToken,
    TrekSession, COMMANDS,
};

fn poll_ready<F: Future>(fut: F) -> F::Output {
    let mut pinned = Box::pin(fut);
    let waker = Waker::noop();
    let mut cx = Context::from_waker(waker);
    match Pin::as_mut(&mut pinned).poll(&mut cx) {
        Poll::Ready(val) => val,
        Poll::Pending => panic!("future unexpectedly yielded waiting for I/O"),
    }
}

#[test]
fn rng_exact_park_miller_sequence_seed_0_1_42() {
    // Seed 0 maps to 123_459_876 per Darwin/macOS srand(0).
    let mut rng0 = MacOsRng::new(0);
    assert_eq!(rng0.state(), 123_459_876);
    let expected0: [i32; 5] = [
        ((123_459_876u64 * 16_807) % 2_147_483_647) as i32,
        0,
        0,
        0,
        0,
    ];
    let mut seq0 = Vec::new();
    for _ in 0..5 {
        seq0.push(rng0.rand_int());
    }
    assert_eq!(seq0[0], expected0[0]);
    // Verify recurrence on subsequent elements.
    for i in 1..5 {
        let next = (((seq0[i - 1] as u64) * 16_807) % 2_147_483_647) as i32;
        assert_eq!(seq0[i], next);
    }

    // Seed 1: canonical Park-Miller MINSTD sequence.
    let mut rng1 = MacOsRng::new(1);
    assert_eq!(rng1.state(), 1);
    assert_eq!(rng1.rand_int(), 16_807);
    assert_eq!(rng1.rand_int(), 282_475_249);
    assert_eq!(rng1.rand_int(), 1_622_650_073);
    assert_eq!(rng1.rand_int(), 984_943_658);
    assert_eq!(rng1.rand_int(), 1_144_108_930);

    // Seed 42: exact Park-Miller recurrence.
    let mut rng42 = MacOsRng::new(42);
    assert_eq!(rng42.state(), 42);
    let r1 = rng42.rand_int();
    assert_eq!(r1, 42 * 16_807); // 705_894
    let r2 = rng42.rand_int();
    assert_eq!(r2, (((705_894u64) * 16_807) % 2_147_483_647) as i32);

    // Seed == modulus wraps to 0 -> 123_459_876.
    let rng_mod = MacOsRng::new(2_147_483_647);
    assert_eq!(rng_mod.state(), 123_459_876);
    assert_eq!(rng_mod, MacOsRng::new(0));
}

#[test]
fn rng_rand_f64_expran_and_iran_bounds() {
    let mut rng_a = MacOsRng::new(42);
    let mut rng_b = MacOsRng::new(42);

    let expected_f64 = (rng_b.rand_int() as f64) / 2_147_483_648.0;
    let actual_expran = rng_a.expran(12.5);
    let expected_expran = -12.5 * (1e-7 + expected_f64).ln();
    assert!((actual_expran - expected_expran).abs() < 1e-12);
    assert!(actual_expran > 0.0);

    // Verify iran8 covers 1..=8 and never goes out of bounds.
    let mut seen8 = [false; 9];
    let mut seen8_y = [false; 9];
    for _ in 0..5_000 {
        let (x, y) = rng_a.iran8();
        assert!((1..=8).contains(&x), "iran8 x out of bounds: {x}");
        assert!((1..=8).contains(&y), "iran8 y out of bounds: {y}");
        seen8[x as usize] = true;
        seen8_y[y as usize] = true;
    }
    assert!(seen8[1..=8].iter().all(|&v| v));
    assert!(seen8_y[1..=8].iter().all(|&v| v));

    // Verify iran10 covers 1..=10 and never goes out of bounds.
    let mut seen10 = [false; 11];
    let mut seen10_y = [false; 11];
    for _ in 0..5_000 {
        let (x, y) = rng_a.iran10();
        assert!((1..=10).contains(&x), "iran10 x out of bounds: {x}");
        assert!((1..=10).contains(&y), "iran10 y out of bounds: {y}");
        seen10[x as usize] = true;
        seen10_y[y as usize] = true;
    }
    assert!(seen10[1..=10].iter().all(|&v| v));
    assert!(seen10_y[1..=10].iter().all(|&v| v));
}

#[test]
fn scanner_parse_c_double_prefix_cases() {
    assert_eq!(parse_c_double_prefix("3.5abc"), Some((3.5, 3)));
    assert_eq!(parse_c_double_prefix("-12.25e2xyz"), Some((-1225.0, 8)));
    assert_eq!(parse_c_double_prefix("+42.5"), Some((42.5, 5)));
    assert_eq!(parse_c_double_prefix(".5x"), Some((0.5, 2)));
    assert_eq!(parse_c_double_prefix("-.25"), Some((-0.25, 4)));
    assert_eq!(parse_c_double_prefix("1e+3foo"), Some((1000.0, 4)));
    assert_eq!(parse_c_double_prefix("1e-2foo"), Some((0.01, 4)));
    // Incomplete exponent should not consume 'e'
    assert_eq!(parse_c_double_prefix("2ebar"), Some((2.0, 1)));
    assert_eq!(parse_c_double_prefix("2e+bar"), Some((2.0, 1)));
    // Invalid bare signs/dots
    assert_eq!(parse_c_double_prefix("+"), None);
    assert_eq!(parse_c_double_prefix("-"), None);
    assert_eq!(parse_c_double_prefix("."), None);
    assert_eq!(parse_c_double_prefix("+."), None);
    assert_eq!(parse_c_double_prefix("-e5"), None);
    assert_eq!(parse_c_double_prefix("abc"), None);
}

#[test]
fn scanner_single_line_tokenizing_truncation_and_invalid_numeric() {
    let io = IoHandle::new();
    let mut st = GameState::new(1, false, "  SrScAn   3.5abc   abcdefghijklmno  ");

    assert_eq!(poll_ready(scan(&mut st, &io)), ScanToken::Alpha);
    assert_eq!(st.citem, "srscan");

    assert_eq!(poll_ready(scan(&mut st, &io)), ScanToken::Real);
    assert!((st.aaitem - 3.5).abs() < 1e-12);

    // Remaining "abc" after "3.5" is read as Alpha
    assert_eq!(poll_ready(scan(&mut st, &io)), ScanToken::Alpha);
    assert_eq!(st.citem, "abc");

    // 9-character truncation
    assert_eq!(poll_ready(scan(&mut st, &io)), ScanToken::Alpha);
    assert_eq!(st.citem, "abcdefghi");
    assert_eq!(st.citem.len(), 9);

    // Trailing whitespace on consumed line -> Eol without reading another line
    assert_eq!(poll_ready(scan(&mut st, &io)), ScanToken::Eol);

    // Bare "+", "-", "." return Eol and chew the rest of the line
    for bad in ["+ extra", "- extra", ". extra"] {
        st.line = bad.to_string();
        st.line_pos = 0;
        st.consumed_on_line = false;
        assert_eq!(poll_ready(scan(&mut st, &io)), ScanToken::Eol);
        assert!(st.line.is_empty());
        assert!(!st.consumed_on_line);
    }
}

#[test]
fn scanner_chew_vs_chew2_and_multiline_via_session() {
    // chew() clears line and sets consumed_on_line = false, so next scan prompts for a new line.
    // chew2() clears line and sets consumed_on_line = true, so next scan returns Eol immediately
    // without prompting (matching C's linep != line when trailing newline was consumed).
    let io = IoHandle::new();
    let mut st = GameState::new(1, false, "");
    st.line = "yes".to_string();
    st.line_pos = 3;
    st.chew2();
    assert_eq!(poll_ready(scan(&mut st, &io)), ScanToken::Eol);
    assert!(!st.consumed_on_line);

    // Multi-line input via TrekSession: "tournament 1" followed by separate lines
    let mut session = TrekSession::new(1, false, "");
    let r1 = session.step("tournament", false);
    assert!(r1.output.contains("Type in tournament number-"));
    assert_eq!(r1.prompt_type, PromptType::Line);

    let r2 = session.step("1", false);
    assert!(r2.output.contains("Short, Medium, or Long"));

    let r3 = session.step("short", false);
    assert!(r3.output.contains("Novice, Fair, Good, Expert, or Emeritus"));

    // Single-line multi-token cmdline initializes everything in one go
    let mut session_one = TrekSession::new(1, false, "tournament 1 short novice mypass");
    assert!(session_one.hud().in_game);
    assert_eq!(session_one.hud().stardate as i32, 3100);

    // Multi-token command on one line (e.g. "srscan no" vs "srscan\n")
    let res = session_one.step("srscan no", false);
    assert!(!res.output.contains("Stardate"));
    assert!(res.output.contains("COMMAND> "));
}

#[test]
fn scanner_isit_abbreviations_and_priority_order() {
    let mut st = GameState::default();
    st.citem = "sr".to_string();
    assert!(st.isit("srscan"));
    assert!(!st.isit("status"));
    assert!(!st.isit("s"));

    st.citem.clear();
    assert!(st.isit(""));
    assert!(!st.isit("srscan"));

    // Verify COMMANDS partitioning: 0..29 allow prefixes, 29..37 require exact match.
    assert_eq!(COMMANDS[28], "score");
    assert_eq!(COMMANDS[29], "abandon");
    assert_eq!(COMMANDS[36], "help");

    let mut session = TrekSession::new(1, false, "tournament 1 short expert mypass");

    // Prefix priority tests among 0..29:
    // "s" -> srscan (0)
    let r = session.step("s no", false);
    assert!(r.output.contains("1 2 3 4 5 6 7 8 9 10"));

    // "sh" -> shields (5) before shuttle (18)
    let r = session.step("sh down", false);
    assert!(r.output.contains("Shields already down.") || r.output.contains("Shields lowered."));

    // "st" -> status (12)
    let r = session.step("st", false);
    assert!(r.output.contains("Stardate") && !r.output.contains("1 2 3 4 5 6 7 8 9 10"));

    // "c" -> chart (8) before crystals/computer/commands/cloak/capture
    let r = session.step("c", false);
    assert!(r.output.contains("STAR CHART FOR THE KNOWN GALAXY"));

    // "co" -> computer (22) before commands (23)
    let r = session.step("co", false);
    assert!(r.output.contains("Destination quadrant and/or sector?"));
    let _ = session.step("invalid", false);

    // "comm" -> commands (23)
    let r = session.step("comm", false);
    assert!(r.output.contains("SRSCAN") && r.output.contains("HELP"));

    // "r" -> rest (10), "req" -> request (20), "rep" -> report (21)
    let r = session.step("r 0", false);
    assert!(!r.output.contains("UNRECOGNIZED"));
    let r = session.step("req da", false);
    assert!(r.output.contains("Stardate"));
    let r = session.step("rep", false);
    assert!(r.output.contains("short expert game"));

    // Exact-match commands (29..37): abbreviations MUST fail
    for abbr in ["ab", "aba", "de", "dest", "fr", "freez", "dea", "death", "cal", "qu", "qui", "he", "hel"] {
        let r = session.step(abbr, false);
        assert!(
            r.output.contains("UNRECOGNIZED COMMAND."),
            "abbreviation {abbr:?} should NOT match exact-only commands, got: {}",
            r.output
        );
    }

    // "ca" matches "capture" (idx 27 < 29), whereas "cal" does not match "call" (idx 34 >= 29)
    let r = session.step("ca", false);
    assert!(
        !r.output.contains("UNRECOGNIZED COMMAND."),
        "ca should match capture (idx 27)"
    );

    // Exact match for "help" works
    let r = session.step("help quit", false);
    assert!(r.output.contains("Mnemonic:  QUIT"));

    // Exact match for "quit" exits the game (tournament mode asks to record score first)
    let r = session.step("quit", false);
    assert!(r.output.contains("Do you want your score recorded?"));
    let r2 = session.step("no", false);
    assert!(r2.output.contains("Do you want to play again?"));
}
