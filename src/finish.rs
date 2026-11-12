//! End-of-game conditions, self-destruct, scoring, and Commodore Emeritus plaque.

use std::fmt::Write as _;

use crate::battle::deadkl;
use crate::io::{pause, prout, prouts, skip, stars, IoHandle};
use crate::scanner::{ja, scan};
use crate::state::{
    FinType, GameState, DCOMPTR, IHE, IHF, SEMERITUS, SEXPERT, SFAIR, SGOOD, SNOVICE,
};

pub async fn dstrct(state: &mut GameState, io: &IoHandle) {
    state.chew();
    if state.damage[DCOMPTR] != 0.0 {
        prout(state, io, "Computer damaged; cannot execute destruct sequence.").await;
        return;
    }
    skip(state, io, 1).await;
    prouts(state, "---WORKING---");
    skip(state, io, 1).await;
    prout(state, io, "SELF-DESTRUCT-SEQUENCE-ACTIVATED").await;
    prouts(state, "   10");
    skip(state, io, 1).await;
    prouts(state, "       9");
    skip(state, io, 1).await;
    prouts(state, "          8");
    skip(state, io, 1).await;
    prouts(state, "             7");
    skip(state, io, 1).await;
    prouts(state, "                6");
    skip(state, io, 1).await;
    prout(state, io, "ENTER-CORRECT-PASSWORD-TO-CONTINUE-").await;
    prout(state, io, "SELF-DESTRUCT-SEQUENCE-OTHERWISE-").await;
    prout(state, io, "SELF-DESTRUCT-SEQUENCE-WILL-BE-ABORTED").await;
    scan(state, io).await;
    state.chew();
    if state.passwd != state.citem {
        prouts(state, "PASSWORD-REJECTED;");
        skip(state, io, 1).await;
        prout(state, io, "CONTINUITY-EFFECTED").await;
        skip(state, io, 1).await;
        return;
    }
    prouts(state, "PASSWORD-ACCEPTED");
    skip(state, io, 1).await;
    prouts(state, "                   5");
    skip(state, io, 1).await;
    prouts(state, "                      4");
    skip(state, io, 1).await;
    prouts(state, "                         3");
    skip(state, io, 1).await;
    prouts(state, "                            2");
    skip(state, io, 1).await;
    prouts(state, "                              1");
    skip(state, io, 1).await;
    if state.rng.rand_f64() < 0.15 {
        prouts(state, "GOODBYE-CRUEL-WORLD");
        skip(state, io, 1).await;
    }
    skip(state, io, 2).await;
    kaboom(state, io).await;
}

pub async fn kaboom(state: &mut GameState, io: &IoHandle) {
    stars(state, io).await;
    if state.a.ship == IHE {
        prouts(state, "***");
    }
    prouts(state, "********* Entropy of ");
    state.crmshp();
    prouts(state, " maximized *********");
    skip(state, io, 1).await;
    stars(state, io).await;
    skip(state, io, 1).await;
    if state.a.nenhere != 0 {
        let whammo = 25.0 * state.a.energy;
        let mut l = 1usize;
        while l <= state.a.nenhere as usize {
            if state.kpower[l] * state.kdist[l] <= whammo {
                let x = state.kx[l];
                let y = state.ky[l];
                let typ = state.quad[x as usize][y as usize];
                deadkl(state, io, x, y, typ, x, y).await;
            } else {
                l += 1;
            }
        }
    }
    finish(state, io, FinType::Dilithium).await;
}

pub async fn finish(state: &mut GameState, io: &IoHandle, ifin: FinType) {
    let mut igotit = false;
    state.a.alldone = 1;
    skip(state, io, 3).await;
    let date_msg = format!("It is stardate {:.1} .\n\n", state.d.date);
    state.proutn(&date_msg);

    match ifin {
        FinType::Won => {
            if state.d.nromrem != 0 {
                let msg = format!(
                    "The remaining {} Romulan ships surrender to Starfleet Command.\n",
                    state.d.nromrem
                );
                state.proutn(&msg);
            }
            prout(
                state,
                io,
                "You have smashed the Klingon invasion fleet and saved",
            )
            .await;
            prout(state, io, "the Federation.").await;

            if state.a.alive != 0 && state.a.brigcapacity - state.d.brigfree > 0 {
                let cap = state.a.brigcapacity - state.d.brigfree;
                state.d.kcaptured += cap;
                let msg = format!(
                    "The {} captured Klingons are transferred to Star Fleet Command.\n",
                    cap
                );
                state.proutn(&msg);
            }

            state.a.gamewon = 1;
            if state.a.alive != 0 {
                let mut badpt = 5.0 * (state.d.starkl as f64)
                    + (state.a.casual as f64)
                    + 10.0 * (state.d.nplankl as f64)
                    + 45.0 * (state.a.nhelp as f64)
                    + 100.0 * (state.d.basekl as f64);
                if state.a.ship == IHF {
                    badpt += 100.0;
                } else if state.a.ship == '\0' {
                    badpt += 200.0;
                }
                if badpt < 100.0 {
                    badpt = 0.0;
                }
                let dt = state.d.date - state.a.indate;
                let kills = (state.d.killk + state.d.killc + state.d.nsckill) as f64;
                let sk = state.a.skill as f64;
                if dt < 5.0 || (kills / dt) >= 0.1 * sk * (sk + 1.0) + 0.1 + 0.008 * badpt {
                    skip(state, io, 1).await;
                    prout(
                        state,
                        io,
                        "In fact, you have done so well that Starfleet Command",
                    )
                    .await;
                    match state.a.skill {
                        SNOVICE => {
                            prout(
                                state,
                                io,
                                "promotes you one step in rank from \"Novice\" to \"Fair\".",
                            )
                            .await;
                        }
                        SFAIR => {
                            prout(
                                state,
                                io,
                                "promotes you one step in rank from \"Fair\" to \"Good\".",
                            )
                            .await;
                        }
                        SGOOD => {
                            prout(
                                state,
                                io,
                                "promotes you one step in rank from \"Good\" to \"Expert\".",
                            )
                            .await;
                        }
                        SEXPERT => {
                            prout(state, io, "promotes you to Commodore Emeritus.").await;
                            skip(state, io, 1).await;
                            prout(
                                state,
                                io,
                                "Now that you think you're really good, try playing",
                            )
                            .await;
                            prout(
                                state,
                                io,
                                "the \"Emeritus\" game. It will splatter your ego.",
                            )
                            .await;
                        }
                        SEMERITUS => {
                            skip(state, io, 1).await;
                            prout(state, io, "Computer-  ERROR-ERROR-ERROR-ERROR").await;
                            skip(state, io, 1).await;
                            prout(
                                state,
                                io,
                                "  YOUR-SKILL-HAS-EXCEEDED-THE-CAPACITY-OF-THIS-PROGRAM",
                            )
                            .await;
                            prout(state, io, "  THIS-PROGRAM-MUST-SURVIVE").await;
                            prout(state, io, "  THIS-PROGRAM-MUST-SURVIVE").await;
                            prout(state, io, "  THIS-PROGRAM-MUST-SURVIVE").await;
                            prout(
                                state,
                                io,
                                "  THIS-PROGRAM-MUST?- MUST ? - SUR? ? -?  VI",
                            )
                            .await;
                            skip(state, io, 1).await;
                            prout(
                                state,
                                io,
                                "Now you can retire and write your own Star Trek game!",
                            )
                            .await;
                            skip(state, io, 1).await;
                        }
                        _ => {}
                    }
                    if state.a.skill > SGOOD {
                        if state.a.thawed != 0 && !state.debug_mode {
                            prout(state, io, "You cannot get a citation, so...").await;
                        } else {
                            prout(
                                state,
                                io,
                                "Do you want your Commodore Emeritus Citation printed?",
                            )
                            .await;
                            state.proutn("(You need a 132 column printer.)");
                            state.chew();
                            if ja(state, io).await {
                                igotit = true;
                            }
                        }
                    }
                }
                skip(state, io, 1).await;
                prout(state, io, "LIVE LONG AND PROSPER.").await;
            }
            score(state, io, false).await;
            if igotit {
                plaque(state, io).await;
            }
            return;
        }
        FinType::Deplete => {
            prout(
                state,
                io,
                "Your time has run out and the Federation has been",
            )
            .await;
            prout(
                state,
                io,
                "conquered.  Your starship is now Klingon property,",
            )
            .await;
            prout(
                state,
                io,
                "and you are put on trial as a war criminal.  On the",
            )
            .await;
            state.proutn("basis of your record, you are ");
            if (state.d.remkl as f64) * 3.0 > (state.a.inkling as f64) {
                prout(state, io, "aquitted.").await;
                skip(state, io, 1).await;
                prout(state, io, "LIVE LONG AND PROSPER.").await;
            } else {
                prout(state, io, "found guilty and").await;
                prout(state, io, "sentenced to death by slow torture.").await;
                state.a.alive = 0;
            }
            score(state, io, false).await;
            return;
        }
        FinType::Lifesup => {
            prout(state, io, "Your life support reserves have run out, and").await;
            prout(
                state,
                io,
                "you die of thirst, starvation, and asphyxiation.",
            )
            .await;
            prout(state, io, "Your starship is a derelict in space.").await;
        }
        FinType::Nrg => {
            prout(state, io, "Your energy supply is exhausted.").await;
            skip(state, io, 1).await;
            prout(state, io, "Your starship is a derelict in space.").await;
        }
        FinType::Battle => {
            state.proutn("The ");
            state.crmshp();
            prout(state, io, " has been destroyed in battle.").await;
            skip(state, io, 1).await;
            prout(state, io, "Dulce et decorum est pro patria mori.").await;
        }
        FinType::Neg3 => {
            prout(
                state,
                io,
                "You have made three attempts to cross the negative energy",
            )
            .await;
            prout(state, io, "barrier which surrounds the galaxy.").await;
            skip(state, io, 1).await;
            prout(state, io, "Your navigation is abominable.").await;
            score(state, io, false).await;
            return;
        }
        FinType::Nova => {
            prout(state, io, "Your starship has been destroyed by a nova.").await;
            prout(state, io, "That was a great shot.").await;
            skip(state, io, 1).await;
        }
        FinType::Snovaed => {
            state.proutn("The ");
            state.crmshp();
            prout(state, io, " has been fried by a supernova.").await;
            prout(state, io, "...Not even cinders remain...").await;
        }
        FinType::Abandn => {
            prout(
                state,
                io,
                "You have been captured by the Klingons. If you still",
            )
            .await;
            prout(
                state,
                io,
                "had a starbase to be returned to, you would have been",
            )
            .await;
            prout(
                state,
                io,
                "repatriated and given another chance. Since you have",
            )
            .await;
            prout(
                state,
                io,
                "no starbases, you will be mercilessly tortured to death.",
            )
            .await;
        }
        FinType::Dilithium => {
            prout(
                state,
                io,
                "Your starship is now an expanding cloud of subatomic particles",
            )
            .await;
        }
        FinType::Materialize => {
            prout(
                state,
                io,
                "Starbase was unable to re-materialize your starship.",
            )
            .await;
            prout(state, io, "Sic transit gloria muntdi").await;
        }
        FinType::Phaser => {
            state.proutn("The ");
            state.crmshp();
            prout(state, io, " has been cremated by its own phasers.").await;
        }
        FinType::Lost => {
            prout(state, io, "You and your landing party have been").await;
            prout(
                state,
                io,
                "converted to energy, dissipating through space.",
            )
            .await;
        }
        FinType::Mining => {
            prout(state, io, "You are left with your landing party on").await;
            prout(
                state,
                io,
                "a wild jungle planet inhabited by primitive cannibals.",
            )
            .await;
            skip(state, io, 1).await;
            prout(state, io, "They are very fond of \"Captain Kirk\" soup.").await;
            skip(state, io, 1).await;
            state.proutn("Without your leadership, the ");
            state.crmshp();
            prout(state, io, " is destroyed.").await;
        }
        FinType::Dplanet => {
            prout(state, io, "You and your mining party perish.").await;
            skip(state, io, 1).await;
            prout(state, io, "That was a great shot.").await;
            skip(state, io, 1).await;
        }
        FinType::Ssc => {
            prout(
                state,
                io,
                "The Galileo is instantly annihilated by the supernova.",
            )
            .await;
            // Fallthrough to Pnova
            pnova_msg(state, io).await;
        }
        FinType::Pnova => {
            pnova_msg(state, io).await;
        }
        FinType::Stractor => {
            prout(state, io, "The shuttle craft Galileo is also caught,").await;
            prout(state, io, "and breaks up under the strain.").await;
            skip(state, io, 1).await;
            prout(state, io, "Your debris is scattered for millions of miles.").await;
            state.proutn("Without your leadership, the ");
            state.crmshp();
            prout(state, io, " is destroyed.").await;
        }
        FinType::Dray => {
            prout(state, io, "The mutants attack and kill Spock.").await;
            prout(state, io, "Your ship is captured by Klingons, and").await;
            prout(state, io, "your crew is put on display in a Klingon zoo.").await;
        }
        FinType::Tribble => {
            prout(state, io, "Tribbles consume all remaining water,").await;
            prout(state, io, "food, and oxygen on your ship.").await;
            skip(state, io, 1).await;
            prout(
                state,
                io,
                "You die of thirst, starvation, and asphyxiation.",
            )
            .await;
            prout(state, io, "Your starship is a derelict in space.").await;
        }
        FinType::Hole => {
            prout(state, io, "Your ship is drawn to the center of the black hole.").await;
            prout(state, io, "You are crushed into extremely dense matter.").await;
        }
        FinType::Cloak => {
            state.a.ncviol += 1;
            prout(state, io, "You have violated the Treaty of Algeron.").await;
            prout(
                state,
                io,
                "The Romulan Empire can never trust you again.",
            )
            .await;
        }
    }

    if ifin != FinType::Won && ifin != FinType::Cloak && state.a.iscloaked {
        prout(
            state,
            io,
            "Your ship was cloaked so your subspace radio did not receive anything.",
        )
        .await;
        prout(state, io, "You may have missed some warning messages.").await;
        skip(state, io, 1).await;
    }

    if state.a.ship == IHF {
        state.a.ship = '\0';
    } else if state.a.ship == IHE {
        state.a.ship = IHF;
    }
    state.a.alive = 0;

    if state.d.remkl != 0 {
        let goodies = state.d.remres / state.a.inresor;
        let baddies = ((state.d.remkl + 2 * state.d.remcom) as f64)
            / ((state.a.inkling + 2 * state.a.incom) as f64);
        if goodies / baddies >= 1.0 + 0.5 * state.rng.rand_f64() {
            prout(
                state,
                io,
                "As a result of your actions, a treaty with the Klingon",
            )
            .await;
            prout(
                state,
                io,
                "Empire has been signed. The terms of the treaty are",
            )
            .await;
            if goodies / baddies >= 3.0 + state.rng.rand_f64() {
                prout(state, io, "favorable to the Federation.").await;
                skip(state, io, 1).await;
                prout(state, io, "Congratulations!").await;
            } else {
                prout(state, io, "highly unfavorable to the Federation.").await;
            }
        } else {
            prout(state, io, "The Federation will be destroyed.").await;
        }
    } else {
        prout(
            state,
            io,
            "Since you took the last Klingon with you, you are a",
        )
        .await;
        prout(
            state,
            io,
            "martyr and a hero. Someday maybe they'll erect a",
        )
        .await;
        prout(
            state,
            io,
            "statue in your memory. Rest in peace, and try not",
        )
        .await;
        prout(state, io, "to think about pigeons.").await;
        state.a.gamewon = 1;
    }
    score(state, io, false).await;
}

async fn pnova_msg(state: &mut GameState, io: &IoHandle) {
    prout(state, io, "You and your mining party are atomized.").await;
    skip(state, io, 1).await;
    state.proutn("Mr. Spock takes command of the ");
    state.crmshp();
    prout(state, io, " and").await;
    prout(
        state,
        io,
        "joins the Romulans, reigning terror on the Federation.",
    )
    .await;
}

pub async fn score(state: &mut GameState, io: &IoHandle, in_game: bool) {
    let mut timused = state.d.date - state.a.indate;
    let mut dnromrem = state.d.nromrem;

    if !in_game {
        pause(state, io, 0).await;
    }

    state.iskill = state.a.skill;
    if (timused == 0.0 || state.d.remkl != 0) && timused < 5.0 {
        timused = 5.0;
    }
    state.perdate = ((state.d.killc + state.d.killk + state.d.nsckill) as f64) / timused;
    let ithperd = (500.0 * state.perdate + 0.5) as i32;
    let iwon = if state.a.gamewon != 0 {
        100 * state.a.skill
    } else {
        0
    };
    let klship = if state.a.ship == IHE {
        0
    } else if state.a.ship == IHF {
        1
    } else {
        2
    };
    if state.a.gamewon == 0 || in_game {
        dnromrem = 0;
    }
    state.iscore = 10 * state.d.killk
        + 50 * state.d.killc
        + ithperd
        + iwon
        - 100 * state.d.basekl
        - 100 * klship
        - 45 * state.a.nhelp
        - 5 * state.d.starkl
        - state.a.casual
        + 20 * state.d.nromkl
        + 200 * state.d.nsckill
        - 10 * state.d.nplankl
        + dnromrem
        - 100 * state.a.ncviol
        + 3 * state.d.kcaptured;
    if state.a.alive == 0 {
        state.iscore -= 200;
    }

    skip(state, io, 2).await;
    if in_game {
        prout(state, io, "Your score so far --").await;
    } else {
        prout(state, io, "Your score --").await;
    }

    if state.d.nromkl != 0 {
        let msg = if state.d.nromkl > 1 {
            format!(
                "{:6} Romulan ships destroyed            {:5}\n",
                state.d.nromkl,
                20 * state.d.nromkl
            )
        } else {
            format!(
                "{:6} Romulan ship destroyed             {:5}\n",
                state.d.nromkl,
                20 * state.d.nromkl
            )
        };
        state.proutn(&msg);
    }
    if dnromrem != 0 {
        let msg = if dnromrem > 1 {
            format!(
                "{:6} Romulan ships surrendered         {:5}\n",
                dnromrem, dnromrem
            )
        } else {
            format!(
                "{:6} Romulan ship surrendered           {:5}\n",
                dnromrem, dnromrem
            )
        };
        state.proutn(&msg);
    }
    if state.d.killk != 0 {
        let msg = if state.d.killk > 1 {
            format!(
                "{:6} ordinary Klingon ships destroyed   {:5}\n",
                state.d.killk,
                10 * state.d.killk
            )
        } else {
            format!(
                "{:6} ordinary Klingon ship destroyed    {:5}\n",
                state.d.killk,
                10 * state.d.killk
            )
        };
        state.proutn(&msg);
    }
    if state.d.killc != 0 {
        let msg = if state.d.killc > 1 {
            format!(
                "{:6} Klingon Commander ships destroyed  {:5}\n",
                state.d.killc,
                50 * state.d.killc
            )
        } else {
            format!(
                "{:6} Klingon Commander ship destroyed   {:5}\n",
                state.d.killc,
                50 * state.d.killc
            )
        };
        state.proutn(&msg);
    }
    if state.d.nsckill != 0 {
        let msg = format!(
            "{:6} Super-Commander ship destroyed     {:5}\n",
            state.d.nsckill,
            200 * state.d.nsckill
        );
        state.proutn(&msg);
    }
    if ithperd != 0 {
        let msg = format!(
            "{:6.2} Klingon ships per stardate         {:5}\n",
            state.perdate, ithperd
        );
        state.proutn(&msg);
    }
    if state.d.kcaptured != 0 {
        let msg = if state.d.kcaptured > 1 {
            format!(
                "{:6} Klingons captured                  {:5}\n",
                state.d.kcaptured,
                3 * state.d.kcaptured
            )
        } else {
            format!(
                "{:6} Klingon captured                   {:5}\n",
                state.d.kcaptured,
                3 * state.d.kcaptured
            )
        };
        state.proutn(&msg);
    }
    if state.d.starkl != 0 {
        let msg = if state.d.starkl > 1 {
            format!(
                "{:6} stars destroyed by your action     {:5}\n",
                state.d.starkl,
                -5 * state.d.starkl
            )
        } else {
            format!(
                "{:6} star destroyed by your action      {:5}\n",
                state.d.starkl,
                -5 * state.d.starkl
            )
        };
        state.proutn(&msg);
    }
    if state.d.nplankl != 0 {
        let msg = if state.d.nplankl > 1 {
            format!(
                "{:6} planets destroyed by your action   {:5}\n",
                state.d.nplankl,
                -10 * state.d.nplankl
            )
        } else {
            format!(
                "{:6} planet destroyed by your action    {:5}\n",
                state.d.nplankl,
                -10 * state.d.nplankl
            )
        };
        state.proutn(&msg);
    }
    if state.d.basekl != 0 {
        let msg = if state.d.basekl > 1 {
            format!(
                "{:6} bases destroyed by your action     {:5}\n",
                state.d.basekl,
                -100 * state.d.basekl
            )
        } else {
            format!(
                "{:6} base destroyed by your action      {:5}\n",
                state.d.basekl,
                -100 * state.d.basekl
            )
        };
        state.proutn(&msg);
    }
    if state.a.nhelp != 0 {
        let msg = if state.a.nhelp > 1 {
            format!(
                "{:6} calls for help from starbase       {:5}\n",
                state.a.nhelp,
                -45 * state.a.nhelp
            )
        } else {
            format!(
                "{:6} call for help from starbase        {:5}\n",
                state.a.nhelp,
                -45 * state.a.nhelp
            )
        };
        state.proutn(&msg);
    }
    if state.a.casual != 0 {
        let msg = if state.a.casual > 1 {
            format!(
                "{:6} casualties incurred                {:5}\n",
                state.a.casual, -state.a.casual
            )
        } else {
            format!(
                "{:6} casualty incurred                  {:5}\n",
                state.a.casual, -state.a.casual
            )
        };
        state.proutn(&msg);
    }
    if klship != 0 {
        let msg = if klship > 1 {
            format!(
                "{:6} ships lost or destroyed            {:5}\n",
                klship,
                -100 * klship
            )
        } else {
            format!(
                "{:6} ship lost or destroyed             {:5}\n",
                klship,
                -100 * klship
            )
        };
        state.proutn(&msg);
    }
    if state.a.ncviol > 0 {
        let msg = if state.a.ncviol > 1 {
            format!(
                "{:6} Treaty of Algeron violations       {:5}\n",
                state.a.ncviol,
                -100 * state.a.ncviol
            )
        } else {
            format!(
                "{:6} Treaty of Algeron violation        {:5}\n",
                state.a.ncviol,
                -100 * state.a.ncviol
            )
        };
        state.proutn(&msg);
    }
    if state.a.alive == 0 {
        prout(state, io, "Penalty for getting yourself killed        -200").await;
    }
    if state.a.gamewon != 0 {
        skip(state, io, 1).await;
        state.proutn("Bonus for winning ");
        match state.a.skill {
            SNOVICE => state.proutn("Novice game  "),
            SFAIR => state.proutn("Fair game    "),
            SGOOD => state.proutn("Good game    "),
            SEXPERT => state.proutn("Expert game  "),
            SEMERITUS => state.proutn("Emeritus game"),
            _ => {}
        }
        let msg = format!("           {:5}\n", iwon);
        state.proutn(&msg);
    }
    skip(state, io, 2).await;
    let total_msg = format!("TOTAL SCORE                               {:5}\n", state.iscore);
    state.proutn(&total_msg);
    if in_game && state.a.skill < SGOOD {
        state.proutn(
            "REMEMBER--The score doesn't really matter until the mission is accomplished!\n",
        );
    }
}

pub async fn plaque(state: &mut GameState, io: &IoHandle) {
    skip(state, io, 2).await;

    let fname = loop {
        state.proutn("File or device name for your plaque:");
        let raw = io.read_line(state).await;
        let trimmed = raw.trim_end_matches(&['\r', '\n'][..]).trim();
        if trimmed.is_empty() {
            state.proutn("Invalid name.\n");
        } else {
            break trimmed.to_string();
        }
    };

    state.proutn("Enter name to go on plaque (up to 30 characters):");
    let raw_winner = io.read_line(state).await;
    let winner: String = raw_winner
        .trim_end_matches(&['\r', '\n'][..])
        .chars()
        .take(30)
        .collect();
    let nskip = 64usize.saturating_sub(winner.len() / 2);

    let mut out = String::new();
    out.push_str("\n\n\n\n");
    out.push_str("                                                                EEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEE\n");
    out.push_str("                                      EEE                      E  : :                                         :  E\n");
    out.push_str("                                    EE   EEE                   E  : :                   NCC-1701              :  E\n");
    out.push_str("                    EEEEEEEEEEEEEEEE        EEEEEEEEEEEEEEE    E  : :                                         : E\n");
    out.push_str("                     E                                     E    EEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEE\n");
    out.push_str("                      EEEEEEEEE               EEEEEEEEEEEEE                 E  E\n");
    out.push_str("                               EEEEEEE   EEEEE    E          E              E  E\n");
    out.push_str("                                      EEE           E          E            E  E\n");
    out.push_str("                                                       E         E          E  E\n");
    out.push_str("                                                         EEEEEEEEEEEEE      E  E\n");
    out.push_str("                                                      EEE :           EEEEEEE  EEEEEEEE\n");
    out.push_str("                                                    :E    :                 EEEE       E\n");
    out.push_str("                                                   .-E   -:-----                       E\n");
    out.push_str("                                                    :E    :                            E\n");
    out.push_str("                                                      EE  :                    EEEEEEEE\n");
    out.push_str("                                                       EEEEEEEEEEEEEEEEEEEEEEE\n");
    out.push_str("\n\n\n");
    out.push_str("                                                       U. S. S. ENTERPRISE\n");
    out.push_str("\n\n\n\n");
    out.push_str("                                  For demonstrating outstanding ability as a starship captain\n");
    out.push_str("\n");
    out.push_str("                                                Starfleet Command bestows to you\n");
    out.push_str("\n");
    let _ = writeln!(out, "{:>width$}{}\n", "", winner, width = nskip);
    out.push_str("                                                           the rank of\n\n");
    out.push_str("                                                       \"Commodore Emeritus\"\n\n");
    out.push_str("                                                          ");
    match state.iskill {
        SEXPERT => out.push_str(" Expert level\n\n"),
        SEMERITUS => out.push_str("Emeritus level\n\n"),
        _ => out.push_str(" Cheat level\n\n"),
    }
    let ts = if state.plaque_timestamp.is_empty() {
        "Jan  1 2026, 00:00:00"
    } else {
        state.plaque_timestamp.as_str()
    };
    let _ = writeln!(
        out,
        "                                                 This day of {}\n",
        ts
    );
    let _ = writeln!(
        out,
        "                                                        Your score:  {}\n",
        state.iscore
    );
    let _ = writeln!(
        out,
        "                                                    Klingons per stardate:  {:.2}",
        state.perdate
    );

    state.plaques.insert(fname.clone(), out.clone());
    state.plaques_updated.push((fname, out));
}
