/**
 * Object energy heatmap overlay.
 *
 * A purely client-side, object-driven complement to the renderer-computed
 * `speaker-heatmap.js`. For each live object we deposit a theoretical acoustic
 * energy gradient (inverse-square falloff from the object's current audio
 * level), accumulate every object's contribution over a grid, then reduce the
 * volume into three depth bands (front / mid / rear along the Omniphony X axis)
 * displayed as pseudo-3D planes in the scene.
 *
 * No renderer/Tauri dependency: it reads `sourcePositionsRaw` + `sourceLevels`
 * directly. Updated on a throttled tick from the animation loop, like trails.
 */

import * as THREE from 'three';

import { app, sourcePositionsRaw, sourceLevels, objectMuted } from '../state.js';
import { normalizedOmniphonyToScenePosition } from '../coordinates.js';
import { scene } from './setup.js';

// Below this RMS the object contributes nothing (mirror of trails.js).
const SILENT_RMS_DBFS = -100;
// Minimum interval between full field rebuilds (mirror of TRAIL_MIN_REBUILD_INTERVAL_MS).
const MIN_REBUILD_INTERVAL_MS = 70;
// Slicing axes. `fixed` is the axis the slices march along (Omniphony space);
// `freeA`/`freeB` are the two axes spanning each slice plane (grid col/row).
// Naming matches the mpv overlay's pseudo-3D convention (omni X = width on
// screen, omni Y = depth into the screen, omni Z = height):
//   x = "width slices"  → planes span (depth, height)   [default in studio]
//   y = "depth slices"  → planes span (width, height)
//   z = "height slices" → planes span (width, depth)
const AXES = {
  x: { fixed: 'x', freeA: 'y', freeB: 'z' },
  y: { fixed: 'y', freeA: 'x', freeB: 'z' },
  z: { fixed: 'z', freeA: 'x', freeB: 'y' },
};

// Bands span the fixed axis ∈ [-1, 1], split into `count` contiguous slices.
// Each plane is displayed at its band centre. Keys are prefixed by axis so the
// three stacks coexist in `bandMeshes`.
function computeBands(axisKey, count) {
  const n = Math.max(1, Math.min(12, Math.round(Number(count) || 3)));
  const bands = new Array(n);
  for (let i = 0; i < n; i += 1) {
    bands[i] = {
      key: `${axisKey}${i}`,
      axis: AXES[axisKey],
      range: [-1 + (2 * i) / n, -1 + (2 * (i + 1)) / n],
    };
  }
  return bands;
}

const heatmapGroup = new THREE.Group();
heatmapGroup.visible = false;
heatmapGroup.renderOrder = 22;
scene.add(heatmapGroup);

// One mesh per depth band, lazily allocated.
const bandMeshes = new Map();
// Cached geometry resolution; geometries are rebuilt when it changes.
let cachedResolution = 0;

// Reusable scratch list of active objects, refilled each tick (no per-tick alloc
// once the array has grown to its working size).
const activeObjects = [];

// Colour stops blue → cyan → green → yellow → red, interpolated in RGB.
// Adjacent stops share a channel at 1.0 so every transition stays vivid (no dark
// bands), and the stop spacing widens the warm (yellow) band, which a uniform
// hue sweep renders too narrow. Kept in sync with the mpv overlay's
// `heatmap_rgb` (orender_engine/overlay.rs).
const HEATMAP_STOPS = [
  [0.00, 0.0, 0.0, 1.0], // blue
  [0.25, 0.0, 1.0, 1.0], // cyan
  [0.48, 0.0, 1.0, 0.0], // green
  [0.70, 1.0, 1.0, 0.0], // yellow
  [1.00, 1.0, 0.0, 0.0], // red
];

function heatmapColor(value, target) {
  const t = Math.max(0, Math.min(1, Number(value) || 0));
  let i = 0;
  while (i + 1 < HEATMAP_STOPS.length && t > HEATMAP_STOPS[i + 1][0]) {
    i += 1;
  }
  const [t0, r0, g0, b0] = HEATMAP_STOPS[i];
  const [t1, r1, g1, b1] = HEATMAP_STOPS[Math.min(i + 1, HEATMAP_STOPS.length - 1)];
  const f = t1 > t0 ? (t - t0) / (t1 - t0) : 0;
  return target.setRGB(r0 + (r1 - r0) * f, g0 + (g1 - g0) * f, b0 + (b1 - b0) * f);
}

function createBandMaterial() {
  return new THREE.MeshBasicMaterial({
    vertexColors: true,
    transparent: true,
    opacity: 0.55,
    side: THREE.DoubleSide,
    depthWrite: false,
    depthTest: false,
    toneMapped: false,
    polygonOffset: true,
    polygonOffsetFactor: -1,
    polygonOffsetUnits: -1,
  });
}

function ensureBandMesh(key) {
  let mesh = bandMeshes.get(key);
  if (!mesh) {
    mesh = new THREE.Mesh(new THREE.BufferGeometry(), createBandMaterial());
    mesh.frustumCulled = false;
    mesh.renderOrder = 22;
    heatmapGroup.add(mesh);
    bandMeshes.set(key, mesh);
  }
  return mesh;
}

// dBFS RMS → linear power, with the silence floor folded in.
function objectEnergyLinear(rmsDbfs) {
  const db = Number(rmsDbfs);
  if (!Number.isFinite(db) || db <= SILENT_RMS_DBFS) {
    return 0;
  }
  return Math.pow(10, db / 10);
}

// Refill `activeObjects` with { x, y, z, energy } for every audible, non-muted
// object. Returns the count of usable entries.
function collectActiveObjects() {
  let count = 0;
  sourcePositionsRaw.forEach((raw, id) => {
    if (objectMuted.has(id)) {
      return;
    }
    const level = sourceLevels.get(id);
    const energy = objectEnergyLinear(level?.rmsDbfs);
    if (energy <= 0) {
      return;
    }
    const x = Number(raw?.x);
    const y = Number(raw?.y);
    const z = Number(raw?.z);
    if (!Number.isFinite(x) || !Number.isFinite(y) || !Number.isFinite(z)) {
      return;
    }
    let entry = activeObjects[count];
    if (!entry) {
      entry = { x: 0, y: 0, z: 0, energy: 0 };
      activeObjects[count] = entry;
    }
    entry.x = x;
    entry.y = y;
    entry.z = z;
    entry.energy = energy;
    count += 1;
  });
  return count;
}

// Build (or refresh in-place) the plane geometry for one band. Vertices sit on
// the band's centre along its fixed (slicing) axis; the two free axes span the
// plane grid. Vertex energy is accumulated from each object over `subsamples`
// slices spanning the band range along the fixed axis (the "cumulate all cells"
// reduction), using an inverse-square falloff capped by `r0sq`.
function rebuildBand(band, objectCount, resolution, subsamples, r0sq, maxEnergyRef) {
  const mesh = ensureBandMesh(band.key);
  const n = resolution;
  const vertexCount = n * n;
  const rebuildGeometry = cachedResolution !== resolution
    || !mesh.geometry.getAttribute('position')
    || mesh.geometry.getAttribute('position').count !== vertexCount;

  let positions = mesh.geometry.getAttribute('position')?.array;
  // Color is RGBA: the 4th component carries the per-vertex alpha (energy-driven),
  // which three multiplies by the material opacity (the global "Field opacity").
  let colors = mesh.geometry.getAttribute('color')?.array;
  if (rebuildGeometry || !positions || !colors) {
    positions = new Float32Array(vertexCount * 3);
    colors = new Float32Array(vertexCount * 4);
  }

  const { fixed, freeA, freeB } = band.axis;
  const [fixedMin, fixedMax] = band.range;
  const centerFixed = (fixedMin + fixedMax) * 0.5;
  // Subsample positions spanning the band along the fixed axis.
  const subStep = subsamples > 1 ? (fixedMax - fixedMin) / (subsamples - 1) : 0;

  // Scratch points (reused) — addressed by axis key so the loop body stays
  // axis-agnostic. `sample` carries the fixed-axis subsample; `omni` the centre.
  const omni = rebuildBand._omni || (rebuildBand._omni = { x: 0, y: 0, z: 0 });
  const sample = rebuildBand._sample || (rebuildBand._sample = { x: 0, y: 0, z: 0 });

  let vi = 0;
  for (let row = 0; row < n; row += 1) {
    const cellB = n > 1 ? (row / (n - 1)) * 2 - 1 : 0;
    for (let col = 0; col < n; col += 1) {
      const cellA = n > 1 ? (col / (n - 1)) * 2 - 1 : 0;

      // Vertex sits on the band centre plane for display.
      omni[fixed] = centerFixed;
      omni[freeA] = cellA;
      omni[freeB] = cellB;
      const mapped = normalizedOmniphonyToScenePosition(omni);
      positions[vi * 3] = mapped.x;
      positions[vi * 3 + 1] = mapped.y;
      positions[vi * 3 + 2] = mapped.z;

      // The free-axis components of the sample point are constant per cell.
      sample[freeA] = cellA;
      sample[freeB] = cellB;

      // Accumulate energy from every object across the band's depth subsamples.
      let energy = 0;
      for (let s = 0; s < subsamples; s += 1) {
        sample[fixed] = subsamples > 1 ? fixedMin + subStep * s : centerFixed;
        const sx = sample.x;
        const sy = sample.y;
        const sz = sample.z;
        for (let o = 0; o < objectCount; o += 1) {
          const obj = activeObjects[o];
          const dx = sx - obj.x;
          const dy = sy - obj.y;
          const dz = sz - obj.z;
          const d2 = dx * dx + dy * dy + dz * dz;
          energy += obj.energy / (d2 + r0sq);
        }
      }
      // Stash raw energy in the color buffer's R slot temporarily; normalised
      // into RGBA in a second pass once the global max is known.
      colors[vi * 4] = energy;
      if (energy > maxEnergyRef.value) {
        maxEnergyRef.value = energy;
      }
      vi += 1;
    }
  }

  if (rebuildGeometry) {
    const indices = [];
    for (let row = 0; row < n - 1; row += 1) {
      for (let col = 0; col < n - 1; col += 1) {
        const a = row * n + col;
        const b = a + 1;
        const c = a + n;
        const d = c + 1;
        indices.push(a, c, b, b, c, d);
      }
    }
    const geometry = new THREE.BufferGeometry();
    geometry.setAttribute('position', new THREE.BufferAttribute(positions, 3));
    geometry.setAttribute('color', new THREE.BufferAttribute(colors, 4));
    geometry.setIndex(indices);
    mesh.geometry.dispose();
    mesh.geometry = geometry;
  } else {
    mesh.geometry.getAttribute('position').needsUpdate = true;
  }
  mesh.visible = true;
  return { positions, colors, vertexCount, mesh };
}

// Second pass: convert the stashed raw energy (R slot) into RGBA — heatmap
// colour plus an energy-driven alpha so weak regions fade out and hot regions
// stay solid. The material opacity scales this alpha globally.
function colorizeBand(bandResult, invMax) {
  const { colors, vertexCount, mesh } = bandResult;
  const tmp = colorizeBand._color || (colorizeBand._color = new THREE.Color());
  for (let i = 0; i < vertexCount; i += 1) {
    const t = Math.max(0, Math.min(1, colors[i * 4] * invMax));
    heatmapColor(t, tmp);
    colors[i * 4] = tmp.r;
    colors[i * 4 + 1] = tmp.g;
    colors[i * 4 + 2] = tmp.b;
    colors[i * 4 + 3] = t;
  }
  mesh.geometry.getAttribute('color').needsUpdate = true;
}

function hideAll() {
  heatmapGroup.visible = false;
  for (const mesh of bandMeshes.values()) {
    mesh.visible = false;
  }
}

export function clearObjectEnergyHeatmap() {
  for (const mesh of bandMeshes.values()) {
    heatmapGroup.remove(mesh);
    mesh.geometry.dispose();
    mesh.material.dispose();
  }
  bandMeshes.clear();
  cachedResolution = 0;
  heatmapGroup.visible = false;
}

export function refreshObjectEnergyHeatmap(nowMs) {
  if (!app.objectEnergyHeatmapEnabled) {
    if (heatmapGroup.visible) {
      hideAll();
    }
    return;
  }

  const now = Number.isFinite(nowMs) ? nowMs : performance.now();
  if (now - (app.lastObjectEnergyHeatmapAt || 0) < MIN_REBUILD_INTERVAL_MS) {
    return;
  }
  app.lastObjectEnergyHeatmapAt = now;

  const objectCount = collectActiveObjects();
  if (objectCount === 0) {
    hideAll();
    return;
  }

  const resolution = Math.max(4, Math.min(96, Math.round(Number(app.objectEnergyHeatmapResolution) || 24)));
  const subsamples = Math.max(1, Math.min(16, Math.round(Number(app.objectEnergyHeatmapDepthSubsamples) || 4)));
  const r0 = Math.max(0.01, Math.min(1.0, Number(app.objectEnergyHeatmapFalloffRadius) || 0.12));
  const opacity = Math.max(0.05, Math.min(1.0, Number(app.objectEnergyHeatmapOpacity) || 0.55));
  const r0sq = r0 * r0;

  // Build the band list across every enabled slicing axis. The three stacks
  // share the band count, resolution, falloff and opacity settings.
  const bands = [];
  if (app.objectEnergyHeatmapAxisX) bands.push(...computeBands('x', app.objectEnergyHeatmapBandCount));
  if (app.objectEnergyHeatmapAxisY) bands.push(...computeBands('y', app.objectEnergyHeatmapBandCount));
  if (app.objectEnergyHeatmapAxisZ) bands.push(...computeBands('z', app.objectEnergyHeatmapBandCount));

  // Retire les meshes des bandes qui n'existent plus (axe désactivé ou nombre de
  // plans réduit).
  const liveKeys = new Set(bands.map((band) => band.key));
  for (const [key, mesh] of bandMeshes) {
    if (!liveKeys.has(key)) {
      heatmapGroup.remove(mesh);
      mesh.geometry.dispose();
      mesh.material.dispose();
      bandMeshes.delete(key);
    }
  }

  if (bands.length === 0) {
    hideAll();
    return;
  }

  const maxEnergyRef = { value: 0 };
  const bandResults = [];
  for (const band of bands) {
    bandResults.push(rebuildBand(band, objectCount, resolution, subsamples, r0sq, maxEnergyRef));
  }
  cachedResolution = resolution;

  const invMax = maxEnergyRef.value > 0 ? 1 / maxEnergyRef.value : 0;
  for (const bandResult of bandResults) {
    colorizeBand(bandResult, invMax);
    bandResult.mesh.material.opacity = opacity;
  }

  heatmapGroup.visible = true;
}
