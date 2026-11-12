//! Combat: phasers, photon torpedoes, shields, cloaking, enemy attack, ramming, and capture.

use crate::ai::movcom;
use crate::ai::movetho;
use crate::events::nova;
use crate::finish::finish;
use crate::io::{huh, pause, prout, prouts, skip, stars, IoHandle};
use crate::reports::dreprt;
use crate::scanner::{ja, scan, ScanToken};
use crate::state::{
    square, FinType, GameState, ALGERON, DCLOAK, DCOMPTR, DDRAY, DEVICES, DPHASER, DPHOTON,
    DSHCTRL, DSHIELD, DSHUTTL, DSRSENS, DTRANSP, FTBEAM, FSCDBAS, FSCMOVE, IHBLANK, IHB, IHC,
    IHDOCKED, IHDOT, IHE, IHF, IHK, IHP, IHQUEST, IHR, IHS, IHSTAR, IHT, IHWEB, NDEVICE, PHASEFAC,
    SFAIR,
};

pub async fn cloak(state: &mut GameState, io: &IoHandle) {
    #[derive(PartialEq, Eq)]
    enum Action {
        ClOn,
        ClOff,
    }
    let action;

    if state.a.ship == IHF {
        prout(state, io, "Ye Faerie Queene has no cloaking device.").await;
        return;
    }

    let key = scan(state, io).await;
    if key == ScanToken::Real {
        return;
    }

    if key == ScanToken::Alpha {
        if state.isit("on") {
            if state.a.iscloaked {
                prout(state, io, "The cloaking device has already been switched on.").await;
                return;
            }
            action = Action::ClOn;
        } else if state.isit("off") {
            if !state.a.iscloaked {
                prout(state, io, "The cloaking device has already been switched off.").await;
                return;
            }
            action = Action::ClOff;
        } else {
            huh(state, io).await;
            return;
        }
    } else {
        if !state.a.iscloaked {
            state.proutn("Switch cloaking device on?");
            if !ja(state, io).await {
                return;
            }
            action = Action::ClOn;
        } else {
            state.proutn("Switch cloaking device off?");
            if !ja(state, io).await {
                return;
            }
            action = Action::ClOff;
        }
    }

    if action == Action::ClOff {
        if state.a.irhere != 0 && state.d.date >= ALGERON && !state.a.isviolreported {
            prout(
                state,
                io,
                "Spock- \"Captain, the Treaty of Algeron is in effect.\n   Are you sure this is wise?\"",
            )
            .await;
            if !ja(state, io).await {
                return;
            }
        }
        prout(state, io, "Engineer Scott- \"Aye, Sir.\"").await;
        state.a.iscloaked = false;
        if state.a.irhere != 0 && state.d.date >= ALGERON && !state.a.isviolreported {
            prout(
                state,
                io,
                "The Romulan ship discovers you are breaking the Treaty of Algeron!",
            )
            .await;
            state.a.ncviol += 1;
            state.a.isviolreported = true;
        }
        return;
    }

    if state.damage[DCLOAK] != 0.0 {
        prout(
            state,
            io,
            "Engineer Scott- \"The cloaking device is damaged, Sir.\"",
        )
        .await;
        return;
    }
    if state.a.condit == IHDOCKED {
        prout(state, io, "You cannot cloak while docked.").await;
        return;
    }
    if state.d.date >= ALGERON && !state.a.isviolreported {
        prout(
            state,
            io,
            "Spock- \"Captain, using the cloaking device is be a violation",
        )
        .await;
        prout(
            state,
            io,
            "  of the Treaty of Algeron. Considering the alternatives,",
        )
        .await;
        state.proutn("  are you sure this is wise?");
        if !ja(state, io).await {
            return;
        }
    }
    prout(
        state,
        io,
        "Engineer Scott- \"The cloaking device has been engaged, Sir.\"",
    )
    .await;
    state.a.iscloaking = true;
    if state.a.irhere != 0 && state.d.date >= ALGERON && !state.a.isviolreported {
        prout(
            state,
            io,
            "The Romulan ship discovers you are breaking the Treaty of Algeron!",
        )
        .await;
        state.a.ncviol += 1;
        state.a.isviolreported = true;
    }
}

pub async fn sheild(state: &mut GameState, io: &IoHandle, i: i32) {
    #[derive(PartialEq, Eq)]
    enum Action {
        None,
        ShUp,
        ShDn,
        Nrg,
    }
    let mut action = Action::None;
    state.a.ididit = 0;

    if i == 2 {
        action = Action::ShUp;
    } else {
        let key = scan(state, io).await;
        if key == ScanToken::Alpha {
            if state.isit("transfer") {
                action = Action::Nrg;
            } else {
                state.chew();
                if state.damage[DSHIELD] != 0.0 {
                    prout(state, io, "Shields damaged and down.").await;
                    return;
                }
                if state.isit("up") {
                    action = Action::ShUp;
                } else if state.isit("down") {
                    action = Action::ShDn;
                }
            }
        }
        if action == Action::None {
            state.proutn("Do you wish to change shield energy? ");
            if ja(state, io).await {
                state.proutn("Energy to transfer to shields- ");
                action = Action::Nrg;
            } else if state.damage[DSHIELD] != 0.0 {
                prout(state, io, "Shields damaged and down.").await;
                return;
            } else if state.a.shldup != 0 {
                state.proutn("Shields are up. Do you want them down? ");
                if ja(state, io).await {
                    action = Action::ShDn;
                } else {
                    state.chew();
                    return;
                }
            } else {
                state.proutn("Shields are down. Do you want them up? ");
                if ja(state, io).await {
                    action = Action::ShUp;
                } else {
                    state.chew();
                    return;
                }
            }
        }
    }

    match action {
        Action::ShUp => {
            if state.a.shldup != 0 {
                prout(state, io, "Shields already up.").await;
                return;
            }
            state.a.shldup = 1;
            state.a.shldchg = 1;
            if state.a.condit != IHDOCKED {
                state.a.energy -= 50.0;
            }
            prout(state, io, "Shields raised.").await;
            if state.a.energy <= 0.0 {
                skip(state, io, 1).await;
                prout(state, io, "Shields raising uses up last of energy.").await;
                finish(state, io, FinType::Nrg).await;
                return;
            }
            state.a.ididit = 1;
        }
        Action::ShDn => {
            if state.a.shldup == 0 {
                prout(state, io, "Shields already down.").await;
                return;
            }
            state.a.shldup = 0;
            state.a.shldchg = 1;
            prout(state, io, "Shields lowered.").await;
            state.a.ididit = 1;
        }
        Action::Nrg => {
            while scan(state, io).await != ScanToken::Real {
                state.chew();
                state.proutn("Energy to transfer to shields- ");
            }
            state.chew();
            if state.aaitem == 0.0 {
                return;
            }
            if state.aaitem > state.a.energy {
                prout(state, io, "Insufficient ship energy.").await;
                return;
            }
            state.a.ididit = 1;
            if state.a.shield + state.aaitem >= state.a.inshld {
                prout(state, io, "Shield energy maximized.").await;
                if state.a.shield + state.aaitem > state.a.inshld {
                    prout(
                        state,
                        io,
                        "Excess energy requested returned to ship energy",
                    )
                    .await;
                }
                state.a.energy -= state.a.inshld - state.a.shield;
                state.a.shield = state.a.inshld;
                return;
            }
            if state.aaitem < 0.0 && state.a.energy - state.aaitem > state.a.inenrg {
                skip(state, io, 1).await;
                prout(state, io, "Engineering to bridge--").await;
                prout(
                    state,
                    io,
                    "  Scott here. Power circuit problem, Captain.",
                )
                .await;
                prout(state, io, "  I can't drain the shields.").await;
                state.a.ididit = 0;
                return;
            }
            if state.a.shield + state.aaitem < 0.0 {
                prout(state, io, "All shield energy transferred to ship.").await;
                state.a.energy += state.a.shield;
                state.a.shield = 0.0;
                return;
            }
            state.proutn("Scotty- \"");
            if state.aaitem > 0.0 {
                prout(state, io, "Transferring energy to shields.\"").await;
            } else {
                prout(state, io, "Draining energy from shields.\"").await;
            }
            state.a.shield += state.aaitem;
            state.a.energy -= state.aaitem;
        }
        Action::None => {}
    }
}

pub async fn ram(
    state: &mut GameState,
    io: &IoHandle,
    ibumpd: bool,
    ienm: char,
    ix: i32,
    iy: i32,
) {
    prouts(state, "***RED ALERT!  RED ALERT!");
    skip(state, io, 1).await;
    prout(state, io, "***COLLISION IMMINENT.").await;
    skip(state, io, 2).await;
    state.proutn("***");
    state.crmshp();
    let typ = match ienm {
        IHR => 1.5,
        IHC => 2.0,
        IHS => 2.5,
        IHT => 0.5,
        _ => 1.0,
    };
    state.proutn(if ibumpd { " rammed by " } else { " rams " });
    state.crmena(0, ienm, 2, ix, iy);
    if ibumpd {
        state.proutn(" (original position)");
    }
    skip(state, io, 1).await;
    deadkl(state, io, ix, iy, ienm, state.a.sectx, state.a.secty).await;
    state.proutn("***");
    state.crmshp();
    prout(state, io, " heavily damaged.").await;
    let icas = (10.0 + 20.0 * state.rng.rand_f64()) as i32;
    state.proutn("***Sickbay reports ");
    state.crami(icas, 1);
    prout(state, io, " casualties.").await;
    state.a.casual += icas;
    for l in 1..=NDEVICE {
        if l == DDRAY || state.damage[l] < 0.0 {
            continue;
        }
        let extradm = (10.0 * typ * state.rng.rand_f64() + 1.0) * state.a.damfac;
        state.damage[l] += state.a.time + extradm;
    }
    state.a.shldup = 0;
    if state.d.remkl != 0 {
        pause(state, io, 2).await;
        dreprt(state, io).await;
    } else {
        finish(state, io, FinType::Won).await;
    }
}

pub async fn torpedo(
    state: &mut GameState,
    io: &IoHandle,
    course: f64,
    r: f64,
    inx: i32,
    iny: i32,
    hit: &mut f64,
) {
    let ac = course + 0.25 * r;
    let angle = (15.0 - ac) * 0.5235988;
    let bullseye = (15.0 - course) * 0.5235988;
    let mut deltax = -angle.sin();
    let mut deltay = angle.cos();
    let mut x = inx as f64;
    let mut y = iny as f64;

    let bigger = deltax.abs().max(deltay.abs());
    deltax /= bigger;
    deltay /= bigger;

    let mut shoved = false;
    let mut ix = 0i32;
    let mut iy = 0i32;
    let mut jx = 0i32;
    let mut jy = 0i32;
    let mut iquad = IHDOT;

    for l in 1..=15 {
        x += deltax;
        ix = (x + 0.5) as i32;
        if !(1..=10).contains(&ix) {
            break;
        }
        y += deltay;
        iy = (y + 0.5) as i32;
        if !(1..=10).contains(&iy) {
            break;
        }
        if l == 4 || l == 9 {
            skip(state, io, 1).await;
        }
        state.cramf(x, 0, 1);
        state.proutn(" - ");
        state.cramf(y, 0, 1);
        state.proutn("   ");
        iquad = state.quad[ix as usize][iy as usize];
        if iquad == IHDOT {
            continue;
        }

        skip(state, io, 1).await;
        match iquad {
            IHE | IHF => {
                skip(state, io, 1).await;
                state.proutn("Torpedo hits ");
                state.crmshp();
                prout(state, io, ".").await;
                *hit = (700.0 + 100.0 * state.rng.rand_f64()
                    - 1000.0
                        * (square((ix - inx) as f64) + square((iy - iny) as f64)).sqrt()
                        * (bullseye - angle).sin().abs())
                .abs();
                state.newcnd();
                if state.a.landed == 1 {
                    return;
                }
                let ang = angle + 2.5 * (state.rng.rand_f64() - 0.5);
                let temp = ang.sin().abs().max(ang.cos().abs());
                let xx = -ang.sin() / temp;
                let yy = ang.cos() / temp;
                jx = ((ix as f64) + xx + 0.5) as i32;
                jy = ((iy as f64) + yy + 0.5) as i32;
                if !(1..=10).contains(&jx) || !(1..=10).contains(&jy) {
                    return;
                }
                if state.quad[jx as usize][jy as usize] == IHBLANK {
                    finish(state, io, FinType::Hole).await;
                    return;
                }
                if state.quad[jx as usize][jy as usize] != IHDOT {
                    return;
                }
                state.a.sectx = jx;
                state.a.secty = jy;
                state.crmshp();
                shoved = true;
                break;
            }
            IHC | IHS | IHR | IHK => {
                if (iquad == IHC || iquad == IHS) && state.rng.rand_f64() <= 0.05 {
                    state.crmena(1, iquad, 2, ix, iy);
                    prout(state, io, " uses anti-photon device;").await;
                    prout(state, io, "   torpedo neutralized.").await;
                    return;
                }
                let mut ll = 1usize;
                while ll <= state.a.nenhere as usize {
                    if ix == state.kx[ll] && iy == state.ky[ll] {
                        break;
                    }
                    ll += 1;
                }
                let kp = state.kpower[ll].abs();
                let mut h1 = (700.0 + 100.0 * state.rng.rand_f64()
                    - 1000.0
                        * (square((ix - inx) as f64) + square((iy - iny) as f64)).sqrt()
                        * (bullseye - angle).sin().abs())
                .abs();
                if kp < h1 {
                    h1 = kp;
                }
                state.kpower[ll] -= if state.kpower[ll] < 0.0 { -h1 } else { h1 };
                if state.kpower[ll] == 0.0 {
                    deadkl(state, io, ix, iy, iquad, ix, iy).await;
                    return;
                }
                state.crmena(1, iquad, 2, ix, iy);
                let ang = angle + 2.5 * (state.rng.rand_f64() - 0.5);
                let temp = ang.sin().abs().max(ang.cos().abs());
                let xx = -ang.sin() / temp;
                let yy = ang.cos() / temp;
                jx = ((ix as f64) + xx + 0.5) as i32;
                jy = ((iy as f64) + yy + 0.5) as i32;
                if !(1..=10).contains(&jx) || !(1..=10).contains(&jy) {
                    prout(state, io, " damaged but not destroyed.").await;
                    return;
                }
                if state.quad[jx as usize][jy as usize] == IHBLANK {
                    prout(state, io, " buffeted into black hole.").await;
                    deadkl(state, io, ix, iy, iquad, jx, jy).await;
                    return;
                }
                if state.quad[jx as usize][jy as usize] != IHDOT {
                    prout(state, io, " damaged but not destroyed.").await;
                    return;
                }
                prout(state, io, " damaged--").await;
                state.kx[ll] = jx;
                state.ky[ll] = jy;
                shoved = true;
                break;
            }
            IHB => {
                prout(state, io, "***STARBASE DESTROYED..").await;
                let qx = state.a.quadx as usize;
                let qy = state.a.quady as usize;
                if state.starch[qx][qy] < 0 {
                    state.starch[qx][qy] = 0;
                }
                let remb = state.d.rembase as usize;
                for ll in 1..=remb {
                    if state.d.baseqx[ll] == state.a.quadx
                        && state.d.baseqy[ll] == state.a.quady
                    {
                        state.d.baseqx[ll] = state.d.baseqx[remb];
                        state.d.baseqy[ll] = state.d.baseqy[remb];
                        break;
                    }
                }
                state.quad[ix as usize][iy as usize] = IHDOT;
                state.d.rembase -= 1;
                state.a.basex = 0;
                state.a.basey = 0;
                state.d.galaxy[qx][qy] -= 10;
                state.d.basekl += 1;
                state.newcnd();
                return;
            }
            IHP => {
                state.crmena(1, iquad, 2, ix, iy);
                prout(state, io, " destroyed.").await;
                state.d.nplankl += 1;
                state.d.newstuf[state.a.quadx as usize][state.a.quady as usize] -= 1;
                state.d.plnets[state.a.iplnet as usize] = crate::state::Planet::default();
                state.a.iplnet = 0;
                state.a.plnetx = 0;
                state.a.plnety = 0;
                state.quad[ix as usize][iy as usize] = IHDOT;
                if state.a.landed == 1 {
                    finish(state, io, FinType::Dplanet).await;
                }
                return;
            }
            IHSTAR => {
                if state.rng.rand_f64() > 0.10 {
                    Box::pin(nova(state, io, ix, iy)).await;
                    return;
                }
                state.crmena(1, IHSTAR, 2, ix, iy);
                prout(state, io, " unaffected by photon blast.").await;
                return;
            }
            IHQUEST => {
                skip(state, io, 1).await;
                prouts(state, "AAAAIIIIEEEEEEEEAAAAAAAAUUUUUGGGGGHHHHHHHHHHHH!!!");
                skip(state, io, 1).await;
                prouts(state, "    HACK!     HACK!    HACK!        *CHOKE!*  ");
                skip(state, io, 1).await;
                state.proutn("Mr. Spock-");
                prouts(state, "  \"Fascinating!\"");
                skip(state, io, 1).await;
                state.quad[ix as usize][iy as usize] = IHDOT;
                return;
            }
            IHBLANK => {
                skip(state, io, 1).await;
                state.crmena(1, IHBLANK, 2, ix, iy);
                prout(state, io, " swallows torpedo.").await;
                return;
            }
            IHWEB => {
                skip(state, io, 1).await;
                prout(state, io, "***Torpedo absorbed by Tholian web.").await;
                return;
            }
            IHT => {
                skip(state, io, 1).await;
                state.crmena(1, IHT, 2, ix, iy);
                let h1 = (700.0 + 100.0 * state.rng.rand_f64()
                    - 1000.0
                        * (square((ix - inx) as f64) + square((iy - iny) as f64)).sqrt()
                        * (bullseye - angle).sin().abs())
                .abs();
                if h1 >= 600.0 {
                    prout(state, io, " destroyed.").await;
                    state.quad[ix as usize][iy as usize] = IHDOT;
                    state.a.ithere = 0;
                    state.a.ithx = 0;
                    state.a.ithy = 0;
                    return;
                }
                if state.rng.rand_f64() > 0.05 {
                    prout(state, io, " survives photon blast.").await;
                    return;
                }
                prout(state, io, " disappears.").await;
                state.quad[ix as usize][iy as usize] = IHWEB;
                state.a.ithere = 0;
                state.a.ithx = 0;
                state.a.ithy = 0;
                state.dropin(IHBLANK);
                return;
            }
            _ => {
                skip(state, io, 1).await;
                state.proutn("Don't know how to handle collision with ");
                state.crmena(1, iquad, 2, ix, iy);
                skip(state, io, 1).await;
                return;
            }
        }
    }

    if shoved {
        state.quad[jx as usize][jy as usize] = iquad;
        state.quad[ix as usize][iy as usize] = IHDOT;
        state.proutn(" displaced by blast to");
        state.cramlc(2, jx, jy);
        skip(state, io, 1).await;
        for ll in 1..=(state.a.nenhere as usize) {
            let d = (square((state.a.sectx - state.kx[ll]) as f64)
                + square((state.a.secty - state.ky[ll]) as f64))
            .sqrt();
            state.kdist[ll] = d;
            state.kavgd[ll] = d;
        }
        state.sortkl();
        return;
    }

    skip(state, io, 1).await;
    prout(state, io, "Torpedo missed.").await;
}

async fn fry(state: &mut GameState, io: &IoHandle, hit: f64) {
    if hit < (275.0 - 25.0 * (state.a.skill as f64)) * (1.0 + 0.5 * state.rng.rand_f64()) {
        return;
    }

    let ncrit = 1.0 + hit / (500.0 + 100.0 * state.rng.rand_f64());
    let ncrit_count = ncrit as usize;
    state.proutn("***CRITICAL HIT--");

    let mut cdam: Vec<usize> = Vec::with_capacity(ncrit_count);
    let mut ktr = 1usize;

    for l in 1..=ncrit_count {
        let j = loop {
            let candidate = ((NDEVICE as f64) * state.rng.rand_f64() + 1.0) as usize;
            if state.damage[candidate] < 0.0
                || (candidate == DSHUTTL && state.a.iscraft != 1)
                || (candidate == DCLOAK && state.a.ship != IHE)
                || candidate == DDRAY
            {
                continue;
            }
            break candidate;
        };
        let dup = cdam.contains(&j);
        cdam.push(j);
        let extradm = (hit * state.a.damfac) / (ncrit * (75.0 + 25.0 * state.rng.rand_f64()));
        state.damage[j] += extradm;
        if l > 1 {
            if dup {
                continue;
            }
            ktr += 1;
            if ktr == 3 {
                skip(state, io, 1).await;
            }
            state.proutn(" and ");
        }
        state.proutn(DEVICES[j]);
    }
    prout(state, io, " damaged.").await;
    if state.damage[DSHIELD] != 0.0 && state.a.shldup != 0 {
        prout(state, io, "***Shields knocked down.").await;
        state.a.shldup = 0;
    }
    if state.damage[DCLOAK] != 0.0 && state.a.iscloaked {
        prout(state, io, "***Cloaking device rendered inoperative.").await;
        state.a.iscloaked = false;
    }
}

pub async fn attack(state: &mut GameState, io: &IoHandle, k: i32) {
    if state.a.iscloaked && !state.a.iscloaking {
        return;
    }
    state.a.iattak = 1;
    if state.a.alldone != 0 {
        return;
    }
    if state.debug_mode {
        prout(state, io, "ATTACK!").await;
    }

    if state.a.ithere != 0 {
        movetho(state, io).await;
    }
    if state.a.neutz != 0 {
        state.a.neutz = 0;
        return;
    }
    if ((state.a.comhere != 0 || state.a.ishere != 0) && state.a.justin == 0)
        || state.a.skill == crate::state::SEMERITUS
    {
        movcom(state, io).await;
    }
    if state.a.nenhere == 0 {
        return;
    }

    let pfac = 1.0 / state.a.inshld;
    let chgfac = if state.a.shldchg == 1 {
        0.25 + 0.5 * state.rng.rand_f64()
    } else {
        1.0
    };
    skip(state, io, 1).await;
    let i = if state.a.skill <= SFAIR { 2 } else { 0 };
    let mut ihurt = false;
    let mut atackd = false;
    let mut attempt = false;
    let mut hitmax = 0.0f64;
    let mut hittot = 0.0f64;

    let mut l = 1usize;
    while l <= state.a.nenhere as usize {
        let idx = l;
        l += 1;
        if state.kpower[idx] < 0.0 {
            continue;
        }
        let mut r = state.rng.rand_f64();
        if state.a.condit == IHDOCKED {
            r *= 0.25;
        }
        if state.kpower[idx] < 500.0 {
            r *= 0.25;
        }
        let jx = state.kx[idx];
        let jy = state.ky[idx];
        let iquad = state.quad[jx as usize][jy as usize];
        let itflag = (iquad == IHK && r > 0.0005)
            || k == 0
            || (iquad == IHC && r > 0.015)
            || (iquad == IHR && r > 0.3)
            || (iquad == IHS && r > 0.07);
        let mut hit: f64;
        if itflag {
            if state.a.condit == IHDOCKED {
                continue;
            }
            attempt = true;
            let dustfac = 0.8 + 0.05 * state.rng.rand_f64();
            hit = state.kpower[idx] * dustfac.powf(state.kavgd[idx]);
            state.kpower[idx] *= 0.75;
        } else {
            let course =
                1.90985 * ((state.a.secty - jy) as f64).atan2((jx - state.a.sectx) as f64);
            hit = 0.0;
            state.proutn("***TORPEDO INCOMING");
            if state.damage[DSRSENS] <= 0.0 {
                state.proutn(" From ");
                state.crmena(0, iquad, i, jx, jy);
            }
            attempt = true;
            prout(state, io, "--").await;
            let mut tr = (state.rng.rand_f64() + state.rng.rand_f64()) * 0.5 - 0.5;
            tr += 0.002 * state.kpower[idx] * tr;
            torpedo(state, io, course, tr, jx, jy, &mut hit).await;
            if state.d.remkl == 0 {
                finish(state, io, FinType::Won).await;
            }
            if state.d.galaxy[state.a.quadx as usize][state.a.quady as usize] == 1000
                || state.a.alldone != 0
            {
                return;
            }
            if idx <= state.a.nenhere as usize
                && (state.kx[idx] != jx || state.ky[idx] != jy)
            {
                l = idx;
            }
            if hit == 0.0 {
                continue;
            }
        }

        if state.a.shldup != 0 || state.a.shldchg != 0 {
            let mut propor = pfac * state.a.shield;
            if propor < 0.1 {
                propor = 0.1;
            }
            let hitsh = propor * chgfac * hit + 1.0;
            atackd = true;
            let mut absorb = 0.8 * hitsh;
            if absorb > state.a.shield {
                absorb = state.a.shield;
            }
            state.a.shield -= absorb;
            hit -= hitsh;
            if propor > 0.1 && hit < 0.005 * state.a.energy {
                continue;
            }
        }

        atackd = true;
        ihurt = true;
        state.cramf(hit, 0, 2);
        state.proutn(" unit hit");
        if (state.damage[DSRSENS] > 0.0 && itflag) || state.a.skill <= SFAIR {
            state.proutn(" on the ");
            state.crmshp();
        }
        if state.damage[DSRSENS] <= 0.0 && itflag {
            state.proutn(" from ");
            state.crmena(0, iquad, i, jx, jy);
        }
        skip(state, io, 1).await;
        if hit > hitmax {
            hitmax = hit;
        }
        hittot += hit;
        fry(state, io, hit).await;
        state.a.energy -= hit;
    }

    if state.a.energy <= 0.0 {
        finish(state, io, FinType::Battle).await;
        return;
    }
    if !attempt && state.a.condit == IHDOCKED {
        prout(state, io, "***Enemies decide against attacking your ship.").await;
    }
    if !atackd {
        return;
    }
    let percent = (100.0 * pfac * state.a.shield + 0.5) as i32;
    if !ihurt {
        state.proutn("Enemy attack reduces shield strength to ");
    } else {
        skip(state, io, 1).await;
        state.proutn("Energy left ");
        state.cramf(state.a.energy, 0, 2);
        state.proutn("    shields ");
        if state.a.shldup != 0 {
            state.proutn("up, ");
        } else if state.damage[DSHIELD] == 0.0 {
            state.proutn("down, ");
        } else {
            state.proutn("damaged, ");
        }
    }
    state.crami(percent, 1);
    state.proutn("%   torpedoes left ");
    state.crami(state.a.torps, 1);
    skip(state, io, 1).await;

    if hitmax >= 200.0 || hittot >= 500.0 {
        let icas = (hittot * state.rng.rand_f64() * 0.015) as i32;
        if icas >= 2 {
            skip(state, io, 1).await;
            state.proutn("Mc Coy-  \"Sickbay to bridge.  We suffered ");
            state.crami(icas, 1);
            prout(state, io, " casualties").await;
            prout(state, io, "   in that last attack.\"").await;
            state.a.casual += icas;
        }
    }

    for l in 1..=(state.a.nenhere as usize) {
        state.kavgd[l] = state.kdist[l];
    }
    state.sortkl();
}

pub async fn deadkl(
    state: &mut GameState,
    io: &IoHandle,
    ix: i32,
    iy: i32,
    typ: char,
    ixx: i32,
    iyy: i32,
) {
    match typ {
        IHR | IHK | IHC | IHS | IHT => {}
        _ => return,
    }

    state.crmena(1, typ, 2, ixx, iyy);
    if typ == IHR {
        state.d.newstuf[state.a.quadx as usize][state.a.quady as usize] -= 10;
        state.a.irhere -= 1;
        state.d.nromkl += 1;
        state.d.nromrem -= 1;
    } else if typ == IHT {
        state.a.ithere = 0;
    } else {
        state.d.galaxy[state.a.quadx as usize][state.a.quady as usize] -= 100;
        state.a.klhere -= 1;
        state.d.remkl -= 1;
        match typ {
            IHC => {
                state.a.comhere = 0;
                let remc = state.d.remcom as usize;
                let mut found = 1usize;
                while found <= remc {
                    if state.d.cx[found] == state.a.quadx && state.d.cy[found] == state.a.quady
                    {
                        break;
                    }
                    found += 1;
                }
                if found <= remc {
                    state.d.cx[found] = state.d.cx[remc];
                    state.d.cy[found] = state.d.cy[remc];
                }
                state.d.cx[remc] = 0;
                state.d.cy[remc] = 0;
                state.d.remcom -= 1;
                state.future[FTBEAM] = 1e30;
                if state.d.remcom != 0 {
                    state.future[FTBEAM] = state.d.date
                        + state
                            .rng
                            .expran(1.0 * (state.a.incom as f64) / (state.d.remcom as f64));
                }
                state.d.killc += 1;
            }
            IHK => {
                state.d.killk += 1;
            }
            IHS => {
                state.d.nscrem = 0;
                state.a.ishere = 0;
                state.d.isx = 0;
                state.d.isy = 0;
                state.a.isatb = 0;
                state.a.iscate = 0;
                state.d.nsckill = 1;
                state.future[FSCMOVE] = 1e30;
                state.future[FSCDBAS] = 1e30;
            }
            _ => {}
        }
    }

    prout(state, io, " destroyed.").await;
    state.quad[ix as usize][iy as usize] = IHDOT;
    let denom = state.d.remkl + 4 * state.d.remcom;
    if denom > 0 {
        state.d.remtime = state.d.remres / (denom as f64);
    }
    if typ == IHT {
        return;
    }

    let nh = state.a.nenhere as usize;
    let mut i = 1usize;
    while i <= nh {
        if state.kx[i] == ix && state.ky[i] == iy {
            break;
        }
        i += 1;
    }
    if i <= nh && state.a.nenhere > 0 {
        state.a.nenhere -= 1;
        let new_nh = state.a.nenhere as usize;
        if i <= new_nh {
            for j in i..=new_nh {
                state.kx[j] = state.kx[j + 1];
                state.ky[j] = state.ky[j + 1];
                state.kpower[j] = state.kpower[j + 1];
                state.kdist[j] = state.kdist[j + 1];
                state.kavgd[j] = state.kdist[j + 1];
            }
        }
        state.kx[new_nh + 1] = 0;
        state.ky[new_nh + 1] = 0;
        state.kdist[new_nh + 1] = 0.0;
        state.kavgd[new_nh + 1] = 0.0;
        state.kpower[new_nh + 1] = 0.0;
    }
}

async fn targetcheck(
    state: &mut GameState,
    io: &IoHandle,
    x: f64,
    y: f64,
    course: &mut f64,
) -> bool {
    if !(1.0..=10.0).contains(&x) || !(1.0..=10.0).contains(&y) {
        huh(state, io).await;
        return true;
    }
    let deltx = 0.1 * (y - (state.a.secty as f64));
    let delty = 0.1 * ((state.a.sectx as f64) - x);
    if deltx == 0.0 && delty == 0.0 {
        skip(state, io, 1).await;
        prout(state, io, "Spock-  \"Bridge to sickbay.  Dr. McCoy,").await;
        prout(state, io, "  I recommend an immediate review of").await;
        prout(state, io, "  the Captain's psychological profile.").await;
        state.chew();
        return true;
    }
    *course = 1.90985932 * deltx.atan2(delty);
    false
}

pub async fn photon(state: &mut GameState, io: &IoHandle) {
    let mut targ = [[0.0f64; 3]; 4];
    let mut course = [0.0f64; 4];
    let mut dummy = 0.0f64;

    state.a.ididit = 0;

    if state.damage[DPHOTON] != 0.0 {
        prout(state, io, "Photon tubes damaged.").await;
        state.chew();
        return;
    }
    if state.a.torps == 0 {
        prout(state, io, "No torpedoes left.").await;
        state.chew();
        return;
    }

    let mut key = scan(state, io).await;
    let n: usize;
    loop {
        match key {
            ScanToken::Alpha => {
                huh(state, io).await;
                return;
            }
            ScanToken::Eol => {
                state.crami(state.a.torps, 1);
                prout(state, io, " torpedoes left.").await;
                state.proutn("Number of torpedoes to fire- ");
                key = scan(state, io).await;
            }
            ScanToken::Real => {
                let nv = (state.aaitem + 0.5) as i32;
                if nv <= 0 {
                    state.chew();
                    return;
                }
                if nv > 3 {
                    prout(state, io, "Maximum of 3 torpedoes per burst.").await;
                } else if nv <= state.a.torps {
                    n = nv as usize;
                    break;
                }
                state.chew();
                key = ScanToken::Eol;
            }
        }
    }

    let mut i = 1usize;
    while i <= n {
        key = scan(state, io).await;
        if i == 1 && key == ScanToken::Eol {
            break;
        }
        if i == 2 && key == ScanToken::Eol {
            while i <= n {
                targ[i][1] = targ[1][1];
                targ[i][2] = targ[1][2];
                course[i] = course[1];
                i += 1;
            }
            break;
        }
        if key != ScanToken::Real {
            huh(state, io).await;
            return;
        }
        targ[i][1] = state.aaitem;
        key = scan(state, io).await;
        if key != ScanToken::Real {
            huh(state, io).await;
            return;
        }
        targ[i][2] = state.aaitem;
        if targetcheck(state, io, targ[i][1], targ[i][2], &mut course[i]).await {
            return;
        }
        i += 1;
    }
    state.chew();
    if i == 1 && key == ScanToken::Eol {
        for idx in 1..=n {
            state.proutn("Target sector for torpedo number");
            state.crami(idx as i32, 2);
            state.proutn("- ");
            key = scan(state, io).await;
            if key != ScanToken::Real {
                huh(state, io).await;
                return;
            }
            targ[idx][1] = state.aaitem;
            key = scan(state, io).await;
            if key != ScanToken::Real {
                huh(state, io).await;
                return;
            }
            targ[idx][2] = state.aaitem;
            state.chew();
            if targetcheck(state, io, targ[idx][1], targ[idx][2], &mut course[idx]).await {
                return;
            }
        }
    }

    state.a.ididit = 1;
    let mut osuabor = false;
    for idx in 1..=n {
        if osuabor {
            break;
        }
        if state.a.condit != IHDOCKED {
            state.a.torps -= 1;
        }
        let mut r = (state.rng.rand_f64() + state.rng.rand_f64()) * 0.5 - 0.5;
        if r.abs() >= 0.47 {
            r = (state.rng.rand_f64() + 1.2) * r;
            if n > 1 {
                prouts(state, "***TORPEDO NUMBER");
                state.crami(idx as i32, 2);
                prouts(state, " MISFIRES.");
            } else {
                prouts(state, "***TORPEDO MISFIRES.");
            }
            skip(state, io, 1).await;
            if idx < n {
                prout(state, io, "  Remainder of burst aborted.").await;
            }
            osuabor = true;
            if state.rng.rand_f64() <= 0.2 {
                prout(state, io, "***Photon tubes damaged by misfire.").await;
                state.damage[DPHOTON] =
                    state.a.damfac * (1.0 + 2.0 * state.rng.rand_f64());
                break;
            }
        }
        if state.a.iscloaked {
            r *= 1.2;
        } else if state.a.shldup != 0 || state.a.condit == IHDOCKED {
            r *= 1.0 + 0.0001 * state.a.shield;
        }
        if n != 1 {
            skip(state, io, 1).await;
            state.proutn("Track for torpedo number");
            state.crami(idx as i32, 2);
            state.proutn("-   ");
        } else {
            skip(state, io, 1).await;
            state.proutn("Torpedo track- ");
        }
        torpedo(
            state,
            io,
            course[idx],
            r,
            state.a.sectx,
            state.a.secty,
            &mut dummy,
        )
        .await;
        if state.a.alldone != 0
            || state.d.galaxy[state.a.quadx as usize][state.a.quady as usize] == 1000
        {
            return;
        }
    }
    if state.d.remkl == 0 {
        finish(state, io, FinType::Won).await;
    }
}

async fn overheat(state: &mut GameState, io: &IoHandle, rpow: f64) {
    if rpow > 1500.0 {
        let chekbrn = (rpow - 1500.0) * 0.00038;
        if state.rng.rand_f64() <= chekbrn {
            prout(
                state,
                io,
                "Weapons officer Sulu-  \"Phasers overheated, sir.\"",
            )
            .await;
            state.damage[DPHASER] =
                state.a.damfac * (1.0 + state.rng.rand_f64()) * (1.0 + chekbrn);
        }
    }
}

async fn checkshctrl(state: &mut GameState, io: &IoHandle, rpow: f64) -> bool {
    skip(state, io, 1).await;
    if state.rng.rand_f64() < 0.998 {
        prout(state, io, "Shields lowered.").await;
        return false;
    }
    prouts(state, "***RED ALERT!  RED ALERT!");
    skip(state, io, 2).await;
    let hit = rpow * state.a.shield / state.a.inshld;
    state.a.energy -= rpow + hit * 0.8;
    state.a.shield -= hit * 0.2;
    if state.a.energy <= 0.0 {
        prouts(state, "Sulu-  \"Captain! Shield malf***********************\"");
        skip(state, io, 1).await;
        stars(state, io).await;
        finish(state, io, FinType::Phaser).await;
        return true;
    }
    prouts(
        state,
        "Sulu-  \"Captain! Shield malfunction! Phaser fire contained!\"",
    );
    skip(state, io, 2).await;
    prout(state, io, "Lt. Uhura-  \"Sir, all decks reporting damage.\"").await;
    let icas = (hit * state.rng.rand_f64() * 0.012) as i32;
    skip(state, io, 1).await;
    fry(state, io, 0.8 * hit).await;
    if icas != 0 {
        skip(state, io, 1).await;
        prout(state, io, "McCoy to bridge- \"Severe radiation burns, Jim.").await;
        state.proutn("  ");
        state.crami(icas, 1);
        prout(state, io, " casualties so far.\"").await;
        state.a.casual += icas;
    }
    skip(state, io, 1).await;
    prout(state, io, "Phaser energy dispersed by shields.").await;
    prout(state, io, "Enemy unaffected.").await;
    overheat(state, io, rpow).await;
    true
}

pub async fn phasers(state: &mut GameState, io: &IoHandle) {
    #[derive(PartialEq, Eq)]
    enum AutoMode {
        NotSet,
        Manual,
        ForceMan,
        Automatic,
    }
    let mut hits = [0.0f64; 21];
    let mut rpow = 0.0f64;
    let mut kz = 0usize;
    let mut ifast = false;
    let mut no = false;
    let ipoop = state.damage[DSRSENS] + state.damage[DCOMPTR] <= 0.0;
    let mut msgflag = true;
    let mut automode = AutoMode::NotSet;
    let mut key = ScanToken::Eol;

    skip(state, io, 1).await;
    if state.a.condit == IHDOCKED {
        prout(
            state,
            io,
            "Phasers can't be fired through base shields.",
        )
        .await;
        state.chew();
        return;
    }
    if state.damage[DPHASER] != 0.0 {
        prout(state, io, "Phaser control damaged.").await;
        state.chew();
        return;
    }
    if state.a.shldup != 0 {
        if state.damage[DSHCTRL] != 0.0 {
            prout(state, io, "High speed shield control damaged.").await;
            state.chew();
            return;
        }
        if state.a.energy <= 200.0 {
            prout(
                state,
                io,
                "Insufficient energy to activate high-speed shield control.",
            )
            .await;
            state.chew();
            return;
        }
        prout(
            state,
            io,
            "Weapons Officer Sulu-  \"High-speed shield control enabled, sir.\"",
        )
        .await;
        ifast = true;
    }

    state.a.ididit = 1;
    while automode == AutoMode::NotSet {
        key = scan(state, io).await;
        match key {
            ScanToken::Alpha => {
                if state.isit("manual") {
                    if state.a.nenhere == 0 {
                        prout(state, io, "There is no enemy present to select.").await;
                        state.chew();
                        key = ScanToken::Eol;
                        automode = AutoMode::Automatic;
                    } else {
                        automode = AutoMode::Manual;
                        key = scan(state, io).await;
                    }
                } else if state.isit("automatic") {
                    if !ipoop && state.a.nenhere != 0 {
                        automode = AutoMode::ForceMan;
                    } else {
                        if state.a.nenhere == 0 {
                            prout(state, io, "Energy will be expended into space.").await;
                        }
                        automode = AutoMode::Automatic;
                        key = scan(state, io).await;
                    }
                } else if state.isit("no") {
                    no = true;
                } else {
                    huh(state, io).await;
                    state.a.ididit = 0;
                    return;
                }
            }
            ScanToken::Real => {
                if state.a.nenhere == 0 {
                    prout(state, io, "Energy will be expended into space.").await;
                    automode = AutoMode::Automatic;
                } else if !ipoop {
                    automode = AutoMode::ForceMan;
                } else {
                    automode = AutoMode::Automatic;
                }
            }
            ScanToken::Eol => {
                if state.a.nenhere == 0 {
                    prout(state, io, "Energy will be expended into space.").await;
                    automode = AutoMode::Automatic;
                } else if !ipoop {
                    automode = AutoMode::ForceMan;
                } else {
                    state.proutn("Manual or automatic? ");
                }
            }
        }
    }

    match automode {
        AutoMode::Automatic => {
            if key == ScanToken::Alpha && state.isit("no") {
                no = true;
                key = scan(state, io).await;
            }
            let avail = if ifast {
                state.a.energy - 200.0
            } else {
                state.a.energy
            };
            if key != ScanToken::Real && state.a.nenhere != 0 {
                state.proutn("Phasers locked on target. Energy available =");
                state.cramf(avail, 1, 2);
                skip(state, io, 1).await;
            }
            loop {
                while key != ScanToken::Real {
                    state.chew();
                    state.proutn("Units to fire=");
                    key = scan(state, io).await;
                }
                rpow = state.aaitem;
                let avail_now = if ifast {
                    state.a.energy - 200.0
                } else {
                    state.a.energy
                };
                if rpow >= avail_now {
                    state.proutn("Energy available= ");
                    state.cramf(avail_now, 1, 2);
                    skip(state, io, 1).await;
                    key = ScanToken::Eol;
                } else {
                    break;
                }
            }
            if rpow <= 0.0 {
                state.a.ididit = 0;
                state.chew();
                return;
            }
            key = scan(state, io).await;
            if key == ScanToken::Alpha && state.isit("no") {
                no = true;
            }
            if ifast {
                state.a.energy -= 200.0;
                if checkshctrl(state, io, rpow).await {
                    return;
                }
            }
            state.chew();
            state.a.energy -= rpow;
            let mut extra = rpow;
            if state.a.nenhere != 0 {
                extra = 0.0;
                let mut powrem = rpow;
                for i in 1..=(state.a.nenhere as usize) {
                    hits[i] = 0.0;
                    if powrem <= 0.0 {
                        continue;
                    }
                    hits[i] = state.kpower[i].abs() / (PHASEFAC * 0.90f64.powf(state.kdist[i]));
                    let mut over = (0.01 + 0.05 * state.rng.rand_f64()) * hits[i];
                    let temp = powrem;
                    powrem -= hits[i] + over;
                    if powrem <= 0.0 && temp < hits[i] {
                        hits[i] = temp;
                    }
                    if powrem <= 0.0 {
                        over = 0.0;
                    }
                    extra += over;
                }
                if powrem > 0.0 {
                    extra += powrem;
                }
                hittem(state, io, &hits).await;
            }
            if extra > 0.0 && state.a.alldone == 0 {
                if state.a.ithere != 0 {
                    state.proutn("*** Tholian web absorbs ");
                    if state.a.nenhere > 0 {
                        state.proutn("excess ");
                    }
                    prout(state, io, "phaser energy.").await;
                } else {
                    state.cramf(extra, 0, 2);
                    prout(state, io, " expended on empty space.").await;
                }
            }
        }
        AutoMode::ForceMan | AutoMode::Manual => {
            if automode == AutoMode::ForceMan {
                state.chew();
                key = ScanToken::Eol;
                if state.damage[DCOMPTR] != 0.0 {
                    prout(state, io, "Battle computer damaged, manual file only.").await;
                } else {
                    skip(state, io, 1).await;
                    prouts(state, "---WORKING---");
                    skip(state, io, 1).await;
                    prout(state, io, "Short-range-sensors-damaged").await;
                    prout(
                        state,
                        io,
                        "Insufficient-data-for-automatic-phaser-fire",
                    )
                    .await;
                    prout(state, io, "Manual-fire-must-be-used").await;
                    skip(state, io, 1).await;
                }
            }

            rpow = 0.0;
            let mut k = 1usize;
            while k <= state.a.nenhere as usize {
                let ii = state.kx[k];
                let jj = state.ky[k];
                let ienm = state.quad[ii as usize][jj as usize];
                if msgflag {
                    state.proutn("Energy available= ");
                    state.cramf(
                        state.a.energy - 0.006 - if ifast { 200.0 } else { 0.0 },
                        0,
                        2,
                    );
                    skip(state, io, 1).await;
                    msgflag = false;
                    rpow = 0.0;
                }
                if state.damage[DSRSENS] != 0.0
                    && !((state.a.sectx - ii).abs() < 2 && (state.a.secty - jj).abs() < 2)
                    && (ienm == IHC || ienm == IHS)
                {
                    state.cramen(ienm);
                    prout(
                        state,
                        io,
                        " can't be located without short range scan.",
                    )
                    .await;
                    state.chew();
                    key = ScanToken::Eol;
                    hits[k] = 0.0;
                    k += 1;
                    continue;
                }
                if key == ScanToken::Eol {
                    state.chew();
                    if ipoop && k > kz {
                        let irec = ((state.kpower[k].abs()
                            / (PHASEFAC * 0.9f64.powf(state.kdist[k])))
                            * (1.01 + 0.05 * state.rng.rand_f64())
                            + 1.0) as i32;
                        kz = k;
                        state.proutn("(");
                        state.crami(irec, 1);
                        state.proutn(")  ");
                    }
                    state.proutn("units to fire at ");
                    state.crmena(0, ienm, 2, ii, jj);
                    state.proutn("-  ");
                    key = scan(state, io).await;
                }
                if key == ScanToken::Alpha && state.isit("no") {
                    no = true;
                    key = scan(state, io).await;
                    continue;
                }
                if key == ScanToken::Alpha {
                    huh(state, io).await;
                    state.a.ididit = 0;
                    return;
                }
                if key == ScanToken::Eol {
                    if k == 1 {
                        msgflag = true;
                    }
                    continue;
                }
                if state.aaitem < 0.0 {
                    state.a.ididit = 0;
                    state.chew();
                    return;
                }
                hits[k] = state.aaitem;
                rpow += state.aaitem;
                if rpow >= if ifast {
                    state.a.energy - 200.0
                } else {
                    state.a.energy
                } {
                    prout(state, io, "Available energy exceeded -- try again.").await;
                    state.chew();
                    key = ScanToken::Eol;
                    k = 1;
                    msgflag = true;
                    continue;
                }
                key = scan(state, io).await;
                k += 1;
            }
            if rpow == 0.0 {
                state.a.ididit = 0;
                state.chew();
                return;
            }
            if key == ScanToken::Alpha && state.isit("no") {
                no = true;
            }
            state.a.energy -= rpow;
            state.chew();
            if ifast {
                state.a.energy -= 200.0;
                if checkshctrl(state, io, rpow).await {
                    return;
                }
            }
            hittem(state, io, &hits).await;
            state.a.ididit = 1;
        }
        AutoMode::NotSet => {}
    }

    if state.a.alldone != 0 {
        return;
    }
    if ifast {
        skip(state, io, 1).await;
        if !no {
            if state.rng.rand_f64() >= 0.99 {
                prout(
                    state,
                    io,
                    "Sulu-  \"Sir, the high-speed shield control has malfunctioned . . .",
                )
                .await;
                prouts(state, "         CLICK   CLICK   POP  . . .");
                prout(state, io, " No  response, sir!").await;
                state.a.shldup = 0;
            } else {
                prout(state, io, "Shields raised.").await;
            }
        } else {
            state.a.shldup = 0;
        }
    }
    overheat(state, io, rpow).await;
}

pub async fn hittem(state: &mut GameState, io: &IoHandle, hits: &[f64; 21]) {
    let nenhr2 = state.a.nenhere as usize;
    let mut kk = 1usize;

    skip(state, io, 1).await;

    for k in 1..=nenhr2 {
        let wham = hits[k];
        if wham == 0.0 {
            kk += 1;
            continue;
        }
        let dustfac = 0.9 + 0.01 * state.rng.rand_f64();
        let hit = wham * dustfac.powf(state.kdist[kk]);
        let kpini = state.kpower[kk];
        let mut kp = kpini.abs();
        if PHASEFAC * hit < kp {
            kp = PHASEFAC * hit;
        }
        state.kpower[kk] -= if state.kpower[kk] < 0.0 { -kp } else { kp };
        let kpow = state.kpower[kk];
        let ii = state.kx[kk];
        let jj = state.ky[kk];
        if hit > 0.005 {
            state.cramf(hit, 0, 2);
            state.proutn(" unit hit on ");
        } else {
            state.proutn("Very small hit on ");
        }
        let ienm = state.quad[ii as usize][jj as usize];
        state.crmena(0, ienm, 2, ii, jj);
        skip(state, io, 1).await;
        if kpow == 0.0 {
            deadkl(state, io, ii, jj, ienm, ii, jj).await;
            if state.d.remkl == 0 {
                finish(state, io, FinType::Won).await;
            }
            if state.a.alldone != 0 {
                return;
            }
            // Don't increment kk because deadkl shifted array left
        } else {
            if kpow > 0.0
                && state.rng.rand_f64() >= 0.9
                && kpow <= ((0.4 + 0.4 * state.rng.rand_f64()) * kpini)
            {
                state.proutn("***Mr. Spock-  \"Captain, the vessel at");
                state.cramlc(2, ii, jj);
                skip(state, io, 1).await;
                prout(state, io, "   has just lost its firepower.\"").await;
                state.kpower[kk] = -kpow;
            }
            kk += 1;
        }
    }
}

pub async fn capture(state: &mut GameState, io: &IoHandle) {
    state.a.ididit = 0;
    state.a.time = 0.0;

    if state.d.brigfree == 0 {
        state.proutn("Security reports the brig is already full.\n");
        return;
    }
    if !state.reports() {
        state.proutn("Uhura- \"We have no subspace radio communication, sir.\"\n");
        return;
    }
    if state.damage[DTRANSP] != 0.0 {
        state.proutn("Scotty- \"Transporter damaged, sir.\"\n");
        return;
    }
    if state.a.klhere < 1 {
        state.proutn("Uhura- \"Getting no response, sir.\"\n");
        return;
    }

    let k = selectklingon(state);
    state.a.time = 0.05;
    state.a.ididit = 1;

    let mut x = state.a.energy;
    let kpow = state.kpower[k].abs().max(1.0);
    x /= kpow * (state.a.nenhere as f64);
    x *= 2.5;
    let prob = x as i32;
    if state.debug_mode {
        let msg = format!("Prob = {} ({:.4})\n", prob, x);
        state.proutn(&msg);
    }
    if (prob as f64) > 100.0 * state.rng.rand_f64() {
        let msg = format!(
            "Klingon captain at {},{} surrenders\n",
            state.kx[k], state.ky[k]
        );
        state.proutn(&msg);
        let mut cap = (200.0 * state.rng.rand_f64()) as i32;
        if cap > 0 {
            let msg = format!(
                "{} Klingons commit suicide rather than be taken captive\n",
                200 - cap
            );
            state.proutn(&msg);
        }
        if cap > state.d.brigfree && state.a.condit != IHDOCKED {
            let msg = format!(
                "{} Klingons die because there is no room for them in the brig.\n",
                cap - state.d.brigfree
            );
            state.proutn(&msg);
            cap = state.d.brigfree;
        }
        if state.a.condit != IHDOCKED {
            state.d.brigfree -= cap;
            let msg = format!("{} captives taken\n", cap);
            state.proutn(&msg);
        } else {
            state.d.kcaptured += cap;
            let msg = format!("{} captives taken and transferred to base\n", cap);
            state.proutn(&msg);
        }
        let kx = state.kx[k];
        let ky = state.ky[k];
        let typ = state.quad[kx as usize][ky as usize];
        deadkl(state, io, kx, ky, typ, kx, ky).await;
        if state.d.remkl == 0 {
            finish(state, io, FinType::Won).await;
        }
        return;
    }

    state.proutn("Fat chance, captain\n");
}

pub fn selectklingon(state: &GameState) -> usize {
    if state.a.nenhere < 2 {
        1
    } else {
        let mut pow = 1e6f64;
        let mut best = 1usize;
        for j in 1..=(state.a.nenhere as usize) {
            if state.quad[state.kx[j] as usize][state.ky[j] as usize] == IHR {
                continue;
            }
            let kp = state.kpower[j].abs();
            if kp < pow {
                pow = kp;
                best = j;
            }
        }
        best
    }
}
