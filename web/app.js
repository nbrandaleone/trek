const SAVE_PREFIX = 'trek.save.';
const THEME_KEY = 'trek.theme';
const COORD_FIXED_KEY = 'trek.coord_fixed';
const PLAQUE_KEY = 'trek.plaque.latest';
const MAX_TERMINAL_CHARS = 240000;
const HISTORY_LIMIT = 200;

const SYMBOL_NAMES = {
  E: 'U.S.S. Enterprise',
  F: 'Faerie Queene',
  B: 'Starbase',
  K: 'Klingon',
  C: 'Klingon Commander',
  S: 'Super-Commander',
  R: 'Romulan',
  P: 'Planet',
  T: 'Tholian',
  '#': 'Tholian Web',
  '?': 'Space Thing',
  '-': 'Unscanned (SR sensors damaged)',
  '*': 'Star',
  '.': 'Empty space',
  ' ': 'Black hole or unscanned',
};

const CONDITION_STYLES = {
  GREEN: { cls: 'condition-green', symbol: '●', label: 'GREEN' },
  YELLOW: { cls: 'condition-yellow', symbol: '▲', label: 'YELLOW' },
  RED: { cls: 'condition-red', symbol: '◆', label: 'RED' },
  DOCKED: { cls: 'condition-docked', symbol: '■', label: 'DOCKED' },
  CLOAKED: { cls: 'condition-cloaked', symbol: '◌', label: 'CLOAKED' },
};

const encoder = new TextEncoder();
const decoder = new TextDecoder('utf-8');

const state = {
  wasm: null,
  ready: false,
  promptType: 'line',
  emexit: false,
  isComposing: false,
  history: [],
  historyIndex: -1,
  historyDraft: '',
  manualLoaded: false,
  manualSections: [],
  latestPlaque: null,
  coordFixed: false,
};

const dom = {
  wasmStatus: document.getElementById('wasm-status'),
  wasmStatusText: document.getElementById('wasm-status-text'),
  newGameBtn: document.getElementById('new-game-btn'),
  coordFixedToggle: document.getElementById('coord-fixed-toggle'),
  coordModeLabel: document.getElementById('coord-mode-label'),
  themeSelect: document.getElementById('theme-select'),
  openManualBtn: document.getElementById('open-manual-btn'),
  openSavesBtn: document.getElementById('open-saves-btn'),
  openPlaqueBtn: document.getElementById('open-plaque-btn'),
  plaqueBadge: document.getElementById('plaque-badge'),

  bossScreen: document.getElementById('boss-screen'),
  bossResumeBtn: document.getElementById('boss-resume-btn'),
  terminalOutput: document.getElementById('terminal-output'),
  terminalCode: document.getElementById('terminal-code'),
  continueBanner: document.getElementById('continue-banner'),
  continueBtn: document.getElementById('continue-btn'),
  exitedBanner: document.getElementById('exited-banner'),
  exitedNewGameBtn: document.getElementById('exited-new-game-btn'),
  cmdForm: document.getElementById('cmd-form'),
  cmdInput: document.getElementById('cmd-input'),
  cmdSubmit: document.getElementById('cmd-submit'),
  quickCmdsGrid: document.getElementById('quick-cmds-grid'),

  hudCondition: document.getElementById('hud-condition'),
  hudConditionSymbol: document.getElementById('hud-condition-symbol'),
  hudConditionText: document.getElementById('hud-condition-text'),
  hudShip: document.getElementById('hud-ship'),
  hudStardate: document.getElementById('hud-stardate'),
  hudTimeLeft: document.getElementById('hud-time-left'),
  hudQuadrant: document.getElementById('hud-quadrant'),
  hudSector: document.getElementById('hud-sector'),
  hudWarp: document.getElementById('hud-warp'),
  hudTorps: document.getElementById('hud-torps'),
  hudLifeSupport: document.getElementById('hud-life-support'),
  hudEnergyBar: document.getElementById('hud-energy-bar'),
  hudEnergyText: document.getElementById('hud-energy-text'),
  hudShieldBar: document.getElementById('hud-shield-bar'),
  hudShieldText: document.getElementById('hud-shield-text'),
  hudShieldState: document.getElementById('hud-shield-state'),
  hudKlingons: document.getElementById('hud-klingons'),
  hudCommanders: document.getElementById('hud-commanders'),
  hudSuperCmdr: document.getElementById('hud-super-cmdr'),
  hudRomulans: document.getElementById('hud-romulans'),
  hudBases: document.getElementById('hud-bases'),
  hudProbes: document.getElementById('hud-probes'),
  hudBrig: document.getElementById('hud-brig'),
  hudCaptured: document.getElementById('hud-captured'),
  hudCasualties: document.getElementById('hud-casualties'),
  flagCrystals: document.getElementById('flag-crystals'),
  flagOrbit: document.getElementById('flag-orbit'),
  flagCloaked: document.getElementById('flag-cloaked'),
  quadGrid: document.getElementById('quad-grid'),
  devicesList: document.getElementById('devices-list'),
  starChartHead: document.getElementById('star-chart-head'),
  starChartBody: document.getElementById('star-chart-body'),
  srAnnouncer: document.getElementById('sr-announcer'),

  manualDialog: document.getElementById('manual-dialog'),
  manualSectionSelect: document.getElementById('manual-section-select'),
  manualSearchInput: document.getElementById('manual-search-input'),
  manualFilterForm: document.getElementById('manual-filter-form'),
  manualMatchStatus: document.getElementById('manual-match-status'),
  manualContent: document.getElementById('manual-content'),

  savesDialog: document.getElementById('saves-dialog'),
  uploadTrkInput: document.getElementById('upload-trk-input'),
  savesStatus: document.getElementById('saves-status'),
  savesList: document.getElementById('saves-list'),

  plaqueDialog: document.getElementById('plaque-dialog'),
  plaqueEmptyMsg: document.getElementById('plaque-empty-msg'),
  plaqueContent: document.getElementById('plaque-content'),
  plaqueDownloadBtn: document.getElementById('plaque-download-btn'),
  plaquePrintBtn: document.getElementById('plaque-print-btn'),
};

function announce(message) {
  dom.srAnnouncer.textContent = '';
  requestAnimationFrame(() => {
    dom.srAnnouncer.textContent = message;
  });
}

function setWasmStatus(statusState, text) {
  dom.wasmStatus.dataset.state = statusState;
  dom.wasmStatusText.textContent = text;
}

function randomSeed32() {
  if (globalThis.crypto && typeof globalThis.crypto.getRandomValues === 'function') {
    const arr = new Uint32Array(1);
    globalThis.crypto.getRandomValues(arr);
    return arr[0] >>> 0;
  }
  return (Math.floor(Math.random() * 0xffffffff) >>> 0);
}

async function instantiateTrekWasm() {
  const imports = {};
  const response = await fetch('./trek.wasm');
  if (!response.ok) {
    throw new Error(`HTTP ${response.status} loading ./trek.wasm`);
  }

  if (typeof WebAssembly.instantiateStreaming === 'function') {
    const contentType = response.headers.get('Content-Type') || '';
    if (contentType.includes('application/wasm')) {
      try {
        const result = await WebAssembly.instantiateStreaming(response.clone(), imports);
        return result.instance.exports;
      } catch (_streamingErr) {
        // Fall through to ArrayBuffer instantiation.
      }
    }
  }

  const bytes = await response.arrayBuffer();
  const result = await WebAssembly.instantiate(bytes, imports);
  return result.instance.exports;
}

function withWasmBytes(str, fn) {
  const bytes = encoder.encode(str);
  if (bytes.length === 0) {
    return fn(0, 0);
  }
  const ptr = state.wasm.trek_alloc(bytes.length);
  try {
    const mem = new Uint8Array(state.wasm.memory.buffer, ptr, bytes.length);
    mem.set(bytes);
    return fn(ptr, bytes.length);
  } finally {
    state.wasm.trek_dealloc(ptr, bytes.length);
  }
}

function withTwoWasmStrings(strA, strB, fn) {
  const bytesA = encoder.encode(strA);
  const bytesB = encoder.encode(strB);
  const ptrA = bytesA.length > 0 ? state.wasm.trek_alloc(bytesA.length) : 0;
  let ptrB = 0;
  try {
    if (bytesA.length > 0) {
      new Uint8Array(state.wasm.memory.buffer, ptrA, bytesA.length).set(bytesA);
    }
    ptrB = bytesB.length > 0 ? state.wasm.trek_alloc(bytesB.length) : 0;
    if (bytesB.length > 0) {
      new Uint8Array(state.wasm.memory.buffer, ptrB, bytesB.length).set(bytesB);
    }
    return fn(ptrA, bytesA.length, ptrB, bytesB.length);
  } finally {
    if (bytesB.length > 0 && ptrB !== 0) {
      state.wasm.trek_dealloc(ptrB, bytesB.length);
    }
    if (bytesA.length > 0 && ptrA !== 0) {
      state.wasm.trek_dealloc(ptrA, bytesA.length);
    }
  }
}

function readWasmResponse() {
  const ptr = state.wasm.trek_response_ptr();
  const len = state.wasm.trek_response_len();
  if (!len) {
    return null;
  }
  const view = new Uint8Array(state.wasm.memory.buffer, ptr, len);
  const jsonText = decoder.decode(view);
  return JSON.parse(jsonText);
}

function importAllSavedGames() {
  if (!state.wasm) return;
  const keys = [];
  for (let i = 0; i < localStorage.length; i += 1) {
    const k = localStorage.key(i);
    if (k && k.startsWith(SAVE_PREFIX)) {
      keys.push(k);
    }
  }
  for (const k of keys) {
    const name = k.slice(SAVE_PREFIX.length);
    const data = localStorage.getItem(k);
    if (name && data !== null) {
      withTwoWasmStrings(name, data, (nPtr, nLen, dPtr, dLen) => {
        state.wasm.trek_import_save(nPtr, nLen, dPtr, dLen);
      });
    }
  }
}

function appendTerminalText(text) {
  if (!text) return;
  let next = dom.terminalCode.textContent + text;
  if (next.length > MAX_TERMINAL_CHARS) {
    const cut = next.indexOf('\n', next.length - MAX_TERMINAL_CHARS);
    next = cut >= 0 ? next.slice(cut + 1) : next.slice(next.length - MAX_TERMINAL_CHARS);
  }
  dom.terminalCode.textContent = next;
  dom.terminalOutput.scrollTop = dom.terminalOutput.scrollHeight;
}

function setTerminalText(text) {
  dom.terminalCode.textContent = text || '';
  dom.terminalOutput.scrollTop = dom.terminalOutput.scrollHeight;
}

function persistSaves(savesUpdated) {
  if (!Array.isArray(savesUpdated) || savesUpdated.length === 0) return;
  for (const entry of savesUpdated) {
    if (entry && typeof entry.name === 'string' && typeof entry.data === 'string') {
      try {
        localStorage.setItem(SAVE_PREFIX + entry.name, entry.data);
      } catch (_err) {
        // Ignore storage quota errors gracefully.
      }
    }
  }
  if (dom.savesDialog.open) {
    renderSavesList();
  }
}

function persistPlaques(plaquesUpdated) {
  if (!Array.isArray(plaquesUpdated) || plaquesUpdated.length === 0) return;
  const latest = plaquesUpdated[plaquesUpdated.length - 1];
  if (latest && typeof latest.content === 'string') {
    state.latestPlaque = {
      name: latest.name || 'plaque.txt',
      content: latest.content,
    };
    try {
      localStorage.setItem(PLAQUE_KEY, JSON.stringify(state.latestPlaque));
    } catch (_err) {
      // Ignore storage quota errors.
    }
    updatePlaqueUI();
    announce('Commodore Emeritus Citation Plaque awarded. Open Plaque to view or print.');
  }
}

function updatePlaqueUI() {
  const hasPlaque = Boolean(state.latestPlaque && state.latestPlaque.content);
  dom.plaqueBadge.hidden = !hasPlaque;
  dom.plaqueEmptyMsg.hidden = hasPlaque;
  dom.plaqueContent.hidden = !hasPlaque;
  dom.plaqueDownloadBtn.disabled = !hasPlaque;
  dom.plaquePrintBtn.disabled = !hasPlaque;
  if (hasPlaque) {
    dom.plaqueContent.textContent = state.latestPlaque.content;
  } else {
    dom.plaqueContent.textContent = '';
  }
}

function parseQuadRow(row) {
  if (typeof row !== 'string') return Array(10).fill('.');
  if (row.length >= 19) {
    const cells = [];
    for (let i = 0; i < 10; i += 1) {
      const idx = i * 2;
      cells.push(idx < row.length ? row[idx] : '.');
    }
    if (cells.every((c) => c.length === 1)) {
      return cells;
    }
  }
  const tokens = row.trim().split(/\s+/).filter(Boolean);
  if (tokens.length === 10) {
    return tokens.map((t) => t[0] || '.');
  }
  const compact = row.replace(/\s+/g, '');
  if (compact.length >= 10) {
    return compact.slice(0, 10).split('');
  }
  return Array.from({ length: 10 }, (_, idx) => compact[idx] || '.');
}

function renderQuadGrid(quadRows, coordFixed) {
  const rows = Array.isArray(quadRows) && quadRows.length === 10
    ? quadRows
    : Array(10).fill('. . . . . . . . . .');

  const frag = document.createDocumentFragment();

  const corner = document.createElement('div');
  corner.className = 'quad-axis';
  corner.textContent = coordFixed ? 'Y\\X' : 'R\\C';
  corner.setAttribute('aria-hidden', 'true');
  frag.appendChild(corner);

  for (let c = 1; c <= 10; c += 1) {
    const colHdr = document.createElement('div');
    colHdr.className = 'quad-axis';
    colHdr.textContent = String(c);
    colHdr.setAttribute('aria-hidden', 'true');
    frag.appendChild(colHdr);
  }

  for (let rIdx = 0; rIdx < 10; rIdx += 1) {
    const displayRow = coordFixed ? 10 - rIdx : rIdx + 1;
    const rowHdr = document.createElement('div');
    rowHdr.className = 'quad-axis';
    rowHdr.textContent = String(displayRow);
    rowHdr.setAttribute('aria-hidden', 'true');
    frag.appendChild(rowHdr);

    const cells = parseQuadRow(rows[rIdx]);
    for (let cIdx = 0; cIdx < 10; cIdx += 1) {
      const displayCol = cIdx + 1;
      const sym = cells[cIdx] ?? '.';
      const btn = document.createElement('button');
      btn.type = 'button';
      btn.className = 'quad-cell';
      btn.dataset.sym = sym;

      const arg1 = coordFixed ? displayCol : displayRow;
      const arg2 = coordFixed ? displayRow : displayCol;
      const symDesc = SYMBOL_NAMES[sym] || `Object ${sym}`;
      const coordDesc = coordFixed
        ? `Sector X ${arg1} Y ${arg2}`
        : `Sector row ${arg1} column ${arg2}`;

      btn.setAttribute('aria-label', `${coordDesc}: ${symDesc}. Click to insert coordinates.`);
      btn.textContent = sym === ' ' ? '◌' : sym;
      btn.addEventListener('click', () => insertCoordsIntoInput(arg1, arg2));
      frag.appendChild(btn);
    }
  }

  dom.quadGrid.replaceChildren(frag);
}

function insertCoordsIntoInput(first, second) {
  if (dom.cmdInput.disabled) return;
  const pair = `${first} ${second}`;
  const current = dom.cmdInput.value;
  if (!current.trim()) {
    dom.cmdInput.value = pair;
  } else if (/\s$/.test(current)) {
    dom.cmdInput.value = `${current}${pair}`;
  } else {
    dom.cmdInput.value = `${current} ${pair}`;
  }
  dom.cmdInput.focus();
  const end = dom.cmdInput.value.length;
  dom.cmdInput.setSelectionRange(end, end);
}

function renderDevices(devices) {
  if (!Array.isArray(devices) || devices.length === 0) {
    dom.devicesList.innerHTML = '<li class="device-nominal">No device telemetry reported.</li>';
    return;
  }
  const damaged = devices.filter((d) => Number(d.damage) > 0);
  const frag = document.createDocumentFragment();
  if (damaged.length === 0) {
    const li = document.createElement('li');
    li.className = 'device-nominal';
    li.textContent = 'All ship systems nominal (0.0 damage)';
    frag.appendChild(li);
  } else {
    for (const dev of damaged) {
      const li = document.createElement('li');
      li.className = 'device-damaged';
      const nameSpan = document.createElement('span');
      nameSpan.textContent = dev.name || `Device ${dev.id}`;
      const dmgSpan = document.createElement('span');
      dmgSpan.textContent = `${Number(dev.damage).toFixed(2)} SD`;
      li.append(nameSpan, dmgSpan);
      frag.appendChild(li);
    }
  }
  dom.devicesList.replaceChildren(frag);
}

function renderStarChart(chartRows, quadx, quady, coordFixed) {
  const headRow = document.createElement('tr');
  const corner = document.createElement('th');
  corner.scope = 'col';
  corner.textContent = coordFixed ? 'Y\\X' : 'Q';
  headRow.appendChild(corner);

  for (let c = 1; c <= 8; c += 1) {
    const th = document.createElement('th');
    th.scope = 'col';
    th.textContent = String(c);
    headRow.appendChild(th);
  }
  dom.starChartHead.replaceChildren(headRow);

  const bodyFrag = document.createDocumentFragment();
  for (let rIdx = 0; rIdx < 8; rIdx += 1) {
    const tr = document.createElement('tr');
    const rowLabelNum = coordFixed ? 8 - rIdx : rIdx + 1;
    const th = document.createElement('th');
    th.scope = 'row';
    th.textContent = String(rowLabelNum);
    tr.appendChild(th);

    const rowData = Array.isArray(chartRows) && Array.isArray(chartRows[rIdx])
      ? chartRows[rIdx]
      : Array(8).fill('...');

    for (let cIdx = 0; cIdx < 8; cIdx += 1) {
      const td = document.createElement('td');
      const val = String(rowData[cIdx] ?? '...').trim() || '...';
      td.textContent = val;

      const isCurrent = coordFixed
        ? (cIdx + 1 === quadx && rowLabelNum === quady)
        : (rowLabelNum === quadx && cIdx + 1 === quady);

      if (isCurrent) {
        td.classList.add('chart-current');
        td.setAttribute('aria-current', 'location');
      }
      if (val === '1000' || val === '***') {
        td.classList.add('chart-supernova');
      }
      tr.appendChild(td);
    }
    bodyFrag.appendChild(tr);
  }
  dom.starChartBody.replaceChildren(bodyFrag);
}

function updateHud(hud) {
  if (!hud) return;

  state.coordFixed = Boolean(hud.coord_fixed);
  dom.coordFixedToggle.checked = state.coordFixed;
  dom.coordModeLabel.textContent = state.coordFixed
    ? 'Coords: -f Fixed X-Axis (X, Y)'
    : 'Coords: Standard (Row, Col)';

  const condKey = hud.cloaked
    ? 'CLOAKED'
    : String(hud.condition || 'GREEN').toUpperCase();
  const condStyle = CONDITION_STYLES[condKey] || CONDITION_STYLES.GREEN;
  dom.hudCondition.className = `condition-badge ${condStyle.cls}`;
  dom.hudConditionSymbol.textContent = condStyle.symbol;
  dom.hudConditionText.textContent = condStyle.label;

  dom.hudShip.textContent = hud.ship || 'Enterprise';
  dom.hudStardate.textContent = Number(hud.stardate ?? 0).toFixed(1);
  dom.hudTimeLeft.textContent = Number(hud.time_left ?? 0).toFixed(2);
  dom.hudTimeLeft.className = Number(hud.time_left) < 1.5 ? 'text-danger' : '';

  dom.hudQuadrant.textContent = `${hud.quadx ?? '?'} - ${hud.quady ?? '?'}`;
  dom.hudSector.textContent = `${hud.sectx ?? '?'} - ${hud.secty ?? '?'}`;
  dom.hudWarp.textContent = Number(hud.warp_factor ?? 0).toFixed(1);
  dom.hudTorps.textContent = `${hud.torps ?? 0} / ${hud.max_torps ?? 10}`;

  if (hud.life_support_active) {
    dom.hudLifeSupport.textContent = 'ACTIVE';
    dom.hudLifeSupport.className = '';
  } else {
    dom.hudLifeSupport.textContent = `RESERVES ${Number(hud.life_support_reserves ?? 0).toFixed(2)}`;
    dom.hudLifeSupport.className = 'text-danger';
  }

  const maxEnergy = Math.max(1, Number(hud.max_energy ?? 5000));
  const energy = Math.max(0, Number(hud.energy ?? 0));
  dom.hudEnergyBar.max = maxEnergy;
  dom.hudEnergyBar.value = Math.min(maxEnergy, energy);
  dom.hudEnergyText.textContent = `${energy.toFixed(0)} / ${maxEnergy.toFixed(0)}`;

  const maxShield = Math.max(1, Number(hud.max_shield ?? 2500));
  const shield = Math.max(0, Number(hud.shield ?? 0));
  dom.hudShieldBar.max = maxShield;
  dom.hudShieldBar.value = Math.min(maxShield, shield);
  const shieldPct = Math.round((shield / maxShield) * 100);
  dom.hudShieldText.textContent = `${shield.toFixed(0)} / ${maxShield.toFixed(0)} (${shieldPct}%)`;

  if (hud.shield_damaged) {
    dom.hudShieldState.textContent = '⚠ DAMAGED';
    dom.hudShieldState.dataset.state = 'damaged';
  } else if (hud.shields_up) {
    dom.hudShieldState.textContent = '▲ UP';
    dom.hudShieldState.dataset.state = 'up';
  } else {
    dom.hudShieldState.textContent = '▽ DOWN';
    dom.hudShieldState.dataset.state = 'down';
  }

  dom.hudKlingons.textContent = String(hud.klingons_left ?? 0);
  dom.hudCommanders.textContent = String(hud.commanders_left ?? 0);
  dom.hudSuperCmdr.textContent = String(hud.super_commander_left ?? 0);
  dom.hudRomulans.textContent = String(hud.romulans_left ?? 0);
  dom.hudBases.textContent = String(hud.bases_left ?? 0);
  dom.hudProbes.textContent = String(hud.probes_left ?? 0);
  dom.hudBrig.textContent = `${hud.brig_captives ?? 0} / ${hud.brig_capacity ?? 400}`;
  dom.hudCaptured.textContent = String(hud.kcaptured ?? 0);
  dom.hudCasualties.textContent = String(hud.casualties ?? 0);

  dom.flagCrystals.textContent = hud.crystals_aboard ? '◆ Crystals Aboard' : '◇ No Crystals';
  dom.flagCrystals.dataset.active = String(Boolean(hud.crystals_aboard));

  if (hud.landed) {
    dom.flagOrbit.textContent = '▼ Landed on Planet';
    dom.flagOrbit.dataset.active = 'true';
  } else if (hud.in_orbit) {
    dom.flagOrbit.textContent = '◎ Standard Orbit';
    dom.flagOrbit.dataset.active = 'true';
  } else {
    dom.flagOrbit.textContent = '○ Free Space';
    dom.flagOrbit.dataset.active = 'false';
  }

  dom.flagCloaked.hidden = !hud.cloaked;
  dom.flagCloaked.dataset.active = String(Boolean(hud.cloaked));

  renderQuadGrid(hud.quad_rows, state.coordFixed);
  renderDevices(hud.devices);
  renderStarChart(hud.chart_rows, hud.quadx, hud.quady, state.coordFixed);
}

function updatePromptControls(promptType, emexit) {
  state.promptType = promptType || 'line';
  state.emexit = Boolean(emexit);

  const isLine = state.promptType === 'line' && !state.emexit;
  const isKey = state.promptType === 'key' && !state.emexit;
  const isExited = state.promptType === 'exited' || state.emexit;

  dom.bossScreen.hidden = !state.emexit;
  dom.terminalOutput.hidden = state.emexit;

  dom.continueBanner.hidden = !isKey;
  dom.exitedBanner.hidden = !(isExited && !state.emexit);

  dom.cmdInput.disabled = !isLine;
  dom.cmdSubmit.disabled = !isLine;

  for (const btn of dom.quickCmdsGrid.querySelectorAll('button')) {
    btn.disabled = !isLine;
  }

  if (isKey && !anyDialogOpen()) {
    dom.continueBtn.focus();
  } else if (isLine && !anyDialogOpen() && document.activeElement === document.body) {
    dom.cmdInput.focus();
  }
}

function applyWasmResponse(res, { replaceTerminal = false } = {}) {
  if (!res) return;

  persistSaves(res.saves_updated);
  persistPlaques(res.plaques_updated);

  if (res.emexit) {
    setTerminalText('');
  } else if (res.clear_screen || replaceTerminal) {
    setTerminalText(res.output || '');
  } else if (res.output) {
    appendTerminalText(res.output);
  }

  if (res.hud) {
    updateHud(res.hud);
  }

  updatePromptControls(res.prompt_type, res.emexit);
}

function startNewGame() {
  if (!state.wasm) return;
  dom.bossScreen.hidden = true;
  dom.terminalOutput.hidden = false;

  const seed = randomSeed32();
  const coordFixed = dom.coordFixedToggle.checked ? 1 : 0;
  state.wasm.trek_init(seed, coordFixed, 0, 0);
  importAllSavedGames();
  const res = readWasmResponse();
  applyWasmResponse(res, { replaceTerminal: true });
  if (state.promptType === 'line') {
    dom.cmdInput.focus();
  }
}

function sendLineCommand(rawCommand) {
  if (!state.ready || state.promptType !== 'line' || state.emexit) return;
  const cmd = rawCommand;
  appendTerminalText(`> ${cmd}\n`);

  const trimmed = cmd.trim();
  if (trimmed && state.history[state.history.length - 1] !== trimmed) {
    state.history.push(trimmed);
    if (state.history.length > HISTORY_LIMIT) {
      state.history.shift();
    }
  }
  state.historyIndex = -1;
  state.historyDraft = '';

  withWasmBytes(cmd, (ptr, len) => {
    state.wasm.trek_step(ptr, len, 0);
  });
  const res = readWasmResponse();
  applyWasmResponse(res);
}

function sendKeyCommand(keyChar = ' ') {
  if (!state.ready || state.promptType !== 'key' || state.emexit) return;
  withWasmBytes(keyChar, (ptr, len) => {
    state.wasm.trek_step(ptr, len, 1);
  });
  const res = readWasmResponse();
  applyWasmResponse(res);
  if (state.promptType === 'line' && !anyDialogOpen()) {
    dom.cmdInput.focus();
  }
}

function anyDialogOpen() {
  return Boolean(document.querySelector('dialog[open]'));
}

/* --- Native <dialog> Safari closedby="any" fallback --- */
function installDialogLightDismissFallback() {
  if ('closedBy' in HTMLDialogElement.prototype) return;

  const isOutside = (e, dialog) => {
    const r = dialog.getBoundingClientRect();
    return e.clientX < r.left || e.clientX > r.right || e.clientY < r.top || e.clientY > r.bottom;
  };

  for (const dialog of document.querySelectorAll('dialog[closedby="any"]')) {
    let pressedOutside = false;
    dialog.addEventListener('pointerdown', (e) => {
      pressedOutside = isOutside(e, dialog);
    });
    dialog.addEventListener('pointerup', (e) => {
      if (pressedOutside && isOutside(e, dialog)) {
        dialog.close();
      }
      pressedOutside = false;
    });
  }
}

/* --- Operations Manual (SST.DOC) --- */
function parseManualSections(docText) {
  const clean = docText.replace(/\r\n/g, '\n');
  const lines = clean.split('\n');
  const sections = [];
  let current = { id: 'HEADER', title: 'Cover & Copyright', lines: [] };

  const majorRe = /^\f?\s*-{4,}([A-Z0-9 /()-]+?)-{4,}/;

  for (let i = 0; i < lines.length; i += 1) {
    const line = lines[i];
    const majorMatch = line.match(majorRe);
    if (majorMatch) {
      if (current.lines.length > 0) {
        sections.push(current);
      }
      const title = majorMatch[1].trim();
      current = {
        id: `SEC_${sections.length}`,
        title,
        lines: [line.replace(/\f/g, '')],
      };
      continue;
    }

    if (
      i + 2 < lines.length &&
      /^\s*\*{5,}\s*$/.test(line) &&
      /^\s*\*\s+([^*]+?)\s+\*\s*$/.test(lines[i + 1]) &&
      /^\s*\*{5,}\s*$/.test(lines[i + 2])
    ) {
      if (current.lines.length > 0) {
        sections.push(current);
      }
      const cmdTitle = lines[i + 1].match(/^\s*\*\s+([^*]+?)\s+\*\s*$/)[1].trim();
      current = {
        id: `CMD_${sections.length}`,
        title: `Command: ${cmdTitle}`,
        lines: [line, lines[i + 1], lines[i + 2]],
      };
      i += 2;
      continue;
    }

    current.lines.push(line.replace(/\f/g, ''));
  }

  if (current.lines.length > 0) {
    sections.push(current);
  }
  return sections;
}

async function ensureManualLoaded() {
  if (state.manualLoaded) return;
  dom.manualContent.textContent = 'Fetching ./sst.doc…';
  try {
    const res = await fetch('./sst.doc');
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    const text = await res.text();
    state.manualSections = parseManualSections(text);
    state.manualLoaded = true;

    const frag = document.createDocumentFragment();
    const allOpt = document.createElement('option');
    allOpt.value = 'ALL';
    allOpt.textContent = `All Sections (${state.manualSections.length})`;
    frag.appendChild(allOpt);

    for (const sec of state.manualSections) {
      const opt = document.createElement('option');
      opt.value = sec.id;
      opt.textContent = sec.title;
      frag.appendChild(opt);
    }
    dom.manualSectionSelect.replaceChildren(frag);
    renderManualView();
  } catch (err) {
    dom.manualContent.textContent = `Failed to load ./sst.doc: ${err.message}`;
  }
}

function renderManualView() {
  if (!state.manualLoaded) return;
  const selectedId = dom.manualSectionSelect.value;
  const query = dom.manualSearchInput.value.trim().toLowerCase();

  const candidateSections = selectedId === 'ALL'
    ? state.manualSections
    : state.manualSections.filter((s) => s.id === selectedId);

  if (!query) {
    const combined = candidateSections.map((s) => s.lines.join('\n')).join('\n\n');
    dom.manualContent.textContent = combined;
    dom.manualMatchStatus.textContent = `Showing ${candidateSections.length} section(s).`;
    return;
  }

  const matchedSections = [];
  let matchingLineCount = 0;
  for (const sec of candidateSections) {
    const secText = sec.lines.join('\n');
    if (sec.title.toLowerCase().includes(query) || secText.toLowerCase().includes(query)) {
      const matchingLines = sec.lines.filter((l) => l.toLowerCase().includes(query));
      matchingLineCount += matchingLines.length;
      matchedSections.push(secText);
    }
  }

  if (matchedSections.length === 0) {
    dom.manualContent.textContent = `No manual sections matched "${dom.manualSearchInput.value.trim()}".`;
    dom.manualMatchStatus.textContent = '0 matching sections.';
  } else {
    dom.manualContent.textContent = matchedSections.join('\n\n────────────────────────────────────────\n\n');
    dom.manualMatchStatus.textContent = `${matchedSections.length} section(s), ${matchingLineCount} matching line(s).`;
  }
}

/* --- Frozen Games (.trk) Manager --- */
function listSavedGames() {
  const saves = [];
  for (let i = 0; i < localStorage.length; i += 1) {
    const k = localStorage.key(i);
    if (k && k.startsWith(SAVE_PREFIX)) {
      const name = k.slice(SAVE_PREFIX.length);
      const data = localStorage.getItem(k) || '';
      saves.push({ name, data });
    }
  }
  saves.sort((a, b) => a.name.localeCompare(b.name));
  return saves;
}

function renderSavesList() {
  const saves = listSavedGames();
  if (saves.length === 0) {
    dom.savesList.innerHTML = '<li class="save-item">No frozen games saved in browser storage yet. Use <code>FREEZE</code> in game or upload a <code>.trk</code> file.</li>';
    return;
  }

  const frag = document.createDocumentFragment();
  for (const item of saves) {
    const li = document.createElement('li');
    li.className = 'save-item';

    const meta = document.createElement('div');
    meta.className = 'save-meta';
    const nameEl = document.createElement('span');
    nameEl.className = 'save-name';
    nameEl.textContent = item.name;
    const sizeEl = document.createElement('span');
    sizeEl.className = 'save-size';
    sizeEl.textContent = `${item.data.length} chars`;
    meta.append(nameEl, sizeEl);

    const actions = document.createElement('div');
    actions.className = 'save-actions';

    const insertBtn = document.createElement('button');
    insertBtn.type = 'button';
    insertBtn.className = 'btn btn-primary';
    insertBtn.textContent = 'Insert';
    insertBtn.setAttribute('aria-label', `Insert frozen ${item.name} into command input`);
    insertBtn.addEventListener('click', () => {
      dom.savesDialog.close();
      if (state.emexit || state.promptType === 'exited') {
        startNewGame();
      }
      dom.cmdInput.value = `frozen ${item.name}`;
      dom.cmdInput.focus();
      const end = dom.cmdInput.value.length;
      dom.cmdInput.setSelectionRange(end, end);
    });

    const downloadBtn = document.createElement('button');
    downloadBtn.type = 'button';
    downloadBtn.className = 'btn';
    downloadBtn.textContent = 'Download';
    downloadBtn.setAttribute('aria-label', `Download ${item.name}`);
    downloadBtn.addEventListener('click', () => {
      triggerFileDownload(item.name, item.data, 'text/plain;charset=utf-8');
    });

    const deleteBtn = document.createElement('button');
    deleteBtn.type = 'button';
    deleteBtn.className = 'btn btn-danger';
    deleteBtn.textContent = 'Delete';
    deleteBtn.setAttribute('aria-label', `Delete ${item.name}`);
    deleteBtn.addEventListener('click', () => {
      localStorage.removeItem(SAVE_PREFIX + item.name);
      dom.savesStatus.textContent = `Deleted ${item.name} from local storage.`;
      renderSavesList();
    });

    actions.append(insertBtn, downloadBtn, deleteBtn);
    li.append(meta, actions);
    frag.appendChild(li);
  }
  dom.savesList.replaceChildren(frag);
}

function triggerFileDownload(filename, textContent, mimeType) {
  const blob = new Blob([textContent], { type: mimeType });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

/* --- Theme & Preferences --- */
function applyTheme(themeName) {
  const valid = ['phosphor-green', 'amber-crt', 'starfleet-modern'];
  const chosen = valid.includes(themeName) ? themeName : 'phosphor-green';
  document.documentElement.dataset.theme = chosen;
  dom.themeSelect.value = chosen;
  try {
    localStorage.setItem(THEME_KEY, chosen);
  } catch (_err) {
    // Ignore storage errors.
  }
}

function loadSavedPreferences() {
  try {
    const savedTheme = localStorage.getItem(THEME_KEY);
    if (savedTheme) applyTheme(savedTheme);

    const savedCoordFixed = localStorage.getItem(COORD_FIXED_KEY);
    if (savedCoordFixed === '1') {
      dom.coordFixedToggle.checked = true;
      state.coordFixed = true;
    }

    const savedPlaque = localStorage.getItem(PLAQUE_KEY);
    if (savedPlaque) {
      const parsed = JSON.parse(savedPlaque);
      if (parsed && typeof parsed.content === 'string') {
        state.latestPlaque = parsed;
      }
    }
  } catch (_err) {
    // Ignore storage errors.
  }
  updatePlaqueUI();
}

/* --- Event Listeners --- */
function bindEvents() {
  dom.themeSelect.addEventListener('change', (e) => {
    applyTheme(e.target.value);
  });

  dom.coordFixedToggle.addEventListener('change', () => {
    const checked = dom.coordFixedToggle.checked;
    try {
      localStorage.setItem(COORD_FIXED_KEY, checked ? '1' : '0');
    } catch (_err) {
      // Ignore storage errors.
    }
    if (!state.ready || !state.wasm) return;
    state.wasm.trek_set_coord_fixed(checked ? 1 : 0);
    const res = readWasmResponse();
    applyWasmResponse(res);
  });

  dom.newGameBtn.addEventListener('click', () => {
    startNewGame();
  });

  dom.exitedNewGameBtn.addEventListener('click', () => {
    startNewGame();
  });

  dom.bossResumeBtn.addEventListener('click', () => {
    startNewGame();
  });

  dom.continueBtn.addEventListener('click', () => {
    sendKeyCommand(' ');
  });

  dom.cmdInput.addEventListener('compositionstart', () => {
    state.isComposing = true;
  });
  dom.cmdInput.addEventListener('compositionend', () => {
    state.isComposing = false;
  });

  dom.cmdInput.addEventListener('keydown', (event) => {
    // MANDATORY IME composition guard
    if (event.isComposing || event.keyCode === 229) return;

    if (event.key === 'ArrowUp') {
      if (state.history.length === 0) return;
      event.preventDefault();
      if (state.historyIndex === -1) {
        state.historyDraft = dom.cmdInput.value;
        state.historyIndex = state.history.length - 1;
      } else if (state.historyIndex > 0) {
        state.historyIndex -= 1;
      }
      dom.cmdInput.value = state.history[state.historyIndex];
      const end = dom.cmdInput.value.length;
      dom.cmdInput.setSelectionRange(end, end);
    } else if (event.key === 'ArrowDown') {
      if (state.historyIndex === -1) return;
      event.preventDefault();
      if (state.historyIndex < state.history.length - 1) {
        state.historyIndex += 1;
        dom.cmdInput.value = state.history[state.historyIndex];
      } else {
        state.historyIndex = -1;
        dom.cmdInput.value = state.historyDraft;
      }
      const end = dom.cmdInput.value.length;
      dom.cmdInput.setSelectionRange(end, end);
    }
  });

  dom.cmdForm.addEventListener('submit', (event) => {
    event.preventDefault();
    if (state.isComposing) return;
    const value = dom.cmdInput.value;
    dom.cmdInput.value = '';
    sendLineCommand(value);
  });

  dom.quickCmdsGrid.addEventListener('click', (event) => {
    const btn = event.target.closest('button[data-cmd]');
    if (!btn || btn.disabled) return;
    sendLineCommand(btn.dataset.cmd);
  });

  document.addEventListener('keydown', (event) => {
    if (event.isComposing || event.keyCode === 229) return;
    if (state.promptType !== 'key' || state.emexit || anyDialogOpen()) return;
    if (event.ctrlKey || event.metaKey || event.altKey) return;

    if (event.key === ' ' || event.key === 'Enter') {
      event.preventDefault();
      sendKeyCommand(' ');
    } else if (event.key.length === 1) {
      event.preventDefault();
      sendKeyCommand(event.key);
    }
  });

  /* Dialogs */
  dom.openManualBtn.addEventListener('click', async () => {
    dom.manualDialog.showModal();
    await ensureManualLoaded();
  });

  dom.manualFilterForm.addEventListener('submit', (e) => e.preventDefault());
  dom.manualSectionSelect.addEventListener('change', renderManualView);
  dom.manualSearchInput.addEventListener('input', renderManualView);

  dom.openSavesBtn.addEventListener('click', () => {
    dom.savesStatus.textContent = '';
    renderSavesList();
    dom.savesDialog.showModal();
  });

  dom.uploadTrkInput.addEventListener('change', async (event) => {
    const file = event.target.files && event.target.files[0];
    if (!file) return;
    try {
      const text = await file.text();
      const saveName = file.name.trim() || 'uploaded.trk';
      localStorage.setItem(SAVE_PREFIX + saveName, text);
      if (state.wasm) {
        withTwoWasmStrings(saveName, text, (nPtr, nLen, dPtr, dLen) => {
          state.wasm.trek_import_save(nPtr, nLen, dPtr, dLen);
        });
      }
      dom.savesStatus.textContent = `Imported ${saveName} (${text.length} chars). Use "Insert" to load via frozen command.`;
      renderSavesList();
    } catch (err) {
      dom.savesStatus.textContent = `Failed to import save: ${err.message}`;
    } finally {
      dom.uploadTrkInput.value = '';
    }
  });

  dom.openPlaqueBtn.addEventListener('click', () => {
    updatePlaqueUI();
    dom.plaqueDialog.showModal();
  });

  dom.plaqueDownloadBtn.addEventListener('click', () => {
    if (!state.latestPlaque) return;
    triggerFileDownload(
      state.latestPlaque.name || 'plaque.txt',
      state.latestPlaque.content,
      'text/plain;charset=utf-8',
    );
  });

  dom.plaquePrintBtn.addEventListener('click', () => {
    if (!state.latestPlaque) return;
    window.print();
  });

  for (const dialog of document.querySelectorAll('dialog')) {
    dialog.addEventListener('close', () => {
      dialog.returnValue = '';
    });
  }
}

async function boot() {
  loadSavedPreferences();
  installDialogLightDismissFallback();
  bindEvents();

  try {
    setWasmStatus('loading', 'Loading ./trek.wasm…');
    state.wasm = await instantiateTrekWasm();
    state.ready = true;
    setWasmStatus('ready', 'WASM Core Online');
    startNewGame();
  } catch (err) {
    state.ready = false;
    setWasmStatus('error', 'WASM Load Failed');
    setTerminalText(
      `Unable to load ./trek.wasm: ${err.message}\n` +
      'Run ./scripts/build_web.sh to compile the WebAssembly core and start the server with cargo run --bin trek.',
    );
    updatePromptControls('exited', false);
  }
}

boot();
