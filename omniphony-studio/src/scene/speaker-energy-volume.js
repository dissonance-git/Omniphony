/**
 * Speaker energy field — ray-marched volume (gain-table driven).
 *
 * Same renderer as the object field (energy-volume-core.js), but the per-cell
 * energy comes from the renderer's precomputed VBAP gain table × live speaker
 * levels:  energy(cell) = Σ_speaker (gain · level)².  The gain table is the
 * cartesian evaluation artifact shipped once over OSC (P1/P2) and decoded by the
 * Tauri backend into `{ xCount, yCount, zCount, speakerCount, gains }`, a regular
 * grid over Omniphony-normalised [-1,1]³ (x=width, y=depth, z=height — same frame
 * as objects). Lookup is nearest-cell; live levels react each tick.
 *
 * For v1 the gradient / mix / γ / opacity / resolution settings are shared with
 * the object field (no separate sliders), only the enable toggle is dedicated.
 */

import { app, speakerLevels } from '../state.js';
import { EnergyVolume } from './energy-volume-core.js';
import { MIN_REBUILD_INTERVAL_MS, clampVolumeGamma, colormapIndex } from './object-energy-shared.js';

const SILENT_RMS_DBFS = -100;

const volume = new EnergyVolume();

// Decoded cartesian gain table, or null until the first `speaker_gaintable` event.
let table = null;
// Reused per-tick linear level array (grows to speakerCount).
let levelLin = null;

/**
 * Store the decoded gain table pushed by the Tauri backend. Cartesian only for
 * v1 (polar artifacts are ignored — the field falls back to hidden).
 */
export function setSpeakerGainTable(payload) {
  if (!payload || payload.domain !== 'cartesian') {
    table = null;
    return;
  }
  const nx = Number(payload.xCount) | 0;
  const ny = Number(payload.yCount) | 0;
  const nz = Number(payload.zCount) | 0;
  const sc = Number(payload.speakerCount) | 0;
  const gains = Array.isArray(payload.gains) ? Float32Array.from(payload.gains) : null;
  if (nx < 1 || ny < 1 || nz < 1 || sc < 1 || !gains || gains.length < nx * ny * nz * sc) {
    table = null;
    return;
  }
  table = { nx, ny, nz, sc, gains };
}

export function hasSpeakerGainTable() {
  return table !== null;
}

export function hideSpeakerEnergyVolume() {
  volume.hide();
}

export function clearSpeakerEnergyVolume() {
  volume.dispose();
}

function clampIdx(value, n) {
  if (value < 0) return 0;
  if (value > n - 1) return n - 1;
  return value;
}

export function refreshSpeakerEnergyVolume(nowMs) {
  if (!app.speakerEnergyVolumeEnabled || !table) {
    volume.hide();
    return;
  }

  const now = Number.isFinite(nowMs) ? nowMs : performance.now();
  if (now - (app.lastSpeakerEnergyVolumeAt || 0) < MIN_REBUILD_INTERVAL_MS) {
    return;
  }
  app.lastSpeakerEnergyVolumeAt = now;

  const { nx, ny, nz, sc, gains } = table;

  // Per-speaker linear level (dBFS RMS → power), in gain-table column order
  // (= speaker layout index = speakerLevels key).
  if (!levelLin || levelLin.length < sc) {
    levelLin = new Float32Array(sc);
  }
  let anyLevel = false;
  for (let s = 0; s < sc; s += 1) {
    const meter = speakerLevels.get(String(s));
    const db = meter ? Number(meter.rmsDbfs) : SILENT_RMS_DBFS;
    const lin = Number.isFinite(db) && db > SILENT_RMS_DBFS ? Math.pow(10, db / 10) : 0;
    levelLin[s] = lin;
    if (lin > 0) anyLevel = true;
  }
  if (!anyLevel) {
    volume.hide();
    return;
  }

  const nxh = nx - 1;
  const nyh = ny - 1;
  const nzh = nz - 1;

  volume.update({
    resolution: app.objectEnergyHeatmapResolution,
    opacity: app.objectEnergyHeatmapOpacity,
    mix: app.objectEnergyVolumeMix,
    gammaAccumulate: clampVolumeGamma('accumulate', app.objectEnergyVolumeGammaAccumulate),
    gammaMip: clampVolumeGamma('mip', app.objectEnergyVolumeGammaMip),
    colormap: colormapIndex(app.objectEnergyColormap),
    // Gain-table axes: x = width (ow), y = depth (od), z = height (oh).
    sampleEnergy: (ow, od, oh) => {
      const xi = clampIdx(Math.round(((ow + 1) * 0.5) * nxh), nx);
      const yi = clampIdx(Math.round(((od + 1) * 0.5) * nyh), ny);
      const zi = clampIdx(Math.round(((oh + 1) * 0.5) * nzh), nz);
      const base = (xi + nx * (yi + ny * zi)) * sc;
      let energy = 0;
      for (let s = 0; s < sc; s += 1) {
        const g = gains[base + s] * levelLin[s];
        energy += g * g;
      }
      return energy;
    },
  });
}
