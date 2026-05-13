/**
 * Modal dialog and accordion section toggle helpers.
 */

import { app } from './state.js';
import { emitOverlayLayoutChanged } from './ui/layout/overlay-layout-state.js';

// ---------------------------------------------------------------------------
// DOM refs (queried once at module load)
// ---------------------------------------------------------------------------

function getTrailInfoModalEl() { return document.getElementById('trailInfoModal'); }
function getEffectiveRenderInfoModalEl() { return document.getElementById('effectiveRenderInfoModal'); }
function getOscInfoModalEl() { return document.getElementById('oscInfoModal'); }
function getAboutModalEl() { return document.getElementById('aboutModal'); }
function getRoomGeometryInfoModalEl() { return document.getElementById('roomGeometryInfoModal'); }
function getAdaptiveResamplingInfoModalEl() { return document.getElementById('adaptiveResamplingInfoModal'); }
function getTelemetryGaugesInfoModalEl() { return document.getElementById('telemetryGaugesInfoModal'); }
function getRampModeInfoModalEl() { return document.getElementById('rampModeInfoModal'); }
function getVbapPositionInterpolationInfoModalEl() { return document.getElementById('vbapPositionInterpolationInfoModal'); }
function getEvaluationInfoModalEl() { return document.getElementById('evaluationInfoModal'); }
function getBackendInfoModalEl() { return document.getElementById('backendInfoModal'); }
function getDistanceModelInfoModalEl() { return document.getElementById('distanceModelInfoModal'); }
function getBarycenterInfoModalEl() { return document.getElementById('barycenterInfoModal'); }
function getExperimentalDistanceInfoModalEl() { return document.getElementById('experimentalDistanceInfoModal'); }
function getInputInfoModalEl() { return document.getElementById('inputInfoModal'); }
function getInputClockInfoModalEl() { return document.getElementById('inputClockInfoModal'); }
function getInputLfeInfoModalEl() { return document.getElementById('inputLfeInfoModal'); }
function getDrcInfoModalEl() { return document.getElementById('drcInfoModal'); }
function getHeatmapInfoModalEl() { return document.getElementById('heatmapInfoModal'); }
function getTelemetryGaugesFormEl() { return document.getElementById('telemetryGaugesForm'); }
function getTelemetryGaugesToggleBtnEl() { return document.getElementById('telemetryGaugesToggleBtn'); }
function getDisplaySectionContentEl() { return document.getElementById('displaySectionContent'); }
function getDisplaySectionToggleBtnEl() { return document.getElementById('displaySectionToggleBtn'); }
function getDrcSectionContentEl() { return document.getElementById('drcSectionContent'); }
function getDrcSummaryEl() { return document.getElementById('drcSummary'); }
function getDrcSectionToggleBtnEl() { return document.getElementById('drcSectionToggleBtn'); }
function getAudioOutputSectionContentEl() { return document.getElementById('audioOutputSectionContent'); }
function getAudioOutputSummaryEl() { return document.getElementById('audioOutputSummary'); }
function getAudioOutputSectionToggleBtnEl() { return document.getElementById('audioOutputSectionToggleBtn'); }
function getInputSectionContentEl() { return document.getElementById('inputSectionContent'); }
function getInputSummaryEl() { return document.getElementById('inputSummary'); }
function getInputSectionToggleBtnEl() { return document.getElementById('inputSectionToggleBtn'); }
function getRendererSectionContentEl() { return document.getElementById('rendererSectionContent'); }
function getRendererSummaryEl() { return document.getElementById('rendererSummary'); }
function getRendererSectionToggleBtnEl() { return document.getElementById('rendererSectionToggleBtn'); }
function getSpreadFromDistanceInfoModalEl() { return document.getElementById('spreadFromDistanceInfoModal'); }
function getDistanceDiffuseInfoModalEl() { return document.getElementById('distanceDiffuseInfoModal'); }

// ---------------------------------------------------------------------------
// Simple info modals
// ---------------------------------------------------------------------------

export function setTrailInfoModalOpen(open) {
  const trailInfoModalEl = getTrailInfoModalEl();
  if (!trailInfoModalEl) return;
  trailInfoModalEl.classList.toggle('open', Boolean(open));
}

export function setEffectiveRenderInfoModalOpen(open) {
  const effectiveRenderInfoModalEl = getEffectiveRenderInfoModalEl();
  if (!effectiveRenderInfoModalEl) return;
  effectiveRenderInfoModalEl.classList.toggle('open', Boolean(open));
}

export function setOscInfoModalOpen(open) {
  const oscInfoModalEl = getOscInfoModalEl();
  if (!oscInfoModalEl) return;
  oscInfoModalEl.classList.toggle('open', Boolean(open));
}

export function setAboutModalOpen(open) {
  const aboutModalEl = getAboutModalEl();
  if (!aboutModalEl) return;
  aboutModalEl.classList.toggle('open', Boolean(open));
}

export function setRoomGeometryInfoModalOpen(open) {
  const roomGeometryInfoModalEl = getRoomGeometryInfoModalEl();
  if (!roomGeometryInfoModalEl) return;
  roomGeometryInfoModalEl.classList.toggle('open', Boolean(open));
}

export function setAdaptiveResamplingInfoModalOpen(open) {
  const adaptiveResamplingInfoModalEl = getAdaptiveResamplingInfoModalEl();
  if (!adaptiveResamplingInfoModalEl) return;
  adaptiveResamplingInfoModalEl.classList.toggle('open', Boolean(open));
}

export function setTelemetryGaugesInfoModalOpen(open) {
  const telemetryGaugesInfoModalEl = getTelemetryGaugesInfoModalEl();
  if (!telemetryGaugesInfoModalEl) return;
  telemetryGaugesInfoModalEl.classList.toggle('open', Boolean(open));
}

export function setRampModeInfoModalOpen(open) {
  const rampModeInfoModalEl = getRampModeInfoModalEl();
  if (!rampModeInfoModalEl) return;
  rampModeInfoModalEl.classList.toggle('open', Boolean(open));
}

export function setVbapPositionInterpolationInfoModalOpen(open) {
  const vbapPositionInterpolationInfoModalEl = getVbapPositionInterpolationInfoModalEl();
  if (!vbapPositionInterpolationInfoModalEl) return;
  vbapPositionInterpolationInfoModalEl.classList.toggle('open', Boolean(open));
}

export function setEvaluationInfoModalOpen(open) {
  const evaluationInfoModalEl = getEvaluationInfoModalEl();
  if (!evaluationInfoModalEl) return;
  evaluationInfoModalEl.classList.toggle('open', Boolean(open));
}

export function setBackendInfoModalOpen(open) {
  const backendInfoModalEl = getBackendInfoModalEl();
  if (!backendInfoModalEl) return;
  backendInfoModalEl.classList.toggle('open', Boolean(open));
}

export function setDistanceModelInfoModalOpen(open) {
  const distanceModelInfoModalEl = getDistanceModelInfoModalEl();
  if (!distanceModelInfoModalEl) return;
  distanceModelInfoModalEl.classList.toggle('open', Boolean(open));
}

export function setBarycenterInfoModalOpen(open) {
  const barycenterInfoModalEl = getBarycenterInfoModalEl();
  if (!barycenterInfoModalEl) return;
  barycenterInfoModalEl.classList.toggle('open', Boolean(open));
}

export function setExperimentalDistanceInfoModalOpen(open) {
  const experimentalDistanceInfoModalEl = getExperimentalDistanceInfoModalEl();
  if (!experimentalDistanceInfoModalEl) return;
  experimentalDistanceInfoModalEl.classList.toggle('open', Boolean(open));
}

export function setInputInfoModalOpen(open) {
  const inputInfoModalEl = getInputInfoModalEl();
  if (!inputInfoModalEl) return;
  inputInfoModalEl.classList.toggle('open', Boolean(open));
}

export function setInputClockInfoModalOpen(open) {
  const inputClockInfoModalEl = getInputClockInfoModalEl();
  if (!inputClockInfoModalEl) return;
  inputClockInfoModalEl.classList.toggle('open', Boolean(open));
}

export function setInputLfeInfoModalOpen(open) {
  const inputLfeInfoModalEl = getInputLfeInfoModalEl();
  if (!inputLfeInfoModalEl) return;
  inputLfeInfoModalEl.classList.toggle('open', Boolean(open));
}

export function setDrcInfoModalOpen(open) {
  const drcInfoModalEl = getDrcInfoModalEl();
  if (!drcInfoModalEl) return;
  drcInfoModalEl.classList.toggle('open', Boolean(open));
}

export function setHeatmapInfoModalOpen(open) {
  const heatmapInfoModalEl = getHeatmapInfoModalEl();
  if (!heatmapInfoModalEl) return;
  heatmapInfoModalEl.classList.toggle('open', Boolean(open));
}

export function setSpreadFromDistanceInfoModalOpen(open) {
  const spreadFromDistanceInfoModalEl = getSpreadFromDistanceInfoModalEl();
  if (!spreadFromDistanceInfoModalEl) return;
  spreadFromDistanceInfoModalEl.classList.toggle('open', Boolean(open));
}

export function setDistanceDiffuseInfoModalOpen(open) {
  const distanceDiffuseInfoModalEl = getDistanceDiffuseInfoModalEl();
  if (!distanceDiffuseInfoModalEl) return;
  distanceDiffuseInfoModalEl.classList.toggle('open', Boolean(open));
}

// ---------------------------------------------------------------------------
// Accordion sections (update shared state)
// ---------------------------------------------------------------------------

export function setTelemetryGaugesOpen(open) {
  const telemetryGaugesFormEl = getTelemetryGaugesFormEl();
  const telemetryGaugesToggleBtnEl = getTelemetryGaugesToggleBtnEl();
  app.telemetryGaugesOpen = Boolean(open);
  if (telemetryGaugesFormEl) {
    telemetryGaugesFormEl.classList.toggle('open', app.telemetryGaugesOpen);
  }
  if (telemetryGaugesToggleBtnEl) {
    telemetryGaugesToggleBtnEl.textContent = app.telemetryGaugesOpen ? '▾' : '▸';
  }
  emitOverlayLayoutChanged('telemetry-gauges-toggle');
}

export function setDisplaySectionOpen(open) {
  const displaySectionContentEl = getDisplaySectionContentEl();
  const displaySectionToggleBtnEl = getDisplaySectionToggleBtnEl();
  app.displaySectionOpen = Boolean(open);
  if (displaySectionContentEl) {
    displaySectionContentEl.classList.toggle('open', app.displaySectionOpen);
  }
  if (displaySectionToggleBtnEl) {
    displaySectionToggleBtnEl.textContent = app.displaySectionOpen ? '▾' : '▸';
  }
  emitOverlayLayoutChanged('display-section-toggle');
}

export function setDrcSectionOpen(open) {
  const drcSectionContentEl = getDrcSectionContentEl();
  const drcSummaryEl = getDrcSummaryEl();
  const drcSectionToggleBtnEl = getDrcSectionToggleBtnEl();
  app.drcSectionOpen = Boolean(open);
  if (drcSectionContentEl) {
    drcSectionContentEl.classList.toggle('open', app.drcSectionOpen);
  }
  if (drcSummaryEl) {
    drcSummaryEl.style.display = app.drcSectionOpen ? 'none' : 'block';
  }
  if (drcSectionToggleBtnEl) {
    drcSectionToggleBtnEl.textContent = app.drcSectionOpen ? '▾' : '▸';
  }
  emitOverlayLayoutChanged('drc-section-toggle');
}

export function setAudioOutputSectionOpen(open) {
  const audioOutputSectionContentEl = getAudioOutputSectionContentEl();
  const audioOutputSummaryEl = getAudioOutputSummaryEl();
  const audioOutputSectionToggleBtnEl = getAudioOutputSectionToggleBtnEl();
  app.audioOutputSectionOpen = Boolean(open);
  if (audioOutputSectionContentEl) {
    audioOutputSectionContentEl.classList.toggle('open', app.audioOutputSectionOpen);
  }
  if (audioOutputSummaryEl) {
    audioOutputSummaryEl.style.display = app.audioOutputSectionOpen ? 'none' : 'block';
  }
  if (audioOutputSectionToggleBtnEl) {
    audioOutputSectionToggleBtnEl.textContent = app.audioOutputSectionOpen ? '▾' : '▸';
  }
  emitOverlayLayoutChanged('audio-output-section-toggle');
}

export function setInputSectionOpen(open) {
  const inputSectionContentEl = getInputSectionContentEl();
  const inputSummaryEl = getInputSummaryEl();
  const inputSectionToggleBtnEl = getInputSectionToggleBtnEl();
  app.inputSectionOpen = Boolean(open);
  if (inputSectionContentEl) {
    inputSectionContentEl.classList.toggle('open', app.inputSectionOpen);
  }
  if (inputSummaryEl) {
    inputSummaryEl.style.display = app.inputSectionOpen ? 'none' : 'block';
  }
  if (inputSectionToggleBtnEl) {
    inputSectionToggleBtnEl.textContent = app.inputSectionOpen ? '▾' : '▸';
  }
  emitOverlayLayoutChanged('input-section-toggle');
}

export function setRendererSectionOpen(open) {
  const rendererSectionContentEl = getRendererSectionContentEl();
  const rendererSummaryEl = getRendererSummaryEl();
  const rendererSectionToggleBtnEl = getRendererSectionToggleBtnEl();
  app.rendererSectionOpen = Boolean(open);
  if (rendererSectionContentEl) {
    rendererSectionContentEl.classList.toggle('open', app.rendererSectionOpen);
  }
  if (rendererSummaryEl) {
    rendererSummaryEl.style.display = app.rendererSectionOpen ? 'none' : 'block';
  }
  if (rendererSectionToggleBtnEl) {
    rendererSectionToggleBtnEl.textContent = app.rendererSectionOpen ? '▾' : '▸';
  }
  emitOverlayLayoutChanged('renderer-section-toggle');
}

export function collapseRuntimeSections() {
  setTelemetryGaugesOpen(false);
  setDisplaySectionOpen(false);
  setDrcSectionOpen(false);
  setAudioOutputSectionOpen(false);
  setInputSectionOpen(false);
  setRendererSectionOpen(false);
}
