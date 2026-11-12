//! Ship reports, short/long range scans, star chart, damage report, and battle computer ETA.

use crate::io::{huh, prout, skip, IoHandle};
use crate::scanner::{scan, ScanToken};
use crate::state::{
    square, GameState, DCOMPTR, DDRAY, DLIFSUP, DLRSENS, DSHIELD, DSRSENS, IHDOCKED, IHE,
    IHGREEN, IHRED, IHYELLOW, NDEVICE, DEVICES, SEMERITUS, SEXPERT, SFAIR, SGOOD, SNOVICE,
};

pub async fn attakreport(state: &mut GameState, io: &IoHandle) {
    if state.future[crate::state::FCDBAS] < 1e30 {
        state.proutn("Starbase in ");
        state.cramlc(1, state.a.batx, state.a.baty);
        prout(state, io, " is currently under attack.").await;
        state.proutn("It can hold out until Stardate ");
        state.cramf(state.future[crate::state::FCDBAS], 0, 1);
        prout(state, io, ".").await;
    }
    if state.a.isatb == 1 {
        state.proutn("Starbase in ");
        state.cramlc(1, state.d.isx, state.d.isy);
        prout(state, io, " is under Super-commander attack.").await;
        state.proutn("It can hold out until Stardate ");
        state.cramf(state.future[crate::state::FSCDBAS], 0, 1);
        prout(state, io, ".").await;
    }
}

pub async fn report(state: &mut GameState, io: &IoHandle, f: bool) {
    state.chew();
    let s1 = if state.a.thawed != 0 { "thawed " } else { "" };
    let s2 = match state.a.length {
        1 => "short",
        2 => "medium",
        4 => "long",
        _ => "unknown length",
    };
    let s3 = match state.a.skill {
        SNOVICE => "novice",
        SFAIR => "fair",
        SGOOD => "good",
        SEXPERT => "expert",
        SEMERITUS => "emeritus",
        _ => "skilled",
    };
    let msg = format!(
        "\nYou {} playing a {}{} {} game.\n",
        if state.a.alldone != 0 { "were" } else { "are now" },
        s1,
        s2,
        s3
    );
    state.proutn(&msg);
    if state.a.skill > SGOOD && state.a.thawed != 0 && state.a.alldone == 0 {
        prout(state, io, "No plaque is allowed.").await;
    }
    if state.a.tourn != 0 {
        let msg = format!("This is tournament game {}.\n", state.a.tourn);
        state.proutn(&msg);
    }
    if f {
        let msg = format!("Your secret password is \"{}\"\n", state.passwd);
        state.proutn(&msg);
    }
    let destroyed = state.d.killk + state.d.killc + state.d.nsckill;
    let msg = format!(
        "{} of {} Klingon ships have been destroyed",
        destroyed, state.a.inkling
    );
    state.proutn(&msg);
    if state.d.killc != 0 {
        let msg = format!(
            ", including {} Commander{}.\n",
            state.d.killc,
            if state.d.killc == 1 { "" } else { "s" }
        );
        state.proutn(&msg);
    } else if state.d.killk + state.d.nsckill > 0 {
        prout(state, io, ", but no Commanders.").await;
    } else {
        prout(state, io, ".").await;
    }
    if state.a.skill > SFAIR {
        let msg = format!(
            "The Super Commander has {}been destroyed.\n",
            if state.d.nscrem != 0 { "not " } else { "" }
        );
        state.proutn(&msg);
    }
    if state.d.rembase != state.a.inbase {
        state.proutn("There ");
        if state.a.inbase - state.d.rembase == 1 {
            state.proutn("has been 1 base");
        } else {
            state.proutn("have been ");
            state.crami(state.a.inbase - state.d.rembase, 1);
            state.proutn(" bases");
        }
        state.proutn(" destroyed, ");
        state.crami(state.d.rembase, 1);
        prout(state, io, " remaining.").await;
    } else {
        let msg = format!("There are {} bases.\n", state.a.inbase);
        state.proutn(&msg);
    }
    if state.reports() || state.a.iseenit != 0 {
        attakreport(state, io).await;
        state.a.iseenit = 1;
    }
    if state.a.casual != 0 {
        let msg = format!(
            "{} casualt{} suffered so far.\n",
            state.a.casual,
            if state.a.casual == 1 { "y" } else { "ies" }
        );
        state.proutn(&msg);
    }
    if state.a.brigcapacity != state.d.brigfree {
        let cap = state.a.brigcapacity - state.d.brigfree;
        let msg = format!(
            "{} Klingon{} in brig.\n",
            cap,
            if cap > 1 { "s" } else { "" }
        );
        state.proutn(&msg);
    }
    if state.d.kcaptured > 0 {
        let msg = format!(
            "{} captured Klingon{} turned in to Star Fleet.\n",
            state.d.kcaptured,
            if state.d.kcaptured > 1 { "s" } else { "" }
        );
        state.proutn(&msg);
    }
    if state.a.nhelp != 0 {
        let msg = format!(
            "There {} {} call{} for help.\n",
            if state.a.nhelp == 1 { "was" } else { "were" },
            state.a.nhelp,
            if state.a.nhelp == 1 { "" } else { "s" }
        );
        state.proutn(&msg);
    }
    if state.a.ship == IHE {
        state.proutn("You have ");
        if state.a.nprobes != 0 {
            state.crami(state.a.nprobes, 1);
        } else {
            state.proutn("no");
        }
        state.proutn(" deep space probe");
        if state.a.nprobes != 1 {
            state.proutn("s");
        }
        prout(state, io, ".").await;
    }
    if state.reports() && state.future[crate::state::FDSPROB] != 1e30 {
        if state.a.isarmed {
            state.proutn("An armed deep space probe is in");
        } else {
            state.proutn("A deep space probe is in");
        }
        state.cramlc(1, state.a.probecx, state.a.probecy);
        prout(state, io, ".").await;
    }
    if state.a.icrystl != 0 {
        if state.a.cryprob <= 0.05 {
            prout(
                state,
                io,
                "Dilithium crystals aboard ship...not yet used.",
            )
            .await;
        } else {
            let mut i = 0;
            let mut ai = 0.05;
            while state.a.cryprob > ai {
                ai *= 2.0;
                i += 1;
            }
            let msg = format!(
                "Dilithium crystals have been used {} time{}.\n",
                i,
                if i == 1 { "" } else { "s" }
            );
            state.proutn(&msg);
        }
    }
    skip(state, io, 1).await;
}

pub async fn lrscan(state: &mut GameState, io: &IoHandle) {
    state.chew();
    if state.damage[DLRSENS] != 0.0 {
        if state.a.condit != IHDOCKED {
            prout(state, io, "LONG-RANGE SENSORS DAMAGED.").await;
            return;
        }
        skip(state, io, 1).await;
        state.proutn("Starbase's long-range scan for");
    } else {
        skip(state, io, 1).await;
        state.proutn("Long-range scan for");
    }
    state.cramlc(1, state.a.quadx, state.a.quady);
    skip(state, io, 1).await;
    if state.coordfixed {
        for y in (state.a.quady - 1..=state.a.quady + 1).rev() {
            for x in (state.a.quadx - 1)..=(state.a.quadx + 1) {
                if x <= 0 || x > 8 || y <= 0 || y > 8 {
                    state.proutn("   -1");
                } else {
                    let val = state.d.galaxy[x as usize][y as usize];
                    let msg = format!("{:5}", val);
                    state.proutn(&msg);
                    state.starch[x as usize][y as usize] = if state.damage[crate::state::DRADIO] > 0.0 {
                        val + 1000
                    } else {
                        1
                    };
                }
            }
            state.proutn("\n");
        }
    } else {
        for x in (state.a.quadx - 1)..=(state.a.quadx + 1) {
            for y in (state.a.quady - 1)..=(state.a.quady + 1) {
                if x <= 0 || x > 8 || y <= 0 || y > 8 {
                    state.proutn("   -1");
                } else {
                    let val = state.d.galaxy[x as usize][y as usize];
                    let msg = format!("{:5}", val);
                    state.proutn(&msg);
                    state.starch[x as usize][y as usize] = if state.damage[crate::state::DRADIO] > 0.0 {
                        val + 1000
                    } else {
                        1
                    };
                }
            }
            state.proutn("\n");
        }
    }
}

pub async fn dreprt(state: &mut GameState, io: &IoHandle) {
    let mut jdam = false;
    state.chew();

    for i in 1..=NDEVICE {
        if state.damage[i] > 0.0 {
            if !jdam {
                skip(state, io, 1).await;
                prout(state, io, "DEVICE            -REPAIR TIMES-").await;
                prout(state, io, "                IN FLIGHT   DOCKED").await;
                jdam = true;
            }
            let msg = format!("  {:>16} ", DEVICES[i]);
            state.proutn(&msg);
            if i == DDRAY {
                state.proutn("           ");
                state.cramf(state.damage[i] + 0.005, 8, 2);
            } else {
                state.cramf(state.damage[i] + 0.05, 8, 2);
                state.proutn("  ");
                state.cramf(state.a.docfac * state.damage[i] + 0.005, 8, 2);
            }
            skip(state, io, 1).await;
        }
    }
    if !jdam {
        prout(state, io, "All devices functional.").await;
    }
}

pub async fn chart(state: &mut GameState, io: &IoHandle, nn: bool) {
    state.chew();
    skip(state, io, 1).await;
    if state.a.stdamtim != 1e30
        && state.a.stdamtim != state.d.date
        && state.a.condit == IHDOCKED
    {
        prout(state, io, "Spock-  \"I revised the Star Chart from the").await;
        prout(state, io, "  starbase's records.\"").await;
        skip(state, io, 1).await;
    }
    if !nn {
        prout(state, io, "STAR CHART FOR THE KNOWN GALAXY").await;
    }
    if state.a.stdamtim != 1e30 {
        if state.a.condit == IHDOCKED {
            state.a.stdamtim = state.d.date;
            for i in 1..=8 {
                for j in 1..=8 {
                    if state.starch[i][j] == 1 {
                        state.starch[i][j] = state.d.galaxy[i][j] + 1000;
                    }
                }
            }
        } else {
            state.proutn("(Last surveillance update ");
            state.cramf(state.d.date - state.a.stdamtim, 0, 1);
            prout(state, io, " stardates ago.)").await;
        }
    }
    if !nn {
        skip(state, io, 1).await;
    }

    prout(state, io, "      1    2    3    4    5    6    7    8").await;
    prout(state, io, "    ----------------------------------------").await;
    if !nn {
        prout(state, io, "  -").await;
    }

    if state.coordfixed {
        for j in (1..=8).rev() {
            let hdr = format!("{} -", j);
            state.proutn(&hdr);
            for i in 1..=8 {
                let s = state.starch[i][j];
                if s < 0 {
                    state.proutn("  .1.");
                } else if s == 0 {
                    state.proutn("  ...");
                } else if s > 999 {
                    let cell = format!("{:5}", s - 1000);
                    state.proutn(&cell);
                } else {
                    let cell = format!("{:5}", state.d.galaxy[i][j]);
                    state.proutn(&cell);
                }
            }
            prout(state, io, "  -").await;
        }
    } else {
        for i in 1..=8 {
            let hdr = format!("{} -", i);
            state.proutn(&hdr);
            for j in 1..=8 {
                let s = state.starch[i][j];
                if s < 0 {
                    state.proutn("  .1.");
                } else if s == 0 {
                    state.proutn("  ...");
                } else if s > 999 {
                    let cell = format!("{:5}", s - 1000);
                    state.proutn(&cell);
                } else {
                    let cell = format!("{:5}", state.d.galaxy[i][j]);
                    state.proutn(&cell);
                }
            }
            prout(state, io, "  -").await;
        }
    }

    if !nn {
        skip(state, io, 1).await;
        state.crmshp();
        state.proutn(" is currently in");
        state.cramlc(1, state.a.quadx, state.a.quady);
        skip(state, io, 1).await;
    }
}

pub async fn srscan(state: &mut GameState, io: &IoHandle, l: i32) {
    const REQUESTS: [&str; 11] = [
        "", "da", "co", "po", "ls", "wa", "en", "to", "sh", "kl", "ti",
    ];
    let mut leftside = true;
    let mut rightside = true;
    let mut k = 0usize;
    let mut nn = false;
    let mut good_scan = true;

    match l {
        1 => {
            if state.damage[DSRSENS] != 0.0 {
                if state.a.condit != IHDOCKED {
                    prout(state, io, "SHORT-RANGE SENSORS DAMAGED").await;
                    good_scan = false;
                } else {
                    prout(state, io, "[Using starbase's sensors]").await;
                }
            }
            if good_scan {
                let qx = state.a.quadx as usize;
                let qy = state.a.quady as usize;
                state.starch[qx][qy] = if state.damage[crate::state::DRADIO] > 0.0 {
                    state.d.galaxy[qx][qy] + 1000
                } else {
                    1
                };
            }
            scan(state, io).await;
            if state.isit("chart") {
                nn = true;
            }
            if state.isit("no") {
                rightside = false;
            }
            state.chew();
            prout(state, io, "\n    1 2 3 4 5 6 7 8 9 10").await;
        }
        2 => {
            while scan(state, io).await == ScanToken::Eol {
                state.proutn("Information desired? ");
            }
            state.chew();
            let mut found = 11usize;
            for idx in 1..=10 {
                let n = state.citem.len().min(2);
                if n > 0 && REQUESTS[idx].as_bytes()[..n] == state.citem.as_bytes()[..n] {
                    found = idx;
                    break;
                }
            }
            k = found;
            if k > 10 {
                prout(
                    state,
                    io,
                    "UNRECOGNIZED REQUEST. Legal requests are:\n  date, condition, position, lsupport, warpfactor,\n  energy, torpedoes, shields, klingons, time.",
                )
                .await;
                return;
            }
            state.chew();
            leftside = false;
            skip(state, io, 1).await;
        }
        3 => {
            state.chew();
            leftside = false;
            skip(state, io, 1).await;
        }
        _ => {}
    }

    for i in 1..=10 {
        let jj = if k != 0 { k } else { i };
        if leftside {
            if state.coordfixed {
                let y = 11 - i;
                let hdr = format!("{:2}  ", y);
                state.proutn(&hdr);
                for j in 1..=10 {
                    if good_scan
                        || ((y as i32 - state.a.secty).abs() <= 1
                            && (j as i32 - state.a.sectx).abs() <= 1)
                    {
                        let ch = state.quad[j][y];
                        let s = format!("{} ", ch);
                        state.proutn(&s);
                    } else {
                        state.proutn("- ");
                    }
                }
            } else {
                let hdr = format!("{:2}  ", i);
                state.proutn(&hdr);
                for j in 1..=10 {
                    if good_scan
                        || ((i as i32 - state.a.sectx).abs() <= 1
                            && (j as i32 - state.a.secty).abs() <= 1)
                    {
                        let ch = state.quad[i][j];
                        let s = format!("{} ", ch);
                        state.proutn(&s);
                    } else {
                        state.proutn("- ");
                    }
                }
            }
        }
        if rightside {
            match jj {
                1 => {
                    let msg = format!(" Stardate      {:.1}", state.d.date);
                    state.proutn(&msg);
                }
                2 => {
                    if state.a.condit != IHDOCKED {
                        state.newcnd();
                    }
                    let cp = match state.a.condit {
                        IHRED => "RED",
                        IHGREEN => "GREEN",
                        IHYELLOW => "YELLOW",
                        IHDOCKED => "DOCKED",
                        _ => "",
                    };
                    let msg = format!(" Condition     {}", cp);
                    state.proutn(&msg);
                    if state.a.iscloaked {
                        state.proutn(", CLOAKED");
                    }
                }
                3 => {
                    state.proutn(" Position     ");
                    state.cramlc(0, state.a.quadx, state.a.quady);
                    state.proutn(",");
                    state.cramlc(0, state.a.sectx, state.a.secty);
                }
                4 => {
                    state.proutn(" Life Support  ");
                    if state.damage[DLIFSUP] != 0.0 {
                        if state.a.condit == IHDOCKED {
                            state.proutn("DAMAGED, supported by starbase");
                        } else {
                            let msg = format!("DAMAGED, reserves={:4.2}", state.a.lsupres);
                            state.proutn(&msg);
                        }
                    } else {
                        state.proutn("ACTIVE");
                    }
                }
                5 => {
                    let msg = format!(" Warp Factor   {:.1}", state.a.warpfac);
                    state.proutn(&msg);
                }
                6 => {
                    let msg = format!(" Energy        {:.2}", state.a.energy);
                    state.proutn(&msg);
                }
                7 => {
                    let msg = format!(" Torpedoes     {}", state.a.torps);
                    state.proutn(&msg);
                }
                8 => {
                    state.proutn(" Shields       ");
                    if state.damage[DSHIELD] != 0.0 {
                        state.proutn("DAMAGED,");
                    } else if state.a.shldup != 0 {
                        state.proutn("UP,");
                    } else {
                        state.proutn("DOWN,");
                    }
                    let pct = ((100.0 * state.a.shield) / state.a.inshld + 0.5) as i32;
                    let msg = format!(" {}% {:.1} units", pct, state.a.shield);
                    state.proutn(&msg);
                }
                9 => {
                    let msg = format!(" Klingons Left {}", state.d.remkl);
                    state.proutn(&msg);
                }
                10 => {
                    let msg = format!(" Time Left     {:.2}", state.d.remtime);
                    state.proutn(&msg);
                }
                _ => {}
            }
        }
        skip(state, io, 1).await;
        if k != 0 {
            return;
        }
    }
    if nn {
        chart(state, io, true).await;
    }
}

pub async fn eta(state: &mut GameState, io: &IoHandle) {
    let mut prompt = false;
    if state.damage[DCOMPTR] != 0.0 {
        prout(state, io, "COMPUTER DAMAGED, USE A POCKET CALCULATOR.").await;
        skip(state, io, 1).await;
        return;
    }
    if scan(state, io).await != ScanToken::Real {
        prompt = true;
        state.chew();
        state.proutn("Destination quadrant and/or sector? ");
        if scan(state, io).await != ScanToken::Real {
            huh(state, io).await;
            return;
        }
    }
    let mut iy1 = (state.aaitem + 0.5) as i32;
    if scan(state, io).await != ScanToken::Real {
        huh(state, io).await;
        return;
    }
    let mut ix1 = (state.aaitem + 0.5) as i32;
    let ix2: i32;
    let iy2: i32;
    if scan(state, io).await == ScanToken::Real {
        iy2 = (state.aaitem + 0.5) as i32;
        if scan(state, io).await != ScanToken::Real {
            huh(state, io).await;
            return;
        }
        ix2 = (state.aaitem + 0.5) as i32;
    } else {
        ix2 = ix1;
        iy2 = iy1;
        ix1 = state.a.quady;
        iy1 = state.a.quadx;
    }

    if ix1 > 8 || ix1 < 1 || iy1 > 8 || iy1 < 1 || ix2 > 10 || ix2 < 1 || iy2 > 10 || iy2 < 1 {
        huh(state, io).await;
        return;
    }
    state.a.dist = (square((iy1 - state.a.quadx) as f64 + 0.1 * ((iy2 - state.a.sectx) as f64))
        + square((ix1 - state.a.quady) as f64 + 0.1 * ((ix2 - state.a.secty) as f64)))
    .sqrt();
    let mut wfl = false;
    let mut twarp;

    if prompt {
        prout(state, io, "Answer \"no\" if you don't know the value:").await;
    }
    loop {
        state.chew();
        state.proutn("Time or arrival date? ");
        if scan(state, io).await == ScanToken::Real {
            let mut ttime = state.aaitem;
            if ttime > state.d.date {
                ttime -= state.d.date;
            }
            twarp = ((((10.0 * state.a.dist) / ttime).sqrt() * 10.0).floor() + 1.0) / 10.0;
            if ttime <= 1e-10 || twarp > 10.0 {
                prout(state, io, "We'll never make it, sir.").await;
                state.chew();
                return;
            }
            if twarp < 1.0 {
                twarp = 1.0;
            }
            break;
        }
        state.chew();
        state.proutn("Warp factor? ");
        if scan(state, io).await == ScanToken::Real {
            wfl = true;
            twarp = state.aaitem;
            if twarp < 1.0 || twarp > 10.0 {
                huh(state, io).await;
                return;
            }
            break;
        }
        prout(
            state,
            io,
            "Captain, certainly you can give me one of these.",
        )
        .await;
    }

    loop {
        state.chew();
        let ttime = (10.0 * state.a.dist) / square(twarp);
        let mut tpower =
            state.a.dist * twarp * twarp * twarp * ((state.a.shldup + 1) as f64);
        if tpower >= state.a.energy {
            prout(state, io, "Insufficient energy, sir: we would need ").await;
            state.cramf(tpower, 1, 1);
            state.proutn(" units.");
            if state.a.shldup == 0 || tpower > state.a.energy * 2.0 {
                if !wfl {
                    return;
                }
                state.proutn("New warp factor to try? ");
                if scan(state, io).await == ScanToken::Real {
                    wfl = true;
                    twarp = state.aaitem;
                    if twarp < 1.0 || twarp > 10.0 {
                        huh(state, io).await;
                        return;
                    }
                    continue;
                } else {
                    state.chew();
                    skip(state, io, 1).await;
                    return;
                }
            }
            prout(state, io, "But if you lower your shields,").await;
            state.proutn("remaining");
            tpower /= 2.0;
        } else {
            state.proutn("Remaining");
        }
        state.proutn(" energy will be ");
        state.cramf(state.a.energy - tpower, 1, 1);
        prout(state, io, ".").await;
        if wfl {
            state.proutn("And we will arrive at stardate ");
            state.cramf(state.d.date + ttime, 1, 1);
            prout(state, io, ".").await;
        } else if twarp == 1.0 {
            prout(state, io, "Any warp speed is adequate.").await;
        } else {
            state.proutn("Minimum warp needed is ");
            state.cramf(twarp, 1, 2);
            skip(state, io, 1).await;
            state.proutn("and we will arrive at stardate ");
            state.cramf(state.d.date + ttime, 1, 2);
            prout(state, io, ".").await;
        }
        if state.d.remtime < ttime {
            prout(
                state,
                io,
                "Unfortunately, the Federation will be destroyed by then.",
            )
            .await;
        }
        if twarp > 6.0 {
            prout(state, io, "You'll be taking risks at that speed, Captain").await;
        }
        if (state.a.isatb == 1
            && state.d.isy == ix1
            && state.d.isx == iy1
            && state.future[crate::state::FSCDBAS] < ttime + state.d.date)
            || (state.future[crate::state::FCDBAS] < ttime + state.d.date
                && state.a.baty == ix1
                && state.a.batx == iy1)
        {
            prout(state, io, "The starbase there will be destroyed by then.").await;
        }
        state.proutn("New warp factor to try? ");
        if scan(state, io).await == ScanToken::Real {
            wfl = true;
            twarp = state.aaitem;
            if twarp < 1.0 || twarp > 10.0 {
                huh(state, io).await;
                return;
            }
        } else {
            state.chew();
            skip(state, io, 1).await;
            return;
        }
    }
}
