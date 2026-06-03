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
  const previousValue = String(visibleIndex);
  const existing = Array.from(selectEl.options).map((option) => option.textContent);
  const needsRebuild = existing.length !== labels.length
    || existing.some((label, index) => label !== labels[index]);
  if (needsRebuild) {
    selectEl.replaceChildren();
    labels.forEach((label, index) => {
      const option = document.createElement('option');
      option.value = String(index);
      option.textContent = label;
      selectEl.appendChild(option);
    });
  }
  selectEl.value = previousValue;
  return labels;
}
