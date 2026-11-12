use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

use trek::ai::{movcom, scom};
use trek::battle::{attack, deadkl, selectklingon};
use trek::events::{events, snova};
use trek::finish::kaboom;
use trek::moving::{timwrp, warp};
use trek::wasm::{
    trek_alloc, trek_dealloc, trek_import_save, trek_init, trek_response_len, trek_response_ptr,
    trek_step,
};
use trek::{
    GameState, IoHandle, MacOsRng, TrekSession, DDRAY, DPHASER, DPHOTON, DRADIO, DSRSENS,
    DWARPEN, FCDBAS, FSCDBAS, FSCMOVE, IHBLANK, IHC, IHDOT, IHE, IHGREEN, IHK, IHQUEST, IHR,
    IHSTAR, IHT, IHWEB, SEXPERT, SNOVICE,
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

fn clear_quad_keep_ship(st: &mut GameState) {
    for x in 1..=10 {
        for y in 1..=10 {
            st.quad[x][y] = IHDOT;
        }
    }
    st.quad[st.a.sectx as usize][st.a.secty as usize] = st.a.ship;
    st.a.nenhere = 0;
    st.a.klhere = 0;
    st.a.comhere = 0;
    st.a.ishere = 0;
    st.a.irhere = 0;
    st.a.ithere = 0;
    st.kx = [0; 21];
    st.ky = [0; 21];
    st.kpower = [0.0; 21];
    st.kdist = [0.0; 21];
    st.kavgd = [0.0; 21];
}

fn find_raw_seed_below(threshold: f64) -> u32 {
    for s in 1..500_000u32 {
        let mut r = MacOsRng::from_raw(s);
        if r.rand_f64() < threshold {
            return s;
        }
    }
    panic!("no raw seed below {threshold}");
}

fn find_raw_seed_above(threshold: f64) -> u32 {
    for s in 1..500_000u32 {
        let mut r = MacOsRng::from_raw(s);
        if r.rand_f64() > threshold {
            return s;
        }
    }
    panic!("no raw seed above {threshold}");
}

fn read_wasm_response_json() -> String {
    unsafe {
        let ptr = trek_response_ptr();
        let len = trek_response_len();
        assert!(!ptr.is_null() && len > 0);
        let bytes = std::slice::from_raw_parts(ptr, len);
        String::from_utf8(bytes.to_vec()).expect("valid utf8 wasm response")
    }
}

fn wasm_import_save(name: &str, data: &str) -> u32 {
    let n_bytes = name.as_bytes();
    let d_bytes = data.as_bytes();
    let n_ptr = trek_alloc(n_bytes.len());
    let d_ptr = trek_alloc(d_bytes.len());
    unsafe {
        std::ptr::copy_nonoverlapping(n_bytes.as_ptr(), n_ptr, n_bytes.len());
        std::ptr::copy_nonoverlapping(d_bytes.as_ptr(), d_ptr, d_bytes.len());
        let rc = trek_import_save(n_ptr, n_bytes.len(), d_ptr, d_bytes.len());
        trek_dealloc(n_ptr, n_bytes.len());
        trek_dealloc(d_ptr, d_bytes.len());
        rc
    }
}

fn wasm_step_str(input: &str, is_key: bool) -> String {
    let bytes = input.as_bytes();
    let ptr = trek_alloc(bytes.len());
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
        let rc = trek_step(ptr, bytes.len(), if is_key { 1 } else { 0 });
        trek_dealloc(ptr, bytes.len());
        assert_eq!(rc, 1);
    }
    read_wasm_response_json()
}

/// BUG-01: Super-Commander diagonal move in `scom()` previously computed
/// `iqy = state.d.isy + ideltax` instead of `state.d.isy + ideltay`,
/// corrupting off-diagonal movement (such as `(+1, -1)` becoming `(+1, +1)`).
#[test]
fn regression_bug_01_scom_diagonal_move_y_axis() {
    let io = IoHandle::new();
    let mut st = GameState::new(1, false, "");
    st.skip_pauses = true;
    st.a.skill = SNOVICE;
    st.a.quadx = 1;
    st.a.quady = 1;
    st.a.indate = 3100.0;
    st.d.date = 3105.0;
    st.d.killk = 5; // flag = false so scom moves toward nearest starbase
    st.d.nscrem = 1;
    st.d.isx = 4;
    st.d.isy = 6;
    st.d.rembase = 1;
    st.d.baseqx[1] = 6;
    st.d.baseqy[1] = 4;
    st.d.remkl = 2;
    st.d.galaxy[4][6] = 100;
    st.d.galaxy[6][4] = 10;

    let mut ipage = 1;
    poll_ready(scom(&mut st, &io, &mut ipage));

    // ideltax = +1, ideltay = -1 -> must move from (4, 6) to (5, 5), NOT (5, 7)
    assert_eq!((st.d.isx, st.d.isy), (5, 5));
}

/// BUG-02: Starbase distance bubble sort in `scom()` looped `1..(remb - 1)`,
/// skipping the last starbase pair (`remb == 2` never sorted at all).
#[test]
fn regression_bug_02_scom_starbase_sort_off_by_one() {
    let io = IoHandle::new();
    let mut st = GameState::new(1, false, "");
    st.skip_pauses = true;
    st.a.skill = SNOVICE;
    st.a.quadx = 5;
    st.a.quady = 5;
    st.a.indate = 3100.0;
    st.d.date = 3105.0;
    st.d.killk = 5; // flag = false
    st.d.nscrem = 1;
    st.d.isx = 1;
    st.d.isy = 1;
    st.d.rembase = 2;
    // Base 1 is far away at (8, 8); Base 2 is adjacent at (1, 2)
    st.d.baseqx[1] = 8;
    st.d.baseqy[1] = 8;
    st.d.baseqx[2] = 1;
    st.d.baseqy[2] = 2;
    st.d.remkl = 2;
    st.d.galaxy[1][1] = 100;
    st.d.galaxy[8][8] = 10;
    st.d.galaxy[1][2] = 10;

    let mut ipage = 1;
    poll_ready(scom(&mut st, &io, &mut ipage));

    // Must sort Base 2 first and move onto (1, 2) instead of moving toward (8, 8) -> (2, 2)
    assert_eq!((st.d.isx, st.d.isy), (1, 2));
    assert_eq!(st.a.isatb, 1);
    assert!(st.future[FSCDBAS] <= st.d.date + 3.0);
}

/// BUG-03: `scom()` base-attack guard checked `isx != batx && isy != baty`,
/// refusing to attack a base that shared either X or Y with `batx, baty`.
#[test]
fn regression_bug_03_scom_base_attack_guard_or_condition() {
    let io = IoHandle::new();
    let mut st = GameState::new(1, false, "");
    st.skip_pauses = true;
    st.a.skill = SNOVICE;
    st.a.quadx = 8;
    st.a.quady = 8;
    st.a.indate = 3100.0;
    st.d.date = 3105.0;
    st.d.killk = 5; // flag = false
    st.d.nscrem = 1;
    st.d.isx = 3;
    st.d.isy = 3;
    st.d.rembase = 1;
    st.d.baseqx[1] = 3;
    st.d.baseqy[1] = 2;
    // Active regular Commander attack at a different base sharing X=3
    st.a.batx = 3;
    st.a.baty = 7;
    st.future[FCDBAS] = st.d.date + 1.5;
    st.d.remkl = 3;
    st.d.remcom = 1;
    st.d.cx[1] = 3;
    st.d.cy[1] = 7;
    st.d.galaxy[3][3] = 100;
    st.d.galaxy[3][2] = 10;
    st.d.galaxy[3][7] = 110;

    let mut ipage = 1;
    poll_ready(scom(&mut st, &io, &mut ipage));

    assert_eq!((st.d.isx, st.d.isy), (3, 2));
    assert_eq!(st.a.isatb, 1);
    assert!(st.future[FSCDBAS] <= st.d.date + 5.0);
}

/// BUG-04: `movcom()` incremented `i` unconditionally when `movebaddy()`
/// triggered `tryexit()`, skipping the enemy swap-removed into slot `i`.
#[test]
fn regression_bug_04_movcom_does_not_skip_enemy_after_tryexit() {
    let io = IoHandle::new();
    let mut st = GameState::new(1, false, "");
    st.skip_pauses = true;
    st.a.skill = SEXPERT;
    st.a.quadx = 4;
    st.a.quady = 4;
    st.a.sectx = 5;
    st.a.secty = 5;
    st.a.energy = 5000.0;
    st.a.shldup = 1;
    st.a.torps = 10;
    st.damage[DPHASER] = 0.0;
    st.damage[DPHOTON] = 0.0;
    st.d.remkl = 2;
    clear_quad_keep_ship(&mut st);

    // Weak IHK #1 at corner (1,1) flees across quadrant border via tryexit -> (3,3)
    st.quad[1][1] = IHK;
    st.kx[1] = 1;
    st.ky[1] = 1;
    st.kpower[1] = 100.0;
    st.kdist[1] = (16.0_f64 + 16.0).sqrt();

    // Strong IHK #2 at (2,8) advances toward Enterprise at (5,5); previously skipped!
    st.quad[2][8] = IHK;
    st.kx[2] = 2;
    st.ky[2] = 8;
    st.kpower[2] = 3000.0;
    st.kdist[2] = (9.0_f64 + 9.0).sqrt();

    st.a.nenhere = 2;
    st.a.klhere = 2;
    st.d.galaxy[4][4] = 200;
    st.d.galaxy[3][3] = 0;

    poll_ready(movcom(&mut st, &io));

    // Enemy #1 fled out of quadrant; remaining enemy #2 MUST have moved from (2,8)
    assert_eq!(st.a.nenhere, 1);
    assert_ne!((st.kx[1], st.ky[1]), (2, 8));
}

/// BUG-05: `tryexit()` ran its Commander escape branch in a catch-all `else`,
/// corrupting `d.cx/cy` and clearing `a.comhere` when an ordinary `IHK` fled.
#[test]
fn regression_bug_05_tryexit_ihk_escape_preserves_commander_state() {
    let io = IoHandle::new();
    let mut st = GameState::new(1, false, "");
    st.skip_pauses = true;
    st.a.skill = SEXPERT;
    st.a.quadx = 4;
    st.a.quady = 4;
    st.a.sectx = 5;
    st.a.secty = 5;
    st.a.energy = 5000.0;
    st.a.shldup = 1;
    st.a.torps = 10;
    st.damage[DPHASER] = 0.0;
    st.damage[DPHOTON] = 0.0;
    st.d.remkl = 2;
    st.d.remcom = 1;
    st.d.cx[1] = 4;
    st.d.cy[1] = 4;
    clear_quad_keep_ship(&mut st);

    // Slot 1: weak IHK at (1,1) -> flees to (3,3)
    st.quad[1][1] = IHK;
    st.kx[1] = 1;
    st.ky[1] = 1;
    st.kpower[1] = 100.0;

    // Slot 2: IHC at (8,8) -> stays in quadrant (4,4)
    st.quad[8][8] = IHC;
    st.kx[2] = 8;
    st.ky[2] = 8;
    st.kpower[2] = 2000.0;

    st.a.comhere = 1;
    st.a.klhere = 2;
    st.a.nenhere = 2;
    st.d.galaxy[4][4] = 200;
    st.d.galaxy[3][3] = 0;

    poll_ready(movcom(&mut st, &io));

    // IHK escaped to (3,3), while IHC remains at (4,4) with comhere == 1 and cx/cy == (4,4)
    assert_eq!(st.d.galaxy[3][3], 100);
    assert_eq!(st.d.cx[1], 4);
    assert_eq!(st.d.cy[1], 4);
    assert_eq!(st.a.comhere, 1);
    assert_eq!(st.a.nenhere, 1);
    assert_eq!(st.quad[st.kx[1] as usize][st.ky[1] as usize], IHC);
}

/// BUG-06: Backward time warp in `timwrp()` previously set `future[FSCMOVE] = 0.2777`
/// instead of `state.d.date + 0.2777`, jumping `date` thousands of stardates backward.
#[test]
fn regression_bug_06_backward_timwrp_sets_relative_fscmove() {
    let io = IoHandle::new();
    let mut s = TrekSession::new(1, false, "tournament 1 short novice testpass");
    s.with_state_mut(|st| {
        st.skip_pauses = true;
        st.d.date = 3102.0;
        st.d.snap = 1;
        st.d.nscrem = 1;
        st.d.isx = 8;
        st.d.isy = 8;
        st.d.galaxy[8][8] = 100;
        st.snapsht = st.d.clone();
        st.snapsht.date = 3100.0;
        st.snapsht.nscrem = 1;
        st.snapsht.snap = 1;
        st.rng = MacOsRng::from_raw(find_raw_seed_below(0.25)); // triggers backward warp (< 0.5)

        poll_ready(timwrp(st, &io));

        assert_eq!(st.d.date, 3100.0);
        assert!((st.future[FSCMOVE] - 3100.2777).abs() < 1e-6);

        // Ensure next events() tick does NOT jump date backward to ~0.2777
        st.a.time = 0.5;
        poll_ready(events(st, &io));
        assert!(st.d.date >= 3100.0 && st.d.date <= 3101.0);
    });
}

/// BUG-07: `warp()` previously continued into `timwrp()`, `blooey` engine damage,
/// and `ididit = 1` after `finish(FinType::Nrg)` had already ended the game.
#[test]
fn regression_bug_07_warp_stops_after_energy_exhaustion_finish() {
    let io = IoHandle::new();
    let mut st = GameState::new(1, false, "");
    st.skip_pauses = true;
    clear_quad_keep_ship(&mut st);
    st.a.quadx = 4;
    st.a.quady = 4;
    st.a.sectx = 5;
    st.a.secty = 5;
    st.quad[5][5] = IHE;
    st.a.warpfac = 10.0;
    st.a.wfacsq = 100.0;
    st.a.dist = 0.5;
    st.a.direc = 3.0;
    st.a.energy = 100.0; // dist * warpfac^3 = 500 > 100 -> FinType::Nrg
    st.damage[DWARPEN] = 0.0;
    st.a.time = 0.0;
    st.a.ididit = 0;

    // Seed where blooey/twarp would have fired if warp() didn't return early after finish()
    st.rng = MacOsRng::from_raw(find_raw_seed_below(0.2));

    // Use i == 2 (forced/tractor-beam style warp) to exercise the post-lmove energy <= 0 branch
    poll_ready(warp(&mut st, &io, 2));

    assert_eq!(st.a.alldone, 1);
    assert!(st.output_buf.contains("TOTAL SCORE"));
    assert!(!st.output_buf.contains("TIME WARP ENTERED"));
    assert!(!st.output_buf.contains("warp engines are damaged"));
    assert_eq!(st.damage[DWARPEN], 0.0);
    assert_eq!(st.a.time, 0.0);
    assert_eq!(st.a.ididit, 0);
}

/// BUG-08: Supernova in the Enterprise's own quadrant killing the last Klingon
/// without destroying the Enterprise previously skipped `finish(FinType::Won)`
/// and caused `events()` to divide by zero (`remkl + 4*remcom == 0`).
#[test]
fn regression_bug_08_snova_in_ship_quadrant_killing_last_klingon_wins_cleanly() {
    let io = IoHandle::new();
    let mut st = GameState::new(1, false, "");
    st.skip_pauses = true;
    clear_quad_keep_ship(&mut st);
    st.a.quadx = 4;
    st.a.quady = 4;
    st.a.sectx = 1;
    st.a.secty = 1;
    st.quad[1][1] = IHE;

    // Star at (9,9) and last Klingon at (10,10); Enterprise at (1,1) is > 2.1 distance^2 away
    st.quad[9][9] = IHSTAR;
    st.quad[10][10] = IHK;
    st.kx[1] = 10;
    st.ky[1] = 10;
    st.kpower[1] = 300.0;
    st.a.nenhere = 1;
    st.a.klhere = 1;
    st.d.remkl = 1;
    st.d.remcom = 0;
    st.d.galaxy[4][4] = 101;

    poll_ready(snova(&mut st, &io, 4, 4));

    assert_eq!(st.d.remkl, 0);
    assert_eq!(st.a.alldone, 1);
    assert_eq!(st.a.gamewon, 1);
    assert!(st.output_buf.contains("You have smashed the Klingon invasion fleet"));
    assert!(st.d.remtime.is_finite());

    // Even if events() is invoked afterward, remtime must not become NaN/Inf
    st.a.time = 0.0;
    poll_ready(events(&mut st, &io));
    assert!(st.d.remtime.is_finite());
}

/// BUG-09: `attack()` previously printed stray debug output `Hit <x> energy <y>`.
#[test]
fn regression_bug_09_attack_no_stray_hit_energy_debug_print() {
    let io = IoHandle::new();
    let mut st = GameState::new(1, false, "");
    st.skip_pauses = true;
    clear_quad_keep_ship(&mut st);
    st.a.quadx = 4;
    st.a.quady = 4;
    st.a.sectx = 5;
    st.a.secty = 5;
    st.quad[5][5] = IHE;
    st.a.shldup = 0;
    st.a.energy = 3000.0;
    st.a.skill = SNOVICE;
    st.quad[5][7] = IHK;
    st.kx[1] = 5;
    st.ky[1] = 7;
    st.kpower[1] = 400.0;
    st.kdist[1] = 2.0;
    st.kavgd[1] = 2.0;
    st.a.nenhere = 1;
    st.a.klhere = 1;
    st.d.remkl = 1;
    st.d.galaxy[4][4] = 100;
    st.rng = MacOsRng::from_raw(find_raw_seed_above(0.1)); // phaser hit on Enterprise

    poll_ready(attack(&mut st, &io, 2));

    assert!(st.output_buf.contains("unit hit"));
    assert!(!st.output_buf.contains("Hit "));
    assert!(!st.output_buf.contains("energy "));
}

/// BUG-10: `attack()` cached `1..=nenhere`, reading ghost `(0, 0)` after an enemy
/// torpedo killed another enemy mid-loop; `deadkl()` also lacked underflow guards.
#[test]
fn regression_bug_10_attack_loop_mid_kill_no_ghost_torpedo_and_deadkl_guard() {
    let io = IoHandle::new();
    let mut st = GameState::new(1, false, "");
    st.skip_pauses = true;
    clear_quad_keep_ship(&mut st);
    st.a.quadx = 4;
    st.a.quady = 4;
    st.a.sectx = 5;
    st.a.secty = 7;
    st.quad[5][7] = IHE;
    st.a.skill = SNOVICE; // skips movcom
    st.a.justin = 1;
    st.a.energy = 5000.0;
    st.d.remkl = 2;

    // Enemy #1 (Romulan IHR so r <= 0.3 triggers torpedo at k=2) at (5,5)
    // Enemy #2 at (5,6) lies directly on the straight torpedo path to Enterprise (5,7)
    st.quad[5][5] = IHR;
    st.kx[1] = 5;
    st.ky[1] = 5;
    st.kpower[1] = 500.0;
    st.kdist[1] = 2.0;
    st.kavgd[1] = 2.0;

    st.quad[5][6] = IHK;
    st.kx[2] = 5;
    st.ky[2] = 6;
    st.kpower[2] = 300.0;
    st.kdist[2] = 1.0;
    st.kavgd[2] = 1.0;

    st.a.nenhere = 2;
    st.a.klhere = 1;
    st.a.irhere = 1;
    st.d.galaxy[4][4] = 100;

    // Find a seed where enemy #1 fires a torpedo (rand < 0.3) and trajectory hits (5,6)
    let mut found = false;
    for s in 1..2000 {
        let mut r = MacOsRng::from_raw(s);
        let r0 = r.rand_f64();
        let r1 = r.rand_f64();
        let r2 = r.rand_f64();
        let tr = (r1 + r2) * 0.5 - 0.5;
        if r0 <= 0.3 && tr.abs() < 0.05 {
            st.rng = MacOsRng::from_raw(s);
            found = true;
            break;
        }
    }
    assert!(found);

    poll_ready(attack(&mut st, &io, 2));

    // Enemy #2 was killed mid-loop; must NOT read (0,0) for a ghost second torpedo
    assert_eq!(st.a.nenhere, 1);
    assert!(!st.output_buf.contains("Sector 0 - 0"));
    assert!(!st.output_buf.contains("0 - 0"));

    // Guard check: deadkl() on empty/out-of-bounds nenhere == 0 must not underflow/panic
    st.a.nenhere = 0;
    poll_ready(deadkl(&mut st, &io, 1, 1, IHK, 1, 1));
    assert_eq!(st.a.nenhere, 0);
}

/// BUG-11: Emasculated Klingons have negative `kpower`, which previously gave negative
/// surrender probability in `capture()` and inverted selection order in `selectklingon()`.
#[test]
fn regression_bug_11_emasculated_negative_kpower_capture_and_selectklingon() {
    // Part A: selectklingon() picks smallest |kpower| (100 < 600), not -600 < -100
    let mut st = GameState::new(1, false, "");
    st.a.nenhere = 2;
    st.kpower[1] = -600.0;
    st.kpower[2] = -100.0;
    assert_eq!(selectklingon(&st), 2);

    // Part B: capture() succeeds against emasculated Klingon with kpower = -50.0
    let mut session = TrekSession::new(1, false, "tournament 1 short novice testpass");
    session.with_state_mut(|s| {
        s.skip_pauses = true;
        clear_quad_keep_ship(s);
        s.a.condit = IHGREEN;
        s.a.energy = 5000.0;
        s.damage[DRADIO] = 0.0;
        s.d.brigfree = 400;
        s.quad[5][6] = IHK;
        s.kx[1] = 5;
        s.ky[1] = 6;
        s.kpower[1] = -50.0;
        s.kdist[1] = 1.0;
        s.kavgd[1] = 1.0;
        s.a.nenhere = 1;
        s.a.klhere = 1;
        s.d.remkl = 3;
        s.d.galaxy[s.a.quadx as usize][s.a.quady as usize] = 100;
    });
    let r = session.step("capture", false);
    assert!(
        r.output.contains("surrenders") || r.output.contains("commits suicide"),
        "expected emasculated Klingon to surrender (or commit suicide), got: {}",
        r.output
    );
    assert!(!r.output.contains("refuses to surrender"));
}

/// BUG-12: `kaboom()` incremented `l` unconditionally after calling `deadkl()`,
/// skipping the second enemy shifted into slot `l`.
#[test]
fn regression_bug_12_kaboom_destroys_consecutive_enemies_for_martyr_win() {
    let io = IoHandle::new();
    let mut st = GameState::new(1, false, "");
    st.skip_pauses = true;
    clear_quad_keep_ship(&mut st);
    st.a.quadx = 4;
    st.a.quady = 4;
    st.a.sectx = 5;
    st.a.secty = 5;
    st.quad[5][5] = IHE;
    st.a.energy = 1000.0; // whammo = 25000.0
    st.d.remkl = 2;

    // Two adjacent Klingons both well within blast lethal range (300 * 2 = 600 <= 25000)
    st.quad[5][6] = IHK;
    st.kx[1] = 5;
    st.ky[1] = 6;
    st.kpower[1] = 300.0;
    st.kdist[1] = 2.0;

    st.quad[5][7] = IHK;
    st.kx[2] = 5;
    st.ky[2] = 7;
    st.kpower[2] = 300.0;
    st.kdist[2] = 2.0;

    st.a.nenhere = 2;
    st.a.klhere = 2;
    st.d.galaxy[4][4] = 200;

    poll_ready(kaboom(&mut st, &io));

    assert_eq!(st.d.remkl, 0);
    assert_eq!(st.a.nenhere, 0);
    assert_eq!(st.a.alldone, 1);
    assert_eq!(st.a.gamewon, 1);
    assert!(st.output_buf.contains("martyr and a hero"));
}

/// BUG-13: `deathray()` fell through after `finish(FinType::Won)` to print Spock's
/// post-game death ray status line and mutate `damage[DDRAY]`.
#[test]
fn regression_bug_13_deathray_returns_immediately_on_victory() {
    let mut s = TrekSession::new(1, false, "tournament 1 short novice testpass");
    s.with_state_mut(|st| {
        st.skip_pauses = true;
        clear_quad_keep_ship(st);
        st.a.ship = IHE;
        st.quad[5][6] = IHK;
        st.kx[1] = 5;
        st.ky[1] = 6;
        st.kpower[1] = 300.0;
        st.a.nenhere = 1;
        st.a.klhere = 1;
        st.d.remkl = 1;
        st.damage[DDRAY] = 0.0;
        st.d.galaxy[st.a.quadx as usize][st.a.quady as usize] = 100;
        st.rng = MacOsRng::from_raw(find_raw_seed_above(0.35)); // triggers success branch (> 0.30)
    });

    let r_prompt = s.step("deathray", false);
    assert!(r_prompt.output.contains("are you sure this is wise?"));
    let r = s.step("y", false);

    assert!(r.hud.alldone);
    assert!(r.hud.gamewon);
    assert!(r.output.contains("TOTAL SCORE"));
    assert!(!r.output.contains("Experimental Death Ray' is still operational"));
    assert!(!r.output.contains("rendered dysfunctional"));
    s.with_state(|st| assert_eq!(st.damage[DDRAY], 0.0));
}

/// BUG-14: Web UI previously called `importAllSavedGames()` before `trek_init()`,
/// causing saved games to be discarded on initial load or New Game / Resume.
#[test]
fn regression_bug_14_wasm_import_save_and_web_init_ordering() {
    // 1. Verify WASM runtime rejects or loses saves imported before trek_init(),
    //    and succeeds when imported AFTER trek_init()
    let mut s = TrekSession::new(1, false, "tournament 1 short novice testpass");
    let r_freeze = s.step("freeze mygame", false);
    let saved_data = r_freeze
        .saves_updated
        .iter()
        .find(|e| e.name == "mygame.trk")
        .expect("freeze should export mygame.trk")
        .data
        .clone();

    // Initialize session 1 via C ABI and import save into it
    assert_eq!(trek_init(1, 0, std::ptr::null(), 0), 1);
    assert_eq!(wasm_import_save("mygame.trk", &saved_data), 1);

    // Re-initializing via trek_init() starts a fresh session (previously discarded imports!)
    assert_eq!(trek_init(1, 0, std::ptr::null(), 0), 1);
    // Import AFTER trek_init() — the fixed ordering in web/app.js
    assert_eq!(wasm_import_save("mygame.trk", &saved_data), 1);

    let r_json = wasm_step_str("frozen mygame", false);
    assert!(
        !r_json.contains("Can't find game file"),
        "expected frozen save to load cleanly, got: {r_json}"
    );
    assert!(
        r_json.contains("You are now playing a thawed") || r_json.contains("Password"),
        "expected frozen prompt or restore in response, got: {r_json}"
    );

    // 2. Verify web/app.js calls importAllSavedGames() AFTER trek_init(...) inside startNewGame()
    let app_js = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/web/app.js"))
        .expect("read web/app.js");
    let start_fn = app_js
        .split("function startNewGame()")
        .nth(1)
        .expect("startNewGame exists")
        .split("\nfunction ")
        .next()
        .expect("startNewGame body");
    let init_pos = start_fn
        .find("state.wasm.trek_init(")
        .expect("trek_init call in startNewGame");
    let import_pos = start_fn
        .find("importAllSavedGames()")
        .expect("importAllSavedGames call in startNewGame");
    assert!(
        import_pos > init_pos,
        "importAllSavedGames() must execute AFTER state.wasm.trek_init() in startNewGame()"
    );
}

/// BUG-15: Web UI Saves Manager `Insert` failed from `emexit`/exited state, and
/// `SYMBOL_NAMES`/CSS lacked sector symbols `'T'`, `'#'`, `'?'`, `'-'`.
#[test]
fn regression_bug_15_hud_sector_symbols_and_web_saves_insert_from_emexit() {
    // 1. Verify Rust HudState exposes 'T', '#', '?' and damaged-SR '-' correctly
    let mut s = TrekSession::new(1, false, "tournament 1 short novice testpass");
    s.with_state_mut(|st| {
        clear_quad_keep_ship(st);
        st.quad[1][2] = IHT;
        st.quad[1][3] = IHWEB;
        st.quad[1][4] = IHQUEST;
        st.quad[1][5] = IHSTAR;
        st.quad[1][6] = IHBLANK;
    });
    let r = s.step("status", false);
    assert_eq!(r.hud.quad_rows[0].as_bytes()[2], b'T');
    assert_eq!(r.hud.quad_rows[0].as_bytes()[4], b'#');
    assert_eq!(r.hud.quad_rows[0].as_bytes()[6], b'?');

    // Damaged SR sensors replace non-adjacent objects in HUD with '-'
    s.with_state_mut(|st| {
        st.damage[DSRSENS] = 5.0;
        st.a.sectx = 5;
        st.a.secty = 5;
    });
    let r_dmg = s.step("status", false);
    assert_eq!(r_dmg.hud.quad_rows[0].as_bytes()[2], b'-');

    // 2. Verify web/app.js handles emexit/exited in renderSavesList() Insert handler
    let app_js = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/web/app.js"))
        .expect("read web/app.js");
    assert!(app_js.contains("if (state.emexit || state.promptType === 'exited')"));
    assert!(app_js.contains("startNewGame();"));
    for sym_key in ["T: 'Tholian'", "'#': 'Tholian Web'", "'?': 'Space Thing'", "'-': 'Unscanned"] {
        assert!(
            app_js.contains(sym_key),
            "web/app.js SYMBOL_NAMES missing {sym_key}"
        );
    }

    // 3. Verify web/styles.css styles .quad-cell[data-sym=...] for T, #, ?, -
    let styles_css =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/web/styles.css"))
            .expect("read web/styles.css");
    for attr in [
        r##".quad-cell[data-sym="T"]"##,
        r##".quad-cell[data-sym="#"]"##,
        r##".quad-cell[data-sym="?"]"##,
        r##".quad-cell[data-sym="-"]"##,
    ] {
        assert!(
            styles_css.contains(attr),
            "web/styles.css missing selector {attr}"
        );
    }
}

/// BUG-16: `deadkl()` early return on `remkl == 0` skipped `nenhere` decrement
/// and `kx`/`ky` cleanup, causing infinite loops in `kaboom()` and `deathray()`
/// when destroying the final Klingon in the galaxy.
#[test]
fn regression_bug_16_deadkl_updates_nenhere_when_remkl_hits_zero() {
    let io = IoHandle::new();

    // 1. Direct deadkl() on the last Klingon (remkl 1 -> 0) must still decrement
    //    nenhere, zero kx[1]/ky[1]/kpower[1], and keep remtime finite.
    let mut st = GameState::new(1, false, "");
    st.skip_pauses = true;
    clear_quad_keep_ship(&mut st);
    st.a.quadx = 4;
    st.a.quady = 4;
    st.a.sectx = 5;
    st.a.secty = 5;
    st.quad[5][5] = IHE;
    st.quad[3][3] = IHK;
    st.kx[1] = 3;
    st.ky[1] = 3;
    st.kpower[1] = 300.0;
    st.kdist[1] = 2.0;
    st.kavgd[1] = 2.0;
    st.a.nenhere = 1;
    st.a.klhere = 1;
    st.d.remkl = 1;
    st.d.remcom = 0;
    st.d.remres = 10.0;
    st.d.remtime = 10.0;
    st.d.galaxy[4][4] = 100;

    poll_ready(deadkl(&mut st, &io, 3, 3, IHK, 3, 3));

    assert_eq!(st.d.remkl, 0);
    assert_eq!(st.a.nenhere, 0);
    assert_eq!(st.kx[1], 0);
    assert_eq!(st.ky[1], 0);
    assert_eq!(st.kpower[1], 0.0);
    assert_eq!(st.quad[3][3], IHDOT);
    assert!(st.d.remtime.is_finite());

    // Calling deadkl() on non-enemy (e.g. IHDOT) must not mutate nenhere
    st.a.nenhere = 2;
    poll_ready(deadkl(&mut st, &io, 3, 3, IHDOT, 3, 3));
    assert_eq!(st.a.nenhere, 2);

    // 2. kaboom() killing the single remaining Klingon terminates cleanly
    let mut st_kaboom = GameState::new(1, false, "");
    st_kaboom.skip_pauses = true;
    clear_quad_keep_ship(&mut st_kaboom);
    st_kaboom.a.quadx = 4;
    st_kaboom.a.quady = 4;
    st_kaboom.a.sectx = 5;
    st_kaboom.a.secty = 5;
    st_kaboom.quad[5][5] = IHE;
    st_kaboom.a.energy = 1000.0;
    st_kaboom.quad[3][3] = IHK;
    st_kaboom.kx[1] = 3;
    st_kaboom.ky[1] = 3;
    st_kaboom.kpower[1] = 300.0;
    st_kaboom.kdist[1] = 2.0;
    st_kaboom.a.nenhere = 1;
    st_kaboom.a.klhere = 1;
    st_kaboom.d.remkl = 1;
    st_kaboom.d.galaxy[4][4] = 100;

    poll_ready(kaboom(&mut st_kaboom, &io));

    assert_eq!(st_kaboom.d.remkl, 0);
    assert_eq!(st_kaboom.a.nenhere, 0);
    assert_eq!(st_kaboom.a.alldone, 1);
    assert_eq!(st_kaboom.a.gamewon, 1);
    assert!(st_kaboom.output_buf.contains("martyr and a hero"));

    // 3. deathray() killing the single remaining Klingon terminates cleanly
    let mut s = TrekSession::new(1, false, "tournament 1 short novice testpass");
    s.with_state_mut(|s_st| {
        s_st.skip_pauses = true;
        clear_quad_keep_ship(s_st);
        s_st.a.ship = IHE;
        s_st.quad[3][3] = IHK;
        s_st.kx[1] = 3;
        s_st.ky[1] = 3;
        s_st.kpower[1] = 300.0;
        s_st.a.nenhere = 1;
        s_st.a.klhere = 1;
        s_st.d.remkl = 1;
        s_st.damage[DDRAY] = 0.0;
        s_st.d.galaxy[s_st.a.quadx as usize][s_st.a.quady as usize] = 100;
        s_st.rng = MacOsRng::from_raw(find_raw_seed_above(0.35));
    });
    let _ = s.step("deathray", false);
    let r = s.step("y", false);
    assert!(r.hud.alldone);
    assert!(r.hud.gamewon);
    assert_eq!(r.hud.klingons_left, 0);
}
