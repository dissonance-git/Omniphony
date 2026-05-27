/**
 * Config saved indicator.
 */

import { app, dirty } from '../state.js';
import { scheduleUIFlush, flushCallbacks } from '../flush.js';
import { inAudioPanel, inSaveFooter } from '../ui/panel-roots.js';

function getConfigSavedIndicatorEl() { return inAudioPanel('configSavedIndicator'); }
function getSaveConfigBtnEl() { return inSaveFooter('saveConfigBtn'); }
function getReloadConfigBtnEl() { return inSaveFooter('reloadConfigBtn'); }

export function renderConfigSavedUI() {
  const configSavedIndicatorEl = getConfigSavedIndicatorEl();
  const saveConfigBtnEl = getSaveConfigBtnEl();
  const reloadConfigBtnEl = getReloadConfigBtnEl();
  const runtimeConnected = app.oscSnapshotReady === true;
  if (configSavedIndicatorEl) {
    if (typeof app.saveError === 'string' && app.saveError.length > 0) {
      configSavedIndicatorEl.textContent = app.saveError;
      configSavedIndicatorEl.title = app.saveError;
      configSavedIndicatorEl.style.color = '#ff7676';
    } else {
      configSavedIndicatorEl.textContent = '';
      configSavedIndicatorEl.removeAttribute('title');
      configSavedIndicatorEl.style.color = '';
    }
  }
  if (saveConfigBtnEl) {
    const alreadySaved = app.configSaved === true;
    const pending = app.saveRequested === true;
    const enabled = runtimeConnected && !alreadySaved && !pending;
    saveConfigBtnEl.disabled = !enabled;
    saveConfigBtnEl.style.opacity = enabled ? '1' : '0.5';
    saveConfigBtnEl.style.cursor = enabled ? 'pointer' : 'default';
  }
  if (reloadConfigBtnEl) {
    reloadConfigBtnEl.disabled = !runtimeConnected;
    reloadConfigBtnEl.style.opacity = runtimeConnected ? '1' : '0.5';
    reloadConfigBtnEl.style.cursor = runtimeConnected ? 'pointer' : 'default';
  }
}

export function updateConfigSavedUI() {
  dirty.configSaved = true;
  scheduleUIFlush();
}

// Wire render function into the flush callback registry.
flushCallbacks.renderConfigSavedUI = renderConfigSavedUI;
