// Generic diagnostic-metrics plot. Reads the schema published by the renderer
// (app.diagSchema = { items: [{name, label, group, unit}, ...] }) and lets the
// user pick which metrics to trace via a multi-select chip row. Each selected
// metric is rendered in its own stacked panel with an independent y-scale.
//
// The renderer's DiagRegistry is the source of truth: adding a new metric on
// the Rust side instantly makes it available here without any change.
// User selections persist across reloads via localStorage.

import { app } from '../state.js';

const POLL_INTERVAL_MS = 100;
const WINDOW_MS = 60000;
const STORAGE_KEY = 'diagPlot.selectedMetrics.v1';
const PALETTE = [
  '#7ad7ff', '#9cffa3', '#ffb86b', '#d6a0ff',
  '#ff8da6', '#ffd166', '#a0e5ff', '#b6ff8c',
  '#ffa07a', '#c099ff', '#ff9bba', '#ffe88a',
];

let canvasEl = null;
let containerEl = null;
let controlsEl = null;
let toggleBtnEl = null;
let intervalId = null;
let visible = false;
const buffer = [];
let selected = loadSelection();
let renderedSchemaSignature = null;

function loadSelection() {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return new Set();
    const arr = JSON.parse(raw);
    return new Set(Array.isArray(arr) ? arr : []);
  } catch (_) {
    return new Set();
  }
}

function saveSelection() {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify([...selected]));
  } catch (_) { /* ignore */ }
}

function getElements() {
  if (!canvasEl) canvasEl = document.getElementById('diagPlotCanvas');
  if (!containerEl) containerEl = document.getElementById('diagPlotContainer');
  if (!controlsEl) controlsEl = document.getElementById('diagPlotControls');
  if (!toggleBtnEl) toggleBtnEl = document.getElementById('diagPlotToggleBtn');
  return { canvasEl, containerEl, controlsEl, toggleBtnEl };
}

function schemaItems() {
  const schema = app.diagSchema;
  if (!schema || !Array.isArray(schema.items)) return [];
  return schema.items;
}

function colorFor(name) {
  let hash = 0;
  for (let i = 0; i < name.length; i += 1) hash = (hash * 31 + name.charCodeAt(i)) | 0;
  return PALETTE[Math.abs(hash) % PALETTE.length];
}

function rebuildControlsIfSchemaChanged() {
  const { controlsEl: ctrl } = getElements();
  if (!ctrl) return;
  const items = schemaItems();
  const signature = items.map((it) => it.name).join('|');
  if (signature === renderedSchemaSignature) return;
  renderedSchemaSignature = signature;

  ctrl.innerHTML = '';
  if (items.length === 0) {
    ctrl.textContent = 'No diag metrics registered yet.';
    return;
  }
  // Group by `group` field, preserve registration order within group.
  const groups = new Map();
  for (const item of items) {
    const g = item.group || 'misc';
    if (!groups.has(g)) groups.set(g, []);
    groups.get(g).push(item);
  }
  for (const [groupName, members] of groups) {
    const groupEl = document.createElement('div');
    groupEl.style.display = 'flex';
    groupEl.style.flexWrap = 'wrap';
    groupEl.style.alignItems = 'center';
    groupEl.style.gap = '0.25rem';
    groupEl.style.padding = '0.15rem 0.35rem';
    groupEl.style.border = '1px solid rgba(255,255,255,0.08)';
    groupEl.style.borderRadius = '6px';
    const lbl = document.createElement('span');
    lbl.textContent = groupName;
    lbl.style.opacity = '0.7';
    lbl.style.marginRight = '0.15rem';
    groupEl.appendChild(lbl);
    for (const item of members) {
      const chip = document.createElement('button');
      chip.type = 'button';
      chip.textContent = item.label || item.name;
      chip.title = `${item.name}${item.unit ? ' (' + item.unit + ')' : ''}`;
      chip.dataset.name = item.name;
      chip.style.cssText = chipStyle(selected.has(item.name), colorFor(item.name));
      chip.addEventListener('click', () => {
        if (selected.has(item.name)) selected.delete(item.name);
        else selected.add(item.name);
        chip.style.cssText = chipStyle(selected.has(item.name), colorFor(item.name));
        saveSelection();
      });
      groupEl.appendChild(chip);
    }
    ctrl.appendChild(groupEl);
  }
}

function chipStyle(on, color) {
  const baseBg = on ? `${color}33` : 'rgba(255,255,255,0.06)';
  const borderColor = on ? color : 'rgba(255,255,255,0.15)';
  const txt = on ? color : '#d9ecff';
  return `font-size:10px;padding:0.15rem 0.45rem;border-radius:999px;border:1px solid ${borderColor};background:${baseBg};color:${txt};cursor:pointer;`;
}

function sampleValues() {
  const values = app.diagValues || {};
  const sample = { t: Date.now() };
  for (const name of selected) {
    const v = values[name];
    sample[name] = typeof v === 'number' && Number.isFinite(v) ? v : null;
  }
  return sample;
}

function renderPanel(ctx, item, x0, y0, panelW, panelH, tMin, tMax) {
  ctx.strokeStyle = 'rgba(255, 255, 255, 0.07)';
  ctx.lineWidth = 1;
  ctx.beginPath();
  ctx.moveTo(x0, y0 + panelH + 0.5);
  ctx.lineTo(x0 + panelW, y0 + panelH + 0.5);
  ctx.stroke();

  let vMin = Infinity;
  let vMax = -Infinity;
  for (const sample of buffer) {
    const v = sample[item.name];
    if (typeof v === 'number' && Number.isFinite(v)) {
      if (v < vMin) vMin = v;
      if (v > vMax) vMax = v;
    }
  }
  const color = colorFor(item.name);
  const label = item.label || item.name;
  const unit = item.unit ? ` ${item.unit}` : '';
  if (vMin === Infinity) {
    ctx.fillStyle = 'rgba(217, 236, 255, 0.55)';
    ctx.font = '10px sans-serif';
    ctx.textBaseline = 'top';
    ctx.fillText(`${label}: no data`, x0 + 4, y0 + 2);
    return;
  }
  const vMinRaw = vMin;
  const vMaxRaw = vMax;
  if (vMax - vMin < 1e-9) vMax = vMin + 1;
  const pad = (vMax - vMin) * 0.1;
  vMin -= pad;
  vMax += pad;
  const xFor = (t) => x0 + ((t - tMin) / (tMax - tMin)) * panelW;
  const yFor = (v) => y0 + ((vMax - v) / (vMax - vMin)) * (panelH - 4) + 2;

  ctx.strokeStyle = color;
  ctx.lineWidth = 1.5;
  ctx.beginPath();
  let started = false;
  for (const sample of buffer) {
    const v = sample[item.name];
    if (typeof v !== 'number' || !Number.isFinite(v)) continue;
    const xx = xFor(sample.t);
    const yy = yFor(v);
    if (!started) { ctx.moveTo(xx, yy); started = true; }
    else { ctx.lineTo(xx, yy); }
  }
  if (started) ctx.stroke();

  ctx.fillStyle = 'rgba(217, 236, 255, 0.85)';
  ctx.font = '10px sans-serif';
  ctx.textBaseline = 'top';
  ctx.fillText(
    `${label} ${vMinRaw.toFixed(2)}…${vMaxRaw.toFixed(2)}${unit} (Δ ${(vMaxRaw - vMinRaw).toFixed(2)})`,
    x0 + 4,
    y0 + 2
  );
}

function render() {
  const { canvasEl: canvas } = getElements();
  if (!canvas) return;
  const ctx = canvas.getContext('2d');
  const w = canvas.width;
  const h = canvas.height;
  ctx.fillStyle = 'rgba(15, 23, 36, 0.9)';
  ctx.fillRect(0, 0, w, h);

  const items = schemaItems().filter((it) => selected.has(it.name));
  if (items.length === 0) {
    ctx.fillStyle = 'rgba(217, 236, 255, 0.85)';
    ctx.font = '10px sans-serif';
    ctx.fillText('Select one or more metrics above.', 6, h / 2);
    return;
  }
  if (buffer.length < 2) {
    ctx.fillStyle = 'rgba(217, 236, 255, 0.85)';
    ctx.font = '10px sans-serif';
    ctx.fillText('Waiting for diag telemetry…', 6, h / 2);
    return;
  }
  const tMax = buffer[buffer.length - 1].t;
  const tMin = tMax - WINDOW_MS;
  const panelH = Math.floor(h / items.length);
  for (let i = 0; i < items.length; i += 1) {
    const y0 = i * panelH;
    const ph = i === items.length - 1 ? h - y0 : panelH;
    renderPanel(ctx, items[i], 0, y0, w, ph, tMin, tMax);
  }
}

function resizeCanvasForSelection() {
  const { canvasEl: canvas } = getElements();
  if (!canvas) return;
  const count = Math.max(1, [...selected].length);
  const target = Math.min(640, Math.max(120, count * 80));
  if (canvas.height !== target) {
    canvas.height = target;
  }
}

function tick() {
  rebuildControlsIfSchemaChanged();
  resizeCanvasForSelection();
  const sample = sampleValues();
  buffer.push(sample);
  const cutoff = sample.t - WINDOW_MS;
  while (buffer.length && buffer[0].t < cutoff) buffer.shift();
  render();
}

export function setDiagPlotVisible(open) {
  const { containerEl: c, toggleBtnEl: btn } = getElements();
  if (!c) return;
  visible = Boolean(open);
  c.style.display = visible ? 'block' : 'none';
  if (btn) {
    btn.classList.toggle('is-active', visible);
    btn.setAttribute('aria-pressed', visible ? 'true' : 'false');
  }
  if (visible) {
    if (!intervalId) intervalId = setInterval(tick, POLL_INTERVAL_MS);
    tick();
  } else {
    if (intervalId) {
      clearInterval(intervalId);
      intervalId = null;
    }
    buffer.length = 0;
    render();
  }
}

export function isDiagPlotVisible() {
  return visible;
}

export function toggleDiagPlot() {
  setDiagPlotVisible(!visible);
}
