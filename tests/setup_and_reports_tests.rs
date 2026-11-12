use trek::{
    DCOMPTR, DDRAY, DLIFSUP, DLRSENS, DRADIO, DSHIELD, DSRSENS, FDSPROB, FCDBAS, FSCDBAS,
    IHDOCKED, IHGREEN, IHK, SEMERITUS, SEXPERT, SFAIR, SGOOD, SNOVICE, TrekSession,
};

fn start_t1(coord_fixed: bool, length: &str, skill: &str) -> TrekSession {
    let cmd = format!("tournament 1 {length} {skill} testpass");
    TrekSession::new(1, coord_fixed, &cmd)
}

#[test]
fn setup_tournament_1_exact_mac_os_parity() {
    let session = TrekSession::new(1, false, "tournament 1 short novice testpass");
    let resp = session.last_response();
    let hud = session.hud();

    assert!(hud.in_game);
    assert_eq!(hud.ship, "E");
    assert_eq!(hud.stardate as i32, 3100);
    assert_eq!(hud.klingons_left, 3);
    assert_eq!(hud.bases_left, 2);
    assert_eq!(hud.quadx, 2);
    assert_eq!(hud.quady, 1);
    assert_eq!(hud.sectx, 7);
    assert_eq!(hud.secty, 7);
    assert_eq!(hud.time_left as i32, 7);
    assert_eq!(hud.energy as i32, 5000);
    assert_eq!(hud.shield as i32, 2500);
    assert_eq!(hud.torps, 10);

    // Output text parity: 2 supporting starbases at 7 - 7 and 2 - 1; Enterprise at Quadrant 2 - 1 Sector 7 - 7
    assert!(resp.output.contains("It is stardate 3100."));
    assert!(resp.output.contains("2 supporting starbases"));
    assert!(resp.output.contains("7 - 7"));
    assert!(resp.output.contains("2 - 1"));
    assert!(resp.output.contains("Quadrant 2 - 1  Sector 7 - 7"));

    session.with_state(|st| {
        assert_eq!(st.a.tourn, 1);
        assert_eq!(st.d.baseqx[1], 7);
        assert_eq!(st.d.baseqy[1], 7);
        assert_eq!(st.d.baseqx[2], 2);
        assert_eq!(st.d.baseqy[2], 1);
        assert_eq!(st.a.skill, SNOVICE);
        assert_eq!(st.a.length, 1);
    });
}

#[test]
fn setup_interactive_validation_and_regular_game() {
    let mut session = TrekSession::new(42, false, "");
    let r0 = session.step("bogus", false);
    assert!(r0.output.contains("What is \"bogus\"?"));

    // Tournament 0 should be rejected and re-prompted
    let r1 = session.step("tournament 0", false);
    assert!(r1.output.contains("Would you like a regular, tournament, or frozen game?"));

    let r2 = session.step("regular", false);
    assert!(r2.output.contains("Short, Medium, or Long"));

    let r3 = session.step("boguslen", false);
    assert!(r3.output.contains("What is \"boguslen\"?"));

    let r4 = session.step("medium", false);
    assert!(r4.output.contains("Novice, Fair, Good, Expert, or Emeritus"));

    let r5 = session.step("fair", false);
    assert!(r5.output.contains("secret password"));

    let r6 = session.step("secret", false);
    assert!(r6.hud.in_game);
    assert_eq!(r6.hud.time_left as i32, 14);
    assert!(r6.output.contains("Stardate "));
    assert!(r6.output.contains("an unknown number of Romulans"));
}

#[test]
fn setup_all_skill_levels_and_game_lengths() {
    let skills = [
        ("novice", SNOVICE, false),
        ("fair", SFAIR, false),
        ("good", SGOOD, true),
        ("expert", SEXPERT, true),
        ("emeritus", SEMERITUS, true),
    ];
    for (name, expected_skill, has_sc) in skills {
        let mut s = start_t1(false, "short", name);
        s.with_state(|st| {
            assert_eq!(st.a.skill, expected_skill);
            assert!((st.a.damfac - 0.5 * expected_skill as f64).abs() < 1e-12);
            assert_eq!(st.d.nscrem, if has_sc { 1 } else { 0 });
        });
        if has_sc {
            assert!(s.last_response().output.contains("Super-Commander"));
            assert!(s.last_response().output.contains("YOU'LL NEED IT."));
        } else {
            assert!(!s.last_response().output.contains("Super-Commander"));
        }
        // Novice/Fair print full legal commands on unrecognized input; Good+ do not.
        let r = s.step("xyzzy", false);
        if expected_skill <= SFAIR {
            assert!(r.output.contains("LEGAL COMMANDS ARE:"));
        } else {
            assert!(r.output.contains("UNRECOGNIZED COMMAND."));
            assert!(!r.output.contains("LEGAL COMMANDS ARE:"));
        }
    }

    for (len_name, len_val, expected_time) in [("short", 1, 7.0), ("medium", 2, 14.0), ("long", 4, 28.0)] {
        let s = start_t1(false, len_name, "good");
        s.with_state(|st| {
            assert_eq!(st.a.length, len_val);
            assert!((st.a.intime - expected_time).abs() < 1e-12);
            assert!((st.d.remtime - expected_time).abs() < 1e-12);
        });
    }
}

#[test]
fn srscan_modes_coordfixed_and_damaged_vs_docked() {
    let mut s = start_t1(false, "short", "novice");
    // Place an identifiable marker at (1, 10) and Enterprise at (7, 7)
    s.with_state_mut(|st| {
        st.quad[1][10] = IHK;
        st.a.condit = IHGREEN;
        st.damage[DSRSENS] = 0.0;
    });

    // Default srscan
    let r_def = s.step("srscan", false);
    assert!(r_def.output.contains("1 2 3 4 5 6 7 8 9 10"));
    assert!(r_def.output.contains("Stardate"));
    assert!(r_def.output.contains("Time Left"));
    // Without coordfixed, row 1 has column 10 == 'K'
    let row1 = r_def.output.lines().find(|l| l.starts_with(" 1  ")).unwrap();
    assert!(row1.contains("K  Stardate") || row1.contains("K "));

    // `srscan no` suppresses right-hand status panel
    let r_no = s.step("srscan no", false);
    assert!(r_no.output.contains("1 2 3 4 5 6 7 8 9 10"));
    assert!(!r_no.output.contains("Stardate"));

    // `srscan chart` appends compact star chart
    let r_chart = s.step("srscan chart", false);
    assert!(r_chart.output.contains("1    2    3    4    5    6    7    8"));
    assert!(!r_chart.output.contains("STAR CHART FOR THE KNOWN GALAXY"));

    // Damaged SR sensors while undocked: only 3x3 around (7, 7) visible, rest '-'
    s.with_state_mut(|st| {
        st.damage[DSRSENS] = 3.0;
        st.a.condit = IHGREEN;
    });
    let r_dam = s.step("srscan", false);
    assert!(r_dam.output.contains("SHORT-RANGE SENSORS DAMAGED"));
    let row1_dam = r_dam.output.lines().find(|l| l.starts_with(" 1  ")).unwrap();
    assert!(row1_dam.contains("- - - - - - - - - -"));

    // Docked with damaged SR sensors: uses starbase's sensors
    s.with_state_mut(|st| {
        st.a.condit = IHDOCKED;
    });
    let r_dock = s.step("srscan no", false);
    assert!(r_dock.output.contains("[Using starbase's sensors]"));
    let row1_dock = r_dock.output.lines().find(|l| l.starts_with(" 1  ")).unwrap();
    assert!(!row1_dock.contains("- - - - - - - - - -"));

    // -f (coordfixed = true): top row is y=10, and quad[1][10] ('K') is at first cell of row 10!
    s.set_coord_fixed(true);
    let r_cf = s.step("srscan no", false);
    let row10_cf = r_cf.output.lines().find(|l| l.starts_with("10  ")).unwrap();
    assert!(row10_cf.starts_with("10  K "));
}

#[test]
fn lrscan_normal_coordfixed_and_damaged_vs_docked() {
    let mut s = start_t1(false, "short", "novice");
    s.with_state_mut(|st| {
        st.a.quadx = 1;
        st.a.quady = 1;
        st.d.galaxy[1][1] = 103;
        st.d.galaxy[1][2] = 204;
        st.d.galaxy[2][1] = 15;
        st.d.galaxy[2][2] = 5;
        st.damage[DRADIO] = 0.0;
    });

    let r = s.step("lrscan", false);
    assert!(r.output.contains("Long-range scan for Quadrant 1 - 1"));
    assert!(r.output.contains("   -1   -1   -1"));
    assert!(r.output.contains("   -1  103  204"));
    assert!(r.output.contains("   -1   15    5"));

    // With coordfixed, y goes top-to-bottom (y=2, y=1, y=0) and x goes left-to-right (x=0, x=1, x=2)
    s.set_coord_fixed(true);
    let r_cf = s.step("lrscan", false);
    assert!(r_cf.output.contains("   -1  204    5"));
    assert!(r_cf.output.contains("   -1  103   15"));

    // Damaged LR sensors while undocked
    s.with_state_mut(|st| {
        st.damage[DLRSENS] = 2.0;
        st.a.condit = IHGREEN;
    });
    let r_dam = s.step("lrscan", false);
    assert!(r_dam.output.contains("LONG-RANGE SENSORS DAMAGED."));

    // Docked overrides damaged LR sensors
    s.with_state_mut(|st| {
        st.a.condit = IHDOCKED;
    });
    let r_dock = s.step("lrscan", false);
    assert!(r_dock.output.contains("Starbase's long-range scan for Quadrant 1 - 1"));
}

#[test]
fn chart_radio_damaged_memorization_and_docked_revision() {
    let mut s = start_t1(false, "short", "novice");
    s.with_state_mut(|st| {
        st.starch[1][1] = 0; // "..."
        st.starch[7][7] = -1; // ".1." (base)
        st.d.galaxy[2][1] = 15;
        st.starch[2][1] = 1015; // frozen radio-damaged record
        st.d.galaxy[2][1] = 315; // actual galaxy changed while radio damaged
        st.a.stdamtim = st.d.date - 2.5;
        st.damage[DRADIO] = 4.0;
        st.a.condit = IHGREEN;
    });

    let r_undocked = s.step("chart", false);
    assert!(r_undocked.output.contains("STAR CHART FOR THE KNOWN GALAXY"));
    assert!(r_undocked.output.contains("(Last surveillance update 2.5 stardates ago.)"));
    assert!(r_undocked.output.contains("  .1."));
    assert!(r_undocked.output.contains("  ..."));
    // Shows stale 15 instead of 315 while radio damaged & undocked
    assert!(r_undocked.output.contains("   15"));
    assert!(!r_undocked.output.contains("  315"));

    // Docked: Spock revises chart from starbase records (updates stdamtim and shows 1015?
    // Wait: chart() sets starch[i][j] = galaxy + 1000 for starch == 1, and updates stdamtim = date)
    s.with_state_mut(|st| {
        st.starch[3][3] = 1;
        st.d.galaxy[3][3] = 207;
        st.a.condit = IHDOCKED;
    });
    let r_docked = s.step("chart", false);
    assert!(r_docked.output.contains("I revised the Star Chart from the"));
    assert!(r_docked.output.contains("  207"));

    // -f coordfixed chart prints rows 8 down to 1
    s.set_coord_fixed(true);
    let r_cf = s.step("chart", false);
    let lines: Vec<&str> = r_cf.output.lines().filter(|l| l.contains(" -  ")).collect();
    assert!(lines.first().unwrap().starts_with("8 -"));
    assert!(lines.last().unwrap().starts_with("1 -"));
}

#[test]
fn dreprt_in_flight_docked_and_deathray_special_case() {
    let mut s = start_t1(false, "short", "novice");
    let r_ok = s.step("damages", false);
    assert!(r_ok.output.contains("All devices functional."));

    s.with_state_mut(|st| {
        st.damage[DSHIELD] = 4.0;
        st.damage[DDRAY] = 10.0;
        st.a.docfac = 0.25;
    });
    let r_dam = s.step("damages", false);
    assert!(r_dam.output.contains("DEVICE            -REPAIR TIMES-"));
    assert!(r_dam.output.contains("IN FLIGHT   DOCKED"));
    assert!(r_dam.output.contains("Shields"));
    // Shields: in-flight = 4.0 + 0.05 = 4.05, docked = 0.25*4.0 + 0.005 = 1.01 (or 1.00/1.01)
    assert!(r_dam.output.contains("4.05"));
    assert!(r_dam.output.contains("1.00") || r_dam.output.contains("1.01"));
    // Death Ray cannot be repaired in flight: in-flight column is spaces, docked column is 10.00/10.01
    let dray_line = r_dam.output.lines().find(|l| l.contains("Death Ray")).unwrap();
    assert!(dray_line.contains("Death Ray            "));
    assert!(dray_line.contains("10.0"));
}

#[test]
fn status_and_request_all_10_fields() {
    let mut s = start_t1(false, "short", "novice");
    s.with_state_mut(|st| {
        st.damage[DLIFSUP] = 2.0;
        st.a.lsupres = 3.25;
        st.a.iscloaked = true;
    });

    let r_status = s.step("status", false);
    assert!(r_status.output.contains("Stardate"));
    assert!(r_status.output.contains("Condition"));
    assert!(r_status.output.contains("CLOAKED"));
    assert!(r_status.output.contains("DAMAGED, reserves=3.25"));
    assert!(r_status.output.contains("Time Left"));
    // Must not print the 10x10 sector grid
    assert!(!r_status.output.contains("1 2 3 4 5 6 7 8 9 10"));

    // All 10 `request` fields
    let requests = [
        ("date", "Stardate"),
        ("condition", "Condition"),
        ("position", "Position"),
        ("lsupport", "Life Support"),
        ("warpfactor", "Warp Factor"),
        ("energy", "Energy"),
        ("torpedoes", "Torpedoes"),
        ("shields", "Shields"),
        ("klingons", "Klingons Left"),
        ("time", "Time Left"),
    ];
    for (req, expected) in requests {
        let cmd = format!("request {req}");
        let r = s.step(&cmd, false);
        assert!(
            r.output.contains(expected),
            "request {req} missing {expected}, got: {}",
            r.output
        );
    }

    // Interactive prompt + unrecognized request
    let r_p = s.step("request", false);
    assert!(r_p.output.contains("Information desired?"));
    let r_bad = s.step("bogus", false);
    assert!(r_bad.output.contains("UNRECOGNIZED REQUEST."));
}

#[test]
fn report_all_counters_and_crystal_probe_states() {
    let mut s = start_t1(false, "short", "expert");
    s.with_state_mut(|st| {
        st.a.thawed = 1;
        st.d.killk = 2;
        st.d.killc = 1;
        st.d.nsckill = 0;
        st.d.nscrem = 1;
        st.d.rembase = st.a.inbase - 1;
        st.a.casual = 3;
        st.d.brigfree = st.a.brigcapacity - 12;
        st.d.kcaptured = 5;
        st.a.nhelp = 1;
        st.a.nprobes = 2;
        st.future[FDSPROB] = st.d.date + 0.5;
        st.a.isarmed = true;
        st.a.probecx = 4;
        st.a.probecy = 6;
        st.a.icrystl = 1;
        st.a.cryprob = 0.2; // used 2 times (0.05 -> 0.1 -> 0.2)
        st.a.isatb = 1;
        st.d.isx = 3;
        st.d.isy = 5;
        st.future[FSCDBAS] = st.d.date + 1.5;
        st.a.batx = 7;
        st.a.baty = 7;
        st.future[FCDBAS] = st.d.date + 2.0;
    });

    let r = s.step("report", false);
    assert!(r.output.contains("thawed short expert game"));
    assert!(r.output.contains("No plaque is allowed."));
    assert!(r.output.contains("This is tournament game 1."));
    assert!(r.output.contains("including 1 Commander."));
    assert!(r.output.contains("The Super Commander has not been destroyed."));
    assert!(r.output.contains("has been 1 base destroyed"));
    assert!(r.output.contains("Starbase in  Quadrant 7 - 7 is currently under attack."));
    assert!(r.output.contains("Starbase in  Quadrant 3 - 5 is under Super-commander attack."));
    assert!(r.output.contains("3 casualties suffered so far."));
    assert!(r.output.contains("12 Klingons in brig."));
    assert!(r.output.contains("5 captured Klingons turned in to Star Fleet."));
    assert!(r.output.contains("There was 1 call for help."));
    assert!(r.output.contains("You have 2 deep space probes."));
    assert!(r.output.contains("An armed deep space probe is in Quadrant 4 - 6."));
    assert!(r.output.contains("Dilithium crystals have been used 2 times."));

    // Unused crystals check
    s.with_state_mut(|st| st.a.cryprob = 0.05);
    let r2 = s.step("report", false);
    assert!(r2.output.contains("Dilithium crystals aboard ship...not yet used."));
}

#[test]
fn eta_computer_calculator_paths_and_warnings() {
    let mut s = start_t1(false, "short", "novice");

    // 1. Damaged computer error
    s.with_state_mut(|st| st.damage[DCOMPTR] = 1.0);
    let r_dam = s.step("computer 2 1 8 8", false);
    assert!(r_dam.output.contains("COMPUTER DAMAGED, USE A POCKET CALCULATOR."));

    s.with_state_mut(|st| st.damage[DCOMPTR] = 0.0);

    // 2. Arrival stardate mode -> computes minimum warp; also test warp > 6 & time warning & base destroyed warning
    s.with_state_mut(|st| {
        st.a.energy = 5000.0;
        st.a.shldup = 1;
        st.d.remtime = 0.1; // short remaining time triggers Federation destroyed warning
        st.a.isatb = 1;
        st.d.isx = 7;
        st.d.isy = 7;
        st.future[FSCDBAS] = st.d.date + 0.05;
    });
    // Destination: quad (7, 7) sector (7, 7) from (2, 1) (7, 7) -> dist = sqrt(5^2 + 6^2) = 7.81
    let r_eta1 = s.step("computer 7 7 7 7", false);
    assert!(r_eta1.output.contains("Time or arrival date?"));

    // Give arrival date = 3102.0 (2 stardates) -> min warp sqrt(10*7.81/2) ~ 6.3
    let r_eta2 = s.step("3102", false);
    assert!(r_eta2.output.contains("Minimum warp needed is"));
    assert!(r_eta2.output.contains("and we will arrive at stardate"));
    assert!(r_eta2.output.contains("Unfortunately, the Federation will be destroyed by then."));
    assert!(r_eta2.output.contains("You'll be taking risks at that speed, Captain"));
    assert!(r_eta2.output.contains("The starbase there will be destroyed by then."));
    assert!(r_eta2.output.contains("New warp factor to try?"));

    // 3. Try warp 10 -> insufficient energy with shields up, or completely insufficient
    let r_eta3 = s.step("10", false);
    assert!(r_eta3.output.contains("Insufficient energy, sir: we would need"));

    // 4. Try warp 2 -> succeeds, exits when answered "no"
    let r_eta4 = s.step("2", false);
    assert!(r_eta4.output.contains("Remaining energy will be"));
    assert!(r_eta4.output.contains("And we will arrive at stardate"));

    let r_done = s.step("no", false);
    assert!(r_done.output.contains("COMMAND> "));

    // 5. Sector-only ETA with shields-down hint
    s.with_state_mut(|st| {
        st.a.energy = 100.0;
        st.a.shldup = 1;
        st.d.remtime = 10.0;
    });
    let r_sec = s.step("computer 1 1", false);
    assert!(r_sec.output.contains("Time or arrival date?"));
    let _ = s.step("no", false);
    let r_w = s.step("5", false);
    // dist from (7,7) to (1,1) = sqrt(0.36+0.36) ~ 0.8485; power = 0.8485 * 125 * 2 = 212.1 > 100, half=106 > 100 -> try 4: 0.8485*64*2 = 108.6 > 100, half=54.3 <= 100!
    assert!(r_w.output.contains("Insufficient energy"));
    let r_w4 = s.step("4", false);
    assert!(r_w4.output.contains("But if you lower your shields,"));
    assert!(r_w4.output.contains("remaining energy will be"));
    let _ = s.step("no", false);
}
