//! Super Star Trek (Tom Almy's Mac OS edition) converted from ANSI C to safe Rust.

pub mod ai;
pub mod battle;
pub mod events;
pub mod finish;
pub mod help;
pub mod io;
pub mod moving;
pub mod planets;
pub mod reports;
pub mod rng;
pub mod scanner;
pub mod session;
pub mod setup;
pub mod state;
pub mod wasm;

pub use crate::help::COMMANDS;
pub use crate::io::{huh, pause, prout, prouts, skip, stars, InputEvent, IoHandle, PromptType};
pub use crate::rng::MacOsRng;
pub use crate::scanner::{ja, parse_c_double_prefix, scan, ScanToken};
pub use crate::session::{
    game_main, makemoves, DeviceStatus, HudState, PlaqueEntry, SavedEntry, StepResponse,
    TrekSession,
};
pub use crate::setup::{deserialize_freeze, serialize_freeze};
pub use crate::state::{
    fmt_json_f64, json_escape, square, FinType, GalaxyData, GameState, Planet, ShipVars, ALGERON,
    DCLOAK, DCOMPTR, DDRAY, DDSP, DEVICES, DIMPULS, DLIFSUP, DLRSENS, DPHASER, DPHOTON, DRADIO,
    DSHCTRL, DSHIELD, DSHUTTL, DSRSENS, DTRANSP, DWARPEN, FBATTAK, FCDBAS, FDSPROB, FSCDBAS,
    FSCMOVE, FSNAP, FSNOVA, FSPY, FTBEAM, IHB, IHBLANK, IHC, IHDOCKED, IHDOT, IHE, IHF, IHGREEN,
    IHK, IHP, IHQUEST, IHR, IHRED, IHS, IHSTAR, IHT, IHWEB, IHYELLOW, NDEVICE, NEVENTS, PHASEFAC,
    PLNETMAX, SEMERITUS, SEXPERT, SFAIR, SGOOD, SNOVICE,
};
