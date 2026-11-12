# Trek Architecture Diagrams

This document provides visual architecture, inter-module call graph, execution lifecycle, data model, and state-machine diagrams for the **Trek** (Rust + WebAssembly) port of *Super Star Trek for Mac OS*.

- Standalone Mermaid source: [architecture.mmd](file:///Users/nbrand/projects/trek/docs/architecture.mmd)
- Vector PDF export: [architecture.pdf](file:///Users/nbrand/projects/trek/docs/architecture.pdf)

---

## 1. System & Layer Architecture

Shows the static HTTP server ([src/bin/server.rs](file:///Users/nbrand/projects/trek/src/bin/server.rs)), browser terminal/HUD ([web/](file:///Users/nbrand/projects/trek/web)), WebAssembly C-ABI boundary ([src/wasm.rs](file:///Users/nbrand/projects/trek/src/wasm.rs)), `async`/`await` step driver ([src/session.rs](file:///Users/nbrand/projects/trek/src/session.rs)), domain gameplay modules, and shared state/I/O/RNG foundation.

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

---

## 2. Gameplay Engine Inter-Module Call Graph

Maps cross-module call relationships across the domain modules inside [src/](file:///Users/nbrand/projects/trek/src).

```mermaid
flowchart LR
    Session["session.rs (game_main, makemoves)"]
    Setup["setup.rs (setup, newqad, freeze, thaw, abandn)"]
    Moving["moving.rs (warp, impuls, lmove, dock, atover, timwrp, probe, help)"]
    Events["events.rs (events, waiting, nova, snova)"]
    Planets["planets.rs (orbit, sensor, beam, mine, usecrystals, shuttle, deathray)"]
    Battle["battle.rs (phasers, photon, sheild, cloak, capture, attack, deadkl, ram)"]
    Ai["ai.rs (movcom, scom, movetho)"]
    Reports["reports.rs (srscan, lrscan, chart, report, dreprt, eta, attakreport)"]
    Finish["finish.rs (finish, score, plaque, dstrct, kaboom)"]
    Help["help.rs (list_commands, helpme, debugme)"]

    Session -->|"Dispatches 37 commands"| Setup & Moving & Events & Planets & Battle & Ai & Reports & Finish & Help
    Setup -->|"Initial report / combat / finish"| Reports & Battle & Finish
    Moving -->|"Quadrant entry"| Setup
    Moving -->|"Combat / ramming / reports"| Battle & Reports
    Moving -->|"Game over"| Finish
    Events -->|"Super-Commander and Tholian AI"| Ai
    Events -->|"Scheduled attacks / shields / kills"| Battle
    Events -->|"Supernova displacement"| Moving & Setup
    Events -->|"Depletion / supernova end"| Finish
    Planets -->|"Casualties / events / kaboom / victory"| Battle & Events & Finish
    Battle -->|"Commander and Tholian movement"| Ai
    Battle -->|"Star hit -> nova"| Events
    Battle -->|"Damage report / game end"| Reports & Finish
    Ai -->|"Enemy ramming"| Battle
    Finish -->|"Clear remaining enemies"| Battle
```

---

## 3. Interactive Step & Coroutine Lifecycle

Illustrates how the blocking prompt-driven C control flow runs in single-threaded WASM without threads by polling a pinned `Future` until the next `IoHandle` read yields `Poll::Pending`.

```mermaid
sequenceDiagram
    participant Browser as web/app.js
    participant ABI as src/wasm.rs
    participant Session as TrekSession (session.rs)
    participant Future as game_main / makemoves
    participant IO as IoHandle and Scanner

    Note over Browser,IO: Session Boot and Initial Setup
    Browser->>ABI: trek_init(seed, coord_fixed, ptr, len)
    ABI->>Session: TrekSession::new(seed, coord_fixed, cmdline)
    Session->>Future: Box::pin(game_main(state, io)) and poll_once()
    Future->>IO: prelim() and setup() call scan()
    IO-->>Future: IoHandle::read_line(state) parks GameState (Poll::Pending)
    Session->>Session: StepResponse::to_json() captures output and HudState
    ABI-->>Browser: trek_response_ptr() / trek_response_len()
    Browser->>Browser: applyWasmResponse() and updateHud()

    Note over Browser,IO: Interactive Turn Step
    Browser->>ABI: trek_alloc(len) + UTF-8 bytes + trek_step(ptr, len, is_key)
    ABI->>Session: TrekSession::step(input, is_key)
    Session->>IO: Store InputEvent::Line or InputEvent::Key
    Session->>Future: poll_once() resumes pinned Future via Waker::noop()
    Future->>Future: Dispatch command, run events() and attack() if needed
    alt Next prompt reached
        Future->>IO: IoHandle::read_line() or read_key() -> Poll::Pending
        Session->>Session: Build StepResponse JSON delta (PromptType::Line or Key)
    else Session ended (Quit, Victory, Destruction, or EMEXIT)
        Future->>IO: io.park_finished(state) -> Poll::Ready(())
        Session->>Session: Build final StepResponse (PromptType::Exited)
    end
    ABI-->>Browser: JSON (output, hud, saves_updated, plaques_updated)
    Browser->>Browser: Render console, telemetry HUD, and persist to localStorage
```

---

## 4. Core Data Model

Shows the primary structs and enums shared between the coroutine host, HUD telemetry serializer, gameplay state, and Park–Miller PRNG.

```mermaid
classDiagram
    class TrekSession {
        -IoHandle io
        -Pin~Box~Future~~ fut
        -StepResponse last_response
        +new(seed, coord_fixed, cmdline)
        +step(input, is_key)
        +set_coord_fixed(coord_fixed)
        +import_save(name, data)
    }

    class StepResponse {
        +String output
        +bool clear_screen
        +PromptType prompt_type
        +bool emexit
        +HudState hud
        +Vec~SavedEntry~ saves_updated
        +Vec~PlaqueEntry~ plaques_updated
        +to_json()
    }

    class HudState {
        +bool in_game
        +bool coord_fixed
        +String ship
        +String condition
        +f64 date
        +f64 energy
        +f64 shield
        +i32 torps
        +i32 remkl
        +Vec~String~ quad_rows
        +Vec~String~ chart_rows
        +Vec~DeviceStatus~ devices
    }

    class IoHandle {
        +read_line(state)
        +read_key(state)
        +park_finished(state)
    }

    class GameState {
        +GalaxyData d
        +ShipVars a
        +Planet~21~ plnets
        +f64~17~ damage
        +f64~9~ future
        +MacOsRng rng
        +bool coordfixed
        +bool in_game
        +bool emexit
    }

    class GalaxyData {
        +i32~9x9~ galaxy
        +i32 snap
        +i32 remkl
        +i32 remcom
        +i32 rembase
        +f64 date
        +f64 remtime
        +i32 isx
        +i32 isy
    }

    class ShipVars {
        +u8 ship
        +u8 condit
        +f64 energy
        +f64 shield
        +i32 shldup
        +i32 torps
        +i32 quadx
        +i32 quady
        +i32 sectx
        +i32 secty
        +f64 warpfac
        +bool iscloaked
        +i32 brigfree
        +i32 skill
        +i32 length
        +i32 tourn
    }

    class MacOsRng {
        +new(seed)
        +rand_f64()
        +expran(avrage)
        +iran8()
        +iran10()
    }

    class PromptType {
        <<enumeration>>
        Line
        Key
        Exited
    }

    class ScanToken {
        <<enumeration>>
        Eol
        Alpha
        Real
    }

    class FinType {
        <<enumeration>>
        Won
        Deplete
        Lifesup
        Nrg
        Battle
        Nova
        Snovaed
        Abandn
    }

    TrekSession *-- IoHandle
    TrekSession *-- StepResponse
    StepResponse *-- HudState
    IoHandle o-- GameState
    GameState *-- GalaxyData
    GameState *-- ShipVars
    GameState *-- MacOsRng
    StepResponse --> PromptType
    GameState ..> FinType
    GameState ..> ScanToken
```

---

## 5. Game Loop & Turn Execution State Machine

Depicts mission setup, the 37-command loop in `makemoves()`, post-turn event and AI resolution, and endgame scoring/plaque transitions.

```mermaid
stateDiagram-v2
    [*] --> Prelim : Start session
    Prelim --> Setup : prelim()
    Setup --> CommandPrompt : Regular / Tournament / Frozen thaw

    CommandPrompt --> ExecuteCommand : COMMAND> scan() matches COMMANDS
    ExecuteCommand --> EmergencyExit : EMEXIT -> freeze emsave.trk
    EmergencyExit --> [*]

    ExecuteCommand --> FinishScoring : QUIT / DESTRUCT / ABANDON / Victory
    ExecuteCommand --> PostTurnChecks : Turn command completes

    PostTurnChecks --> AdvanceEvents : time > 0 -> events()
    PostTurnChecks --> TholianMove : time == 0 and nenhere == 0 -> movetho()

    AdvanceEvents --> FinishScoring : alldone != 0
    AdvanceEvents --> SupernovaEscape : galaxy == 1000 -> atover()
    SupernovaEscape --> PostTurnChecks : Continue in new quadrant
    AdvanceEvents --> TholianMove : nenhere == 0 -> movetho()
    AdvanceEvents --> EnemyCounterattack : nenhere != 0 and hitme

    TholianMove --> EnemyCounterattack : hitme and justin == 0 -> attack()
    TholianMove --> CommandPrompt : No counterattack

    EnemyCounterattack --> FinishScoring : alldone != 0
    EnemyCounterattack --> SupernovaEscape : galaxy == 1000 -> atover()
    EnemyCounterattack --> CommandPrompt : alldone == 0

    FinishScoring --> EmeritusPlaque : Won at Emeritus (unfrozen) -> plaque()
    FinishScoring --> PlayAgainPrompt : Score displayed
    EmeritusPlaque --> PlayAgainPrompt : Citation recorded

    PlayAgainPrompt --> Setup : Yes
    PlayAgainPrompt --> [*] : No
```

---

## 6. Module Summary

| Path | Role |
| --- | --- |
| [src/lib.rs](file:///Users/nbrand/projects/trek/src/lib.rs) | Library crate root re-exporting public modules, types, and `TrekSession` API |
| [src/state.rs](file:///Users/nbrand/projects/trek/src/state.rs) | Core game state (`GameState`, `GalaxyData`, `ShipVars`, `Planet`, `FinType`), constants, and JSON helpers |
| [src/rng.rs](file:///Users/nbrand/projects/trek/src/rng.rs) | Deterministic 31-bit Park–Miller PRNG (`MacOsRng`) matching `osx.c` `Rand()` |
| [src/io.rs](file:///Users/nbrand/projects/trek/src/io.rs) | Coroutine I/O suspension (`IoHandle`, `PromptType`, `InputEvent`) and console formatting (`prout`, `pause`, `skip`) |
| [src/scanner.rs](file:///Users/nbrand/projects/trek/src/scanner.rs) | Command/numeric tokenizer (`scan`, `ja`, `parse_c_double_prefix`, `ScanToken`) matching C `scan()` |
| [src/session.rs](file:///Users/nbrand/projects/trek/src/session.rs) | `game_main`, `makemoves`, `TrekSession`, `StepResponse`, and anti-cheat `HudState` snapshot builder |
| [src/wasm.rs](file:///Users/nbrand/projects/trek/src/wasm.rs) | `cdylib` C-ABI exports (`trek_init`, `trek_step`, `trek_set_coord_fixed`, `trek_import_save`, memory allocators) |
| [src/setup.rs](file:///Users/nbrand/projects/trek/src/setup.rs) | Galaxy initialization, quadrant generation (`newqad`), `abandn`, and `.trk` freeze/thaw serialization |
| [src/reports.rs](file:///Users/nbrand/projects/trek/src/reports.rs) | `srscan`, `lrscan`, `chart`, `report`, `dreprt`, `eta`, and starbase attack reports |
| [src/moving.rs](file:///Users/nbrand/projects/trek/src/moving.rs) | `warp`, `impuls`, `lmove`, `dock`, `setwrp`, `atover`, `timwrp`, `probe`, and `help` (SOS call) |
| [src/battle.rs](file:///Users/nbrand/projects/trek/src/battle.rs) | `phasers`, `photon`, `torpedo`, `sheild`, `cloak`, `capture`, `attack`, `deadkl`, and `ram` |
| [src/events.rs](file:///Users/nbrand/projects/trek/src/events.rs) | Future event scheduler (`events`), `waiting` (`REST`), `nova`, and `snova` |
| [src/ai.rs](file:///Users/nbrand/projects/trek/src/ai.rs) | Klingon Commander (`movcom`), Super-Commander (`scom`), and Tholian (`movetho`) AI |
| [src/planets.rs](file:///Users/nbrand/projects/trek/src/planets.rs) | Planetary `orbit`, `sensor`, `beam`, `mine`, `usecrystals`, `shuttle`, `preport`, and `deathray` |
| [src/finish.rs](file:///Users/nbrand/projects/trek/src/finish.rs) | Endgame `finish`, `score`, 132-column Commodore Emeritus `plaque`, `dstrct`, and `kaboom` |
| [src/help.rs](file:///Users/nbrand/projects/trek/src/help.rs) | `COMMANDS` table, `list_commands`, `helpme` (`assets/sst.doc` lookup), and `debugme` |
| [src/bin/server.rs](file:///Users/nbrand/projects/trek/src/bin/server.rs) | Pure-`std` TCP HTTP server serving `./web` on `127.0.0.1:8080` (configurable via `--port` or `PORT`) |
| [web/](file:///Users/nbrand/projects/trek/web) | Interactive terminal UI (`index.html`, `styles.css`, `app.js`), `sst.doc`, and compiled `trek.wasm` |
