//! Planetary operations: orbit, sensor scan, beam, shuttle, mining, dilithium crystals, death ray.

use crate::battle::deadkl;
use crate::events::events;
use crate::finish::{finish, kaboom};
use crate::io::{prout, prouts, skip, stars, IoHandle};
use crate::scanner::ja;
use crate::state::{
    FinType, GameState, DDRAY, DIMPULS, DSHUTTL, DSRSENS, DTRANSP, DWARPEN, IHDOCKED, IHDOT,
    IHE, IHQUEST,
};

const CLASSES: [&str; 4] = ["", "M", "N", "O"];

async fn consume_time(state: &mut GameState, io: &IoHandle) -> bool {
    state.a.ididit = 1;
    events(state, io).await;
    state.a.alldone != 0
        || state.d.galaxy[state.a.quadx as usize][state.a.quady as usize] == 1000
        || state.a.justin != 0
}

pub async fn preport(state: &mut GameState, io: &IoHandle) {
    let mut iknow = false;
    skip(state, io, 1).await;
    state.chew();
    prout(state, io, "Spock-  \"Planet report follows, Captain.\"").await;
    skip(state, io, 1).await;
    for i in 1..=(state.a.inplan as usize) {
        let p = state.d.plnets[i];
        if p.known != 0 || (state.debug_mode && p.x != 0) {
            iknow = true;
            if state.debug_mode && p.known == 0 {
                state.proutn("(Unknown) ");
            }
            state.cramlc(1, p.x, p.y);
            state.proutn("   class ");
            state.proutn(CLASSES[p.pclass as usize]);
            state.proutn("   ");
            if p.crystals == 0 {
                state.proutn("no ");
            }
            prout(state, io, "dilithium crystals present.").await;
            if p.known == 2 {
                prout(state, io, "    Shuttle Craft Galileo on surface.").await;
            }
        }
    }
    if !iknow {
        prout(state, io, "No information available.").await;
    }
}

pub async fn orbit(state: &mut GameState, io: &IoHandle) {
    skip(state, io, 1).await;
    state.chew();
    state.a.ididit = 0;
    if state.a.inorbit != 0 {
        prout(state, io, "Already in standard orbit.").await;
        return;
    }
    if state.damage[DWARPEN] != 0.0 && state.damage[DIMPULS] != 0.0 {
        prout(state, io, "Both warp and impulse engines damaged.").await;
        return;
    }
    if state.a.plnetx == 0
        || (state.a.sectx - state.a.plnetx).abs() > 1
        || (state.a.secty - state.a.plnety).abs() > 1
    {
        state.crmshp();
        prout(state, io, " not adjacent to planet.\n").await;
        return;
    }
    state.a.time = 0.02 + 0.03 * state.rng.rand_f64();
    prout(
        state,
        io,
        "Helmsman Sulu-  \"Entering standard orbit, Sir.\"",
    )
    .await;
    state.newcnd();
    if consume_time(state, io).await {
        return;
    }
    state.height = (1400.0 + 7200.0 * state.rng.rand_f64()) as i32;
    state.proutn("Sulu-  \"Entered orbit at altitude ");
    state.cramf(state.height as f64, 0, 2);
    prout(state, io, " kilometers.\"").await;
    state.a.inorbit = 1;
}

pub async fn sensor(state: &mut GameState, io: &IoHandle) {
    skip(state, io, 1).await;
    state.chew();
    if state.damage[DSRSENS] != 0.0 {
        prout(state, io, "Short range sensors damaged.").await;
        return;
    }
    if state.a.plnetx == 0 {
        prout(state, io, "No planet in this quadrant.").await;
        return;
    }
    state.proutn("Spock-  \"Sensor scan for");
    state.cramlc(1, state.a.quadx, state.a.quady);
    prout(state, io, "-").await;
    skip(state, io, 1).await;
    state.proutn("         Planet at");
    state.cramlc(2, state.a.plnetx, state.a.plnety);
    state.proutn(" is of class ");
    let ip = state.a.iplnet as usize;
    state.proutn(CLASSES[state.d.plnets[ip].pclass as usize]);
    prout(state, io, ".").await;
    if state.d.plnets[ip].known == 2 {
        prout(
            state,
            io,
            "         Sensors show Galileo still on surface.",
        )
        .await;
    }
    state.proutn("         Readings indicate");
    if state.d.plnets[ip].crystals == 0 {
        state.proutn(" no");
    }
    prout(state, io, " dilithium crystals present.\"").await;
    if state.d.plnets[ip].known == 0 {
        state.d.plnets[ip].known = 1;
    }
}

pub async fn beam(state: &mut GameState, io: &IoHandle) {
    state.chew();
    skip(state, io, 1).await;
    let ip = state.a.iplnet as usize;
    if state.damage[DTRANSP] != 0.0 {
        prout(state, io, "Transporter damaged.").await;
        if state.damage[DSHUTTL] == 0.0
            && (state.d.plnets[ip].known == 2 || state.a.iscraft == 1)
        {
            skip(state, io, 1).await;
            prout(state, io, "Spock-  \"May I suggest the shuttle craft, Sir?\" ").await;
            if ja(state, io).await {
                Box::pin(shuttle(state, io)).await;
            }
        }
        return;
    }
    if state.a.inorbit == 0 {
        state.crmshp();
        prout(state, io, " not in standard orbit.").await;
        return;
    }
    if state.a.shldup != 0 {
        prout(state, io, "Impossible to transport through shields.").await;
        return;
    }
    if state.d.plnets[ip].known == 0 {
        prout(
            state,
            io,
            "Spock-  \"Captain, we have no information on this planet",
        )
        .await;
        prout(
            state,
            io,
            "  and Starfleet Regulations clearly state that in this situation",
        )
        .await;
        prout(state, io, "  you may not go down.\"").await;
        return;
    }

    if state.a.landed == 1 {
        if state.d.plnets[ip].known == 2 {
            state.proutn("Spock-  \"Wouldn't you rather take the Galileo?\" ");
            if ja(state, io).await {
                state.chew();
                return;
            }
            prout(
                state,
                io,
                "Your crew hides the Galileo to prevent capture by aliens.",
            )
            .await;
        }
        prout(state, io, "Landing party assembled, ready to beam up.").await;
        skip(state, io, 1).await;
        prout(state, io, "Kirk whips out communicator...").await;
        prouts(state, "BEEP  BEEP  BEEP");
        skip(state, io, 2).await;
        prout(
            state,
            io,
            "\"Kirk to enterprise-  Lock on coordinates...energize.\"",
        )
        .await;
    } else {
        if state.d.plnets[ip].crystals == 0 {
            prout(
                state,
                io,
                "Spock-  \"Captain, I fail to see the logic in",
            )
            .await;
            prout(state, io, "  exploring a planet with no dilithium crystals.").await;
            state.proutn("  Are you sure this is wise?\" ");
            if !ja(state, io).await {
                state.chew();
                return;
            }
        }
        prout(state, io, "Scotty-  \"Transporter room ready, Sir.\"").await;
        skip(state, io, 1).await;
        prout(
            state,
            io,
            "Kirk, and landing party prepare to beam down to planet surface.",
        )
        .await;
        skip(state, io, 1).await;
        prout(state, io, "Kirk-  \"Energize.\"").await;
    }

    skip(state, io, 1).await;
    prouts(state, "WWHOOOIIIIIRRRRREEEE.E.E.  .  .  .  .   .    .");
    skip(state, io, 2).await;
    if state.rng.rand_f64() > 0.98 {
        prouts(state, "BOOOIIIOOOIIOOOOIIIOIING . . .");
        skip(state, io, 2).await;
        prout(state, io, "Scotty-  \"Oh my God!  I've lost them.\"").await;
        finish(state, io, FinType::Lost).await;
        return;
    }
    prouts(state, ".    .   .  .  .  .  .E.E.EEEERRRRRIIIIIOOOHWW");
    skip(state, io, 2).await;
    prout(state, io, "Transport complete.").await;
    state.a.landed = -state.a.landed;
    if state.a.landed == 1 && state.d.plnets[ip].known == 2 {
        prout(state, io, "The shuttle craft Galileo is here!").await;
    }
    if state.a.landed != 1 && state.a.imine == 1 {
        state.a.icrystl = 1;
        state.a.cryprob = 0.05;
    }
    state.a.imine = 0;
}

pub async fn mine(state: &mut GameState, io: &IoHandle) {
    state.a.ididit = 0;
    skip(state, io, 1).await;
    state.chew();
    if state.a.landed != 1 {
        prout(state, io, "Mining party not on planet.").await;
        return;
    }
    let ip = state.a.iplnet as usize;
    if state.d.plnets[ip].crystals == 0 {
        prout(state, io, "No dilithium crystals on this planet.").await;
        return;
    }
    if state.a.imine == 1 {
        prout(
            state,
            io,
            "You've already mined enough crystals for this trip.",
        )
        .await;
        return;
    }
    if state.a.icrystl == 1 && state.a.cryprob == 0.05 {
        state.proutn("With all those fresh crystals aboard the ");
        state.crmshp();
        skip(state, io, 1).await;
        prout(
            state,
            io,
            "there's no reason to mine more at this time.",
        )
        .await;
        return;
    }
    state.a.time = (0.1 + 0.2 * state.rng.rand_f64()) * (state.d.plnets[ip].pclass as f64);
    if consume_time(state, io).await {
        return;
    }
    prout(state, io, "Mining operation complete.").await;
    state.a.imine = 1;
}

pub async fn usecrystals(state: &mut GameState, io: &IoHandle) {
    skip(state, io, 1).await;
    state.chew();
    if state.a.icrystl != 1 {
        prout(state, io, "No dilithium crystals available.").await;
        return;
    }
    if state.a.energy >= 1000.0 {
        prout(
            state,
            io,
            "Spock-  \"Captain, Starfleet Regulations prohibit such an operation",
        )
        .await;
        prout(state, io, "  except when condition Yellow exists.").await;
        return;
    }
    prout(
        state,
        io,
        "Spock- \"Captain, I must warn you that loading",
    )
    .await;
    prout(state, io, "  raw dilithium crystals into the ship's power").await;
    prout(state, io, "  system may risk a severe explosion.").await;
    state.proutn("  Are you sure this is wise?\" ");
    if !ja(state, io).await {
        state.chew();
        return;
    }
    skip(state, io, 1).await;
    prout(
        state,
        io,
        "Engineering Officer Scott-  \"(GULP) Aye Sir.",
    )
    .await;
    prout(state, io, "  Mr. Spock and I will try it.\"").await;
    skip(state, io, 1).await;
    prout(state, io, "Spock-  \"Crystals in place, Sir.").await;
    prout(state, io, "  Ready to activate circuit.\"").await;
    skip(state, io, 1).await;
    prouts(state, "Scotty-  \"Keep your fingers crossed, Sir!\"");
    skip(state, io, 1).await;
    if state.rng.rand_f64() <= state.a.cryprob {
        prouts(state, "  \"Activating now! - - No good!  It's***");
        skip(state, io, 2).await;
        prouts(state, "***RED ALERT!  RED A*L********************************");
        skip(state, io, 1).await;
        stars(state, io).await;
        prouts(state, "******************   KA-BOOM!!!!   *******************");
        skip(state, io, 1).await;
        kaboom(state, io).await;
        return;
    }
    state.a.energy += 5000.0 * (1.0 + 0.9 * state.rng.rand_f64());
    prouts(state, "  \"Activating now! - - ");
    prout(state, io, "The instruments").await;
    prout(state, io, "   are going crazy, but I think it's").await;
    prout(state, io, "   going to work!!  Congratulations, Sir!\"").await;
    state.a.cryprob *= 2.0;
}

pub async fn shuttle(state: &mut GameState, io: &IoHandle) {
    state.chew();
    skip(state, io, 1).await;
    state.a.ididit = 0;
    let ip = state.a.iplnet as usize;

    if state.damage[DSHUTTL] != 0.0 {
        if state.damage[DSHUTTL] == -1.0 {
            if state.a.inorbit != 0 && state.d.plnets[ip].known == 2 {
                prout(
                    state,
                    io,
                    "Ye Faerie Queene has no shuttle craft bay to dock it at.",
                )
                .await;
            } else {
                prout(state, io, "Ye Faerie Queene had no shuttle craft.").await;
            }
        } else if state.damage[DSHUTTL] > 0.0 {
            prout(state, io, "The Galileo is damaged.").await;
        } else {
            prout(state, io, "Shuttle craft is now serving Big Mac's.").await;
        }
        return;
    }
    if state.a.inorbit == 0 {
        state.crmshp();
        prout(state, io, " not in standard orbit.").await;
        return;
    }
    if state.d.plnets[ip].known != 2 && state.a.iscraft != 1 {
        prout(state, io, "Shuttle craft not currently available.").await;
        return;
    }
    if state.a.landed == -1 && state.d.plnets[ip].known == 2 {
        prout(
            state,
            io,
            "You will have to beam down to retrieve the shuttle craft.",
        )
        .await;
        return;
    }
    if state.a.shldup != 0 || state.a.condit == IHDOCKED {
        prout(state, io, "Shuttle craft cannot pass through shields.").await;
        return;
    }
    if state.d.plnets[ip].known == 0 {
        prout(
            state,
            io,
            "Spock-  \"Captain, we have no information on this planet",
        )
        .await;
        prout(
            state,
            io,
            "  and Starfleet Regulations clearly state that in this situation",
        )
        .await;
        prout(state, io, "  you may not fly down.\"").await;
        return;
    }

    state.a.time = 3.0e-5 * (state.height as f64);
    if state.a.time >= 0.8 * state.d.remtime {
        prout(
            state,
            io,
            "First Officer Spock-  \"Captain, I compute that such",
        )
        .await;
        prout(state, io, "  a maneuver would require approximately ").await;
        state.cramf(100.0 * state.a.time / state.d.remtime, 0, 4);
        prout(state, io, "% of our").await;
        prout(state, io, "remaining time.").await;
        prout(state, io, "Are you sure this is wise?\" ").await;
        if !ja(state, io).await {
            state.a.time = 0.0;
            return;
        }
    }

    if state.a.landed == 1 {
        if state.a.iscraft == 1 {
            if state.damage[DTRANSP] == 0.0 {
                state.proutn("Spock-  \"Would you rather use the transporter?\" ");
                if ja(state, io).await {
                    Box::pin(beam(state, io)).await;
                    return;
                }
                state.proutn("Shuttle crew");
            } else {
                state.proutn("Rescue party");
            }
            prout(
                state,
                io,
                " boards Galileo and swoops toward planet surface.",
            )
            .await;
            state.a.iscraft = 0;
            skip(state, io, 1).await;
            if consume_time(state, io).await {
                return;
            }
            state.d.plnets[ip].known = 2;
            prout(state, io, "Trip complete.").await;
        } else {
            prout(state, io, "You and your mining party board the").await;
            prout(
                state,
                io,
                "shuttle craft for the trip back to the Enterprise.",
            )
            .await;
            skip(state, io, 1).await;
            prout(state, io, "The short hop begins . . .").await;
            state.d.plnets[ip].known = 1;
            state.a.icraft = 1;
            skip(state, io, 1).await;
            state.a.landed = -1;
            if consume_time(state, io).await {
                return;
            }
            state.a.iscraft = 1;
            state.a.icraft = 0;
            if state.a.imine != 0 {
                state.a.icrystl = 1;
                state.a.cryprob = 0.05;
            }
            state.a.imine = 0;
            prout(state, io, "Trip complete.").await;
        }
    } else {
        prout(state, io, "Mining party assembles in the hangar deck,").await;
        prout(
            state,
            io,
            "ready to board the shuttle craft \"Galileo\".",
        )
        .await;
        skip(state, io, 1).await;
        prouts(state, "The hangar doors open; the trip begins.");
        skip(state, io, 1).await;
        state.a.icraft = 1;
        state.a.iscraft = 0;
        if consume_time(state, io).await {
            return;
        }
        state.d.plnets[ip].known = 2;
        state.a.landed = 1;
        state.a.icraft = 0;
        prout(state, io, "Trip complete").await;
    }
}

pub async fn deathray(state: &mut GameState, io: &IoHandle) {
    let mut r = state.rng.rand_f64();

    state.a.ididit = 0;
    skip(state, io, 1).await;
    state.chew();
    if state.a.ship != IHE {
        prout(state, io, "Ye Faerie Queene has no death ray.").await;
        return;
    }
    if state.a.nenhere == 0 {
        prout(
            state,
            io,
            "Sulu-  \"But Sir, there are no enemies in this quadrant.\"",
        )
        .await;
        return;
    }
    if state.damage[DDRAY] > 0.0 {
        prout(state, io, "Death Ray is damaged.").await;
        return;
    }
    prout(
        state,
        io,
        "Spock-  \"Captain, the 'Experimental Death Ray'",
    )
    .await;
    prout(
        state,
        io,
        "  is highly unpredictable.  Considering the alternatives,",
    )
    .await;
    prout(state, io, "  are you sure this is wise?\" ").await;
    if !ja(state, io).await {
        return;
    }
    prout(state, io, "Spock-  \"Acknowledged.\"").await;
    skip(state, io, 1).await;
    state.a.ididit = 1;
    prouts(state, "WHOOEE ... WHOOEE ... WHOOEE ... WHOOEE");
    skip(state, io, 1).await;
    prout(state, io, "Crew scrambles in emergency preparation.").await;
    prout(state, io, "Spock and Scotty ready the death ray and").await;
    prout(
        state,
        io,
        "prepare to channel all ship's power to the device.",
    )
    .await;
    skip(state, io, 1).await;
    prout(state, io, "Spock-  \"Preparations complete, sir.\"").await;
    prout(state, io, "Kirk-  \"Engage!\"").await;
    skip(state, io, 1).await;
    prouts(state, "WHIRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRR");
    skip(state, io, 1).await;
    if r > 0.30 {
        prouts(state, "Sulu- \"Captain!  It's working!\"");
        skip(state, io, 2).await;
        while state.a.nenhere > 0 {
            let x = state.kx[1];
            let y = state.ky[1];
            let typ = state.quad[x as usize][y as usize];
            deadkl(state, io, x, y, typ, x, y).await;
        }
        prout(state, io, "Ensign Chekov-  \"Congratulations, Captain!\"").await;
        if state.d.remkl == 0 {
            finish(state, io, FinType::Won).await;
            return;
        }
        prout(
            state,
            io,
            "Spock-  \"Captain, I believe the `Experimental Death Ray'",
        )
        .await;
        if state.rng.rand_f64() <= 0.05 {
            prout(state, io, "   is still operational.\"").await;
        } else {
            prout(state, io, "   has been rendered dysfunctional.\"").await;
            state.damage[DDRAY] = 39.95;
        }
        return;
    }
    r = state.rng.rand_f64();
    if r <= 0.30 {
        prouts(state, "Sulu- \"Captain!  It's working!\"");
        skip(state, io, 1).await;
        prouts(state, "***RED ALERT!  RED ALERT!");
        skip(state, io, 1).await;
        prout(state, io, "***MATTER-ANTIMATTER IMPLOSION IMMINENT!").await;
        skip(state, io, 1).await;
        prouts(state, "***RED ALERT!  RED A*L********************************");
        skip(state, io, 1).await;
        stars(state, io).await;
        prouts(state, "******************   KA-BOOM!!!!   *******************");
        skip(state, io, 1).await;
        kaboom(state, io).await;
        return;
    }
    if r <= 0.55 {
        prouts(
            state,
            "Sulu- \"Captain!  Yagabandaghangrapl, brachriigringlanbla!\"",
        );
        skip(state, io, 1).await;
        prout(state, io, "Lt. Uhura-  \"Graaeek!  Graaeek!\"").await;
        skip(state, io, 1).await;
        prout(
            state,
            io,
            "Spock-  \"Fascinating!  . . . All humans aboard",
        )
        .await;
        prout(
            state,
            io,
            "  have apparently been transformed into strange mutations.",
        )
        .await;
        prout(state, io, "  Vulcans do not seem to be affected.").await;
        skip(state, io, 1).await;
        prout(state, io, "Kirk-  \"Raauch!  Raauch!\"").await;
        finish(state, io, FinType::Dray).await;
        return;
    }
    if r <= 0.75 {
        prouts(state, "Sulu- \"Captain!  It's   --WHAT?!?!\"");
        skip(state, io, 2).await;
        state.proutn("Spock-  \"I believe the word is");
        prouts(state, " *ASTONISHING*");
        prout(state, io, " Mr. Sulu.").await;
        for i in 1..=10 {
            for j in 1..=10 {
                if state.quad[i][j] == IHDOT {
                    state.quad[i][j] = IHQUEST;
                }
            }
        }
        prout(
            state,
            io,
            "  Captain, our quadrant is now infested with",
        )
        .await;
        prouts(state, " - - - - - -  *THINGS*.");
        skip(state, io, 1).await;
        prout(state, io, "  I have no logical explanation.\"").await;
        return;
    }
    prouts(
        state,
        "Sulu- \"Captain!  The Death Ray is creating tribbles!\"",
    );
    skip(state, io, 1).await;
    prout(state, io, "Scotty-  \"There are so many tribbles down here").await;
    prout(
        state,
        io,
        "  in Engineering, we can't move for 'em, Captain.\"",
    )
    .await;
    finish(state, io, FinType::Tribble).await;
}
