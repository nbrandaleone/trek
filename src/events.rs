//! Scheduled and random galaxy events: supernovae, novae, tractor beams, snapshots, and probe.

use crate::ai::{movetho, scom};
use crate::battle::{attack, deadkl, sheild};
use crate::finish::finish;
use crate::io::{huh, pause, prout, prouts, skip, IoHandle};
use crate::moving::{atover, lmove};
use crate::reports::attakreport;
use crate::scanner::{ja, scan, ScanToken};
use crate::setup::newqad;
use crate::state::{
    square, FinType, GameState, Planet, DDRAY, DLIFSUP, DPHASER, DPHOTON, DRADIO, DSHIELD,
    DSHUTTL, FBATTAK, FCDBAS, FDSPROB, FSNAP, FSCDBAS, FSCMOVE, FSNOVA, FSPY, FTBEAM, IHB, IHC,
    IHDOCKED, IHDOT, IHE, IHF, IHK, IHP, IHR, IHS, IHSTAR, NDEVICE, NEVENTS,
};

pub async fn events(state: &mut GameState, io: &IoHandle) {
    let mut ictbeam = 0;
    let mut ipage = 0;
    let mut istract = 0;
    let mut fintim = state.d.date + state.a.time;

    if state.debug_mode {
        prout(state, io, "EVENTS").await;
    }

    if state.a.stdamtim == 1e30 && !state.reports() {
        state.a.stdamtim = state.d.date;
        for i in 1..=8 {
            for j in 1..=8 {
                if state.starch[i][j] == 1 {
                    state.starch[i][j] = state.d.galaxy[i][j] + 1000;
                }
            }
        }
    }

    loop {
        let mut line = FSPY;
        if state.a.alldone != 0 {
            return;
        }
        let mut datemin = fintim;
        for l in 1..=NEVENTS {
            if state.future[l] <= datemin {
                line = l;
                datemin = state.future[l];
            }
        }
        let xtime = datemin - state.d.date;

        if state.a.iscloaking {
            state.a.energy -= xtime * 500.0;
            if state.a.energy <= 0.0 {
                finish(state, io, FinType::Nrg).await;
                return;
            }
        }

        state.d.date = datemin;
        state.d.remres -= ((state.d.remkl + 4 * state.d.remcom) as f64) * xtime;
        state.d.remtime = state.d.remres / ((state.d.remkl + 4 * state.d.remcom) as f64);
        if state.d.remtime <= 0.0 {
            finish(state, io, FinType::Deplete).await;
            return;
        }

        if state.damage[DLIFSUP] != 0.0 && state.a.condit != IHDOCKED {
            if state.a.lsupres < xtime && state.damage[DLIFSUP] > state.a.lsupres {
                finish(state, io, FinType::Lifesup).await;
                return;
            }
            state.a.lsupres -= xtime;
            if state.damage[DLIFSUP] <= xtime {
                state.a.lsupres = state.a.inlsr;
            }
        }

        let mut repair = xtime;
        if state.a.condit == IHDOCKED {
            repair /= state.a.docfac;
        }
        for l in 1..=NDEVICE {
            if state.damage[l] > 0.0 && l != DDRAY {
                if state.damage[l] > repair {
                    state.damage[l] -= repair;
                } else {
                    state.damage[l] = 0.0;
                }
            }
        }
        if state.damage[DDRAY] > 0.0 && state.a.condit == IHDOCKED {
            if state.damage[DDRAY] > xtime {
                state.damage[DDRAY] -= xtime;
            } else {
                state.damage[DDRAY] = 0.0;
            }
        }

        if state.a.stdamtim != 1e30 && state.reports() {
            state.a.stdamtim = 1e30;
            prout(
                state,
                io,
                "Lt. Uhura- \"Captain, the sub-space radio is working and",
            )
            .await;
            prout(state, io, "   surveillance reports are coming in.").await;
            skip(state, io, 1).await;
            for i in 1..=8 {
                for j in 1..=8 {
                    if state.starch[i][j] > 999 {
                        state.starch[i][j] = 1;
                    }
                }
            }
            if state.a.iseenit == 0 {
                attakreport(state, io).await;
                state.a.iseenit = 1;
            }
            skip(state, io, 1).await;
            prout(state, io, "   The star chart is now up to date.\"").await;
            skip(state, io, 1).await;
        }

        state.a.time -= xtime;
        match line {
            FSNOVA => {
                if ipage == 0 {
                    pause(state, io, 1).await;
                }
                ipage = 1;
                snova(state, io, 0, 0).await;
                state.future[FSNOVA] = state.d.date + state.rng.expran(0.5 * state.a.intime);
                if state.d.galaxy[state.a.quadx as usize][state.a.quady as usize] == 1000 {
                    return;
                }
            }
            FSPY | FTBEAM => {
                let mut do_tbeam = false;
                let mut yank = 0.0;
                let mut com_idx = 0usize;

                if line == FSPY {
                    if state.d.nscrem == 0
                        || state.a.iscloaked
                        || ictbeam + istract > 0
                        || state.a.condit == IHDOCKED
                        || state.a.isatb == 1
                        || state.a.iscate == 1
                    {
                        return;
                    }
                    if state.a.ientesc != 0
                        || (state.a.energy < 2000.0
                            && state.a.torps < 4
                            && state.a.shield < 1250.0)
                        || (state.damage[DPHASER] > 0.0
                            && (state.damage[DPHOTON] > 0.0 || state.a.torps < 4))
                        || (state.damage[DSHIELD] > 0.0
                            && (state.a.energy < 2500.0 || state.damage[DPHASER] > 0.0)
                            && (state.a.torps < 5 || state.damage[DPHOTON] > 0.0))
                    {
                        istract = 1;
                        yank = square((state.d.isx - state.a.quadx) as f64)
                            + square((state.d.isy - state.a.quady) as f64);
                        do_tbeam = true;
                    } else {
                        return;
                    }
                } else {
                    if state.d.remcom == 0 {
                        state.future[FTBEAM] = 1e30;
                    } else {
                        com_idx =
                            (state.rng.rand_f64() * (state.d.remcom as f64) + 1.0) as usize;
                        yank = square((state.d.cx[com_idx] - state.a.quadx) as f64)
                            + square((state.d.cy[com_idx] - state.a.quady) as f64);
                        if istract != 0
                            || state.a.condit == IHDOCKED
                            || state.a.iscloaked
                            || yank == 0.0
                        {
                            state.future[FTBEAM] = state.d.date
                                + state.a.time
                                + state
                                    .rng
                                    .expran(1.5 * state.a.intime / (state.d.remcom as f64));
                        } else {
                            do_tbeam = true;
                        }
                    }
                }

                if do_tbeam {
                    yank = yank.sqrt();
                    if ipage == 0 {
                        pause(state, io, 1).await;
                    }
                    ipage = 1;
                    state.a.time = (10.0 / (7.5 * 7.5)) * yank;
                    ictbeam = 1;
                    skip(state, io, 1).await;
                    state.proutn("***");
                    state.crmshp();
                    prout(state, io, " caught in long range tractor beam--").await;
                    atover(state, io, true).await;
                    if state.a.alldone != 0 {
                        return;
                    }
                    if state.a.icraft == 1 {
                        finish(state, io, FinType::Stractor).await;
                        return;
                    }
                    if state.a.iscraft == 0 {
                        skip(state, io, 1).await;
                        if state.rng.rand_f64() > 0.5 {
                            prout(
                                state,
                                io,
                                "Galileo, left on the planet surface, is captured",
                            )
                            .await;
                            prout(state, io, "by aliens and made into a flying McDonald's.").await;
                            state.damage[DSHUTTL] = -10.0;
                            state.a.iscraft = -1;
                        } else {
                            prout(
                                state,
                                io,
                                "Galileo, left on the planet surface, is well hidden.",
                            )
                            .await;
                        }
                    }
                    if line == 0 {
                        state.a.quadx = state.d.isx;
                        state.a.quady = state.d.isy;
                    } else {
                        state.a.quadx = state.d.cx[com_idx];
                        state.a.quady = state.d.cy[com_idx];
                    }
                    let (sx, sy) = state.rng.iran10();
                    state.a.sectx = sx;
                    state.a.secty = sy;
                    state.crmshp();
                    state.proutn(" is pulled to");
                    state.cramlc(1, state.a.quadx, state.a.quady);
                    state.proutn(", ");
                    state.cramlc(2, state.a.sectx, state.a.secty);
                    skip(state, io, 1).await;
                    if state.a.resting != 0 {
                        prout(
                            state,
                            io,
                            "(Remainder of rest/repair period cancelled.)",
                        )
                        .await;
                        state.a.resting = 0;
                    }
                    if state.a.shldup == 0 {
                        if state.damage[DSHIELD] == 0.0 && state.a.shield > 0.0 {
                            sheild(state, io, 2).await;
                            state.a.shldchg = 0;
                        } else {
                            prout(state, io, "(Shields not currently usable.)").await;
                        }
                    }
                    newqad(state, io, false).await;
                    fintim = state.d.date + state.a.time;
                    if state.d.remcom <= 0 {
                        state.future[FTBEAM] = 1e30;
                    } else {
                        state.future[FTBEAM] = state.d.date
                            + state.a.time
                            + state
                                .rng
                                .expran(1.5 * state.a.intime / (state.d.remcom as f64));
                    }
                }
            }
            FSNAP => {
                state.snapsht = state.d.clone();
                state.d.snap = 1;
                state.future[FSNAP] = state.d.date + state.rng.expran(0.5 * state.a.intime);
            }
            FBATTAK => {
                if state.d.remcom == 0 || state.d.rembase == 0 {
                    state.future[FBATTAK] = 1e30;
                    state.future[FCDBAS] = 1e30;
                } else {
                    let mut found_j = 0usize;
                    for j in 1..=(state.d.rembase as usize) {
                        let bx = state.d.baseqx[j];
                        let by = state.d.baseqy[j];
                        let mut matched = false;
                        for k in 1..=(state.d.remcom as usize) {
                            if bx == state.d.cx[k]
                                && by == state.d.cy[k]
                                && (bx != state.a.quadx || by != state.a.quady)
                                && (bx != state.d.isx || by != state.d.isy)
                            {
                                matched = true;
                                break;
                            }
                        }
                        if matched {
                            found_j = j;
                            break;
                        }
                    }
                    if found_j == 0 {
                        state.future[FBATTAK] =
                            state.d.date + state.rng.expran(0.3 * state.a.intime);
                        state.future[FCDBAS] = 1e30;
                    } else {
                        state.a.batx = state.d.baseqx[found_j];
                        state.a.baty = state.d.baseqy[found_j];
                        state.future[FCDBAS] =
                            state.d.date + 1.0 + 3.0 * state.rng.rand_f64();
                        if state.a.isatb != 0 {
                            state.future[FCDBAS] += state.future[FSCDBAS] - state.d.date;
                        }
                        state.future[FBATTAK] =
                            state.future[FCDBAS] + state.rng.expran(0.3 * state.a.intime);
                        state.a.iseenit = 0;
                        if state.reports() {
                            state.a.iseenit = 1;
                            if ipage == 0 {
                                pause(state, io, 1).await;
                            }
                            ipage = 1;
                            skip(state, io, 1).await;
                            state.proutn("Lt. Uhura-  \"Captain, the starbase in");
                            state.cramlc(1, state.a.batx, state.a.baty);
                            skip(state, io, 1).await;
                            prout(
                                state,
                                io,
                                "   reports that it is under atttack and that it can",
                            )
                            .await;
                            state.proutn("   hold out only until stardate ");
                            state.cramf(state.future[FCDBAS], 1, 1);
                            prout(state, io, ".\"").await;
                            if state.a.resting != 0 {
                                skip(state, io, 1).await;
                                state.proutn(
                                    "Mr. Spock-  \"Captain, shall we cancel the rest period?\"",
                                );
                                if ja(state, io).await {
                                    state.a.resting = 0;
                                    state.a.time = 0.0;
                                    return;
                                }
                            }
                        }
                    }
                }
            }
            FSCDBAS | FCDBAS => {
                let mut ixhold = 0;
                let mut iyhold = 0;
                let mut proceed = true;

                if line == FSCDBAS {
                    state.future[FSCDBAS] = 1e30;
                    state.a.isatb = 2;
                    if state.d.galaxy[state.d.isx as usize][state.d.isy as usize] % 100 < 10 {
                        proceed = false;
                    } else {
                        ixhold = state.a.batx;
                        iyhold = state.a.baty;
                        state.a.batx = state.d.isx;
                        state.a.baty = state.d.isy;
                    }
                } else {
                    state.future[FCDBAS] = 1e30;
                    let mut found = false;
                    for i in 1..=(state.d.remcom as usize) {
                        if state.d.cx[i] == state.a.batx && state.d.cy[i] == state.a.baty {
                            found = true;
                            break;
                        }
                    }
                    if !found
                        || state.d.rembase == 0
                        || state.d.galaxy[state.a.batx as usize][state.a.baty as usize] % 100 < 10
                    {
                        state.a.batx = 0;
                        state.a.baty = 0;
                        proceed = false;
                    }
                }

                if proceed {
                    let bx = state.a.batx as usize;
                    let by = state.a.baty as usize;
                    if state.starch[bx][by] == -1 {
                        state.starch[bx][by] = 0;
                    }
                    if state.a.batx == state.a.quadx && state.a.baty == state.a.quady {
                        if state.starch[bx][by] > 999 {
                            state.starch[bx][by] -= 10;
                        }
                        state.quad[state.a.basex as usize][state.a.basey as usize] = IHDOT;
                        state.a.basex = 0;
                        state.a.basey = 0;
                        state.newcnd();
                        skip(state, io, 1).await;
                        prout(
                            state,
                            io,
                            "Spock-  \"Captain, I believe the starbase has been destroyed.\"",
                        )
                        .await;
                    } else if state.d.rembase != 1 && state.reports() {
                        if ipage == 0 {
                            pause(state, io, 1).await;
                        }
                        ipage = 1;
                        skip(state, io, 1).await;
                        prout(
                            state,
                            io,
                            "Lt. Uhura-  \"Captain, Starfleet Command reports that",
                        )
                        .await;
                        state.proutn("   the starbase in");
                        state.cramlc(1, state.a.batx, state.a.baty);
                        prout(state, io, " has been destroyed by").await;
                        if state.a.isatb == 2 {
                            prout(state, io, "the Klingon Super-Commander").await;
                        } else {
                            prout(state, io, "a Klingon Commander").await;
                        }
                    }

                    state.d.galaxy[bx][by] -= 10;
                    let remb = state.d.rembase as usize;
                    for i in 1..=remb {
                        if state.d.baseqx[i] == state.a.batx && state.d.baseqy[i] == state.a.baty
                        {
                            state.d.baseqx[i] = state.d.baseqx[remb];
                            state.d.baseqy[i] = state.d.baseqy[remb];
                        }
                    }
                    state.d.rembase -= 1;
                    if state.a.isatb == 2 {
                        state.a.batx = ixhold;
                        state.a.baty = iyhold;
                        state.a.isatb = 0;
                    } else {
                        state.a.batx = 0;
                        state.a.baty = 0;
                    }
                }
            }
            FSCMOVE => {
                state.future[FSCMOVE] = state.d.date + 0.2777;
                if state.a.ientesc + istract == 0
                    && state.a.isatb != 1
                    && (state.a.iscate != 1 || state.a.justin == 1)
                {
                    scom(state, io, &mut ipage).await;
                }
            }
            FDSPROB => {
                state.future[FDSPROB] = state.d.date + 0.01;
                state.a.probex += state.a.probeinx;
                state.a.probey += state.a.probeiny;
                let i = (state.a.probex / 10.0 + 0.05) as i32;
                let j = (state.a.probey / 10.0 + 0.05) as i32;
                if state.a.probecx != i || state.a.probecy != j {
                    state.a.probecx = i;
                    state.a.probecy = j;
                    if i < 1
                        || i > 8
                        || j < 1
                        || j > 8
                        || state.d.galaxy[i as usize][j as usize] == 1000
                    {
                        if state.reports() {
                            if ipage == 0 {
                                pause(state, io, 1).await;
                            }
                            ipage = 1;
                            skip(state, io, 1).await;
                            state.proutn("Lt. Uhura-  \"The deep space probe ");
                            if i < 1 || i > 8 || j < 1 || j > 8 {
                                state.proutn("has left the galaxy");
                            } else {
                                state.proutn("is no longer transmitting");
                            }
                            prout(state, io, ".\"").await;
                        }
                        state.future[FDSPROB] = 1e30;
                        continue;
                    }
                    if state.reports() {
                        if ipage == 0 {
                            pause(state, io, 1).await;
                        }
                        ipage = 1;
                        skip(state, io, 1).await;
                        state.proutn("Lt. Uhura-  \"The deep space probe is now in ");
                        state.cramlc(1, state.a.probecx, state.a.probecy);
                        prout(state, io, ".\"").await;
                    }
                }
                if state.reports() {
                    let px = state.a.probecx as usize;
                    let py = state.a.probecy as usize;
                    state.starch[px][py] = if state.damage[DRADIO] > 0.0 {
                        state.d.galaxy[px][py] + 1000
                    } else {
                        1
                    };
                }
                state.a.proben -= 1;
                if state.a.proben == 0
                    && state.a.isarmed
                    && state.d.galaxy[state.a.probecx as usize][state.a.probecy as usize] % 10 > 0
                {
                    snova(state, io, 1, 0).await;
                    state.future[FDSPROB] = 1e30;
                    if state.d.galaxy[state.a.quadx as usize][state.a.quady as usize] == 1000 {
                        return;
                    }
                }
            }
            _ => {}
        }
    }
}

pub async fn waiting(state: &mut GameState, io: &IoHandle) {
    state.a.ididit = 0;
    let mut key;
    loop {
        key = scan(state, io).await;
        if key != ScanToken::Eol {
            break;
        }
        state.proutn("How long? ");
    }
    state.chew();
    if key != ScanToken::Real {
        huh(state, io).await;
        return;
    }
    let mut delay = state.aaitem;
    if delay <= 0.0 {
        return;
    }
    if delay >= state.d.remtime || state.a.nenhere != 0 {
        prout(state, io, "Are you sure? ").await;
        if !ja(state, io).await {
            return;
        }
    }

    state.a.resting = 1;
    loop {
        if delay <= 0.0 {
            state.a.resting = 0;
        }
        if state.a.resting == 0 {
            state.cramf(state.d.remtime, 0, 2);
            prout(state, io, " stardates left.").await;
            return;
        }
        let mut temp = delay;
        state.a.time = delay;

        if state.a.nenhere != 0 {
            let rtime = 1.0 + state.rng.rand_f64();
            if rtime < temp {
                temp = rtime;
            }
            state.a.time = temp;
        }
        if state.a.time < delay {
            attack(state, io, 0).await;
        }
        if state.a.nenhere == 0 {
            movetho(state, io).await;
        }
        if state.a.alldone != 0 {
            return;
        }
        events(state, io).await;
        state.a.ididit = 1;
        if state.a.alldone != 0 {
            return;
        }
        delay -= temp;
        if state.d.galaxy[state.a.quadx as usize][state.a.quady as usize] == 1000 {
            break;
        }
    }

    state.a.resting = 0;
    state.a.time = 0.0;
}

pub async fn nova(state: &mut GameState, io: &IoHandle, ix: i32, iy: i32) {
    const COURSE: [f64; 10] = [0.0, 10.5, 12.0, 1.5, 9.0, 0.0, 3.0, 7.5, 6.0, 4.5];

    if state.rng.rand_f64() < 0.05 {
        snova(state, io, ix, iy).await;
        return;
    }

    state.quad[ix as usize][iy as usize] = IHDOT;
    state.crmena(1, IHSTAR, 2, ix, iy);
    prout(state, io, " novas.").await;
    state.d.galaxy[state.a.quadx as usize][state.a.quady as usize] -= 1;
    state.d.starkl += 1;

    let mut hits: Vec<(i32, i32)> = vec![(ix, iy)];
    let mut bot = 0usize;
    let mut top = 0usize;
    let mut kount = 0i32;
    let mut icx = 0i32;
    let mut icy = 0i32;

    loop {
        for mm in bot..=top {
            let (hx, hy) = hits[mm];
            for nn in 1..=3 {
                for j in 1..=3 {
                    if j == 2 && nn == 2 {
                        continue;
                    }
                    let ii = hx + nn - 2;
                    let jj = hy + j - 2;
                    if !(1..=10).contains(&ii) || !(1..=10).contains(&jj) {
                        continue;
                    }
                    let iquad = state.quad[ii as usize][jj as usize];
                    match iquad {
                        IHSTAR => {
                            if state.rng.rand_f64() < 0.05 {
                                snova(state, io, ii, jj).await;
                                return;
                            }
                            hits.push((ii, jj));
                            state.d.galaxy[state.a.quadx as usize][state.a.quady as usize] -= 1;
                            state.d.starkl += 1;
                            state.crmena(1, IHSTAR, 2, ii, jj);
                            prout(state, io, " novas.").await;
                            state.quad[ii as usize][jj as usize] = IHDOT;
                        }
                        IHP => {
                            state.d.newstuf[state.a.quadx as usize][state.a.quady as usize] -= 1;
                            state.d.nplankl += 1;
                            state.crmena(1, IHP, 2, ii, jj);
                            prout(state, io, " destroyed.").await;
                            state.d.plnets[state.a.iplnet as usize] = Planet::default();
                            state.a.iplnet = 0;
                            state.a.plnetx = 0;
                            state.a.plnety = 0;
                            if state.a.landed == 1 {
                                finish(state, io, FinType::Pnova).await;
                                return;
                            }
                            state.quad[ii as usize][jj as usize] = IHDOT;
                        }
                        IHB => {
                            state.d.galaxy[state.a.quadx as usize][state.a.quady as usize] -= 10;
                            let remb = state.d.rembase as usize;
                            for i in 1..=remb {
                                if state.d.baseqx[i] == state.a.quadx
                                    && state.d.baseqy[i] == state.a.quady
                                {
                                    state.d.baseqx[i] = state.d.baseqx[remb];
                                    state.d.baseqy[i] = state.d.baseqy[remb];
                                    break;
                                }
                            }
                            state.d.rembase -= 1;
                            state.a.basex = 0;
                            state.a.basey = 0;
                            state.d.basekl += 1;
                            state.newcnd();
                            state.crmena(1, IHB, 2, ii, jj);
                            prout(state, io, " destroyed.").await;
                            state.quad[ii as usize][jj as usize] = IHDOT;
                        }
                        IHE | IHF => {
                            prout(state, io, "***Starship buffeted by nova.").await;
                            if state.a.shldup != 0 {
                                if state.a.shield >= 2000.0 {
                                    state.a.shield -= 2000.0;
                                } else {
                                    let diff = 2000.0 - state.a.shield;
                                    state.a.energy -= diff;
                                    state.a.shield = 0.0;
                                    state.a.shldup = 0;
                                    prout(state, io, "***Shields knocked out.").await;
                                    state.damage[DSHIELD] +=
                                        0.005 * state.a.damfac * state.rng.rand_f64() * diff;
                                }
                            } else {
                                state.a.energy -= 2000.0;
                            }
                            if state.a.energy <= 0.0 {
                                finish(state, io, FinType::Nova).await;
                                return;
                            }
                            icx += state.a.sectx - hx;
                            icy += state.a.secty - hy;
                            kount += 1;
                        }
                        IHK => {
                            deadkl(state, io, ii, jj, iquad, ii, jj).await;
                        }
                        IHC | IHS | IHR => {
                            let mut ll = 1usize;
                            while ll <= state.a.nenhere as usize {
                                if state.kx[ll] == ii && state.ky[ll] == jj {
                                    break;
                                }
                                ll += 1;
                            }
                            if ll <= state.a.nenhere as usize {
                                state.kpower[ll] -= 800.0;
                                if state.kpower[ll] <= 0.0 {
                                    deadkl(state, io, ii, jj, iquad, ii, jj).await;
                                    continue;
                                }
                                let newcx = ii + ii - hx;
                                let newcy = jj + jj - hy;
                                state.crmena(1, iquad, 2, ii, jj);
                                state.proutn(" damaged");
                                if !(1..=10).contains(&newcx) || !(1..=10).contains(&newcy) {
                                    skip(state, io, 1).await;
                                    continue;
                                }
                                let iquad1 = state.quad[newcx as usize][newcy as usize];
                                if iquad1 == crate::state::IHBLANK {
                                    state.proutn(", blasted into ");
                                    state.crmena(0, crate::state::IHBLANK, 2, newcx, newcy);
                                    skip(state, io, 1).await;
                                    deadkl(state, io, ii, jj, iquad, newcx, newcy).await;
                                    continue;
                                }
                                if iquad1 != IHDOT {
                                    skip(state, io, 1).await;
                                    continue;
                                }
                                state.proutn(", buffeted to");
                                state.cramlc(2, newcx, newcy);
                                state.quad[ii as usize][jj as usize] = IHDOT;
                                state.quad[newcx as usize][newcy as usize] = iquad;
                                state.kx[ll] = newcx;
                                state.ky[ll] = newcy;
                                let d = (square((state.a.sectx - newcx) as f64)
                                    + square((state.a.secty - newcy) as f64))
                                .sqrt();
                                state.kavgd[ll] = d;
                                state.kdist[ll] = d;
                                skip(state, io, 1).await;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        let new_top = hits.len() - 1;
        if top == new_top {
            break;
        }
        bot = top + 1;
        top = new_top;
    }

    if kount == 0 {
        return;
    }

    state.a.dist = (kount as f64) * 0.1;
    if icx != 0 {
        icx = if icx < 0 { -1 } else { 1 };
    }
    if icy != 0 {
        icy = if icy < 0 { -1 } else { 1 };
    }
    state.a.direc = COURSE[(3 * (icx + 1) + icy + 2) as usize];
    if state.a.direc == 0.0 {
        state.a.dist = 0.0;
    }
    if state.a.dist == 0.0 {
        return;
    }
    state.a.time = 10.0 * state.a.dist / 16.0;
    skip(state, io, 1).await;
    prout(state, io, "Force of nova displaces starship.").await;
    state.a.iattak = 2;
    lmove(state, io).await;
    state.a.time = 10.0 * state.a.dist / 16.0;
}

pub async fn snova(state: &mut GameState, io: &IoHandle, insx: i32, insy: i32) {
    let mut nqx = 1i32;
    let mut nqy = 1i32;
    let mut nsx = insx;
    let mut nsy = insy;
    let mut insipient = false;

    if insy == 0 {
        if insx == 1 {
            nqx = state.a.probecx;
            nqy = state.a.probecy;
        } else {
            let mut stars_cnt = 0;
            for qx in 1..=8 {
                for qy in 1..=8 {
                    stars_cnt += state.d.galaxy[qx][qy] % 10;
                }
            }
            if stars_cnt == 0 {
                return;
            }
            let mut num = (state.rng.rand_f64() * (stars_cnt as f64) + 1.0) as i32;
            'outer: for qx in 1..=8 {
                for qy in 1..=8 {
                    num -= state.d.galaxy[qx][qy] % 10;
                    if num <= 0 {
                        nqx = qx as i32;
                        nqy = qy as i32;
                        break 'outer;
                    }
                }
            }
            if state.debug_mode {
                state.proutn("Super nova here?");
                if ja(state, io).await {
                    nqx = state.a.quadx;
                    nqy = state.a.quady;
                }
            }
        }

        if nqx != state.a.quadx || nqy != state.a.quady || state.a.justin != 0 {
            if state.reports() {
                skip(state, io, 1).await;
                state.proutn("Message from Starfleet Command       Stardate ");
                state.cramf(state.d.date, 0, 1);
                skip(state, io, 1).await;
                state.proutn("     Supernova in");
                state.cramlc(1, nqx, nqy);
                prout(state, io, "; caution advised.").await;
            }
        } else {
            insipient = true;
            let mut num = (state.rng.rand_f64()
                * ((state.d.galaxy[nqx as usize][nqy as usize] % 10) as f64)
                + 1.0) as i32;
            'star_find: for sx in 1..=10 {
                for sy in 1..=10 {
                    if state.quad[sx][sy] == IHSTAR {
                        num -= 1;
                        if num == 0 {
                            nsx = sx as i32;
                            nsy = sy as i32;
                            break 'star_find;
                        }
                    }
                }
            }
        }
    } else {
        insipient = true;
    }

    if insipient {
        skip(state, io, 1).await;
        prouts(state, "***RED ALERT!  RED ALERT!");
        skip(state, io, 1).await;
        state.proutn("***Incipient supernova detected at");
        state.cramlc(2, nsx, nsy);
        skip(state, io, 1).await;
        nqx = state.a.quadx;
        nqy = state.a.quady;
        if square((nsx - state.a.sectx) as f64) + square((nsy - state.a.secty) as f64) <= 2.1 {
            state.proutn("Emergency override attempts t");
            prouts(state, "***************");
            skip(state, io, 1).await;
            crate::io::stars(state, io).await;
            state.a.alldone = 1;
        }
    }

    let num = state.d.galaxy[nqx as usize][nqy as usize];
    let mut kldead = num / 100;
    state.d.remkl -= kldead;
    let mut comdead = 0;
    let mut iscdead = 0;

    if nqx == state.d.isx && nqy == state.d.isy {
        state.d.nscrem = 0;
        state.d.isx = 0;
        state.d.isy = 0;
        state.a.isatb = 0;
        state.a.iscate = 0;
        iscdead = 1;
        kldead -= 1;
        state.future[FSCMOVE] = 1e30;
        state.future[FSCDBAS] = 1e30;
    }

    if state.d.remcom != 0 {
        let maxloop = state.d.remcom as usize;
        for l in 1..=maxloop {
            if state.d.cx[l] == nqx && state.d.cy[l] == nqy {
                let remc = state.d.remcom as usize;
                state.d.cx[l] = state.d.cx[remc];
                state.d.cy[l] = state.d.cy[remc];
                state.d.cx[remc] = 0;
                state.d.cy[remc] = 0;
                state.d.remcom -= 1;
                kldead -= 1;
                comdead += 1;
                if state.d.remcom == 0 {
                    state.future[FTBEAM] = 1e30;
                }
                break;
            }
        }
    }

    let nstuff = state.d.newstuf[nqx as usize][nqy as usize];
    state.d.newstuf[nqx as usize][nqy as usize] = 0;
    let nrmdead = nstuff / 10;
    state.d.nromrem -= nrmdead;
    let npdead = nstuff - nrmdead * 10;
    if npdead != 0 {
        for l in 1..=(state.a.inplan as usize) {
            if state.d.plnets[l].x == nqx && state.d.plnets[l].y == nqy {
                state.d.plnets[l] = Planet::default();
            }
        }
    }

    if state.d.rembase != 0 {
        let maxloop = state.d.rembase as usize;
        for l in 1..=maxloop {
            if state.d.baseqx[l] == nqx && state.d.baseqy[l] == nqy {
                let remb = state.d.rembase as usize;
                state.d.baseqx[l] = state.d.baseqx[remb];
                state.d.baseqy[l] = state.d.baseqy[remb];
                state.d.baseqx[remb] = 0;
                state.d.baseqy[remb] = 0;
                state.d.rembase -= 1;
                break;
            }
        }
    }

    if insx != 0 {
        let rem = state.d.galaxy[nqx as usize][nqy as usize] % 100;
        state.d.starkl += rem % 10;
        state.d.basekl += rem / 10;
        state.d.killk += kldead;
        state.d.killc += comdead;
        state.d.nromkl += nrmdead;
        state.d.nplankl += npdead;
        state.d.nsckill += iscdead;
    }

    if (state.a.quadx == nqx && state.a.quady == nqy) || state.reports() {
        state.starch[nqx as usize][nqy as usize] = 1;
    }
    state.d.galaxy[nqx as usize][nqy as usize] = 1000;

    if state.a.alldone != 0 {
        finish(state, io, FinType::Snovaed).await;
        return;
    }

    if state.d.remkl == 0 {
        skip(state, io, 2).await;
        if insx == 0 {
            prout(state, io, "Lucky you!").await;
        }
        state.proutn("A supernova in");
        state.cramlc(1, nqx, nqy);
        prout(state, io, " has just destroyed the last Klingons.").await;
        finish(state, io, FinType::Won).await;
    }
}
