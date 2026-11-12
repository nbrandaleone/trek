use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

use trek::moving::timwrp;
use trek::{
    GameState, IoHandle, MacOsRng, PromptType, TrekSession, DCOMPTR, DIMPULS, DRADIO, DSHUTTL,
    DWARPEN, IHBLANK, IHDOCKED, IHDOT, IHGREEN, IHK, IHSTAR, IHWEB,
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
    st.a.basex = 0;
    st.a.basey = 0;
    st.a.plnetx = 0;
    st.a.plnety = 0;
    st.future.fill(1e30);
    st.skip_pauses = true;
}

fn find_raw_seed_below(threshold: f64) -> u32 {
    for s in 1u32..100_000 {
        let mut r = MacOsRng::from_raw(s);
        if r.rand_f64() < threshold {
            return s;
        }
    }
    unreachable!()
}

fn find_raw_seed_above(threshold: f64) -> u32 {
    for s in 1u32..500_000 {
        let mut r = MacOsRng::from_raw(s);
        if r.rand_f64() > threshold {
            return s;
        }
    }
    unreachable!()
}

#[test]
fn warp_and_impulse_manual_coordfixed_and_automatic_navigation() {
    // 1. Manual move without -f: (0.2, -0.1) moves +2 in secty and +1 in sectx
    let mut s = TrekSession::new(1, false, "tournament 1 short novice testpass");
    s.with_state_mut(|st| {
        st.a.quadx = 4;
        st.a.quady = 4;
        st.a.sectx = 5;
        st.a.secty = 5;
        clear_quad_keep_ship(st);
    });
    let r = s.step("move manual 0.2 -0.1", false);
    assert!(r.output.contains("Helmsman Sulu- \"Aye, Sir.\""));
    assert_eq!(r.hud.sectx, 6);
    assert_eq!(r.hud.secty, 7);

    // 2. Manual move WITH -f (coordfixed): (0.2, 0.1) moves +2 in sectx and +1 in secty
    let mut s_cf = TrekSession::new(1, true, "tournament 1 short novice testpass");
    s_cf.with_state_mut(|st| {
        st.a.quadx = 4;
        st.a.quady = 4;
        st.a.sectx = 5;
        st.a.secty = 5;
        clear_quad_keep_ship(st);
    });
    let r_cf = s_cf.step("move manual 0.2 0.1", false);
    assert_eq!(r_cf.hud.sectx, 7);
    assert_eq!(r_cf.hud.secty, 6);

    // 3. Automatic sector-only move
    let r_auto_sec = s.step("move automatic 3 4", false);
    assert!(r_auto_sec.output.contains("COMMAND> "));
    assert_eq!(r_auto_sec.hud.sectx, 3);
    assert_eq!(r_auto_sec.hud.secty, 4);

    // 4. Automatic quadrant & sector move
    s.with_state_mut(|st| {
        st.d.galaxy[5][5] = 0;
        st.d.newstuf[5][5] = 0;
    });
    let r_auto_q = s.step("move automatic 5 5 2 8", false);
    assert!(r_auto_q.output.contains("Entering Quadrant 5 - 5"));
    assert_eq!(r_auto_q.hud.quadx, 5);
    assert_eq!(r_auto_q.hud.quady, 5);
    assert_eq!(r_auto_q.hud.sectx, 2);
    assert_eq!(r_auto_q.hud.secty, 8);

    // 5. Computer damaged forces manual movement only (chews extra args on line and prompts)
    s.with_state_mut(|st| st.damage[DCOMPTR] = 2.0);
    let r_cdam = s.step("move 0.1 0.0", false);
    assert!(r_cdam.output.contains("Computer damaged; manual movement only"));
    assert!(r_cdam.output.contains("X and Y displacements- "));
    let r_cdam2 = s.step("0.1 0.0", false);
    assert_eq!(r_cdam2.hud.secty, 9);
}

#[test]
fn impulse_movement_errors_energy_and_low_time_abort() {
    let mut s = TrekSession::new(1, false, "tournament 1 short novice testpass");
    s.with_state_mut(|st| {
        st.a.quadx = 4;
        st.a.quady = 4;
        st.a.sectx = 5;
        st.a.secty = 5;
        clear_quad_keep_ship(st);
    });

    // Normal impulse move: dist = 0.2 -> energy -= 20 + 20 = 40, time = 0.2 / 0.095
    let e_before = s.hud().energy;
    let r1 = s.step("impulse manual 0.2 0.0", false);
    assert_eq!(r1.hud.sectx, 5);
    assert_eq!(r1.hud.secty, 7);
    assert!((e_before - r1.hud.energy - 40.0).abs() < 1.0);

    // Low remaining time confirmation -> decline aborts
    s.with_state_mut(|st| {
        st.d.remtime = 1.0;
        st.d.remres = 1.0 * ((st.d.remkl + 4 * st.d.remcom) as f64);
    });
    let r2 = s.step("impulse manual 0.3 0.0", false);
    assert!(r2.output.contains("0.95 sectors per stardate"));
    let r2_no = s.step("no", false);
    assert_eq!(r2_no.hud.secty, 7);

    // Insufficient energy (> 30 but < 20 + 100*dist)
    s.with_state_mut(|st| st.a.energy = 35.0);
    let r3 = s.step("impulse manual 0.5 0.0", false);
    assert!(r3.output.contains("require 20.0 units to engage"));
    assert!(r3.output.contains("maximum of"));

    // Energy <= 30 -> "useless"
    s.with_state_mut(|st| st.a.energy = 25.0);
    let r4 = s.step("impulse", false);
    assert!(r4.output.contains("They are, therefore, useless."));

    // Impulse engines damaged
    s.with_state_mut(|st| {
        st.a.energy = 5000.0;
        st.damage[DIMPULS] = 3.0;
    });
    let r5 = s.step("impulse", false);
    assert!(r5.output.contains("The impulse engines are damaged, Sir."));
}

#[test]
fn setwrp_bounds_and_damaged_warp_caps() {
    let mut s = TrekSession::new(1, false, "tournament 1 short novice testpass");

    // Below 1
    let r_low = s.step("warp 0.5", false);
    assert!(r_low.output.contains("We can't go below warp 1"));
    assert!((r_low.hud.warp_factor - 5.0).abs() < 1e-12);

    // Above 10
    let r_hi = s.step("warp 11", false);
    assert!(r_hi.output.contains("Our top speed is warp 10"));

    // Valid safe speed <= 6
    let r_6 = s.step("warp 6", false);
    assert!(r_6.output.contains("Warp factor 6.0, Captain."));
    assert!((r_6.hud.warp_factor - 6.0).abs() < 1e-12);

    // 7..9 warning
    let r_7 = s.step("warp 7.5", false);
    assert!(r_7.output.contains("maximum safe speed is warp 6"));

    // 8..9.9 warning
    let r_9 = s.step("warp 9", false);
    assert!(r_9.output.contains("our engines may not take it"));

    // Warp 10
    let r_10 = s.step("warp 10", false);
    assert!(r_10.output.contains("Aye, Captain, we'll try it."));

    // Damaged warp (0 < damage <= 10): caps warp setting > 4 and refuses move at warp > 4
    s.with_state_mut(|st| st.damage[DWARPEN] = 5.0);
    let r_cap_set = s.step("warp 5", false);
    assert!(r_cap_set.output.contains("right now we can only go warp 4"));

    let r_cap_move = s.step("move manual 0.1 0.0", false);
    assert!(r_cap_move.output.contains("I can only give you warp 4"));

    // Setting warp 4 succeeds while damage <= 10
    let r_w4 = s.step("warp 4", false);
    assert!((r_w4.hud.warp_factor - 4.0).abs() < 1e-12);

    // Damage > 10: warp engines inoperative
    s.with_state_mut(|st| st.damage[DWARPEN] = 10.5);
    let r_inop_set = s.step("warp 2", false);
    assert!(r_inop_set.output.contains("Warp engines inoperative."));
    let r_inop_move = s.step("move manual 0.1 0.0", false);
    assert!(r_inop_move.output.contains("The warp engines are damaged, Sir."));
}

#[test]
fn warp_above_6_blooey_and_time_warp_forward_and_backward() {
    // 1. Trigger blooey engine damage via warp 10
    let mut s = TrekSession::new(1, false, "tournament 1 short novice testpass");
    s.with_state_mut(|st| {
        st.a.quadx = 4;
        st.a.quady = 4;
        st.a.sectx = 5;
        st.a.secty = 5;
        clear_quad_keep_ship(st);
        st.a.warpfac = 10.0;
        st.a.wfacsq = 100.0;
        // First rand_f64() < prob (blooey), second rand_f64() for dist factor, third rand_f64() >= 0.5*dist (no twarp)
        // Raw seed 1 gives tiny rand_f64 (~7.8e-6) then ~0.1315 (still < 0.5*dist? wait: dist becomes ~0.1315*2=0.26, 0.5*0.26=0.13 < 0.1315? Let's test!).
        st.rng = MacOsRng::from_raw(1);
    });
    let r = s.step("move manual 0.2 0.0", false);
    assert!(r.output.contains("The warp engines are damaged."));
    assert!(r.output.contains("reduce speed to warp 4."));
    s.with_state(|st| assert!(st.damage[DWARPEN] > 0.0));

    // 2. Direct timwrp() forward in time (when snap == 0)
    let io = IoHandle::new();
    let mut st = GameState::new(1, false, "");
    st.a.ship = 'E';
    st.a.quadx = 2;
    st.a.quady = 2;
    st.a.sectx = 5;
    st.a.secty = 5;
    st.a.intime = 7.0;
    st.d.date = 3100.0;
    st.d.snap = 0;
    st.future[trek::FTBEAM] = 3105.0;
    st.damage[DRADIO] = 1.0;
    st.rng = MacOsRng::from_raw(10_000);
    poll_ready(timwrp(&mut st, &io));
    assert!(st.output_buf.contains("***TIME WARP ENTERED."));
    assert!(st.output_buf.contains("traveling forward in time"));
    assert!(st.a.time > 0.0);
    assert!(st.future[trek::FTBEAM] > 3105.0);
    assert!(st.damage[DRADIO] > 1.0);

    // 3. Direct timwrp() backward in time (snap == 1, rand_f64() < 0.5) + Galileo consistency + chart memory
    let mut st2 = GameState::new(1, false, "");
    st2.a.ship = 'E';
    st2.a.quadx = 2;
    st2.a.quady = 2;
    st2.a.sectx = 5;
    st2.a.secty = 5;
    st2.a.intime = 7.0;
    st2.a.inplan = 1;
    st2.a.iscraft = 1; // Galileo is aboard
    st2.d.date = 3103.0;
    st2.d.snap = 1;
    st2.snapsht = st2.d.clone();
    st2.snapsht.date = 3101.0;
    st2.snapsht.plnets[1].known = 2; // In snapshot, Galileo was on planet 1!
    st2.starch[3][3] = 1005; // Frozen chart entry (> 1)
    st2.d.galaxy[3][3] = 5;
    st2.damage[DRADIO] = 0.0;
    st2.rng = MacOsRng::from_raw(find_raw_seed_below(0.4));

    poll_ready(timwrp(&mut st2, &io));
    assert!(st2.output_buf.contains("traveling backwards in time 2.00 stardates."));
    assert!((st2.d.date - 3101.0).abs() < 1e-12);
    assert_eq!(st2.d.snap, 0);
    // Galileo disappeared because snapshot says it was on planet surface!
    assert!(st2.output_buf.contains("Galileo has disappeared"));
    assert_eq!(st2.a.iscraft, 0);
    assert!(st2.output_buf.contains("reconstructed a correct star chart from memory"));
    assert_eq!(st2.starch[3][3], 1);

    // Conversely, if iscraft == 0 and snapshot has known != 2 and shuttle not destroyed, Galileo reappears!
    st2.d.date = 3103.0;
    st2.d.snap = 1;
    st2.snapsht.date = 3102.0;
    st2.snapsht.plnets[1].known = 1;
    st2.damage[DSHUTTL] = 0.0;
    st2.a.iscraft = 0;
    st2.rng = MacOsRng::from_raw(find_raw_seed_below(0.4));
    st2.output_buf.clear();
    poll_ready(timwrp(&mut st2, &io));
    assert!(st2.output_buf.contains("Galileo has reappeared in the dock"));
    assert_eq!(st2.a.iscraft, 1);
}

#[test]
fn negative_energy_barrier_three_strikes_destroys_ship() {
    let mut s = TrekSession::new(1, false, "tournament 1 short novice testpass");
    s.with_state_mut(|st| {
        st.a.quadx = 1;
        st.a.quady = 1;
        st.a.sectx = 1;
        st.a.secty = 5;
        clear_quad_keep_ship(st);
        st.a.warpfac = 2.0;
        st.a.wfacsq = 4.0;
    });

    // Strike 1: move upward across quadx=1 boundary (manual without -f: deltax=0, deltay=0.5)
    let r1 = s.step("move manual 0.0 0.5", false);
    assert!(r1.output.contains("NEGATIVE ENERGY BARRIER"));
    assert!(r1.output.contains("THE THIRD TIME YOU TRY THIS"));
    assert!(!r1.hud.alldone);
    s.with_state(|st| assert_eq!(st.a.nkinks, 1));

    // Strike 2: move back into edge
    s.with_state_mut(|st| {
        st.a.quadx = 1;
        st.a.sectx = 1;
        clear_quad_keep_ship(st);
    });
    let r2 = s.step("move manual 0.0 0.5", false);
    assert!(r2.output.contains("NEGATIVE ENERGY BARRIER"));
    assert!(!r2.hud.alldone);
    s.with_state(|st| assert_eq!(st.a.nkinks, 2));

    // Strike 3: FinType::Neg3
    s.with_state_mut(|st| {
        st.a.quadx = 1;
        st.a.sectx = 1;
        clear_quad_keep_ship(st);
    });
    let r3 = s.step("move manual 0.0 0.5", false);
    assert!(r3.output.contains("three attempts to cross the negative energy"));
    assert!(r3.output.contains("Your navigation is abominable."));
    assert!(r3.hud.alldone);
    assert_eq!(r3.prompt_type, PromptType::Line); // "Do you want to play again?"
}

#[test]
fn black_hole_collision_and_emergency_stop_on_star_and_web() {
    // 1. Black hole collision -> FinType::Hole
    let mut s = TrekSession::new(1, false, "tournament 1 short novice testpass");
    s.with_state_mut(|st| {
        st.a.quadx = 4;
        st.a.quady = 4;
        st.a.sectx = 5;
        st.a.secty = 5;
        clear_quad_keep_ship(st);
        st.quad[5][7] = IHBLANK;
    });
    let r_hole = s.step("move manual 0.3 0.0", false);
    assert!(r_hole.output.contains("pulled into black hole at Sector 5 - 7"));
    assert!(r_hole.output.contains("crushed into extremely dense matter"));
    assert!(r_hole.hud.alldone);

    // 2. Star collision -> emergency stop (and web collision)
    let mut s2 = TrekSession::new(1, false, "tournament 1 short novice testpass");
    s2.with_state_mut(|st| {
        st.a.quadx = 4;
        st.a.quady = 4;
        st.a.sectx = 5;
        st.a.secty = 5;
        clear_quad_keep_ship(st);
        st.quad[5][8] = IHSTAR;
        st.quad[5][3] = IHWEB;
    });
    let r_star = s2.step("move manual 0.4 0.0", false);
    assert!(r_star.output.contains("blocked by object at Sector 5 - 8"));
    assert!(r_star.output.contains("Emergency stop required"));
    assert_eq!(r_star.hud.sectx, 5);
    assert_eq!(r_star.hud.secty, 7);

    let r_web = s2.step("move manual -0.5 0.0", false);
    assert!(r_web.output.contains("encounters Tholian web at Sector 5 - 3"));
    assert_eq!(r_web.hud.sectx, 5);
    assert_eq!(r_web.hud.secty, 4);

    // 3. Emergency stop exhausting energy -> FinType::Nrg
    s2.with_state_mut(|st| {
        st.a.energy = 100.0;
        st.a.warpfac = 5.0;
        st.a.wfacsq = 25.0;
    });
    // stopegy = 50 * dist / (10 * dist / wfacsq) = 5 * 25 = 125 > 100!
    let r_nrg = s2.step("move manual -0.2 0.0", false);
    assert!(r_nrg.output.contains("Your energy supply is exhausted."));
    assert!(r_nrg.hud.alldone);
}

#[test]
fn enemy_ramming_damages_ship_and_kills_enemy() {
    let mut s = TrekSession::new(1, false, "tournament 1 short novice testpass");
    s.with_state_mut(|st| {
        st.a.quadx = 4;
        st.a.quady = 4;
        st.a.sectx = 5;
        st.a.secty = 5;
        clear_quad_keep_ship(st);
        st.quad[5][7] = IHK;
        st.kx[1] = 5;
        st.ky[1] = 7;
        st.kpower[1] = 350.0;
        st.kdist[1] = 2.0;
        st.kavgd[1] = 2.0;
        st.a.nenhere = 1;
        st.a.klhere = 1;
        st.d.galaxy[4][4] = 100;
        st.d.remkl = 2; // not last Klingon
        st.a.shldup = 1;
    });

    let r = s.step("move manual 0.3 0.0", false);
    assert!(r.output.contains("***COLLISION IMMINENT."));
    assert!(r.output.contains("Enterprise rams Klingon at Sector 5 - 7"));
    assert!(r.output.contains("destroyed."));
    assert!(r.output.contains("heavily damaged."));
    assert!(r.output.contains("casualties."));
    assert!(r.output.contains("DEVICE            -REPAIR TIMES-"));
    assert!(!r.hud.shields_up);
    assert!(r.hud.casualties >= 10);
    assert_eq!(r.hud.klingons_left, 1);
    assert_eq!(r.hud.sectx, 5);
    assert_eq!(r.hud.secty, 7);
}

#[test]
fn dock_resupply_prisoner_transfer_and_guards() {
    let mut s = TrekSession::new(1, false, "tournament 1 short novice testpass");
    // Tournament 1 starts at (2, 1) with a starbase in the quadrant, let's place base adjacent at (7, 8)
    s.with_state_mut(|st| {
        clear_quad_keep_ship(st);
        st.a.basex = 7;
        st.a.basey = 8;
        st.quad[7][8] = 'B';
        st.a.condit = IHGREEN;
        st.a.energy = 1200.0;
        st.a.shield = 400.0;
        st.a.torps = 2;
        st.a.lsupres = 1.0;
        st.d.brigfree = st.a.brigcapacity - 42;
        st.a.stdamtim = 3099.0;
        st.future[trek::FCDBAS] = 3102.0;
        st.a.batx = 7;
        st.a.baty = 7;
    });

    // Can't dock while cloaked
    s.with_state_mut(|st| st.a.iscloaked = true);
    let r_clk = s.step("dock", false);
    assert!(r_clk.output.contains("You cannot dock while cloaked."));
    s.with_state_mut(|st| st.a.iscloaked = false);

    // Can't dock while in orbit
    s.with_state_mut(|st| st.a.inorbit = 1);
    let r_orb = s.step("dock", false);
    assert!(r_orb.output.contains("You must first leave standard orbit."));
    s.with_state_mut(|st| st.a.inorbit = 0);

    // Successful dock
    let r = s.step("dock", false);
    assert!(r.output.contains("Docked."));
    assert!(r.output.contains("42 captured Klingons transferred to base."));
    assert!(r.output.contains("important message from the starbase"));
    assert!(r.output.contains("Starbase in  Quadrant 7 - 7 is currently under attack."));
    assert_eq!(r.hud.condition, "DOCKED");
    assert_eq!(r.hud.energy as i32, 5000);
    assert_eq!(r.hud.shield as i32, 2500);
    assert_eq!(r.hud.torps, 10);
    assert!((r.hud.life_support_reserves - 4.0).abs() < 1e-12);
    assert_eq!(r.hud.kcaptured, 42);
    assert_eq!(r.hud.brig_captives, 0);

    // Already docked
    let r_again = s.step("dock", false);
    assert!(r_again.output.contains("Already docked."));

    // Not adjacent to base
    s.with_state_mut(|st| {
        st.a.condit = IHGREEN;
        st.a.basex = 1;
        st.a.basey = 1;
    });
    let r_far = s.step("dock", false);
    assert!(r_far.output.contains("not adjacent to base."));
}

#[test]
fn call_help_dematerialize_success_and_materialization_failure() {
    // 1. Error guards: docked, radio damaged, no bases, landed
    let mut s = TrekSession::new(1, false, "tournament 1 short novice testpass");
    s.with_state_mut(|st| st.a.condit = IHDOCKED);
    assert!(s.step("call", false).output.contains("already docked"));

    s.with_state_mut(|st| {
        st.a.condit = IHGREEN;
        st.damage[DRADIO] = 1.0;
    });
    assert!(s.step("call", false).output.contains("Subspace radio damaged."));

    s.with_state_mut(|st| {
        st.damage[DRADIO] = 0.0;
        st.d.rembase = 0;
    });
    assert!(s.step("call", false).output.contains("not getting any response"));

    s.with_state_mut(|st| {
        st.d.rembase = 2;
        st.a.landed = 1;
    });
    assert!(s.step("call", false).output.contains("must be aboard the Enterprise"));

    // 2. Successful call from a remote quadrant to nearest starbase
    s.with_state_mut(|st| {
        st.a.landed = -1;
        st.a.quadx = 3;
        st.a.quady = 1; // nearest base in T1 is (2, 1) at ddist = 10.0
        st.a.basex = 0;
        st.a.basey = 0;
        st.a.energy = 1000.0;
        // High rand_f64() > probf (~0.566) succeeds on 1st attempt!
        st.rng = MacOsRng::from_raw(find_raw_seed_above(0.95));
    });
    let r_ok = s.step("call", false);
    assert!(r_ok.output.contains("Starbase in Quadrant 2 - 1 responds--Enterprise dematerializes."));
    assert!(r_ok.output.contains("1st attempt to re-materialize Enterprise"));
    assert!(r_ok.output.contains("succeeds."));
    assert!(r_ok.output.contains("Captain, we made it!"));
    assert_eq!(r_ok.hud.condition, "DOCKED");
    assert_eq!(r_ok.hud.quadx, 2);
    assert_eq!(r_ok.hud.quady, 1);
    assert_eq!(r_ok.hud.energy as i32, 5000);
    s.with_state(|st| assert_eq!(st.a.nhelp, 1));

    // 3. Three failed rematerialization attempts -> FinType::Materialize
    let mut s_fail = TrekSession::new(1, false, "tournament 1 short novice testpass");
    s_fail.with_state_mut(|st| {
        st.a.quadx = 5;
        st.a.quady = 5;
        st.a.basex = 0;
        st.a.basey = 0;
        // Ensure 3 consecutive rand_f64() calls are all <= probf (ddist ~ 50 -> probf ~ 0.86)
        // Find raw seed where 3 draws are all < 0.5
        let mut chosen = 1u32;
        for cand in 1u32..50_000 {
            let mut r = MacOsRng::from_raw(cand);
            if r.rand_f64() < 0.5 && r.rand_f64() < 0.5 && r.rand_f64() < 0.5 {
                chosen = cand;
                break;
            }
        }
        st.rng = MacOsRng::from_raw(chosen);
    });
    let r_fail = s_fail.step("call", false);
    assert!(r_fail.output.contains("3rd attempt to re-materialize Enterprise"));
    assert!(r_fail.output.contains("Starbase was unable to re-materialize your starship."));
    assert!(r_fail.hud.alldone);
}
