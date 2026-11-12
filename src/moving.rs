//! Starship movement (warp, impulse, tractor beam, supernova override, time warp, probe, help).

use crate::battle::attack;
use crate::finish::finish;
use crate::io::{huh, prout, prouts, skip, IoHandle};
use crate::reports::attakreport;
use crate::scanner::{ja, scan, ScanToken};
use crate::setup::newqad;
use crate::state::{
    square, FinType, GameState, DCOMPTR, DIMPULS, DRADIO, DTRANSP, DWARPEN, DDSP,
    FBATTAK, FCDBAS, FDSPROB, FSNAP, FSCDBAS, FSCMOVE, FSNOVA, FTBEAM, IHBLANK, IHC, IHDOCKED,
    IHDOT, IHK, IHR, IHS, IHT, IHWEB,
};

pub async fn lmove(state: &mut GameState, io: &IoHandle) {
    let mut trbeam = false;

    if state.a.inorbit != 0 {
        prout(state, io, "SULU- \"Leaving standard orbit.\"").await;
        state.a.inorbit = 0;
    }

    let angle = (15.0 - state.a.direc) * 0.5235988;
    let mut deltax = -angle.sin();
    let mut deltay = angle.cos();
    let bigger = deltax.abs().max(deltay.abs());
    deltay /= bigger;
    deltax /= bigger;

    if state.a.iscloaked && state.d.date + state.a.time >= state.future[FTBEAM] {
        state.future[FTBEAM] = state.d.date
            + state.a.time
            + state.rng.expran(1.5 * state.a.intime / (state.d.remcom as f64));
    }

    if state.d.date + state.a.time >= state.future[FTBEAM] {
        trbeam = true;
        state.a.condit = crate::state::IHRED;
        state.a.dist =
            state.a.dist * (state.future[FTBEAM] - state.d.date) / state.a.time + 0.1;
        state.a.time = state.future[FTBEAM] - state.d.date + 1e-5;
    }

    state.quad[state.a.sectx as usize][state.a.secty as usize] = IHDOT;
    let mut x = state.a.sectx as f64;
    let mut y = state.a.secty as f64;
    let n = (10.0 * state.a.dist * bigger + 0.5) as i32;
    let mut ix = state.a.sectx;
    let mut iy = state.a.secty;

    if n > 0 {
        for _l in 1..=n {
            x += deltax;
            y += deltay;
            ix = (x + 0.5) as i32;
            iy = (y + 0.5) as i32;
            if !(1..=10).contains(&ix) || !(1..=10).contains(&iy) {
                if state.a.nenhere != 0 && state.a.iattak != 2 && !state.a.iscloaked {
                    state.newcnd();
                    for idx in 1..=(state.a.nenhere as usize) {
                        let finald = (((ix - state.kx[idx]) as f64).powi(2)
                            + ((iy - state.ky[idx]) as f64).powi(2))
                        .sqrt();
                        state.kavgd[idx] = 0.5 * (finald + state.kdist[idx]);
                    }
                    if state.d.galaxy[state.a.quadx as usize][state.a.quady as usize] != 1000 {
                        attack(state, io, 0).await;
                    }
                    if state.a.alldone != 0 {
                        return;
                    }
                }

                let gx = (10 * (state.a.quadx - 1) + state.a.sectx) as f64;
                let gy = (10 * (state.a.quady - 1) + state.a.secty) as f64;
                let mut gix = (gx + 10.0 * state.a.dist * bigger * deltax + 0.5) as i32;
                let mut giy = (gy + 10.0 * state.a.dist * bigger * deltay + 0.5) as i32;

                let mut kinks = false;
                loop {
                    let mut kink = false;
                    if gix <= 0 {
                        gix = -gix + 1;
                        kink = true;
                    }
                    if giy <= 0 {
                        giy = -giy + 1;
                        kink = true;
                    }
                    if gix > 80 {
                        gix = 161 - gix;
                        kink = true;
                    }
                    if giy > 80 {
                        giy = 161 - giy;
                        kink = true;
                    }
                    if kink {
                        kinks = true;
                    } else {
                        break;
                    }
                }

                if kinks {
                    state.a.nkinks += 1;
                    if state.a.nkinks == 3 {
                        finish(state, io, FinType::Neg3).await;
                        return;
                    }
                    prout(
                        state,
                        io,
                        "\nYOU HAVE ATTEMPTED TO CROSS THE NEGATIVE ENERGY BARRIER\nAT THE EDGE OF THE GALAXY.  THE THIRD TIME YOU TRY THIS,\nYOU WILL BE DESTROYED.\n",
                    )
                    .await;
                }

                if trbeam {
                    return;
                }
                let oldquadx = state.a.quadx;
                let oldquady = state.a.quady;
                state.a.quadx = (gix + 9) / 10;
                state.a.quady = (giy + 9) / 10;
                state.a.sectx = gix - 10 * (state.a.quadx - 1);
                state.a.secty = giy - 10 * (state.a.quady - 1);
                if state.a.quadx != oldquadx || state.a.quady != oldquady {
                    state.proutn("\nEntering");
                    state.cramlc(1, state.a.quadx, state.a.quady);
                } else {
                    prout(state, io, "(Negative energy barrier disturbs quadrant.)").await;
                }
                skip(state, io, 1).await;
                state.quad[state.a.sectx as usize][state.a.secty as usize] = state.a.ship;
                newqad(state, io, false).await;
                return;
            }

            let iquad = state.quad[ix as usize][iy as usize];
            if iquad != IHDOT {
                let stopegy = 50.0 * state.a.dist / state.a.time;
                state.a.dist = 0.1
                    * (((state.a.sectx - ix) as f64).powi(2)
                        + ((state.a.secty - iy) as f64).powi(2))
                    .sqrt();
                match iquad {
                    IHT | IHK | IHC | IHS | IHR => {
                        state.a.sectx = ix;
                        state.a.secty = iy;
                        crate::battle::ram(state, io, false, iquad, state.a.sectx, state.a.secty)
                            .await;
                    }
                    IHBLANK => {
                        skip(state, io, 1).await;
                        prouts(state, "***RED ALERT!  RED ALERT!");
                        skip(state, io, 1).await;
                        state.proutn("***");
                        state.crmshp();
                        state.proutn(" pulled into black hole at");
                        state.cramlc(2, ix, iy);
                        skip(state, io, 1).await;
                        finish(state, io, FinType::Hole).await;
                        return;
                    }
                    _ => {
                        skip(state, io, 1).await;
                        state.crmshp();
                        if iquad == IHWEB {
                            state.proutn(" encounters Tholian web at");
                        } else {
                            state.proutn(" blocked by object at");
                        }
                        state.cramlc(2, ix, iy);
                        prout(state, io, ";").await;
                        state.proutn("Emergency stop required ");
                        state.cramf(stopegy, 0, 2);
                        prout(state, io, " units of energy.").await;
                        state.a.energy -= stopegy;
                        state.a.sectx = (x - deltax + 0.5) as i32;
                        state.a.secty = (y - deltay + 0.5) as i32;
                        if state.a.energy <= 0.0 {
                            finish(state, io, FinType::Nrg).await;
                            return;
                        }
                    }
                }
                // Post-collision label100
                state.quad[state.a.sectx as usize][state.a.secty as usize] = state.a.ship;
                if state.a.nenhere != 0 {
                    for l in 1..=(state.a.nenhere as usize) {
                        let finald = (((ix - state.kx[l]) as f64).powi(2)
                            + ((iy - state.ky[l]) as f64).powi(2))
                        .sqrt();
                        state.kavgd[l] = 0.5 * (finald + state.kdist[l]);
                        state.kdist[l] = finald;
                    }
                    state.sortkl();
                    if state.d.galaxy[state.a.quadx as usize][state.a.quady as usize] != 1000
                        && state.a.iattak == 0
                    {
                        attack(state, io, 0).await;
                    }
                    for l in 1..=(state.a.nenhere as usize) {
                        state.kavgd[l] = state.kdist[l];
                    }
                }
                state.newcnd();
                state.a.iattak = 0;
                return;
            }
        }
        state.a.dist = 0.1
            * (((state.a.sectx - ix) as f64).powi(2) + ((state.a.secty - iy) as f64).powi(2))
                .sqrt();
        state.a.sectx = ix;
        state.a.secty = iy;
    }

    state.quad[state.a.sectx as usize][state.a.secty as usize] = state.a.ship;
    if state.a.nenhere != 0 {
        for l in 1..=(state.a.nenhere as usize) {
            let finald = (((ix - state.kx[l]) as f64).powi(2)
                + ((iy - state.ky[l]) as f64).powi(2))
            .sqrt();
            state.kavgd[l] = 0.5 * (finald + state.kdist[l]);
            state.kdist[l] = finald;
        }
        state.sortkl();
        if state.d.galaxy[state.a.quadx as usize][state.a.quady as usize] != 1000
            && state.a.iattak == 0
        {
            attack(state, io, 0).await;
        }
        for l in 1..=(state.a.nenhere as usize) {
            state.kavgd[l] = state.kdist[l];
        }
    }
    state.newcnd();
    state.a.iattak = 0;
}

pub async fn dock(state: &mut GameState, io: &IoHandle) {
    state.chew();
    if state.a.condit == IHDOCKED {
        prout(state, io, "Already docked.").await;
        return;
    }
    if state.a.inorbit != 0 {
        prout(state, io, "You must first leave standard orbit.").await;
        return;
    }
    if state.a.basex == 0
        || (state.a.sectx - state.a.basex).abs() > 1
        || (state.a.secty - state.a.basey).abs() > 1
    {
        state.crmshp();
        prout(state, io, " not adjacent to base.").await;
        return;
    }
    if state.a.iscloaked {
        prout(state, io, "You cannot dock while cloaked.").await;
        return;
    }
    state.a.condit = IHDOCKED;
    prout(state, io, "Docked.").await;
    if state.a.energy < state.a.inenrg {
        state.a.energy = state.a.inenrg;
    }
    state.a.shield = state.a.inshld;
    state.a.torps = state.a.intorps;
    state.a.lsupres = state.a.inlsr;
    if state.a.brigcapacity - state.d.brigfree > 0 {
        let cap = state.a.brigcapacity - state.d.brigfree;
        let msg = format!("{} captured Klingons transferred to base.\n", cap);
        state.proutn(&msg);
        state.d.kcaptured += cap;
        state.d.brigfree = state.a.brigcapacity;
    }
    if state.a.stdamtim != 1e30
        && (state.future[FCDBAS] < 1e30 || state.a.isatb == 1)
        && state.a.iseenit == 0
    {
        prout(
            state,
            io,
            "Lt. Uhura- \"Captain, an important message from the starbase:\"",
        )
        .await;
        attakreport(state, io).await;
        state.a.iseenit = 1;
    }
}

async fn getcd(
    state: &mut GameState,
    io: &IoHandle,
    isprobe: bool,
    mut akey: Option<ScanToken>,
) {
    let mut irowq = state.a.quadx;
    let mut icolq = state.a.quady;
    let irows: i32;
    let icols: i32;
    let mut itemp = 0;
    let mut iprompt = false;
    let mut key = ScanToken::Eol;
    let mut automatic: i32 = -1;

    state.a.direc = -1.0;

    if state.a.landed == 1 && !isprobe {
        prout(
            state,
            io,
            "Dummy! You can't leave standard orbit until you",
        )
        .await;
        state.proutn("are back aboard the ");
        state.crmshp();
        prout(state, io, ".").await;
        state.chew();
        return;
    }

    while automatic == -1 {
        if state.damage[DCOMPTR] != 0.0 {
            if isprobe {
                prout(state, io, "Computer damaged; manual navigation only").await;
            } else {
                prout(state, io, "Computer damaged; manual movement only").await;
            }
            state.chew();
            automatic = 0;
            key = ScanToken::Eol;
            break;
        }
        if isprobe {
            if let Some(k) = akey.take() {
                key = k;
            } else {
                key = scan(state, io).await;
            }
        } else {
            key = scan(state, io).await;
        }

        match key {
            ScanToken::Eol => {
                state.proutn("Manual or automatic- ");
                iprompt = true;
                state.chew();
            }
            ScanToken::Alpha => {
                if state.isit("manual") {
                    automatic = 0;
                    key = scan(state, io).await;
                    break;
                } else if state.isit("automatic") {
                    automatic = 1;
                    key = scan(state, io).await;
                    break;
                } else {
                    huh(state, io).await;
                    state.chew();
                    return;
                }
            }
            ScanToken::Real => {
                if isprobe {
                    prout(state, io, "(Manual navigation assumed.)").await;
                } else {
                    prout(state, io, "(Manual movement assumed.)").await;
                }
                automatic = 0;
                break;
            }
        }
    }

    let mut deltax: f64;
    let mut deltay: f64;

    if automatic != 0 {
        while key == ScanToken::Eol {
            if isprobe {
                state.proutn("Target quadrant or quadrant&sector- ");
            } else {
                state.proutn("Destination sector or quadrant&sector- ");
            }
            state.chew();
            iprompt = true;
            key = scan(state, io).await;
        }
        if key != ScanToken::Real {
            huh(state, io).await;
            return;
        }
        let xi = state.aaitem;
        key = scan(state, io).await;
        if key != ScanToken::Real {
            huh(state, io).await;
            return;
        }
        let xj = state.aaitem;
        key = scan(state, io).await;
        if key == ScanToken::Real {
            let xk = state.aaitem;
            key = scan(state, io).await;
            if key != ScanToken::Real {
                huh(state, io).await;
                return;
            }
            let xl = state.aaitem;
            irowq = (xi + 0.5) as i32;
            icolq = (xj + 0.5) as i32;
            irows = (xk + 0.5) as i32;
            icols = (xl + 0.5) as i32;
        } else {
            if isprobe {
                irowq = (xi + 0.5) as i32;
                icolq = (xj + 0.5) as i32;
                irows = 5;
                icols = 5;
            } else {
                irows = (xi + 0.5) as i32;
                icols = (xj + 0.5) as i32;
            }
            itemp = 1;
        }
        if !(1..=8).contains(&irowq)
            || !(1..=8).contains(&icolq)
            || !(1..=10).contains(&irows)
            || !(1..=10).contains(&icols)
        {
            huh(state, io).await;
            return;
        }
        skip(state, io, 1).await;
        if !isprobe {
            if itemp != 0 {
                if iprompt {
                    state.proutn("Helmsman Sulu- \"Course locked in for");
                    state.cramlc(2, irows, icols);
                    prout(state, io, ".\"").await;
                }
            } else {
                prout(
                    state,
                    io,
                    "Ensign Chekov- \"Course laid in, Captain.\"",
                )
                .await;
            }
        }
        deltax = (icolq - state.a.quady) as f64 + 0.1 * ((icols - state.a.secty) as f64);
        deltay = (state.a.quadx - irowq) as f64 + 0.1 * ((state.a.sectx - irows) as f64);
    } else {
        while key == ScanToken::Eol {
            state.proutn("X and Y displacements- ");
            state.chew();
            key = scan(state, io).await;
        }
        itemp = 2;
        if key != ScanToken::Real {
            huh(state, io).await;
            return;
        }
        deltax = state.aaitem;
        key = scan(state, io).await;
        if key == ScanToken::Eol {
            deltay = 0.0;
        } else if key != ScanToken::Real {
            huh(state, io).await;
            return;
        } else {
            deltay = state.aaitem;
        }
        if state.coordfixed {
            let temp = deltax;
            deltax = deltay;
            deltay = -temp;
        }
    }

    if deltax == 0.0 && deltay == 0.0 {
        state.chew();
        return;
    }
    if itemp == 2 && !isprobe {
        skip(state, io, 1).await;
        prout(state, io, "Helmsman Sulu- \"Aye, Sir.\"").await;
    }
    state.a.dist = (deltax * deltax + deltay * deltay).sqrt();
    state.a.direc = deltax.atan2(deltay) * 1.90985932;
    if state.a.direc < 0.0 {
        state.a.direc += 12.0;
    }
    state.chew();
}

pub async fn impuls(state: &mut GameState, io: &IoHandle) {
    state.a.ididit = 0;
    if state.damage[DIMPULS] != 0.0 {
        state.chew();
        skip(state, io, 1).await;
        prout(
            state,
            io,
            "Engineer Scott- \"The impulse engines are damaged, Sir.\"",
        )
        .await;
        return;
    }

    let mut power = if state.a.energy > 30.0 {
        getcd(state, io, false, None).await;
        if state.a.direc == -1.0 {
            return;
        }
        20.0 + 100.0 * state.a.dist
    } else {
        30.0
    };

    if power >= state.a.energy {
        skip(state, io, 1).await;
        prout(
            state,
            io,
            "First Officer Spock- \"Captain, the impulse engines",
        )
        .await;
        prout(
            state,
            io,
            "require 20.0 units to engage, plus 100.0 units per",
        )
        .await;
        if state.a.energy > 30.0 {
            state.proutn("quadrant.  We can go, therefore, a maximum of ");
            state.cramf(0.01 * (state.a.energy - 20.0) - 0.05, 0, 1);
            prout(state, io, " quadrants.\"").await;
        } else {
            prout(state, io, "quadrant.  They are, therefore, useless.\"").await;
        }
        state.chew();
        return;
    }

    state.a.time = state.a.dist / 0.095;
    if state.a.time >= state.d.remtime {
        prout(
            state,
            io,
            "First Officer Spock- \"Captain, our speed under impulse",
        )
        .await;
        prout(
            state,
            io,
            "power is only 0.95 sectors per stardate. Are you sure",
        )
        .await;
        prout(state, io, "we dare spend the time?\"").await;
        if !ja(state, io).await {
            state.a.time = 0.0;
            return;
        }
    }

    lmove(state, io).await;
    state.a.ididit = 1;
    if state.a.alldone != 0 {
        return;
    }
    power = 20.0 + 100.0 * state.a.dist;
    state.a.energy -= power;
    if state.a.energy <= 0.0 {
        finish(state, io, FinType::Nrg).await;
        if state.a.alldone != 0 {
            return;
        }
    }
}

pub async fn warp(state: &mut GameState, io: &IoHandle, i: i32) {
    let mut blooey = false;
    let mut twarp = false;

    if i != 2 {
        state.a.ididit = 0;
        if state.a.iscloaked {
            state.chew();
            skip(state, io, 1).await;
            prout(
                state,
                io,
                "Engineer Scott- \"The warp engines cannot be used while cloaked, Sir.\"",
            )
            .await;
            return;
        }
        if state.damage[DWARPEN] > 10.0 {
            state.chew();
            skip(state, io, 1).await;
            prout(
                state,
                io,
                "Engineer Scott- \"The warp engines are damaged, Sir.\"",
            )
            .await;
            return;
        }
        if state.damage[DWARPEN] > 0.0 && state.a.warpfac > 4.0 {
            state.chew();
            skip(state, io, 1).await;
            prout(
                state,
                io,
                "Engineer Scott- \"Sorry, Captain. Until this damage",
            )
            .await;
            prout(state, io, "  is repaired, I can only give you warp 4.\"").await;
            return;
        }

        getcd(state, io, false, None).await;
        if state.a.direc == -1.0 {
            return;
        }

        let power = (state.a.dist + 0.05)
            * state.a.warpfac
            * state.a.warpfac
            * state.a.warpfac
            * ((state.a.shldup + 1) as f64);

        if power >= state.a.energy {
            state.a.ididit = 0;
            skip(state, io, 1).await;
            prout(state, io, "Engineering to bridge--").await;
            if state.a.shldup == 0 || 0.5 * power > state.a.energy {
                let iwarp = (state.a.energy / (state.a.dist + 0.05)).powf(0.333333333) as i32;
                if iwarp <= 0 {
                    prout(
                        state,
                        io,
                        "We can't do it, Captain. We haven't the energy.",
                    )
                    .await;
                } else {
                    state.proutn("We haven't the energy, but we could do it at warp ");
                    state.crami(iwarp, 1);
                    if state.a.shldup != 0 {
                        prout(state, io, ",\nif you'll lower the shields.").await;
                    } else {
                        prout(state, io, ".").await;
                    }
                }
            } else {
                prout(
                    state,
                    io,
                    "We haven't the energy to go that far with the shields up.",
                )
                .await;
            }
            return;
        }

        state.a.time = 10.0 * state.a.dist / state.a.wfacsq;
        if state.a.time >= 0.8 * state.d.remtime {
            skip(state, io, 1).await;
            prout(
                state,
                io,
                "First Officer Spock- \"Captain, I compute that such",
            )
            .await;
            state.proutn("  a trip would require approximately ");
            state.cramf(100.0 * state.a.time / state.d.remtime, 0, 2);
            prout(state, io, " percent of our").await;
            prout(state, io, "  remaining time.  Are you sure this is wise?\"").await;
            if !ja(state, io).await {
                state.a.time = 0.0;
                return;
            }
        }
    }

    if state.a.warpfac > 6.0 {
        let prob = state.a.dist * (6.0 - state.a.warpfac) * (6.0 - state.a.warpfac) / 66.666666666;
        if prob > state.rng.rand_f64() {
            blooey = true;
            state.a.dist *= state.rng.rand_f64();
        }
        if 0.5 * state.a.dist * 7.0f64.powf(state.a.warpfac - 10.0) > state.rng.rand_f64() {
            twarp = true;
        }
        if state.debug_mode && state.a.warpfac == 10.0 && !twarp {
            blooey = false;
            state.proutn("Force time warp? ");
            if ja(state, io).await {
                twarp = true;
            }
        }
        if blooey || twarp {
            let angle = (15.0 - state.a.direc) * 0.5235998;
            let mut deltax = -angle.sin();
            let mut deltay = angle.cos();
            let bigger = deltax.abs().max(deltay.abs());
            deltax /= bigger;
            deltay /= bigger;
            let n = (10.0 * state.a.dist * bigger + 0.5) as i32;
            let mut x = state.a.sectx as f64;
            let mut y = state.a.secty as f64;
            for _l in 1..=n {
                x += deltax;
                let ix = (x + 0.5) as i32;
                if !(1..=10).contains(&ix) {
                    break;
                }
                y += deltay;
                let iy = (y + 0.5) as i32;
                if !(1..=10).contains(&iy) {
                    break;
                }
                if state.quad[ix as usize][iy as usize] != IHDOT {
                    blooey = false;
                    twarp = false;
                }
            }
        }
    }

    lmove(state, io).await;
    if state.a.alldone != 0 {
        return;
    }
    state.a.energy -= state.a.dist
        * state.a.warpfac
        * state.a.warpfac
        * state.a.warpfac
        * ((state.a.shldup + 1) as f64);
    if state.a.energy <= 0.0 {
        finish(state, io, FinType::Nrg).await;
        if state.a.alldone != 0 {
            return;
        }
    }
    state.a.time = 10.0 * state.a.dist / state.a.wfacsq;
    if twarp {
        timwrp(state, io).await;
    }
    if blooey {
        state.damage[DWARPEN] = state.a.damfac * (3.0 * state.rng.rand_f64() + 1.0);
        skip(state, io, 1).await;
        prout(state, io, "Engineering to bridge--").await;
        prout(state, io, "  Scott here.  The warp engines are damaged.").await;
        prout(state, io, "  We'll have to reduce speed to warp 4.").await;
    }
    state.a.ididit = 1;
}

pub async fn setwrp(state: &mut GameState, io: &IoHandle) {
    let mut key;
    loop {
        key = scan(state, io).await;
        if key != ScanToken::Eol {
            break;
        }
        state.chew();
        state.proutn("Warp factor-");
    }
    state.chew();
    if key != ScanToken::Real {
        huh(state, io).await;
        return;
    }
    if state.damage[DWARPEN] > 10.0 {
        prout(state, io, "Warp engines inoperative.").await;
        return;
    }
    if state.damage[DWARPEN] > 0.0 && state.aaitem > 4.0 {
        prout(
            state,
            io,
            "Engineer Scott- \"I'm doing my best, Captain,\n  but right now we can only go warp 4.\"",
        )
        .await;
        return;
    }
    if state.aaitem > 10.0 {
        prout(
            state,
            io,
            "Helmsman Sulu- \"Our top speed is warp 10, Captain.\"",
        )
        .await;
        return;
    }
    if state.aaitem < 1.0 {
        prout(
            state,
            io,
            "Helmsman Sulu- \"We can't go below warp 1, Captain.\"",
        )
        .await;
        return;
    }
    let oldfac = state.a.warpfac;
    state.a.warpfac = state.aaitem;
    state.a.wfacsq = state.a.warpfac * state.a.warpfac;
    if state.a.warpfac <= oldfac || state.a.warpfac <= 6.0 {
        state.proutn("Helmsman Sulu- \"Warp factor ");
        state.cramf(state.a.warpfac, 0, 1);
        prout(state, io, ", Captain.\"").await;
        return;
    }
    if state.a.warpfac < 8.00 {
        prout(
            state,
            io,
            "Engineer Scott- \"Aye, but our maximum safe speed is warp 6.\"",
        )
        .await;
        return;
    }
    if state.a.warpfac == 10.0 {
        prout(state, io, "Engineer Scott- \"Aye, Captain, we'll try it.\"").await;
        return;
    }
    prout(
        state,
        io,
        "Engineer Scott- \"Aye, Captain, but our engines may not take it.\"",
    )
    .await;
}

pub async fn atover(state: &mut GameState, io: &IoHandle, igrab: bool) {
    state.chew();
    if state.a.landed == 1 {
        if state.damage[DTRANSP] != 0.0 {
            finish(state, io, FinType::Pnova).await;
            return;
        }
        prout(state, io, "Scotty rushes to the transporter controls.").await;
        if state.a.shldup != 0 {
            prout(state, io, "But with the shields up it's hopeless.").await;
            finish(state, io, FinType::Pnova).await;
            return;
        }
        prouts(state, "His desperate attempt to rescue you . . .");
        if state.rng.rand_f64() <= 0.5 {
            prout(state, io, "fails.").await;
            finish(state, io, FinType::Pnova).await;
            return;
        }
        prout(state, io, "SUCCEEDS!").await;
        if state.a.imine != 0 {
            state.a.imine = 0;
            state.proutn("The crystals mined were ");
            if state.rng.rand_f64() <= 0.25 {
                prout(state, io, "lost.").await;
            } else {
                prout(state, io, "saved.").await;
                state.a.icrystl = 1;
            }
        }
    }
    if igrab {
        return;
    }

    if state.a.icraft != 0 {
        finish(state, io, FinType::Stractor).await;
    }
    if state.a.alldone != 0 {
        return;
    }

    skip(state, io, 1).await;
    loop {
        if state.a.justin != 0 {
            prouts(state, "***RED ALERT!  RED ALERT!");
            skip(state, io, 1).await;
            state.proutn("The ");
            state.crmshp();
            prout(state, io, " has stopped in a quadrant containing").await;
            prouts(state, "   a supernova.");
            skip(state, io, 2).await;
        }
        state.proutn("***Emergency automatic override attempts to hurl ");
        state.crmshp();
        skip(state, io, 1).await;
        prout(state, io, "safely out of quadrant.").await;
        let qx = state.a.quadx as usize;
        let qy = state.a.quady as usize;
        state.starch[qx][qy] = if state.damage[DRADIO] > 0.0 {
            state.d.galaxy[qx][qy] + 1000
        } else {
            1
        };

        if state.damage[DWARPEN] != 0.0 {
            skip(state, io, 1).await;
            prout(state, io, "Warp engines damaged.").await;
            finish(state, io, FinType::Snovaed).await;
            return;
        }
        state.a.warpfac = 6.0 + 2.0 * state.rng.rand_f64();
        state.a.wfacsq = state.a.warpfac * state.a.warpfac;
        state.proutn("Warp factor set to ");
        state.cramf(state.a.warpfac, 1, 1);
        skip(state, io, 1).await;
        let power = 0.75 * state.a.energy;
        state.a.dist =
            power / (state.a.warpfac * state.a.warpfac * state.a.warpfac * ((state.a.shldup + 1) as f64));
        let distreq = 1.4142 + state.rng.rand_f64();
        if distreq < state.a.dist {
            state.a.dist = distreq;
        }
        state.a.time = 10.0 * state.a.dist / state.a.wfacsq;
        state.a.direc = 12.0 * state.rng.rand_f64();
        state.a.justin = 0;
        state.a.inorbit = 0;
        warp(state, io, 2).await;
        if state.a.justin == 0 {
            if state.a.alldone != 0 {
                return;
            }
            skip(state, io, 1).await;
            prout(state, io, "Insufficient energy to leave quadrant.").await;
            finish(state, io, FinType::Snovaed).await;
            return;
        }
        if state.d.galaxy[state.a.quadx as usize][state.a.quady as usize] != 1000 {
            break;
        }
    }
    if state.d.remkl == 0 {
        finish(state, io, FinType::Won).await;
    }
}

pub async fn timwrp(state: &mut GameState, io: &IoHandle) {
    prout(state, io, "***TIME WARP ENTERED.").await;
    if state.d.snap != 0 && state.rng.rand_f64() < 0.5 {
        state.proutn("You are traveling backwards in time ");
        state.cramf(state.d.date - state.snapsht.date, 0, 2);
        prout(state, io, " stardates.").await;
        state.d = state.snapsht.clone();
        state.d.snap = 0;
        if state.d.remcom != 0 {
            state.future[FTBEAM] =
                state.d.date + state.rng.expran(state.a.intime / (state.d.remcom as f64));
            state.future[FBATTAK] = state.d.date + state.rng.expran(0.3 * state.a.intime);
        }
        state.future[FSNOVA] = state.d.date + state.rng.expran(0.5 * state.a.intime);
        state.future[FSNAP] = state.d.date + state.rng.expran(0.25 * state.d.remtime);
        if state.d.nscrem != 0 {
            state.future[FSCMOVE] = state.d.date + 0.2777;
        }
        state.a.isatb = 0;
        state.future[FCDBAS] = 1e30;
        state.future[FSCDBAS] = 1e30;
        state.a.batx = 0;
        state.a.baty = 0;

        let mut gotit = false;
        for l in 1..=(state.a.inplan as usize) {
            if state.d.plnets[l].known == 2 {
                gotit = true;
                if state.a.iscraft == 1 && state.a.ship == crate::state::IHE {
                    prout(
                        state,
                        io,
                        "Checkov-  \"Security reports the Galileo has disappeared, Sir!",
                    )
                    .await;
                    state.a.iscraft = 0;
                }
            }
        }
        if state.a.iscraft == 0 && !gotit && state.damage[crate::state::DSHUTTL] >= 0.0 {
            prout(
                state,
                io,
                "Checkov-  \"Security reports the Galileo has reappeared in the dock!\"",
            )
            .await;
            state.a.iscraft = 1;
        }

        if state.damage[DRADIO] == 0.0 || state.a.stdamtim > state.d.date {
            for l in 1..=8 {
                for ll in 1..=8 {
                    if state.starch[l][ll] > 1 {
                        state.starch[l][ll] = if state.damage[DRADIO] > 0.0 {
                            state.d.galaxy[l][ll] + 1000
                        } else {
                            1
                        };
                    }
                }
            }
            prout(
                state,
                io,
                "Spock has reconstructed a correct star chart from memory",
            )
            .await;
            if state.damage[DRADIO] > 0.0 {
                state.a.stdamtim = state.d.date;
            }
        }
    } else {
        state.a.time = -0.5 * state.a.intime * state.rng.rand_f64().ln();
        state.proutn("You are traveling forward in time ");
        state.cramf(state.a.time, 1, 2);
        prout(state, io, " stardates.").await;
        state.future[FTBEAM] += state.a.time;
        state.damage[DRADIO] += state.a.time;
    }
    newqad(state, io, false).await;
}

pub async fn probe(state: &mut GameState, io: &IoHandle) {
    if state.a.nprobes == 0 {
        state.chew();
        skip(state, io, 1).await;
        if state.a.ship == crate::state::IHE {
            prout(
                state,
                io,
                "Engineer Scott- \"We have no more deep space probes, Sir.\"",
            )
            .await;
        } else {
            prout(state, io, "Ye Faerie Queene has no deep space probes.").await;
        }
        return;
    }
    if state.damage[DDSP] != 0.0 {
        state.chew();
        skip(state, io, 1).await;
        prout(
            state,
            io,
            "Engineer Scott- \"The probe launcher is damaged, Sir.\"",
        )
        .await;
        return;
    }
    if state.future[FDSPROB] != 1e30 {
        state.chew();
        skip(state, io, 1).await;
        if state.reports() {
            prout(
                state,
                io,
                "Uhura- \"The previous probe is still reporting data, Sir.\"",
            )
            .await;
        } else {
            prout(
                state,
                io,
                "Spock-  \"Records show the previous probe has not yet",
            )
            .await;
            prout(state, io, "   reached its destination.\"").await;
        }
        return;
    }

    let mut key = scan(state, io).await;
    if key == ScanToken::Eol {
        state.crami(state.a.nprobes, 1);
        prout(
            state,
            io,
            if state.a.nprobes == 1 {
                " probe left."
            } else {
                " probes left."
            },
        )
        .await;
        state.proutn("Are you sure you want to fire a probe? ");
        if !ja(state, io).await {
            return;
        }
    }

    state.a.isarmed = false;
    if key == ScanToken::Alpha && state.citem == "armed" {
        state.a.isarmed = true;
        key = scan(state, io).await;
    } else if key == ScanToken::Eol {
        state.proutn("Arm NOVAMAX warhead?");
        state.a.isarmed = ja(state, io).await;
    }
    getcd(state, io, true, Some(key)).await;
    if state.a.direc == -1.0 {
        return;
    }
    state.a.nprobes -= 1;
    let angle = (15.0 - state.a.direc) * 0.5235988;
    state.a.probeinx = -angle.sin();
    state.a.probeiny = angle.cos();
    let bigger = state.a.probeinx.abs().max(state.a.probeiny.abs());
    state.a.probeiny /= bigger;
    state.a.probeinx /= bigger;
    state.a.proben = (10.0 * state.a.dist * bigger + 0.5) as i32;
    state.a.probex = (state.a.quadx * 10 + state.a.sectx - 1) as f64;
    state.a.probey = (state.a.quady * 10 + state.a.secty - 1) as f64;
    state.a.probecx = state.a.quadx;
    state.a.probecy = state.a.quady;
    state.future[FDSPROB] = state.d.date + 0.01;
    prout(
        state,
        io,
        "Ensign Chekov-  \"The deep space probe is launched, Captain.\"",
    )
    .await;
}

pub async fn help(state: &mut GameState, io: &IoHandle) {
    state.chew();
    if state.a.condit == IHDOCKED {
        prout(
            state,
            io,
            "Lt. Uhura-  \"But Captain, we're already docked.\"",
        )
        .await;
        return;
    }
    if state.damage[DRADIO] != 0.0 {
        prout(state, io, "Subspace radio damaged.").await;
        return;
    }
    if state.d.rembase == 0 {
        prout(
            state,
            io,
            "Lt. Uhura-  \"Captain, I'm not getting any response from Starbase.\"",
        )
        .await;
        return;
    }
    if state.a.landed == 1 {
        state.proutn("You must be aboard the ");
        state.crmshp();
        prout(state, io, ".").await;
        return;
    }

    state.a.nhelp += 1;
    let ddist: f64;
    if state.a.basex != 0 {
        ddist = (square((state.a.basex - state.a.sectx) as f64)
            + square((state.a.basey - state.a.secty) as f64))
        .sqrt();
    } else {
        let mut min_dist = 1e30;
        let mut best_line = 1usize;
        for l in 1..=(state.d.rembase as usize) {
            let xdist = 10.0
                * (square((state.d.baseqx[l] - state.a.quadx) as f64)
                    + square((state.d.baseqy[l] - state.a.quady) as f64))
                .sqrt();
            if xdist < min_dist {
                min_dist = xdist;
                best_line = l;
            }
        }
        ddist = min_dist;
        state.a.quadx = state.d.baseqx[best_line];
        state.a.quady = state.d.baseqy[best_line];
        newqad(state, io, true).await;
    }

    state.quad[state.a.sectx as usize][state.a.secty as usize] = IHDOT;
    state.proutn("Starbase in");
    state.cramlc(1, state.a.quadx, state.a.quady);
    state.proutn(" responds--");
    state.crmshp();
    prout(state, io, " dematerializes.").await;

    let probf = (1.0 - 0.98f64.powf(ddist)).powf(0.33333333);
    let mut succeeded_attempt = false;
    for l in 1..=3 {
        match l {
            1 => state.proutn("1st"),
            2 => state.proutn("2nd"),
            _ => state.proutn("3rd"),
        }
        state.proutn(" attempt to re-materialize ");
        state.crmshp();
        prouts(state, " . . . . . ");
        if state.rng.rand_f64() > probf {
            succeeded_attempt = true;
            break;
        }
        prout(state, io, "fails.").await;
    }
    if !succeeded_attempt {
        finish(state, io, FinType::Materialize).await;
        return;
    }

    for _l in 1..=5 {
        let ix = state.a.basex + (3.0 * state.rng.rand_f64()) as i32 - 1;
        let iy = state.a.basey + (3.0 * state.rng.rand_f64()) as i32 - 1;
        if (1..=10).contains(&ix)
            && (1..=10).contains(&iy)
            && state.quad[ix as usize][iy as usize] == IHDOT
        {
            prout(state, io, "succeeds.").await;
            state.a.sectx = ix;
            state.a.secty = iy;
            state.quad[ix as usize][iy as usize] = state.a.ship;
            dock(state, io).await;
            skip(state, io, 1).await;
            prout(state, io, "Lt. Uhura-  \"Captain, we made it!\"").await;
            return;
        }
    }
    finish(state, io, FinType::Materialize).await;
}
