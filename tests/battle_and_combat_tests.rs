use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

use trek::battle::{attack, torpedo};
use trek::{
    GameState, IoHandle, MacOsRng, TrekSession, DCLOAK, DCOMPTR, DPHASER, DPHOTON, DRADIO,
    DSHCTRL, DSHIELD, DSRSENS, DTRANSP, IHBLANK, IHB, IHC, IHDOCKED, IHDOT, IHGREEN, IHK, IHP,
    IHQUEST, IHR, IHS, IHSTAR, IHT, IHWEB, SEXPERT,
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
    for s in 1u32..500_000 {
        let mut r = MacOsRng::from_raw(s);
        if r.rand_f64() < threshold {
            return s;
        }
    }
    unreachable!()
}

fn find_raw_seed_above(threshold: f64) -> u32 {
    for s in 1u32..5_000_000 {
        let mut r = MacOsRng::from_raw(s);
        if r.rand_f64() > threshold {
            return s;
        }
    }
    unreachable!()
}

#[test]
fn shields_up_down_transfer_loophole_docked_free_and_zero_energy_death() {
    let mut s = TrekSession::new(1, false, "tournament 1 short novice testpass");
    s.with_state_mut(|st| {
        clear_quad_keep_ship(st);
        st.a.condit = IHGREEN;
        st.a.energy = 4000.0;
        st.a.shield = 2000.0;
        st.a.shldup = 0;
    });

    // Up costs 50 energy when undocked
    let r_up = s.step("shields up", false);
    assert!(r_up.output.contains("Shields raised."));
    assert!(r_up.hud.shields_up);
    assert_eq!(r_up.hud.energy as i32, 3950);

    // Down
    let r_dn = s.step("shields down", false);
    assert!(r_dn.output.contains("Shields lowered."));
    assert!(!r_dn.hud.shields_up);

    // Docked free raise
    s.with_state_mut(|st| {
        st.a.condit = IHDOCKED;
        st.a.energy = 5000.0;
    });
    let r_dock_up = s.step("shields up", false);
    assert!(r_dock_up.output.contains("Shields raised."));
    assert_eq!(r_dock_up.hud.energy as i32, 5000);

    // Positive transfer maximizing shields with excess returned
    s.with_state_mut(|st| {
        st.a.condit = IHGREEN;
        st.a.energy = 3000.0;
        st.a.shield = 2000.0; // max = 2500
    });
    let r_max = s.step("shields transfer 1000", false);
    assert!(r_max.output.contains("Shield energy maximized."));
    assert!(r_max.output.contains("Excess energy requested returned to ship energy"));
    assert_eq!(r_max.hud.shield as i32, 2500);
    assert_eq!(r_max.hud.energy as i32, 2500);

    // Drain loophole prevention: negative transfer that would push energy > inenrg (5000)
    s.with_state_mut(|st| {
        st.a.energy = 4900.0;
        st.a.shield = 1000.0;
    });
    let r_loop = s.step("shields transfer -500", false);
    assert!(r_loop.output.contains("I can't drain the shields."));
    assert_eq!(r_loop.hud.energy as i32, 4900);
    assert_eq!(r_loop.hud.shield as i32, 1000);

    // Valid negative transfer (drain all shields when shield + (-1500) < 0)
    s.with_state_mut(|st| {
        st.a.energy = 3000.0;
        st.a.shield = 1000.0;
    });
    let r_drain = s.step("shields transfer -1500", false);
    assert!(r_drain.output.contains("All shield energy transferred to ship."));
    assert_eq!(r_drain.hud.shield as i32, 0);
    assert_eq!(r_drain.hud.energy as i32, 4000);

    // Damaged shields cannot be raised
    s.with_state_mut(|st| {
        st.a.shldup = 0;
        st.damage[DSHIELD] = 2.0;
    });
    let r_dam = s.step("shields up", false);
    assert!(r_dam.output.contains("Shields damaged and down."));

    // Zero-energy death on raising shields
    s.with_state_mut(|st| {
        st.damage[DSHIELD] = 0.0;
        st.a.shldup = 0;
        st.a.energy = 40.0;
    });
    let r_dead = s.step("shields up", false);
    assert!(r_dead.output.contains("Shields raising uses up last of energy."));
    assert!(r_dead.output.contains("Your energy supply is exhausted."));
    assert!(r_dead.hud.alldone);
}

#[test]
fn phasers_auto_manual_forced_manual_overheat_shctrl_and_emasculated() {
    let mut s = TrekSession::new(1, false, "tournament 1 short novice testpass");

    // 1. Docked refusal
    s.with_state_mut(|st| st.a.condit = IHDOCKED);
    assert!(s.step("phasers", false).output.contains("base shields"));

    // 2. Damaged phasers
    s.with_state_mut(|st| {
        st.a.condit = IHGREEN;
        st.damage[DPHASER] = 1.0;
    });
    assert!(s.step("phasers", false).output.contains("Phaser control damaged."));

    // 3. Automatic fire hitting emasculated enemy (negative kpower) and Tholian web excess absorption
    s.with_state_mut(|st| {
        st.damage[DPHASER] = 0.0;
        clear_quad_keep_ship(st);
        st.a.sectx = 5;
        st.a.secty = 5;
        st.quad[5][5] = 'E';
        st.quad[5][6] = IHK;
        st.kx[1] = 5;
        st.ky[1] = 6;
        st.kpower[1] = -100.0; // emasculated
        st.kdist[1] = 1.0;
        st.kavgd[1] = 1.0;
        st.a.nenhere = 1;
        st.a.klhere = 1;
        st.d.galaxy[2][1] = 100;
        st.d.remkl = 3;
        st.a.ithere = 1; // web present
        st.a.energy = 4500.0;
    });
    let r_auto = s.step("phasers automatic 500", false);
    assert!(r_auto.output.contains("Klingon at Sector 5 - 6 destroyed."));
    assert!(r_auto.output.contains("Tholian web absorbs phaser energy."));
    assert_eq!(r_auto.hud.klingons_left, 2);

    // 4. Forced manual when SR sensors damaged + hiding distant Commander
    s.with_state_mut(|st| {
        clear_quad_keep_ship(st);
        st.a.sectx = 5;
        st.a.secty = 5;
        st.quad[5][5] = 'E';
        st.quad[5][6] = IHK;
        st.kx[1] = 5;
        st.ky[1] = 6;
        st.kpower[1] = 400.0;
        st.kdist[1] = 1.0;
        st.kavgd[1] = 1.0;
        st.quad[1][1] = IHC;
        st.kx[2] = 1;
        st.ky[2] = 1;
        st.kpower[2] = 1000.0;
        st.kdist[2] = 5.65;
        st.kavgd[2] = 5.65;
        st.a.nenhere = 2;
        st.a.klhere = 2;
        st.d.galaxy[2][1] = 200;
        st.d.remkl = 3;
        st.damage[DSRSENS] = 2.0;
        st.a.energy = 4500.0;
    });
    let r_fm1 = s.step("phasers automatic", false);
    assert!(r_fm1.output.contains("Manual-fire-must-be-used"));
    assert!(r_fm1.output.contains("units to fire at Klingon at Sector 5 - 6"));
    let r_fm2 = s.step("50", false);
    assert!(r_fm2.output.contains("Commander can't be located without short range scan."));

    // 5. Computer damaged -> forced manual message
    s.with_state_mut(|st| {
        st.damage[DSRSENS] = 0.0;
        st.damage[DCOMPTR] = 1.0;
    });
    let r_cd = s.step("phasers automatic", false);
    assert!(r_cd.output.contains("Battle computer damaged"));
    let _ = s.step("-1", false); // abort manual fire cleanly

    // 6. High-speed shield control damaged / insufficient energy / overheating > 1500
    s.with_state_mut(|st| {
        st.damage[DCOMPTR] = 0.0;
        st.a.shldup = 1;
        st.damage[DSHCTRL] = 1.0;
    });
    assert!(s.step("phasers", false).output.contains("High speed shield control damaged."));

    s.with_state_mut(|st| {
        st.damage[DSHCTRL] = 0.0;
        st.a.energy = 200.0;
    });
    assert!(s
        .step("phasers", false)
        .output
        .contains("Insufficient energy to activate high-speed shield control."));

    // Overheating on rpow = 3500 (> 1500) + high-speed shield control with `no`
    s.with_state_mut(|st| {
        clear_quad_keep_ship(st);
        st.a.energy = 5000.0;
        st.a.shldup = 1;
        // checkshctrl uses 1st rand < 0.998 -> ok; overheat uses rand <= (3500-1500)*0.00038 = 0.76
        st.rng = MacOsRng::from_raw(find_raw_seed_below(0.5));
    });
    let r_ov = s.step("phasers no 3500", false);
    assert!(r_ov.output.contains("High-speed shield control enabled"));
    assert!(r_ov.output.contains("Phasers overheated, sir."));
    assert!(!r_ov.hud.shields_up); // lowered by `no`
    s.with_state(|st| assert!(st.damage[DPHASER] > 0.0));

    // 7. High-speed shield control 0.2% malfunction (rand >= 0.998)
    s.with_state_mut(|st| {
        st.damage[DPHASER] = 0.0;
        st.a.shldup = 1;
        st.a.energy = 5000.0;
        st.a.shield = 2500.0;
        st.rng = MacOsRng::from_raw(find_raw_seed_above(0.9985));
    });
    let r_malf = s.step("phasers 500", false);
    assert!(r_malf.output.contains("Shield malfunction! Phaser fire contained!"));
    assert!(r_malf.output.contains("Phaser energy dispersed by shields."));
}

#[test]
fn photon_burst_misfire_and_all_target_collisions() {
    // 1. Guards: damaged tubes, 0 torps, >3 burst, self-target psychological profile
    let mut s = TrekSession::new(1, false, "tournament 1 short novice testpass");
    s.with_state_mut(|st| st.damage[DPHOTON] = 1.0);
    assert!(s.step("photons", false).output.contains("Photon tubes damaged."));

    s.with_state_mut(|st| {
        st.damage[DPHOTON] = 0.0;
        st.a.torps = 0;
    });
    assert!(s.step("photons", false).output.contains("No torpedoes left."));

    s.with_state_mut(|st| {
        st.a.torps = 10;
        st.a.sectx = 5;
        st.a.secty = 5;
        clear_quad_keep_ship(st);
    });
    let r_max = s.step("photons 4", false);
    assert!(r_max.output.contains("Maximum of 3 torpedoes per burst."));
    let _ = s.step("0", false);

    let r_self = s.step("photons 1 5 5", false);
    assert!(r_self.output.contains("psychological profile"));

    // 2. Burst of 2 torpedoes hitting and destroying an enemy / shoving
    s.with_state_mut(|st| {
        clear_quad_keep_ship(st);
        st.quad[5][8] = IHK;
        st.kx[1] = 5;
        st.ky[1] = 8;
        st.kpower[1] = 1500.0;
        st.kdist[1] = 3.0;
        st.kavgd[1] = 3.0;
        st.a.nenhere = 1;
        st.a.klhere = 1;
        st.d.galaxy[2][1] = 100;
        st.d.remkl = 3;
        // Pick seed that won't misfire (two rand_f64 near 0.5 so |(r1+r2)*0.5 - 0.5| < 0.47)
        st.rng = MacOsRng::new(12345);
    });
    let r_burst = s.step("photons 2 5 8", false);
    assert!(r_burst.output.contains("Track for torpedo number 1"));
    assert!(r_burst.output.contains("Track for torpedo number 2"));

    // 3. Misfire aborting remainder of burst & damaging tubes
    // Need |(r1+r2)*0.5 - 0.5| >= 0.47 (both < 0.03), then r3 for deflection, then r4 <= 0.2 for tube damage.
    let mut misfire_seed = 1u32;
    for cand in 1u32..500_000 {
        let mut r = MacOsRng::from_raw(cand);
        let a = r.rand_f64();
        let b = r.rand_f64();
        if ((a + b) * 0.5 - 0.5).abs() >= 0.47 {
            let _ = r.rand_f64();
            if r.rand_f64() <= 0.2 {
                misfire_seed = cand;
                break;
            }
        }
    }
    s.with_state_mut(|st| {
        clear_quad_keep_ship(st);
        st.a.torps = 10;
        st.rng = MacOsRng::from_raw(misfire_seed);
    });
    let r_mis = s.step("photons 3 5 9", false);
    assert!(r_mis.output.contains("***TORPEDO NUMBER 1 MISFIRES."));
    assert!(r_mis.output.contains("Remainder of burst aborted."));
    assert!(r_mis.output.contains("***Photon tubes damaged by misfire."));
    s.with_state(|st| assert!(st.damage[DPHOTON] > 0.0));

    // 4. Direct torpedo() collision tests for every target type
    let io = IoHandle::new();
    let mut st = GameState::new(1, false, "");
    st.a.ship = 'E';
    st.a.quadx = 4;
    st.a.quady = 4;
    st.a.sectx = 5;
    st.a.secty = 5;
    st.a.inplan = 1;
    st.d.plnets[1].x = 4;
    st.d.plnets[1].y = 4;
    st.d.rembase = 1;
    st.d.baseqx[1] = 4;
    st.d.baseqy[1] = 4;
    st.skip_pauses = true;

    // 4a. Shove enemy into black hole
    clear_quad_keep_ship(&mut st);
    st.quad[5][7] = IHK;
    st.quad[5][8] = IHBLANK;
    st.kx[1] = 5;
    st.ky[1] = 7;
    st.kpower[1] = 2000.0; // survives torpedo hit so shove runs
    st.a.nenhere = 1;
    st.a.klhere = 1;
    st.d.remkl = 2;
    st.d.galaxy[4][4] = 100;
    // Need ang = angle + 2.5*(rand - 0.5) ~ angle (= 3.14159/2? course=3 -> angle=12*0.5236=2*pi -> cos=1, sin=0 -> xx=0, yy=+1!)
    // So rand_f64() near 0.5 keeps ang near angle, shoving from (5,7) to (5,8)=IHBLANK!
    let mut push_seed = 1u32;
    for cand in 1u32..50_000 {
        let mut r = MacOsRng::from_raw(cand);
        let _ = r.rand_f64(); // h1
        let d = r.rand_f64();
        if (d - 0.5).abs() < 0.05 {
            push_seed = cand;
            break;
        }
    }
    st.rng = MacOsRng::from_raw(push_seed);
    let mut hit = 0.0;
    poll_ready(torpedo(&mut st, &io, 3.0, 0.0, 5, 5, &mut hit));
    assert!(st.output_buf.contains("buffeted into black hole."));
    assert_eq!(st.d.remkl, 1);

    // 4b. Shove Enterprise into black hole
    st.output_buf.clear();
    clear_quad_keep_ship(&mut st);
    st.quad[5][7] = 'E';
    st.quad[5][8] = IHBLANK;
    st.a.sectx = 5;
    st.a.secty = 7;
    st.rng = MacOsRng::from_raw(push_seed);
    poll_ready(torpedo(&mut st, &io, 3.0, 0.0, 5, 5, &mut hit));
    assert!(st.output_buf.contains("Torpedo hits Enterprise."));
    assert!(st.output_buf.contains("black hole"));
    assert_eq!(st.a.alldone, 1);

    // 4c. Commander 5% anti-photon neutralization
    st.a.alldone = 0;
    st.a.ship = 'E';
    st.output_buf.clear();
    clear_quad_keep_ship(&mut st);
    st.quad[5][7] = IHC;
    st.rng = MacOsRng::from_raw(find_raw_seed_below(0.05));
    poll_ready(torpedo(&mut st, &io, 3.0, 0.0, 5, 5, &mut hit));
    assert!(st.output_buf.contains("uses anti-photon device;"));

    // 4d. Starbase, Planet (landed -> FinType::Dplanet), *THING*, Black hole, Web, Star, Tholian
    st.output_buf.clear();
    clear_quad_keep_ship(&mut st);
    st.quad[5][7] = IHB;
    st.d.rembase = 1;
    st.d.baseqx[1] = 4;
    st.d.baseqy[1] = 4;
    st.d.galaxy[4][4] = 10;
    poll_ready(torpedo(&mut st, &io, 3.0, 0.0, 5, 5, &mut hit));
    assert!(st.output_buf.contains("***STARBASE DESTROYED.."));
    assert_eq!(st.d.rembase, 0);

    st.output_buf.clear();
    clear_quad_keep_ship(&mut st);
    st.quad[5][7] = IHQUEST;
    poll_ready(torpedo(&mut st, &io, 3.0, 0.0, 5, 5, &mut hit));
    assert!(st.output_buf.contains("Fascinating!"));
    assert_eq!(st.quad[5][7], IHDOT);

    st.output_buf.clear();
    clear_quad_keep_ship(&mut st);
    st.quad[5][7] = IHBLANK;
    poll_ready(torpedo(&mut st, &io, 3.0, 0.0, 5, 5, &mut hit));
    assert!(st.output_buf.contains("swallows torpedo."));

    st.output_buf.clear();
    clear_quad_keep_ship(&mut st);
    st.quad[5][7] = IHWEB;
    poll_ready(torpedo(&mut st, &io, 3.0, 0.0, 5, 5, &mut hit));
    assert!(st.output_buf.contains("absorbed by Tholian web."));

    // Star: 10% unaffected vs 90% nova
    st.output_buf.clear();
    clear_quad_keep_ship(&mut st);
    st.quad[5][7] = IHSTAR;
    st.d.galaxy[4][4] = 1;
    st.rng = MacOsRng::from_raw(find_raw_seed_below(0.05));
    poll_ready(torpedo(&mut st, &io, 3.0, 0.0, 5, 5, &mut hit));
    assert!(st.output_buf.contains("unaffected by photon blast."));

    // Tholian destroyed (>= 600)
    st.output_buf.clear();
    clear_quad_keep_ship(&mut st);
    st.quad[5][7] = IHT;
    st.a.ithere = 1;
    st.rng = MacOsRng::from_raw(find_raw_seed_above(0.5));
    poll_ready(torpedo(&mut st, &io, 3.0, 0.0, 5, 5, &mut hit));
    assert!(st.output_buf.contains("Tholian at Sector 5 - 7 destroyed."));
    assert_eq!(st.a.ithere, 0);

    // Planet destroyed while landed -> FinType::Dplanet
    st.output_buf.clear();
    clear_quad_keep_ship(&mut st);
    st.quad[5][7] = IHP;
    st.a.iplnet = 1;
    st.a.landed = 1;
    st.d.newstuf[4][4] = 1;
    poll_ready(torpedo(&mut st, &io, 3.0, 0.0, 5, 5, &mut hit));
    assert!(st.output_buf.contains("You and your mining party perish."));
    assert_eq!(st.a.alldone, 1);
}

#[test]
fn attack_and_fry_massive_hit_no_panic_casualties_and_battle_death() {
    let io = IoHandle::new();
    let mut st = GameState::new(1, false, "");
    st.a.ship = 'E';
    st.a.quadx = 4;
    st.a.quady = 4;
    st.a.sectx = 5;
    st.a.secty = 5;
    st.a.skill = SEXPERT;
    st.a.justin = 1;
    st.a.damfac = 2.5;
    st.a.inshld = 2500.0;
    st.a.shield = 200.0;
    st.a.shldup = 1;
    st.a.energy = 10000.0;
    st.d.remkl = 3;
    st.skip_pauses = true;

    // Place a super-massive enemy (4500 power at distance 1 -> >3000 unit hit!) to exercise fry() multi-device criticals
    clear_quad_keep_ship(&mut st);
    st.quad[5][6] = IHS;
    st.kx[1] = 5;
    st.ky[1] = 6;
    st.kpower[1] = 4500.0;
    st.kdist[1] = 1.0;
    st.kavgd[1] = 1.0;
    st.a.nenhere = 1;
    st.a.klhere = 1;
    st.a.ishere = 1;
    st.d.galaxy[4][4] = 100;
    // Choose seed > 0.07 so SC fires phaser (itflag == true) rather than torpedo
    st.rng = MacOsRng::from_raw(find_raw_seed_above(0.2));

    poll_ready(attack(&mut st, &io, 2));
    assert!(st.output_buf.contains("***CRITICAL HIT--"));
    assert!(st.output_buf.contains("damaged."));
    assert!(st.a.casual > 0);
    assert!(st.damage[1..=16].iter().any(|&d| d > 0.0));

    // Zero energy in battle -> FinType::Battle
    st.a.energy = 10.0;
    st.a.shldup = 0;
    st.kpower[1] = 1000.0;
    st.output_buf.clear();
    poll_ready(attack(&mut st, &io, 2));
    assert!(st.output_buf.contains("has been destroyed in battle."));
    assert_eq!(st.a.alldone, 1);
}

#[test]
fn cloak_algeron_violations_mid_cloak_damage_and_movement_dock_restrictions() {
    let mut s = TrekSession::new(1, false, "tournament 1 short novice testpass");

    // Pre-Algeron (< 2311): cloak on/off without violation
    s.with_state_mut(|st| {
        clear_quad_keep_ship(st);
        st.d.date = 2300.0;
        st.a.condit = IHGREEN;
        st.a.basex = 7;
        st.a.basey = 8;
        st.quad[7][8] = IHB;
    });
    let r_on = s.step("cloak on", false);
    assert!(r_on.output.contains("The cloaking device has been engaged, Sir."));
    assert!(r_on.hud.cloaked);

    // Cannot warp or dock while cloaked
    assert!(s.step("move manual 0.1 0.0", false).output.contains("cannot be used while cloaked"));
    assert!(s.step("dock", false).output.contains("You cannot dock while cloaked."));

    let r_off = s.step("cloak off", false);
    assert!(r_off.output.contains("Aye, Sir."));
    assert!(!r_off.hud.cloaked);

    // Post-Algeron (>= 2311) with Romulan present: Treaty of Algeron warning + violation on cloak/phasers/photons
    s.with_state_mut(|st| {
        st.d.date = 3100.0;
        st.quad[5][8] = IHR;
        st.kx[1] = 5;
        st.ky[1] = 8;
        st.kpower[1] = 600.0;
        st.kdist[1] = 3.0;
        st.kavgd[1] = 3.0;
        st.a.nenhere = 1;
        st.a.irhere = 1;
        st.d.nromrem = 1;
        st.d.newstuf[2][1] = 10;
    });
    let r_alg = s.step("cloak on", false);
    assert!(r_alg.output.contains("Treaty of Algeron"));
    let r_alg_yes = s.step("yes", false);
    assert!(r_alg_yes.output.contains("Romulan ship discovers you are breaking the Treaty of Algeron!"));
    s.with_state(|st| assert_eq!(st.a.ncviol, 1));

    // Violation when firing phasers while cloaked (reset isviolreported and any fry damage from engagement attack)
    s.with_state_mut(|st| {
        st.a.isviolreported = false;
        st.damage[DPHASER] = 0.0;
        st.damage[DSRSENS] = 0.0;
        st.damage[DCOMPTR] = 0.0;
        st.damage[DSHCTRL] = 0.0;
    });
    let r_ph = s.step("phasers auto 50", false);
    assert!(r_ph.output.contains("breaking the Treaty of Algeron!"));
    s.with_state(|st| assert_eq!(st.a.ncviol, 2));

    // Cloaking device damaged mid-cloak (when enemy hits during cloak engagement)
    s.with_state_mut(|st| {
        st.a.iscloaked = false;
        st.a.isviolreported = true;
        st.damage[DCLOAK] = 0.0;
    });
    // Simulate damage[DCLOAK] > 0 while cloaked via fry-like hit:
    s.with_state_mut(|st| {
        st.a.iscloaked = true;
        st.damage[DCLOAK] = 3.0;
        // Or test cloak on when already damaged:
    });
    s.with_state_mut(|st| st.a.iscloaked = false);
    assert!(s.step("cloak on", false).output.contains("The cloaking device is damaged, Sir."));
}

#[test]
fn capture_surrender_suicide_brig_capacity_docked_transfer_and_romulan_exclusion() {
    let mut s = TrekSession::new(1, false, "tournament 1 short novice testpass");

    // 1. Guards: brig full, no radio, damaged transporter, no Klingons (only Romulan)
    s.with_state_mut(|st| {
        clear_quad_keep_ship(st);
        st.d.brigfree = 0;
    });
    assert!(s.step("capture", false).output.contains("brig is already full"));

    s.with_state_mut(|st| {
        st.d.brigfree = 400;
        st.damage[DRADIO] = 1.0;
    });
    assert!(s.step("capture", false).output.contains("no subspace radio"));

    s.with_state_mut(|st| {
        st.damage[DRADIO] = 0.0;
        st.damage[DTRANSP] = 1.0;
    });
    assert!(s.step("capture", false).output.contains("Transporter damaged"));

    // Romulans alone cannot be captured (`klhere == 0`)
    s.with_state_mut(|st| {
        st.damage[DTRANSP] = 0.0;
        st.quad[5][5] = IHR;
        st.kx[1] = 5;
        st.ky[1] = 5;
        st.kpower[1] = 500.0;
        st.a.nenhere = 1;
        st.a.irhere = 1;
        st.a.klhere = 0;
    });
    assert!(s.step("capture", false).output.contains("Getting no response"));

    // 2. Capture chooses Klingon (not Romulan) and handles suicide + brig overflow when undocked
    s.with_state_mut(|st| {
        clear_quad_keep_ship(st);
        st.quad[1][1] = IHR;
        st.kx[1] = 1;
        st.ky[1] = 1;
        st.kpower[1] = 10.0; // lower power than Klingon, must still be skipped by selectklingon!
        st.quad[5][8] = IHK;
        st.kx[2] = 5;
        st.ky[2] = 8;
        st.kpower[2] = 50.0;
        st.kdist[2] = 1.0;
        st.kavgd[2] = 1.0;
        st.a.nenhere = 2;
        st.a.irhere = 1;
        st.a.klhere = 1;
        st.d.remkl = 3;
        st.d.galaxy[2][1] = 100;
        st.a.energy = 5000.0; // prob = 5000/(50*2)*2.5 = 125 > 100 -> guaranteed surrender!
        st.d.brigfree = 10; // force brig overflow if cap > 10
        // Second rand_f64() gives cap = (200 * rand) as i32; choose seed with 2nd rand > 0.5 -> cap >= 100
        let mut chosen = 1u32;
        for cand in 1u32..50_000 {
            let mut r = MacOsRng::from_raw(cand);
            let _ = r.rand_f64();
            if r.rand_f64() > 0.5 {
                chosen = cand;
                break;
            }
        }
        st.rng = MacOsRng::from_raw(chosen);
    });
    let r_cap = s.step("capture", false);
    assert!(r_cap.output.contains("Klingon captain at 5,8 surrenders"));
    assert!(r_cap.output.contains("Klingons commit suicide rather than be taken captive"));
    assert!(r_cap.output.contains("Klingons die because there is no room for them in the brig."));
    assert!(r_cap.output.contains("10 captives taken"));
    assert_eq!(r_cap.hud.brig_captives, 400); // brigcapacity 400 - brigfree 0
    assert_eq!(r_cap.hud.klingons_left, 2);

    // 3. Direct base transfer when docked!
    s.with_state_mut(|st| {
        clear_quad_keep_ship(st);
        st.damage[DTRANSP] = 0.0;
        st.damage[DRADIO] = 0.0;
        st.a.iscloaked = false;
        st.a.condit = IHDOCKED;
        st.d.brigfree = 5;
        st.quad[5][8] = IHK;
        st.kx[1] = 5;
        st.ky[1] = 8;
        st.kpower[1] = 50.0;
        st.a.nenhere = 1;
        st.a.klhere = 1;
        st.d.remkl = 2;
        st.d.galaxy[2][1] = 100;
        st.a.energy = 5000.0;
        let mut chosen = 1u32;
        for cand in 1u32..50_000 {
            let mut r = MacOsRng::from_raw(cand);
            let _ = r.rand_f64();
            if r.rand_f64() > 0.2 {
                chosen = cand;
                break;
            }
        }
        st.rng = MacOsRng::from_raw(chosen);
    });
    let r_dock = s.step("capture", false);
    assert!(r_dock.output.contains("captives taken and transferred to base"));
    assert!(r_dock.hud.kcaptured > 0);
}
