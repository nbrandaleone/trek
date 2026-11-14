# Trek Architecture Diagrams

End-to-end program flow, gameplay subsystem dispatch, stackless coroutine lifecycle, core data model, and turn-execution state machine for **Trek** — the zero-dependency Rust + WebAssembly port of Tom Almy's *Super Star Trek for Mac OS* ([`Cargo.toml`](../Cargo.toml), [`c_src/`](../c_src/)).

- Standalone Mermaid source (Diagram 1): [`architecture.mmd`](./architecture.mmd)
- Vector landscape PDF export (all 5 diagrams): [`architecture.pdf`](./architecture.pdf)

---

## 1. System & Program Flow Architecture

Traces execution from the pure-`std` HTTP server ([`src/bin/server.rs`](../src/bin/server.rs)) through the browser Bridge Computer and Live Telemetry HUD ([`web/index.html`](../web/index.html), [`web/styles.css`](../web/styles.css), [`web/app.js`](../web/app.js)), across the WebAssembly C-ABI boundary ([`src/wasm.rs`](../src/wasm.rs)) and single-turn `Future` poller ([`src/session.rs`](../src/session.rs), [`src/io.rs`](../src/io.rs)), into the deterministic gameplay engine ([`src/lib.rs`](../src/lib.rs), [`src/scanner.rs`](../src/scanner.rs), [`src/state.rs`](../src/state.rs), [`src/rng.rs`](../src/rng.rs)).

```mermaid
flowchart TB
    subgraph Host["1․ HTTP Server &amp; Static Asset Delivery (src/bin/server.rs, web/sst.doc)"]
        direction LR
        Launch(["Launch Binary (trek)<br/>Default bind 127.0.0.1:8080"])
        CliCfg["CLI &amp; Env Config Parser<br/>--host / --host= (HOST)<br/>--port, -p / --port= (PORT) | --help, -h"]
        HttpSrv["Pure-std TCP HTTP/1.1 Server<br/>Thread-per-conn GET &amp; HEAD<br/>percent_decode() + canonicalize() guard<br/>Serves index.html, styles.css, app.js, sst.doc, trek.wasm"]

        Launch --> CliCfg
        CliCfg -->|"TcpListener::bind(addr)"| HttpSrv
    end

    subgraph UI["2․ Browser Terminal, Live HUD &amp; Persistence (web/index.html, web/styles.css, web/app.js)"]
        direction LR
        Console["Bridge Terminal &amp; Input Controls<br/>Line/Key prompt modes + 200-cmd history<br/>20 Quick-Command buttons + Boss Screen<br/>Themes: phosphor-green, amber-crt, starfleet-modern"]
        HudView["Live Telemetry HUD &amp; Tactical Grids<br/>Condition badge, Energy/Shield bars, 9 fleet counters<br/>Clickable 10x10 Sector Grid (-f X/Y coord toggle)<br/>8x8 Star Chart + 16-Device Damage list"]
        Modals["Browser localStorage &amp; 3 Native &lt;dialog&gt;s<br/>Manual (SST.DOC section/text filter)<br/>Saves (.trk upload, trek.save.*, emsave.trk)<br/>Plaque (132-col Emeritus download &amp; print)"]

        Console <-->|"Click sector -&gt; coords<br/>-f coordFixed sync"| HudView
        HudView <-->|"trek.save.* / plaque /<br/>SST.DOC dialogs"| Modals
    end

    subgraph Bridge["3․ WASM C-ABI &amp; Stackless Coroutine Session (src/wasm.rs, src/session.rs, src/io.rs)"]
        direction LR
        WasmAbi["WASM C-ABI (src/wasm.rs)<br/>RefCell&lt;Option&lt;WasmRuntime&gt;&gt;<br/>trek_alloc / trek_dealloc · trek_init / trek_step<br/>trek_set_coord_fixed · trek_import_save<br/>trek_response_ptr / trek_response_len"]
        PollHub{"TrekSession::poll_once()<br/>Pin&lt;Box&lt;dyn Future&gt;&gt;<br/>+ Waker::noop()"}
        IoBridge["Coroutine I/O &amp; Telemetry (io.rs, session.rs)<br/>IoHandle Rc&lt;RefCell&lt;IoShared&gt;&gt; (read_line / getch)<br/>Swaps GameState via empty_placeholder()<br/>StepResponse::to_json() + anti-cheat HudState<br/>(masks distant sectors when DSRSENS &gt; 0)"]
        SessionEnd(["Session Concluded<br/>park_finished() -&gt; Exited<br/>or EMEXIT (emsave.trk)"])

        WasmAbi -->|"new() / step()"| PollHub
        PollHub <-->|"Pending (Line | Key)<br/>or Ready(())"| IoBridge
        IoBridge -.->|"No replay / EMEXIT"| SessionEnd
    end

    subgraph Engine["4․ Zero-Dependency Gameplay Engine &amp; Foundation (src/lib.rs, src/scanner.rs, src/state.rs, src/rng.rs)"]
        direction LR
        Lifecycle["Game Lifecycle &amp; Turn Dispatcher<br/>game_main() + makemoves() (session.rs)<br/>prelim, setup, newqad, freeze/thaw (setup.rs)<br/>29 prefix + 8 exact commands (COMMANDS[37])"]
        Subsystems["Domain Gameplay Subsystems<br/>moving.rs · battle.rs · ai.rs<br/>events.rs · planets.rs · reports.rs<br/>finish.rs (22 FinType, plaque) · help.rs (SST_DOC)"]
        Foundation["Deterministic Core Foundation (std only)<br/>scanner.rs: scan(), ja(), parse_c_double_prefix()<br/>state.rs: GameState, GalaxyData, ShipVars, Planet<br/>rng.rs: MacOsRng (31-bit Park-Miller LCG)"]

        Lifecycle -->|"Dispatches commands &amp; events"| Subsystems
        Subsystems <-->|"Tokenizes input, mutates state,<br/>draws deterministic PRNG"| Foundation
    end

    Host -->|"HTTP GET / + ./trek.wasm + ./sst.doc"| UI
    UI <-->|"trek_step() UTF-8 buffers &lt;-&gt; StepResponse JSON (hud, saves_updated, plaques_updated)"| Bridge
    UI -.->|"trek_import_save(.trk) &amp; trek_set_coord_fixed(-f)"| Bridge
    Bridge <-->|"Polls pinned Future &lt;-&gt; parks GameState at read_line / getch"| Engine

    classDef pill fill:#dcfce7,stroke:#16a34a,color:#052e16;
    classDef hub fill:#fef3c7,stroke:#d97706,color:#451a03;
    classDef core fill:#f1f5f9,stroke:#475569,color:#0f172a;
    class Launch,SessionEnd pill;
    class PollHub hub;
    class Foundation core;
```

---

## 2. Gameplay Engine Subsystems & Turn Dispatch

Groups the 10 gameplay domain modules in [`src/`](../src/) into four functional stages, showing `makemoves()` command dispatch, cross-module combat/navigation/planet interactions, scheduled stardate events (including subspace radio `attakreport()`), and endgame scoring/plaque paths.

```mermaid
flowchart TB
    subgraph Boot["1․ Mission Setup, Persistence &amp; Command Dispatch (src/session.rs, src/setup.rs, src/help.rs)"]
        direction LR
        StartGame(["game_main(state, io)<br/>prelim() banner + mission loop"])
        SetupMod["setup.rs — Universe &amp; Saves<br/>setup(), choose() (regular / tournament / frozen)<br/>newqad() quadrant populate · abandn()<br/>freeze() / thaw() (.trk serialize_freeze)"]
        CmdHub{"makemoves() Loop<br/>29 prefix + 8 exact cmds<br/>(COMMANDS[0..37])"}
        HelpMod["help.rs — Manual &amp; Debug<br/>list_commands() · helpme() (SST_DOC)<br/>debugme() (idebug, device/event edit)"]

        StartGame -->|"setup() / thawed check"| SetupMod
        SetupMod -->|"in_game = true"| CmdHub
        CmdHub -->|"COMMANDS / HELP / DEBUG"| HelpMod
    end

    subgraph NavOps["2․ Navigation, Sensors &amp; Planetary Operations (src/moving.rs, src/reports.rs, src/planets.rs)"]
        direction LR
        MovingMod["moving.rs — Helm &amp; Deep-Space Probes<br/>warp(), impuls(), lmove(), setwrp()<br/>dock(), atover() (supernova warp)<br/>timwrp() time-travel · probe() · help() SOS"]
        ReportsMod["reports.rs — Sensors &amp; Ship Computer<br/>srscan() (SR/status/request), lrscan()<br/>chart(), report(), dreprt() damages<br/>eta() warp calculator · attakreport()"]
        PlanetsMod["planets.rs — Orbit, Mining &amp; Shuttle<br/>orbit(), sensor(), preport(), beam()<br/>mine(), usecrystals(), shuttle() (Galileo)<br/>deathray() experimental weapon"]

        MovingMod -->|"attakreport() radio alerts"| ReportsMod
        PlanetsMod -.->|"Shuttle / beam / orbit state"| MovingMod
    end

    subgraph CombatAI["3․ Tactical Combat &amp; Enemy AI Subsystems (src/battle.rs, src/ai.rs)"]
        direction LR
        BattleMod["battle.rs — Weapons, Shields &amp; Capture<br/>phasers(), photon(), torpedo(), hittem()<br/>sheild(), cloak() (Treaty of Algeron)<br/>attack(), deadkl(), ram(), capture()"]
        AiMod["ai.rs — Tactical &amp; Strategic Enemy AI<br/>movcom() Klingon/Romulan retreat &amp; ram<br/>scom() Super-Commander starbase hunt<br/>movetho() Tholian corner move &amp; # web"]

        BattleMod -->|"movcom() &amp; movetho() maneuvers"| AiMod
        AiMod -->|"Enemy collision -&gt; ram()"| BattleMod
    end

    subgraph PostEnd["4․ Post-Turn Event Scheduler &amp; Endgame Scoring (src/events.rs, src/finish.rs)"]
        direction LR
        EventsMod["events.rs — Future Event Scheduler<br/>events() advances stardate across future[1..=8]<br/>(FSNOVA, FTBEAM, FSNAP, FBATTAK,<br/>FCDBAS, FSCMOVE, FSCDBAS, FDSPROB)<br/>waiting() (REST) · nova() · snova()"]
        FinishMod["finish.rs — Endgame &amp; 132-Col Plaque<br/>finish(FinType: 22 win/loss outcomes)<br/>dstrct() password countdown · kaboom()<br/>score() breakdown · plaque() (skill &gt; SGOOD)"]
        ExitLoop(["Mission Concluded<br/>Tournament freeze() prompt<br/>Play again? -&gt; Replay or Exit"])

        EventsMod -->|"Depletion / supernova / life support"| FinishMod
        FinishMod -->|"alldone = 1 / EMEXIT"| ExitLoop
    end

    Boot -->|"Helm, sensor &amp; planet cmds; moving/events call setup::newqad()"| NavOps
    NavOps -->|"Moving/Setup trigger attack(); Planets call deadkl() &amp; events()"| CombatAI
    CombatAI <-->|"Star hit -&gt; nova(); Scheduled raids, scom/movetho &amp; reports::attakreport/dreprt"| PostEnd
    NavOps -.->|"Black hole / SOS / mining / deathray -&gt; finish() &amp; kaboom()"| PostEnd

    classDef pill fill:#dcfce7,stroke:#16a34a,color:#052e16;
    classDef hub fill:#fef3c7,stroke:#d97706,color:#451a03;
    class StartGame,ExitLoop pill;
    class CmdHub hub;
```

---

## 3. Interactive Step & Coroutine Lifecycle

Illustrates how the prompt-driven C control flow runs in single-threaded WebAssembly without OS threads or manual state-machine rewrites: [`TrekSession`](../src/session.rs) polls a pinned `Future` with `Waker::noop()` until [`IoHandle::read_line`](../src/io.rs) (`ReadLineFuture`) or [`IoHandle::getch`](../src/io.rs) (`GetChFuture`) swaps `GameState` out via `empty_placeholder()` and yields `Poll::Pending`.

```mermaid
sequenceDiagram
    participant Browser as web/app.js<br/>(WASM Host & HUD)
    participant ABI as src/wasm.rs<br/>(C-ABI + WasmRuntime)
    participant Session as TrekSession<br/>(src/session.rs)
    participant Future as game_main /<br/>makemoves
    participant IO as IoHandle & Scanner<br/>(io.rs, scanner.rs)

    Note over Browser,IO: 1. Session Boot, Save Hydration & Initial Prompt
    Browser->>ABI: trek_alloc(len) + cmdline + trek_init(seed, coord_fixed, ptr, len)
    ABI->>Session: TrekSession::new(seed, coord_fixed, cmdline)
    Session->>Future: Box::pin(game_main(state, io)) + poll_once() (Waker::noop)
    Future->>IO: prelim() -> setup() -> choose() -> scan() -> read_line(state)
    IO-->>Session: Swap GameState via empty_placeholder() -> Poll::Pending (Line)
    Session->>Session: Drain output_buf, anti-cheat HudState,<br/>StepResponse::to_json()
    ABI-->>Browser: trek_response_ptr() / trek_response_len() -> JSON
    Browser->>ABI: Hydrate localStorage .trk saves via trek_import_save(...)

    Note over Browser,IO: 2. Interactive Turn Step (Line Input or Single-Key Pause)
    Browser->>ABI: trek_alloc(len) + UTF-8 bytes + trek_step(ptr, len, is_key)
    ABI->>Session: TrekSession::step(input, is_key) -> InputEvent::Line or Key
    Session->>Future: poll_once() resumes pinned Future via Waker::noop()
    IO-->>Future: Restore parked GameState & yield ScanToken / char
    Future->>Future: Dispatch COMMANDS[0..37], mid-turn shield/cloak<br/>attack(), post-turn events(), movetho() & attack()
    alt Awaiting command or parameter line
        Future->>IO: IoHandle::read_line(state) -> Poll::Pending
        Session->>Session: StepResponse JSON delta (PromptType::Line)
    else Paused at page-full or announcement prompt
        Future->>IO: pause() -> IoHandle::getch(state) -> Poll::Pending
        Session->>Session: StepResponse JSON delta (PromptType::Key)
    else Session ended (No replay or EMEXIT Boss Screen)
        Future->>IO: io.park_finished(state) -> Poll::Ready(())
        Session->>Session: Final StepResponse (PromptType::Exited, emexit)
    end
    ABI-->>Browser: JSON (output, clear_screen, hud, saves_updated, plaques_updated)
    Browser->>Browser: Update console/Boss view, HUD grids & localStorage
```

---

## 4. Core Data Model

Details the structs and enums across [`src/session.rs`](../src/session.rs), [`src/io.rs`](../src/io.rs), [`src/state.rs`](../src/state.rs), [`src/scanner.rs`](../src/scanner.rs), and [`src/rng.rs`](../src/rng.rs), including 1-based C-faithful galaxy/sector arrays, virtual save/plaque maps, anti-cheat HUD telemetry, and all 22 `FinType` endgame outcomes.

```mermaid
classDiagram
    direction LR

    class TrekSession {
        -IoHandle io
        -Pin~Box~dyn Future~~ fut
        -StepResponse last_response
        +new(seed, coord_fixed, cmdline) Self
        +step(input, is_key) StepResponse
        +set_coord_fixed(coord_fixed) StepResponse
        +import_save(name, data) bool
        +with_state(f) / with_state_mut(f)
    }

    class StepResponse {
        +String output
        +bool clear_screen, emexit
        +PromptType prompt_type
        +HudState hud
        +Vec~SavedEntry~ saves_updated
        +Vec~PlaqueEntry~ plaques_updated
        +to_json() String
    }

    class HudState {
        +bool in_game, alldone, gamewon, coord_fixed, cloaked
        +String ship, condition
        +f64 stardate, time_left, warp_factor
        +i32 quadx, quady, sectx, secty
        +f64 energy, max_energy, shield, max_shield
        +bool shields_up, shield_damaged, life_support_active
        +f64 life_support_reserves
        +i32 torps, max_torps, probes_left, bases_left
        +i32 klingons_left, commanders_left, super_commander_left, romulans_left
        +bool crystals_aboard, in_orbit, landed
        +i32 brig_captives, brig_capacity, kcaptured, casualties
        +Vec~String~ quad_rows
        +Vec~Vec~String~~ chart_rows
        +Vec~DeviceStatus~ devices
        +from_state(state) Self
        +push_json(out)
    }

    class IoHandle {
        -Rc~RefCell~IoShared~~ shared
        +read_line(state) String
        +getch(state) char
        +park_finished(state)
    }

    class GameState {
        +GalaxyData d, snapsht
        +ShipVars a
        +char[11][11] quad
        +i32[9][9] starch
        +i32[21] kx, ky
        +f64[21] kpower, kdist, kavgd
        +f64[17] damage
        +f64[9] future
        +MacOsRng rng
        +u32 initial_seed
        +String passwd, line, citem, output_buf
        +f64 aaitem
        +bool coordfixed, in_game, emexit, exited
        +BTreeMap~String,String~ saved_games, plaques
        +randomize() / newcnd() / sortkl()
    }

    class GalaxyData {
        +i32[9][9] galaxy, newstuf
        +Planet[11] plnets
        +i32[11] cx, cy
        +i32[6] baseqx, baseqy
        +i32 snap, remkl, remcom, rembase
        +i32 nscrem, nromrem, nromkl, nsckill
        +i32 killk, killc, starkl, basekl, nplankl
        +i32 isx, isy, kcaptured, brigfree
        +f64 date, remtime, remres
    }

    class Planet {
        +i32 x, y, pclass, crystals, known
    }

    class ShipVars {
        +char ship, condit
        +f64 energy, inenrg, shield, inshld
        +i32 shldup, torps, intorps
        +i32 quadx, quady, sectx, secty
        +f64 warpfac, lsupres, time
        +i32 skill, length, tourn, thawed
        +bool iscloaked, iscloaking, isarmed
        +i32 brigcapacity, nprobes, casual, ncviol
        +i32 alldone, gamewon, alive, justin
    }

    class MacOsRng {
        -u32 state
        +new(seed) Self
        +from_raw(state) Self
        +srand(seed)
        +state() u32
        +rand_int() i32
        +rand_f64() f64
        +expran(avrage) f64
        +iran8() Tuple~i32,i32~
        +iran10() Tuple~i32,i32~
    }

    class PromptType {
        <<enumeration>>
        Line · Key · Exited
    }

    class ScanToken {
        <<enumeration>>
        Eol · Alpha · Real
    }

    class FinType {
        <<enumeration>>
        Won · Deplete · Lifesup · Nrg · Battle · Neg3
        Nova · Snovaed · Abandn · Dilithium · Materialize
        Phaser · Lost · Mining · Dplanet · Pnova · Ssc
        Stractor · Dray · Tribble · Hole · Cloak
    }

    TrekSession *-- IoHandle
    TrekSession *-- StepResponse
    StepResponse *-- HudState : SavedEntry / PlaqueEntry
    StepResponse --> PromptType
    IoHandle o-- GameState
    GameState *-- GalaxyData
    GameState *-- ShipVars
    GameState *-- MacOsRng
    GalaxyData *-- Planet
    GameState ..> FinType
    GameState ..> ScanToken
```

---

## 5. Game Loop & Turn Execution State Machine

Captures the full control flow across `game_main()`, `setup()`, `makemoves()`, and `finish()`, including thawed score-only `.trk` inspection (`alldone != 0`), mid-command free enemy attacks during `SHIELDS`/`CLOAK`, supernova emergency displacement (`atover`), Commodore Emeritus citation eligibility (`skill > SGOOD` when unfrozen), and tournament `.trk` score recording.

```mermaid
stateDiagram-v2
    [*] --> Prelim : game_main()
    Prelim --> Setup : prelim() banner

    Setup --> ThawedScore : Thawed finished .trk (alldone != 0)
    Setup --> CommandPrompt : Regular / Tournament / Active .trk (in_game = true)
    ThawedScore --> PlayAgainPrompt : score(false) -> alldone = 0

    CommandPrompt --> DispatchCmd : COMMAND> scan() matches COMMANDS[0..37]
    DispatchCmd --> EmergencyExit : EMEXIT -> freeze(emsave.trk)
    EmergencyExit --> [*] : park_finished()

    DispatchCmd --> MidTurnAttack : SHIELDS / CLOAK -> attack(2)
    MidTurnAttack --> PostTurnChecks : Update shldchg / iscloaked

    DispatchCmd --> FinishOutcome : QUIT / DESTRUCT / ABANDON / win / loss
    DispatchCmd --> PostTurnChecks : Turn or info cmd completes

    PostTurnChecks --> AdvanceEvents : time != 0.0 -> events()
    PostTurnChecks --> SupernovaWarp : time == 0 & galaxy == 1000 -> atover(false)
    PostTurnChecks --> TholianMove : time == 0 & nenhere == 0 -> movetho()
    PostTurnChecks --> Counterattack : time == 0 & nenhere != 0 & hitme & justin == 0
    PostTurnChecks --> CommandPrompt : time == 0 & no Tholian or counterattack

    AdvanceEvents --> FinishOutcome : alldone != 0
    AdvanceEvents --> SupernovaWarp : galaxy == 1000 -> atover(false)
    AdvanceEvents --> TholianMove : nenhere == 0 -> movetho()
    AdvanceEvents --> Counterattack : nenhere != 0 & hitme & justin == 0
    AdvanceEvents --> CommandPrompt : Turn settled

    TholianMove --> Counterattack : hitme & justin == 0 -> attack(2)
    TholianMove --> CommandPrompt : Turn settled

    Counterattack --> FinishOutcome : alldone != 0
    Counterattack --> SupernovaWarp : galaxy == 1000 (hitme = true)
    Counterattack --> CommandPrompt : alldone == 0
    SupernovaWarp --> PostTurnChecks : Re-check in new quadrant

    FinishOutcome --> EmeritusPlaque : Won & promoted (skill > SGOOD, unfrozen) -> plaque()
    FinishOutcome --> TournamentSave : score(false) displayed
    EmeritusPlaque --> TournamentSave : 132-col citation recorded

    TournamentSave --> PlayAgainPrompt : tourn != 0 -> optional freeze(.trk)
    PlayAgainPrompt --> Setup : Yes
    PlayAgainPrompt --> [*] : No -> farewell + park_finished()
```

---

## 6. Module & Asset Summary

| Path | Role |
| --- | --- |
| [`Cargo.toml`](../Cargo.toml) | Zero-dependency Rust 2021 manifest defining `trek` (`cdylib` + `rlib`) and the `trek` HTTP server binary |
| [`Makefile`](../Makefile) / [`Dockerfile`](../Dockerfile) | Build, WASM compilation ([`scripts/build_web.sh`](../scripts/build_web.sh)), server launch, test, check, and multi-stage container targets |
| [`src/lib.rs`](../src/lib.rs) | Library crate root re-exporting public modules, constants, `.trk` helpers, and the `TrekSession` API |
| [`src/bin/server.rs`](../src/bin/server.rs) | Pure-`std` multi-threaded TCP HTTP/1.1 server serving `./web` on `--host`/`HOST` and `--port`/`PORT` (`127.0.0.1:8080` default) |
| [`src/wasm.rs`](../src/wasm.rs) | `cdylib` C-ABI exports (`trek_alloc`, `trek_dealloc`, `trek_init`, `trek_step`, `trek_set_coord_fixed`, `trek_import_save`, `trek_response_ptr`, `trek_response_len`) |
| [`src/session.rs`](../src/session.rs) | `game_main`, `makemoves`, `TrekSession` single-turn `Future` poller (`Waker::noop`), `StepResponse`, and anti-cheat `HudState` serializer |
| [`src/io.rs`](../src/io.rs) | Coroutine suspension bridge (`IoHandle`, `PromptType`, `InputEvent`, `read_line`, `getch`, `park_finished`) and paging/output helpers (`prout`, `pause`, `skip`) |
| [`src/scanner.rs`](../src/scanner.rs) | C-compatible input tokenizer (`scan`, `ja`, `parse_c_double_prefix`, `ScanToken`) matching `osx.c` / `sst.c` `scan()` |
| [`src/state.rs`](../src/state.rs) | Core state (`GameState`, `GalaxyData`, `ShipVars`, `Planet`, 22-variant `FinType`), 16 device & 9 future-event constants, and JSON helpers |
| [`src/rng.rs`](../src/rng.rs) | Deterministic 31-bit Darwin/macOS Park–Miller LCG (`MacOsRng`: `srand`, `rand_int`, `rand_f64`, `expran`, `iran8`, `iran10`) |
| [`src/setup.rs`](../src/setup.rs) | Mission initialization (`prelim`, `setup`, `choose`), quadrant generation (`newqad`), `abandn`, and `.trk` `freeze`/`thaw` serialization |
| [`src/reports.rs`](../src/reports.rs) | `srscan`, `lrscan`, `chart`, `report`, `dreprt`, `eta`, and subspace radio `attakreport` |
| [`src/moving.rs`](../src/moving.rs) | `warp`, `impuls`, `lmove`, `dock`, `setwrp`, emergency `atover`, `timwrp`, deep-space `probe`, and Starbase `help` (SOS) |
| [`src/battle.rs`](../src/battle.rs) | `phasers`, `photon`, `torpedo`, `hittem`, `sheild`, `cloak` (Treaty of Algeron), `capture`, `attack`, `deadkl`, and `ram` |
| [`src/events.rs`](../src/events.rs) | Scheduled stardate progression (`events` over `future[1..=8]`), `waiting` (`REST`), stellar `nova`, and quadrant `snova` |
| [`src/ai.rs`](../src/ai.rs) | Klingon/Romulan Commander (`movcom`), Super-Commander (`scom`), and Tholian (`movetho` + `#` web) AI |
| [`src/planets.rs`](../src/planets.rs) | Planetary `orbit`, `sensor`, `preport`, `beam` transporter, dilithium `mine`, `usecrystals`, Galileo `shuttle`, and experimental `deathray` |
| [`src/finish.rs`](../src/finish.rs) | Endgame `finish`, `score` breakdown, 132-column Commodore Emeritus `plaque`, self-destruct `dstrct`, and `kaboom` |
| [`src/help.rs`](../src/help.rs) | 37-entry `COMMANDS` table, `list_commands`, `helpme` ([`assets/sst.doc`](../assets/sst.doc) lookup), and `debugme` |
| [`web/`](../web/) | Interactive Bridge Terminal & Live Telemetry HUD ([`index.html`](../web/index.html), [`styles.css`](../web/styles.css), [`app.js`](../web/app.js)), [`sst.doc`](../web/sst.doc), and `trek.wasm` |
| [`tests/`](../tests/) / [`c_src/`](../c_src/) | 7 integration & regression test suites verifying RNG, scanner, navigation, combat, AI, planets, and parity with original ANSI C sources |
