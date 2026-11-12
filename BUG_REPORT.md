# Super Star Trek — Bug Audit & Fix Report

This report documents every bug identified and fixed across the Rust engine (`src/`), WebAssembly interface, and Web UI (`web/`), verified against the reference Mac OS binary (`/tmp/sst_inspect/osx_arm/sstosxAppleSilicon/sst`) and C source (`c_src/*.c`).

---

## BUG-01: Super-Commander diagonal move uses `ideltax` for Y axis
- **Severity**: High
- **Category**: Latent C Bug / AI Logic Bug
- **Root Cause & Location**: `src/ai.rs` — `scom()` (ported from `c_src/ai.c:469`)
  - When moving the Super-Commander diagonally toward the nearest starbase, `iqy` was computed as `state.d.isy + ideltax` instead of `state.d.isy + ideltay`, restricting diagonal movement to `(+1,+1)`, `(-1,-1)`, or `(0,0)` and sending the Super-Commander off-course or out-of-bounds.
- **Fix Applied**: Changed `let mut iqy = state.d.isy + ideltax;` to `let mut iqy = state.d.isy + ideltay;`.
- **Exact Deterministic Reproduction Steps**:
  1. Initialize `GameState` with `d.nscrem = 1`, Super-Commander at `(d.isx, d.isy) = (4, 6)` and a single starbase at `(d.baseqx[1], d.baseqy[1]) = (6, 4)` (`ideltax = +1`, `ideltay = -1`), with empty adjacent destination quadrants (`< 900`).
  2. Call `scom(&mut state, &io, &mut ipage)` (with `state.rng` and `d.date` set so `scom` selects a move).
  3. **Before fix**: `(isx, isy)` becomes `(5, 7)` (moved `+1` in Y instead of `-1`).
  4. **After fix**: `(isx, isy)` moves along `(ideltax, ideltay)` toward `(6, 4)` (e.g., `(5, 5)`).

---

## BUG-02: Super-Commander starbase distance bubble sort off-by-one
- **Severity**: Medium
- **Category**: Latent C Bug / AI Logic Bug
- **Root Cause & Location**: `src/ai.rs` — `scom()` (ported from `c_src/ai.c:415`)
  - Starbases are 1-indexed (`1..=remb`), and the bubble sort looped `for i in 1..(remb - 1)`. When `remb == 2`, `1..1` is empty, and for `remb >= 2` the last starbase (`bdist[remb]`) was never compared against `bdist[remb - 1]`.
- **Fix Applied**: Changed loop range to `for i in 1..remb` so adjacent pairs `(i, i + 1)` are compared across all `1..=remb` starbases.
- **Exact Deterministic Reproduction Steps**:
  1. Initialize `GameState` with `d.rembase = 2`, `d.nscrem = 1`, Super-Commander at `(1, 1)`, base 1 far away at `(8, 8)`, and base 2 adjacent at `(1, 2)` (both having `d.galaxy[bx][by] < 900` and no Enterprise in quadrant).
  2. Run `scom(&mut state, &io, &mut ipage)`.
  3. **Before fix**: Base distances `[98, 1]` are never swapped (`1..1` is empty), so the Super-Commander targets base 1 at `(8, 8)` instead of the adjacent base 2 at `(1, 2)`.
  4. **After fix**: Base 2 at `(1, 2)` is sorted first and targeted.

---

## BUG-03: Super-Commander base-attack guard checks `&&` instead of `||`
- **Severity**: Medium
- **Category**: Latent C Bug / AI Logic Bug
- **Root Cause & Location**: `src/ai.rs` — `scom()`
  - The check `state.d.isx != state.a.batx && state.d.isy != state.a.baty` skipped attacking a starbase whenever the Super-Commander's target base happened to share either the X or Y coordinate of a different base already under attack by a regular Commander `(batx, baty)`.
- **Fix Applied**: Changed condition to `(state.d.isx != state.a.batx || state.d.isy != state.a.baty)` so only the exact same base `(batx, baty)` is skipped.
- **Exact Deterministic Reproduction Steps**:
  1. Initialize `GameState` with `d.rembase = 1`, base 1 at `(3, 2)`, Super-Commander at `(3, 3)` (`ideltax = 0, ideltay = -1`), and an active Commander attack at a different quadrant sharing X coordinate: `a.batx = 3, a.baty = 7`.
  2. Call `scom(&mut state, &io, &mut ipage)` so the Super-Commander moves to `(3, 2)`.
  3. **Before fix**: Because `isx == batx` (`3 == 3`), `isx != batx` is false and the Super-Commander does not begin attacking the base at `(3, 2)` (`future[FSCDBAS]` stays `1e30`).
  4. **After fix**: `future[FSCDBAS]` is scheduled (`<= state.d.date + 2.0`) and `a.isatb = 1`.

---

## BUG-04: `movcom()` skips an enemy when `movebaddy()` triggers `tryexit()`
- **Severity**: High
- **Category**: Logic Bug / Loop Invariant Bug
- **Root Cause & Location**: `src/ai.rs` — `movcom()`
  - For `skill > SGOOD`, `movcom()` iterated `while i <= state.a.nenhere as usize` over `IHK`/`IHR` enemies and unconditionally incremented `i += 1`. When `movebaddy()` caused an enemy at slot `i` to flee via `tryexit()`, `tryexit()` swap-removed slot `i` with `kx[nenhere]` and decremented `nenhere`. Incrementing `i` then skipped processing the enemy swapped into slot `i`. Additionally, `movcom()` did not stop if `movebaddy()` -> `ram()` ended the game (`alldone != 0`).
- **Fix Applied**: Captured `let prev_nh = state.a.nenhere;` before `movebaddy()`, returned immediately if `state.a.alldone != 0`, and only incremented `i += 1` when `state.a.nenhere == prev_nh`.
- **Exact Deterministic Reproduction Steps**:
  1. Set `a.skill = SEXPERT`, `a.nenhere = 2`, `a.klhere = 2`, Enterprise at `(5, 5)`.
  2. Place fleeing `IHK` #1 at edge `(1, 1)` with low `kpower[1] = 100.0` (so `movebaddy` calls `tryexit` with `irun = 1` into valid adjacent quadrant) and `IHK` #2 at `(2, 8)` with `kpower[2] = 500.0`.
  3. Call `movcom(&mut state, &io)`.
  4. **Before fix**: `IHK` #1 escapes via `tryexit` and `IHK` #2 is swapped into slot `1`; `i` increments to `2 > nenhere (1)`, so `IHK` #2 never moves.
  5. **After fix**: `i` stays at `1` after `nenhere` shrinks, so `IHK` #2 is also processed.

---

## BUG-05: `tryexit()` runs Commander escape branch when an ordinary Klingon flees
- **Severity**: High
- **Category**: Latent C Bug / State Corruption
- **Root Cause & Location**: `src/ai.rs` — `tryexit()`
  - `tryexit()` ended with `if ienm == IHS { ... } else { ... state.a.comhere = 0; }`. When `skill > SGOOD` and an ordinary Klingon (`IHK`) fled out of a quadrant that also contained a Commander (`IHC`), the `else` branch updated `d.cx[l], d.cy[l]` to the ordinary Klingon's escape quadrant and cleared `state.a.comhere = 0`, even though the `IHC` ship remained inside `state.quad`.
- **Fix Applied**: Changed `else` to `else if ienm == IHC`.
- **Exact Deterministic Reproduction Steps**:
  1. In quadrant `(4, 4)` (`d.galaxy[4][4] = 200`), set `d.remcom = 1`, `d.cx[1] = 4, d.cy[1] = 4`, `a.comhere = 1`, `a.klhere = 2`, `a.nenhere = 2`, `a.skill = SEXPERT`.
  2. Place `IHK` at slot `1` `(1, 1)` (`kpower[1] = 100.0`) and `IHC` at slot `2` `(8, 8)` (`kpower[2] = 1000.0`, `quad[8][8] = 'C'`).
  3. Trigger `tryexit` for the `IHK` (`ienm = 'K'`) escaping to `(3, 3)` (`movcom` or direct move).
  4. **Before fix**: `d.cx[1], d.cy[1]` becomes `(3, 3)` and `a.comhere` becomes `0`, while `'C'` is still in `quad[8][8]` at `(4, 4)`.
  5. **After fix**: `d.cx[1], d.cy[1]` stays `(4, 4)` and `a.comhere` remains `1`.

---

## BUG-06: Backward time warp corrupts `future[FSCMOVE]` to `0.2777`
- **Severity**: Critical
- **Category**: Latent C Bug / Event Scheduling Corruption
- **Root Cause & Location**: `src/moving.rs` — `timwrp()` (ported from `c_src/moving.c:810`)
  - When a Super-Commander exists (`state.d.nscrem != 0`) and a backward time warp restores the snapshot, `timwrp()` set `state.future[FSCMOVE] = 0.2777;` instead of `state.d.date + 0.2777`. On the next `events()` tick, `future[FSCMOVE]` (~`0.2777`) is ~3100 stardates in the past, jumping `d.date` backward to `0.2777` and corrupting `d.remtime` / `d.remres` by thousands of stardates.
- **Fix Applied**: Changed `state.future[FSCMOVE] = 0.2777;` to `state.future[FSCMOVE] = state.d.date + 0.2777;`.
- **Exact Deterministic Reproduction Steps**:
  1. Initialize a game with `d.nscrem = 1`, `d.date = 3102.0`, `snapsht.date = 3100.0`, `a.snap = 1`.
  2. Call `timwrp(&mut state, &io)` with `rng` producing a backward warp (`rand_f64() < 0.2` or appropriate seed where `d.snap != 0` and backward warp occurs; note `timwrp` takes the backward branch when `d.snap != 0` and `xtime < 0`).
  3. Inspect `state.future[FSCMOVE]`.
  4. **Before fix**: `state.future[FSCMOVE] == 0.2777` (smaller than `state.d.date = 3100.0`).
  5. **After fix**: `state.future[FSCMOVE] == state.d.date + 0.2777` (`3100.2777`).

---

## BUG-07: `warp()` continues executing after energy exhaustion (`FinType::Nrg`)
- **Severity**: Medium
- **Category**: Logic Bug / Post-Death Execution
- **Root Cause & Location**: `src/moving.rs` — `warp()` (and `impuls()`)
  - When warp travel drained `state.a.energy <= 0.0`, `warp()` called `finish(state, io, FinType::Nrg).await` (setting `alldone = 1`, printing final score), and then continued into `timwrp()` and `blooey` engine-damage messages after the game had already ended.
- **Fix Applied**: Added `if state.a.alldone != 0 { return; }` immediately after `finish(state, io, FinType::Nrg).await` in `warp()` (and `impuls()`).
- **Exact Deterministic Reproduction Steps**:
  1. Start a game with `energy = 50.0`, `warpfac = 5.0`, `alldone = 0`.
  2. Execute `move a 5 5` across 2+ sectors (e.g. `dist = 0.5`, cost `62.5 > 50.0`) with `blooey` or `twarp` triggered (or simply observe post-`finish` `state.a.time` / `damage[DWARPEN]`).
  3. **Before fix**: `warp()` sets `state.a.time` and `state.a.ididit = 1` (and prints `Scott here. The warp engines are damaged` if `blooey`) after `finish(FinType::Nrg)` already printed `"TOTAL SCORE"`.
  4. **After fix**: `warp()` returns immediately once `alldone != 0`.

---

## BUG-08: Supernova in Enterprise's quadrant killing last Klingon leaves game unwon and triggers division by zero
- **Severity**: Critical
- **Category**: Latent C Bug / Soft-Lock & NaN/Inf Bug
- **Root Cause & Location**: `src/events.rs` — `snova()` (ported from `c_src/events.c:756`)
  - `snova()` checked `if state.d.remkl == 0 && (nqx != state.a.quadx || nqy != state.a.quady)` before checking `alldone`. If a supernova occurred in the Enterprise's own quadrant (`nqx == quadx && nqy == quady`), the Enterprise survived (`alldone == 0` because no adjacent star), and the supernova destroyed the last Klingon(s) (`d.remkl == 0`), `finish(FinType::Won)` was skipped, `d.remcom` became `0`, and the next `events()` call divided `d.remres / (d.remkl + 4 * d.remcom)` (`0`), producing `inf`/`NaN` in `d.remtime` and soft-locking the game.
- **Fix Applied**: Checked `if state.a.alldone != 0 { finish(state, io, FinType::Snovaed).await; return; }` first, and then `if state.d.remkl == 0 { ... finish(state, io, FinType::Won).await; }` unconditionally regardless of `nqx, nqy`.
- **Exact Deterministic Reproduction Steps**:
  1. Place Enterprise at `(4, 4)` sector `(1, 1)`, one remaining Klingon (`d.remkl = 1, d.remcom = 0`) at `(4, 4)` sector `(10, 10)` (`quad[10][10] = 'K'`), and a star at `(4, 4)` sector `(9, 9)` (distance^2 from Enterprise `(1,1)` is `128 > 2.1`, so Enterprise survives).
  2. Call `snova(&mut state, &io, 4, 4)`.
  3. **Before fix**: `state.d.remkl == 0` and `state.a.alldone == 0` after `snova()` returns; game does not win, and calling `events()` sets `remtime = inf`/`NaN`.
  4. **After fix**: `snova()` announces victory and sets `state.a.alldone = 1`, `state.a.gamewon = 1`.

---

## BUG-09: Stray debug print `"Hit ... energy ..."` in `attack()`
- **Severity**: Medium
- **Category**: Stray Debug Output (Latent C Artifact)
- **Root Cause & Location**: `src/battle.rs` — `attack()` (ported from `c_src/battle.c:642`)
  - Every enemy hit on the Enterprise printed an internal debug line `Hit <hit> energy <energy>\n` between `fry()` and subtracting `hit` from `energy`.
- **Fix Applied**: Removed the `format!("Hit {:.6} energy {:.6}\n", ...)` and `state.proutn(&msg)` lines.
- **Exact Deterministic Reproduction Steps**:
  1. Start a game with a hostile Klingon in the current quadrant and shields down (`shldup = 0`).
  2. Fire weak phasers (`phasers a 10`) or rest (`rest 0.5`) so `attack()` hits the Enterprise.
  3. **Before fix**: Terminal output contains a raw `"Hit ... energy ..."` debug line.
  4. **After fix**: Clean damage output with no stray `"Hit ... energy ..."` line.

---

## BUG-10: `attack()` fixed `1..=nenhere` loop reads `(0, 0)` after mid-loop kill, and `deadkl()` underflow risk
- **Severity**: High
- **Category**: Panic / State Corruption Bug
- **Root Cause & Location**: `src/battle.rs` — `attack()` and `deadkl()`
  - `attack()` iterated `for l in 1..=(state.a.nenhere as usize)`, caching `nenhere` at loop entry. If an enemy torpedo hit and killed another enemy (via `torpedo` -> `deadkl` or `nova`), `nenhere` shrank and trailing `kx`/`ky`/`kpower` were zeroed to `(0, 0, 0.0)`. Subsequent iterations read `kx[l]=0, ky[l]=0, kpower[l]=0.0`, firing ghost enemy torpedoes from `(0, 0)`.
  - Additionally, `deadkl()` unconditionally executed `state.a.nenhere -= 1` without checking `i <= nh && state.a.nenhere > 0`, risking `usize` wrap-around panic (`(-1i32) as usize`) on `state.kx[new_nh + 1]`.
- **Fix Applied**: Replaced the fixed `for` loop in `attack()` with a `while l <= state.a.nenhere as usize` loop that advances `l` at loop top and rewinds `l = idx` only if `kx[idx], ky[idx]` shifted due to a removal at `<= idx`; guarded `deadkl()` with `if i <= nh && state.a.nenhere > 0`.
- **Exact Deterministic Reproduction Steps**:
  1. Set `a.nenhere = 2`, `kx[1]=5, ky[1]=5` (`kpower[1]=500.0`), `kx[2]=5, ky[2]=6` (`kpower[2]=300.0`), Enterprise at `(5, 7)` so enemy 1's torpedo path passes through `(5, 6)`.
  2. Trigger `attack(&mut state, &io, 0)` with `k == 0` (forces torpedo mode) so enemy 1 destroys enemy 2.
  3. **Before fix**: Iteration `l = 2` executes anyway, reads `(kx[2], ky[2]) = (0, 0)` and fires a second `***TORPEDO INCOMING` from Sector `0 - 0`.
  4. **After fix**: `while l <= state.a.nenhere as usize` terminates cleanly after enemy 2 is removed.

---

## BUG-11: Negative `kpower` for emasculated Klingons breaks `capture()` and `selectklingon()`
- **Severity**: Medium
- **Category**: Logic Bug / Combat Mechanics
- **Root Cause & Location**: `src/battle.rs` — `capture()` and `selectklingon()`
  - `hittem()` marks an emasculated Klingon (hit below 200 units when weakened) by negating `kpower[kk] = -kpow`.
  - `capture()` divided `x /= state.kpower[k] * (state.a.nenhere as f64)` using signed `kpower[k]`, yielding a negative probability `prob < 0`, making emasculated Klingons *impossible* to capture (`prob < 5 + 30*Rand()` was always false). Also lacked a zero-power guard.
  - `selectklingon()` compared signed `state.kpower[j] < pow` instead of `state.kpower[j].abs() < pow`, picking `-600` over `-100` when multiple emasculated Klingons existed.
- **Fix Applied**: Used `state.kpower[k].abs().max(1.0)` in `capture()` and `state.kpower[j].abs()` in `selectklingon()`.
- **Exact Deterministic Reproduction Steps**:
  1. Place 1 Klingon (`a.klhere = 1, a.nenhere = 1`) with `kpower[1] = -50.0` (emasculated) and Enterprise with `a.energy = 5000.0`, `damage[DRADIO] = 0.0`, `d.brigfree = 400`.
  2. Issue `capture`.
  3. **Before fix**: `x = 5000 / (-50 * 1) * 2.5 = -250`, `prob = -250`, surrender check `prob > ...` always fails (`"The Klingon refuses to surrender"`).
  4. **After fix**: `x = 5000 / 50 * 2.5 = 250`, `prob = 250`, surrender succeeds.

---

## BUG-12: `kaboom()` skips enemy when `deadkl()` removes slot `l`
- **Severity**: Medium
- **Category**: Latent C Bug / Loop Index Bug
- **Root Cause & Location**: `src/finish.rs` — `kaboom()` (ported from `c_src/finish.c:57-60`)
  - When `state.kpower[l] * state.kdist[l] <= whammo`, `kaboom()` called `deadkl()`, which shifts `l+1..=nenhere` down by 1 and decrements `nenhere`, and then unconditionally incremented `l += 1`, skipping the enemy shifted into slot `l`.
- **Fix Applied**: Changed loop so `l += 1` is only executed in the `else` branch (when `deadkl` was not called).
- **Exact Deterministic Reproduction Steps**:
  1. Place Enterprise with `a.energy = 1000.0` (`whammo = 25000.0`) and 2 Klingons (`nenhere = 2`) both with `kpower = 300.0, kdist = 2.0` (`kpower * kdist = 600 <= 25000`).
  2. Call `kaboom(&mut state, &io)`.
  3. **Before fix**: Slot 1 is killed by `deadkl`, slot 2 shifts to slot 1, `l` becomes `2 > nenhere (1)`, so Klingon 2 survives (`remkl` still 1 instead of 0, depriving player of martyr victory).
  4. **After fix**: Both Klingons are destroyed (`d.remkl == 0`), awarding martyr victory.

---

## BUG-13: `deathray()` prints Spock diagnostic and mutates `damage[DDRAY]` after game-ending `finish(FinType::Won)`
- **Severity**: Low
- **Category**: Control Flow / Post-Game Output Bug
- **Root Cause & Location**: `src/planets.rs` — `deathray()`
  - On the success branch (`r > 0.30`), when `state.d.remkl == 0`, `deathray()` called `finish(state, io, FinType::Won).await` (printing score/plaque and ending the game) and then fell through to print `"Spock- \"Captain, I believe the 'Experimental Death Ray' is still operational / has been rendered dysfunctional.\""` and set `state.damage[DDRAY] = 39.95`.
- **Fix Applied**: Added `return;` immediately after `finish(state, io, FinType::Won).await;`.
- **Exact Deterministic Reproduction Steps**:
  1. Set up a game with 1 Klingon remaining in the current quadrant (`d.remkl = 1, a.nenhere = 1`) and `damage[DDRAY] = 0.0`.
  2. Invoke `deathray` -> `"y"` with an RNG state producing `r > 0.30`.
  3. **Before fix**: After `"You have smashed the Klingon invasion fleet..."` and `"TOTAL SCORE"`, output erroneously appends Spock's post-game death ray status line.
  4. **After fix**: `deathray()` returns immediately after `finish(FinType::Won)`.

---

## BUG-14: Web UI calls `importAllSavedGames()` before `trek_init()`, discarding all `localStorage` saves
- **Severity**: Critical
- **Category**: Web / WASM Integration Bug
- **Root Cause & Location**: `web/app.js` — `startNewGame()`
  - `startNewGame()` called `importAllSavedGames()` *before* `state.wasm.trek_init(seed, coordFixed, 0, 0)`.
  - On initial page load, `RUNTIME` in `src/wasm.rs` is `None`, so `trek_import_save` returns `0` and imports nothing.
  - On subsequent `New Game` or `Resume / New Session` clicks, `importAllSavedGames()` imported saves into the *old* `TrekSession`, which was immediately overwritten by `trek_init(...)` creating a fresh session with an empty `saved_games` map.
  - Consequently, loading any previously saved `.trk` game (including `emsave.trk` after `emexit`) via `frozen <name>` always failed with `"Can't find game file"` after a page refresh, `New Game`, or `emexit`.
- **Fix Applied**: Moved `importAllSavedGames();` to execute immediately *after* `state.wasm.trek_init(...)` inside `startNewGame()`.
- **Exact Deterministic Reproduction Steps**:
  1. Start a game, run `freeze mygame` (persisted to `localStorage` under `trek.save.mygame.trk`), then click `New Game` (or trigger `emexit` and click `Resume / New Session`, or reload page).
  2. At `"Would you like a regular, tournament, or frozen game?"`, enter `frozen mygame`.
  3. **Before fix**: Output prints `Can't find game file mygame.trk`.
  4. **After fix**: `mygame.trk` is imported into the newly initialized `TrekSession` and resumes cleanly.

---

## BUG-15: Web UI Saves Manager `Insert` button fails from `emexit`/game-over state & missing sector symbols in HUD
- **Severity**: Medium
- **Category**: Web UI / UX Bug
- **Root Cause & Location**: `web/app.js` (`renderSavesList`, `SYMBOL_NAMES`) & `web/styles.css`
  - In `renderSavesList()`, clicking `Insert` on a saved game while `state.emexit` (Boss screen) or `state.promptType === 'exited'` was active only wrote `frozen <name>` into the disabled `#cmd-input` and closed the dialog without starting a fresh session to accept the command.
  - `SYMBOL_NAMES` and `.quad-cell[data-sym=...]` lacked labels/styles for `'T'` (Tholian), `'#'` (Tholian Web), `'?'` (Space Thing), and `'-'` (Unscanned when SR sensors are damaged).
- **Fix Applied**: Updated `Insert` click handler to call `startNewGame()` first when `state.emexit || state.promptType === 'exited'`; added `'T'`, `'#'`, `'?'`, and `'-'` entries to `SYMBOL_NAMES` in `web/app.js` and matching styles in `web/styles.css`.
- **Exact Deterministic Reproduction Steps**:
  1. Run `emexit` in Web UI (entering Boss Screen where `state.emexit === true` and `#cmd-input.disabled === true`).
  2. Open `Saves` dialog and click `Insert` next to `emsave.trk`.
  3. **Before fix**: Dialog closes, Boss Screen remains visible, `#cmd-input` remains disabled and unusable.
  4. **After fix**: `startNewGame()` exits Boss Screen, imports `emsave.trk`, enables `#cmd-input`, and pre-fills `frozen emsave.trk` ready to submit.

---

## BUG-16: `deadkl()` early return on `remkl == 0` skips `nenhere` decrement and `kx`/`ky` cleanup, causing infinite loops in `kaboom()` and `deathray()`
- **Severity**: Critical
- **Category**: Latent C Bug / Infinite Loop & State Corruption
- **Root Cause & Location**: `src/battle.rs` — `deadkl()` (ported from `c_src/battle.c:755`)
  - After destroying the last Klingon (`state.d.remkl == 0`) and clearing `state.quad[ix][iy] = IHDOT`, `deadkl()` returned early before decrementing `state.a.nenhere` and shifting/clearing `kx`, `ky`, `kpower`, `kdist`, and `kavgd` solely to avoid dividing `remres` by `remkl + 4 * remcom == 0`.
  - Meanwhile, the non-enemy fallback arm (`_ =>`) also decremented `nenhere` when `nenhere > 1` instead of returning immediately like C `default: return;`.
  - When `deathray()` (`while state.a.nenhere > 0 { deadkl(kx[1], ky[1], quad[kx[1]][ky[1]], ...); }`) or `kaboom()` (`while l <= state.a.nenhere { ... deadkl(...); }`) destroyed the last Klingon (`d.remkl` reached `0`), `quad[kx[1]][ky[1]]` became `IHDOT` while `state.a.nenhere` remained `>= 1`, spinning forever in an infinite loop.
- **Fix Applied**:
  - Removed the early `if state.d.remkl == 0 { return; }` before array cleanup and guarded only the `remtime` division with `let denom = state.d.remkl + 4 * state.d.remcom; if denom > 0 { state.d.remtime = state.d.remres / (denom as f64); }`, so `nenhere` is always decremented and `kx`/`ky`/`kpower`/`kdist`/`kavgd` are always shifted/zeroed even when `remkl == 0`.
  - Changed the non-enemy fallback arm `_ =>` in `deadkl()` to `return` immediately without modifying `nenhere`.
- **Exact Deterministic Reproduction Steps**:
  1. **`deathray()` case**: Set `d.remkl = 1`, `a.klhere = 1`, `a.nenhere = 1`, `kx[1] = 3, ky[1] = 3`, `quad[3][3] = IHK`, `damage[DDRAY] = 0.0`. Call `deathray()` and confirm `"y"` with an RNG seed yielding `r > 0.30`.
     - **Before fix**: `deadkl()` sets `remkl = 0` and `quad[3][3] = IHDOT` and returns early, leaving `nenhere = 1`; `while state.a.nenhere > 0` loops infinitely.
     - **After fix**: `nenhere` becomes `0`, `kx[1]`/`ky[1]` are cleared, and `deathray()` invokes `finish(FinType::Won)` and exits cleanly.
  2. **`kaboom()` case**: Set `d.remkl = 1`, `a.klhere = 1`, `a.nenhere = 1`, `a.energy = 1000.0`, `kx[1] = 3, ky[1] = 3`, `kpower[1] = 300.0, kdist[1] = 2.0`, `quad[3][3] = IHK`. Call `kaboom(&mut state, &io)`.
     - **Before fix**: `deadkl()` leaves `nenhere = 1` and `quad[3][3] = IHDOT`; `kaboom()` loops infinitely at `l = 1` without incrementing `l`.
     - **After fix**: `nenhere` becomes `0`, `kaboom()` proceeds to `finish(FinType::Dilithium)`, awarding martyr victory.

