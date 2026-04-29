/**
 * 2D mirror of the live WebGL canvas.
 *
 * Sits between the WebGL canvas (z-index 0) and the overlay panels
 * (z-index 2) so that any `backdrop-filter: blur()` on the panels samples
 * a regular 2D canvas instead of the WebGL surface directly. The studio
 * README ("Filtres CSS au-dessus du canvas WebGL") explains why sampling
 * the live WebGL canvas under WebKitGTK aliases sprite textures with the
 * compositor's backdrop snapshot and corrupts label sprites.
 *
 * The mirror is updated synchronously after each renderer.render(). The
 * user visually sees the mirror, with at most one frame of lag — that's
 * the entire point: route the compositor's backdrop sampling through a
 * 2D canvas whose texture pool is disjoint from the WebGL context's.
 */

const MIRROR_ID = 'omniphony-canvas-mirror';

let mirrorCanvas = null;
let mirrorCtx = null;
let getRendererRef = null;

function ensureMirrorElement() {
  if (mirrorCanvas && mirrorCanvas.isConnected) {
    return mirrorCanvas;
  }
  const existing = document.getElementById(MIRROR_ID);
  if (existing instanceof HTMLCanvasElement) {
    mirrorCanvas = existing;
  } else {
    if (existing) existing.remove();
    mirrorCanvas = document.createElement('canvas');
    mirrorCanvas.id = MIRROR_ID;
    Object.assign(mirrorCanvas.style, {
      position: 'fixed',
      inset: '0',
      width: '100vw',
      height: '100vh',
      zIndex: '1',
      pointerEvents: 'none'
    });
    document.body.prepend(mirrorCanvas);
  }
  mirrorCtx = mirrorCanvas.getContext('2d');
  return mirrorCanvas;
}

function syncMirrorSize(sourceCanvas) {
  if (mirrorCanvas.width !== sourceCanvas.width) {
    mirrorCanvas.width = sourceCanvas.width;
  }
  if (mirrorCanvas.height !== sourceCanvas.height) {
    mirrorCanvas.height = sourceCanvas.height;
  }
}

export function setupCanvasMirror({ getRenderer }) {
  if (typeof getRenderer !== 'function') {
    return;
  }
  getRendererRef = getRenderer;
  ensureMirrorElement();
}

export function mirrorRenderedFrame() {
  if (!getRendererRef) {
    return;
  }
  ensureMirrorElement();
  if (!mirrorCanvas || !mirrorCtx) {
    return;
  }
  const renderer = getRendererRef();
  const sourceCanvas = renderer?.domElement;
  if (!sourceCanvas || !sourceCanvas.width || !sourceCanvas.height) {
    return;
  }
  syncMirrorSize(sourceCanvas);
  try {
    mirrorCtx.drawImage(sourceCanvas, 0, 0);
  } catch (_error) {
    // drawImage from a context-lost canvas can throw; the next frame retries.
  }
}
