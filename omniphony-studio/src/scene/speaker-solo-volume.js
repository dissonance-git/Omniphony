/**
 * Per-speaker heatmap — ray-marched volume of a single speaker's gain field.
 *
 * Replaces the old OSC-sampled per-speaker "volume" mode: reads the local gain
 * table (no per-display OSC request) and renders the selected speaker's coverage
 * as energy(cell) = gain(cell, speaker)². Same renderer/box as the object volume
 * and shares its mix/γ/opacity/resolution, but has its own gradient
 * (`speakerHeatmapVolumeColormap`).
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

/** Nearest cell index in a (small) position array, by absolute distance. */
function nearestIndex(positions, n, value) {
  let best = 0;
  let bestDist = Infinity;
  for (let i = 0; i < n; i += 1) {
    const d = Math.abs(positions[i] - value);
    if (d < bestDist) {
      bestDist = d;
      best = i;
    }
  }
  return best;
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

  const { nx, ny, nz, sc, gains, zPositions } = table;
  const nxh = nx - 1;
  const nyh = ny - 1;
  const nzh = nz - 1;

  // The gain table's height (z) axis is NON-uniform: `cartesian_z_axis` packs a
  // few cells into the negative region and the rest into [0, 1], so z=0 lands at
  // table index `z_neg_size`, not the middle. Mapping omni height linearly would
  // drag the field toward the floor (z=0 rendered at the box bottom). Map it
  // through the real cell-centre positions instead. x (width) and y (depth) are
  // evenly spaced over [-1, 1], so they stay linear. oh is constant across the
  // inner depth loop → memoise the last height→cell lookup.
  let cachedOh = NaN;
  let cachedZi = 0;
  const lookupZi = (oh) => {
    if (oh === cachedOh) return cachedZi;
    cachedOh = oh;
    cachedZi = zPositions
      ? nearestIndex(zPositions, nz, oh)
      : clampIdx(Math.round(((oh + 1) * 0.5) * nzh), nz);
    return cachedZi;
  };

  volume.update({
    resolution: app.objectEnergyHeatmapResolution,
    opacity: app.objectEnergyHeatmapOpacity,
    mix: app.objectEnergyVolumeMix,
    gammaAccumulate: clampVolumeGamma('accumulate', app.objectEnergyVolumeGammaAccumulate),
    gammaMip: clampVolumeGamma('mip', app.objectEnergyVolumeGammaMip),
    colormap: colormapIndex(app.speakerHeatmapVolumeColormap),
    // Gain-table axes: x = width (ow), y = depth (od), z = height (oh).
    sampleEnergy: (ow, od, oh) => {
      const xi = clampIdx(Math.round(((ow + 1) * 0.5) * nxh), nx);
      const yi = clampIdx(Math.round(((od + 1) * 0.5) * nyh), ny);
      const zi = lookupZi(oh);
      const g = gains[(xi + nx * (yi + ny * zi)) * sc + speaker];
      return g * g;
    },
  });
}
