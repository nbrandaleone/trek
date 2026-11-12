//! Game initialization, quadrant generation, abandon ship, and freeze/thaw serialization.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::battle::attack;
use crate::finish::finish;
use crate::io::{huh, prout, prouts, skip, IoHandle};
use crate::reports::report;
use crate::rng::MacOsRng;
use crate::scanner::{scan, ScanToken};
use crate::state::{
    fmt_json_f64, json_escape, square, FinType, GalaxyData, GameState, Planet, ShipVars, DCLOAK,
    DSHUTTL, DSRSENS, FBATTAK, FCDBAS, FDSPROB, FSCDBAS, FSCMOVE, FSNAP, FSNOVA, FTBEAM, IHB,
    IHBLANK, IHC, IHDOCKED, IHDOT, IHE, IHF, IHK, IHP, IHQUEST, IHR, IHS, IHSTAR, IHT, NDEVICE,
    PLNETMAX, SEMERITUS, SEXPERT, SFAIR, SGOOD, SNOVICE,
};

pub async fn prelim(state: &mut GameState, io: &IoHandle) {
    skip(state, io, 2).await;
    prout(state, io, "-SUPER- STAR TREK").await;
    skip(state, io, 1).await;
    prout(state, io, "Latest update-21 Sept 78").await;
    skip(state, io, 1).await;
}

pub async fn freeze(state: &mut GameState, io: &IoHandle, boss: bool) {
    if boss {
        state.citem = String::from("emsave.trk");
    } else {
        let mut key = scan(state, io).await;
        if key == ScanToken::Eol {
            state.proutn("File name(9 characters maximum): ");
            key = scan(state, io).await;
        }
        if key != ScanToken::Alpha {
            huh(state, io).await;
            return;
        }
        state.chew();
        if !state.citem.contains('.') {
            state.citem.push_str(".trk");
        }
    }
    let filename = state.citem.clone();
    if filename.is_empty() {
        state.proutn("Can't freeze game as file ");
        state.proutn(&filename);
        skip(state, io, 1).await;
        return;
    }
    let data = serialize_freeze(state);
    state.saved_games.insert(filename.clone(), data.clone());
    state.saves_updated.push((filename, data));
}

pub async fn thaw(state: &mut GameState, io: &IoHandle) {
    state.passwd.clear();
    let mut key = scan(state, io).await;
    if key == ScanToken::Eol {
        state.proutn("File name: ");
        key = scan(state, io).await;
    }
    if key != ScanToken::Alpha {
        huh(state, io).await;
        return;
    }
    state.chew();
    if !state.citem.contains('.') {
        state.citem.push_str(".trk");
    }
    let filename = state.citem.clone();
    let Some(raw) = state.saved_games.get(&filename).cloned() else {
        state.proutn("Can't find game file ");
        state.proutn(&filename);
        skip(state, io, 1).await;
        return;
    };
    if !deserialize_freeze(state, &raw) {
        state.passwd.clear();
        state.proutn("Can't find game file ");
        state.proutn(&filename);
        skip(state, io, 1).await;
    }
}

pub async fn abandn(state: &mut GameState, io: &IoHandle) {
    state.chew();
    if state.a.condit == IHDOCKED {
        if state.a.ship != IHE {
            prout(state, io, "You cannot abandon Ye Faerie Queene.").await;
            return;
        }
    } else {
        if state.damage[DSHUTTL] == -1.0 {
            prout(state, io, "Ye Faerie Queene has no shuttle craft.").await;
            return;
        }
        if state.damage[DSHUTTL] < 0.0 {
            prout(state, io, "Shuttle craft now serving Big Mac's.").await;
            return;
        }
        if state.damage[DSHUTTL] > 0.0 {
            prout(state, io, "Shuttle craft damaged.").await;
            return;
        }
        if state.a.landed == 1 {
            prout(state, io, "You must be aboard the Enterprise.").await;
            return;
        }
        if state.a.iscraft != 1 {
            prout(state, io, "Shuttle craft not currently available.").await;
            return;
        }
        skip(state, io, 1).await;
        prouts(state, "***ABANDON SHIP!  ABANDON SHIP!");
        skip(state, io, 1).await;
        prouts(state, "***ALL HANDS ABANDON SHIP!");
        skip(state, io, 2).await;
        prout(state, io, "Captain and crew escape in shuttle craft.").await;
        prout(state, io, "Remainder of ship's complement beam down").await;
        prout(state, io, "to nearest habitable planet.").await;
        if state.d.rembase == 0 {
            finish(state, io, FinType::Abandn).await;
            return;
        }
        skip(state, io, 1).await;
        state.a.icrystl = 0;
        state.a.nprobes = 0;
        prout(state, io, "You are captured by Klingons and released to").await;
        prout(state, io, "the Federation in a prisoner-of-war exchange.").await;
        let nb = (state.rng.rand_f64() * state.d.rembase as f64 + 1.0) as usize;
        if state.a.quadx != state.d.baseqx[nb] || state.a.quady != state.d.baseqy[nb] {
            state.a.quadx = state.d.baseqx[nb];
            state.a.quady = state.d.baseqy[nb];
            state.a.sectx = 5;
            state.a.secty = 5;
            newqad(state, io, true).await;
        }
        loop {
            let sx = state.a.sectx as usize;
            let sy = state.a.secty as usize;
            state.quad[sx][sy] = IHDOT;
            let mut l = 1;
            while l <= 10 {
                state.a.sectx = (3.0 * state.rng.rand_f64() - 1.0 + state.a.basex as f64) as i32;
                state.a.secty = (3.0 * state.rng.rand_f64() - 1.0 + state.a.basey as f64) as i32;
                if (1..=10).contains(&state.a.sectx)
                    && (1..=10).contains(&state.a.secty)
                    && state.quad[state.a.sectx as usize][state.a.secty as usize] == IHDOT
                {
                    break;
                }
                l += 1;
            }
            if l < 11 {
                break;
            }
            state.a.sectx = 5;
            state.a.secty = 5;
            newqad(state, io, true).await;
        }
    }

    state.a.ship = IHF;
    let sx = state.a.sectx as usize;
    let sy = state.a.secty as usize;
    state.quad[sx][sy] = IHF;
    prout(state, io, "Starfleet puts you in command of another ship,").await;
    prout(state, io, "the Faerie Queene, which is antiquated but,").await;
    prout(state, io, "still usable.").await;
    if state.a.icrystl != 0 {
        prout(state, io, "The dilithium crystals have been moved.").await;
    }
    state.a.imine = 0;
    state.a.iscraft = 0;
    state.a.brigcapacity = 300;
    state.d.brigfree = state.a.brigcapacity;
    state.a.iscloaked = false;
    state.a.iscloaking = false;
    state.a.condit = IHDOCKED;
    for l in 1..=NDEVICE {
        state.damage[l] = 0.0;
    }
    state.damage[DSHUTTL] = -1.0;
    state.a.energy = 3000.0;
    state.a.inenrg = 3000.0;
    state.a.shield = 1250.0;
    state.a.inshld = 1250.0;
    state.a.torps = 6;
    state.a.intorps = 6;
    state.a.lsupres = 3.0;
    state.a.inlsr = 3.0;
    state.a.shldup = 0;
    state.a.warpfac = 5.0;
    state.a.wfacsq = 25.0;
    state.damage[DCLOAK] = 0.0;
}

pub async fn setup(state: &mut GameState, io: &IoHandle) {
    state.a.alldone = 0;
    state.a.gamewon = 0;
    state.debug_mode = false;
    if choose(state, io).await {
        return;
    }

    state.a.ship = IHE;
    state.a.energy = 5000.0;
    state.a.inenrg = 5000.0;
    state.a.shield = 2500.0;
    state.a.inshld = 2500.0;
    state.a.shldchg = 0;
    state.a.shldup = 0;
    state.a.inlsr = 4.0;
    state.a.lsupres = 4.0;
    let (qx, qy) = state.rng.iran8();
    state.a.quadx = qx;
    state.a.quady = qy;
    let (sx, sy) = state.rng.iran10();
    state.a.sectx = sx;
    state.a.secty = sy;
    state.a.torps = 10;
    state.a.intorps = 10;
    state.a.nprobes = (3.0 * state.rng.rand_f64() + 2.0) as i32;
    state.a.warpfac = 5.0;
    state.a.wfacsq = state.a.warpfac * state.a.warpfac;
    for i in 0..=NDEVICE {
        state.damage[i] = 0.0;
    }

    state.a.batx = 0;
    state.a.baty = 0;
    state.d.date = 100.0 * ((31.0 * state.rng.rand_f64() + 20.0) as i32 as f64);
    state.a.indate = state.d.date;
    state.d.killk = 0;
    state.d.killc = 0;
    state.a.nkinks = 0;
    state.a.nhelp = 0;
    state.a.resting = 0;
    state.a.casual = 0;
    state.d.nromkl = 0;
    state.a.isatb = 0;
    state.a.iscate = 0;
    state.a.imine = 0;
    state.a.icrystl = 0;
    state.d.nsckill = 0;
    state.d.nplankl = 0;
    state.d.starkl = 0;
    state.a.iscraft = 1;
    state.a.landed = -1;
    state.a.alive = 1;
    state.a.docfac = 0.25;
    for i in 1..=8 {
        for j in 1..=8 {
            state.d.newstuf[i][j] = 0;
            state.starch[i][j] = 0;
        }
    }

    state.future[FSNOVA] = state.d.date + state.rng.expran(0.5 * state.a.intime);
    state.future[FTBEAM] = state.d.date + state.rng.expran(1.5 * (state.a.intime / state.d.remcom as f64));
    state.future[FSNAP] = state.d.date + 1.0 + state.rng.rand_f64();
    state.future[FBATTAK] = state.d.date + state.rng.expran(0.3 * state.a.intime);
    state.future[FCDBAS] = 1e30;
    state.future[FSCMOVE] = if state.d.nscrem != 0 {
        state.d.date + 0.2777
    } else {
        1e30
    };
    state.future[FSCDBAS] = 1e30;
    state.future[FDSPROB] = 1e30;
    state.a.stdamtim = 1e30;

    state.a.instar = 0;
    for i in 1..=8 {
        for j in 1..=8 {
            let k = (state.rng.rand_f64() * 9.0 + 1.0) as i32;
            state.a.instar += k;
            state.d.galaxy[i][j] = k;
        }
    }

    for i in 1..=(state.a.inbase as usize) {
        let mut ix;
        let mut iy;
        loop {
            loop {
                let (rx, ry) = state.rng.iran8();
                ix = rx;
                iy = ry;
                if state.d.galaxy[ix as usize][iy as usize] < 10 {
                    break;
                }
            }
            let mut contflag = false;
            for j in (1..i).rev() {
                let distq = square((ix - state.d.baseqx[j]) as f64)
                    + square((iy - state.d.baseqy[j]) as f64);
                if distq < 6.0 * (6 - state.a.inbase) as f64 && state.rng.rand_f64() < 0.75 {
                    contflag = true;
                    break;
                }
            }
            if !contflag {
                break;
            }
        }
        state.d.baseqx[i] = ix;
        state.d.baseqy[i] = iy;
        state.starch[ix as usize][iy as usize] = -1;
        state.d.galaxy[ix as usize][iy as usize] += 10;
    }

    let mut krem = state.a.inkling - state.a.incom - state.d.nscrem;
    let mut klumper = (0.25 * state.a.skill as f64 * (9.0 - state.a.length as f64) + 1.0) as i32;
    if klumper > 9 {
        klumper = 9;
    }
    loop {
        let r = state.rng.rand_f64();
        let mut klump = ((1.0 - r * r) * klumper as f64) as i32;
        if klump > krem {
            klump = krem;
        }
        krem -= klump;
        klump *= 100;
        let (mut ix, mut iy);
        loop {
            let (rx, ry) = state.rng.iran8();
            ix = rx as usize;
            iy = ry as usize;
            if state.d.galaxy[ix][iy] + klump < 1000 {
                break;
            }
        }
        state.d.galaxy[ix][iy] += klump;
        if krem <= 0 {
            break;
        }
    }

    let mut dbg_klumper = 1usize;
    for i in 1..=(state.a.incom as usize) {
        let mut ix;
        let mut iy;
        loop {
            loop {
                if state.debug_mode && dbg_klumper <= state.a.inbase as usize {
                    ix = state.d.baseqx[dbg_klumper];
                    iy = state.d.baseqy[dbg_klumper];
                    dbg_klumper += 1;
                } else {
                    let (rx, ry) = state.rng.iran8();
                    ix = rx;
                    iy = ry;
                }
                let g = state.d.galaxy[ix as usize][iy as usize];
                if !((g < 99 && state.rng.rand_f64() < 0.75) || g > 899) {
                    break;
                }
            }
            let mut dup = false;
            for j in 1..i {
                if state.d.cx[j] == ix && state.d.cy[j] == iy {
                    dup = true;
                    break;
                }
            }
            if !dup {
                break;
            }
        }
        state.d.galaxy[ix as usize][iy as usize] += 100;
        state.d.cx[i] = ix;
        state.d.cy[i] = iy;
    }

    for i in 1..=(state.a.inplan as usize) {
        let (mut ix, mut iy);
        loop {
            let (rx, ry) = state.rng.iran8();
            ix = rx;
            iy = ry;
            if state.d.newstuf[ix as usize][iy as usize] <= 0 {
                break;
            }
        }
        state.d.newstuf[ix as usize][iy as usize] = 1;
        state.d.plnets[i].x = ix;
        state.d.plnets[i].y = iy;
        state.d.plnets[i].pclass = (state.rng.rand_f64() * 3.0 + 1.0) as i32;
        state.d.plnets[i].crystals = (1.5 * state.rng.rand_f64()) as i32;
        state.d.plnets[i].known = 0;
    }

    for _ in 1..=state.d.nromrem {
        let (ix, iy) = state.rng.iran8();
        state.d.newstuf[ix as usize][iy as usize] += 10;
    }

    if state.d.nscrem > 0 {
        let (mut ix, mut iy);
        loop {
            let (rx, ry) = state.rng.iran8();
            ix = rx;
            iy = ry;
            if state.d.galaxy[ix as usize][iy as usize] < 900 {
                break;
            }
        }
        state.d.isx = ix;
        state.d.isy = iy;
        state.d.galaxy[ix as usize][iy as usize] += 100;
    }

    if state.rng.rand_f64() < 0.1 && state.a.thingx != -1 {
        let (tx, ty) = state.rng.iran8();
        state.a.thingx = tx;
        state.a.thingy = ty;
    } else {
        state.a.thingx = 0;
        state.a.thingy = 0;
    }

    skip(state, io, 3).await;
    state.d.snap = 0;

    if state.a.skill == SNOVICE {
        prout(
            state,
            io,
            &format!(
                "It is stardate {}. The Federation is being attacked by",
                state.d.date as i32
            ),
        )
        .await;
        prout(
            state,
            io,
            "a deadly Klingon invasion force. As captain of the United",
        )
        .await;
        prout(
            state,
            io,
            "Starship U.S.S. Enterprise, it is your mission to seek out",
        )
        .await;
        prout(
            state,
            io,
            &format!(
                "and destroy this invasion force of {} battle cruisers.",
                state.a.inkling
            ),
        )
        .await;
        prout(
            state,
            io,
            &format!(
                "You have an initial allotment of {} stardates to complete",
                state.a.intime as i32
            ),
        )
        .await;
        prout(
            state,
            io,
            "your mission.  As you proceed you may be given more time.",
        )
        .await;
        skip(state, io, 1).await;
        prout(
            state,
            io,
            &format!("You will have {} supporting starbases.", state.a.inbase),
        )
        .await;
        state.proutn("Starbase locations-  ");
    } else {
        prout(state, io, &format!("Stardate {}.", state.d.date as i32)).await;
        skip(state, io, 1).await;
        prout(state, io, &format!("{} Klingons,", state.a.inkling)).await;
        prout(state, io, "an unknown number of Romulans").await;
        if state.d.nscrem != 0 {
            prout(state, io, "and one (GULP) Super-Commander.").await;
        }
        prout(state, io, &format!("{} stardates", state.a.intime as i32)).await;
        state.proutn(&format!("{} starbases in  ", state.a.inbase));
    }

    for i in 1..=(state.a.inbase as usize) {
        state.cramlc(0, state.d.baseqx[i], state.d.baseqy[i]);
        if i < state.a.inbase as usize {
            state.proutn("  ");
        }
    }
    skip(state, io, 2).await;
    state.proutn("The Enterprise is currently in");
    state.cramlc(1, state.a.quadx, state.a.quady);
    state.proutn(" ");
    state.cramlc(2, state.a.sectx, state.a.secty);
    skip(state, io, 2).await;
    prout(state, io, "Good Luck!").await;
    if state.d.nscrem != 0 {
        state.proutn("  YOU'LL NEED IT.");
    }
    skip(state, io, 1).await;
    newqad(state, io, false).await;
    if state.a.nenhere != 0 {
        state.a.shldup = 1;
    }
    if state.a.neutz != 0 {
        attack(state, io, 0).await;
    }
}

pub async fn choose(state: &mut GameState, io: &IoHandle) -> bool {
    state.a.tourn = 0;
    state.a.thawed = 0;
    state.a.skill = 0;
    state.a.length = 0;
    loop {
        if state.fromcommandline {
            state.fromcommandline = false;
        } else {
            state.proutn("Would you like a regular, tournament, or frozen game?");
        }
        scan(state, io).await;
        if state.citem.is_empty() {
            continue;
        }
        if state.isit("tournament") {
            while scan(state, io).await == ScanToken::Eol {
                state.proutn("Type in tournament number-");
            }
            if state.aaitem == 0.0 {
                state.chew();
                continue;
            }
            state.a.tourn = state.aaitem as i32;
            state.a.thingx = -1;
            state.rng.srand(state.a.tourn as u32);
            break;
        }
        if state.isit("frozen") {
            thaw(state, io).await;
            state.chew();
            if state.passwd.is_empty() {
                continue;
            }
            state.randomize();
            let _ = state.rng.rand_f64();
            let _ = state.rng.rand_f64();
            let _ = state.rng.rand_f64();
            let _ = state.rng.rand_f64();
            if state.a.alldone == 0 {
                state.a.thawed = 1;
            }
            report(state, io, true).await;
            return true;
        }
        if state.isit("regular") {
            skip(state, io, 2).await;
            state.randomize();
            let _ = state.rng.rand_f64();
            let _ = state.rng.rand_f64();
            let _ = state.rng.rand_f64();
            let _ = state.rng.rand_f64();
            break;
        }
        state.proutn("What is \"");
        let item = state.citem.clone();
        state.proutn(&item);
        prout(state, io, "\"?").await;
        state.chew();
    }

    while state.a.length == 0 || state.a.skill == 0 {
        if scan(state, io).await == ScanToken::Alpha {
            if state.isit("short") {
                state.a.length = 1;
            } else if state.isit("medium") {
                state.a.length = 2;
            } else if state.isit("long") {
                state.a.length = 4;
            } else if state.isit("novice") {
                state.a.skill = SNOVICE;
            } else if state.isit("fair") {
                state.a.skill = SFAIR;
            } else if state.isit("good") {
                state.a.skill = SGOOD;
            } else if state.isit("expert") {
                state.a.skill = SEXPERT;
            } else if state.isit("emeritus") {
                state.a.skill = SEMERITUS;
            } else {
                state.proutn("What is \"");
                let item = state.citem.clone();
                state.proutn(&item);
                prout(state, io, "\"?").await;
            }
        } else {
            state.chew();
            if state.a.length == 0 {
                state.proutn("Would you like a Short, Medium, or Long game? ");
            } else if state.a.skill == 0 {
                state.proutn("Are you a Novice, Fair, Good, Expert, or Emeritus player?");
            }
        }
    }

    loop {
        scan(state, io).await;
        state.passwd = state.citem.clone();
        state.chew();
        if !state.passwd.is_empty() {
            break;
        }
        state.proutn("Please type in a secret password (9 characters maximum)-");
    }
    if state.allow_debug && state.passwd == "debug" {
        state.debug_mode = true;
    }

    state.a.damfac = 0.5 * state.a.skill as f64;
    state.d.rembase = (3.0 * state.rng.rand_f64() + 2.0) as i32;
    state.a.inbase = state.d.rembase;
    state.a.inplan = ((PLNETMAX / 2) as f64 + ((PLNETMAX / 2 + 1) as f64) * state.rng.rand_f64()) as i32;
    state.d.nromrem = ((2.0 + state.rng.rand_f64()) * state.a.skill as f64) as i32;
    state.d.nscrem = if state.a.skill > SFAIR { 1 } else { 0 };
    state.d.remtime = 7.0 * state.a.length as f64;
    state.a.intime = state.d.remtime;
    state.d.remkl = (2.0
        * state.a.intime
        * ((state.a.skill as f64 + 1.0 - 2.0 * state.rng.rand_f64()) * state.a.skill as f64 * 0.1
            + 0.15)) as i32;
    state.a.inkling = state.d.remkl;
    state.a.incom =
        (state.a.skill as f64 + 0.0625 * state.a.inkling as f64 * state.rng.rand_f64()) as i32;
    state.d.remcom = 10.min(state.a.incom);
    state.a.incom = state.d.remcom;
    state.d.remres = ((state.a.inkling + 4 * state.a.incom) as f64) * state.a.intime;
    state.a.inresor = state.d.remres;
    if state.a.inkling > 50 {
        state.d.rembase += 1;
        state.a.inbase = state.d.rembase;
    }
    state.a.brigcapacity = 400;
    state.d.brigfree = state.a.brigcapacity;
    state.d.kcaptured = 0;
    state.a.ncviol = 0;
    state.a.iscloaked = false;
    state.a.iscloaking = false;
    false
}

pub async fn newqad(state: &mut GameState, io: &IoHandle, shutup: bool) {
    let qx = state.a.quadx as usize;
    let qy = state.a.quady as usize;
    let mut quadnum = state.d.galaxy[qx][qy];
    let newnum = state.d.newstuf[qx][qy];

    state.a.iattak = 1;
    state.a.justin = 1;
    state.a.basex = 0;
    state.a.basey = 0;
    state.a.klhere = 0;
    state.a.comhere = 0;
    state.a.plnetx = 0;
    state.a.plnety = 0;
    state.a.ishere = 0;
    state.a.irhere = 0;
    state.a.iplnet = 0;
    state.a.nenhere = 0;
    state.a.neutz = 0;
    state.a.inorbit = 0;
    state.a.landed = -1;
    state.a.ientesc = 0;
    state.a.ithere = 0;
    state.a.iseenit = 0;
    state.a.isviolreported = false;

    if state.a.iscate != 0 {
        state.a.iscate = 0;
        state.a.ientesc = 1;
    }

    for i in 1..=10 {
        for j in 1..=10 {
            state.quad[i][j] = IHDOT;
        }
    }

    if quadnum > 999 {
        return;
    }

    state.a.klhere = quadnum / 100;
    state.a.irhere = newnum / 10;
    let nplan = newnum % 10;
    state.a.nenhere = state.a.klhere + state.a.irhere;

    state.quad[state.a.sectx as usize][state.a.secty as usize] = state.a.ship;

    let need_tholian = (state.a.skill < SGOOD && state.rng.rand_f64() <= 0.02)
        || (state.a.skill == SGOOD && state.rng.rand_f64() <= 0.05)
        || (state.a.skill > SGOOD && state.rng.rand_f64() <= 0.08)
        || (state.allow_debug && state.passwd == "tholianx");

    if need_tholian {
        loop {
            state.a.ithx = if state.rng.rand_f64() > 0.5 { 10 } else { 1 };
            state.a.ithy = if state.rng.rand_f64() > 0.5 { 10 } else { 1 };
            if state.quad[state.a.ithx as usize][state.a.ithy as usize] == IHDOT {
                break;
            }
        }
        state.quad[state.a.ithx as usize][state.a.ithy as usize] = IHT;
        state.a.ithere = 1;
        if state.quad[1][1] == IHDOT {
            state.quad[1][1] = 'X';
        }
        if state.quad[1][10] == IHDOT {
            state.quad[1][10] = 'X';
        }
        if state.quad[10][1] == IHDOT {
            state.quad[10][1] = 'X';
        }
        if state.quad[10][10] == IHDOT {
            state.quad[10][10] = 'X';
        }
    }

    if quadnum >= 100 {
        quadnum -= 100 * state.a.klhere;
        let mut last_ix = 0;
        let mut last_iy = 0;
        for i in 1..=(state.a.klhere as usize) {
            let (ix, iy) = state.dropin(IHK);
            last_ix = ix;
            last_iy = iy;
            state.kx[i] = ix;
            state.ky[i] = iy;
            let d = (square((state.a.sectx - ix) as f64) + square((state.a.secty - iy) as f64))
                .sqrt();
            state.kdist[i] = d;
            state.kavgd[i] = d;
            state.kpower[i] =
                state.rng.rand_f64() * 150.0 + 300.0 + 25.0 * state.a.skill as f64;
        }
        let has_com = (1..=(state.d.remcom as usize))
            .any(|i| state.d.cx[i] == state.a.quadx && state.d.cy[i] == state.a.quady);
        if has_com && state.a.klhere > 0 {
            state.quad[last_ix as usize][last_iy as usize] = IHC;
            state.kpower[state.a.klhere as usize] =
                950.0 + 400.0 * state.rng.rand_f64() + 50.0 * state.a.skill as f64;
            state.a.comhere = 1;
        }
        if state.a.quadx == state.d.isx && state.a.quady == state.d.isy && state.a.klhere > 0 {
            state.quad[state.kx[1] as usize][state.ky[1] as usize] = IHS;
            state.kpower[1] = 1175.0 + 400.0 * state.rng.rand_f64() + 125.0 * state.a.skill as f64;
            state.a.iscate = 1;
            state.a.ishere = 1;
        }
    }

    for i in (state.a.klhere as usize + 1)..=(state.a.nenhere as usize) {
        let (ix, iy) = state.dropin(IHR);
        state.kx[i] = ix;
        state.ky[i] = iy;
        let d = (square((state.a.sectx - ix) as f64) + square((state.a.secty - iy) as f64)).sqrt();
        state.kdist[i] = d;
        state.kavgd[i] = d;
        state.kpower[i] = state.rng.rand_f64() * 400.0 + 450.0 + 50.0 * state.a.skill as f64;
    }
    state.sortkl();

    if quadnum >= 10 {
        quadnum -= 10;
        let (bx, by) = state.dropin(IHB);
        state.a.basex = bx;
        state.a.basey = by;
    }

    if nplan != 0 {
        if let Some(i) = (1..=(state.a.inplan as usize)).find(|&i| {
            state.d.plnets[i].x == state.a.quadx && state.d.plnets[i].y == state.a.quady
        }) {
            state.a.iplnet = i as i32;
            let (px, py) = state.dropin(IHP);
            state.a.plnetx = px;
            state.a.plnety = py;
        }
    }

    state.newcnd();
    for _ in 1..=quadnum {
        let _ = state.dropin(IHSTAR);
    }

    if state.a.irhere > 0 && state.a.klhere == 0 && state.a.basex == 0 {
        state.a.neutz = 1;
        if state.reports() {
            skip(state, io, 1).await;
            prout(state, io, "LT. UHURA- \"Captain, an urgent message.").await;
            prout(state, io, "  I'll put it on audio.\"  CLICK").await;
            skip(state, io, 1).await;
            prout(
                state,
                io,
                "INTRUDER! YOU HAVE VIOLATED THE ROMULAN NEUTRAL ZONE.",
            )
            .await;
            prout(
                state,
                io,
                "LEAVE AT ONCE, OR YOU WILL BE DESTROYED!",
            )
            .await;
        }
    }

    if !shutup && state.a.thingx == state.a.quadx && state.a.thingy == state.a.quady {
        let _ = state.dropin(IHQUEST);
        state.a.thingx = 0;
        state.a.thingy = 0;
        if state.damage[DSRSENS] == 0.0 {
            skip(state, io, 1).await;
            prout(state, io, "MR. SPOCK- \"Captain, this is most unusual.").await;
            prout(state, io, "    Please examine your short-range scan.\"").await;
        }
    }

    for _ in 1..=3 {
        if state.rng.rand_f64() > 0.5 {
            let _ = state.dropin(IHBLANK);
        }
    }

    if state.a.ithere != 0 {
        if state.quad[1][1] == 'X' {
            state.quad[1][1] = IHDOT;
        }
        if state.quad[1][10] == 'X' {
            state.quad[1][10] = IHDOT;
        }
        if state.quad[10][1] == 'X' {
            state.quad[10][1] = IHDOT;
        }
        if state.quad[10][10] == 'X' {
            state.quad[10][10] = IHDOT;
        }
    }
}

// ---------- Pure-std JSON serialization & deserialization for freeze/thaw ----------

fn push_i32_arr(out: &mut String, slice: &[i32]) {
    out.push('[');
    for (idx, val) in slice.iter().enumerate() {
        if idx > 0 {
            out.push(',');
        }
        let _ = write!(out, "{}", val);
    }
    out.push(']');
}

fn push_f64_arr(out: &mut String, slice: &[f64]) {
    out.push('[');
    for (idx, val) in slice.iter().enumerate() {
        if idx > 0 {
            out.push(',');
        }
        out.push_str(&fmt_json_f64(*val));
    }
    out.push(']');
}

fn push_i32_grid_9(out: &mut String, grid: &[[i32; 9]; 9]) {
    out.push('[');
    for (i, row) in grid.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        push_i32_arr(out, row);
    }
    out.push(']');
}

fn push_galaxy_data(out: &mut String, g: &GalaxyData) {
    out.push('{');
    let _ = write!(
        out,
        "\"snap\":{},\"remkl\":{},\"remcom\":{},\"rembase\":{},\"starkl\":{},\"basekl\":{},\"killk\":{},\"killc\":{},",
        g.snap, g.remkl, g.remcom, g.rembase, g.starkl, g.basekl, g.killk, g.killc
    );
    out.push_str("\"galaxy\":");
    push_i32_grid_9(out, &g.galaxy);
    out.push_str(",\"cx\":");
    push_i32_arr(out, &g.cx);
    out.push_str(",\"cy\":");
    push_i32_arr(out, &g.cy);
    out.push_str(",\"baseqx\":");
    push_i32_arr(out, &g.baseqx);
    out.push_str(",\"baseqy\":");
    push_i32_arr(out, &g.baseqy);
    out.push_str(",\"newstuf\":");
    push_i32_grid_9(out, &g.newstuf);
    let _ = write!(
        out,
        ",\"isx\":{},\"isy\":{},\"nscrem\":{},\"nromkl\":{},\"nromrem\":{},\"nsckill\":{},\"nplankl\":{},\"plnets\":[",
        g.isx, g.isy, g.nscrem, g.nromkl, g.nromrem, g.nsckill, g.nplankl
    );
    for (i, p) in g.plnets.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        let _ = write!(
            out,
            "{{\"x\":{},\"y\":{},\"pclass\":{},\"crystals\":{},\"known\":{}}}",
            p.x, p.y, p.pclass, p.crystals, p.known
        );
    }
    let _ = write!(
        out,
        "],\"kcaptured\":{},\"brigfree\":{},\"date\":{},\"remres\":{},\"remtime\":{}}}",
        g.kcaptured,
        g.brigfree,
        fmt_json_f64(g.date),
        fmt_json_f64(g.remres),
        fmt_json_f64(g.remtime)
    );
}

fn push_char_str(out: &mut String, ch: char) {
    let mut buf = [0u8; 4];
    let s = ch.encode_utf8(&mut buf);
    json_escape(s, out);
}

fn push_ship_vars(out: &mut String, a: &ShipVars) {
    out.push('{');
    let _ = write!(
        out,
        "\"inkling\":{},\"inbase\":{},\"incom\":{},\"instar\":{},\"intorps\":{},\"condit\":",
        a.inkling, a.inbase, a.incom, a.instar, a.intorps
    );
    push_char_str(out, a.condit);
    let _ = write!(out, ",\"torps\":{},\"ship\":", a.torps);
    push_char_str(out, a.ship);
    let _ = write!(
        out,
        ",\"quadx\":{},\"quady\":{},\"sectx\":{},\"secty\":{},\"length\":{},\"skill\":{},\"basex\":{},\"basey\":{},\"klhere\":{},\"comhere\":{},\"casual\":{},\"nhelp\":{},\"nkinks\":{},\"ididit\":{},\"gamewon\":{},\"alive\":{},\"justin\":{},\"alldone\":{},\"shldchg\":{},\"thingx\":{},\"thingy\":{},\"plnetx\":{},\"plnety\":{},\"inorbit\":{},\"landed\":{},\"iplnet\":{},\"imine\":{},\"inplan\":{},\"nenhere\":{},\"ishere\":{},\"neutz\":{},\"irhere\":{},\"icraft\":{},\"ientesc\":{},\"iscraft\":{},\"isatb\":{},\"iscate\":{},\"iscloaked\":{},\"iscloaking\":{},\"ncviol\":{},\"isviolreported\":{},\"brigcapacity\":{},\"iattak\":{},\"icrystl\":{},\"tourn\":{},\"thawed\":{},\"batx\":{},\"baty\":{},\"ithere\":{},\"ithx\":{},\"ithy\":{},\"iseenit\":{},\"probecx\":{},\"probecy\":{},\"proben\":{},\"isarmed\":{},\"nprobes\":{},\"inresor\":{},\"intime\":{},\"inenrg\":{},\"inshld\":{},\"inlsr\":{},\"indate\":{},\"energy\":{},\"shield\":{},\"shldup\":{},\"warpfac\":{},\"wfacsq\":{},\"lsupres\":{},\"dist\":{},\"direc\":{},\"time\":{},\"docfac\":{},\"resting\":{},\"damfac\":{},\"stdamtim\":{},\"cryprob\":{},\"probex\":{},\"probey\":{},\"probeinx\":{},\"probeiny\":{}}}",
        a.quadx,
        a.quady,
        a.sectx,
        a.secty,
        a.length,
        a.skill,
        a.basex,
        a.basey,
        a.klhere,
        a.comhere,
        a.casual,
        a.nhelp,
        a.nkinks,
        a.ididit,
        a.gamewon,
        a.alive,
        a.justin,
        a.alldone,
        a.shldchg,
        a.thingx,
        a.thingy,
        a.plnetx,
        a.plnety,
        a.inorbit,
        a.landed,
        a.iplnet,
        a.imine,
        a.inplan,
        a.nenhere,
        a.ishere,
        a.neutz,
        a.irhere,
        a.icraft,
        a.ientesc,
        a.iscraft,
        a.isatb,
        a.iscate,
        a.iscloaked,
        a.iscloaking,
        a.ncviol,
        a.isviolreported,
        a.brigcapacity,
        a.iattak,
        a.icrystl,
        a.tourn,
        a.thawed,
        a.batx,
        a.baty,
        a.ithere,
        a.ithx,
        a.ithy,
        a.iseenit,
        a.probecx,
        a.probecy,
        a.proben,
        a.isarmed,
        a.nprobes,
        fmt_json_f64(a.inresor),
        fmt_json_f64(a.intime),
        fmt_json_f64(a.inenrg),
        fmt_json_f64(a.inshld),
        fmt_json_f64(a.inlsr),
        fmt_json_f64(a.indate),
        fmt_json_f64(a.energy),
        fmt_json_f64(a.shield),
        a.shldup,
        fmt_json_f64(a.warpfac),
        fmt_json_f64(a.wfacsq),
        fmt_json_f64(a.lsupres),
        fmt_json_f64(a.dist),
        fmt_json_f64(a.direc),
        fmt_json_f64(a.time),
        fmt_json_f64(a.docfac),
        a.resting,
        fmt_json_f64(a.damfac),
        fmt_json_f64(a.stdamtim),
        fmt_json_f64(a.cryprob),
        fmt_json_f64(a.probex),
        fmt_json_f64(a.probey),
        fmt_json_f64(a.probeinx),
        fmt_json_f64(a.probeiny)
    );
}

pub fn serialize_freeze(state: &GameState) -> String {
    let mut out = String::with_capacity(4096);
    out.push_str("{\"d\":");
    push_galaxy_data(&mut out, &state.d);
    out.push_str(",\"snapsht\":");
    push_galaxy_data(&mut out, &state.snapsht);
    out.push_str(",\"quad\":[");
    for (i, row) in state.quad.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push('[');
        for (j, ch) in row.iter().enumerate() {
            if j > 0 {
                out.push(',');
            }
            push_char_str(&mut out, *ch);
        }
        out.push(']');
    }
    out.push_str("],\"kx\":");
    push_i32_arr(&mut out, &state.kx);
    out.push_str(",\"ky\":");
    push_i32_arr(&mut out, &state.ky);
    out.push_str(",\"starch\":");
    push_i32_grid_9(&mut out, &state.starch);
    out.push_str(",\"kpower\":");
    push_f64_arr(&mut out, &state.kpower);
    out.push_str(",\"kdist\":");
    push_f64_arr(&mut out, &state.kdist);
    out.push_str(",\"kavgd\":");
    push_f64_arr(&mut out, &state.kavgd);
    out.push_str(",\"damage\":");
    push_f64_arr(&mut out, &state.damage);
    out.push_str(",\"future\":");
    push_f64_arr(&mut out, &state.future);
    out.push_str(",\"a\":");
    push_ship_vars(&mut out, &state.a);
    out.push_str(",\"passwd\":");
    json_escape(&state.passwd, &mut out);
    let _ = write!(
        out,
        ",\"rng_state\":{},\"iscore\":{},\"iskill\":{},\"perdate\":{},\"height\":{}}}",
        state.rng.state(),
        state.iscore,
        state.iskill,
        fmt_json_f64(state.perdate),
        state.height
    );
    out
}

#[derive(Debug)]
enum JsonVal {
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<JsonVal>),
    Obj(BTreeMap<String, JsonVal>),
    Null,
}

struct JsonParser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> JsonParser<'a> {
    fn new(s: &'a str) -> Self {
        Self {
            bytes: s.as_bytes(),
            pos: 0,
        }
    }

    fn skip_ws(&mut self) {
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.skip_ws();
        self.bytes.get(self.pos).copied()
    }

    fn parse_value(&mut self) -> Option<JsonVal> {
        match self.peek()? {
            b'{' => self.parse_obj(),
            b'[' => self.parse_arr(),
            b'"' => self.parse_str().map(JsonVal::Str),
            b't' => {
                if self.bytes[self.pos..].starts_with(b"true") {
                    self.pos += 4;
                    Some(JsonVal::Bool(true))
                } else {
                    None
                }
            }
            b'f' => {
                if self.bytes[self.pos..].starts_with(b"false") {
                    self.pos += 5;
                    Some(JsonVal::Bool(false))
                } else {
                    None
                }
            }
            b'n' => {
                if self.bytes[self.pos..].starts_with(b"null") {
                    self.pos += 4;
                    Some(JsonVal::Null)
                } else {
                    None
                }
            }
            b'-' | b'0'..=b'9' => self.parse_num().map(JsonVal::Num),
            _ => None,
        }
    }

    fn parse_str(&mut self) -> Option<String> {
        self.skip_ws();
        if *self.bytes.get(self.pos)? != b'"' {
            return None;
        }
        self.pos += 1;
        let mut out = String::new();
        while self.pos < self.bytes.len() {
            let b = self.bytes[self.pos];
            if b == b'"' {
                self.pos += 1;
                return Some(out);
            }
            if b == b'\\' {
                self.pos += 1;
                let esc = *self.bytes.get(self.pos)?;
                self.pos += 1;
                match esc {
                    b'"' => out.push('"'),
                    b'\\' => out.push('\\'),
                    b'/' => out.push('/'),
                    b'b' => out.push('\u{08}'),
                    b'f' => out.push('\u{0C}'),
                    b'n' => out.push('\n'),
                    b'r' => out.push('\r'),
                    b't' => out.push('\t'),
                    b'u' => {
                        let hex = std::str::from_utf8(self.bytes.get(self.pos..self.pos + 4)?).ok()?;
                        let code = u32::from_str_radix(hex, 16).ok()?;
                        out.push(char::from_u32(code)?);
                        self.pos += 4;
                    }
                    _ => return None,
                }
            } else {
                let rest = std::str::from_utf8(&self.bytes[self.pos..]).ok()?;
                let ch = rest.chars().next()?;
                out.push(ch);
                self.pos += ch.len_utf8();
            }
        }
        None
    }

    fn parse_num(&mut self) -> Option<f64> {
        self.skip_ws();
        let start = self.pos;
        while self.pos < self.bytes.len() {
            let b = self.bytes[self.pos];
            if matches!(b, b'-' | b'+' | b'0'..=b'9' | b'.' | b'e' | b'E') {
                self.pos += 1;
            } else {
                break;
            }
        }
        let s = std::str::from_utf8(&self.bytes[start..self.pos]).ok()?;
        s.parse::<f64>().ok()
    }

    fn parse_arr(&mut self) -> Option<JsonVal> {
        self.skip_ws();
        if *self.bytes.get(self.pos)? != b'[' {
            return None;
        }
        self.pos += 1;
        let mut arr = Vec::new();
        if self.peek()? == b']' {
            self.pos += 1;
            return Some(JsonVal::Arr(arr));
        }
        loop {
            arr.push(self.parse_value()?);
            match self.peek()? {
                b',' => self.pos += 1,
                b']' => {
                    self.pos += 1;
                    return Some(JsonVal::Arr(arr));
                }
                _ => return None,
            }
        }
    }

    fn parse_obj(&mut self) -> Option<JsonVal> {
        self.skip_ws();
        if *self.bytes.get(self.pos)? != b'{' {
            return None;
        }
        self.pos += 1;
        let mut map = BTreeMap::new();
        if self.peek()? == b'}' {
            self.pos += 1;
            return Some(JsonVal::Obj(map));
        }
        loop {
            let key = self.parse_str()?;
            if self.peek()? != b':' {
                return None;
            }
            self.pos += 1;
            let val = self.parse_value()?;
            map.insert(key, val);
            match self.peek()? {
                b',' => self.pos += 1,
                b'}' => {
                    self.pos += 1;
                    return Some(JsonVal::Obj(map));
                }
                _ => return None,
            }
        }
    }
}

fn get_i32(map: &BTreeMap<String, JsonVal>, key: &str) -> Option<i32> {
    match map.get(key)? {
        JsonVal::Num(v) => Some(*v as i32),
        _ => None,
    }
}

fn get_f64(map: &BTreeMap<String, JsonVal>, key: &str) -> Option<f64> {
    match map.get(key)? {
        JsonVal::Num(v) => Some(*v),
        _ => None,
    }
}

fn get_bool(map: &BTreeMap<String, JsonVal>, key: &str) -> Option<bool> {
    match map.get(key)? {
        JsonVal::Bool(b) => Some(*b),
        _ => None,
    }
}

fn get_char(map: &BTreeMap<String, JsonVal>, key: &str) -> Option<char> {
    match map.get(key)? {
        JsonVal::Str(s) => s.chars().next(),
        _ => None,
    }
}

fn get_str(map: &BTreeMap<String, JsonVal>, key: &str) -> Option<String> {
    match map.get(key)? {
        JsonVal::Str(s) => Some(s.clone()),
        _ => None,
    }
}

fn fill_i32_arr<const N: usize>(val: &JsonVal, dst: &mut [i32; N]) -> Option<()> {
    let JsonVal::Arr(arr) = val else {
        return None;
    };
    if arr.len() != N {
        return None;
    }
    for (idx, item) in arr.iter().enumerate() {
        let JsonVal::Num(v) = item else {
            return None;
        };
        dst[idx] = *v as i32;
    }
    Some(())
}

fn fill_f64_arr<const N: usize>(val: &JsonVal, dst: &mut [f64; N]) -> Option<()> {
    let JsonVal::Arr(arr) = val else {
        return None;
    };
    if arr.len() != N {
        return None;
    }
    for (idx, item) in arr.iter().enumerate() {
        let JsonVal::Num(v) = item else {
            return None;
        };
        dst[idx] = *v;
    }
    Some(())
}

fn fill_i32_grid_9(val: &JsonVal, dst: &mut [[i32; 9]; 9]) -> Option<()> {
    let JsonVal::Arr(rows) = val else {
        return None;
    };
    if rows.len() != 9 {
        return None;
    }
    for (i, row) in rows.iter().enumerate() {
        fill_i32_arr(row, &mut dst[i])?;
    }
    Some(())
}

fn parse_galaxy_data(val: &JsonVal) -> Option<GalaxyData> {
    let JsonVal::Obj(map) = val else {
        return None;
    };
    let mut g = GalaxyData {
        snap: get_i32(map, "snap")?,
        remkl: get_i32(map, "remkl")?,
        remcom: get_i32(map, "remcom")?,
        rembase: get_i32(map, "rembase")?,
        starkl: get_i32(map, "starkl")?,
        basekl: get_i32(map, "basekl")?,
        killk: get_i32(map, "killk")?,
        killc: get_i32(map, "killc")?,
        isx: get_i32(map, "isx")?,
        isy: get_i32(map, "isy")?,
        nscrem: get_i32(map, "nscrem")?,
        nromkl: get_i32(map, "nromkl")?,
        nromrem: get_i32(map, "nromrem")?,
        nsckill: get_i32(map, "nsckill")?,
        nplankl: get_i32(map, "nplankl")?,
        kcaptured: get_i32(map, "kcaptured")?,
        brigfree: get_i32(map, "brigfree")?,
        date: get_f64(map, "date")?,
        remres: get_f64(map, "remres")?,
        remtime: get_f64(map, "remtime")?,
        ..GalaxyData::default()
    };
    fill_i32_grid_9(map.get("galaxy")?, &mut g.galaxy)?;
    fill_i32_arr(map.get("cx")?, &mut g.cx)?;
    fill_i32_arr(map.get("cy")?, &mut g.cy)?;
    fill_i32_arr(map.get("baseqx")?, &mut g.baseqx)?;
    fill_i32_arr(map.get("baseqy")?, &mut g.baseqy)?;
    fill_i32_grid_9(map.get("newstuf")?, &mut g.newstuf)?;
    let JsonVal::Arr(pl) = map.get("plnets")? else {
        return None;
    };
    if pl.len() != 11 {
        return None;
    }
    for (idx, item) in pl.iter().enumerate() {
        let JsonVal::Obj(pm) = item else {
            return None;
        };
        g.plnets[idx] = Planet {
            x: get_i32(pm, "x")?,
            y: get_i32(pm, "y")?,
            pclass: get_i32(pm, "pclass")?,
            crystals: get_i32(pm, "crystals")?,
            known: get_i32(pm, "known")?,
        };
    }
    Some(g)
}

fn parse_ship_vars(val: &JsonVal) -> Option<ShipVars> {
    let JsonVal::Obj(m) = val else {
        return None;
    };
    Some(ShipVars {
        inkling: get_i32(m, "inkling")?,
        inbase: get_i32(m, "inbase")?,
        incom: get_i32(m, "incom")?,
        instar: get_i32(m, "instar")?,
        intorps: get_i32(m, "intorps")?,
        condit: get_char(m, "condit")?,
        torps: get_i32(m, "torps")?,
        ship: get_char(m, "ship")?,
        quadx: get_i32(m, "quadx")?,
        quady: get_i32(m, "quady")?,
        sectx: get_i32(m, "sectx")?,
        secty: get_i32(m, "secty")?,
        length: get_i32(m, "length")?,
        skill: get_i32(m, "skill")?,
        basex: get_i32(m, "basex")?,
        basey: get_i32(m, "basey")?,
        klhere: get_i32(m, "klhere")?,
        comhere: get_i32(m, "comhere")?,
        casual: get_i32(m, "casual")?,
        nhelp: get_i32(m, "nhelp")?,
        nkinks: get_i32(m, "nkinks")?,
        ididit: get_i32(m, "ididit")?,
        gamewon: get_i32(m, "gamewon")?,
        alive: get_i32(m, "alive")?,
        justin: get_i32(m, "justin")?,
        alldone: get_i32(m, "alldone")?,
        shldchg: get_i32(m, "shldchg")?,
        thingx: get_i32(m, "thingx")?,
        thingy: get_i32(m, "thingy")?,
        plnetx: get_i32(m, "plnetx")?,
        plnety: get_i32(m, "plnety")?,
        inorbit: get_i32(m, "inorbit")?,
        landed: get_i32(m, "landed")?,
        iplnet: get_i32(m, "iplnet")?,
        imine: get_i32(m, "imine")?,
        inplan: get_i32(m, "inplan")?,
        nenhere: get_i32(m, "nenhere")?,
        ishere: get_i32(m, "ishere")?,
        neutz: get_i32(m, "neutz")?,
        irhere: get_i32(m, "irhere")?,
        icraft: get_i32(m, "icraft")?,
        ientesc: get_i32(m, "ientesc")?,
        iscraft: get_i32(m, "iscraft")?,
        isatb: get_i32(m, "isatb")?,
        iscate: get_i32(m, "iscate")?,
        iscloaked: get_bool(m, "iscloaked")?,
        iscloaking: get_bool(m, "iscloaking")?,
        ncviol: get_i32(m, "ncviol")?,
        isviolreported: get_bool(m, "isviolreported")?,
        brigcapacity: get_i32(m, "brigcapacity")?,
        iattak: get_i32(m, "iattak")?,
        icrystl: get_i32(m, "icrystl")?,
        tourn: get_i32(m, "tourn")?,
        thawed: get_i32(m, "thawed")?,
        batx: get_i32(m, "batx")?,
        baty: get_i32(m, "baty")?,
        ithere: get_i32(m, "ithere")?,
        ithx: get_i32(m, "ithx")?,
        ithy: get_i32(m, "ithy")?,
        iseenit: get_i32(m, "iseenit")?,
        probecx: get_i32(m, "probecx")?,
        probecy: get_i32(m, "probecy")?,
        proben: get_i32(m, "proben")?,
        isarmed: get_bool(m, "isarmed")?,
        nprobes: get_i32(m, "nprobes")?,
        inresor: get_f64(m, "inresor")?,
        intime: get_f64(m, "intime")?,
        inenrg: get_f64(m, "inenrg")?,
        inshld: get_f64(m, "inshld")?,
        inlsr: get_f64(m, "inlsr")?,
        indate: get_f64(m, "indate")?,
        energy: get_f64(m, "energy")?,
        shield: get_f64(m, "shield")?,
        shldup: get_i32(m, "shldup")?,
        warpfac: get_f64(m, "warpfac")?,
        wfacsq: get_f64(m, "wfacsq")?,
        lsupres: get_f64(m, "lsupres")?,
        dist: get_f64(m, "dist")?,
        direc: get_f64(m, "direc")?,
        time: get_f64(m, "time")?,
        docfac: get_f64(m, "docfac")?,
        resting: get_i32(m, "resting")?,
        damfac: get_f64(m, "damfac")?,
        stdamtim: get_f64(m, "stdamtim")?,
        cryprob: get_f64(m, "cryprob")?,
        probex: get_f64(m, "probex")?,
        probey: get_f64(m, "probey")?,
        probeinx: get_f64(m, "probeinx")?,
        probeiny: get_f64(m, "probeiny")?,
    })
}

pub fn deserialize_freeze(state: &mut GameState, raw: &str) -> bool {
    let mut parser = JsonParser::new(raw);
    let Some(JsonVal::Obj(map)) = parser.parse_value() else {
        return false;
    };
    let Some(d) = map.get("d").and_then(parse_galaxy_data) else {
        return false;
    };
    let Some(snapsht) = map.get("snapsht").and_then(parse_galaxy_data) else {
        return false;
    };
    let Some(a) = map.get("a").and_then(parse_ship_vars) else {
        return false;
    };
    let Some(passwd) = get_str(&map, "passwd") else {
        return false;
    };
    let mut quad = [[IHDOT; 11]; 11];
    let Some(JsonVal::Arr(qrows)) = map.get("quad") else {
        return false;
    };
    if qrows.len() != 11 {
        return false;
    }
    for (i, row_val) in qrows.iter().enumerate() {
        let JsonVal::Arr(cols) = row_val else {
            return false;
        };
        if cols.len() != 11 {
            return false;
        }
        for (j, cell) in cols.iter().enumerate() {
            let JsonVal::Str(s) = cell else {
                return false;
            };
            quad[i][j] = s.chars().next().unwrap_or(IHDOT);
        }
    }
    let mut kx = [0; 21];
    let mut ky = [0; 21];
    let mut starch = [[0; 9]; 9];
    let mut kpower = [0.0; 21];
    let mut kdist = [0.0; 21];
    let mut kavgd = [0.0; 21];
    let mut damage = [0.0; 17];
    let mut future = [1e30; 9];
    if map.get("kx").and_then(|v| fill_i32_arr(v, &mut kx)).is_none()
        || map.get("ky").and_then(|v| fill_i32_arr(v, &mut ky)).is_none()
        || map
            .get("starch")
            .and_then(|v| fill_i32_grid_9(v, &mut starch))
            .is_none()
        || map
            .get("kpower")
            .and_then(|v| fill_f64_arr(v, &mut kpower))
            .is_none()
        || map
            .get("kdist")
            .and_then(|v| fill_f64_arr(v, &mut kdist))
            .is_none()
        || map
            .get("kavgd")
            .and_then(|v| fill_f64_arr(v, &mut kavgd))
            .is_none()
        || map
            .get("damage")
            .and_then(|v| fill_f64_arr(v, &mut damage))
            .is_none()
        || map
            .get("future")
            .and_then(|v| fill_f64_arr(v, &mut future))
            .is_none()
    {
        return false;
    }

    state.d = d;
    state.snapsht = snapsht;
    state.quad = quad;
    state.kx = kx;
    state.ky = ky;
    state.starch = starch;
    state.kpower = kpower;
    state.kdist = kdist;
    state.kavgd = kavgd;
    state.damage = damage;
    state.future = future;
    state.a = a;
    state.passwd = passwd;
    if let Some(JsonVal::Num(r)) = map.get("rng_state") {
        let mut s = *r as u32;
        if s == 0 {
            s = 123_459_876;
        }
        state.rng = MacOsRng::from_raw(s);
    }
    if let Some(v) = get_i32(&map, "iscore") {
        state.iscore = v;
    }
    if let Some(v) = get_i32(&map, "iskill") {
        state.iskill = v;
    }
    if let Some(v) = get_f64(&map, "perdate") {
        state.perdate = v;
    }
    if let Some(v) = get_i32(&map, "height") {
        state.height = v;
    }
    true
}
