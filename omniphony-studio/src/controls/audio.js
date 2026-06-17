/**
 * Audio format display controls.
 *
 * Extracted from app.js (lines 4295-4378).
 */

import { invoke } from '@tauri-apps/api/core';
import {
  app,
  dirty,
  AUDIO_SAMPLE_RATE_PRESETS,
  hasProducerDomain
} from '../state.js';
import { t, tf } from '../i18n.js';
import { scheduleUIFlush } from '../flush.js';
import { inAudioPanel, inRendererPanel } from '../ui/panel-roots.js';
import { syncVirtualBedObjects, renderChannelEditor } from './virtual-bed.js';

function getAudioFormatInfoEl() { return inAudioPanel('audioFormatInfo'); }
function getAudioOutputDeviceSelectEl() { return inAudioPanel('audioOutputDeviceSelect'); }
function getRampModeSelectEl() { return inRendererPanel('rampModeSelect'); }
function getChannelSpatializeToggleEl() { return document.getElementById('channelSpatializeToggle'); }
function getAudioSampleRateInputEl() { return inAudioPanel('audioSampleRateInput'); }
function getAudioSampleRateMenuEl() { return inAudioPanel('audioSampleRateMenu'); }
function getAudioOutputSummaryEl() { return inAudioPanel('audioOutputSummary'); }

export function buildAudioConfigPayload() {
  return {
    outputDevice: app.audioOutputDevice || null,
    sampleRate: app.audioSampleRate || null,
    latencyTargetMs: app.latencyRequestedMs || app.latencyTargetMs || null,
    adaptiveResampling: {
      enabled: app.adaptiveResamplingEnabled === true,
      enableFarMode: app.adaptiveResamplingEnableFarMode === true,
      forceSilenceInFarMode: app.adaptiveResamplingForceSilenceInFarMode === true,
      hardRecoverHighInFarMode: app.adaptiveResamplingHardRecoverHighInFarMode === true,
      hardRecoverLowInFarMode: app.adaptiveResamplingHardRecoverLowInFarMode === true,
      farModeReturnFadeInMs: Math.max(0, Math.round(app.adaptiveResamplingFarModeReturnFadeInMs ?? 0)),
      kpNear: Number(app.adaptiveResamplingKpNear ?? 1),
      ki: Number(app.adaptiveResamplingKi ?? 1),
      integralDischargeRatio: Number(app.adaptiveResamplingIntegralDischargeRatio ?? 0.25),
      maxAdjust: Number(app.adaptiveResamplingMaxAdjust ?? 0.01),
      highRecoverEntryMarginMs: Math.max(1, Math.round(app.adaptiveResamplingHighRecoverEntryMarginMs ?? 1000)),
      updateIntervalCallbacks: Math.max(1, Math.round(app.adaptiveResamplingUpdateIntervalCallbacks ?? 1)),
      lowRecoverSettleStableMs: Math.max(0, Number(app.adaptiveResamplingLowRecoverSettleStableMs ?? 200)),
      lowRecoverEntryMarginMs: Math.max(0, Number(app.adaptiveResamplingLowRecoverEntryMarginMs ?? 18)),
      lowRecoverExitMarginMs: Math.max(0, Number(app.adaptiveResamplingLowRecoverExitMarginMs ?? 6)),
      lowRecoverSettleMarginMs: Math.max(0, Number(app.adaptiveResamplingLowRecoverSettleMarginMs ?? 6)),
      lowRecoverRefillDeltaAlpha: Math.min(1, Math.max(0, Number(app.adaptiveResamplingLowRecoverRefillDeltaAlpha ?? 0.5))),
      controlSmoothingCutoffHz: Math.max(0.001, Number(app.adaptiveResamplingControlSmoothingCutoffHz ?? 0.5)),
      controlSmoothingOrder: Math.min(2, Math.max(1, Math.round(Number(app.adaptiveResamplingControlSmoothingOrder ?? 1)))),
      paused: app.adaptiveResamplingPaused === true,
      usePreBridgeClock: app.adaptiveResamplingUsePreBridgeClock === true,
      useOutputPacing: app.adaptiveResamplingUseOutputPacing === true,
      disableBackpressure: app.adaptiveResamplingDisableBackpressure === true
    }
  };
}

export function sendAudioConfig({ apply = true } = {}) {
  const payload = buildAudioConfigPayload();
  return invoke('control_audio_config', { payload }).then(() => {
    if (!apply) return null;
    return invoke('control_audio_config_apply');
  });
}

export function renderAudioFormatDisplay() {
  const audioFormatInfoEl = getAudioFormatInfoEl();
  const audioOutputDeviceSelectEl = getAudioOutputDeviceSelectEl();
  const rampModeSelectEl = getRampModeSelectEl();
  const audioSampleRateInputEl = getAudioSampleRateInputEl();
  const audioOutputSummaryEl = getAudioOutputSummaryEl();
  const hasAudioDomain = hasProducerDomain('audio');
  if (audioFormatInfoEl) {
    const rateText = app.audioSampleRate ? `${app.audioSampleRate} Hz` : '—';
    const fmtText = app.audioSampleFormat || '—';
    const baseText = tf('status.audioFormat', { rate: rateText, format: fmtText });
    audioFormatInfoEl.textContent = app.audioError ? `${baseText} • Error: ${app.audioError}` : baseText;
  }
  if (audioOutputDeviceSelectEl) {
    const defaultLabel = app.oscSnapshotReady ? t('status.defaultOutputDevice') : '—';
    const options = [{ value: '', label: defaultLabel }, ...app.audioOutputDevices];
    if (app.audioOutputDevice && !options.some((entry) => entry.value === app.audioOutputDevice)) {
      options.push({ value: app.audioOutputDevice, label: app.audioOutputDevice });
    }
    const selectedValue = app.audioOutputDeviceEditing
      ? String(audioOutputDeviceSelectEl.value || '')
      : (app.audioOutputDevice || '');
    audioOutputDeviceSelectEl.innerHTML = '';
    options.forEach((entry) => {
      const optionEl = document.createElement('option');
      optionEl.value = entry.value;
      optionEl.textContent = entry.label || entry.value || t('status.defaultOutputDevice');
      audioOutputDeviceSelectEl.appendChild(optionEl);
    });
    audioOutputDeviceSelectEl.value = options.some((entry) => entry.value === selectedValue)
      ? selectedValue
      : '';
    audioOutputDeviceSelectEl.disabled = !app.oscSnapshotReady || !hasAudioDomain;
  }
  // Output backend selector + device/file rows.
  const isFileBackend = app.audioOutputBackend === 'file';
  const audioOutputBackendSelectEl = inAudioPanel('audioOutputBackendSelect');
  const audioOutputDeviceRowEl = inAudioPanel('audioOutputDeviceRow');
  const audioOutputFileRowEl = inAudioPanel('audioOutputFileRow');
  const audioOutputFileFormatRowEl = inAudioPanel('audioOutputFileFormatRow');
  const audioOutputFileInputEl = inAudioPanel('audioOutputFileInput');
  const audioOutputFileFormatSelectEl = inAudioPanel('audioOutputFileFormatSelect');
  if (audioOutputBackendSelectEl) {
    audioOutputBackendSelectEl.value = isFileBackend ? 'file' : 'device';
    audioOutputBackendSelectEl.disabled = !app.oscSnapshotReady || !hasAudioDomain;
  }
  if (audioOutputDeviceRowEl) audioOutputDeviceRowEl.style.display = isFileBackend ? 'none' : '';
  if (audioOutputFileRowEl) audioOutputFileRowEl.style.display = isFileBackend ? '' : 'none';
  if (audioOutputFileFormatRowEl) audioOutputFileFormatRowEl.style.display = isFileBackend ? '' : 'none';
  if (audioOutputFileInputEl && !app.audioOutputFileEditing) {
    audioOutputFileInputEl.value = app.audioOutputFile || '-';
    audioOutputFileInputEl.disabled = !app.oscSnapshotReady || !hasAudioDomain;
  }
  if (audioOutputFileFormatSelectEl) {
    audioOutputFileFormatSelectEl.value = ['raw_f32', 'caf'].includes(app.audioOutputFileFormat)
      ? app.audioOutputFileFormat
      : 'raw_f32';
    audioOutputFileFormatSelectEl.disabled = !app.oscSnapshotReady || !hasAudioDomain;
  }
  if (rampModeSelectEl) {
    rampModeSelectEl.value = ['off', 'frame', 'sample', 'interp'].includes(app.rampMode) ? app.rampMode : 'frame';
  }
  const channelSpatializeToggleEl = getChannelSpatializeToggleEl();
  if (channelSpatializeToggleEl) {
    // Off = host (let the player decode), on = spatial (render through the
    // virtual bed). Legacy `direct`/`virtual` snapshots count as spatial.
    const spatial = app.channelRenderMode !== 'host';
    channelSpatializeToggleEl.checked = spatial;
    const virtualBedActions = document.getElementById('virtualBedActions');
    if (virtualBedActions) virtualBedActions.style.display = spatial ? 'flex' : 'none';
    const surroundRow = document.getElementById('surroundPlacementRow');
    if (surroundRow) surroundRow.style.display = spatial ? 'flex' : 'none';
    updateSurroundPlacementUI();
    syncVirtualBedObjects();
    renderChannelEditor();
  }
  if (audioSampleRateInputEl && !app.audioSampleRateEditing) {
    audioSampleRateInputEl.value = String(app.audioSampleRate || 0);
    audioSampleRateInputEl.disabled = !app.oscSnapshotReady || !hasAudioDomain;
  }
  if (audioOutputSummaryEl) {
    const requestedValue = (app.audioOutputDevice || '').trim();
    const effectiveValue = (app.audioOutputDeviceEffective || requestedValue).trim();
    const deviceEntry = app.audioOutputDevices.find((entry) => entry.value === effectiveValue);
    const deviceText = effectiveValue
      ? (deviceEntry?.label || effectiveValue)
      : (app.oscSnapshotReady ? t('status.defaultOutputDevice') : '—');
    const rateText = app.audioSampleRate ? `${app.audioSampleRate} Hz` : '—';
    const fmtText = app.audioSampleFormat || '—';
    const summary = tf('audio.summary', {
      device: deviceText,
      rate: rateText,
      format: fmtText
    });
    audioOutputSummaryEl.textContent = app.audioError ? `${summary} • Error: ${app.audioError}` : summary;
  }
}

export function closeAudioSampleRateMenu() {
  const audioSampleRateMenuEl = getAudioSampleRateMenuEl();
  if (!audioSampleRateMenuEl) return;
  audioSampleRateMenuEl.style.display = 'none';
}

export function openAudioSampleRateMenu() {
  const audioSampleRateMenuEl = getAudioSampleRateMenuEl();
  const audioSampleRateInputEl = getAudioSampleRateInputEl();
  if (!audioSampleRateMenuEl) return;
  app.audioSampleRateEditing = true;
  audioSampleRateMenuEl.innerHTML = '';
  AUDIO_SAMPLE_RATE_PRESETS.forEach((rate) => {
    const item = document.createElement('button');
    item.type = 'button';
    item.style.cssText = 'display:block;width:100%;text-align:left;background:none;border:none;color:#d9ecff;padding:0.25rem 0.35rem;border-radius:6px;cursor:pointer;font-size:12px';
    item.textContent = rate === 0 ? t('status.nativeRate') : `${rate} Hz`;
    item.addEventListener('click', () => {
      if (audioSampleRateInputEl) {
        audioSampleRateInputEl.value = String(rate);
      }
      applyAudioSampleRateNow();
      closeAudioSampleRateMenu();
    });
    item.addEventListener('mouseenter', () => {
      item.style.background = 'rgba(255,255,255,0.12)';
    });
    item.addEventListener('mouseleave', () => {
      item.style.background = 'transparent';
    });
    audioSampleRateMenuEl.appendChild(item);
  });
  audioSampleRateMenuEl.style.display = 'block';
}

export function updateAudioFormatDisplay() {
  dirty.audioFormat = true;
  scheduleUIFlush();
}

export function applyAudioSampleRateNow() {
  const audioSampleRateInputEl = getAudioSampleRateInputEl();
  const requested = Math.max(0, Math.round(Number(audioSampleRateInputEl?.value) || 0));
  app.audioSampleRate = requested > 0 ? requested : null;
  updateAudioFormatDisplay();
  sendAudioConfig();
  app.audioSampleRateEditing = false;
  closeAudioSampleRateMenu();
}

export function applyAudioOutputDeviceNow() {
  const audioOutputDeviceSelectEl = getAudioOutputDeviceSelectEl();
  const requested = String(audioOutputDeviceSelectEl?.value || '').trim();
  app.audioOutputDevice = requested || null;
  updateAudioFormatDisplay();
  sendAudioConfig();
  app.audioOutputDeviceEditing = false;
}

export function applyAudioOutputBackendNow() {
  const el = inAudioPanel('audioOutputBackendSelect');
  const requested = String(el?.value || 'device').trim() === 'file' ? 'file' : 'device';
  app.audioOutputBackend = requested;
  // Explicit backend switch goes through its own control (not the batch audio
  // config), so unrelated config applies never flip the backend.
  invoke('control_audio_output_backend', { backend: requested });
  updateAudioFormatDisplay();
}

export function applyAudioOutputFileNow() {
  const el = inAudioPanel('audioOutputFileInput');
  const requested = String(el?.value ?? '').trim() || '-';
  app.audioOutputFile = requested;
  invoke('control_audio_output_file', { path: requested });
  app.audioOutputFileEditing = false;
  updateAudioFormatDisplay();
}

export function applyAudioOutputFileFormatNow() {
  const el = inAudioPanel('audioOutputFileFormatSelect');
  const requested = String(el?.value || 'raw_f32').trim();
  app.audioOutputFileFormat = requested;
  invoke('control_audio_output_file_format', { format: requested });
  updateAudioFormatDisplay();
}

export function applyRampModeNow() {
  const rampModeSelectEl = getRampModeSelectEl();
  const requested = String(rampModeSelectEl?.value || 'frame').trim().toLowerCase();
  if (!['off', 'frame', 'sample'].includes(requested)) {
    return;
  }
  app.rampMode = requested;
  updateAudioFormatDisplay();
  invoke('control_ramp_mode', { value: requested });
}

export function applyChannelRenderModeNow() {
  const el = getChannelSpatializeToggleEl();
  if (!el) return;
  const requested = el.checked ? 'spatial' : 'host';
  app.channelRenderMode = requested;
  const virtualBedActions = document.getElementById('virtualBedActions');
  if (virtualBedActions) virtualBedActions.style.display = el.checked ? 'flex' : 'none';
  const surroundRow = document.getElementById('surroundPlacementRow');
  if (surroundRow) surroundRow.style.display = el.checked ? 'flex' : 'none';
  syncVirtualBedObjects(true);
  renderChannelEditor(true);
  invoke('control_channel_render_mode', { value: requested });
}

// Reflect the active Side/Back button from `app.surroundPlacement` (called both
// on user action and on a state broadcast from the renderer).
export function updateSurroundPlacementUI() {
  const placement = app.surroundPlacement === 'back' ? 'back' : 'side';
  const sideBtn = document.getElementById('surroundPlacementSide');
  const backBtn = document.getElementById('surroundPlacementBack');
  if (sideBtn) sideBtn.classList.toggle('active', placement === 'side');
  if (backBtn) backBtn.classList.toggle('active', placement === 'back');
}

// Commit a Side/Back choice: update state + the active button and push it to the
// engine, which renders it live and persists it to config. The visible effect is
// at playback of a 4.x/5.x source (the engine streams Ls/Rs at the chosen
// corner); the at-rest editor keeps showing the full canonical 7.1 set.
export function applySurroundPlacementNow(value) {
  const requested = value === 'back' ? 'back' : 'side';
  app.surroundPlacement = requested;
  updateSurroundPlacementUI();
  invoke('control_surround_placement', { value: requested });
}
