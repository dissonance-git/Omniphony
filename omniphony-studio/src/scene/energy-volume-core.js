/**
 * Reusable ray-marched energy volume.
 *
 * Uploads an n³ scalar field as a `Data3DTexture` and ray-marches a single box in
 * the fragment shader (NearestFilter → crisp illuminated cells). Shared by the
 * object field (inverse-square) and the speaker field (precomputed gain table ×
 * live levels): only the per-cell energy differs, supplied as a `sampleEnergy`
 * callback. The texture is sampled uniformly in *scene* space so voxels fill the
 * (depth-warped) room; only the depth axis is non-linear, so the warp costs `n`
 * inversions, not n³.
 */

import * as THREE from 'three';

import { app } from '../state.js';
import { mapRoomDepth, inverseMapRoomDepth } from '../coordinates.js';
import { scene, renderer } from './setup.js';
import { HEATMAP_GLSL } from './object-energy-shared.js';

// Trilinear (LinearFilter) sampling of a float 3D texture needs this WebGL2
// extension; without it linear float sampling returns 0 (black). Checked lazily
// so the "smooth" toggle silently falls back to crisp cells where unsupported.
let floatLinearSupport = null;
function supportsFloatLinear() {
  if (floatLinearSupport === null) {
    floatLinearSupport = !!(renderer?.extensions?.get('OES_texture_float_linear'));
  }
  return floatLinearSupport;
}

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
uniform float uGammaAccumulate;
uniform float uGammaMip;
uniform float uStepNorm;
uniform float uMix;
uniform int uColormap;
uniform int uSteps;
out vec4 outColor;

${HEATMAP_GLSL}

void main() {
  vec3 ro = cameraPosition;
  vec3 rd = normalize(vWorldPos - ro);
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
  vec4 acc = vec4(0.0);
  float eMax = 0.0;
  for (int s = 0; s < 512; s++) {
    if (s >= uSteps) break;
    float t = tNear + (float(s) + 0.5) * dt;
    vec3 p = ro + rd * t;
    vec3 uvw = (p - uBoxMin) / boxSize;
    float e = clamp(texture(uVolume, uvw).r * uInvMax, 0.0, 1.0);
    eMax = max(eMax, e);
    if (e > 0.004) {
      vec3 col = heatmapColor(e);
      float a = clamp(pow(e, uGammaAccumulate) * uOpacity * uStepNorm, 0.0, 1.0);
      acc.rgb += (1.0 - acc.a) * col * a;
      acc.a += (1.0 - acc.a) * a;
      if (uMix < 0.001 && acc.a > 0.98) break;
    }
  }
  float aMip = clamp(pow(eMax, uGammaMip) * uOpacity, 0.0, 1.0);
  vec4 peak = vec4(heatmapColor(eMax) * aMip, aMip);
  vec4 result = mix(acc, peak, uMix);
  if (result.a <= 0.0) { discard; }
  outColor = result;
}
`;

export class EnergyVolume {
  constructor() {
    this.group = new THREE.Group();
    this.group.visible = false;
    this.group.renderOrder = 22;
    scene.add(this.group);
    this.boxGeometry = new THREE.BoxGeometry(1, 1, 1);
    this.material = null;
    this.mesh = null;
    this.texture = null;
    this.data = null;
    this.cachedResolution = 0;
    this.depthByI = null;
    this.heightByJ = null;
    this.widthByK = null;
  }

  ensureMaterial() {
    if (this.material) return;
    this.material = new THREE.RawShaderMaterial({
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
    this.mesh = new THREE.Mesh(this.boxGeometry, this.material);
    this.mesh.frustumCulled = false;
    this.mesh.renderOrder = 22;
    this.group.add(this.mesh);
  }

  ensureTexture(resolution) {
    if (this.texture && this.cachedResolution === resolution) return;
    if (this.texture) this.texture.dispose();
    const n = resolution;
    this.data = new Float32Array(n * n * n);
    this.texture = new THREE.Data3DTexture(this.data, n, n, n);
    this.texture.format = THREE.RedFormat;
    this.texture.type = THREE.FloatType;
    this.texture.minFilter = THREE.NearestFilter;
    this.texture.magFilter = THREE.NearestFilter;
    this.texture.unpackAlignment = 1;
    this.texture.needsUpdate = true;
    this.depthByI = new Float32Array(n);
    this.heightByJ = new Float32Array(n);
    this.widthByK = new Float32Array(n);
    this.cachedResolution = resolution;
    this.ensureMaterial();
    this.material.uniforms.uVolume.value = this.texture;
  }

  hide() {
    if (this.group.visible) this.group.visible = false;
  }

  dispose() {
    if (this.texture) {
      this.texture.dispose();
      this.texture = null;
      this.data = null;
    }
    this.cachedResolution = 0;
    this.group.visible = false;
  }

  /**
   * Rebuild the volume for this tick. `sampleEnergy(omniW, omniDepth, omniHeight)`
   * returns the scalar energy at an Omniphony-normalised position (x=width,
   * y=depth, z=height), each in [-1, 1].
   */
  update({ resolution, opacity, mix, gammaAccumulate, gammaMip, colormap, smooth, sampleEnergy }) {
    const n = Math.max(8, Math.min(64, Math.round(Number(resolution) || 24)));
    this.ensureTexture(n);

    // Crisp cells (NearestFilter) vs trilinear gradient between the 8 corner
    // texels of each cell (LinearFilter), where the float-linear extension allows.
    const filter = smooth && supportsFloatLinear() ? THREE.LinearFilter : THREE.NearestFilter;
    if (this.texture.minFilter !== filter) {
      this.texture.minFilter = filter;
      this.texture.magFilter = filter;
      this.texture.needsUpdate = true;
    }

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

    // Per-texel Omniphony coordinate of each slice, at cell centres.
    for (let i = 0; i < n; i += 1) {
      const sx = xMin + ((i + 0.5) / n) * (xMax - xMin);
      this.depthByI[i] = inverseMapRoomDepth(sx);
    }
    for (let j = 0; j < n; j += 1) {
      const sy = yMin + ((j + 0.5) / n) * (yMax - yMin);
      this.heightByJ[j] = sy >= 0 ? sy / height : sy / lower;
    }
    for (let k = 0; k < n; k += 1) {
      const sz = zMin + ((k + 0.5) / n) * (zMax - zMin);
      this.widthByK[k] = sz / width;
    }

    // idx = i + n*(j + n*k), i=depth, j=height, k=width (contiguous inner loop).
    const data = this.data;
    let maxEnergy = 0;
    let idx = 0;
    for (let k = 0; k < n; k += 1) {
      const ow = this.widthByK[k];
      for (let j = 0; j < n; j += 1) {
        const oh = this.heightByJ[j];
        for (let i = 0; i < n; i += 1) {
          const od = this.depthByI[i];
          const energy = sampleEnergy(ow, od, oh);
          data[idx] = energy;
          if (energy > maxEnergy) maxEnergy = energy;
          idx += 1;
        }
      }
    }
    this.texture.needsUpdate = true;

    const u = this.material.uniforms;
    u.uInvMax.value = maxEnergy > 0 ? 1 / maxEnergy : 0;
    u.uOpacity.value = Math.max(0.05, Math.min(1.0, Number(opacity) || 0.55));
    u.uGammaAccumulate.value = gammaAccumulate;
    u.uGammaMip.value = gammaMip;
    u.uMix.value = Math.max(0, Math.min(1, Number(mix) || 0));
    u.uColormap.value = colormap | 0;
    u.uBoxMin.value.set(xMin, yMin, zMin);
    u.uBoxMax.value.set(xMax, yMax, zMax);
    const steps = Math.max(32, Math.min(384, Math.round(n * 2)));
    u.uSteps.value = steps;
    u.uStepNorm.value = REF_STEPS / steps;

    this.mesh.position.set((xMin + xMax) * 0.5, (yMin + yMax) * 0.5, (zMin + zMax) * 0.5);
    this.mesh.scale.set(xMax - xMin, yMax - yMin, zMax - zMin);
    this.group.visible = true;
  }
}
