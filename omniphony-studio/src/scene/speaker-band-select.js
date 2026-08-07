/**
 * Crossover-band selector ("Heatmap band").
 *
 * The per-speaker heatmap that originally owned this control is gone, but the
 * band index it drives is still read by `computeEffectiveRenderPosition`
 * (sources.js) and `getObjectDominantSpeakerText` (speakers.js) to pick which
 * crossover band's gains to visualise. Keep this tiny sync helper so those
 * features still work for multi-band layouts.
 */

import { app } from '../state.js';
import { computeCrossoverBandLabels } from '../crossover-bands.js';

function getSpeakerHeatmapBandSelectEl() {
  return document.getElementById('speakerHeatmapBandSelect');
}

export function syncSpeakerHeatmapBandSelect() {
  const selectEl = getSpeakerHeatmapBandSelectEl();
  const labels = computeCrossoverBandLabels(app.currentLayoutSpeakers, {
    includeSingleBand: true,
    singleBandLabel: 'Full band',
  }) || ['Full band'];
  const hasLayoutSpeakers = Array.isArray(app.currentLayoutSpeakers) && app.currentLayoutSpeakers.length > 0;
  const desiredIndex = Math.max(0, Math.round(Number(app.speakerHeatmapBandIndex) || 0));
  const maxIndex = Math.max(0, labels.length - 1);
  if (hasLayoutSpeakers) {
    app.speakerHeatmapBandIndex = Math.max(0, Math.min(maxIndex, desiredIndex));
  }
  if (!selectEl) {
    return labels;
  }
  const visibleIndex = hasLayoutSpeakers
    ? app.speakerHeatmapBandIndex
    : Math.max(0, Math.min(maxIndex, desiredIndex));

  // Option list: one per crossover band, plus a "Toutes" entry (only when there's
  // more than one band) for the heatmap's all-bands composite.
  const optionDefs = labels.map((label, index) => ({ value: String(index), text: label }));
  if (labels.length > 1) {
    optionDefs.push({ value: 'all', text: 'Toutes' });
  }
  const existing = Array.from(selectEl.options).map((option) => option.value);
  const needsRebuild = existing.length !== optionDefs.length
    || existing.some((value, index) => value !== optionDefs[index].value);
  if (needsRebuild) {
    selectEl.replaceChildren();
    optionDefs.forEach((def) => {
      const option = document.createElement('option');
      option.value = def.value;
      option.textContent = def.text;
      selectEl.appendChild(option);
    });
  }
  selectEl.value = app.speakerHeatmapAllBands && labels.length > 1
    ? 'all'
    : String(visibleIndex);
  return labels;
}

/**
 * Refresh EVERY crossover-band selector after a layout/band change.
 *
 * Call this rather than a single selector's sync: the band list is derived from
 * the speaker layout, so a caller that refreshes one and not the other leaves a
 * stale dropdown — which is exactly how the global heatmap's selector stopped
 * following new bands. One entry point means a future selector is picked up by
 * every existing call site for free.
 */
export function syncCrossoverBandSelects() {
  const labels = syncSpeakerHeatmapBandSelect();
  syncGlobalEnergyBandSelect();
  return labels;
}

/**
 * Same options for the global energy heatmap, minus the all-bands entry: that
 * composite blends band *colours*, which a diverging dB scale has no room for.
 */
export function syncGlobalEnergyBandSelect() {
  const selectEl = document.getElementById('globalEnergyHeatmapBandSelect');
  const labels = computeCrossoverBandLabels(app.currentLayoutSpeakers, {
    includeSingleBand: true,
    singleBandLabel: 'Full band',
  }) || ['Full band'];
  const maxIndex = Math.max(0, labels.length - 1);
  const desired = Math.max(0, Math.round(Number(app.globalEnergyHeatmapBandIndex) || 0));
  app.globalEnergyHeatmapBandIndex = Math.min(maxIndex, desired);
  if (!selectEl) return labels;

  const existing = Array.from(selectEl.options).map((option) => option.value);
  const needsRebuild = existing.length !== labels.length
    || existing.some((value, index) => value !== String(index));
  if (needsRebuild) {
    selectEl.replaceChildren();
    labels.forEach((label, index) => {
      const option = document.createElement('option');
      option.value = String(index);
      option.textContent = label;
      selectEl.appendChild(option);
    });
  }
  selectEl.value = String(app.globalEnergyHeatmapBandIndex);
  return labels;
}
