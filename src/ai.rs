//! Enemy AI: Commander/Super-commander movement, galaxy-level SC strategy, and Tholian web.

use crate::battle::ram;
use crate::io::{pause, prout, skip, IoHandle};
use crate::scanner::ja;
use crate::state::{
    square, GameState, Planet, DLRSENS, DPHASER, DPHOTON, DSRSENS, FCDBAS, FSCDBAS, FSCMOVE,
    IHBLANK, IHC, IHDOCKED, IHDOT, IHK, IHR, IHS, IHT, IHWEB, SGOOD,
};

async fn tryexit(
    state: &mut GameState,
    io: &IoHandle,
    lookx: i32,
    looky: i32,
    ienm: char,
    loccom: usize,
    irun: i32,
) -> bool {
    let iqx = state.a.quadx + (lookx + 9) / 10 - 1;
    let iqy = state.a.quady + (looky + 9) / 10 - 1;
    if iqx < 1
        || iqx > 8
        || iqy < 1
        || iqy > 8
        || state.d.galaxy[iqx as usize][iqy as usize] > 899
    {
        return false;
    }
    if ienm == IHR {
        return false;
    }
    if irun == 0 {
        if ienm == IHC {
            for l in 1..=(state.d.remcom as usize) {
                if state.d.cx[l] == iqx && state.d.cy[l] == iqy {
                    return false;
                }
            }
            if state.a.batx == state.a.quadx && state.a.baty == state.a.quady {
                return false;
            }
        }
        if state.kpower[loccom] > 1000.0 {
            return false;
        }
    }
    if state.damage[DSRSENS] == 0.0
        || state.damage[DLRSENS] == 0.0
        || state.a.condit == IHDOCKED
    {
        state.proutn("***");
        state.cramen(ienm);
        state.proutn(" escapes to");
        state.cramlc(1, iqx, iqy);
        prout(state, io, " (and regains strength).").await;
    }
    let nh = state.a.nenhere as usize;
    state.kx[loccom] = state.kx[nh];
    state.ky[loccom] = state.ky[nh];
    state.kavgd[loccom] = state.kavgd[nh];
    state.kpower[loccom] = state.kpower[nh];
    state.kdist[loccom] = state.kdist[nh];
    state.a.klhere -= 1;
    state.a.nenhere -= 1;
    if state.a.condit != IHDOCKED {
        state.newcnd();
    }
    state.d.galaxy[state.a.quadx as usize][state.a.quady as usize] -= 100;
    state.d.galaxy[iqx as usize][iqy as usize] += 100;
    if ienm == IHS {
        state.a.ishere = 0;
        state.a.iscate = 0;
        state.a.ientesc = 0;
        state.a.isatb = 0;
        state.future[FSCMOVE] = 0.2777 + state.d.date;
        state.future[FSCDBAS] = 1e30;
        state.d.isx = iqx;
        state.d.isy = iqy;
    } else if ienm == IHC {
        for l in 1..=(state.d.remcom as usize) {
            if state.d.cx[l] == state.a.quadx && state.d.cy[l] == state.a.quady {
                state.d.cx[l] = iqx;
                state.d.cy[l] = iqy;
                break;
            }
        }
        state.a.comhere = 0;
    }
    true
}

async fn movebaddy(
    state: &mut GameState,
    io: &IoHandle,
    comx: i32,
    comy: i32,
    loccom: usize,
    ienm: char,
) {
    let mut motion: i32;
    let mut irun = 0i32;
    let nbaddys = if state.a.skill > SGOOD {
        ((state.a.comhere * 2 + state.a.ishere * 2) as f64
            + (state.a.klhere as f64) * 1.23
            + (state.a.irhere as f64) * 1.5)
            / 2.0
    } else {
        (state.a.comhere + state.a.ishere) as f64
    } as i32;

    let dist1 = state.kdist[loccom];
    let mdist = (dist1 + 0.5) as i32;

    if ienm == IHS
        && (state.kpower[loccom] <= 500.0
            || (state.a.condit == IHDOCKED && state.damage[DPHOTON] == 0.0))
    {
        irun = 1;
        motion = -10;
    } else {
        let mut forces = state.kpower[loccom]
            + 100.0 * (state.a.nenhere as f64)
            + 400.0 * ((nbaddys - 1) as f64);
        if state.a.shldup == 0 {
            forces += 1000.0;
        }
        if state.damage[DPHASER] == 0.0 || state.damage[DPHOTON] == 0.0 {
            if state.damage[DPHASER] != 0.0 {
                forces += 300.0;
            } else {
                forces -= 0.2 * (state.a.energy - 2500.0);
            }
            if state.damage[DPHOTON] != 0.0 {
                forces += 300.0;
            } else {
                forces -= 50.0 * (state.a.torps as f64);
            }
        } else {
            forces += 1000.0;
        }
        motion = 0;
        if forces <= 1000.0 && state.a.condit != IHDOCKED {
            motion = (((forces + 200.0 * state.rng.rand_f64()) / 150.0) - 5.0) as i32;
        } else {
            if forces > 1000.0 {
                motion = ((1.0 - square(state.rng.rand_f64())) * dist1 + 1.0) as i32;
            }
            if state.a.condit == IHDOCKED {
                motion = (motion as f64
                    - (state.a.skill as f64) * (2.0 - square(state.rng.rand_f64())))
                    as i32;
            }
        }
        if state.debug_mode {
            state.proutn("MOTION = ");
            state.cramf(motion as f64, 1, 2);
            state.proutn("  FORCES = ");
            state.cramf(forces, 1, 2);
            skip(state, io, 1).await;
        }
        if motion == 0 {
            return;
        }
        if motion.abs() > state.a.skill {
            motion = if motion < 0 {
                -state.a.skill
            } else {
                state.a.skill
            };
        }
    }

    let mut nsteps = motion.abs();
    if motion > 0 && nsteps > mdist {
        nsteps = mdist;
    }
    if nsteps > 10 {
        nsteps = 10;
    }
    if nsteps < 1 {
        nsteps = 1;
    }
    if state.debug_mode {
        state.proutn("NSTEPS = ");
        state.crami(nsteps, 1);
        skip(state, io, 1).await;
    }

    let mut mx = state.a.sectx - comx;
    let mut my = state.a.secty - comy;
    if 2.0 * (mx.abs() as f64) < (my.abs() as f64) {
        mx = 0;
    }
    if 2.0 * (my.abs() as f64) < ((state.a.sectx - comx).abs() as f64) {
        my = 0;
    }
    if mx != 0 {
        mx = if mx * motion < 0 { -1 } else { 1 };
    }
    if my != 0 {
        my = if my * motion < 0 { -1 } else { 1 };
    }
    let mut nextx = comx;
    let mut nexty = comy;
    state.quad[comx as usize][comy as usize] = IHDOT;

    for ll in 1..=nsteps {
        if state.debug_mode {
            state.crami(ll, 2);
            skip(state, io, 1).await;
        }
        let mut lookx = nextx + mx;
        let mut looky = nexty + my;
        let mut krawlx = if mx < 0 { 1 } else { -1 };
        let mut krawly = if my < 0 { 1 } else { -1 };
        let mut success = false;
        let mut attempts = 0;
        while attempts < 20 && !success {
            attempts += 1;
            if !(1..=10).contains(&lookx) {
                if motion < 0 && tryexit(state, io, lookx, looky, ienm, loccom, irun).await {
                    return;
                }
                if krawlx == mx || my == 0 {
                    break;
                }
                lookx = nextx + krawlx;
                krawlx = -krawlx;
            } else if !(1..=10).contains(&looky) {
                if motion < 0 && tryexit(state, io, lookx, looky, ienm, loccom, irun).await {
                    return;
                }
                if krawly == my || mx == 0 {
                    break;
                }
                looky = nexty + krawly;
                krawly = -krawly;
            } else if state.quad[lookx as usize][looky as usize] != IHDOT {
                if state.quad[lookx as usize][looky as usize] == state.a.ship
                    && (ienm == IHC || ienm == IHS)
                {
                    ram(state, io, true, ienm, comx, comy).await;
                    return;
                }
                if krawlx != mx && my != 0 {
                    lookx = nextx + krawlx;
                    krawlx = -krawlx;
                } else if krawly != my && mx != 0 {
                    looky = nexty + krawly;
                    krawly = -krawly;
                } else {
                    break;
                }
            } else {
                success = true;
            }
        }
        if success {
            nextx = lookx;
            nexty = looky;
            if state.debug_mode {
                state.cramlc(0, nextx, nexty);
                skip(state, io, 1).await;
            }
        } else {
            break;
        }
    }

    state.quad[nextx as usize][nexty as usize] = ienm;
    if nextx != comx || nexty != comy {
        state.kx[loccom] = nextx;
        state.ky[loccom] = nexty;
        let new_dist =
            (square((state.a.sectx - nextx) as f64) + square((state.a.secty - nexty) as f64))
                .sqrt();
        state.kdist[loccom] = new_dist;
        state.kavgd[loccom] = new_dist;
        if state.damage[DSRSENS] == 0.0 || state.a.condit == IHDOCKED {
            state.proutn("***");
            state.cramen(ienm);
            if state.kdist[loccom] < dist1 {
                state.proutn(" advances to");
            } else {
                state.proutn(" retreats to");
            }
            state.cramlc(2, nextx, nexty);
            skip(state, io, 1).await;
        }
    }
}

pub async fn movcom(state: &mut GameState, io: &IoHandle) {
    if state.debug_mode {
        prout(state, io, "MOVCOM").await;
    }

    if state.a.comhere != 0 {
        for i in 1..=(state.a.nenhere as usize) {
            let ix = state.kx[i];
            let iy = state.ky[i];
            if state.quad[ix as usize][iy as usize] == IHC {
                movebaddy(state, io, ix, iy, i, IHC).await;
                break;
            }
        }
    }
    if state.a.ishere != 0 {
        for i in 1..=(state.a.nenhere as usize) {
            let ix = state.kx[i];
            let iy = state.ky[i];
            if state.quad[ix as usize][iy as usize] == IHS {
                movebaddy(state, io, ix, iy, i, IHS).await;
                break;
            }
        }
    }
    if state.a.skill > SGOOD {
        let mut i = 1usize;
        while i <= state.a.nenhere as usize {
            let ix = state.kx[i];
            let iy = state.ky[i];
            let ch = state.quad[ix as usize][iy as usize];
            let prev_nh = state.a.nenhere;
            if ch == IHK || ch == IHR {
                movebaddy(state, io, ix, iy, i, ch).await;
                if state.a.alldone != 0 {
                    return;
                }
            }
            if state.a.nenhere == prev_nh {
                i += 1;
            }
        }
    }

    state.sortkl();
}

async fn checkdest(
    state: &mut GameState,
    io: &IoHandle,
    iqx: i32,
    iqy: i32,
    flag: bool,
    ipage: &mut i32,
) -> bool {
    if (iqx == state.a.quadx && iqy == state.a.quady)
        || iqx < 1
        || iqx > 8
        || iqy < 1
        || iqy > 8
        || state.d.galaxy[iqx as usize][iqy as usize] > 899
    {
        return true;
    }
    if flag {
        for i in 1..=(state.d.rembase as usize) {
            if state.d.baseqx[i] == iqx && state.d.baseqy[i] == iqy {
                return true;
            }
        }
    }

    state.d.galaxy[state.d.isx as usize][state.d.isy as usize] -= 100;
    state.d.isx = iqx;
    state.d.isy = iqy;
    state.d.galaxy[state.d.isx as usize][state.d.isy as usize] += 100;

    if state.a.iscate != 0 {
        state.a.iscate = 0;
        state.a.isatb = 0;
        state.a.ishere = 0;
        state.a.ientesc = 0;
        state.future[FSCDBAS] = 1e30;
        let nh = state.a.nenhere as usize;
        let mut found = 1usize;
        for i in 1..=nh {
            if state.quad[state.kx[i] as usize][state.ky[i] as usize] == IHS {
                found = i;
                break;
            }
        }
        state.quad[state.kx[found] as usize][state.ky[found] as usize] = IHDOT;
        state.kx[found] = state.kx[nh];
        state.ky[found] = state.ky[nh];
        state.kdist[found] = state.kdist[nh];
        state.kavgd[found] = state.kavgd[nh];
        state.kpower[found] = state.kpower[nh];
        state.a.klhere -= 1;
        state.a.nenhere -= 1;
        if state.a.condit != IHDOCKED {
            state.newcnd();
        }
        state.sortkl();
    }

    for i in 1..=(state.a.inplan as usize) {
        if state.d.plnets[i].x == state.d.isx
            && state.d.plnets[i].y == state.d.isy
            && state.d.plnets[i].crystals == 1
        {
            state.d.plnets[i] = Planet::default();
            state.d.newstuf[state.d.isx as usize][state.d.isy as usize] -= 1;
            if state.reports() {
                if *ipage == 0 {
                    pause(state, io, 1).await;
                }
                *ipage = 1;
                prout(
                    state,
                    io,
                    "Lt. Uhura-  \"Captain, Starfleet Intelligence reports",
                )
                .await;
                state.proutn("   a planet in");
                state.cramlc(1, state.d.isx, state.d.isy);
                prout(state, io, " has been destroyed").await;
                prout(state, io, "   by the Super-commander.\"").await;
            }
            break;
        }
    }
    false
}

pub async fn scom(state: &mut GameState, io: &IoHandle, ipage: &mut i32) {
    let mut basetbl = [0usize; 6];
    let mut bdist = [0.0f64; 6];

    if state.debug_mode {
        prout(state, io, "SCOM").await;
    }

    let sk = state.a.skill as f64;
    let flag = (((state.d.killc + state.d.killk) as f64)
        / (state.d.date + 0.01 - state.a.indate)
        < 0.1 * sk * (sk + 1.0))
        || ((state.d.date - state.a.indate) < 3.0);

    let mut ideltax: i32;
    let mut ideltay: i32;

    if state.a.iscate == 0 && flag {
        ideltax = state.d.isx - state.a.quadx;
        ideltay = state.d.isy - state.a.quady;
        if (((ideltax as f64) * (ideltax as f64) + (ideltay as f64) * (ideltay as f64)).sqrt())
            > 2.0
        {
            ideltax = state.d.isy - state.a.quady;
            ideltay = state.a.quadx - state.d.isx;
        }
    } else {
        if state.d.rembase <= 0 {
            state.future[FSCMOVE] = 1e30;
            return;
        }
        let sx = state.d.isx;
        let sy = state.d.isy;
        let remb = state.d.rembase as usize;
        for i in 1..=remb {
            basetbl[i] = i;
            let ibqx = state.d.baseqx[i];
            let ibqy = state.d.baseqy[i];
            bdist[i] = (square((ibqx - sx) as f64) + square((ibqy - sy) as f64)).sqrt();
        }
        if remb > 1 {
            loop {
                let mut iswitch = false;
                for i in 1..remb {
                    if bdist[i] > bdist[i + 1] {
                        bdist.swap(i, i + 1);
                        basetbl.swap(i, i + 1);
                        iswitch = true;
                    }
                }
                if !iswitch {
                    break;
                }
            }
        }
        let mut ifindit = 0;
        let mut iwhichb = 0usize;
        for i2 in 1..=remb {
            let i = basetbl[i2];
            let ibqx = state.d.baseqx[i];
            let ibqy = state.d.baseqy[i];
            if (ibqx == state.a.quadx && ibqy == state.a.quady)
                || (ibqx == state.a.batx && ibqy == state.a.baty)
                || state.d.galaxy[ibqx as usize][ibqy as usize] > 899
            {
                continue;
            }
            let mut matched_com = false;
            for j in 1..=(state.d.remcom as usize) {
                if ibqx == state.d.cx[j] && ibqy == state.d.cy[j] && ifindit != 2 {
                    ifindit = 2;
                    iwhichb = i;
                    matched_com = true;
                    break;
                }
            }
            if !matched_com {
                ifindit = 1;
                iwhichb = i;
                break;
            }
        }
        if ifindit == 0 {
            return;
        }
        let ibqx = state.d.baseqx[iwhichb];
        let ibqy = state.d.baseqy[iwhichb];
        ideltax = ibqx - state.d.isx;
        ideltay = ibqy - state.d.isy;
    }

    ideltax = ideltax.clamp(-1, 1);
    ideltay = ideltay.clamp(-1, 1);

    let mut iqx = state.d.isx + ideltax;
    let mut iqy = state.d.isy + ideltay;
    if checkdest(state, io, iqx, iqy, flag, ipage).await {
        if ideltax == 0 || ideltay == 0 {
            if ideltax != 0 {
                iqy = state.d.isy + 1;
                if checkdest(state, io, iqx, iqy, flag, ipage).await {
                    iqy = state.d.isy - 1;
                    checkdest(state, io, iqx, iqy, flag, ipage).await;
                }
            } else {
                iqx = state.d.isx + 1;
                if checkdest(state, io, iqx, iqy, flag, ipage).await {
                    iqx = state.d.isx - 1;
                    checkdest(state, io, iqx, iqy, flag, ipage).await;
                }
            }
        } else {
            iqy = state.d.isy;
            if checkdest(state, io, iqx, iqy, flag, ipage).await {
                iqy = state.d.isy + ideltay;
                iqx = state.d.isx;
                checkdest(state, io, iqx, iqy, flag, ipage).await;
            }
        }
    }

    if state.d.rembase == 0 {
        state.future[FSCMOVE] = 1e30;
    } else {
        for i in 1..=(state.d.rembase as usize) {
            let ibqx = state.d.baseqx[i];
            let ibqy = state.d.baseqy[i];
            if ibqx == state.d.isx
                && ibqy == state.d.isy
                && (state.d.isx != state.a.batx || state.d.isy != state.a.baty)
            {
                if flag {
                    return;
                }
                state.a.iseenit = 0;
                state.a.isatb = 1;
                state.future[FSCDBAS] = state.d.date + 1.0 + 2.0 * state.rng.rand_f64();
                if state.a.batx != 0 {
                    state.future[FSCDBAS] += state.future[FCDBAS] - state.d.date;
                }
                if !state.reports() {
                    return;
                }
                state.a.iseenit = 1;
                if *ipage == 0 {
                    pause(state, io, 1).await;
                }
                *ipage = 1;
                state.proutn("Lt. Uhura-  \"Captain, the starbase in");
                state.cramlc(1, state.d.isx, state.d.isy);
                skip(state, io, 1).await;
                prout(
                    state,
                    io,
                    "   reports that it is under attack from the Klingon Super-commander.",
                )
                .await;
                state.proutn("   It can survive until stardate ");
                state.cramf(state.future[FSCDBAS], 0, 1);
                prout(state, io, " .\"").await;
                if state.a.resting == 0 {
                    return;
                }
                prout(
                    state,
                    io,
                    "Mr. Spock-  \"Captain, shall we cancel the rest period?\"",
                )
                .await;
                if !ja(state, io).await {
                    return;
                }
                state.a.resting = 0;
                state.a.time = 0.0;
                return;
            }
        }
    }

    if (!state.debug_mode)
        && (state.rng.rand_f64() > 0.2
            || !state.reports()
            || state.starch[state.d.isx as usize][state.d.isy as usize] > 0)
    {
        return;
    }
    if *ipage == 0 {
        pause(state, io, 1).await;
    }
    *ipage = 1;
    prout(
        state,
        io,
        "Lt. Uhura-  \"Captain, Starfleet Intelligence reports",
    )
    .await;
    state.proutn("   the Super-commander is in");
    state.cramlc(1, state.d.isx, state.d.isy);
    prout(state, io, ".\"").await;
}

pub async fn movetho(state: &mut GameState, io: &IoHandle) {
    if state.a.ithere == 0 || state.a.justin == 1 {
        return;
    }

    let (idx, idy) = match (state.a.ithx, state.a.ithy) {
        (1, 1) => (1, 10),
        (1, 10) => (10, 10),
        (10, 10) => (10, 1),
        (10, 1) => (1, 1),
        _ => {
            state.a.ithere = 0;
            return;
        }
    };

    let dest = state.quad[idx as usize][idy as usize];
    if dest != IHDOT && dest != IHWEB {
        return;
    }
    state.quad[state.a.ithx as usize][state.a.ithy as usize] = IHWEB;

    if state.a.ithx != idx {
        let im = if idx > state.a.ithx { 1 } else { -1 };
        while state.a.ithx != idx {
            state.a.ithx += im;
            if state.quad[state.a.ithx as usize][state.a.ithy as usize] == IHDOT {
                state.quad[state.a.ithx as usize][state.a.ithy as usize] = IHWEB;
            }
        }
    } else if state.a.ithy != idy {
        let im = if idy > state.a.ithy { 1 } else { -1 };
        while state.a.ithy != idy {
            state.a.ithy += im;
            if state.quad[state.a.ithx as usize][state.a.ithy as usize] == IHDOT {
                state.quad[state.a.ithx as usize][state.a.ithy as usize] = IHWEB;
            }
        }
    }
    state.quad[state.a.ithx as usize][state.a.ithy as usize] = IHT;

    for i in 1..=10 {
        if state.quad[1][i] != IHWEB && state.quad[1][i] != IHT {
            return;
        }
        if state.quad[10][i] != IHWEB && state.quad[10][i] != IHT {
            return;
        }
        if state.quad[i][1] != IHWEB && state.quad[i][1] != IHT {
            return;
        }
        if state.quad[i][10] != IHWEB && state.quad[i][10] != IHT {
            return;
        }
    }

    state.quad[state.a.ithx as usize][state.a.ithy as usize] = IHWEB;
    state.dropin(IHBLANK);
    state.crmena(1, IHT, 2, state.a.ithx, state.a.ithy);
    prout(state, io, " completes web.").await;
    state.a.ithere = 0;
    state.a.ithx = 0;
    state.a.ithy = 0;
}
