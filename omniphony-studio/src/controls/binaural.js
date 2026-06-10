// Binaural (headphone) output panel: output-mode toggle, HRIR source selection,
// and SensorsOSC head-tracking controls (address, format, smoothing, invert,
// recenter) plus a live head-pose readout.
//
// Self-contained: caches its own DOM elements and applies incoming renderer
// state via `applyBinauralState(payload.binaural)`. All element lookups are
// guarded so a missing node never throws.

import { invoke } from '@tauri-apps/api/core';

const el = (id) => document.getElementById(id);

// Guard against re-binding listeners if the panel is initialised twice.
let bound = false;
// While we are pushing renderer state into the controls, ignore the change
// events that programmatic value updates would otherwise fire back as commands.
let applying = false;

function send(cmd, args) {
  invoke(cmd, args).catch((e) => console.error('[binaural]', cmd, e));
}

export function initBinauralPanel() {
  if (bound) return;
  bound = true;

  // Collapsible section: the content carries `conditional-params` (collapsed by
  // default) and is revealed by toggling the `open` class, like the other panels.
  const toggleBtn = el('binauralSectionToggleBtn');
  const content = el('binauralSectionContent');
  if (toggleBtn && content) {
    toggleBtn.addEventListener('click', () => {
      const open = content.classList.toggle('open');
      toggleBtn.textContent = open ? '▾' : '▸';
    });
  }

  const mode = el('binauralOutputMode');
  if (mode) {
    mode.addEventListener('change', (e) => {
      if (applying) return;
      send('control_output_mode', { value: e.target.value });
    });
  }

  const src = el('binauralHrirSource');
  if (src) {
    src.addEventListener('change', (e) => {
      if (applying) return;
      send('control_hrir_source', { value: e.target.value });
    });
  }

  const recenter = el('binauralRecenter');
  if (recenter) {
    recenter.addEventListener('click', () => send('control_head_recenter', {}));
  }

  const addr = el('binauralTrackAddress');
  if (addr) {
    const commit = () => {
      if (applying) return;
      send('control_head_tracking_address', { value: addr.value });
    };
    addr.addEventListener('change', commit);
  }

  const fmt = el('binauralTrackFormat');
  if (fmt) {
    fmt.addEventListener('change', (e) => {
      if (applying) return;
      send('control_head_tracking_format', { value: e.target.value });
    });
  }

  const smooth = el('binauralTrackSmoothing');
  if (smooth) {
    smooth.addEventListener('input', (e) => {
      if (applying) return;
      const v = Number(e.target.value);
      const out = el('binauralTrackSmoothingVal');
      if (out) out.textContent = v.toFixed(2);
      send('control_head_tracking_smoothing', { value: v });
    });
  }

  const invert = el('binauralTrackInvert');
  if (invert) {
    invert.addEventListener('change', (e) => {
      if (applying) return;
      send('control_head_tracking_invert', { enable: e.target.checked ? 1 : 0 });
    });
  }
}

// Apply the `binaural` object from the renderer state bundle to the controls.
export function applyBinauralState(b) {
  if (!b || typeof b !== 'object') return;
  applying = true;
  try {
    const setVal = (id, v) => { const n = el(id); if (n && v != null) n.value = String(v); };

    if (typeof b.outputMode === 'string') setVal('binauralOutputMode', b.outputMode);
    if (typeof b.hrirSource === 'string') setVal('binauralHrirSource', b.hrirSource);

    const t = b.tracking || {};
    if (typeof t.address === 'string') setVal('binauralTrackAddress', t.address);
    if (typeof t.format === 'string') setVal('binauralTrackFormat', t.format);
    if (typeof t.smoothing === 'number') {
      setVal('binauralTrackSmoothing', t.smoothing);
      const out = el('binauralTrackSmoothingVal');
      if (out) out.textContent = Number(t.smoothing).toFixed(2);
    }
    const inv = el('binauralTrackInvert');
    if (inv && typeof t.invert === 'boolean') inv.checked = t.invert;

    // Head-pose readout as yaw/pitch/roll degrees, derived from the quaternion.
    const pose = b.headPose;
    const out = el('binauralPoseReadout');
    if (out && pose && typeof pose.w === 'number') {
      const { w, x, y, z } = pose;
      const yaw = Math.atan2(2 * (w * z + x * y), 1 - 2 * (y * y + z * z));
      const pitch = Math.asin(Math.max(-1, Math.min(1, 2 * (w * x - z * y))));
      const roll = Math.atan2(2 * (w * y + z * x), 1 - 2 * (x * x + y * y));
      const deg = (r) => (r * 180 / Math.PI).toFixed(0);
      out.textContent = `yaw ${deg(yaw)}°  pitch ${deg(pitch)}°  roll ${deg(roll)}°`;
    }
  } finally {
    applying = false;
  }
}
