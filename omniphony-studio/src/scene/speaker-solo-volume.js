/**
 * Per-speaker heatmap — ray-marched volume of a single speaker's gain field.
 *
 * Replaces the old OSC-sampled per-speaker "volume" mode: reads the local gain
 * table (no per-display OSC request) and renders the selected speaker's coverage
 * as energy(cell) = gain(cell, speaker)². Same renderer/box/gradient as the object
 * and combined-speaker volumes; reuses their gradient/mix/γ/opacity/resolution.
 *
 * Driven by the existing "Heatmap volume" toggle (`speakerHeatmapVolumeEnabled`)
 * + `selectedSpeakerIndex`. Static (no live levels) — it shows where the speaker
 * is panned, not the live signal.
 */

import { app } from '../state.js';
import { EnergyVolume } from './energy-volume-core.js';
import { MIN_REBUILD_INTERVAL_MS, clampVolumeGamma, colormapIndex } from './object-energy-shared.js';
import { getSpeakerGainTable } from './speaker-gaintable.js';

const volume = new EnergyVolume();

export function hideSpeakerSoloVolume() {
  volume.hide();
}

export function clearSpeakerSoloVolume() {
  volume.dispose();
}

function clampIdx(value, n) {
  if (value < 0) return 0;
  if (value > n - 1) return n - 1;
  return value;
}

export function refreshSpeakerSoloVolume(nowMs) {
  const table = getSpeakerGainTable();
  const speaker = app.selectedSpeakerIndex;
  if (
    !app.speakerHeatmapVolumeEnabled
    || !table
    || !Number.isInteger(speaker)
    || speaker < 0
    || speaker >= table.sc
  ) {
    volume.hide();
    return;
  }

  const now = Number.isFinite(nowMs) ? nowMs : performance.now();
  if (now - (app.lastSpeakerSoloVolumeAt || 0) < MIN_REBUILD_INTERVAL_MS) {
    return;
  }
  app.lastSpeakerSoloVolumeAt = now;

  const { nx, ny, nz, sc, gains } = table;
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
      const g = gains[(xi + nx * (yi + ny * zi)) * sc + speaker];
      return g * g;
    },
  });
}
