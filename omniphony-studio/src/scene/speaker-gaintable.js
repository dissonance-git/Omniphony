/**
 * Shared store + subscription for the renderer's precomputed VBAP gain table.
 *
 * Transferred over OSC (chunked) and decoded by the Tauri backend into a regular
 * cartesian grid over Omniphony-normalised [-1,1]³ (x=width, y=depth, z=height).
 * Holds all speakers' gains per cell so every speaker-field display reads it
 * locally instead of issuing per-display requests:
 *   gain(cell, speaker) = gains[(xi + nx*(yi + ny*zi)) * speakerCount + speaker]
 *
 * Pub/sub model: each display that needs the table calls `acquireGainTable(id)`;
 * the first acquisition subscribes (carrying the version we already cached, so the
 * renderer skips the resend if we're current) and arms a 5 s heartbeat that
 * re-asserts the subscription (self-heals a lost push + keeps the client live on
 * the renderer's TTL). When the last consumer releases, we unsubscribe but keep
 * the cached table + version. While subscribed, the renderer pushes a fresh table
 * on every topology rebuild automatically.
 */

import { invoke } from '@tauri-apps/api/core';

// { nx, ny, nz, sc, bands: [{ lowHz, highHz|null, gains: Float32Array }],
//   xPositions, yPositions, zPositions }. One band per crossover band (a single
// all-speaker band when the layout has no crossover).
let table = null;
// Version (hash) of the cached table, echoed to the renderer on (re)subscribe so
// it only re-pushes when its current version differs. 0 = nothing cached yet.
let version = 0;

const consumers = new Set();
let heartbeatTimer = null;
const HEARTBEAT_MS = 5000;

function sendSubscribe() {
  invoke('subscribe_speaker_gaintable', { haveVersion: version | 0 }).catch(() => {});
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

/** Store a table pushed by the Tauri backend. Caches the version unconditionally
 *  (so a non-cartesian push doesn't trigger an endless re-push), but only keeps a
 *  usable grid for the cartesian domain the displays consume. */
export function setSpeakerGainTable(payload) {
  if (payload && Number.isFinite(Number(payload.version))) {
    version = Number(payload.version) | 0;
  }
  if (!payload || payload.domain !== 'cartesian_bands') {
    table = null;
    return;
  }
  const nx = Number(payload.xCount) | 0;
  const ny = Number(payload.yCount) | 0;
  const nz = Number(payload.zCount) | 0;
  const sc = Number(payload.speakerCount) | 0;
  const rawBands = Array.isArray(payload.bandGains) ? payload.bandGains : null;
  const bandMeta = Array.isArray(payload.bands) ? payload.bands : [];
  if (nx < 1 || ny < 1 || nz < 1 || sc < 1 || !rawBands || rawBands.length < 1) {
    table = null;
    return;
  }
  const cellSc = nx * ny * nz * sc;
  const bands = [];
  for (let b = 0; b < rawBands.length; b += 1) {
    const g = Array.isArray(rawBands[b]) ? Float32Array.from(rawBands[b]) : null;
    if (!g || g.length < cellSc) {
      table = null;
      return;
    }
    bands.push({
      lowHz: Number(bandMeta[b]?.lowHz) || 0,
      highHz: bandMeta[b]?.highHz == null ? null : Number(bandMeta[b].highHz),
      gains: g,
    });
  }
  table = {
    nx,
    ny,
    nz,
    sc,
    bands,
    xPositions: Array.isArray(payload.xPositions) ? payload.xPositions : null,
    yPositions: Array.isArray(payload.yPositions) ? payload.yPositions : null,
    zPositions: Array.isArray(payload.zPositions) ? payload.zPositions : null,
  };
}

export function getSpeakerGainTable() {
  return table;
}
