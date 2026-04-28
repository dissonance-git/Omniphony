import { getWindowViewport, subscribeWindowViewport } from '../../core/viewport/window-viewport.js';

const STORAGE_KEY = 'spatialviz.side_panels';
const MIN_WIDTH = 220;
const DEFAULT_WIDTH = 440;

const state = {
  left: { width: DEFAULT_WIDTH, collapsed: false },
  right: { width: DEFAULT_WIDTH, collapsed: false }
};

const subscribers = new Set();
let initialized = false;

export function emitOverlayLayoutChanged(reason) {
  if (typeof window === 'undefined') {
    return;
  }
  window.dispatchEvent(new CustomEvent('omniphony:overlay-layout-changed', {
    detail: { reason }
  }));
}

function snapshot() {
  return {
    left: { ...state.left },
    right: { ...state.right }
  };
}

function notifySubscribers() {
  const nextState = snapshot();
  subscribers.forEach((listener) => {
    listener(nextState);
  });
}

function persistState() {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
  } catch (_e) {
    // ignore
  }
}

function clampWidth(width) {
  const { width: viewportWidth } = getWindowViewport();
  const maxWidth = Math.max(MIN_WIDTH, Math.floor(viewportWidth - 200));
  return Math.min(Math.max(Number(width) || DEFAULT_WIDTH, MIN_WIDTH), maxWidth);
}

function loadState() {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) {
      return;
    }
    const parsed = JSON.parse(raw);
    for (const side of ['left', 'right']) {
      const entry = parsed?.[side];
      if (!entry) {
        continue;
      }
      const nextWidth = Number(entry.width);
      if (Number.isFinite(nextWidth) && nextWidth >= MIN_WIDTH) {
        state[side].width = clampWidth(nextWidth);
      }
      state[side].collapsed = !!entry.collapsed;
    }
  } catch (_e) {
    // ignore
  }
}

function clampAllWidths() {
  state.left.width = clampWidth(state.left.width);
  state.right.width = clampWidth(state.right.width);
}

export function initOverlayLayoutState() {
  if (initialized) {
    return;
  }
  initialized = true;
  loadState();
  clampAllWidths();
  subscribeWindowViewport(() => {
    clampAllWidths();
    notifySubscribers();
  });
}

export function getOverlayLayoutState() {
  initOverlayLayoutState();
  return snapshot();
}

export function subscribeOverlayLayout(listener) {
  initOverlayLayoutState();
  subscribers.add(listener);
  listener(snapshot());
  return () => {
    subscribers.delete(listener);
  };
}

export function setOverlayPanelWidth(side, width, reason = 'side-panel-resize') {
  initOverlayLayoutState();
  if (!state[side]) {
    return;
  }
  state[side].width = clampWidth(width);
  persistState();
  notifySubscribers();
  emitOverlayLayoutChanged(reason);
}

export function setOverlayPanelCollapsed(side, collapsed, reason = 'side-panel-collapse') {
  initOverlayLayoutState();
  if (!state[side]) {
    return;
  }
  state[side].collapsed = !!collapsed;
  persistState();
  notifySubscribers();
  emitOverlayLayoutChanged(reason);
}

export function resetOverlayPanelWidth(side, reason = 'side-panel-width-reset') {
  setOverlayPanelWidth(side, DEFAULT_WIDTH, reason);
}
