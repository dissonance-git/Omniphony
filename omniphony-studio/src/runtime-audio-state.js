import { app } from './state.js';
import {
  setLatencyInstantMs,
  updateLatencyDisplay,
  updateLatencyMeterUI,
  updateRenderTimeUI,
  setRenderTimeMs,
  setDecodeTimeMs,
  setWriteTimeMs,
  setFrameDurationMs,
  updateResampleRatioDisplay
} from './controls/latency.js';
import { updateAudioFormatDisplay } from './controls/audio.js';

function normalizeAudioOutputDevices(values) {
  return Array.isArray(values)
    ? values
      .map((entry) => ({
        value: String(entry?.value || '').trim(),
        label: String(entry?.label || entry?.value || '').trim()
      }))
      .filter((entry) => entry.value.length > 0)
    : [];
}

function applyTimingValue(setter, field, windowField, value) {
  const next = Number(value);
  if (Number.isFinite(next)) {
    setter(next);
  } else {
    app[field] = null;
    if (windowField) {
      app[windowField] = [];
    }
  }
}

function applyAudioFormatValue(field, value) {
  app[field] = typeof value === 'string' ? (value.trim() || null) : null;
}

export function applyRuntimeAudioStateSnapshot(payload) {
  if (typeof payload.latencyMs === 'number') {
    app.latencyMs = payload.latencyMs;
  }
  if (typeof payload.latencyInstantMs === 'number') {
    setLatencyInstantMs(payload.latencyInstantMs);
  }
  if (typeof payload.latencyControlMs === 'number') {
    app.latencyControlMs = payload.latencyControlMs;
  }
  if (typeof payload.latencySmoothedMs === 'number') {
    app.latencySmoothedMs = payload.latencySmoothedMs;
  }
  if (typeof payload.latencyDownstreamMs === 'number') {
    app.latencyDownstreamMs = payload.latencyDownstreamMs;
  }
  if (typeof payload.latencyTargetMs === 'number') {
    app.latencyTargetMs = payload.latencyTargetMs;
  }
  if (typeof payload.latencyRequestedMs === 'number') {
    app.latencyRequestedMs = payload.latencyRequestedMs;
    if (app.latencyTargetMs === null) {
      app.latencyTargetMs = payload.latencyRequestedMs;
    }
    if (app.latencyMs === null) {
      app.latencyMs = payload.latencyRequestedMs;
    }
  }
  if (typeof payload.decodeTimeMs === 'number') {
    setDecodeTimeMs(payload.decodeTimeMs);
  }
  if (typeof payload.renderTimeMs === 'number') {
    setRenderTimeMs(payload.renderTimeMs);
  }
  if (typeof payload.writeTimeMs === 'number') {
    setWriteTimeMs(payload.writeTimeMs);
  }
  if (typeof payload.frameDurationMs === 'number') {
    setFrameDurationMs(payload.frameDurationMs);
  }
  if (typeof payload.resampleRatio === 'number') {
    app.resampleRatio = payload.resampleRatio;
  }
  if (typeof payload.audioSampleRate === 'number') {
    app.audioSampleRate = payload.audioSampleRate > 0 ? payload.audioSampleRate : null;
  }
  if (typeof payload.rampMode === 'string') {
    const next = payload.rampMode.trim().toLowerCase();
    if (next === 'off' || next === 'frame' || next === 'sample' || next === 'interp') {
      app.rampMode = next;
    }
  }
  if (typeof payload.channelRenderMode === 'string') {
    const next = payload.channelRenderMode.trim().toLowerCase();
    // `direct`/`virtual` are legacy values that now collapse to `spatial`.
    if (next === 'host') {
      app.channelRenderMode = 'host';
    } else if (next === 'spatial' || next === 'direct' || next === 'virtual') {
      app.channelRenderMode = 'spatial';
    }
  }
  if (typeof payload.surroundPlacement === 'string') {
    const next = payload.surroundPlacement.trim().toLowerCase();
    if (next === 'side' || next === 'back') {
      app.surroundPlacement = next;
    }
  }
  if (Object.prototype.hasOwnProperty.call(payload, 'virtualBed')) {
    // null = renderer is on the built-in canonical poses; an object is the
    // configured/live bed. The editor seeds defaults when this is null.
    app.virtualBed =
      payload.virtualBed && typeof payload.virtualBed === 'object' ? payload.virtualBed : null;
  }
  if (typeof payload.audioOutputDevice === 'string') {
    app.audioOutputDevice = payload.audioOutputDevice.trim() || null;
  }
  if (typeof payload.audioOutputDeviceEffective === 'string') {
    app.audioOutputDeviceEffective = payload.audioOutputDeviceEffective.trim() || null;
  }
  if (Array.isArray(payload.audioOutputDevices)) {
    app.audioOutputDevices = normalizeAudioOutputDevices(payload.audioOutputDevices);
  }
  if (typeof payload.audioOutputBackend === 'string' && payload.audioOutputBackend.trim()) {
    app.audioOutputBackend = payload.audioOutputBackend.trim() === 'file' ? 'file' : 'device';
  }
  if (typeof payload.audioOutputFile === 'string' && !app.audioOutputFileEditing) {
    app.audioOutputFile = payload.audioOutputFile.trim() || '-';
  }
  if (typeof payload.audioOutputFileFormat === 'string' && payload.audioOutputFileFormat.trim()) {
    app.audioOutputFileFormat = payload.audioOutputFileFormat.trim();
  }
  if (typeof payload.audioSampleFormat === 'string') {
    applyAudioFormatValue('audioSampleFormat', payload.audioSampleFormat);
  }
  if (typeof payload.audioError === 'string') {
    applyAudioFormatValue('audioError', payload.audioError);
  }

  updateLatencyDisplay();
  updateLatencyMeterUI();
  updateRenderTimeUI();
  updateResampleRatioDisplay();
  updateAudioFormatDisplay();
}

export function applyRuntimeAudioEvent(eventType, payload) {
  switch (eventType) {
    case 'decode:time_ms':
      applyTimingValue(setDecodeTimeMs, 'decodeTimeMs', 'decodeTimeWindow', payload?.value);
      updateRenderTimeUI();
      return;
    case 'render:time_ms':
      applyTimingValue(setRenderTimeMs, 'renderTimeMs', 'renderTimeWindow', payload?.value);
      updateRenderTimeUI();
      return;
    case 'write:time_ms':
      applyTimingValue(setWriteTimeMs, 'writeTimeMs', 'writeTimeWindow', payload?.value);
      updateRenderTimeUI();
      return;
    case 'frame:duration_ms': {
      const value = Number(payload?.value);
      if (Number.isFinite(value)) {
        setFrameDurationMs(value);
      } else {
        app.frameDurationMs = null;
      }
      updateRenderTimeUI();
      return;
    }
    case 'latency':
      app.latencyMs = Number(payload?.value);
      updateLatencyDisplay();
      updateLatencyMeterUI();
      return;
    case 'latency:instant':
      setLatencyInstantMs(payload?.value);
      updateLatencyDisplay();
      updateLatencyMeterUI();
      return;
    case 'latency:control':
      app.latencyControlMs = Number(payload?.value);
      updateLatencyDisplay();
      return;
    case 'latency:smoothed':
      app.latencySmoothedMs = Number(payload?.value);
      updateLatencyDisplay();
      updateLatencyMeterUI();
      return;
    case 'latency:downstream':
      app.latencyDownstreamMs = Number(payload?.value);
      updateLatencyDisplay();
      return;
    case 'latency:target':
      app.latencyTargetMs = Number(payload?.value);
      updateLatencyDisplay();
      updateLatencyMeterUI();
      return;
    case 'latency:requested':
      app.latencyRequestedMs = Number(payload?.value);
      if (app.latencyTargetMs === null) {
        app.latencyTargetMs = app.latencyRequestedMs;
      }
      if (app.latencyMs === null) {
        app.latencyMs = app.latencyRequestedMs;
      }
      updateLatencyDisplay();
      updateLatencyMeterUI();
      return;
    case 'resample_ratio':
      app.resampleRatio = Number(payload?.value);
      updateResampleRatioDisplay();
      return;
    case 'audio:sample_rate': {
      const value = Number(payload?.value);
      app.audioSampleRate = Number.isFinite(value) && value > 0 ? Math.round(value) : null;
      updateAudioFormatDisplay();
      return;
    }
    case 'state:ramp_mode': {
      const next = String(payload?.value || '').trim().toLowerCase();
      if (next === 'off' || next === 'frame' || next === 'sample' || next === 'interp') {
        app.rampMode = next;
        updateAudioFormatDisplay();
      }
      return;
    }
    case 'audio:output_device':
    case 'audio:output_device:requested':
      app.audioOutputDevice = typeof payload?.value === 'string' ? (payload.value.trim() || null) : null;
      updateAudioFormatDisplay();
      return;
    case 'audio:output_device:effective':
      app.audioOutputDeviceEffective = typeof payload?.value === 'string' ? (payload.value.trim() || null) : null;
      updateAudioFormatDisplay();
      return;
    case 'audio:output_devices':
      app.audioOutputDevices = normalizeAudioOutputDevices(payload?.values);
      updateAudioFormatDisplay();
      return;
    case 'audio:sample_format':
      applyAudioFormatValue('audioSampleFormat', payload?.value);
      updateAudioFormatDisplay();
      return;
    case 'audio:error':
      applyAudioFormatValue('audioError', payload?.value);
      updateAudioFormatDisplay();
      return;
    default:
  }
}
