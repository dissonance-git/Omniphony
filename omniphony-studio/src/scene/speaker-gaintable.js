/**
 * Shared store for the renderer's precomputed VBAP gain table.
 *
 * Transferred once over OSC (P1/P2) and decoded by the Tauri backend into a
 * regular cartesian grid over Omniphony-normalised [-1,1]³ (x=width, y=depth,
 * z=height). Holds all speakers' gains per cell, so every speaker-field display
 * can read it locally instead of issuing per-display OSC requests:
 *   gain(cell, speaker) = gains[(xi + nx*(yi + ny*zi)) * speakerCount + speaker]
 *
 * Consumers: the combined speaker energy volume (× live levels) and the
 * per-speaker solo volume (gain² of the selected speaker).
 */

// { nx, ny, nz, sc, gains: Float32Array, xPositions, yPositions, zPositions }
let table = null;

/** Store the decoded gain table pushed by the Tauri backend. Cartesian only. */
export function setSpeakerGainTable(payload) {
  if (!payload || payload.domain !== 'cartesian') {
    table = null;
    return;
  }
  const nx = Number(payload.xCount) | 0;
  const ny = Number(payload.yCount) | 0;
  const nz = Number(payload.zCount) | 0;
  const sc = Number(payload.speakerCount) | 0;
  const gains = Array.isArray(payload.gains) ? Float32Array.from(payload.gains) : null;
  if (nx < 1 || ny < 1 || nz < 1 || sc < 1 || !gains || gains.length < nx * ny * nz * sc) {
    table = null;
    return;
  }
  table = {
    nx,
    ny,
    nz,
    sc,
    gains,
    xPositions: Array.isArray(payload.xPositions) ? payload.xPositions : null,
    yPositions: Array.isArray(payload.yPositions) ? payload.yPositions : null,
    zPositions: Array.isArray(payload.zPositions) ? payload.zPositions : null,
  };
}

export function getSpeakerGainTable() {
  return table;
}

export function hasSpeakerGainTable() {
  return table !== null;
}
