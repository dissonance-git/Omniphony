/**
 * Virtual-bed editor: per-channel placement for 2D (channel-based) sources.
 *
 * Each input channel is either routed direct to its speaker (spatialize:false,
 * e.g. LFE → sub) or virtualized as an object at a position (spatialize:true).
 * The bed is a `SpeakerLayout` (one entry per channel label) pushed live to the
 * renderer via `control_virtual_bed` (a YAML/JSON layout string; an empty string
 * resets to the built-in canonical poses).
 *
 * Editing reuses the speaker-editing mechanic: the channels appear in the Objects
 * list and selecting one opens a parameter panel below it (cartesian / polar,
 * normalized / real, + Direct/Virtual + gain). When no live stream is playing the
 * channels are synthesized as objects (`syncVirtualBedObjects`) so the bed stays
 * visible and editable at rest; live stream objects take over while playing.
 */

import { invoke } from '@tauri-apps/api/core';
import { app, sourceMeshes, sourceNames } from '../state.js';
import { t } from '../i18n.js';
import { updateSource, removeSource } from '../sources.js';
import {
  sphericalToCartesianDeg,
  cartesianToSpherical,
  scenePositionToNormalizedOmniphony,
  normalizedOmniphonyToScenePosition,
  normalizedToMeters,
  formatNumber
} from '../coordinates.js';

// Canonical editable channel set (7.1) with the default poses as ADM cartesian
// corners (X left/right, Y rear/front, Z down/up; ear level Z = 0). LFE defaults
// to direct (it cannot be VBAP-panned).
const CANONICAL_BED = [
  { name: 'L', x: -1, y: 1, z: 0, spatialize: true },
  { name: 'R', x: 1, y: 1, z: 0, spatialize: true },
  { name: 'C', x: 0, y: 1, z: 0, spatialize: true },
  { name: 'LFE', x: 0, y: 1, z: 0, spatialize: false },
  { name: 'Ls', x: -1, y: 0, z: 0, spatialize: true },
  { name: 'Rs', x: 1, y: 0, z: 0, spatialize: true },
  { name: 'Lb', x: -1, y: -1, z: 0, spatialize: true },
  { name: 'Rb', x: 1, y: -1, z: 0, spatialize: true }
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
  Rb: ['rb', 'br', 'rrs', 'backright', 'rightback', 'rightrear', 'rearright']
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

// ---------------------------------------------------------------------------
// Model: the editable channel set
// ---------------------------------------------------------------------------

// Polar (az/el/dist) → ADM normalized cartesian, exactly like the speaker editor:
// the room-warp is inverted and the result clamped to [-1, 1]. "Norm" is this
// ADM position, not a raw axis swizzle.
function polarToAdm(azimuth, elevation, distance) {
  return scenePositionToNormalizedOmniphony(sphericalToCartesianDeg(azimuth, elevation, distance));
}

// ADM normalized cartesian → polar, via the same scene round-trip the speaker
// editor uses (re-applies the room-warp, then derives spherical).
function admToPolar(x, y, z) {
  const sph = cartesianToSpherical(normalizedOmniphonyToScenePosition({ x, y, z }));
  return { azimuth: sph.az, elevation: sph.el, distance: Math.max(0.01, sph.dist) };
}

// Default model entry for a channel: the canonical ADM cartesian corner, with
// the polar form derived so the editor/renderer can use either.
function defaultEntry(base) {
  const polar = admToPolar(base.x, base.y, base.z);
  return {
    name: base.name,
    coordMode: 'cartesian',
    x: base.x,
    y: base.y,
    z: base.z,
    ...polar,
    spatialize: base.spatialize,
    gainDb: 0
  };
}

// Read a configured bed entry (polar or cartesian) as a normalized model entry,
// falling back to the canonical default when it can't be parsed.
function readEntry(base, match) {
  if (!match) return defaultEntry(base);
  const cartesian = String(match.coord_mode || '').toLowerCase() === 'cartesian';
  const gainDb = Number.isFinite(Number(match.gain_db)) ? Math.round(Number(match.gain_db)) : 0;
  const spatialize = match.spatialize !== false;
  if (cartesian && Number.isFinite(Number(match.x))) {
    const x = Number(match.x) || 0;
    const y = Number(match.y) || 0;
    const z = Number(match.z) || 0;
    const polar = admToPolar(x, y, z);
    return { name: base.name, coordMode: 'cartesian', x, y, z, ...polar, spatialize, gainDb };
  }
  if (Number.isFinite(Number(match.azimuth))) {
    const azimuth = Number(match.azimuth);
    const elevation = Number(match.elevation) || 0;
    const distance = Number(match.distance) > 0 ? Number(match.distance) : 1.0;
    const norm = polarToAdm(azimuth, elevation, distance);
    return {
      name: base.name,
      coordMode: 'polar',
      azimuth,
      elevation,
      distance,
      x: norm.x,
      y: norm.y,
      z: norm.z,
      spatialize,
      gainDb
    };
  }
  return defaultEntry(base);
}

// The full editable channel set: canonical defaults overridden by any matching
// entry from the live virtual bed (matched by alias, case-insensitive).
export function effectiveChannels() {
  const configured = Array.isArray(app.virtualBed?.speakers) ? app.virtualBed.speakers : [];
  return CANONICAL_BED.map((base) => {
    const match = configured.find((s) => canonicalChannelName(s?.name) === base.name);
    return readEntry(base, match);
  });
}

function channelByName(name) {
  const key = canonicalChannelName(name);
  if (!key) return null;
  return effectiveChannels().find((c) => c.name === key) || null;
}

function buildLayoutPayload(channels) {
  const radius = Number(app.virtualBed?.radius_m) > 0 ? Number(app.virtualBed.radius_m) : 1.0;
  // Always polar: the renderer builds each bed pose from azimuth/elevation/
  // distance, and the angles/distance here are scene-space (what the gizmo and
  // the cartesian inputs both resolve to), so the channel lands where shown.
  return {
    radius_m: radius,
    speakers: channels.map((c) => {
      const entry = {
        name: c.name,
        coord_mode: 'polar',
        spatialize: Boolean(c.spatialize),
        azimuth: Number(c.azimuth) || 0,
        elevation: Number(c.elevation) || 0,
        distance: Number(c.distance) > 0 ? Number(c.distance) : 0.01
      };
      if (Math.round(c.gainDb || 0) !== 0) entry.gain_db = Math.round(c.gainDb);
      return entry;
    })
  };
}

// Update one channel entry (by canonical name) via `mutate`, push the whole bed
// to the renderer, and refresh the synthetic objects + panel.
function commitChannel(name, mutate) {
  const key = canonicalChannelName(name);
  if (!key) return;
  const channels = effectiveChannels();
  const target = channels.find((c) => c.name === key);
  if (!target) return;
  mutate(target);
  app.virtualBed = buildLayoutPayload(channels);
  invoke('control_virtual_bed', { value: JSON.stringify(app.virtualBed) });
  syncVirtualBedObjects(true);
  renderChannelEditor(true);
}

// Current placement of a channel as polar (az/el/dist) + pure normalized
// cartesian (x/y/z), so editor inputs can pull untouched axes from canonical
// state instead of re-reading rounded DOM values (mirrors the speaker editor).
export function getChannelPosition(name) {
  const ch = channelByName(name);
  if (!ch) return null;
  const norm = polarToAdm(ch.azimuth, ch.elevation, ch.distance);
  const meters = normalizedToMeters(norm);
  return {
    azimuth: ch.azimuth,
    elevation: ch.elevation,
    distance: ch.distance,
    x: norm.x,
    y: norm.y,
    z: norm.z,
    mx: meters.x,
    my: meters.y,
    mz: meters.z,
    gainDb: ch.gainDb || 0
  };
}

// Placement of a channel by name: 'virtual' (draggable object), 'direct'
// (anchored to its speaker), or null (not a bed channel).
export function channelPlacement(name) {
  const ch = channelByName(name);
  if (!ch) return null;
  return ch.spatialize ? 'virtual' : 'direct';
}

// ---------------------------------------------------------------------------
// Commit helpers (used by the panel inputs and the 3D drag)
// ---------------------------------------------------------------------------

export function applyChannelPolar(name, azimuth, elevation, distance) {
  commitChannel(name, (c) => {
    c.coordMode = 'polar';
    c.azimuth = azimuth;
    c.elevation = elevation;
    c.distance = Number(distance) > 0 ? Number(distance) : 0.01;
  });
}

// Commit from a scene-space cartesian position (mirrors
// applySpeakerSceneCartesianEdit). The bed is always stored/sent as POLAR using
// the SCENE-space spherical — the same representation the 3D gizmo produces —
// because the renderer derives each bed pose from azimuth/elevation/distance. A
// cartesian wire entry would make the renderer read a de-warped distance and
// place the channel at the wrong radius (the manual-edit-vs-gizmo mismatch).
export function applyChannelSceneCartesian(name, sx, sy, sz) {
  const sph = cartesianToSpherical({ x: sx, y: sy, z: sz });
  applyChannelPolar(name, sph.az, sph.el, Math.max(0.01, sph.dist));
}

// Commit from ADM normalized cartesian [-1, 1] (the "Norm" fields): convert to
// scene space first (exactly like applySpeakerCartesianEdit), then store as polar.
export function applyChannelCartesian(name, x, y, z) {
  const scene = normalizedOmniphonyToScenePosition({
    x: Math.max(-1, Math.min(1, Number(x) || 0)),
    y: Math.max(-1, Math.min(1, Number(y) || 0)),
    z: Math.max(-1, Math.min(1, Number(z) || 0))
  });
  applyChannelSceneCartesian(name, scene.x, scene.y, scene.z);
}

export function applyChannelGain(name, gainDb) {
  commitChannel(name, (c) => {
    c.gainDb = Math.round(Number(gainDb) || 0);
  });
}

export function applyChannelPlacement(name, spatialize) {
  commitChannel(name, (c) => {
    c.spatialize = Boolean(spatialize);
  });
}

export function resetVirtualBed() {
  app.virtualBed = null;
  invoke('control_virtual_bed', { value: '' });
  syncVirtualBedObjects(true);
  renderChannelEditor(true);
}

// ---------------------------------------------------------------------------
// Synthetic virtual objects (visible/editable at rest)
// ---------------------------------------------------------------------------

// Source ids we created from the bed; used to tell synthetic from live objects.
const syntheticIds = new Set();
let lastSyntheticSignature = null;

function liveObjectsPresent() {
  for (const key of sourceMeshes.keys()) {
    if (!syntheticIds.has(String(key))) return true;
  }
  return false;
}

function syntheticPosition(ch) {
  const directSpeakerIndex = ch.spatialize ? undefined : directSpeakerFor(ch.name);
  return {
    coordMode: 'polar',
    azimuthDeg: ch.azimuth,
    elevationDeg: ch.elevation,
    distanceM: Number(ch.distance) > 0 ? Number(ch.distance) : 1.0,
    x: 0,
    y: 0,
    z: 0,
    name: ch.name,
    gainDb: ch.gainDb,
    directSpeakerIndex,
    _noTrail: true
  };
}

// Best-effort output-speaker index for a direct channel, so the synthetic object
// snaps onto its speaker mesh (mirrors the renderer's direct_speaker_index).
function directSpeakerFor(name) {
  const key = canonicalChannelName(name);
  if (!key) return undefined;
  const speakers = Array.isArray(app.currentLayoutSpeakers) ? app.currentLayoutSpeakers : [];
  const idx = speakers.findIndex((s) => canonicalChannelName(s?.id ?? s?.name) === key);
  return idx >= 0 ? idx : undefined;
}

function removeSyntheticObjects() {
  if (syntheticIds.size === 0) return;
  for (const id of [...syntheticIds]) {
    removeSource(id);
    syntheticIds.delete(id);
  }
  lastSyntheticSignature = null;
}

/**
 * Materialize one object per channel from the bed when spatial mode is on and no
 * live stream is present; remove them otherwise. Signature-guarded to avoid
 * per-flush churn unless `force` is set.
 */
export function syncVirtualBedObjects(force = false) {
  const spatial = app.channelRenderMode !== 'host';
  if (!spatial || liveObjectsPresent()) {
    removeSyntheticObjects();
    return;
  }

  const channels = effectiveChannels();
  const signature = JSON.stringify(channels);
  if (!force && signature === lastSyntheticSignature && syntheticIds.size === channels.length) {
    return;
  }
  lastSyntheticSignature = signature;

  const wanted = new Set(channels.map((c) => c.name));
  for (const id of [...syntheticIds]) {
    if (!wanted.has(id)) {
      removeSource(id);
      syntheticIds.delete(id);
    }
  }
  for (const ch of channels) {
    syntheticIds.add(ch.name);
    updateSource(ch.name, syntheticPosition(ch));
  }
}

// ---------------------------------------------------------------------------
// Channel editor panel (mirrors the speaker editor)
// ---------------------------------------------------------------------------

function el(id) { return document.getElementById(id); }

function selectedChannelName() {
  if (app.selectedSourceId === null || app.selectedSourceId === undefined) return null;
  const name = sourceNames.get(String(app.selectedSourceId));
  return canonicalChannelName(name);
}

let lastEditorKey = null;

/**
 * (Re)build the channel editor from the selected object's channel. Hidden in
 * `host` mode or when the selected object is not a bed channel. Skips the rebuild
 * while a field is focused (so typing isn't clobbered) unless `force` is set.
 */
export function renderChannelEditor(force = false) {
  const section = el('channelEditSection');
  if (!section) return;

  // During an active gizmo drag the numeric fields are driven live by
  // previewChannelEditorFromScene; don't let a background flush rebuild them from
  // the (not-yet-committed) bed and clobber the preview.
  if (!force && app.isDraggingVirtualBed) return;

  const key = app.channelRenderMode === 'host' ? null : selectedChannelName();
  if (!key) {
    section.style.display = 'none';
    lastEditorKey = null;
    return;
  }
  if (!force && key === lastEditorKey && section.contains(document.activeElement)) return;
  lastEditorKey = key;

  const ch = channelByName(key);
  if (!ch) {
    section.style.display = 'none';
    return;
  }
  section.style.display = '';

  const titleEl = el('channelEditTitle');
  if (titleEl) titleEl.textContent = `${t('channelEdit.title')} — ${ch.name}`;

  const spatialize = ch.spatialize !== false;
  const toggle = el('channelEditSpatializeToggle');
  if (toggle) toggle.checked = spatialize;
  const toggleText = el('channelEditSpatializeText');
  if (toggleText) toggleText.textContent = spatialize ? t('virtualBed.virtual') : t('virtualBed.direct');

  const mode = app.channelEditCoordMode === 'cartesian' ? 'cartesian' : 'polar';
  const cartMode = el('channelEditCartesianMode');
  const polarMode = el('channelEditPolarMode');
  if (cartMode) cartMode.checked = mode === 'cartesian';
  if (polarMode) polarMode.checked = mode === 'polar';

  // ADM normalized cartesian + real-world metres, computed exactly like the
  // speaker editor (room-warp inverted, clamped).
  const norm = polarToAdm(ch.azimuth, ch.elevation, ch.distance);
  const meters = normalizedToMeters(norm);
  const rMeters = Math.hypot(meters.x, meters.y, meters.z);
  setValueUnlessEditing('channelEditXInput', formatNumber(norm.x, 3));
  setValueUnlessEditing('channelEditYInput', formatNumber(norm.y, 3));
  setValueUnlessEditing('channelEditZInput', formatNumber(norm.z, 3));
  setValueUnlessEditing('channelEditXMetersInput', formatNumber(meters.x, 2));
  setValueUnlessEditing('channelEditYMetersInput', formatNumber(meters.y, 2));
  setValueUnlessEditing('channelEditZMetersInput', formatNumber(meters.z, 2));
  setValueUnlessEditing('channelEditAzInput', formatNumber(ch.azimuth, 1));
  setValueUnlessEditing('channelEditElInput', formatNumber(ch.elevation, 1));
  setValueUnlessEditing('channelEditRInput', formatNumber(ch.distance, 3));
  setValueUnlessEditing('channelEditRMetersInput', formatNumber(rMeters, 2));

  const gainSlider = el('channelEditGainSlider');
  if (gainSlider) gainSlider.value = String(ch.gainDb || 0);
  const gainBox = el('channelEditGainBox');
  if (gainBox) gainBox.textContent = `${ch.gainDb > 0 ? '+' : ''}${ch.gainDb || 0} dB`;

  // Direct channels are pinned to their speaker: only Direct/Virtual + gain edit.
  const positionInputs = [
    'channelEditXInput', 'channelEditYInput', 'channelEditZInput',
    'channelEditXMetersInput', 'channelEditYMetersInput', 'channelEditZMetersInput',
    'channelEditAzInput', 'channelEditElInput', 'channelEditRInput', 'channelEditRMetersInput',
    'channelEditCartesianMode', 'channelEditPolarMode',
    'channelEditCartesianGizmoBtn', 'channelEditPolarGizmoBtn'
  ];
  for (const inputId of positionInputs) {
    const node = el(inputId);
    if (node) node.disabled = !spatialize;
  }

  const cartGizmoBtn = el('channelEditCartesianGizmoBtn');
  if (cartGizmoBtn) cartGizmoBtn.classList.toggle('active', app.cartesianEditArmed && app.activeEditMode === 'cartesian');
  const polarGizmoBtn = el('channelEditPolarGizmoBtn');
  if (polarGizmoBtn) polarGizmoBtn.classList.toggle('active', app.polarEditArmed && app.activeEditMode === 'polar');
}

function setValueUnlessEditing(id, value) {
  const node = el(id);
  if (!node) return;
  if (document.activeElement === node) return;
  node.value = value;
}

/**
 * Live-update the editor's numeric fields from a scene-space position during a
 * 3D-gizmo drag, without committing to the bed or sending OSC — mirrors how the
 * speaker editor's fields track the mesh while dragging. The authoritative
 * commit happens on release.
 */
export function previewChannelEditorFromScene(x, y, z) {
  const section = el('channelEditSection');
  if (!section || section.style.display === 'none') return;
  const norm = scenePositionToNormalizedOmniphony({ x, y, z });
  const meters = normalizedToMeters(norm);
  const rMeters = Math.hypot(meters.x, meters.y, meters.z);
  const sph = cartesianToSpherical({ x, y, z });
  setValueUnlessEditing('channelEditXInput', formatNumber(norm.x, 3));
  setValueUnlessEditing('channelEditYInput', formatNumber(norm.y, 3));
  setValueUnlessEditing('channelEditZInput', formatNumber(norm.z, 3));
  setValueUnlessEditing('channelEditXMetersInput', formatNumber(meters.x, 2));
  setValueUnlessEditing('channelEditYMetersInput', formatNumber(meters.y, 2));
  setValueUnlessEditing('channelEditZMetersInput', formatNumber(meters.z, 2));
  setValueUnlessEditing('channelEditAzInput', formatNumber(sph.az, 1));
  setValueUnlessEditing('channelEditElInput', formatNumber(sph.el, 1));
  setValueUnlessEditing('channelEditRInput', formatNumber(Math.max(0.01, sph.dist), 3));
  setValueUnlessEditing('channelEditRMetersInput', formatNumber(rMeters, 2));
}
