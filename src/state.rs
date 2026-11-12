//! Core game state, constants, 1-based arrays, and synchronous helpers.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::rng::MacOsRng;

pub const NDEVICE: usize = 16;
pub const PHASEFAC: f64 = 2.0;
pub const PLNETMAX: usize = 10;
pub const NEVENTS: usize = 8;
pub const ALGERON: f64 = 2311.0;

// Device indices (1-based)
pub const DSRSENS: usize = 1;
pub const DLRSENS: usize = 2;
pub const DPHASER: usize = 3;
pub const DPHOTON: usize = 4;
pub const DLIFSUP: usize = 5;
pub const DWARPEN: usize = 6;
pub const DIMPULS: usize = 7;
pub const DSHIELD: usize = 8;
pub const DRADIO: usize = 9;
pub const DSHUTTL: usize = 10;
pub const DCOMPTR: usize = 11;
pub const DTRANSP: usize = 12;
pub const DSHCTRL: usize = 13;
pub const DDRAY: usize = 14;
pub const DDSP: usize = 15;
pub const DCLOAK: usize = 16;

pub const DEVICES: [&str; NDEVICE + 1] = [
    "",
    "S. R. Sensors",
    "L. R. Sensors",
    "Phasers",
    "Photon Tubes",
    "Life Support",
    "Warp Engines",
    "Impulse Engines",
    "Shields",
    "Subspace Radio",
    "Shuttle Craft",
    "Computer",
    "Transporter",
    "Shield Control",
    "Death Ray",
    "D. S. Probe",
    "Cloaking Device",
];

// Future event indices
pub const FSPY: usize = 0;
pub const FSNOVA: usize = 1;
pub const FTBEAM: usize = 2;
pub const FSNAP: usize = 3;
pub const FBATTAK: usize = 4;
pub const FCDBAS: usize = 5;
pub const FSCMOVE: usize = 6;
pub const FSCDBAS: usize = 7;
pub const FDSPROB: usize = 8;

// Quadrant / ship / condition characters
pub const IHR: char = 'R';
pub const IHK: char = 'K';
pub const IHC: char = 'C';
pub const IHS: char = 'S';
pub const IHSTAR: char = '*';
pub const IHP: char = 'P';
pub const IHB: char = 'B';
pub const IHBLANK: char = ' ';
pub const IHDOT: char = '.';
pub const IHQUEST: char = '?';
pub const IHE: char = 'E';
pub const IHF: char = 'F';
pub const IHT: char = 'T';
pub const IHWEB: char = '#';
pub const IHGREEN: char = 'G';
pub const IHYELLOW: char = 'Y';
pub const IHRED: char = 'R';
pub const IHDOCKED: char = 'D';

// Skill levels
pub const SNOVICE: i32 = 1;
pub const SFAIR: i32 = 2;
pub const SGOOD: i32 = 3;
pub const SEXPERT: i32 = 4;
pub const SEMERITUS: i32 = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FinType {
    Won,
    Deplete,
    Lifesup,
    Nrg,
    Battle,
    Neg3,
    Nova,
    Snovaed,
    Abandn,
    Dilithium,
    Materialize,
    Phaser,
    Lost,
    Mining,
    Dplanet,
    Pnova,
    Ssc,
    Stractor,
    Dray,
    Tribble,
    Hole,
    Cloak,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Planet {
    pub x: i32,
    pub y: i32,
    pub pclass: i32,
    pub crystals: i32,
    pub known: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GalaxyData {
    pub snap: i32,
    pub remkl: i32,
    pub remcom: i32,
    pub rembase: i32,
    pub starkl: i32,
    pub basekl: i32,
    pub killk: i32,
    pub killc: i32,
    pub galaxy: [[i32; 9]; 9],
    pub cx: [i32; 11],
    pub cy: [i32; 11],
    pub baseqx: [i32; 6],
    pub baseqy: [i32; 6],
    pub newstuf: [[i32; 9]; 9],
    pub isx: i32,
    pub isy: i32,
    pub nscrem: i32,
    pub nromkl: i32,
    pub nromrem: i32,
    pub nsckill: i32,
    pub nplankl: i32,
    pub plnets: [Planet; 11],
    pub kcaptured: i32,
    pub brigfree: i32,
    pub date: f64,
    pub remres: f64,
    pub remtime: f64,
}

impl Default for GalaxyData {
    fn default() -> Self {
        Self {
            snap: 0,
            remkl: 0,
            remcom: 0,
            rembase: 0,
            starkl: 0,
            basekl: 0,
            killk: 0,
            killc: 0,
            galaxy: [[0; 9]; 9],
            cx: [0; 11],
            cy: [0; 11],
            baseqx: [0; 6],
            baseqy: [0; 6],
            newstuf: [[0; 9]; 9],
            isx: 0,
            isy: 0,
            nscrem: 0,
            nromkl: 0,
            nromrem: 0,
            nsckill: 0,
            nplankl: 0,
            plnets: [Planet::default(); 11],
            kcaptured: 0,
            brigfree: 0,
            date: 0.0,
            remres: 0.0,
            remtime: 0.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShipVars {
    pub inkling: i32,
    pub inbase: i32,
    pub incom: i32,
    pub instar: i32,
    pub intorps: i32,
    pub condit: char,
    pub torps: i32,
    pub ship: char,
    pub quadx: i32,
    pub quady: i32,
    pub sectx: i32,
    pub secty: i32,
    pub length: i32,
    pub skill: i32,
    pub basex: i32,
    pub basey: i32,
    pub klhere: i32,
    pub comhere: i32,
    pub casual: i32,
    pub nhelp: i32,
    pub nkinks: i32,
    pub ididit: i32,
    pub gamewon: i32,
    pub alive: i32,
    pub justin: i32,
    pub alldone: i32,
    pub shldchg: i32,
    pub thingx: i32,
    pub thingy: i32,
    pub plnetx: i32,
    pub plnety: i32,
    pub inorbit: i32,
    pub landed: i32,
    pub iplnet: i32,
    pub imine: i32,
    pub inplan: i32,
    pub nenhere: i32,
    pub ishere: i32,
    pub neutz: i32,
    pub irhere: i32,
    pub icraft: i32,
    pub ientesc: i32,
    pub iscraft: i32,
    pub isatb: i32,
    pub iscate: i32,
    pub iscloaked: bool,
    pub iscloaking: bool,
    pub ncviol: i32,
    pub isviolreported: bool,
    pub brigcapacity: i32,
    pub iattak: i32,
    pub icrystl: i32,
    pub tourn: i32,
    pub thawed: i32,
    pub batx: i32,
    pub baty: i32,
    pub ithere: i32,
    pub ithx: i32,
    pub ithy: i32,
    pub iseenit: i32,
    pub probecx: i32,
    pub probecy: i32,
    pub proben: i32,
    pub isarmed: bool,
    pub nprobes: i32,

    pub inresor: f64,
    pub intime: f64,
    pub inenrg: f64,
    pub inshld: f64,
    pub inlsr: f64,
    pub indate: f64,
    pub energy: f64,
    pub shield: f64,
    pub shldup: i32,
    pub warpfac: f64,
    pub wfacsq: f64,
    pub lsupres: f64,
    pub dist: f64,
    pub direc: f64,
    pub time: f64,
    pub docfac: f64,
    pub resting: i32,
    pub damfac: f64,
    pub stdamtim: f64,
    pub cryprob: f64,
    pub probex: f64,
    pub probey: f64,
    pub probeinx: f64,
    pub probeiny: f64,
}

impl Default for ShipVars {
    fn default() -> Self {
        Self {
            inkling: 0,
            inbase: 0,
            incom: 0,
            instar: 0,
            intorps: 0,
            condit: IHGREEN,
            torps: 0,
            ship: '\0',
            quadx: 0,
            quady: 0,
            sectx: 0,
            secty: 0,
            length: 0,
            skill: 0,
            basex: 0,
            basey: 0,
            klhere: 0,
            comhere: 0,
            casual: 0,
            nhelp: 0,
            nkinks: 0,
            ididit: 0,
            gamewon: 0,
            alive: 0,
            justin: 0,
            alldone: 0,
            shldchg: 0,
            thingx: 0,
            thingy: 0,
            plnetx: 0,
            plnety: 0,
            inorbit: 0,
            landed: 0,
            iplnet: 0,
            imine: 0,
            inplan: 0,
            nenhere: 0,
            ishere: 0,
            neutz: 0,
            irhere: 0,
            icraft: 0,
            ientesc: 0,
            iscraft: 0,
            isatb: 0,
            iscate: 0,
            iscloaked: false,
            iscloaking: false,
            ncviol: 0,
            isviolreported: false,
            brigcapacity: 0,
            iattak: 0,
            icrystl: 0,
            tourn: 0,
            thawed: 0,
            batx: 0,
            baty: 0,
            ithere: 0,
            ithx: 0,
            ithy: 0,
            iseenit: 0,
            probecx: 0,
            probecy: 0,
            proben: 0,
            isarmed: false,
            nprobes: 0,
            inresor: 0.0,
            intime: 0.0,
            inenrg: 0.0,
            inshld: 0.0,
            inlsr: 0.0,
            indate: 0.0,
            energy: 0.0,
            shield: 0.0,
            shldup: 0,
            warpfac: 0.0,
            wfacsq: 0.0,
            lsupres: 0.0,
            dist: 0.0,
            direc: 0.0,
            time: 0.0,
            docfac: 0.0,
            resting: 0,
            damfac: 0.0,
            stdamtim: 1e30,
            cryprob: 0.0,
            probex: 0.0,
            probey: 0.0,
            probeinx: 0.0,
            probeiny: 0.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct GameState {
    pub d: GalaxyData,
    pub snapsht: GalaxyData,
    pub quad: [[char; 11]; 11],
    pub a: ShipVars,
    pub kx: [i32; 21],
    pub ky: [i32; 21],
    pub starch: [[i32; 9]; 9],
    pub kpower: [f64; 21],
    pub kdist: [f64; 21],
    pub kavgd: [f64; 21],
    pub damage: [f64; 17],
    pub future: [f64; 9],
    pub passwd: String,
    pub fromcommandline: bool,
    pub coordfixed: bool,
    pub iscore: i32,
    pub iskill: i32,
    pub perdate: f64,
    pub height: i32,

    // Scanner state
    pub aaitem: f64,
    pub citem: String,
    pub line: String,
    pub line_pos: usize,
    pub consumed_on_line: bool,

    // I/O and session state
    pub output_buf: String,
    pub linecount: i32,
    pub clear_screen_flag: bool,
    pub auto_page: bool,
    pub skip_pauses: bool,

    // RNG
    pub rng: MacOsRng,
    pub initial_seed: u32,

    // Debug & session control
    pub debug_mode: bool,
    pub allow_debug: bool,
    pub in_game: bool,
    pub emexit: bool,
    pub exited: bool,

    // Virtual filesystem for saves & plaques
    pub saved_games: BTreeMap<String, String>,
    pub saves_updated: Vec<(String, String)>,
    pub plaques: BTreeMap<String, String>,
    pub plaques_updated: Vec<(String, String)>,
    pub plaque_timestamp: String,
}

impl Default for GameState {
    fn default() -> Self {
        Self::new(1, false, "")
    }
}

impl GameState {
    pub fn new(seed: u32, coord_fixed: bool, cmdline: &str) -> Self {
        let trimmed = cmdline.trim();
        let (fromcommandline, line) = if trimmed.is_empty() {
            (false, String::new())
        } else {
            (true, format!("{} ", trimmed))
        };
        Self {
            d: GalaxyData::default(),
            snapsht: GalaxyData::default(),
            quad: [[IHDOT; 11]; 11],
            a: ShipVars::default(),
            kx: [0; 21],
            ky: [0; 21],
            starch: [[0; 9]; 9],
            kpower: [0.0; 21],
            kdist: [0.0; 21],
            kavgd: [0.0; 21],
            damage: [0.0; 17],
            future: [1e30; 9],
            passwd: String::new(),
            fromcommandline,
            coordfixed: coord_fixed,
            iscore: 0,
            iskill: 0,
            perdate: 0.0,
            height: 0,
            aaitem: 0.0,
            citem: String::new(),
            line,
            line_pos: 0,
            consumed_on_line: false,
            output_buf: String::new(),
            linecount: 0,
            clear_screen_flag: false,
            auto_page: false,
            skip_pauses: false,
            rng: MacOsRng::new(seed),
            initial_seed: seed,
            debug_mode: false,
            allow_debug: false,
            in_game: false,
            emexit: false,
            exited: false,
            saved_games: BTreeMap::new(),
            saves_updated: Vec::new(),
            plaques: BTreeMap::new(),
            plaques_updated: Vec::new(),
            plaque_timestamp: String::from("Jan  1 2026, 00:00:00"),
        }
    }

    /// Lightweight zero-allocation placeholder for temporarily swapping out `GameState` across yields.
    pub(crate) fn empty_placeholder() -> Self {
        Self {
            d: GalaxyData::default(),
            snapsht: GalaxyData::default(),
            quad: [[IHDOT; 11]; 11],
            a: ShipVars::default(),
            kx: [0; 21],
            ky: [0; 21],
            starch: [[0; 9]; 9],
            kpower: [0.0; 21],
            kdist: [0.0; 21],
            kavgd: [0.0; 21],
            damage: [0.0; 17],
            future: [0.0; 9],
            passwd: String::new(),
            fromcommandline: false,
            coordfixed: false,
            iscore: 0,
            iskill: 0,
            perdate: 0.0,
            height: 0,
            aaitem: 0.0,
            citem: String::new(),
            line: String::new(),
            line_pos: 0,
            consumed_on_line: false,
            output_buf: String::new(),
            linecount: 0,
            clear_screen_flag: false,
            auto_page: false,
            skip_pauses: false,
            rng: MacOsRng::default(),
            initial_seed: 1,
            debug_mode: false,
            allow_debug: false,
            in_game: false,
            emexit: false,
            exited: false,
            saved_games: BTreeMap::new(),
            saves_updated: Vec::new(),
            plaques: BTreeMap::new(),
            plaques_updated: Vec::new(),
            plaque_timestamp: String::new(),
        }
    }

    /// Re-seeds the RNG using `initial_seed` (for `regular` or `frozen` games).
    pub fn randomize(&mut self) {
        self.rng.srand(self.initial_seed);
        self.initial_seed = self
            .initial_seed
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
    }

    pub fn reports(&self) -> bool {
        (self.a.condit == IHDOCKED || self.damage[DRADIO] <= 0.0) && !self.a.iscloaked
    }

    pub fn clearscreen(&mut self) {
        self.clear_screen_flag = true;
    }

    pub fn proutn(&mut self, s: &str) {
        self.output_buf.push_str(s);
    }

    pub fn chew(&mut self) {
        self.linecount = 0;
        self.line.clear();
        self.line_pos = 0;
        self.consumed_on_line = false;
    }

    pub fn chew2(&mut self) {
        self.linecount = 0;
        self.line.clear();
        self.line_pos = 0;
        self.consumed_on_line = true;
    }

    pub fn isit(&self, cmd: &str) -> bool {
        if self.citem.is_empty() {
            return cmd.is_empty();
        }
        let n = self.citem.len();
        cmd.len() >= n && cmd.as_bytes()[..n] == self.citem.as_bytes()[..n]
    }

    pub fn cramf(&mut self, x: f64, w: i32, d: i32) {
        let width = w.max(0) as usize;
        let prec = d.max(0) as usize;
        let _ = write!(self.output_buf, "{:>width$.prec$}", x, width = width, prec = prec);
    }

    pub fn crami(&mut self, i: i32, w: i32) {
        let width = w.max(0) as usize;
        let _ = write!(self.output_buf, "{:>width$}", i, width = width);
    }

    pub fn cramen(&mut self, i: char) {
        let s = match i {
            IHR => "Romulan",
            IHK => "Klingon",
            IHC => "Commander",
            IHS => "Super-commander",
            IHSTAR => "Star",
            IHP => "Planet",
            IHB => "Starbase",
            IHBLANK => "Black hole",
            IHT => "Tholian",
            IHWEB => "Tholian web",
            _ => "Unknown??",
        };
        self.proutn(s);
    }

    pub fn cramlc(&mut self, key: i32, x: i32, y: i32) {
        if key == 1 {
            self.proutn(" Quadrant");
        } else if key == 2 {
            self.proutn(" Sector");
        }
        self.proutn(" ");
        self.crami(x, 1);
        self.proutn(" - ");
        self.crami(y, 1);
    }

    pub fn crmena(&mut self, i: i32, enemy: char, key: i32, x: i32, y: i32) {
        if i == 1 {
            self.proutn("***");
        }
        self.cramen(enemy);
        self.proutn(" at");
        self.cramlc(key, x, y);
    }

    pub fn crmshp(&mut self) {
        let s = match self.a.ship {
            IHE => "Enterprise",
            IHF => "Faerie Queene",
            _ => "Ship???",
        };
        self.proutn(s);
    }

    pub fn dropin(&mut self, iquad: char) -> (i32, i32) {
        loop {
            let (ix, iy) = self.rng.iran10();
            if self.quad[ix as usize][iy as usize] == IHDOT {
                self.quad[ix as usize][iy as usize] = iquad;
                return (ix, iy);
            }
        }
    }

    pub fn newcnd(&mut self) {
        self.a.condit = IHGREEN;
        if self.a.energy < 1000.0 {
            self.a.condit = IHYELLOW;
        }
        let qx = self.a.quadx as usize;
        let qy = self.a.quady as usize;
        if (1..=8).contains(&qx)
            && (1..=8).contains(&qy)
            && (self.d.galaxy[qx][qy] > 99 || self.d.newstuf[qx][qy] > 9)
        {
            self.a.condit = IHRED;
        }
    }

    pub fn sortkl(&mut self) {
        let n = self.a.nenhere as usize;
        if n < 2 {
            return;
        }
        loop {
            let mut sw = false;
            for j in 1..n {
                if self.kdist[j] > self.kdist[j + 1] {
                    sw = true;
                    self.kdist.swap(j, j + 1);
                    self.kavgd.swap(j, j + 1);
                    self.kx.swap(j, j + 1);
                    self.ky.swap(j, j + 1);
                    self.kpower.swap(j, j + 1);
                }
            }
            if !sw {
                break;
            }
        }
    }
}

pub fn square(x: f64) -> f64 {
    x * x
}

pub fn json_escape(s: &str, out: &mut String) {
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0C}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

pub fn fmt_json_f64(v: f64) -> String {
    if !v.is_finite() {
        return String::from("0.0");
    }
    let s = format!("{:?}", v);
    if s == "-0.0" {
        String::from("0.0")
    } else {
        s
    }
}
