//! Coroutine session driver (`TrekSession`), HUD snapshot (`HudState`), and JSON `StepResponse`.

use std::fmt::Write as _;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

use crate::ai::movetho;
use crate::battle::{attack, capture, cloak, phasers, photon, sheild};
use crate::events::{events, waiting};
use crate::finish::{dstrct, finish, score};
use crate::help::{debugme, helpme, list_commands, COMMANDS};
use crate::io::{prout, skip, stars, InputEvent, IoHandle, PromptType};
use crate::moving::{atover, dock, help, impuls, probe, setwrp, warp};
use crate::planets::{beam, deathray, mine, orbit, preport, sensor, shuttle, usecrystals};
use crate::reports::{chart, dreprt, eta, lrscan, report, srscan};
use crate::scanner::{ja, scan, ScanToken};
use crate::setup::{abandn, freeze, prelim, setup};
use crate::state::{
    fmt_json_f64, json_escape, GameState, ALGERON, DCLOAK, DEVICES, DLIFSUP, DSHIELD, DSRSENS,
    IHDOCKED, IHRED, IHYELLOW, NDEVICE, SFAIR, SGOOD,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SavedEntry {
    pub name: String,
    pub data: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlaqueEntry {
    pub name: String,
    pub content: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DeviceStatus {
    pub id: usize,
    pub name: String,
    pub damage: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HudState {
    pub in_game: bool,
    pub alldone: bool,
    pub gamewon: bool,
    pub coord_fixed: bool,
    pub ship: String,
    pub condition: String,
    pub cloaked: bool,
    pub stardate: f64,
    pub time_left: f64,
    pub quadx: i32,
    pub quady: i32,
    pub sectx: i32,
    pub secty: i32,
    pub energy: f64,
    pub max_energy: f64,
    pub shield: f64,
    pub max_shield: f64,
    pub shields_up: bool,
    pub shield_damaged: bool,
    pub torps: i32,
    pub max_torps: i32,
    pub warp_factor: f64,
    pub life_support_active: bool,
    pub life_support_reserves: f64,
    pub klingons_left: i32,
    pub commanders_left: i32,
    pub super_commander_left: i32,
    pub romulans_left: i32,
    pub bases_left: i32,
    pub probes_left: i32,
    pub crystals_aboard: bool,
    pub in_orbit: bool,
    pub landed: bool,
    pub brig_captives: i32,
    pub brig_capacity: i32,
    pub kcaptured: i32,
    pub casualties: i32,
    pub quad_rows: Vec<String>,
    pub chart_rows: Vec<Vec<String>>,
    pub devices: Vec<DeviceStatus>,
}

impl HudState {
    pub fn from_state(state: &GameState) -> Self {
        let ship = if state.a.ship == '\0' {
            String::new()
        } else {
            state.a.ship.to_string()
        };
        let condition = match state.a.condit {
            IHDOCKED => String::from("DOCKED"),
            IHYELLOW => String::from("YELLOW"),
            IHRED => String::from("RED"),
            _ => String::from("GREEN"),
        };

        let good_scan = state.damage[DSRSENS] == 0.0 || state.a.condit == IHDOCKED;
        let mut quad_rows = Vec::with_capacity(10);
        if state.coordfixed {
            for y in (1..=10).rev() {
                let mut row = String::with_capacity(19);
                for x in 1..=10 {
                    if x > 1 {
                        row.push(' ');
                    }
                    if good_scan
                        || ((y as i32 - state.a.secty).abs() <= 1
                            && (x as i32 - state.a.sectx).abs() <= 1)
                    {
                        row.push(state.quad[x][y]);
                    } else {
                        row.push('-');
                    }
                }
                quad_rows.push(row);
            }
        } else {
            for x in 1..=10 {
                let mut row = String::with_capacity(19);
                for y in 1..=10 {
                    if y > 1 {
                        row.push(' ');
                    }
                    if good_scan
                        || ((x as i32 - state.a.sectx).abs() <= 1
                            && (y as i32 - state.a.secty).abs() <= 1)
                    {
                        row.push(state.quad[x][y]);
                    } else {
                        row.push('-');
                    }
                }
                quad_rows.push(row);
            }
        }

        let format_chart_cell = |s: i32, gx: usize, gy: usize| -> String {
            if s < 0 {
                String::from(".1.")
            } else if s == 0 {
                String::from("...")
            } else if s > 999 {
                (s - 1000).to_string()
            } else {
                state.d.galaxy[gx][gy].to_string()
            }
        };

        let mut chart_rows = Vec::with_capacity(8);
        if state.coordfixed {
            for j in (1..=8).rev() {
                let mut row = Vec::with_capacity(8);
                for i in 1..=8 {
                    row.push(format_chart_cell(state.starch[i][j], i, j));
                }
                chart_rows.push(row);
            }
        } else {
            for i in 1..=8 {
                let mut row = Vec::with_capacity(8);
                for j in 1..=8 {
                    row.push(format_chart_cell(state.starch[i][j], i, j));
                }
                chart_rows.push(row);
            }
        }

        let mut devices = Vec::with_capacity(NDEVICE);
        for id in 1..=NDEVICE {
            devices.push(DeviceStatus {
                id,
                name: String::from(DEVICES[id]),
                damage: state.damage[id],
            });
        }

        Self {
            in_game: state.in_game,
            alldone: state.a.alldone != 0,
            gamewon: state.a.gamewon != 0,
            coord_fixed: state.coordfixed,
            ship,
            condition,
            cloaked: state.a.iscloaked,
            stardate: state.d.date,
            time_left: state.d.remtime,
            quadx: state.a.quadx,
            quady: state.a.quady,
            sectx: state.a.sectx,
            secty: state.a.secty,
            energy: state.a.energy,
            max_energy: state.a.inenrg,
            shield: state.a.shield,
            max_shield: state.a.inshld,
            shields_up: state.a.shldup != 0,
            shield_damaged: state.damage[DSHIELD] != 0.0,
            torps: state.a.torps,
            max_torps: state.a.intorps,
            warp_factor: state.a.warpfac,
            life_support_active: state.damage[DLIFSUP] == 0.0,
            life_support_reserves: state.a.lsupres,
            klingons_left: state.d.remkl,
            commanders_left: state.d.remcom,
            super_commander_left: state.d.nscrem,
            romulans_left: state.d.nromrem,
            bases_left: state.d.rembase,
            probes_left: state.a.nprobes,
            crystals_aboard: state.a.icrystl != 0,
            in_orbit: state.a.inorbit != 0,
            landed: state.a.landed == 1,
            brig_captives: (state.a.brigcapacity - state.d.brigfree).max(0),
            brig_capacity: state.a.brigcapacity,
            kcaptured: state.d.kcaptured,
            casualties: state.a.casual,
            quad_rows,
            chart_rows,
            devices,
        }
    }

    pub fn push_json(&self, out: &mut String) {
        out.push('{');
        let _ = write!(
            out,
            "\"in_game\":{},\"alldone\":{},\"gamewon\":{},\"coord_fixed\":{},\"ship\":",
            self.in_game, self.alldone, self.gamewon, self.coord_fixed
        );
        json_escape(&self.ship, out);
        out.push_str(",\"condition\":");
        json_escape(&self.condition, out);
        let _ = write!(
            out,
            ",\"cloaked\":{},\"stardate\":{},\"time_left\":{},\"quadx\":{},\"quady\":{},\"sectx\":{},\"secty\":{},\"energy\":{},\"max_energy\":{},\"shield\":{},\"max_shield\":{},\"shields_up\":{},\"shield_damaged\":{},\"torps\":{},\"max_torps\":{},\"warp_factor\":{},\"life_support_active\":{},\"life_support_reserves\":{},\"klingons_left\":{},\"commanders_left\":{},\"super_commander_left\":{},\"romulans_left\":{},\"bases_left\":{},\"probes_left\":{},\"crystals_aboard\":{},\"in_orbit\":{},\"landed\":{},\"brig_captives\":{},\"brig_capacity\":{},\"kcaptured\":{},\"casualties\":{},\"quad_rows\":[",
            self.cloaked,
            fmt_json_f64(self.stardate),
            fmt_json_f64(self.time_left),
            self.quadx,
            self.quady,
            self.sectx,
            self.secty,
            fmt_json_f64(self.energy),
            fmt_json_f64(self.max_energy),
            fmt_json_f64(self.shield),
            fmt_json_f64(self.max_shield),
            self.shields_up,
            self.shield_damaged,
            self.torps,
            self.max_torps,
            fmt_json_f64(self.warp_factor),
            self.life_support_active,
            fmt_json_f64(self.life_support_reserves),
            self.klingons_left,
            self.commanders_left,
            self.super_commander_left,
            self.romulans_left,
            self.bases_left,
            self.probes_left,
            self.crystals_aboard,
            self.in_orbit,
            self.landed,
            self.brig_captives,
            self.brig_capacity,
            self.kcaptured,
            self.casualties
        );
        for (i, row) in self.quad_rows.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            json_escape(row, out);
        }
        out.push_str("],\"chart_rows\":[");
        for (i, row) in self.chart_rows.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push('[');
            for (j, cell) in row.iter().enumerate() {
                if j > 0 {
                    out.push(',');
                }
                json_escape(cell, out);
            }
            out.push(']');
        }
        out.push_str("],\"devices\":[");
        for (i, dev) in self.devices.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str("{\"id\":");
            let _ = write!(out, "{},\"name\":", dev.id);
            json_escape(&dev.name, out);
            let _ = write!(out, ",\"damage\":{}}}", fmt_json_f64(dev.damage));
        }
        out.push_str("]}");
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StepResponse {
    pub output: String,
    pub clear_screen: bool,
    pub prompt_type: PromptType,
    pub emexit: bool,
    pub hud: HudState,
    pub saves_updated: Vec<SavedEntry>,
    pub plaques_updated: Vec<PlaqueEntry>,
}

impl StepResponse {
    pub fn to_json(&self) -> String {
        let mut out = String::with_capacity(2048);
        out.push_str("{\"output\":");
        json_escape(&self.output, &mut out);
        let _ = write!(
            out,
            ",\"clear_screen\":{},\"prompt_type\":",
            self.clear_screen
        );
        json_escape(self.prompt_type.as_str(), &mut out);
        let _ = write!(out, ",\"emexit\":{},\"hud\":", self.emexit);
        self.hud.push_json(&mut out);
        out.push_str(",\"saves_updated\":[");
        for (i, entry) in self.saves_updated.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str("{\"name\":");
            json_escape(&entry.name, &mut out);
            out.push_str(",\"data\":");
            json_escape(&entry.data, &mut out);
            out.push('}');
        }
        out.push_str("],\"plaques_updated\":[");
        for (i, entry) in self.plaques_updated.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str("{\"name\":");
            json_escape(&entry.name, &mut out);
            out.push_str(",\"content\":");
            json_escape(&entry.content, &mut out);
            out.push('}');
        }
        out.push_str("]}");
        out
    }
}

pub async fn makemoves(state: &mut GameState, io: &IoHandle) {
    loop {
        let mut hitme = false;
        state.a.justin = 0;
        state.a.time = 0.0;

        let cmd_idx = loop {
            state.chew();
            skip(state, io, 1).await;
            state.proutn("COMMAND> ");
            if scan(state, io).await == ScanToken::Eol {
                continue;
            }
            let mut matched = None;
            for (i, &cmd) in COMMANDS.iter().enumerate().take(29) {
                if state.isit(cmd) {
                    matched = Some(i);
                    break;
                }
            }
            if matched.is_none() {
                for (i, &cmd) in COMMANDS.iter().enumerate().skip(29) {
                    if cmd == state.citem {
                        matched = Some(i);
                        break;
                    }
                }
            }
            if let Some(i) = matched {
                if i != 33 || state.allow_debug {
                    break i;
                }
            }
            if state.a.skill <= SFAIR {
                prout(state, io, "UNRECOGNIZED COMMAND. LEGAL COMMANDS ARE:").await;
                list_commands(state, io, true).await;
            } else {
                prout(state, io, "UNRECOGNIZED COMMAND.").await;
            }
        };

        match cmd_idx {
            0 => srscan(state, io, 1).await,
            1 => lrscan(state, io).await,
            2 => {
                phasers(state, io).await;
                if state.a.ididit != 0 {
                    if state.a.irhere != 0
                        && state.d.date >= ALGERON
                        && !state.a.isviolreported
                        && state.a.iscloaked
                    {
                        prout(
                            state,
                            io,
                            "The Romulan ship discovers you are breaking the Treaty of Algeron!",
                        )
                        .await;
                        state.a.ncviol += 1;
                        state.a.isviolreported = true;
                    }
                    hitme = true;
                }
            }
            3 => {
                photon(state, io).await;
                if state.a.ididit != 0 {
                    if state.a.irhere != 0
                        && state.d.date >= ALGERON
                        && !state.a.isviolreported
                        && state.a.iscloaked
                    {
                        prout(
                            state,
                            io,
                            "The Romulan ship discovers you are breaking the Treaty of Algeron!",
                        )
                        .await;
                        state.a.ncviol += 1;
                        state.a.isviolreported = true;
                    }
                    hitme = true;
                }
            }
            4 => warp(state, io, 1).await,
            5 => {
                sheild(state, io, 1).await;
                if state.a.ididit != 0 {
                    attack(state, io, 2).await;
                    state.a.shldchg = 0;
                }
            }
            6 => dock(state, io).await,
            7 => dreprt(state, io).await,
            8 => chart(state, io, false).await,
            9 => impuls(state, io).await,
            10 => {
                waiting(state, io).await;
                if state.a.ididit != 0 {
                    hitme = true;
                }
            }
            11 => setwrp(state, io).await,
            12 => srscan(state, io, 3).await,
            13 => sensor(state, io).await,
            14 => {
                orbit(state, io).await;
                if state.a.ididit != 0 {
                    hitme = true;
                }
            }
            15 => beam(state, io).await,
            16 => {
                mine(state, io).await;
                if state.a.ididit != 0 {
                    hitme = true;
                }
            }
            17 => usecrystals(state, io).await,
            18 => {
                shuttle(state, io).await;
                if state.a.ididit != 0 {
                    hitme = true;
                }
            }
            19 => preport(state, io).await,
            20 => srscan(state, io, 2).await,
            21 => report(state, io, false).await,
            22 => eta(state, io).await,
            23 => list_commands(state, io, true).await,
            24 => {
                state.clearscreen();
                freeze(state, io, true).await;
                state.emexit = true;
                state.in_game = false;
                return;
            }
            25 => probe(state, io).await,
            26 => {
                cloak(state, io).await;
                if state.a.iscloaking {
                    attack(state, io, 2).await;
                    state.a.iscloaking = false;
                    if state.damage[DCLOAK] == 0.0 {
                        state.a.iscloaked = true;
                    }
                }
            }
            27 => {
                capture(state, io).await;
                if state.a.ididit != 0 {
                    hitme = true;
                }
            }
            28 => score(state, io, true).await,
            29 => abandn(state, io).await,
            30 => dstrct(state, io).await,
            31 => {
                freeze(state, io, false).await;
                if state.a.skill > SGOOD {
                    prout(state, io, "WARNING--Frozen games produce no plaques!").await;
                }
            }
            32 => {
                deathray(state, io).await;
                if state.a.ididit != 0 {
                    hitme = true;
                }
            }
            33 => debugme(state, io).await,
            34 => help(state, io).await,
            35 => {
                state.a.alldone = 1;
                if state.debug_mode {
                    score(state, io, false).await;
                }
            }
            36 => helpme(state, io).await,
            _ => {}
        }

        loop {
            if state.a.alldone != 0 {
                break;
            }
            if state.debug_mode {
                prout(state, io, "2500").await;
            }
            if state.a.time != 0.0 {
                events(state, io).await;
                if state.a.alldone != 0 {
                    break;
                }
            }
            if state.d.galaxy[state.a.quadx as usize][state.a.quady as usize] == 1000 {
                atover(state, io, false).await;
                continue;
            }
            if state.a.nenhere == 0 {
                movetho(state, io).await;
            }
            if hitme && state.a.justin == 0 {
                attack(state, io, 2).await;
                if state.a.alldone != 0 {
                    break;
                }
                if state.d.galaxy[state.a.quadx as usize][state.a.quady as usize] == 1000 {
                    atover(state, io, false).await;
                    hitme = true;
                    continue;
                }
            }
            break;
        }
        if state.a.alldone != 0 {
            break;
        }
    }
    let _ = finish as fn(_, _, _) -> _;
}

pub async fn game_main(mut state: GameState, io: IoHandle) {
    prelim(&mut state, &io).await;
    loop {
        setup(&mut state, &io).await;
        if state.a.alldone != 0 {
            score(&mut state, &io, false).await;
            state.a.alldone = 0;
        } else {
            state.in_game = true;
            makemoves(&mut state, &io).await;
            state.in_game = false;
        }
        if state.emexit {
            io.park_finished(state);
            return;
        }
        skip(&mut state, &io, 2).await;
        stars(&mut state, &io).await;
        skip(&mut state, &io, 1).await;

        if state.a.tourn != 0 && state.a.alldone != 0 {
            state.proutn("Do you want your score recorded?");
            if ja(&mut state, &io).await {
                state.chew2();
                freeze(&mut state, &io, false).await;
            }
        }
        state.proutn("Do you want to play again?");
        if !ja(&mut state, &io).await {
            break;
        }
    }
    skip(&mut state, &io, 1).await;
    prout(
        &mut state,
        &io,
        "May the Great Bird of the Galaxy roost upon your home planet.",
    )
    .await;
    io.park_finished(state);
}

pub struct TrekSession {
    io: IoHandle,
    fut: Pin<Box<dyn Future<Output = ()>>>,
    last_response: StepResponse,
}

impl TrekSession {
    pub fn new(seed: u32, coord_fixed: bool, cmdline: &str) -> Self {
        let state = GameState::new(seed, coord_fixed, cmdline);
        let io = IoHandle::new();
        let fut = Box::pin(game_main(state, io.clone()));
        let placeholder = StepResponse {
            output: String::new(),
            clear_screen: false,
            prompt_type: PromptType::Line,
            emexit: false,
            hud: HudState::from_state(&GameState::default()),
            saves_updated: Vec::new(),
            plaques_updated: Vec::new(),
        };
        let mut session = Self {
            io,
            fut,
            last_response: placeholder,
        };
        session.poll_once();
        session
    }

    pub fn step(&mut self, input: &str, is_key: bool) -> &StepResponse {
        if self.prompt_type() == PromptType::Exited {
            self.last_response.output.clear();
            self.last_response.clear_screen = false;
            self.last_response.saves_updated.clear();
            self.last_response.plaques_updated.clear();
            return &self.last_response;
        }
        {
            let mut s = self.io.shared().borrow_mut();
            s.input = Some(if is_key {
                InputEvent::Key(input.chars().next().unwrap_or('\n'))
            } else {
                InputEvent::Line(input.to_string())
            });
        }
        self.poll_once();
        &self.last_response
    }

    fn poll_once(&mut self) {
        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);
        let poll = self.fut.as_mut().poll(&mut cx);
        let mut s = self.io.shared().borrow_mut();
        let prompt_type = match poll {
            Poll::Ready(()) => PromptType::Exited,
            Poll::Pending => s.waiting_for.unwrap_or(PromptType::Line),
        };
        if let Some(state) = s.parked_state.as_mut() {
            let output = std::mem::take(&mut state.output_buf);
            let clear_screen = state.clear_screen_flag;
            state.clear_screen_flag = false;
            let saves_updated = std::mem::take(&mut state.saves_updated)
                .into_iter()
                .map(|(name, data)| SavedEntry { name, data })
                .collect();
            let plaques_updated = std::mem::take(&mut state.plaques_updated)
                .into_iter()
                .map(|(name, content)| PlaqueEntry { name, content })
                .collect();
            self.last_response = StepResponse {
                output,
                clear_screen,
                prompt_type,
                emexit: state.emexit,
                hud: HudState::from_state(state),
                saves_updated,
                plaques_updated,
            };
        }
    }

    pub fn set_coord_fixed(&mut self, coord_fixed: bool) -> &StepResponse {
        let mut s = self.io.shared().borrow_mut();
        if let Some(state) = s.parked_state.as_mut() {
            state.coordfixed = coord_fixed;
            self.last_response.output.clear();
            self.last_response.clear_screen = false;
            self.last_response.saves_updated.clear();
            self.last_response.plaques_updated.clear();
            self.last_response.hud = HudState::from_state(state);
        }
        &self.last_response
    }

    pub fn import_save(&mut self, name: &str, data: &str) -> bool {
        if name.is_empty() {
            return false;
        }
        let mut s = self.io.shared().borrow_mut();
        if let Some(state) = s.parked_state.as_mut() {
            state
                .saved_games
                .insert(name.to_string(), data.to_string());
            true
        } else {
            false
        }
    }

    pub fn prompt_type(&self) -> PromptType {
        self.last_response.prompt_type
    }

    pub fn last_response(&self) -> &StepResponse {
        &self.last_response
    }

    pub fn hud(&self) -> &HudState {
        &self.last_response.hud
    }

    pub fn with_state<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&GameState) -> R,
    {
        let s = self.io.shared().borrow();
        s.parked_state.as_ref().map(f)
    }

    pub fn with_state_mut<F, R>(&mut self, f: F) -> Option<R>
    where
        F: FnOnce(&mut GameState) -> R,
    {
        let mut s = self.io.shared().borrow_mut();
        s.parked_state.as_mut().map(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner::parse_c_double_prefix;
    use crate::state::{DRADIO, DSRSENS, IHDOCKED, IHGREEN};
    use crate::wasm::{
        trek_alloc, trek_dealloc, trek_import_save, trek_init, trek_response_len,
        trek_response_ptr, trek_set_coord_fixed, trek_step,
    };

    #[test]
    fn test_tournament_1_short_novice_exact_rng() {
        let mut session = TrekSession::new(1, false, "");
        assert_eq!(session.prompt_type(), PromptType::Line);
        assert!(session
            .last_response()
            .output
            .contains("Would you like a regular, tournament, or frozen game?"));

        let r1 = session.step("tournament 1", false);
        assert!(r1.hud.stardate == 0.0);

        let r2 = session.step("short", false);
        assert_eq!(r2.prompt_type, PromptType::Line);

        let r3 = session.step("novice", false);
        assert_eq!(r3.prompt_type, PromptType::Line);

        let r4 = session.step("testpass", false);
        assert_eq!(r4.prompt_type, PromptType::Line);
        assert!(r4.output.contains("COMMAND> "));
        assert!(r4.hud.in_game);
        assert_eq!(r4.hud.stardate as i32, 3100);
        assert_eq!(r4.hud.klingons_left, 3);
        assert_eq!(r4.hud.bases_left, 2);
        assert_eq!(r4.hud.quadx, 2);
        assert_eq!(r4.hud.quady, 1);
        assert_eq!(r4.hud.sectx, 7);
        assert_eq!(r4.hud.secty, 7);
        assert!(r4.output.contains("7 - 7"));
        assert!(r4.output.contains("2 - 1"));
    }

    #[test]
    fn test_cmdline_startup_and_emexit() {
        let mut session = TrekSession::new(1, false, "tournament 1 short novice testpass");
        assert!(session.last_response().output.contains("COMMAND> "));
        assert_eq!(session.hud().stardate as i32, 3100);
        assert_eq!(session.hud().quadx, 2);
        assert_eq!(session.hud().quady, 1);

        let res = session.step("emexit", false);
        assert!(res.clear_screen);
        assert!(res.emexit);
        assert_eq!(res.prompt_type, PromptType::Exited);
        assert!(!res.hud.in_game);
        assert!(res.saves_updated.iter().any(|s| s.name == "emsave.trk"));
    }

    #[test]
    fn test_scanner_c_double_prefix() {
        assert_eq!(parse_c_double_prefix("3.5abc"), Some((3.5, 3)));
        assert_eq!(parse_c_double_prefix("-12.25e2xyz"), Some((-1225.0, 8)));
        assert_eq!(parse_c_double_prefix("+"), None);
        assert_eq!(parse_c_double_prefix("-"), None);
        assert_eq!(parse_c_double_prefix("."), None);
        assert_eq!(parse_c_double_prefix(".5x"), Some((0.5, 2)));
    }

    #[test]
    fn test_anticheat_hud_quad_and_chart() {
        let mut session = TrekSession::new(1, false, "tournament 1 short novice testpass");
        // Damage SR sensors while not docked; Enterprise is at (7, 7)
        session.with_state_mut(|st| {
            st.damage[DSRSENS] = 5.0;
            st.a.condit = IHGREEN;
            st.starch[1][1] = 0; // unvisited
            st.d.galaxy[1][1] = 999; // hidden value that must NOT leak
            st.starch[2][2] = 1004; // radio-damaged frozen value -> should show "4" even if galaxy changed
            st.d.galaxy[2][2] = 777;
        });
        let r = session.step("", false);
        // Sector (1, 1) is far from (7, 7), must be '-'
        assert_eq!(r.hud.quad_rows[0].chars().next(), Some('-'));
        // Sector (7, 7) is self -> 'E'
        assert_eq!(r.hud.quad_rows[6].split(' ').nth(6), Some("E"));
        // Chart (1, 1) is unvisited -> "..."
        assert_eq!(r.hud.chart_rows[0][0], "...");
        // Chart (2, 2) has starch = 1004 -> "4" (not 777!)
        assert_eq!(r.hud.chart_rows[1][1], "4");

        // When docked, SR sensors are assisted by starbase
        session.with_state_mut(|st| {
            st.a.condit = IHDOCKED;
            st.damage[DRADIO] = 0.0;
        });
        let r2 = session.set_coord_fixed(true);
        assert_ne!(r2.hud.quad_rows[0].chars().next(), Some('-'));
    }

    #[test]
    fn test_freeze_thaw_roundtrip() {
        let mut session = TrekSession::new(1, false, "tournament 1 short novice testpass");
        let r = session.step("freeze mygame", false);
        let save = r
            .saves_updated
            .iter()
            .find(|s| s.name == "mygame.trk")
            .expect("mygame.trk saved")
            .clone();

        let mut session2 = TrekSession::new(999, false, "");
        assert!(session2.import_save(&save.name, &save.data));
        let r2 = session2.step("frozen mygame", false);
        assert!(r2.output.contains("COMMAND> "));
        assert_eq!(r2.hud.stardate as i32, 3100);
        assert_eq!(r2.hud.quadx, 2);
        assert_eq!(r2.hud.quady, 1);
        assert_eq!(r2.hud.sectx, 7);
        assert_eq!(r2.hud.secty, 7);
    }

    #[test]
    fn test_help_lookup() {
        let mut session = TrekSession::new(1, false, "tournament 1 short novice testpass");
        let r = session.step("help srscan", false);
        assert!(r.output.contains("Mnemonic:  SRSCAN"));
        let r2 = session.step("help commands", false);
        assert!(r2.output.contains("ABBREV"));
    }

    #[test]
    fn test_wasm_abi_exports() {
        let cmd = b"tournament 1 short novice testpass";
        let ptr = trek_alloc(cmd.len());
        unsafe {
            std::ptr::copy_nonoverlapping(cmd.as_ptr(), ptr, cmd.len());
        }
        assert_eq!(trek_init(1, 0, ptr, cmd.len()), 1);
        trek_dealloc(ptr, cmd.len());

        let resp_bytes = unsafe {
            std::slice::from_raw_parts(trek_response_ptr(), trek_response_len())
        };
        let resp_str = std::str::from_utf8(resp_bytes).unwrap();
        assert!(resp_str.contains("\"quadx\":2"));
        assert!(resp_str.contains("\"quady\":1"));

        let step_cmd = b"status";
        assert_eq!(trek_step(step_cmd.as_ptr(), step_cmd.len(), 0), 1);
        let resp2 = unsafe {
            std::str::from_utf8(std::slice::from_raw_parts(
                trek_response_ptr(),
                trek_response_len(),
            ))
            .unwrap()
        };
        assert!(resp2.contains("Stardate"));

        assert_eq!(trek_set_coord_fixed(1), 1);
        assert_eq!(trek_import_save(b"x.trk".as_ptr(), 5, b"{}".as_ptr(), 2), 1);
    }
}
