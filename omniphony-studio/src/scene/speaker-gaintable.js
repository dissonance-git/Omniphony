/**
 * Shared store + subscription for the renderer's per-band speaker gain table.
 *
 * The heatmap only shows ONE speaker, so we subscribe per speaker: the renderer
 * keeps the full table cached and ships just the selected speaker's per-band
 * field (one value per cell per crossover band). It arrives as a base64 binary
 * blob (`dataB64`) which we slice into Float32Array views — no per-number JSON
 * parsing. Each speaker's decoded table is kept in a per-speaker cache:
 *   { nx, ny, nz, speakerIndex, bands: [{ lowHz, highHz|null, gains: Float32Array }],
 *     xPositions, yPositions, zPositions }
 *   gain(cell, band) = bands[band].gains[xi + nx*(yi + ny*zi)]
 *
 * Because we cache per speaker (and remember each speaker's last version), going
 * back to an already-loaded speaker is instant: the render reads the cached table
 * immediately and the subscribe carries that speaker's version, so the renderer
 * replies `uptodate` instead of re-transferring. A fresh speaker (version 0) is
 * fetched once. Topology rebuilds bump the version, so a stale cache is refreshed.
 *
 * Pub/sub: displays call `acquireGainTable(id)`; the first subscribes (carrying
 * the selected speaker + that speaker's cached version), arms a 5 s heartbeat, and
 * re-subscribes on speaker change (`refreshGaintableSubscription`). Last release →
 * unsubscribe (the per-speaker cache is kept).
 */

import { invoke } from '@tauri-apps/api/core';

import { app } from '../state.js';

// speaker index → decoded table; speaker index → last version we hold for it.
const tables = new Map();
const versions = new Map();

const consumers = new Set();
let heartbeatTimer = null;
const HEARTBEAT_MS = 5000;

function currentSpeaker() {
  const s = app.selectedSpeakerIndex;
  return Number.isInteger(s) && s >= 0 ? s : 0;
}

function sendSubscribe() {
  const speaker = currentSpeaker();
  invoke('subscribe_speaker_gaintable', {
    haveVersion: versions.get(speaker) | 0,
    speakerIndex: speaker,
  }).catch(() => {});
}

function startHeartbeat() {
  if (heartbeatTimer !== null) return;
  heartbeatTimer = setInterval(() => {
    if (consumers.size > 0) sendSubscribe();
  }, HEARTBEAT_MS);
}

function stopHeartbeat() {
  if (heartbeatTimer === null) return;
  clearInterval(heartbeatTimer);
  heartbeatTimer = null;
}

/** Register a consumer that needs the gain table. First one subscribes. */
export function acquireGainTable(id) {
  const wasEmpty = consumers.size === 0;
  consumers.add(id);
  if (wasEmpty) {
    sendSubscribe();
    startHeartbeat();
  }
}

/** Drop a consumer. When none remain, unsubscribe (the cache is kept). */
export function releaseGainTable(id) {
  if (!consumers.delete(id)) return;
  if (consumers.size === 0) {
    stopHeartbeat();
    invoke('unsubscribe_speaker_gaintable').catch(() => {});
  }
}

/** Re-subscribe immediately for the (possibly changed) selected speaker. */
export function refreshGaintableSubscription() {
  if (consumers.size > 0) sendSubscribe();
}

function base64ToArrayBuffer(b64) {
  const bin = atob(b64);
  const len = bin.length;
  const bytes = new Uint8Array(len);
  for (let i = 0; i < len; i += 1) bytes[i] = bin.charCodeAt(i);
  return bytes.buffer;
}

/** Store the per-speaker band table decoded from the binary payload. */
export function setSpeakerGainTable(payload) {
  if (!payload || payload.domain !== 'cartesian_bands' || typeof payload.dataB64 !== 'string') {
    return;
  }
  const nx = Number(payload.xCount) | 0;
  const ny = Number(payload.yCount) | 0;
  const nz = Number(payload.zCount) | 0;
  const nb = Number(payload.bandCount) | 0;
  const speakerIndex = Number(payload.speakerIndex) | 0;
  const bandMeta = Array.isArray(payload.bands) ? payload.bands : [];
  if (nx < 1 || ny < 1 || nz < 1 || nb < 1) {
    return;
  }
  let buffer;
  try {
    buffer = base64ToArrayBuffer(payload.dataB64);
  } catch {
    return;
  }
  const cells = nx * ny * nz;
  const need = (nx + ny + nz + nb * cells) * 4;
  if (buffer.byteLength < need) {
    return;
  }
  let off = 0;
  const xPositions = new Float32Array(buffer, off, nx); off += nx * 4;
  const yPositions = new Float32Array(buffer, off, ny); off += ny * 4;
  const zPositions = new Float32Array(buffer, off, nz); off += nz * 4;
  const bands = [];
  for (let b = 0; b < nb; b += 1) {
    const gains = new Float32Array(buffer, off, cells); off += cells * 4;
    bands.push({
      lowHz: Number(bandMeta[b]?.lowHz) || 0,
      highHz: bandMeta[b]?.highHz == null ? null : Number(bandMeta[b].highHz),
      gains,
    });
  }
  tables.set(speakerIndex, { nx, ny, nz, speakerIndex, bands, xPositions, yPositions, zPositions });
  if (Number.isFinite(Number(payload.version))) {
    versions.set(speakerIndex, Number(payload.version) | 0);
  }
}

export function getSpeakerGainTable() {
  return tables.get(currentSpeaker()) ?? null;
}
