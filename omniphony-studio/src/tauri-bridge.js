/**
 * Tauri event bridge.
 *
 * Registers all `listen(...)` handlers that receive incremental state updates
 * from the Rust backend and apply them to the frontend state + UI.
 */

import * as THREE from 'three';
import { listen } from '@tauri-apps/api/event';

import {
  app,
  sourceMeshes,
  sourceTrails,
  speakerMuted,
  objectMuted,
  speakerManualMuted,
  objectManualMuted,
  speakerGainCache,
  speakerDelays,
  layoutsByKey,
  usesNumericSpatialPlaceholders
} from './state.js';

import { updateSource, updateSourceLevel, updateSourceGains, updateSourceBandGains, updateSourceSize, updateSourceTag, removeSource } from './sources.js';
import {
  updateSpeakerLevel,
  renderLayout,
  renderSpeakerEditor,
  hydrateLayoutSelect,
  updateSpeakerVisualsFromState,
  setSpeakerSpatializeLocal,
  updateSpeakerControlsUI,
  updateObjectControlsUI
} from './speakers.js';

import {
  setLatencyInstantMs,
  updateLatencyDisplay,
  updateLatencyMeterUI,
  updateRenderTimeUI,
  setRenderTimeMs,
  setDecodeTimeMs,
  setCrossoverTimeMs,
  setWriteTimeMs,
  setFrameDurationMs,
  updateResampleRatioDisplay
} from './controls/latency.js';
import { updateMasterGainUI, updateLoudnessDisplay, updateDistanceModelUI } from './controls/master.js';
import { updateSpreadDisplay } from './controls/spread.js';
import {
  updateRenderBackend,
  updateEvaluationMode,
  updateVbapCartesian,
  updateVbapPolar,
  updateVbapPositionInterpolation,
  renderVbapStatus
} from './controls/vbap.js';
import { updateAudioFormatDisplay } from './controls/audio.js';
import { updateInputControlUI } from './controls/input.js';
import { updateDrcMeterUI } from './controls/drc.js';
import { updateAdaptiveResamplingUI } from './controls/adaptive.js';
import { updateDistanceDiffuseUI } from './controls/distance-diffuse.js';
import { renderOscStatus, setOscStatus } from './controls/osc.js';
import { updateConfigSavedUI } from './controls/config.js';
import {
  updateRoomRatioDisplay,
  applyRoomRatio,
  refreshRoomGeometryInputState
} from './controls/room-geometry.js';
import { normalizeLogLevel, renderLogLevelControl, logState, pushLog } from './log.js';
import { applyInitState } from './init.js';
import { setInputSectionOpen } from './modals.js';
import {
  handleSpeakerHeatmapMeta,
  handleSpeakerHeatmapSlice,
  handleSpeakerHeatmapVolumeChunk,
  handleSpeakerHeatmapUnavailable,
  syncSpeakerHeatmapBandSelect,
} from './scene/speaker-heatmap.js';

export function setupTauriBridge() {
  listen('state:snapshot_ready', ({ payload }) => {
    if (payload && typeof payload === 'object') {
      applyInitState(payload);
      // Heatmap is push-based now: the renderer pushes new tiles automatically
      // to the active subscription. No need to re-request on every state echo
      // — that was the engine of the heatmap storm. Re-subscribe only on
      // explicit user action (speaker selection, heatmap toggle, etc.).
    }
  });

  // -----------------------------------------------------------------------
  // Layouts
  // -----------------------------------------------------------------------

  listen('layouts:update', ({ payload }) => {
    hydrateLayoutSelect(payload.layouts || [], payload.selectedLayoutKey);
  });

  listen('layout:selected', ({ payload }) => {
    if (payload.key && layoutsByKey.has(payload.key)) {
      const layoutSelectEl = document.getElementById('layoutSelect');
      if (layoutSelectEl) layoutSelectEl.value = payload.key;
      renderLayout(payload.key);
    }
  });

  // -----------------------------------------------------------------------
  // Sources
  // -----------------------------------------------------------------------

  listen('source:update', ({ payload }) => {
    updateSource(payload.id, payload.position);
  });

  listen('source:size', ({ payload }) => {
    updateSourceSize(payload.id, payload.size);
  });

  listen('source:remove', ({ payload }) => {
    removeSource(payload.id);
  });

  listen('source:meter', ({ payload }) => {
    updateSourceLevel(payload.id, payload.meter);
  });

  listen('source:gains', ({ payload }) => {
    updateSourceGains(payload.id, payload.gains);
  });

  listen('source:band_gains', ({ payload }) => {
    updateSourceBandGains(payload.id, payload.band, payload.gains);
  });

  listen('meter:drc_gain', ({ payload }) => {
    updateDrcMeterUI(Number(payload.value));
  });

  listen('spatial:frame', ({ payload }) => {
    const isReset = Boolean(payload?.reset);
    const objectCount = Math.max(0, Number(payload?.objectCount ?? 0) | 0);

    if (isReset) {
      for (const trail of sourceTrails.values()) {
        trail.positions.length = 0;
        trail.line.geometry.dispose();
        trail.line.geometry = new THREE.BufferGeometry();
      }
    }

    if (usesNumericSpatialPlaceholders()) {
      // Ensure IDs [0..objectCount-1] exist for renderer snapshots that use numeric IDs.
      for (let i = 0; i < objectCount; i += 1) {
        const id = String(i);
        if (!sourceMeshes.has(id)) {
          updateSource(id, { x: 0, y: 0, z: 0, name: `Object_${i}`, _noTrail: true });
        }
      }

      // Safety purge in case stale objects remain locally.
      for (const id of Array.from(sourceMeshes.keys())) {
        const idx = Number(id);
        if (Number.isInteger(idx) && idx >= objectCount) {
          removeSource(id);
        }
      }
    }
  });

  // -----------------------------------------------------------------------
  // Speakers
  // -----------------------------------------------------------------------

  listen('speaker:meter', ({ payload }) => {
    updateSpeakerLevel(Number(payload.id), payload.meter);
  });

  listen('speaker:gain', ({ payload }) => {
    speakerGainCache.set(String(payload.id), Number(payload.gain));
    updateSpeakerControlsUI();
  });

  listen('speaker:delay', ({ payload }) => {
    const id = String(payload.id);
    const delayMs = Math.max(0, Number(payload.delayMs) || 0);
    speakerDelays.set(id, delayMs);
    renderSpeakerEditor();
    updateSpeakerControlsUI();
  });

  listen('speaker:mute', ({ payload }) => {
    const key = String(payload.id);
    if (Number(payload.muted)) {
      speakerMuted.add(key);
    } else {
      speakerMuted.delete(key);
      speakerManualMuted.delete(key);
    }
    updateSpeakerControlsUI();
  });

  listen('speaker:spatialize', ({ payload }) => {
    const index = Number(payload.id);
    if (!Number.isInteger(index) || index < 0) {
      return;
    }
    const next = Number(payload.spatialize) === 0 ? 0 : 1;
    setSpeakerSpatializeLocal(index, next);
    updateSpeakerControlsUI();
  });

  listen('speaker:name', ({ payload }) => {
    const index = Number(payload.id);
    if (!Number.isInteger(index) || index < 0) {
      return;
    }
    const speaker = app.currentLayoutSpeakers[index];
    if (!speaker) {
      return;
    }
    speaker.id = String(payload.name ?? speaker.id ?? index);
    updateSpeakerVisualsFromState(index);
    updateSpeakerControlsUI();
  });

  listen('speaker:freq_low', ({ payload }) => {
    const index = Number(payload.id);
    if (!Number.isInteger(index) || index < 0) return;
    const speaker = app.currentLayoutSpeakers[index];
    if (!speaker) return;
    const fl = payload.freq_low;
    speaker.freqLow = fl != null && fl > 0 ? fl : null;
    syncSpeakerHeatmapBandSelect();
    if (app.selectedSpeakerIndex === index) renderSpeakerEditor();
  });

  listen('speaker:freq_high', ({ payload }) => {
    const index = Number(payload.id);
    if (!Number.isInteger(index) || index < 0) return;
    const speaker = app.currentLayoutSpeakers[index];
    if (!speaker) return;
    const fh = payload.freq_high;
    speaker.freqHigh = fh != null && fh > 0 ? fh : null;
    syncSpeakerHeatmapBandSelect();
    if (app.selectedSpeakerIndex === index) renderSpeakerEditor();
  });

  // -----------------------------------------------------------------------
  // Objects
  // -----------------------------------------------------------------------

  listen('object:mute', ({ payload }) => {
    const key = String(payload.id);
    if (Number(payload.muted)) {
      objectMuted.add(key);
    } else {
      objectMuted.delete(key);
      objectManualMuted.delete(key);
    }
    updateObjectControlsUI();
  });

  listen('object:source_tag', ({ payload }) => {
    updateSourceTag(payload.id, payload.sourceTag);
  });

  // -----------------------------------------------------------------------
  // OSC
  // -----------------------------------------------------------------------

  listen('osc:status', ({ payload }) => {
    const next = payload?.status;
    if (next === 'initializing' || next === 'connected' || next === 'reconnecting' || next === 'error') {
      setOscStatus(next);
    }
  });

  listen('osc:metering', ({ payload }) => {
    app.oscMeteringEnabled = Number(payload?.enabled) !== 0;
    const oscMeteringToggleEl = document.getElementById('oscMeteringToggle');
    if (oscMeteringToggleEl) oscMeteringToggleEl.checked = app.oscMeteringEnabled;
    if (!app.oscMeteringEnabled) {
      app.decodeTimeMs = null;
      app.decodeTimeWindow = [];
      app.renderTimeMs = null;
      app.renderTimeWindow = [];
      app.writeTimeMs = null;
      app.writeTimeWindow = [];
    }
    updateRenderTimeUI();
  });

  // -----------------------------------------------------------------------
  // Audio input
  // -----------------------------------------------------------------------

  listen('render:bridge_path', ({ payload }) => {
    app.renderBridgePath = String(payload?.value ?? '').trim() || null;
    updateInputControlUI();
  });

  // -----------------------------------------------------------------------
  // Room ratio
  // -----------------------------------------------------------------------

  // -----------------------------------------------------------------------
  // VBAP
  // -----------------------------------------------------------------------

  listen('vbap:recomputing', ({ payload }) => {
    app.vbapRecomputing = payload.enabled === true;
    renderVbapStatus();
  });

  // Heatmap re-requests removed: the renderer pushes new tiles to the
  // active subscription whenever the underlying state changes (and the
  // payload actually differs from the last cached one). The studio just
  // listens for incoming pushes — see `speaker_heatmap:*` handlers below.
  listen('render_evaluation:cartesian:x_size', ({ payload }) => {
    const value = Number(payload.value);
    app.vbapCartesianState.xSize = value > 0 ? value : null;
    updateVbapCartesian();
  });

  listen('render_evaluation:cartesian:y_size', ({ payload }) => {
    const value = Number(payload.value);
    app.vbapCartesianState.ySize = value > 0 ? value : null;
    updateVbapCartesian();
  });

  listen('render_evaluation:cartesian:z_size', ({ payload }) => {
    const value = Number(payload.value);
    app.vbapCartesianState.zSize = value > 0 ? value : null;
    updateVbapCartesian();
  });

  listen('render_evaluation:cartesian:z_neg_size', ({ payload }) => {
    const value = Number(payload.value);
    app.vbapCartesianState.zNegSize = value >= 0 ? value : 0;
    updateVbapCartesian();
  });

  listen('speaker_heatmap:meta', ({ payload }) => {
    handleSpeakerHeatmapMeta(payload);
  });

  listen('speaker_heatmap:slice_xy', ({ payload }) => {
    handleSpeakerHeatmapSlice('xy', payload);
  });

  listen('speaker_heatmap:slice_xz', ({ payload }) => {
    handleSpeakerHeatmapSlice('xz', payload);
  });

  listen('speaker_heatmap:slice_yz', ({ payload }) => {
    handleSpeakerHeatmapSlice('yz', payload);
  });

  listen('speaker_heatmap:volume_chunk', ({ payload }) => {
    handleSpeakerHeatmapVolumeChunk(payload);
  });

  listen('speaker_heatmap:unavailable', ({ payload }) => {
    handleSpeakerHeatmapUnavailable(payload);
  });

  listen('render_evaluation:polar:azimuth_resolution', ({ payload }) => {
    const value = Number(payload.value);
    app.vbapPolarState.azimuthResolution = value > 0 ? value : null;
    updateVbapPolar();
  });

  listen('render_evaluation:polar:elevation_resolution', ({ payload }) => {
    const value = Number(payload.value);
    app.vbapPolarState.elevationResolution = value > 0 ? value : null;
    updateVbapPolar();
  });

  listen('render_evaluation:polar:distance_res', ({ payload }) => {
    const value = Number(payload.value);
    app.vbapPolarState.distanceRes = value > 0 ? value : null;
    updateVbapPolar();
  });

  listen('render_evaluation:polar:distance_max', ({ payload }) => {
    const value = Number(payload.value);
    app.vbapPolarState.distanceMax = value > 0 ? value : null;
    updateVbapPolar();
  });

  listen('render_evaluation:position_interpolation', ({ payload }) => {
    app.vbapPositionInterpolation = payload.enabled === true;
    updateVbapPositionInterpolation();
  });

  listen('vbap:allow_negative_z', ({ payload }) => {
    app.vbapAllowNegativeZ = payload.enabled === true;
    updateVbapPolar();
  });

  // -----------------------------------------------------------------------
  // Render / decode / write timing
  // -----------------------------------------------------------------------

  listen('decode:time_ms', ({ payload }) => {
    const value = Number(payload?.value);
    if (Number.isFinite(value)) {
      setDecodeTimeMs(value);
    } else {
      app.decodeTimeMs = null;
      app.decodeTimeWindow = [];
    }
    updateRenderTimeUI();
  });

  listen('render:time_ms', ({ payload }) => {
    const value = Number(payload?.value);
    if (Number.isFinite(value)) {
      setRenderTimeMs(value);
    } else {
      app.renderTimeMs = null;
      app.renderTimeWindow = [];
    }
    updateRenderTimeUI();
  });

  listen('crossover:time_ms', ({ payload }) => {
    const value = Number(payload?.value);
    if (Number.isFinite(value)) {
      setCrossoverTimeMs(value);
    } else {
      app.crossoverTimeMs = null;
      app.crossoverTimeWindow = [];
    }
    updateRenderTimeUI();
  });

  listen('write:time_ms', ({ payload }) => {
    const value = Number(payload?.value);
    if (Number.isFinite(value)) {
      setWriteTimeMs(value);
    } else {
      app.writeTimeMs = null;
      app.writeTimeWindow = [];
    }
    updateRenderTimeUI();
  });

  listen('frame:duration_ms', ({ payload }) => {
    const value = Number(payload?.value);
    if (Number.isFinite(value)) {
      setFrameDurationMs(value);
    } else {
      app.frameDurationMs = null;
    }
    updateRenderTimeUI();
  });

  // -----------------------------------------------------------------------
  // Loudness
  // -----------------------------------------------------------------------

  // -----------------------------------------------------------------------
  // Master gain
  // -----------------------------------------------------------------------

  listen('master:gain', ({ payload }) => {
    app.masterGain = Number(payload.value);
    updateMasterGainUI();
  });

  // -----------------------------------------------------------------------
  // Distance model & diffuse
  // -----------------------------------------------------------------------

  // -----------------------------------------------------------------------
  // Adaptive resampling
  // -----------------------------------------------------------------------

  listen('adaptive_resampling:band', ({ payload }) => {
    app.adaptiveResamplingBand = typeof payload.value === 'string' ? payload.value : null;
    updateAdaptiveResamplingUI();
  });

  listen('adaptive_resampling:state', ({ payload }) => {
    app.adaptiveResamplingState = typeof payload.value === 'string' ? payload.value : null;
    updateAdaptiveResamplingUI();
  });

  listen('adaptive_resampling:pause', ({ payload }) => {
    app.adaptiveResamplingPaused = payload.enabled !== 0;
    updateAdaptiveResamplingUI();
  });

  // -----------------------------------------------------------------------
  // Config saved
  // -----------------------------------------------------------------------

  listen('config:saved', ({ payload }) => {
    app.configSaved = payload.saved !== 0;
    updateConfigSavedUI();
  });

  // -----------------------------------------------------------------------
  // Latency
  // -----------------------------------------------------------------------

  listen('latency', ({ payload }) => {
    app.latencyMs = Number(payload.value);
    updateLatencyDisplay();
    updateLatencyMeterUI();
  });

  listen('latency:instant', ({ payload }) => {
    setLatencyInstantMs(payload.value);
    updateLatencyDisplay();
    updateLatencyMeterUI();
  });

  listen('latency:control', ({ payload }) => {
    app.latencyControlMs = Number(payload.value);
    updateLatencyDisplay();
  });

  listen('latency:smoothed', ({ payload }) => {
    app.latencySmoothedMs = Number(payload.value);
    updateLatencyDisplay();
    updateLatencyMeterUI();
  });

  listen('latency:downstream', ({ payload }) => {
    app.latencyDownstreamMs = Number(payload.value);
    updateLatencyDisplay();
  });

  // Generic diag registry: schema (list of available metrics) + values map.
  // Lets the diag plot dynamically offer any metric the renderer registers,
  // with zero studio-side change per new metric.
  // The renderer side ships the schema/values as a JSON string inside the
  // OSC payload — parse it here so the plot always sees a real JS object.
  // Idempotent: if Tauri ever starts forwarding it as a structured value
  // directly, the typeof check skips the redundant parse.
  const parseDiagPayload = (payload) => {
    const raw = payload && payload.value !== undefined ? payload.value : null;
    if (typeof raw === 'string') {
      try { return JSON.parse(raw); } catch (_) { return null; }
    }
    return raw;
  };

  listen('diag:schema', ({ payload }) => {
    app.diagSchema = parseDiagPayload(payload);
  });

  listen('diag:values', ({ payload }) => {
    app.diagValues = parseDiagPayload(payload);
  });

  listen('latency:target', ({ payload }) => {
    const value = Number(payload.value);
    app.latencyTargetMs = Number.isFinite(value) ? value : null;
    updateLatencyDisplay();
    updateLatencyMeterUI();
  });

  listen('latency:requested', ({ payload }) => {
    const value = Number(payload.value);
    app.latencyRequestedMs = Number.isFinite(value) ? value : null;
    if (app.latencyTargetMs === null && Number.isFinite(value)) {
      app.latencyTargetMs = value;
    }
    if (app.latencyMs === null && Number.isFinite(value)) {
      app.latencyMs = value;
    }
    updateLatencyDisplay();
    updateLatencyMeterUI();
  });

  // -----------------------------------------------------------------------
  // Resample ratio
  // -----------------------------------------------------------------------

  listen('resample_ratio', ({ payload }) => {
    app.resampleRatio = Number(payload.value);
    updateResampleRatioDisplay();
  });

  // -----------------------------------------------------------------------
  // Audio
  // -----------------------------------------------------------------------

  // -----------------------------------------------------------------------
  // Input pipe
  // -----------------------------------------------------------------------

  listen('state:input_pipe', ({ payload }) => {
    app.orenderInputPipe = typeof payload.value === 'string' ? (payload.value.trim() || null) : null;
    renderOscStatus();
    updateInputControlUI();
  });

  // -----------------------------------------------------------------------
  // Log level
  // -----------------------------------------------------------------------

  listen('state:log_level', ({ payload }) => {
    logState.backendLogLevel = normalizeLogLevel(payload?.value);
    renderLogLevelControl();
  });

  listen('omniphony:log', ({ payload }) => {
    const level = normalizeLogLevel(payload?.level);
    const target = String(payload?.target || '').trim();
    const message = String(payload?.message || '').trim();
    if (!message) return;
    pushLog(level, message, target);
  });
}
