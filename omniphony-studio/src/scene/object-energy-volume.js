/**
 * Object energy field — full-3D ray-marched volume renderer.
 *
 * The whole field is uploaded as an n³ `Data3DTexture` and a single box is
 * ray-marched in the fragment shader. With `NearestFilter` each texel reads back
 * as a crisp, illuminated cell — "every base cell of the cube lit with the colour
 * of its value". One draw call, no transparent-sort artefacts.
 *
 * The scalar field and colour ramp come from the shared core. Coordinates: the
 * energy is evaluated in Omniphony
 * normalised space (x=width, y=depth, z=height), but the texture is sampled
 * uniformly in *scene* space so the voxels fill the (depth-warped) room exactly.
 * Only the depth axis is non-linear, so the warp costs just `n` inversions, not n³.
 */

import * as THREE from 'three';

import { app } from '../state.js';
import { mapRoomDepth, inverseMapRoomDepth } from '../coordinates.js';
import { scene } from './setup.js';
import {
  MIN_REBUILD_INTERVAL_MS,
  HEATMAP_GLSL,
  activeObjects,
  collectActiveObjects,
  clampVolumeGamma,
  colormapIndex,
} from './object-energy-shared.js';

// Reference sample count the opacity is normalised against, so the perceived
// density is independent of the field resolution.
const REF_STEPS = 64;

const VERTEX_SHADER = /* glsl */`
precision highp float;
in vec3 position;
uniform mat4 modelMatrix;
uniform mat4 viewMatrix;
uniform mat4 projectionMatrix;
out vec3 vWorldPos;
void main() {
  vec4 world = modelMatrix * vec4(position, 1.0);
  vWorldPos = world.xyz;
  gl_Position = projectionMatrix * viewMatrix * world;
}
`;

const FRAGMENT_SHADER = /* glsl */`
precision highp float;
precision highp sampler3D;
in vec3 vWorldPos;
uniform vec3 cameraPosition;
uniform sampler3D uVolume;
uniform vec3 uBoxMin;
uniform vec3 uBoxMax;
uniform float uInvMax;
uniform float uOpacity;
uniform float uGammaAccumulate; // alpha γ for the accumulation component
uniform float uGammaMip;        // alpha γ for the peak (MIP) component
uniform float uStepNorm;        // REF_STEPS / uSteps: opacity independent of step count
uniform float uMix;             // 0 = pure accumulate, 1 = pure peak; blends between
uniform int uColormap;          // 0 heatmap, 1 blue→white, 2 red (alpha-only)
uniform int uSteps;
out vec4 outColor;

${HEATMAP_GLSL}

void main() {
  vec3 ro = cameraPosition;
  vec3 rd = normalize(vWorldPos - ro);
  // Slab intersection of the camera ray with the volume box.
  vec3 invd = 1.0 / rd;
  vec3 ta = (uBoxMin - ro) * invd;
  vec3 tb = (uBoxMax - ro) * invd;
  vec3 tmin = min(ta, tb);
  vec3 tmax = max(ta, tb);
  float tNear = max(max(tmin.x, tmin.y), tmin.z);
  float tFar = min(min(tmax.x, tmax.y), tmax.z);
  tNear = max(tNear, 0.0);
  if (tFar <= tNear) { discard; }

  vec3 boxSize = uBoxMax - uBoxMin;
  float dt = (tFar - tNear) / float(uSteps);
  // Both projections are computed in one pass and blended by uMix. Colour always
  // stays linear in the value (heatmap identical to the planes); only the *alpha*
  // runs through a γ curve (its own per component) so faint cells fade out.
  vec4 acc = vec4(0.0); // accumulate component: premultiplied front-to-back
  float eMax = 0.0;     // peak component: max value along the ray
  for (int s = 0; s < 512; s++) {
    if (s >= uSteps) break;
    float t = tNear + (float(s) + 0.5) * dt;
    vec3 p = ro + rd * t;
    vec3 uvw = (p - uBoxMin) / boxSize;
    float e = clamp(texture(uVolume, uvw).r * uInvMax, 0.0, 1.0);
    eMax = max(eMax, e);
    if (e > 0.004) {
      // Step-normalised so the density doesn't depend on the sample count.
      vec3 col = heatmapColor(e);
      float a = clamp(pow(e, uGammaAccumulate) * uOpacity * uStepNorm, 0.0, 1.0);
      acc.rgb += (1.0 - acc.a) * col * a;
      acc.a += (1.0 - acc.a) * a;
      // Only safe to stop early when the peak component isn't needed.
      if (uMix < 0.001 && acc.a > 0.98) break;
    }
  }
  // Peak component: alpha is purely peak × opacity (no depth accumulation),
  // matching the bitmap planes' per-vertex alpha at the brightest cell.
  float aMip = clamp(pow(eMax, uGammaMip) * uOpacity, 0.0, 1.0);
  vec4 peak = vec4(heatmapColor(eMax) * aMip, aMip); // premultiplied
  vec4 result = mix(acc, peak, uMix);
  if (result.a <= 0.0) { discard; }
  outColor = result; // rgb already premultiplied by alpha
}
`;

const volumeGroup = new THREE.Group();
volumeGroup.visible = false;
volumeGroup.renderOrder = 22;
scene.add(volumeGroup);

// A unit cube centred on the origin; the mesh is scaled/positioned each tick to
// span the room's scene-space bounding box.
const boxGeometry = new THREE.BoxGeometry(1, 1, 1);

let material = null;
let mesh = null;
let texture = null;
let data = null;
let cachedResolution = 0;
// Per-axis scratch: Omniphony coord for each texel slice (depth needs the warp
// inversion; width/height are linear). Reallocated on resolution change.
let depthByI = null; // texture dim 0 (scene x) → omni depth (omni y)
let heightByJ = null; // texture dim 1 (scene y) → omni height (omni z)
let widthByK = null; // texture dim 2 (scene z) → omni width (omni x)

function ensureMaterial() {
  if (material) return material;
  material = new THREE.RawShaderMaterial({
    glslVersion: THREE.GLSL3,
    vertexShader: VERTEX_SHADER,
    fragmentShader: FRAGMENT_SHADER,
    uniforms: {
      uVolume: { value: null },
      uBoxMin: { value: new THREE.Vector3() },
      uBoxMax: { value: new THREE.Vector3() },
      uInvMax: { value: 0 },
      uOpacity: { value: 1 },
      uGammaAccumulate: { value: 4 },
      uGammaMip: { value: 3 },
      uStepNorm: { value: 1 },
      uMix: { value: 0.6 },
      uColormap: { value: 0 },
      uSteps: { value: 96 },
    },
    transparent: true,
    premultipliedAlpha: true,
    depthWrite: false,
    depthTest: false,
    side: THREE.BackSide,
    toneMapped: false,
  });
  mesh = new THREE.Mesh(boxGeometry, material);
  mesh.frustumCulled = false;
  mesh.renderOrder = 22;
  volumeGroup.add(mesh);
  return material;
}

function ensureTexture(resolution) {
  if (texture && cachedResolution === resolution) return;
  if (texture) texture.dispose();
  const n = resolution;
  data = new Float32Array(n * n * n);
  texture = new THREE.Data3DTexture(data, n, n, n);
  texture.format = THREE.RedFormat;
  texture.type = THREE.FloatType;
  texture.minFilter = THREE.NearestFilter;
  texture.magFilter = THREE.NearestFilter;
  texture.unpackAlignment = 1;
  texture.needsUpdate = true;
  depthByI = new Float32Array(n);
  heightByJ = new Float32Array(n);
  widthByK = new Float32Array(n);
  cachedResolution = resolution;
  ensureMaterial();
  material.uniforms.uVolume.value = texture;
}

export function hideObjectEnergyVolume() {
  if (volumeGroup.visible) {
    volumeGroup.visible = false;
  }
}

export function clearObjectEnergyVolume() {
  if (texture) {
    texture.dispose();
    texture = null;
    data = null;
  }
  cachedResolution = 0;
  volumeGroup.visible = false;
}

export function refreshObjectEnergyVolume(nowMs) {
  if (!app.objectEnergyHeatmapEnabled) {
    hideObjectEnergyVolume();
    return;
  }

  const now = Number.isFinite(nowMs) ? nowMs : performance.now();
  if (now - (app.lastObjectEnergyHeatmapAt || 0) < MIN_REBUILD_INTERVAL_MS) {
    return;
  }
  app.lastObjectEnergyHeatmapAt = now;

  const objectCount = collectActiveObjects();
  if (objectCount === 0) {
    hideObjectEnergyVolume();
    return;
  }

  const resolution = Math.max(8, Math.min(64, Math.round(Number(app.objectEnergyHeatmapResolution) || 24)));
  const r0 = Math.max(0.01, Math.min(1.0, Number(app.objectEnergyHeatmapFalloffRadius) || 0.12));
  const opacity = Math.max(0.05, Math.min(1.0, Number(app.objectEnergyHeatmapOpacity) || 0.55));
  // Both components render at once and blend by the mix (0 = accumulate, 1 = peak).
  // Each carries its own γ with its own range: the falloff isn't comparable
  // otherwise (peak weighs a single sample, accumulate integrates the whole ray).
  const mix = Math.max(0, Math.min(1, Number(app.objectEnergyVolumeMix) || 0));
  const gammaAccumulate = clampVolumeGamma('accumulate', app.objectEnergyVolumeGammaAccumulate);
  const gammaMip = clampVolumeGamma('mip', app.objectEnergyVolumeGammaMip);
  const r0sq = r0 * r0;
  ensureTexture(resolution);
  const n = resolution;

  // Scene-space bounding box of the (depth-warped) room. Only X (depth) is
  // non-linear; Y (height) and Z (width) are plain ratio scalings.
  const ratio = app.roomRatio || {};
  const height = Math.max(1e-3, Number(ratio.height) || 1);
  const lower = Math.max(1e-3, Number(ratio.lower) || 0.5);
  const width = Math.max(1e-3, Number(ratio.width) || 1);
  const xMin = mapRoomDepth(-1);
  const xMax = mapRoomDepth(1);
  const yMin = -lower;
  const yMax = height;
  const zMin = -width;
  const zMax = width;

  // Per-texel Omniphony coordinate of each slice, sampled at cell centres so the
  // value matches what the NearestFilter sampler reads back for that texel.
  for (let i = 0; i < n; i += 1) {
    const sx = xMin + ((i + 0.5) / n) * (xMax - xMin);
    depthByI[i] = inverseMapRoomDepth(sx); // omni depth (obj.y axis)
  }
  for (let j = 0; j < n; j += 1) {
    const sy = yMin + ((j + 0.5) / n) * (yMax - yMin);
    heightByJ[j] = sy >= 0 ? sy / height : sy / lower; // omni height (obj.z axis)
  }
  for (let k = 0; k < n; k += 1) {
    const sz = zMin + ((k + 0.5) / n) * (zMax - zMin);
    widthByK[k] = sz / width; // omni width (obj.x axis)
  }

  // Fill the volume: energy = Σ objects e / (d² + r0²), in Omniphony space.
  // Layout matches Data3DTexture: idx = i + n*(j + n*k) with i=depth, j=height,
  // k=width — so the inner loop (i) is contiguous in memory.
  let maxEnergy = 0;
  let idx = 0;
  for (let k = 0; k < n; k += 1) {
    const ow = widthByK[k]; // omni x (width)
    for (let j = 0; j < n; j += 1) {
      const oh = heightByJ[j]; // omni z (height)
      for (let i = 0; i < n; i += 1) {
        const od = depthByI[i]; // omni y (depth)
        let energy = 0;
        for (let o = 0; o < objectCount; o += 1) {
          const obj = activeObjects[o];
          const dx = ow - obj.x;
          const dy = od - obj.y;
          const dz = oh - obj.z;
          energy += obj.energy / (dx * dx + dy * dy + dz * dz + r0sq);
        }
        data[idx] = energy;
        if (energy > maxEnergy) maxEnergy = energy;
        idx += 1;
      }
    }
  }
  texture.needsUpdate = true;

  const u = material.uniforms;
  u.uInvMax.value = maxEnergy > 0 ? 1 / maxEnergy : 0;
  u.uOpacity.value = opacity;
  u.uGammaAccumulate.value = gammaAccumulate;
  u.uGammaMip.value = gammaMip;
  u.uMix.value = mix;
  u.uColormap.value = colormapIndex(app.objectEnergyColormap);
  u.uBoxMin.value.set(xMin, yMin, zMin);
  u.uBoxMax.value.set(xMax, yMax, zMax);
  const steps = Math.max(32, Math.min(384, Math.round(n * 2)));
  u.uSteps.value = steps;
  u.uStepNorm.value = REF_STEPS / steps;

  // Position/scale the unit cube to span the room's scene-space box.
  mesh.position.set((xMin + xMax) * 0.5, (yMin + yMax) * 0.5, (zMin + zMax) * 0.5);
  mesh.scale.set(xMax - xMin, yMax - yMin, zMax - zMin);

  volumeGroup.visible = true;
}
