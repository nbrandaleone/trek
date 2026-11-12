//! Command table, command listing, `help` documentation lookup, and debug utility.

use crate::io::{prout, skip, IoHandle};
use crate::scanner::{ja, scan, ScanToken};
use crate::state::{
    GameState, DEVICES, DRADIO, FBATTAK, FCDBAS, FSCDBAS, FSCMOVE, FSNAP, FSNOVA, FTBEAM, NDEVICE,
    NEVENTS,
};

static SST_DOC: &str = include_str!("../assets/sst.doc");

pub const COMMANDS: [&str; 37] = [
    "srscan",
    "lrscan",
    "phasers",
    "photons",
    "move",
    "shields",
    "dock",
    "damages",
    "chart",
    "impulse",
    "rest",
    "warp",
    "status",
    "sensors",
    "orbit",
    "transport",
    "mine",
    "crystals",
    "shuttle",
    "planets",
    "request",
    "report",
    "computer",
    "commands",
    "emexit",
    "probe",
    "cloak",
    "capture",
    "score",
    "abandon",
    "destruct",
    "freeze",
    "deathray",
    "debug",
    "call",
    "quit",
    "help",
];

pub async fn list_commands(state: &mut GameState, io: &IoHandle, show_help: bool) {
    prout(
        state,
        io,
        "   SRSCAN    MOVE      PHASERS   CALL\n\
   STATUS    IMPULSE   PHOTONS   ABANDON\n\
   LRSCAN    WARP      SHIELDS   DESTRUCT\n\
   CHART     REST      DOCK      QUIT\n\
   DAMAGES   REPORT    SENSORS   ORBIT\n\
   TRANSPORT MINE      CRYSTALS  SHUTTLE\n\
   PLANETS   REQUEST   DEATHRAY  FREEZE\n\
   COMPUTER  EMEXIT    PROBE     COMMANDS",
    )
    .await;
    state.proutn("   ");
    state.proutn("SCORE     ");
    state.proutn("CLOAK     ");
    state.proutn("CAPTURE   ");
    if show_help {
        state.proutn("HELP     ");
    }
    prout(state, io, "").await;
}

pub async fn helpme(state: &mut GameState, io: &IoHandle) {
    let mut key = scan(state, io).await;
    let cmd_idx = loop {
        if key == ScanToken::Eol {
            state.proutn("Help on what command?");
            key = scan(state, io).await;
        }
        if key == ScanToken::Eol {
            return;
        }
        if let Some(i) = COMMANDS.iter().position(|&c| c == state.citem) {
            break i;
        }
        skip(state, io, 1).await;
        prout(state, io, "Valid commands:").await;
        list_commands(state, io, false).await;
        key = ScanToken::Eol;
        state.chew();
        skip(state, io, 1).await;
    };

    let cmdbuf = if cmd_idx == 23 {
        String::from(" ABBREV")
    } else {
        format!("  Mnemonic:  {}", COMMANDS[cmd_idx].to_ascii_uppercase())
    };

    let mut lines = SST_DOC.split('\n').map(|l| l.trim_end_matches('\r'));
    let Some(first_match) = lines.find(|line| line.starts_with(&cmdbuf)) else {
        prout(
            state,
            io,
            "Spock- \"Captain, there is no information on that command.\"",
        )
        .await;
        return;
    };

    skip(state, io, 1).await;
    prout(
        state,
        io,
        "Spock- \"Captain, I've found the following information:\"",
    )
    .await;
    skip(state, io, 1).await;

    let mut cur = first_match;
    loop {
        if !cur.starts_with('\x0c') {
            prout(state, io, cur).await;
        }
        let Some(next_line) = lines.next() else {
            break;
        };
        if next_line.contains("******") {
            break;
        }
        cur = next_line;
    }
}

pub async fn debugme(state: &mut GameState, io: &IoHandle) {
    state.proutn("Reset levels? ");
    if ja(state, io).await {
        if state.a.energy < state.a.inenrg {
            state.a.energy = state.a.inenrg;
        }
        state.a.shield = state.a.inshld;
        state.a.torps = state.a.intorps;
        state.a.lsupres = state.a.inlsr;
    }
    state.proutn("Reset damage? ");
    if ja(state, io).await {
        for i in 0..=NDEVICE {
            if state.damage[i] > 0.0 {
                state.damage[i] = 0.0;
            }
        }
        state.a.stdamtim = 1e30;
    }
    state.proutn("Toggle idebug? ");
    if ja(state, io).await {
        state.debug_mode = !state.debug_mode;
        if state.debug_mode {
            prout(state, io, "Debug output ON").await;
        } else {
            prout(state, io, "Debug output OFF").await;
        }
    }
    state.proutn("Cause selective damage? ");
    if ja(state, io).await {
        for i in 1..=NDEVICE {
            state.proutn("Kill ");
            state.proutn(DEVICES[i]);
            state.proutn("? ");
            state.chew();
            let key = scan(state, io).await;
            if key == ScanToken::Alpha && state.isit("y") {
                state.damage[i] = 10.0;
                if i == DRADIO {
                    state.a.stdamtim = state.d.date;
                }
            }
        }
    }
    state.proutn("Examine/change events? ");
    if ja(state, io).await {
        for i in 1..NEVENTS {
            if state.future[i] == 1e30 {
                continue;
            }
            match i {
                FSNOVA => state.proutn("Supernova       "),
                FTBEAM => state.proutn("T Beam          "),
                FSNAP => state.proutn("Snapshot        "),
                FBATTAK => state.proutn("Base Attack     "),
                FCDBAS => state.proutn("Base Destroy    "),
                FSCMOVE => state.proutn("SC Move         "),
                FSCDBAS => state.proutn("SC Base Destroy "),
                _ => {}
            }
            state.cramf(state.future[i] - state.d.date, 8, 2);
            state.chew();
            state.proutn("  ?");
            let key = scan(state, io).await;
            if key == ScanToken::Real {
                state.future[i] = state.d.date + state.aaitem;
            }
        }
        state.chew();
    }
    state.proutn("Make universe visible? ");
    if ja(state, io).await {
        for i in 1..=8 {
            for j in 1..=8 {
                state.starch[i][j] = 1;
            }
        }
    }
}
