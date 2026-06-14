/**
 * Virtual-bed editor: per-channel placement for 2D (channel-based) sources.
 *
 * Each input channel is either routed direct to its speaker (spatialize:false,
 * e.g. LFE → sub) or virtualized as an object at a position (spatialize:true).
 * The editor edits a `SpeakerLayout` (one entry per channel label) and pushes it
 * live to the renderer via `control_virtual_bed` (a YAML/JSON layout string; an
 * empty string resets to the built-in canonical poses). The 3D scene visualises
 * the result through the live object stream.
 */

import { invoke } from '@tauri-apps/api/core';
import { app } from '../state.js';
import { t } from '../i18n.js';

// Canonical editable channel set (7.1) with the renderer's built-in poses.
// LFE defaults to direct (it cannot be VBAP-panned). Polar degrees / metres.
const CANONICAL_BED = [
  { name: 'L', azimuth: -26, elevation: 0, distance: 2.0, spatialize: true },
  { name: 'R', azimuth: 26, elevation: 0, distance: 2.0, spatialize: true },
  { name: 'C', azimuth: 0, elevation: 0, distance: 2.0, spatialize: true },
  { name: 'LFE', azimuth: 0, elevation: 0, distance: 1.0, spatialize: false },
  { name: 'Ls', azimuth: -100, elevation: 0, distance: 1.0, spatialize: true },
  { name: 'Rs', azimuth: 100, elevation: 0, distance: 1.0, spatialize: true },
  { name: 'Lb', azimuth: -142.5, elevation: 0, distance: 1.0, spatialize: true },
  { name: 'Rb', azimuth: 142.5, elevation: 0, distance: 1.0, spatialize: true }
];

// Name aliases per canonical channel (mirrors the renderer's label_aliases) so
// the editor matches a bed/object entry however it is named (L/FL, Ls/SL, …).
const CHANNEL_ALIASES = {
  L: ['l', 'fl', 'frontleft', 'leftfront'],
  R: ['r', 'fr', 'frontright', 'rightfront'],
  C: ['c', 'fc', 'center', 'centre'],
  LFE: ['lfe', 'lfe1', 'sub', 'subwoofer', 'sw'],
  Ls: ['ls', 'sl', 'leftsurround', 'surroundleft'],
  Rs: ['rs', 'sr', 'rightsurround', 'surroundright'],
  Lb: ['lb', 'bl', 'lrs', 'backleft', 'leftback', 'rearleft', 'leftrear'],
  Rb: ['rb', 'br', 'rrs', 'backright', 'rightback', 'rearright', 'rightrear']
};

// Canonical channel key (L/R/C/LFE/Ls/Rs/Lb/Rb) for any alias, or null.
export function canonicalChannelName(name) {
  if (typeof name !== 'string') return null;
  const lower = name.trim().toLowerCase();
  for (const [key, aliases] of Object.entries(CHANNEL_ALIASES)) {
    if (aliases.includes(lower)) return key;
  }
  return null;
}

function getEditorEl() { return document.getElementById('virtualBedEditor'); }

// Match the renderer's cartesian→spherical convention (x=right, y=front, z=up).
function cartesianToPolar(x, y, z) {
  const dist = Math.sqrt(x * x + y * y + z * z);
  const azimuth = (Math.atan2(x, y) * 180) / Math.PI;
  const elevation = dist > 0 ? (Math.atan2(z, Math.sqrt(x * x + y * y)) * 180) / Math.PI : 0;
  return { azimuth, elevation, distance: Math.max(dist, 0.01) };
}

// Read a snapshot bed entry (polar or cartesian) as polar degrees/metres.
function entryToPolar(entry) {
  if (Number.isFinite(Number(entry?.azimuth))) {
    return {
      azimuth: Number(entry.azimuth),
      elevation: Number(entry.elevation) || 0,
      distance: Number(entry.distance) || 1.0
    };
  }
  if (Number.isFinite(Number(entry?.x))) {
    return cartesianToPolar(Number(entry.x) || 0, Number(entry.y) || 0, Number(entry.z) || 0);
  }
  return null;
}

// The full editable channel set: canonical defaults overridden by any matching
// entry from the live virtual bed (by name, case-insensitive).
function effectiveChannels() {
  const configured = Array.isArray(app.virtualBed?.speakers) ? app.virtualBed.speakers : [];
  return CANONICAL_BED.map((base) => {
    const match = configured.find((s) => canonicalChannelName(s?.name) === base.name);
    if (!match) return { ...base };
    const polar = entryToPolar(match) || base;
    return {
      name: base.name,
      azimuth: polar.azimuth,
      elevation: polar.elevation,
      distance: polar.distance,
      spatialize: match.spatialize !== false
    };
  });
}

function buildLayoutPayload(channels) {
  const radius = Number(app.virtualBed?.radius_m) > 0 ? Number(app.virtualBed.radius_m) : 1.0;
  return {
    radius_m: radius,
    speakers: channels.map((c) => ({
      name: c.name,
      coord_mode: 'polar',
      azimuth: Number(c.azimuth) || 0,
      elevation: Number(c.elevation) || 0,
      distance: Number(c.distance) > 0 ? Number(c.distance) : 0.01,
      spatialize: Boolean(c.spatialize)
    }))
  };
}

// Read every row back from the DOM and push the whole bed to the renderer.
function collectAndSend() {
  const editor = getEditorEl();
  if (!editor) return;
  const channels = Array.from(editor.querySelectorAll('[data-vbed-channel]')).map((row) => ({
    name: row.getAttribute('data-vbed-channel'),
    spatialize: row.querySelector('[data-vbed-field="spatialize"]').checked,
    azimuth: Number(row.querySelector('[data-vbed-field="azimuth"]').value),
    elevation: Number(row.querySelector('[data-vbed-field="elevation"]').value),
    distance: Number(row.querySelector('[data-vbed-field="distance"]').value)
  }));
  app.virtualBed = buildLayoutPayload(channels);
  invoke('control_virtual_bed', { value: JSON.stringify(app.virtualBed) });
}

// Placement of a channel by name (case-insensitive): 'virtual' (draggable
// object), 'direct' (anchored to its speaker), or null (not a bed channel).
export function channelPlacement(name) {
  const key = canonicalChannelName(name);
  if (!key) return null;
  const ch = effectiveChannels().find((c) => c.name === key);
  if (!ch) return null;
  return ch.spatialize ? 'virtual' : 'direct';
}

// Apply a new direction (azimuth/elevation in degrees) to one channel from a 3D
// drag, keeping its distance, and push the whole bed to the renderer.
export function applyChannelDirection(name, azimuth, elevation) {
  const key = canonicalChannelName(name);
  if (!key) return;
  const channels = effectiveChannels();
  const target = channels.find((c) => c.name === key);
  if (!target) return;
  target.azimuth = azimuth;
  target.elevation = elevation;
  target.spatialize = true;
  app.virtualBed = buildLayoutPayload(channels);
  invoke('control_virtual_bed', { value: JSON.stringify(app.virtualBed) });
  renderVirtualBedEditor(true);
}

export function resetVirtualBed() {
  app.virtualBed = null;
  invoke('control_virtual_bed', { value: '' });
  renderVirtualBedEditor(true);
}

function numberInput(field, value, disabled) {
  const input = document.createElement('input');
  input.type = 'number';
  input.className = 'delay-input';
  input.setAttribute('data-vbed-field', field);
  input.style.width = '4.5rem';
  input.step = field === 'distance' ? '0.05' : '1';
  input.value = String(Math.round(Number(value) * 100) / 100);
  input.disabled = disabled;
  input.addEventListener('change', collectAndSend);
  return input;
}

/**
 * (Re)build the per-channel editor rows from the current bed. Skips the rebuild
 * while the user is editing a field (focus inside the editor) so typing is not
 * clobbered by an incoming snapshot — unless `force` is set.
 */
let lastSignature = null;

export function renderVirtualBedEditor(force = false) {
  const editor = getEditorEl();
  if (!editor) return;
  const spatial = app.channelRenderMode !== 'host';
  editor.style.display = spatial ? '' : 'none';
  if (!spatial) {
    lastSignature = null;
    return;
  }
  if (!force && editor.contains(document.activeElement)) return;

  // renderAudioFormatDisplay fires on every flush (incl. meter updates); skip
  // the DOM rebuild unless the bed actually changed to avoid churn/flicker.
  const channels = effectiveChannels();
  const signature = JSON.stringify(channels);
  if (!force && signature === lastSignature && editor.childElementCount > 0) return;
  lastSignature = signature;

  editor.replaceChildren();
  for (const ch of channels) {
    const row = document.createElement('div');
    row.setAttribute('data-vbed-channel', ch.name);
    row.style.cssText =
      'display:grid;grid-template-columns:2.5rem auto 1fr;align-items:center;gap:0.4rem;padding:0.12rem 0';

    const name = document.createElement('span');
    name.textContent = ch.name;
    name.style.cssText = 'font-size:12px;font-weight:600;color:#fff';
    row.appendChild(name);

    const toggleWrap = document.createElement('label');
    toggleWrap.className = 'switch-row';
    toggleWrap.style.cssText = 'gap:0.35rem;cursor:pointer;font-size:11px;color:#cfd8e3';
    const toggleText = document.createElement('span');
    toggleText.textContent = ch.spatialize ? t('virtualBed.virtual') : t('virtualBed.direct');
    const toggle = document.createElement('input');
    toggle.type = 'checkbox';
    toggle.setAttribute('data-vbed-field', 'spatialize');
    toggle.checked = ch.spatialize;
    toggle.addEventListener('change', () => {
      toggleText.textContent = toggle.checked ? t('virtualBed.virtual') : t('virtualBed.direct');
      for (const inp of row.querySelectorAll('input[type="number"]')) inp.disabled = !toggle.checked;
      collectAndSend();
    });
    toggleWrap.append(toggleText, toggle);
    row.appendChild(toggleWrap);

    const pos = document.createElement('div');
    pos.style.cssText = 'display:flex;gap:0.25rem;justify-content:flex-end';
    pos.append(
      numberInput('azimuth', ch.azimuth, !ch.spatialize),
      numberInput('elevation', ch.elevation, !ch.spatialize),
      numberInput('distance', ch.distance, !ch.spatialize)
    );
    row.appendChild(pos);
    editor.appendChild(row);
  }
}
