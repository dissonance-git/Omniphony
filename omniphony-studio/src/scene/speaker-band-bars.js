import * as THREE from 'three';

// Per-speaker "frequency extent" gauge shown in the 3D scene: a small vertical
// bar on a log-frequency axis (20 Hz at the bottom → 20 kHz at the top) whose
// lit segment is the speaker's pass-band. It lets you recognise a speaker's
// crossover role at a glance — low-pass (sub) fills the bottom, high-pass
// (tweeter) the top, band-pass (mid) a floating middle segment, and full-band
// the whole bar. Drawn as a billboard sprite so it always reads upright,
// regardless of camera orbit, and redrawn only when the cutoffs change.

const FMIN = 20;
const FMAX = 20000;
const LOG_MIN = Math.log(FMIN);
const LOG_SPAN = Math.log(FMAX) - LOG_MIN;

// Canvas aspect (W/H) matches the sprite scale aspect so the bar isn't stretched.
const CANVAS_W = 64;
const CANVAS_H = 256;
const SCALE_X = 0.055;
const SCALE_Y = 0.22;

// Reference gridlines (Hz) drawn faintly across the track.
const TICKS_HZ = [100, 1000, 10000];

const ACCENT = '#8ec8ff'; // matches the speaker base colour

function logPos(freq) {
  const f = Math.min(FMAX, Math.max(FMIN, Number(freq) || FMIN));
  return (Math.log(f) - LOG_MIN) / LOG_SPAN; // 0 at FMIN (bottom), 1 at FMAX (top)
}

function roundRect(ctx, x, y, w, h, r) {
  const radius = Math.min(r, w / 2, h / 2);
  ctx.beginPath();
  ctx.moveTo(x + radius, y);
  ctx.arcTo(x + w, y, x + w, y + h, radius);
  ctx.arcTo(x + w, y + h, x, y + h, radius);
  ctx.arcTo(x, y + h, x, y, radius);
  ctx.arcTo(x, y, x + w, y, radius);
  ctx.closePath();
}

function bandEdges(speaker) {
  const low = Number(speaker?.freqLow);
  const high = Number(speaker?.freqHigh);
  const hasLow = Number.isFinite(low) && low > 0;
  const hasHigh = Number.isFinite(high) && high > 0;
  // Pass-band lower edge = freqLow (high-pass) or the axis floor; upper edge =
  // freqHigh (low-pass) or the axis ceiling. A single contiguous segment.
  return {
    lo: hasLow ? low : FMIN,
    hi: hasHigh ? high : FMAX,
    key: `${hasLow ? low : 0}|${hasHigh ? high : 0}`
  };
}

function drawBar(ctx, speaker) {
  const W = CANVAS_W;
  const H = CANVAS_H;
  ctx.clearRect(0, 0, W, H);

  const padY = 10;
  const trackW = 22;
  const trackX = (W - trackW) / 2;
  const trackTop = padY;
  const trackH = H - padY * 2;
  const yFor = (freq) => trackTop + (1 - logPos(freq)) * trackH;

  // Track background + border.
  ctx.fillStyle = 'rgba(20, 28, 38, 0.78)';
  roundRect(ctx, trackX, trackTop, trackW, trackH, 8);
  ctx.fill();
  ctx.lineWidth = 2;
  ctx.strokeStyle = 'rgba(255, 255, 255, 0.28)';
  ctx.stroke();

  // Faint reference gridlines at the decade frequencies.
  ctx.lineWidth = 1;
  ctx.strokeStyle = 'rgba(255, 255, 255, 0.14)';
  TICKS_HZ.forEach((hz) => {
    const y = yFor(hz);
    ctx.beginPath();
    ctx.moveTo(trackX + 3, y);
    ctx.lineTo(trackX + trackW - 3, y);
    ctx.stroke();
  });

  // Lit pass-band segment (clipped to the rounded track).
  const { lo, hi } = bandEdges(speaker);
  const yHi = yFor(hi); // smaller y (top)
  const yLo = yFor(lo); // larger y (bottom)
  ctx.save();
  roundRect(ctx, trackX, trackTop, trackW, trackH, 8);
  ctx.clip();
  const grad = ctx.createLinearGradient(0, yHi, 0, yLo);
  grad.addColorStop(0, ACCENT);
  grad.addColorStop(1, '#5b86c8');
  ctx.fillStyle = grad;
  ctx.fillRect(trackX + 2, yHi, trackW - 4, Math.max(2, yLo - yHi));
  ctx.restore();
}

export function createSpeakerBandBar() {
  const canvas = document.createElement('canvas');
  canvas.width = CANVAS_W;
  canvas.height = CANVAS_H;
  const ctx = canvas.getContext('2d');
  const texture = new THREE.CanvasTexture(canvas);
  texture.minFilter = THREE.LinearFilter;
  texture.magFilter = THREE.LinearFilter;
  texture.generateMipmaps = false;
  texture.colorSpace = THREE.SRGBColorSpace;
  const material = new THREE.SpriteMaterial({
    map: texture,
    transparent: true,
    depthTest: false,
    depthWrite: false,
    toneMapped: false
  });
  const sprite = new THREE.Sprite(material);
  sprite.scale.set(SCALE_X, SCALE_Y, 1);
  sprite.frustumCulled = false;
  sprite.renderOrder = 39;
  sprite.userData.bandCanvas = canvas;
  sprite.userData.bandCtx = ctx;
  sprite.userData.bandTexture = texture;
  sprite.userData.bandKey = null;
  return sprite;
}

// Redraw the gauge only when the speaker's cutoffs actually change.
export function updateSpeakerBandBar(sprite, speaker) {
  if (!sprite?.userData?.bandCtx) return;
  const { key } = bandEdges(speaker);
  if (sprite.userData.bandKey === key) return;
  sprite.userData.bandKey = key;
  drawBar(sprite.userData.bandCtx, speaker);
  sprite.userData.bandTexture.needsUpdate = true;
}

export function disposeSpeakerBandBar(sprite) {
  if (!sprite?.userData) return;
  sprite.userData.bandTexture?.dispose();
  sprite.userData.bandTexture = null;
  sprite.userData.bandCanvas = null;
  sprite.userData.bandCtx = null;
  sprite.material?.dispose();
}
