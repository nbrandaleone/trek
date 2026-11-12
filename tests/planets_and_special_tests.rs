use trek::{
    deserialize_freeze, serialize_freeze, GameState, MacOsRng, PromptType, TrekSession, DCOMPTR,
    DDRAY, DIMPULS, DPHASER, DRADIO, DSHUTTL, DSRSENS, DWARPEN, IHB, IHDOCKED, IHDOT, IHE, IHF,
    IHGREEN, IHK, IHP, IHQUEST,
};

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

fn find_seed_two_draws<F: Fn(f64, f64) -> bool>(pred: F) -> u32 {
    for s in 1u32..500_000 {
        let mut r = MacOsRng::from_raw(s);
        let r1 = r.rand_f64();
        let r2 = r.rand_f64();
        if pred(r1, r2) {
            return s;
        }
    }
    unreachable!()
}

fn setup_planet_quad(st: &mut GameState, crystals: i32) {
    st.a.quadx = 4;
    st.a.quady = 4;
    st.a.sectx = 5;
    st.a.secty = 5;
    clear_quad_keep_ship(st);
    st.a.condit = IHGREEN;
    st.a.shldup = 0;
    st.a.inorbit = 0;
    st.a.landed = -1;
    st.a.iscraft = 1;
    st.a.inplan = 1;
    st.a.iplnet = 1;
    st.a.plnetx = 5;
    st.a.plnety = 6;
    st.quad[5][6] = IHP;
    st.d.newstuf[4][4] = 1;
    st.d.plnets[1].x = 4;
    st.d.plnets[1].y = 4;
    st.d.plnets[1].pclass = 1; // M
    st.d.plnets[1].crystals = crystals;
    st.d.plnets[1].known = 0;
}

#[test]
fn planets_orbit_sensor_preport_beam_and_flost() {
    let mut s = TrekSession::new(1, false, "tournament 1 short novice testpass");

    // No planet report initially
    assert!(s.step("planets", false).output.contains("No information available."));

    // Setup planet adjacent at (5,6) with crystals
    s.with_state_mut(|st| setup_planet_quad(st, 1));

    // Sensor scan with damaged SR sensors fails
    s.with_state_mut(|st| st.damage[DSRSENS] = 1.0);
    assert!(s.step("sensors", false).output.contains("Short range sensors damaged."));

    // Working sensor scan marks planet known=1 and shows up in `planets`
    s.with_state_mut(|st| st.damage[DSRSENS] = 0.0);
    let r_sens = s.step("sensors", false);
    assert!(r_sens.output.contains("Planet at Sector 5 - 6 is of class M."));
    assert!(r_sens.output.contains("Readings indicate dilithium crystals present."));

    let r_prep = s.step("planets", false);
    assert!(r_prep.output.contains("Quadrant 4 - 4   class M   dilithium crystals present."));

    // Beam before orbit fails
    assert!(s.step("transport", false).output.contains("not in standard orbit."));

    // Orbit fails if both engines damaged
    s.with_state_mut(|st| {
        st.damage[DWARPEN] = 1.0;
        st.damage[DIMPULS] = 1.0;
    });
    assert!(s.step("orbit", false).output.contains("Both warp and impulse engines damaged."));

    // Enter orbit
    s.with_state_mut(|st| {
        st.damage[DWARPEN] = 0.0;
        st.damage[DIMPULS] = 0.0;
    });
    let r_orb = s.step("orbit", false);
    assert!(r_orb.output.contains("Entered orbit at altitude"));
    assert!(r_orb.hud.in_orbit);

    // Already in orbit
    assert!(s.step("orbit", false).output.contains("Already in standard orbit."));

    // Beam shields check
    s.with_state_mut(|st| st.a.shldup = 1);
    assert!(s.step("transport", false).output.contains("Impossible to transport through shields."));

    // Beam unknown planet check
    s.with_state_mut(|st| {
        st.a.shldup = 0;
        st.d.plnets[1].known = 0;
    });
    assert!(s.step("transport", false).output.contains("no information on this planet"));

    // Beam down succeeds (rand <= 0.98)
    s.with_state_mut(|st| {
        st.d.plnets[1].known = 1;
        st.rng = MacOsRng::from_raw(find_raw_seed_below(0.5));
    });
    let r_down = s.step("transport", false);
    assert!(r_down.output.contains("Transport complete."));
    assert!(r_down.hud.landed);

    // Beam up when Galileo is also on surface -> asks & hides Galileo
    s.with_state_mut(|st| {
        st.d.plnets[1].known = 2;
        st.rng = MacOsRng::from_raw(find_raw_seed_below(0.5));
    });
    let r_gal = s.step("transport", false);
    assert!(r_gal.output.contains("Wouldn't you rather take the Galileo?"));
    let r_up = s.step("no", false);
    assert!(r_up.output.contains("crew hides the Galileo"));
    assert!(r_up.output.contains("Transport complete."));
    assert!(!r_up.hud.landed);

    // 2% beam loss (rand > 0.98) -> FinType::Lost
    s.with_state_mut(|st| {
        st.d.plnets[1].known = 1;
        st.rng = MacOsRng::from_raw(find_raw_seed_above(0.985));
    });
    let r_lost = s.step("transport", false);
    assert!(r_lost.output.contains("Oh my God!  I've lost them."));
    assert!(r_lost.output.contains("dissipating through space."));
    assert!(r_lost.hud.alldone);
}

#[test]
fn shuttle_mine_and_usecrystals_boost_and_kaboom() {
    let mut s = TrekSession::new(1, false, "tournament 1 short novice testpass");
    s.with_state_mut(|st| {
        setup_planet_quad(st, 1);
        st.d.plnets[1].known = 1;
        st.a.inorbit = 1;
        st.height = 2000;
    });

    // Low time abort on shuttle
    s.with_state_mut(|st| {
        st.d.remtime = 0.01;
        st.d.remres = 0.01 * ((st.d.remkl + 4 * st.d.remcom) as f64);
    });
    let r_low = s.step("shuttle", false);
    assert!(r_low.output.contains("Are you sure this is wise?"));
    let _ = s.step("no", false);

    // Fly down in Galileo
    s.with_state_mut(|st| {
        st.d.remtime = 7.0;
        st.d.remres = 7.0 * ((st.d.remkl + 4 * st.d.remcom) as f64);
    });
    let r_dn = s.step("shuttle", false);
    assert!(r_dn.output.contains("ready to board the shuttle craft \"Galileo\"."));
    assert!(r_dn.output.contains("Trip complete"));
    assert!(r_dn.hud.landed);
    s.with_state(|st| {
        assert_eq!(st.d.plnets[1].known, 2);
        assert_eq!(st.a.iscraft, 0);
    });

    // Planet report shows Galileo on surface
    assert!(s.step("planets", false).output.contains("Shuttle Craft Galileo on surface."));

    // Mine dilithium crystals
    let r_mine = s.step("mine", false);
    assert!(r_mine.output.contains("Mining operation complete."));
    // Already mined
    assert!(s.step("mine", false).output.contains("already mined enough crystals"));

    // Fly up in Galileo -> loads crystals onto ship (icrystl = 1, cryprob = 0.05)
    let r_up = s.step("shuttle", false);
    assert!(r_up.output.contains("trip back to the Enterprise"));
    assert!(r_up.output.contains("Trip complete."));
    assert!(!r_up.hud.landed);
    assert!(r_up.hud.crystals_aboard);
    s.with_state(|st| {
        assert_eq!(st.a.iscraft, 1);
        assert!((st.a.cryprob - 0.05).abs() < 1e-12);
    });

    // Use crystals: blocked when energy >= 1000
    assert!(s.step("crystals", false).output.contains("when condition Yellow exists"));

    // Use crystals successfully when energy < 1000
    s.with_state_mut(|st| {
        st.a.energy = 500.0;
        st.rng = MacOsRng::from_raw(find_raw_seed_above(0.2)); // > 0.05
    });
    let r_use1 = s.step("crystals", false);
    assert!(r_use1.output.contains("Are you sure this is wise?"));
    let r_use2 = s.step("yes", false);
    assert!(r_use2.output.contains("Congratulations, Sir!"));
    assert!(r_use2.hud.energy > 5000.0);
    s.with_state(|st| assert!((st.a.cryprob - 0.10).abs() < 1e-12));

    // Use crystals exploding (kaboom -> FinType::Dilithium)
    s.with_state_mut(|st| {
        st.a.energy = 500.0;
        st.rng = MacOsRng::from_raw(find_raw_seed_below(0.05)); // <= 0.10
    });
    let _ = s.step("crystals", false);
    let r_boom = s.step("yes", false);
    assert!(r_boom.output.contains("KA-BOOM!!!!"));
    assert!(r_boom.output.contains("expanding cloud of subatomic particles"));
    assert!(r_boom.hud.alldone);
}

#[test]
fn deathray_all_five_outcomes_and_faerie_queene_rejection() {
    // 1. Faerie Queene rejection, no enemies, damaged
    let mut s = TrekSession::new(1, false, "tournament 1 short novice testpass");
    s.with_state_mut(|st| st.a.ship = IHF);
    assert!(s.step("deathray", false).output.contains("Ye Faerie Queene has no death ray."));

    s.with_state_mut(|st| {
        st.a.ship = IHE;
        clear_quad_keep_ship(st);
    });
    assert!(s.step("deathray", false).output.contains("no enemies in this quadrant"));

    s.with_state_mut(|st| {
        st.a.nenhere = 1;
        st.damage[DDRAY] = 5.0;
    });
    assert!(s.step("deathray", false).output.contains("Death Ray is damaged."));

    // Helper closure to prepare a 1-Klingon quad for deathray
    let prep_enemy = |st: &mut GameState| {
        clear_quad_keep_ship(st);
        st.damage[DDRAY] = 0.0;
        st.a.ship = IHE;
        st.a.alive = 1;
        st.a.alldone = 0;
        st.quad[5][8] = IHK;
        st.kx[1] = 5;
        st.ky[1] = 8;
        st.kpower[1] = 400.0;
        st.kdist[1] = 3.0;
        st.kavgd[1] = 3.0;
        st.a.nenhere = 1;
        st.a.klhere = 1;
        st.d.galaxy[2][1] = 100;
        st.d.remkl = 3;
    };

    // Outcome 1a: r > 0.30, 5% intact (r2 <= 0.05)
    s.with_state_mut(|st| {
        prep_enemy(st);
        st.rng = MacOsRng::from_raw(find_seed_two_draws(|r1, r2| r1 > 0.30 && r2 <= 0.05));
    });
    let _ = s.step("deathray", false);
    let r_intact = s.step("yes", false);
    assert!(r_intact.output.contains("Klingon at Sector 5 - 8 destroyed."));
    assert!(r_intact.output.contains("is still operational."));
    s.with_state(|st| assert_eq!(st.damage[DDRAY], 0.0));

    // Outcome 1b: r > 0.30, 95% dysfunctional (r2 > 0.05)
    s.with_state_mut(|st| {
        prep_enemy(st);
        st.rng = MacOsRng::from_raw(find_seed_two_draws(|r1, r2| r1 > 0.30 && r2 > 0.05));
    });
    let _ = s.step("deathray", false);
    let r_dys = s.step("yes", false);
    assert!(r_dys.output.contains("has been rendered dysfunctional."));
    s.with_state(|st| assert!((st.damage[DDRAY] - 39.95).abs() < 1e-12));

    // Outcome 4: *THINGS* infestation (r1 <= 0.30, 0.55 < r2 <= 0.75) - ship survives!
    s.with_state_mut(|st| {
        prep_enemy(st);
        st.rng = MacOsRng::from_raw(find_seed_two_draws(|r1, r2| r1 <= 0.30 && r2 > 0.55 && r2 <= 0.75));
    });
    let _ = s.step("deathray", false);
    let r_things = s.step("yes", false);
    assert!(r_things.output.contains("infested with\n - - - - - -  *THINGS*."));
    s.with_state(|st| {
        let q_cnt = (1..=10).flat_map(|x| (1..=10).map(move |y| (x, y))).filter(|&(x, y)| st.quad[x][y] == IHQUEST).count();
        assert!(q_cnt > 90);
    });

    // Outcome 2: Implosion kaboom (r1 <= 0.30, r2 <= 0.30) -> FinType::Dilithium
    let mut s2 = TrekSession::new(1, false, "tournament 1 short novice testpass");
    s2.with_state_mut(|st| {
        prep_enemy(st);
        st.rng = MacOsRng::from_raw(find_seed_two_draws(|r1, r2| r1 <= 0.30 && r2 <= 0.30));
    });
    let _ = s2.step("deathray", false);
    let r_imp = s2.step("yes", false);
    assert!(r_imp.output.contains("MATTER-ANTIMATTER IMPLOSION IMMINENT!"));
    assert!(r_imp.output.contains("KA-BOOM!!!!"));
    assert!(r_imp.hud.alldone);

    // Outcome 3: Mutants (r1 <= 0.30, 0.30 < r2 <= 0.55) -> FinType::Dray
    let mut s3 = TrekSession::new(1, false, "tournament 1 short novice testpass");
    s3.with_state_mut(|st| {
        prep_enemy(st);
        st.rng = MacOsRng::from_raw(find_seed_two_draws(|r1, r2| r1 <= 0.30 && r2 > 0.30 && r2 <= 0.55));
    });
    let _ = s3.step("deathray", false);
    let r_mut = s3.step("yes", false);
    assert!(r_mut.output.contains("mutants attack and kill Spock."));
    assert!(r_mut.hud.alldone);

    // Outcome 5: Tribbles (r1 <= 0.30, r2 > 0.75) -> FinType::Tribble
    let mut s5 = TrekSession::new(1, false, "tournament 1 short novice testpass");
    s5.with_state_mut(|st| {
        prep_enemy(st);
        st.rng = MacOsRng::from_raw(find_seed_two_draws(|r1, r2| r1 <= 0.30 && r2 > 0.75));
    });
    let _ = s5.step("deathray", false);
    let r_trib = s5.step("yes", false);
    assert!(r_trib.output.contains("creating tribbles!"));
    assert!(r_trib.output.contains("Tribbles consume all remaining water"));
    assert!(r_trib.hud.alldone);
}

#[test]
fn abandon_docked_and_shuttle_escape_to_faerie_queene_and_no_base_death() {
    // 1. Docked abandon -> Faerie Queene
    let mut s = TrekSession::new(1, false, "tournament 1 short novice testpass");
    s.with_state_mut(|st| {
        clear_quad_keep_ship(st);
        st.a.condit = IHDOCKED;
        st.a.icrystl = 1;
        st.damage[DPHASER] = 5.0;
    });
    let r = s.step("abandon", false);
    assert!(r.output.contains("the Faerie Queene, which is antiquated"));
    assert!(r.output.contains("dilithium crystals have been moved."));
    assert_eq!(r.hud.ship, "F");
    assert_eq!(r.hud.max_energy as i32, 3000);
    assert_eq!(r.hud.max_shield as i32, 1250);
    assert_eq!(r.hud.max_torps, 6);
    assert_eq!(r.hud.brig_capacity, 300);
    s.with_state(|st| {
        assert_eq!(st.damage[DSHUTTL], -1.0);
        assert_eq!(st.damage[DPHASER], 0.0);
    });

    // Cannot abandon Faerie Queene
    assert!(s.step("abandon", false).output.contains("cannot abandon Ye Faerie Queene."));

    // 2. In-flight shuttle escape -> POW exchange -> Faerie Queene
    let mut s2 = TrekSession::new(1, false, "tournament 1 short novice testpass");
    s2.with_state_mut(|st| {
        clear_quad_keep_ship(st);
        st.a.condit = IHGREEN;
        st.a.iscraft = 1;
        st.damage[DSHUTTL] = 0.0;
    });
    let r2 = s2.step("abandon", false);
    assert!(r2.output.contains("***ABANDON SHIP!  ABANDON SHIP!"));
    assert!(r2.output.contains("prisoner-of-war exchange."));
    assert_eq!(r2.hud.ship, "F");

    // 3. In-flight abandon with rembase == 0 -> FinType::Abandn
    let mut s3 = TrekSession::new(1, false, "tournament 1 short novice testpass");
    s3.with_state_mut(|st| {
        clear_quad_keep_ship(st);
        st.a.condit = IHGREEN;
        st.d.rembase = 0;
    });
    let r3 = s3.step("abandon", false);
    assert!(r3.output.contains("no starbases, you will be mercilessly tortured to death."));
    assert!(r3.hud.alldone);
}

#[test]
fn destruct_password_verification_and_explosion() {
    let mut s = TrekSession::new(1, false, "tournament 1 short novice testpass");

    // Damaged computer blocks self-destruct
    s.with_state_mut(|st| st.damage[DCOMPTR] = 1.0);
    assert!(s.step("destruct", false).output.contains("Computer damaged"));

    // Wrong password aborts
    s.with_state_mut(|st| st.damage[DCOMPTR] = 0.0);
    let r1 = s.step("destruct", false);
    assert!(r1.output.contains("ENTER-CORRECT-PASSWORD-TO-CONTINUE-"));
    let r_bad = s.step("wrongpw", false);
    assert!(r_bad.output.contains("PASSWORD-REJECTED;"));
    assert!(!r_bad.hud.alldone);

    // Correct password triggers kaboom destroying nearby enemy
    s.with_state_mut(|st| {
        clear_quad_keep_ship(st);
        st.quad[5][8] = IHK;
        st.kx[1] = 5;
        st.ky[1] = 8;
        st.kpower[1] = 300.0;
        st.kdist[1] = 1.0;
        st.a.nenhere = 1;
        st.a.klhere = 1;
        st.d.remkl = 3;
        st.d.galaxy[2][1] = 100;
    });
    let _ = s.step("destruct", false);
    let r_ok = s.step("testpass", false);
    assert!(r_ok.output.contains("PASSWORD-ACCEPTED"));
    assert!(r_ok.output.contains("Entropy of Enterprise maximized"));
    assert!(r_ok.output.contains("Klingon at Sector 5 - 8 destroyed."));
    assert!(r_ok.output.contains("expanding cloud of subatomic particles"));
    assert!(r_ok.hud.alldone);
}

#[test]
fn freeze_thaw_roundtrip_fidelity_and_emexit() {
    let mut s = TrekSession::new(1, false, "tournament 1 short expert testpass");
    s.with_state_mut(|st| {
        st.a.energy = 3456.75;
        st.a.shield = 1234.5;
        st.a.casual = 7;
        st.d.kcaptured = 15;
        st.damage[DRADIO] = 2.5;
        st.starch[3][4] = 1201;
        st.height = 4200;
    });

    let r_frz = s.step("freeze mysave", false);
    assert!(r_frz.output.contains("Frozen games produce no plaques!"));
    let entry = r_frz
        .saves_updated
        .iter()
        .find(|e| e.name == "mysave.trk")
        .expect("mysave.trk created")
        .clone();

    // Direct serialize/deserialize roundtrip check
    let mut dst = GameState::default();
    assert!(deserialize_freeze(&mut dst, &entry.data));
    assert!((dst.a.energy - 3456.75).abs() < 1e-12);
    assert!((dst.a.shield - 1234.5).abs() < 1e-12);
    assert_eq!(dst.a.casual, 7);
    assert_eq!(dst.d.kcaptured, 15);
    assert!((dst.damage[DRADIO] - 2.5).abs() < 1e-12);
    assert_eq!(dst.starch[3][4], 1201);
    assert_eq!(dst.height, 4200);
    assert_eq!(dst.passwd, "testpass");
    assert!(!deserialize_freeze(&mut dst, "{invalid json"));
    let _ = serialize_freeze(&dst);

    // Thaw in a fresh session
    let mut s2 = TrekSession::new(99, false, "");
    assert!(s2.import_save(&entry.name, &entry.data));
    let r_thaw = s2.step("frozen mysave", false);
    assert!(r_thaw.output.contains("thawed short expert game"));
    assert!(r_thaw.output.contains("Your secret password is \"testpass\""));
    assert_eq!(r_thaw.hud.energy as i32, 3456);
    assert_eq!(r_thaw.hud.casualties, 7);
    assert_eq!(r_thaw.hud.kcaptured, 15);

    // Missing save file handling
    let mut s3 = TrekSession::new(99, false, "");
    let r_miss = s3.step("frozen nonexistent", false);
    assert!(r_miss.output.contains("Can't find game file nonexiste.trk"));

    // emexit creates emsave.trk and exits
    let r_em = s2.step("emexit", false);
    assert!(r_em.emexit);
    assert!(r_em.clear_screen);
    assert_eq!(r_em.prompt_type, PromptType::Exited);
    assert!(r_em.saves_updated.iter().any(|e| e.name == "emsave.trk"));
}

#[test]
fn score_promotion_and_132_col_plaque_generation() {
    let mut s = TrekSession::new(1, false, "tournament 1 short expert testpass");
    s.with_state_mut(|st| {
        clear_quad_keep_ship(st);
        st.plaque_timestamp = "Nov 12 2026, 15:30:00".to_string();
        // Leave 1 Klingon left in current quad and kill it via phasers so FinType::Won triggers promotion & plaque!
        st.quad[5][8] = IHK;
        st.kx[1] = 5;
        st.ky[1] = 8;
        st.kpower[1] = 100.0;
        st.kdist[1] = 1.0;
        st.kavgd[1] = 1.0;
        st.a.nenhere = 1;
        st.a.klhere = 1;
        st.d.remkl = 1;
        st.d.remcom = 0;
        st.d.nscrem = 0;
        st.d.killk = st.a.inkling - 1;
        st.d.galaxy[2][1] = 100;
        st.d.date = st.a.indate + 1.0; // dt < 5.0 -> promotion!
    });

    // Check in-game `score` first
    let r_sc = s.step("score", false);
    assert!(r_sc.output.contains("Your score so far --"));
    assert!(r_sc.output.contains("TOTAL SCORE"));

    // Kill last Klingon -> Won -> Commodore Emeritus promotion -> Citation prompt
    let r_win = s.step("phasers automatic 300", false);
    assert!(r_win.output.contains("You have smashed the Klingon invasion fleet"));
    assert!(r_win.output.contains("promotes you to Commodore Emeritus."));
    assert!(r_win.output.contains("Do you want your Commodore Emeritus Citation printed?"));
    assert!(r_win.output.contains("132 column printer"));

    let r_yes = s.step("yes", false);
    assert!(r_yes.output.contains("File or device name for your plaque:"));

    let r_fn = s.step("kirk_plaque.txt", false);
    assert!(r_fn.output.contains("Enter name to go on plaque"));

    let r_plaque = s.step("James T. Kirk", false);
    let p = r_plaque
        .plaques_updated
        .iter()
        .find(|e| e.name == "kirk_plaque.txt")
        .expect("plaque generated");
    assert!(p.content.contains("U. S. S. ENTERPRISE"));
    assert!(p.content.contains("NCC-1701"));
    assert!(p.content.contains("James T. Kirk"));
    assert!(p.content.contains("\"Commodore Emeritus\""));
    assert!(p.content.contains("Expert level"));
    assert!(p.content.contains("Nov 12 2026, 15:30:00"));
    assert!(p.content.contains("Your score:"));
    assert!(p.content.contains("Klingons per stardate:"));
}

#[test]
fn help_doc_lookup_and_hud_anticheat_visibility() {
    let mut s = TrekSession::new(1, false, "tournament 1 short novice testpass");

    // Help lookup for valid commands and `commands`
    let r_h1 = s.step("help photons", false);
    assert!(r_h1.output.contains("Mnemonic:  PHOTONS"));
    let r_h2 = s.step("help commands", false);
    assert!(r_h2.output.contains("ABBREV"));
    // Invalid command lists valid commands without HELP
    let r_h3 = s.step("help boguscmd", false);
    assert!(r_h3.output.contains("Valid commands:"));
    let _ = s.step("", false); // exit help prompt on empty line

    // HUD anti-cheat visibility: SR sensors damaged hides far sectors, chart hides unvisited & shows frozen >999
    s.with_state_mut(|st| {
        st.damage[DSRSENS] = 4.0;
        st.a.condit = IHGREEN;
        st.a.sectx = 5;
        st.a.secty = 5;
        st.quad[1][1] = IHK; // far from (5,5), must be '-'
        st.quad[5][5] = IHE;
        st.quad[5][6] = IHB; // adjacent, must be visible
        st.starch[1][1] = 0;
        st.d.galaxy[1][1] = 888; // must NOT leak
        st.starch[2][2] = 1205; // radio-damaged snapshot -> "205"
        st.d.galaxy[2][2] = 999;
        st.starch[7][7] = -1; // starbase -> ".1."
    });
    let r_hud = s.step("", false);
    assert_eq!(r_hud.hud.quad_rows[0].chars().next(), Some('-'));
    assert_eq!(r_hud.hud.quad_rows[4].split(' ').nth(4), Some("E"));
    assert_eq!(r_hud.hud.quad_rows[4].split(' ').nth(5), Some("B"));
    assert_eq!(r_hud.hud.chart_rows[0][0], "...");
    assert_eq!(r_hud.hud.chart_rows[1][1], "205");
    assert_eq!(r_hud.hud.chart_rows[6][6], ".1.");

    // coordfixed flips Y in quad_rows and chart_rows
    let r_cf = s.set_coord_fixed(true);
    assert_eq!(r_cf.hud.chart_rows[1][6], ".1."); // row y=7 is index 8-7=1, col x=7 is index 6
}
