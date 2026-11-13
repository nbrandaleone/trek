# Trek — Super Star Trek (Rust + WebAssembly)

![Super Star Trek](docs/super-star-trek.png)

**Trek** is a zero-dependency Rust and WebAssembly port of Tom Almy's *Super Star Trek for Mac OS* (`https://almy.us/sst.html`), featuring an interactive browser terminal, live Bridge Telemetry HUD, and an embedded pure-`std` HTTP server.

## Overview & Attribution

This project converts the classic ANSI C codebase of *Super Star Trek for Mac OS* (preserved in [`c_src/`](file:///Users/nbrand/projects/trek/c_src)) into safe, modular Rust and exposes it to the browser via WebAssembly:

- **Source Template**: Tom Almy's *Super Star Trek for Mac OS* (`SCORE`, `CAPTURE`, and `CLOAKING` options enabled).
- **Zero External Crates**: Both the `trek` library (`cdylib` + `rlib`) and the `trek` HTTP server binary compile using only the Rust standard library (`std`).

## Architecture Highlights

- **Zero-Dependency Rust Game Engine ([`src/`](file:///Users/nbrand/projects/trek/src))**: Faithful port of all C modules—universe setup, reports/scanners, navigation, phasers/torpedoes/combat, random events, Klingon/Romulan/Tholian AI, planets/mining/crystals, endgame scoring, and help.
- **Step-Driven Execution ([`src/session.rs`](file:///Users/nbrand/projects/trek/src/session.rs))**: Uses a single-threaded `async`/`await` generator (`TrekSession`) built on `std::task` so the blocking prompt-driven C game flow pauses cleanly whenever console input or a pause key is needed, without rewriting the game loop as a manual state machine or spawning threads in WASM.
- **Deterministic macOS RNG ([`src/rng.rs`](file:///Users/nbrand/projects/trek/src/rng.rs))**: Implements the 31-bit Park–Miller generator (`MacOsRng`, matching `osx.c` `Rand()`) alongside C-compatible tournament seeding so tournament seeds match expected layouts and behavior.
- **WebAssembly Bridge ([`src/wasm.rs`](file:///Users/nbrand/projects/trek/src/wasm.rs))**: Exposes a C-ABI bridge compiled into `web/trek.wasm` for session initialization, input feeding, freeze/thaw save-file serialization (`.trk`), and live telemetry JSON snapshots.
- **Web Interface ([`web/`](file:///Users/nbrand/projects/trek/web))**: Browser terminal (`index.html`, `styles.css`, `app.js`) with a live Bridge Telemetry HUD (stardate, shields, energy, torpedoes, ship status, crew/time remaining), quick-command helpers, freeze/thaw `.trk` file support, and an integrated `sst.doc` manual viewer.
- **Pure-`std` HTTP Server ([`src/bin/server.rs`](file:///Users/nbrand/projects/trek/src/bin/server.rs))**: Lightweight TCP HTTP server serving `./web` on `127.0.0.1:8080` (configurable via `--port <N>` or `PORT`).

### Architecture Diagram

```mermaid
flowchart LR
    subgraph ClientTier["Tier 1: Server and Browser UI"]
        Server["src/bin/server.rs - Pure-std TCP HTTP Server (:8080)"]
        WebUI["web/index.html + styles.css - Bridge Console and Dialogs"]
        AppJs["web/app.js - WASM Host, HUD Renderer, localStorage"]
        WebDoc["web/sst.doc - Reference Manual"]
    end

    subgraph RuntimeTier["Tier 2: WASM Bridge and Session"]
        WasmRs["src/wasm.rs - C-ABI (trek_init, trek_step, trek_set_coord_fixed, trek_import_save)"]
        TrekSession["src/session.rs::TrekSession - Single-Turn Future Poller (Waker::noop)"]
        GameMain["game_main + makemoves - Lifecycle and 37-Command Dispatcher"]
        StepResp["StepResponse + HudState - JSON Delta and Anti-Cheat Telemetry"]
    end

    subgraph EngineTier["Tier 3: Gameplay Engine Modules (src/)"]
        Setup["setup.rs: prelim, setup, newqad, abandn, freeze/thaw (.trk)"]
        Reports["reports.rs: srscan, lrscan, chart, report, dreprt, eta"]
        Moving["moving.rs: warp, impuls, lmove, dock, setwrp, atover, timwrp, probe, help"]
        Battle["battle.rs: phasers, photon, torpedo, sheild, cloak, capture, attack, deadkl, ram"]
        Events["events.rs: events, waiting, nova, snova"]
        Ai["ai.rs: movcom, scom, movetho"]
        Planets["planets.rs: orbit, sensor, beam, mine, usecrystals, shuttle, preport, deathray"]
        Finish["finish.rs: finish, score, 132-col plaque, dstrct, kaboom"]
        Help["help.rs + assets/sst.doc: COMMANDS, list_commands, helpme, debugme"]
    end

    subgraph CoreTier["Tier 4: Shared Foundation (src/)"]
        Scanner["scanner.rs - scan, ja, parse_c_double_prefix, ScanToken"]
        Io["io.rs - IoHandle, PromptType, InputEvent, prout, pause, skip"]
        State["state.rs - GameState, GalaxyData, ShipVars, Planet, FinType"]
        Rng["rng.rs - MacOsRng (31-bit Park-Miller PRNG)"]
    end

    Server -->|"Static UI and manual"| WebUI
    Server -->|"Manual"| WebDoc
    Server -->|"Serves web/trek.wasm"| WasmRs
    WebUI --> AppJs
    AppJs <-->|"UTF-8 buffers and JSON"| WasmRs
    WasmRs --> TrekSession
    TrekSession -->|"Polls Future"| GameMain
    TrekSession -->|"Serializes"| StepResp
    StepResp -->|"Reads telemetry"| State

    GameMain --> Setup & Reports & Moving & Battle & Events & Ai & Planets & Finish & Help
    EngineTier -->|"Tokenizes input"| Scanner
    EngineTier -->|"Console I/O and suspension"| Io
    EngineTier -->|"Reads and mutates"| State
    Scanner --> Io
    Io --> State
    State --> Rng
```

See the full diagram suite—including the inter-module call graph, step/coroutine sequence diagram, core data model, and turn-execution state machine—in [`docs/architecture.md`](file:///Users/nbrand/projects/trek/docs/architecture.md), [`docs/architecture.mmd`](file:///Users/nbrand/projects/trek/docs/architecture.mmd), and [`docs/architecture.pdf`](file:///Users/nbrand/projects/trek/docs/architecture.pdf).

## Prerequisites

- **Rust toolchain** (`rustc` and `cargo`)
- **`wasm32-unknown-unknown` target**:
  ```sh
  rustup target add wasm32-unknown-unknown
  ```
- **GNU Make** (`make`)
- **Node.js** (`node`, optional — used by `make check` to syntax-check `web/app.js`)

## Makefile Tasks

Use the provided [`Makefile`](file:///Users/nbrand/projects/trek/Makefile) for common workflows:

| Command | Description |
| --- | --- |
| `make build` (or `make all`) | Build `web/trek.wasm` and the release `trek` HTTP server binary |
| `make start` (or `make run`) | Build WASM and start the web server at `http://127.0.0.1:8080` |
| `make start PORT=3000` | Build WASM and start the web server on a custom port |
| `make wasm` | Compile `web/trek.wasm` via [`scripts/build_web.sh`](file:///Users/nbrand/projects/trek/scripts/build_web.sh) |
| `make test` (or `make unit-test`) | Run all Cargo unit, integration, and regression tests |
| `make check` | Run `cargo check` (native + `wasm32-unknown-unknown`) and `node --check web/app.js` |
| `make clean` | Clean Cargo build artifacts and remove `web/trek.wasm` |
| `make help` | Print available Makefile targets |

## Gameplay Quick-Reference

- **Game Types**:
  - **Regular**: Random galaxy seed.
  - **Tournament**: Deterministic galaxy using a numbered tournament seed (e.g., `1`).
  - **Frozen**: Restore a previously saved `.trk` freeze file.
- **Skill Levels & Lengths**:
  - **Lengths**: Short, Medium, Long.
  - **Skill Levels**: Novice, Fair, Good, Expert, Emeritus.
- **Key Commands**:
  - `SRSCAN` / `LRSCAN` / `CHART` / `STATUS` / `DAMAGES` — Sensors, star chart, ship status, and damage report.
  - `MOVE` / `IMPULSE` / `WARP` — Navigation and warp factor selection.
  - `PHASERS` / `PHOTONS` / `SHIELDS` — Combat and defensive systems.
  - `DOCK` / `REST` / `ORBIT` / `SENSORS` / `MINE` / `CRYSTALS` / `SHUTTLE` / `TRANSPORT` / `PROBE` — Starbase docking, planetary exploration, and special equipment.
  - `CLOAK` / `CAPTURE` / `SCORE` — Cloaking device, Klingon capture, and mission score check.
  - `FREEZE` — Save the current game as a `.trk` freeze file.
  - `EMEXIT` — Emergency exit (freezes as `emsave.trk` and exits cleanly without scoring/plaque eligibility).
  - `HELP` / `COMMANDS` — In-game help and command listing (full manual available via `sst.doc`).
- **Coordinate Mode (`-f`)**: Toggle **-f Horizontal X-Axis** in the web header to switch between Standard `(Row, Col)` coordinates and Fixed X-Axis `(X, Y)` coordinates; clicking any sector cell in the HUD inserts its coordinates into the command prompt.
- **Endgame Plaque**: Completing a mission awards a score breakdown, and winning at the Emeritus skill level generates a classic 132-column Commodore Emeritus Citation Plaque (downloadable or printable from the UI; frozen games do not produce plaques).

## Directory Layout

```text
trek/
├── Makefile                 # Common build, start, test, check, and clean targets
├── Cargo.toml               # Crate manifest (cdylib + rlib + trek server bin)
├── README.md                # Project documentation
├── BUG_REPORT.md            # QA bug findings and verified fixes
├── c_src/                   # Original Tom Almy Super Star Trek ANSI C sources
├── assets/                  # Reference documentation (sst.doc)
├── docs/                    # Architecture documentation and diagrams
│   ├── architecture.md      # All 5 Mermaid architecture and state diagrams
│   ├── architecture.mmd     # Standalone Mermaid flowchart source
│   └── architecture.pdf     # Vector landscape PDF export of all diagrams
├── scripts/
│   └── build_web.sh         # Builds wasm32-unknown-unknown release & copies web/trek.wasm
├── src/                     # Rust game engine, WASM bridge, and HTTP server
│   ├── lib.rs
│   ├── state.rs
│   ├── rng.rs
│   ├── io.rs
│   ├── scanner.rs
│   ├── session.rs
│   ├── wasm.rs
│   ├── setup.rs
│   ├── reports.rs
│   ├── moving.rs
│   ├── battle.rs
│   ├── events.rs
│   ├── ai.rs
│   ├── planets.rs
│   ├── finish.rs
│   ├── help.rs
│   └── bin/server.rs
├── tests/                   # Integration and regression test suites
│   ├── rng_and_scanner_tests.rs
│   ├── setup_and_reports_tests.rs
│   ├── movement_and_navigation_tests.rs
│   ├── battle_and_combat_tests.rs
│   ├── events_and_ai_tests.rs
│   ├── planets_and_special_tests.rs
│   └── regression_bugs_tests.rs
└── web/                     # Browser UI, manual, and compiled WebAssembly module
    ├── index.html
    ├── styles.css
    ├── app.js
    ├── sst.doc
    └── trek.wasm
```

## Testing & Multi-Agent QA

The project was built and verified with dedicated conversion, gameplay QA, and test-authoring passes:

- **Bug Tracking**: Issues uncovered during testing and their fixes are documented in [`BUG_REPORT.md`](file:///Users/nbrand/projects/trek/BUG_REPORT.md).
- **Test Suite (`cargo test --no-fail-fast`)**: **70 tests** covering unit internals (`src/lib.rs`) plus integration and regression suites across `tests/`:
  - RNG & scanners ([`tests/rng_and_scanner_tests.rs`](file:///Users/nbrand/projects/trek/tests/rng_and_scanner_tests.rs))
  - Setup & reports ([`tests/setup_and_reports_tests.rs`](file:///Users/nbrand/projects/trek/tests/setup_and_reports_tests.rs))
  - Movement & navigation ([`tests/movement_and_navigation_tests.rs`](file:///Users/nbrand/projects/trek/tests/movement_and_navigation_tests.rs))
  - Battle & combat ([`tests/battle_and_combat_tests.rs`](file:///Users/nbrand/projects/trek/tests/battle_and_combat_tests.rs))
  - Events & AI ([`tests/events_and_ai_tests.rs`](file:///Users/nbrand/projects/trek/tests/events_and_ai_tests.rs))
  - Planets & special commands ([`tests/planets_and_special_tests.rs`](file:///Users/nbrand/projects/trek/tests/planets_and_special_tests.rs))
  - Bug regression tests ([`tests/regression_bugs_tests.rs`](file:///Users/nbrand/projects/trek/tests/regression_bugs_tests.rs))

## Appendix

- **Original Reference & Mac OS C Source**: [Super Star Trek — Tom Almy (https://almy.us/sst.html)](https://almy.us/sst.html)
- **Game Manual**: [`assets/sst.doc`](file:///Users/nbrand/projects/trek/assets/sst.doc) (served in-browser at [`web/sst.doc`](file:///Users/nbrand/projects/trek/web/sst.doc))
- **Architecture Diagrams**: [`docs/architecture.md`](file:///Users/nbrand/projects/trek/docs/architecture.md), [`docs/architecture.mmd`](file:///Users/nbrand/projects/trek/docs/architecture.mmd), and [`docs/architecture.pdf`](file:///Users/nbrand/projects/trek/docs/architecture.pdf)
