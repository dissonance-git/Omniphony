// Components plot: traces the three constituents of `control_available` —
// ring-buffer occupancy, output FIFO (input-domain), and resampler-pending —
// over time so the source of latency oscillations can be localised. Polled
// alongside the existing resample plot.

import { app } from '../state.js';
import { getPlotWindowMs } from './diag-plot.js';

const POLL_INTERVAL_MS = 20;
// Maximum retained history; the rendered window is whichever value the user
// picks in the diag-plot selector — see `getPlotWindowMs`. Keeping the
// largest option preserves history when the user zooms out.
const MAX_RETAIN_MS = 60000;

const TRACES = [
  { key: 'latencyAvailInputMs', color: '#7ad7ff', label: 'avail_input' },
  { key: 'latencyOutputFifoMs', color: '#9cffa3', label: 'output_fifo' },
  { key: 'latencyResamplerPendingMs', color: '#ffb86b', label: 'resampler_pending' },
];

let canvasEl = null;
let containerEl = null;
let toggleBtnEl = null;
let intervalId = null;
let visible = false;
const buffer = [];

function getElements() {
  if (!canvasEl) canvasEl = document.getElementById('componentsPlotCanvas');
  if (!containerEl) containerEl = document.getElementById('componentsPlotContainer');
  if (!toggleBtnEl) toggleBtnEl = document.getElementById('componentsPlotToggleBtn');
  return { canvasEl, containerEl, toggleBtnEl };
}

function sampleApp() {
  return {
    t: Date.now(),
    latencyAvailInputMs: typeof app.latencyAvailInputMs === 'number' ? app.latencyAvailInputMs : null,
    latencyOutputFifoMs: typeof app.latencyOutputFifoMs === 'number' ? app.latencyOutputFifoMs : null,
    latencyResamplerPendingMs:
      typeof app.latencyResamplerPendingMs === 'number' ? app.latencyResamplerPendingMs : null,
  };
}

function drawTimeGrid(ctx, x0, y0, panelW, panelH, tMin, tMax) {
  // 250 ms light + 1 s bold vertical reference lines. Drawn first so
  // traces sit on top.
  const xFor = (t) => x0 + ((t - tMin) / (tMax - tMin)) * panelW;
  ctx.lineWidth = 1;
  ctx.strokeStyle = 'rgba(255, 255, 255, 0.04)';
  ctx.beginPath();
  let t = Math.ceil(tMin / 250) * 250;
  while (t <= tMax) {
    if (t % 1000 !== 0) {
      const x = Math.round(xFor(t)) + 0.5;
      ctx.moveTo(x, y0);
      ctx.lineTo(x, y0 + panelH);
    }
    t += 250;
  }
  ctx.stroke();
  ctx.strokeStyle = 'rgba(255, 255, 255, 0.15)';
  ctx.beginPath();
  t = Math.ceil(tMin / 1000) * 1000;
  while (t <= tMax) {
    const x = Math.round(xFor(t)) + 0.5;
    ctx.moveTo(x, y0);
    ctx.lineTo(x, y0 + panelH);
    t += 1000;
  }
  ctx.stroke();
}

function renderPanel(ctx, trace, x0, y0, panelW, panelH, tMin, tMax) {
  drawTimeGrid(ctx, x0, y0, panelW, panelH, tMin, tMax);
  ctx.strokeStyle = 'rgba(255, 255, 255, 0.07)';
  ctx.lineWidth = 1;
  ctx.beginPath();
  ctx.moveTo(x0, y0 + panelH + 0.5);
  ctx.lineTo(x0 + panelW, y0 + panelH + 0.5);
  ctx.stroke();

  let vMin = Infinity;
  let vMax = -Infinity;
  for (const sample of buffer) {
    if (sample.t < tMin) continue; // off-screen to the left
    const v = sample[trace.key];
    if (typeof v === 'number' && Number.isFinite(v)) {
      if (v < vMin) vMin = v;
      if (v > vMax) vMax = v;
    }
  }
  if (vMin === Infinity) {
    ctx.fillStyle = 'rgba(217, 236, 255, 0.55)';
    ctx.font = '10px sans-serif';
    ctx.textBaseline = 'top';
    ctx.fillText(`${trace.label}: no data`, x0 + 4, y0 + 2);
    return;
  }
  const vMinRaw = vMin;
  const vMaxRaw = vMax;
  if (vMax - vMin < 1e-3) {
    vMax = vMin + 1;
  }
  const pad = (vMax - vMin) * 0.1;
  vMin -= pad;
  vMax += pad;

  const xFor = (t) => x0 + ((t - tMin) / (tMax - tMin)) * panelW;
  const yFor = (v) => y0 + ((vMax - v) / (vMax - vMin)) * (panelH - 4) + 2;

  ctx.strokeStyle = trace.color;
  ctx.lineWidth = 1.5;
  ctx.beginPath();
  let started = false;
  for (const sample of buffer) {
    const v = sample[trace.key];
    if (typeof v !== 'number' || !Number.isFinite(v)) continue;
    const xx = xFor(sample.t);
    const yy = yFor(v);
    if (!started) {
      ctx.moveTo(xx, yy);
      started = true;
    } else {
      ctx.lineTo(xx, yy);
    }
  }
  if (started) ctx.stroke();

  ctx.fillStyle = 'rgba(217, 236, 255, 0.85)';
  ctx.font = '10px sans-serif';
  ctx.textBaseline = 'top';
  ctx.fillText(
    `${trace.label} ${vMinRaw.toFixed(2)}…${vMaxRaw.toFixed(2)} ms (Δ ${(vMaxRaw - vMinRaw).toFixed(2)})`,
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

  if (buffer.length < 2) {
    ctx.fillStyle = 'rgba(217, 236, 255, 0.85)';
    ctx.font = '10px sans-serif';
    ctx.fillText('Waiting for telemetry…', 6, h / 2);
    return;
  }

  const tMax = buffer[buffer.length - 1].t;
  const tMin = tMax - getPlotWindowMs();
  const panelH = Math.floor(h / TRACES.length);
  for (let i = 0; i < TRACES.length; i += 1) {
    const y0 = i * panelH;
    const ph = i === TRACES.length - 1 ? h - y0 : panelH;
    renderPanel(ctx, TRACES[i], 0, y0, w, ph, tMin, tMax);
  }
}

function tick() {
  const sample = sampleApp();
  buffer.push(sample);
  const cutoff = sample.t - MAX_RETAIN_MS;
  while (buffer.length && buffer[0].t < cutoff) buffer.shift();
  render();
}

export function setComponentsPlotVisible(open) {
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

export function isComponentsPlotVisible() {
  return visible;
}

export function toggleComponentsPlot() {
  setComponentsPlotVisible(!visible);
}
