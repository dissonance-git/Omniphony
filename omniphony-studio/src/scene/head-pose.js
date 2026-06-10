// Live head-tracking pose applied to the centre head model (the Dame de
// Brassempouy). The renderer broadcasts `binaural.headPose` (~10 Hz), the
// world→head quaternion it actually applies to object positions. When the
// binaural output stage is active we show the *head orientation in world* —
// the conjugate — on the statue; otherwise the head eases back to neutral.
//
// The 10 Hz updates are smoothed by slerping toward the target every frame,
// so the motion looks continuous in the 3D view.

import * as THREE from 'three';
import { headPoseGroup } from './setup.js';

// Omniphony axes: x = right, y = front, z = up.
// Scene axes (see coordinates.js): scene.x = omni.y, scene.y = omni.z,
// scene.z = omni.x — a cyclic permutation (det +1), so a quaternion maps by
// permuting its vector part the same way.
const target = new THREE.Quaternion();
const identity = new THREE.Quaternion();

/// Fraction of the remaining arc covered per rendered frame (~60 fps →
/// ≈80 ms time constant, comfortably bridging the 10 Hz broadcast).
const SLERP_PER_FRAME = 0.18;

/**
 * Update the target orientation from the renderer's `binaural` state object.
 * Only an actually-applied pose rotates the head: outside binaural output
 * mode the target is neutral, whatever the tracker keeps sending.
 */
export function setHeadPoseTarget(binaural) {
  if (!binaural || typeof binaural !== 'object') return;
  const pose = binaural.headPose;
  const active = binaural.outputMode === 'binaural'
    && pose && typeof pose.w === 'number'
    && [pose.x, pose.y, pose.z].every((v) => typeof v === 'number');
  if (!active) {
    target.copy(identity);
    return;
  }
  // Conjugate (head-in-world), then permute omni (x, y, z) → scene (y, z, x).
  target.set(-pose.y, -pose.z, -pose.x, pose.w).normalize();
}

/** Per-frame tick from the animation loop: ease the statue toward the target. */
export function updateHeadPose() {
  if (!headPoseGroup) return;
  headPoseGroup.quaternion.slerp(target, SLERP_PER_FRAME);
}
