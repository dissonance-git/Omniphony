import { getWindowViewport, subscribeWindowViewport } from '../viewport/window-viewport.js';

let getRendererRef = null;
let getCameraRef = null;
let unsubscribeViewport = null;

function currentRenderer() {
  return typeof getRendererRef === 'function' ? getRendererRef() : null;
}

function currentCamera() {
  return typeof getCameraRef === 'function' ? getCameraRef() : null;
}

function applyViewport(viewport) {
  const renderer = currentRenderer();
  const camera = currentCamera();
  if (!renderer || !camera) {
    return;
  }
  camera.aspect = viewport.width / viewport.height;
  camera.updateProjectionMatrix();
  renderer.setPixelRatio(viewport.dpr);
  renderer.setSize(viewport.width, viewport.height, false);
}

export function syncRenderSurface() {
  applyViewport(getWindowViewport());
}

export function getRenderSurfaceRect() {
  const renderer = currentRenderer();
  return renderer?.domElement?.getBoundingClientRect?.() ?? null;
}

export function initRenderSurfaceController({ getRenderer, getCamera }) {
  getRendererRef = getRenderer;
  getCameraRef = getCamera;
  if (unsubscribeViewport) {
    unsubscribeViewport();
  }
  unsubscribeViewport = subscribeWindowViewport((viewport) => {
    applyViewport(viewport);
  });
  syncRenderSurface();
}
