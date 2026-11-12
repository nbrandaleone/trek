use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

use trek::ai::{movcom, movetho, scom};
use trek::events::{events, nova, snova};
use trek::finish::finish;
use trek::{
    FinType, GameState, IoHandle, MacOsRng, TrekSession, DDRAY, DDSP, DLIFSUP, DRADIO, DSHIELD,
    FBATTAK, FCDBAS, FDSPROB, FSCDBAS, FTBEAM, IHB, IHBLANK, IHC, IHDOCKED, IHDOT, IHK, IHP,
    IHSTAR, IHT, IHWEB, SEXPERT, SGOOD,
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
    for s in 1u32..500_000 {
        let mut r = MacOsRng::from_raw(s);
        if r.rand_f64() > threshold {
            return s;
        }
    }
    unreachable!()
}

#[test]
fn events_depletion_lifesup_repair_rates_and_radio_recovery() {
    let io = IoHandle::new();

    // 1. Resource depletion (FinType::Deplete) via events()
    let mut st = GameState::new(1, false, "");
    st.a.ship = 'E';
    st.a.alive = 1;
    st.a.inkling = 10;
    st.a.incom = 1;
    st.a.inresor = 100.0;
    st.d.date = 3100.0;
    st.d.remkl = 2; // 2 * 3 <= 10 -> guilty branch
    st.d.remcom = 0;
    st.d.remres = 1.0;
    st.d.remtime = 0.5;
    st.a.time = 1.0; // remres -= 2 * 1.0 = -1.0 <= 0
    st.skip_pauses = true;
    poll_ready(events(&mut st, &io));
    assert!(st.output_buf.contains("sentenced to death by slow torture."));
    assert_eq!(st.a.alive, 0);
    assert_eq!(st.a.alldone, 1);

    // Acquitted branch (remkl * 3 > inkling)
    let mut st_acq = GameState::new(1, false, "");
    st_acq.a.ship = 'E';
    st_acq.a.alive = 1;
    st_acq.a.inkling = 10;
    st_acq.d.remkl = 5; // 5 * 3 > 10
    st_acq.skip_pauses = true;
    poll_ready(finish(&mut st_acq, &io, FinType::Deplete));
    assert!(st_acq.output_buf.contains("aquitted."));
    assert_eq!(st_acq.a.alive, 1);

    // 2. Life support exhaustion (FinType::Lifesup) when undocked
    let mut st_ls = GameState::new(1, false, "");
    st_ls.a.ship = 'E';
    st_ls.a.alive = 1;
    st_ls.d.remkl = 3;
    st_ls.d.remres = 100.0;
    st_ls.d.remtime = 30.0;
    st_ls.d.date = 3100.0;
    st_ls.damage[DLIFSUP] = 5.0;
    st_ls.a.lsupres = 1.0;
    st_ls.a.time = 2.0; // lsupres < xtime && damage[DLIFSUP] > lsupres
    st_ls.skip_pauses = true;
    poll_ready(events(&mut st_ls, &io));
    assert!(st_ls.output_buf.contains("life support reserves have run out"));
    assert_eq!(st_ls.a.alldone, 1);

    // 3. Repair rates: in-flight vs docked vs Death Ray + radio recovery updating chart
    let mut st_rep = GameState::new(1, false, "");
    st_rep.a.ship = 'E';
    st_rep.a.quadx = 2;
    st_rep.a.quady = 2;
    st_rep.d.date = 3100.0;
    st_rep.d.remkl = 3;
    st_rep.d.remres = 100.0;
    st_rep.d.remtime = 30.0;
    st_rep.a.docfac = 0.25;
    st_rep.a.inlsr = 4.0;
    st_rep.damage[DSHIELD] = 2.0;
    st_rep.damage[DDRAY] = 5.0;
    st_rep.damage[DRADIO] = 0.5;
    st_rep.a.stdamtim = 3099.0;
    st_rep.starch[3][3] = 1205;
    st_rep.a.batx = 7;
    st_rep.a.baty = 7;
    st_rep.future[FCDBAS] = 3105.0;
    st_rep.skip_pauses = true;

    // In-flight for 1.0 stardate: DSHIELD 2.0->1.0, DRADIO 0.5->0.0 (recovers & updates chart!), DDRAY stays 5.0
    st_rep.a.time = 1.0;
    poll_ready(events(&mut st_rep, &io));
    assert!((st_rep.damage[DSHIELD] - 1.0).abs() < 1e-12);
    assert_eq!(st_rep.damage[DRADIO], 0.0);
    assert!((st_rep.damage[DDRAY] - 5.0).abs() < 1e-12); // unchanged in flight
    assert_eq!(st_rep.starch[3][3], 1); // updated from >999 to 1
    assert!(st_rep.output_buf.contains("sub-space radio is working"));
    assert!(st_rep.output_buf.contains("The star chart is now up to date."));
    assert!(st_rep.output_buf.contains("Starbase in  Quadrant 7 - 7 is currently under attack."));

    // Docked for 1.0 stardate: repair = 1.0 / 0.25 = 4.0 (DSHIELD 1.0->0.0), DDRAY repairs by xtime = 1.0 (5.0->4.0)
    st_rep.a.condit = IHDOCKED;
    st_rep.a.time = 1.0;
    poll_ready(events(&mut st_rep, &io));
    assert_eq!(st_rep.damage[DSHIELD], 0.0);
    assert!((st_rep.damage[DDRAY] - 4.0).abs() < 1e-12);
}

#[test]
fn rest_command_confirms_with_enemies_and_triggers_attack() {
    let mut s = TrekSession::new(1, false, "tournament 1 short novice testpass");
    s.with_state_mut(|st| {
        clear_quad_keep_ship(st);
        st.quad[5][8] = IHK;
        st.kx[1] = 5;
        st.ky[1] = 8;
        st.kpower[1] = 300.0;
        st.kdist[1] = 3.0;
        st.kavgd[1] = 3.0;
        st.a.nenhere = 1;
        st.a.klhere = 1;
        st.d.galaxy[2][1] = 100;
        st.a.energy = 4000.0;
        st.damage[DSHIELD] = 2.0;
    });

    let r1 = s.step("rest 2.0", false);
    assert!(r1.output.contains("Are you sure?"));
    let r2 = s.step("yes", false);
    assert!(r2.output.contains("stardates left."));
    s.with_state(|st| assert_eq!(st.damage[DSHIELD], 0.0));
}

#[test]
fn nova_chain_reaction_planets_bases_enemies_and_ship_displacement() {
    let io = IoHandle::new();
    let mut st = GameState::new(1, false, "");
    st.a.ship = 'E';
    st.a.quadx = 4;
    st.a.quady = 4;
    st.a.sectx = 5;
    st.a.secty = 4;
    st.a.energy = 5000.0;
    st.a.shield = 2500.0;
    st.a.shldup = 1;
    st.a.inplan = 1;
    st.a.iplnet = 1;
    st.d.plnets[1].x = 4;
    st.d.plnets[1].y = 4;
    st.d.newstuf[4][4] = 1;
    st.d.rembase = 1;
    st.d.baseqx[1] = 4;
    st.d.baseqy[1] = 4;
    st.d.remkl = 5;
    st.d.galaxy[4][4] = 212; // 2 klingons, 1 base, 2 stars
    clear_quad_keep_ship(&mut st);

    // Star at (5,5) chains to star at (5,6), adjacent to planet (4,6), base (6,6), Klingon (4,5), Commander (6,5)
    st.quad[5][5] = IHSTAR;
    st.quad[5][6] = IHSTAR;
    st.quad[4][6] = IHP;
    st.quad[6][6] = IHB;
    st.quad[4][5] = IHK;
    st.kx[1] = 4;
    st.ky[1] = 5;
    st.kpower[1] = 300.0;
    st.quad[6][5] = IHC;
    st.kx[2] = 6;
    st.ky[2] = 5;
    st.kpower[2] = 1200.0; // survives 800 hit -> buffeted
    st.a.nenhere = 2;
    st.a.klhere = 2;
    st.a.comhere = 1;
    // Choose RNG so 5% snova checks do NOT trigger (rand >= 0.05)
    st.rng = MacOsRng::from_raw(find_raw_seed_above(0.2));

    poll_ready(nova(&mut st, &io, 5, 5));
    assert!(st.output_buf.contains("Star at Sector 5 - 5 novas."));
    assert!(st.output_buf.contains("Star at Sector 5 - 6 novas."));
    assert!(st.output_buf.contains("Planet at Sector 4 - 6 destroyed."));
    assert!(st.output_buf.contains("Starbase at Sector 6 - 6 destroyed."));
    assert!(st.output_buf.contains("Klingon at Sector 4 - 5 destroyed."));
    assert!(st.output_buf.contains("Commander at Sector 6 - 5 damaged"));
    assert!(st.output_buf.contains("Starship buffeted by nova."));
    assert!(st.output_buf.contains("Force of nova displaces starship."));
    assert_eq!(st.d.starkl, 2);
    assert_eq!(st.d.nplankl, 1);
    assert_eq!(st.d.basekl, 1);
    assert_eq!(st.d.rembase, 0);
    assert!(st.a.shield < 2500.0);
    // Starship displaced away from (5,4)
    assert!((st.a.sectx, st.a.secty) != (5, 4));
}

#[test]
fn snova_remote_destroys_last_klingon_and_incipient_current_quadrant() {
    let io = IoHandle::new();

    // 1. Remote supernova destroying last Klingon -> FinType::Won
    let mut st = GameState::new(1, false, "");
    st.a.ship = 'E';
    st.a.alive = 1;
    st.a.skill = 1;
    st.a.quadx = 1;
    st.a.quady = 1;
    st.d.galaxy[1][1] = 0;
    st.d.galaxy[8][8] = 101; // 1 Klingon + 1 star
    st.d.remkl = 1;
    st.a.inkling = 1;
    st.a.inresor = 10.0;
    st.d.remres = 10.0;
    st.d.date = 3101.0;
    st.a.indate = 3100.0;
    st.skip_pauses = true;
    poll_ready(snova(&mut st, &io, 0, 0));
    assert!(st.output_buf.contains("Supernova in Quadrant 8 - 8"));
    assert!(st.output_buf.contains("has just destroyed the last Klingons."));
    assert_eq!(st.a.gamewon, 1);
    assert_eq!(st.a.alldone, 1);
    assert_eq!(st.d.galaxy[8][8], 1000);

    // 2. Incipient supernova right next to ship (dist^2 <= 2.1) -> immediate FinType::Snovaed
    let mut st2 = GameState::new(1, false, "");
    st2.a.ship = 'E';
    st2.a.alive = 1;
    st2.a.quadx = 4;
    st2.a.quady = 4;
    st2.a.sectx = 5;
    st2.a.secty = 5;
    st2.d.galaxy[4][4] = 1;
    st2.quad[5][6] = IHSTAR;
    st2.d.remkl = 2;
    st2.a.inkling = 3;
    st2.a.inresor = 50.0;
    st2.d.remres = 10.0;
    st2.skip_pauses = true;
    poll_ready(snova(&mut st2, &io, 5, 6));
    assert!(st2.output_buf.contains("***Incipient supernova detected at Sector 5 - 6"));
    assert!(st2.output_buf.contains("fried by a supernova."));
    assert_eq!(st2.a.alldone, 1);
}

#[test]
fn tractor_beams_ftbeam_and_fspy_and_galileo_capture_or_hidden() {
    let io = IoHandle::new();

    // 1. FTBEAM pulls Enterprise to Commander's quadrant, raises shields, cancels rest, hides Galileo
    let mut st = GameState::new(1, false, "");
    st.a.ship = 'E';
    st.a.alive = 1;
    st.a.quadx = 1;
    st.a.quady = 1;
    st.a.sectx = 5;
    st.a.secty = 5;
    st.d.remcom = 1;
    st.d.cx[1] = 6;
    st.d.cy[1] = 6;
    st.d.galaxy[6][6] = 100;
    st.d.remkl = 2;
    st.d.remres = 100.0;
    st.d.date = 3100.0;
    st.a.intime = 7.0;
    st.a.inshld = 2500.0;
    st.a.shield = 2000.0;
    st.a.shldup = 0;
    st.a.resting = 1;
    st.a.iscraft = 0; // Galileo left on planet surface!
    st.future[FTBEAM] = 3100.5;
    st.a.time = 1.0;
    st.skip_pauses = true;
    // Choose RNG so Galileo is hidden (rand <= 0.5)
    let mut hide_seed = 1u32;
    for cand in 1u32..50_000 {
        let mut r = MacOsRng::from_raw(cand);
        let _ = r.rand_f64(); // com_idx
        if r.rand_f64() <= 0.5 {
            hide_seed = cand;
            break;
        }
    }
    st.rng = MacOsRng::from_raw(hide_seed);
    poll_ready(events(&mut st, &io));
    assert!(st.output_buf.contains("caught in long range tractor beam--"));
    assert!(st.output_buf.contains("Galileo, left on the planet surface, is well hidden."));
    assert!(st.output_buf.contains("Enterprise is pulled to Quadrant 6 - 6"));
    assert!(st.output_buf.contains("Remainder of rest/repair period cancelled."));
    assert_eq!(st.a.quadx, 6);
    assert_eq!(st.a.quady, 6);
    assert_eq!(st.a.shldup, 1);
    assert_eq!(st.a.resting, 0);

    // 2. FSPY pulls weak Enterprise to Super-Commander quadrant
    let mut st_spy = GameState::new(1, false, "");
    st_spy.a.ship = 'E';
    st_spy.a.alive = 1;
    st_spy.a.quadx = 1;
    st_spy.a.quady = 1;
    st_spy.d.nscrem = 1;
    st_spy.d.isx = 7;
    st_spy.d.isy = 2;
    st_spy.d.galaxy[7][2] = 100;
    st_spy.d.remkl = 2;
    st_spy.d.remres = 100.0;
    st_spy.d.date = 3100.0;
    st_spy.a.intime = 7.0;
    st_spy.a.iscraft = 1;
    st_spy.a.energy = 1000.0; // weak (< 2000)
    st_spy.a.torps = 2; // < 4
    st_spy.a.shield = 500.0; // < 1250
    st_spy.a.time = 1.0; // no future events <= 3101 -> selects line = FSPY (0)
    st_spy.skip_pauses = true;
    poll_ready(events(&mut st_spy, &io));
    assert!(st_spy.output_buf.contains("caught in long range tractor beam--"));
    assert!(st_spy.output_buf.contains("pulled to Quadrant 7 - 2"));
    assert_eq!(st_spy.a.quadx, 7);
    assert_eq!(st_spy.a.quady, 2);

    // 3. Tractor beam while Galileo is in flight (icraft == 1) -> FinType::Stractor
    let mut st_cr = GameState::new(1, false, "");
    st_cr.a.ship = 'E';
    st_cr.a.alive = 1;
    st_cr.a.quadx = 1;
    st_cr.a.quady = 1;
    st_cr.d.remcom = 1;
    st_cr.d.cx[1] = 5;
    st_cr.d.cy[1] = 5;
    st_cr.d.remkl = 2;
    st_cr.d.remres = 100.0;
    st_cr.d.date = 3100.0;
    st_cr.a.intime = 7.0;
    st_cr.a.icraft = 1;
    st_cr.future[FTBEAM] = 3100.2;
    st_cr.a.time = 1.0;
    st_cr.skip_pauses = true;
    poll_ready(events(&mut st_cr, &io));
    assert!(st_cr.output_buf.contains("The shuttle craft Galileo is also caught"));
    assert_eq!(st_cr.a.alldone, 1);
}

#[test]
fn starbase_attacks_fbattak_fcdbas_and_fscdbas() {
    let io = IoHandle::new();
    let mut st = GameState::new(1, false, "");
    st.a.ship = 'E';
    st.a.quadx = 1;
    st.a.quady = 1;
    st.d.remkl = 3;
    st.d.remres = 100.0;
    st.d.rembase = 2;
    st.d.baseqx[1] = 4;
    st.d.baseqy[1] = 4;
    st.d.baseqx[2] = 7;
    st.d.baseqy[2] = 7;
    st.d.galaxy[4][4] = 110;
    st.d.galaxy[7][7] = 110;
    st.starch[4][4] = -1;
    st.d.remcom = 1;
    st.d.cx[1] = 4;
    st.d.cy[1] = 4;
    st.d.date = 3100.0;
    st.a.intime = 7.0;
    st.future[FBATTAK] = 3100.5;
    st.a.time = 1.0;
    st.skip_pauses = true;

    // 1. FBATTAK initiates attack on base at (4, 4)
    poll_ready(events(&mut st, &io));
    assert_eq!(st.a.batx, 4);
    assert_eq!(st.a.baty, 4);
    assert!(st.future[FCDBAS] < 1e30);
    assert!(st.output_buf.contains("starbase in Quadrant 4 - 4"));
    assert!(st.output_buf.contains("under atttack"));

    // 2. FCDBAS destroys base at (4, 4)
    st.output_buf.clear();
    let cdbas_time = st.future[FCDBAS];
    st.a.time = (cdbas_time - st.d.date) + 0.1;
    st.future[FBATTAK] = 1e30;
    poll_ready(events(&mut st, &io));
    assert!(st.output_buf.contains("starbase in Quadrant 4 - 4 has been destroyed by"));
    assert!(st.output_buf.contains("a Klingon Commander"));
    assert_eq!(st.d.rembase, 1);
    assert_eq!(st.d.galaxy[4][4], 100);
    assert_eq!(st.starch[4][4], 0);
    assert_eq!(st.a.batx, 0);

    // 3. FSCDBAS destroys base at (7, 7) by Super-Commander
    st.output_buf.clear();
    st.d.nscrem = 1;
    st.d.isx = 7;
    st.d.isy = 7;
    st.a.isatb = 1;
    st.d.rembase = 2; // != 1 so Starfleet reports it
    st.d.baseqx[2] = 6;
    st.d.baseqy[2] = 6;
    st.future[FSCDBAS] = st.d.date + 0.5;
    st.a.time = 0.6;
    poll_ready(events(&mut st, &io));
    assert!(st.output_buf.contains("destroyed by"));
    assert!(st.output_buf.contains("the Klingon Super-Commander"));
    assert_eq!(st.d.rembase, 1);
    assert_eq!(st.d.galaxy[7][7], 100);
    assert_eq!(st.a.isatb, 0);
}

#[test]
fn deep_space_probe_armed_unarmed_and_galaxy_exit() {
    let mut s = TrekSession::new(1, false, "tournament 1 short novice testpass");
    s.with_state_mut(|st| {
        st.a.quadx = 2;
        st.a.quady = 1;
        st.a.sectx = 5;
        st.a.secty = 5;
        clear_quad_keep_ship(st);
        st.a.nprobes = 2;
        st.d.galaxy[2][2] = 101; // 1 klingon, 1 star
        st.d.remkl = 3;
        st.damage[DRADIO] = 0.0;
    });

    // Launch armed probe toward Quadrant 2 - 2 (sector 5,5)
    let r_launch = s.step("probe armed automatic 2 2", false);
    assert!(r_launch.output.contains("The deep space probe is launched, Captain."));
    assert_eq!(r_launch.hud.probes_left, 1);

    // Advance time enough for probe to reach (2,2) and detonate armed NOVAMAX warhead!
    s.with_state_mut(|st| st.skip_pauses = true);
    let r_rest = s.step("rest 0.5", false);
    assert!(r_rest.output.contains("deep space probe is now in  Quadrant 2 - 2"));
    assert!(r_rest.output.contains("Supernova in Quadrant 2 - 2"));
    assert_eq!(r_rest.hud.klingons_left, 2);
    s.with_state(|st| {
        assert_eq!(st.d.galaxy[2][2], 1000);
        assert_eq!(st.future[FDSPROB], 1e30);
    });

    // Launch unarmed probe toward edge of galaxy -> leaves galaxy
    s.with_state_mut(|st| {
        st.a.quadx = 1;
        st.a.quady = 1;
        st.a.sectx = 2;
        st.a.secty = 5;
        st.d.galaxy[1][1] = 0;
    });
    let _ = s.step("probe manual 0.0 1.0", false);
    let _ = s.step("no", false); // "Arm NOVAMAX warhead?" -> no
    let r_exit = s.step("rest 0.2", false);
    assert!(r_exit.output.contains("has left the galaxy."));
    s.with_state(|st| assert_eq!(st.future[FDSPROB], 1e30));

    // Damaged probe launcher & 0 probes
    s.with_state_mut(|st| {
        st.a.nprobes = 1;
        st.damage[DDSP] = 1.0;
    });
    assert!(s.step("probe", false).output.contains("probe launcher is damaged"));
    s.with_state_mut(|st| st.a.nprobes = 0);
    assert!(s.step("probe", false).output.contains("no more deep space probes"));
}

#[test]
fn movcom_scom_and_movetho_web_completion() {
    let io = IoHandle::new();

    // 1. movetho: Tholian moves corner-to-corner and completes full perimeter web
    let mut st = GameState::new(1, false, "");
    st.a.ship = 'E';
    st.a.sectx = 5;
    st.a.secty = 5;
    clear_quad_keep_ship(&mut st);
    st.a.ithere = 1;
    st.a.justin = 0;
    st.a.ithx = 1;
    st.a.ithy = 1;
    st.quad[1][1] = IHT;

    // 4 calls to movetho walk (1,1)->(1,10)->(10,10)->(10,1)->(1,1) and complete the web!
    poll_ready(movetho(&mut st, &io));
    assert_eq!((st.a.ithx, st.a.ithy), (1, 10));
    assert_eq!(st.quad[1][5], IHWEB);

    poll_ready(movetho(&mut st, &io));
    assert_eq!((st.a.ithx, st.a.ithy), (10, 10));
    assert_eq!(st.quad[5][10], IHWEB);

    poll_ready(movetho(&mut st, &io));
    assert_eq!((st.a.ithx, st.a.ithy), (10, 1));
    assert_eq!(st.quad[10][5], IHWEB);

    poll_ready(movetho(&mut st, &io));
    assert!(st.output_buf.contains("Tholian at Sector 1 - 1 completes web."));
    assert_eq!(st.a.ithere, 0);
    assert_eq!(st.quad[1][1], IHWEB);
    // A black hole was dropped in the quadrant
    let has_bh = (1..=10).any(|x| (1..=10).any(|y| st.quad[x][y] == IHBLANK));
    assert!(has_bh);

    // 2. movcom: Commander advances or retreats, and escapes when weak
    let mut st_mc = GameState::new(1, false, "");
    st_mc.a.ship = 'E';
    st_mc.a.skill = SEXPERT;
    st_mc.a.quadx = 4;
    st_mc.a.quady = 4;
    st_mc.a.sectx = 5;
    st_mc.a.secty = 5;
    clear_quad_keep_ship(&mut st_mc);
    st_mc.a.energy = 5000.0;
    st_mc.a.torps = 10;
    st_mc.a.shldup = 1;
    st_mc.quad[1][1] = IHC;
    st_mc.kx[1] = 1;
    st_mc.ky[1] = 1;
    st_mc.kpower[1] = 50.0; // very weak -> forces negative motion -> tryexit escapes!
    st_mc.kdist[1] = (32.0f64).sqrt();
    st_mc.kavgd[1] = st_mc.kdist[1];
    st_mc.a.nenhere = 1;
    st_mc.a.klhere = 1;
    st_mc.a.comhere = 1;
    st_mc.d.remcom = 1;
    st_mc.d.cx[1] = 4;
    st_mc.d.cy[1] = 4;
    st_mc.d.galaxy[4][4] = 100;
    st_mc.rng = MacOsRng::from_raw(find_raw_seed_below(0.2));
    poll_ready(movcom(&mut st_mc, &io));
    assert!(st_mc.output_buf.contains("Commander escapes to Quadrant"));
    assert_eq!(st_mc.a.nenhere, 0);

    // 3. scom: Super-Commander moves toward starbase and attacks it (iqx, iqy when ideltax != ideltay)
    let mut st_sc = GameState::new(1, false, "");
    st_sc.a.ship = 'E';
    st_sc.a.skill = SGOOD;
    st_sc.a.quadx = 1;
    st_sc.a.quady = 1;
    st_sc.a.indate = 3100.0;
    st_sc.d.date = 3105.0; // date - indate >= 3.0 so flag == false (head for starbase!)
    st_sc.d.killk = 10;
    st_sc.d.nscrem = 1;
    st_sc.d.isx = 4;
    st_sc.d.isy = 4;
    st_sc.d.galaxy[4][4] = 100;
    st_sc.d.rembase = 1;
    st_sc.d.baseqx[1] = 4;
    st_sc.d.baseqy[1] = 5; // Straight +Y from (4,4): ideltax = 0, ideltay = 1!
    st_sc.d.galaxy[4][5] = 10;
    st_sc.skip_pauses = true;
    let mut ipage = 0;
    poll_ready(scom(&mut st_sc, &io, &mut ipage));
    // SC MUST move from (4,4) to (4,5) to attack the starbase!
    assert_eq!(
        (st_sc.d.isx, st_sc.d.isy),
        (4, 5),
        "Super-Commander should advance along Y (ideltay=+1) to (4,5), but stayed/moved to ({}, {})",
        st_sc.d.isx,
        st_sc.d.isy
    );
    assert_eq!(st_sc.a.isatb, 1);
}
