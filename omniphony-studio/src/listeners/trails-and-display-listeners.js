import { app, sourceMeshes, sourceTrails, speakerLabels, speakerLevels, speakerMeshes } from '../state.js';
import { setLocale } from '../i18n.js';
import { persistTrailPrefs, persistEffectiveRenderPrefs, refreshEffectiveRenderVisibility } from '../controls/room-geometry.js';
import { rebuildTrailGeometry, createTrailRenderable } from '../trails.js';
import { scene } from '../scene/setup.js';
import { applySpeakerLevel, updateSourceDecorations, updateSourceSelectionStyles } from '../sources.js';
import { refreshOverlayLists, updateSpeakerVisualsFromState } from '../speakers.js';
import { subscribeSpeakerHeatmap, syncSpeakerHeatmapBandSelect } from '../scene/speaker-heatmap.js';
import {
  getMpvOverlayStatus,
  setMpvOverlayEnabled,
  setMpvOverlaySocketPath,
  pushMpvOverlayTrailPrefs
} from '../mpvOverlay.js';

export function setupTrailsAndDisplayListeners() {
  const trailToggleEl = document.getElementById('trailToggle');
  const effectiveRenderToggleEl = document.getElementById('effectiveRenderToggle');
  const objectColorsToggleEl = document.getElementById('objectColorsToggle');
  const objectDisplayModeSelectEl = document.getElementById('objectDisplayModeSelect');
  const objectSphereSizeSliderEl = document.getElementById('objectSphereSizeSlider');
  const objectSphereSizeValEl = document.getElementById('objectSphereSizeVal');
  const objectLabelsToggleEl = document.getElementById('objectLabelsToggle');
  const showObjectDetailsToggleEl = document.getElementById('showObjectDetailsToggle');
  const objectDetailsToggleBtnEl = document.getElementById('objectDetailsToggleBtn');
  const speakerLabelsToggleEl = document.getElementById('speakerLabelsToggle');
  const speakerSizeSliderEl = document.getElementById('speakerSizeSlider');
  const speakerSizeValEl = document.getElementById('speakerSizeVal');
  const trailModeSelectEl = document.getElementById('trailModeSelect');
  const trailTtlSliderEl = document.getElementById('trailTtlSlider');
  const trailTtlValEl = document.getElementById('trailTtlVal');
  const trailTeleportSliderEl = document.getElementById('trailTeleportSlider');
  const trailTeleportValEl = document.getElementById('trailTeleportVal');
  const localeSelectEl = document.getElementById('localeSelect');
  const speakerHeatmapSlicesToggleEl = document.getElementById('speakerHeatmapSlicesToggle');
  const speakerHeatmapVolumeToggleEl = document.getElementById('speakerHeatmapVolumeToggle');
  const speakerHeatmapBandSelectEl = document.getElementById('speakerHeatmapBandSelect');
  const speakerHeatmapSampleCountInputEl = document.getElementById('speakerHeatmapSampleCountInput');
  const speakerHeatmapMaxSphereSizeSliderEl = document.getElementById('speakerHeatmapMaxSphereSizeSlider');
  const speakerHeatmapMaxSphereSizeValEl = document.getElementById('speakerHeatmapMaxSphereSizeVal');
  const mpvOverlayToggleEl = document.getElementById('mpvOverlayToggle');
  const mpvOverlaySocketInputEl = document.getElementById('mpvOverlaySocketInput');
  const mpvOverlayStatusEl = document.getElementById('mpvOverlayStatus');

  if (mpvOverlaySocketInputEl) {
    // Placeholder shows the OS-native default mpv writes when launched
    // with `--input-ipc-server=…`. Same convention as the audio input
    // pipe: the user types the literal mpv was given, no normalisation.
    const isWindows = /windows/i.test(navigator.userAgent || '');
    mpvOverlaySocketInputEl.placeholder = isWindows
      ? String.raw`\\.\pipe\omniphony-mpv`
      : '/tmp/omniphony-mpv.sock';
  }

  function refreshMpvOverlayUI() {
    const s = getMpvOverlayStatus();
    // Keep the input / toggle in sync with backend state. Prefs load
    // asynchronously after this listener registers, so on first ticks the
    // controls would otherwise be stuck on their empty initial values.
    if (mpvOverlayToggleEl && mpvOverlayToggleEl.checked !== s.enabled) {
      mpvOverlayToggleEl.checked = s.enabled;
    }
    if (
      mpvOverlaySocketInputEl
      && document.activeElement !== mpvOverlaySocketInputEl
      && mpvOverlaySocketInputEl.value !== s.socketPath
    ) {
      mpvOverlaySocketInputEl.value = s.socketPath;
    }
    if (!mpvOverlayStatusEl) return;
    if (!s.enabled) {
      mpvOverlayStatusEl.textContent = 'disabled';
    } else if (s.connected) {
      mpvOverlayStatusEl.textContent = `connected → ${s.socketPath}`;
    } else if (s.lastError) {
      mpvOverlayStatusEl.textContent = `error: ${s.lastError}`;
    } else {
      mpvOverlayStatusEl.textContent = 'connecting…';
    }
  }

  refreshMpvOverlayUI();
  setInterval(refreshMpvOverlayUI, 500);

  if (mpvOverlayToggleEl) {
    mpvOverlayToggleEl.addEventListener('change', async () => {
      // <input> only fires "change" on blur, so if the user typed a path then
      // clicked the toggle the path is still pending — flush it first.
      if (mpvOverlaySocketInputEl) {
        await setMpvOverlaySocketPath(mpvOverlaySocketInputEl.value);
      }
      await setMpvOverlayEnabled(mpvOverlayToggleEl.checked);
      refreshMpvOverlayUI();
    });
  }

  if (mpvOverlaySocketInputEl) {
    mpvOverlaySocketInputEl.addEventListener('change', async () => {
      await setMpvOverlaySocketPath(mpvOverlaySocketInputEl.value);
      refreshMpvOverlayUI();
    });
  }

  function applyObjectDetailsVisibility(enabled) {
    app.showObjectDetails = Boolean(enabled);
    if (showObjectDetailsToggleEl) {
      showObjectDetailsToggleEl.checked = app.showObjectDetails;
    }
    if (objectDetailsToggleBtnEl) {
      objectDetailsToggleBtnEl.classList.toggle('active', app.showObjectDetails);
      objectDetailsToggleBtnEl.setAttribute('aria-pressed', app.showObjectDetails ? 'true' : 'false');
      objectDetailsToggleBtnEl.textContent = app.showObjectDetails ? '▾' : '▸';
    }
    document.body.classList.toggle('hide-object-details', !app.showObjectDetails);
  }

  if (trailToggleEl) {
    trailToggleEl.addEventListener('change', () => {
      app.trailsEnabled = trailToggleEl.checked;
      sourceTrails.forEach((trail, id) => {
        trail.line.visible = app.trailsEnabled;
        if (app.trailsEnabled) {
          rebuildTrailGeometry(id);
        }
      });
      persistTrailPrefs();
      pushMpvOverlayTrailPrefs(
        app.trailsEnabled, app.trailPointTtlMs,
        app.trailRenderMode, app.trailTeleportThreshold
      );
    });
  }

  if (effectiveRenderToggleEl) {
    effectiveRenderToggleEl.addEventListener('change', () => {
      app.effectiveRenderEnabled = effectiveRenderToggleEl.checked;
      refreshEffectiveRenderVisibility();
      persistEffectiveRenderPrefs();
    });
  }

  if (objectColorsToggleEl) {
    objectColorsToggleEl.checked = app.objectColorsEnabled;
    objectColorsToggleEl.addEventListener('change', () => {
      app.objectColorsEnabled = objectColorsToggleEl.checked;
      updateSourceSelectionStyles();
      sourceTrails.forEach((_trail, id) => {
        rebuildTrailGeometry(id);
      });
      refreshOverlayLists();
      persistEffectiveRenderPrefs();
    });
  }

  if (objectDisplayModeSelectEl) {
    objectDisplayModeSelectEl.value = app.objectDisplayMode;
    objectDisplayModeSelectEl.addEventListener('change', () => {
      const nextMode = objectDisplayModeSelectEl.value;
      app.objectDisplayMode = nextMode === 'transparent-sphere' || nextMode === 'diffuse-sphere'
        ? nextMode
        : 'circle';
      updateSourceSelectionStyles();
      persistEffectiveRenderPrefs();
    });
  }

  if (objectSphereSizeSliderEl) {
    objectSphereSizeSliderEl.value = String(app.objectSphereSize);
    if (objectSphereSizeValEl) {
      objectSphereSizeValEl.textContent = app.objectSphereSize.toFixed(3);
    }
    objectSphereSizeSliderEl.addEventListener('input', () => {
      const nextSize = Number(objectSphereSizeSliderEl.value);
      app.objectSphereSize = Math.max(0.03, Math.min(0.2, Number.isFinite(nextSize) ? nextSize : 0.07));
      if (objectSphereSizeValEl) {
        objectSphereSizeValEl.textContent = app.objectSphereSize.toFixed(3);
      }
      updateSourceSelectionStyles();
      sourceMeshes.forEach((_mesh, id) => {
        updateSourceDecorations(id);
      });
      persistEffectiveRenderPrefs();
    });
  }

  if (objectLabelsToggleEl) {
    objectLabelsToggleEl.checked = app.objectLabelsEnabled;
    objectLabelsToggleEl.addEventListener('change', () => {
      app.objectLabelsEnabled = objectLabelsToggleEl.checked;
      sourceMeshes.forEach((_mesh, id) => {
        updateSourceDecorations(id);
      });
      persistEffectiveRenderPrefs();
    });
  }

  if (showObjectDetailsToggleEl) {
    applyObjectDetailsVisibility(app.showObjectDetails);
    showObjectDetailsToggleEl.addEventListener('change', () => {
      applyObjectDetailsVisibility(showObjectDetailsToggleEl.checked);
      persistEffectiveRenderPrefs();
    });
  }

  if (objectDetailsToggleBtnEl) {
    applyObjectDetailsVisibility(app.showObjectDetails);
    objectDetailsToggleBtnEl.addEventListener('click', () => {
      applyObjectDetailsVisibility(!app.showObjectDetails);
      persistEffectiveRenderPrefs();
    });
  }

  if (speakerLabelsToggleEl) {
    speakerLabelsToggleEl.checked = app.speakerLabelsEnabled;
    speakerLabelsToggleEl.addEventListener('change', () => {
      app.speakerLabelsEnabled = speakerLabelsToggleEl.checked;
      speakerLabels.forEach((label) => {
        if (label) {
          label.visible = app.speakerLabelsEnabled;
        }
      });
      persistEffectiveRenderPrefs();
    });
  }

  if (speakerSizeSliderEl) {
    speakerSizeSliderEl.value = String(app.speakerSize);
    if (speakerSizeValEl) {
      speakerSizeValEl.textContent = app.speakerSize.toFixed(3);
    }
    speakerSizeSliderEl.addEventListener('input', () => {
      const nextSize = Number(speakerSizeSliderEl.value);
      app.speakerSize = Math.max(0.04, Math.min(0.2, Number.isFinite(nextSize) ? nextSize : 0.08));
      if (speakerSizeValEl) {
        speakerSizeValEl.textContent = app.speakerSize.toFixed(3);
      }
      speakerMeshes.forEach((mesh, index) => {
        applySpeakerLevel(mesh, speakerLevels.get(String(index)));
        updateSpeakerVisualsFromState(index);
      });
      persistEffectiveRenderPrefs();
    });
  }

  if (trailModeSelectEl) {
    trailModeSelectEl.value = app.trailRenderMode;
    trailModeSelectEl.addEventListener('change', () => {
      app.trailRenderMode = trailModeSelectEl.value === 'line' ? 'line' : 'diffuse';
      sourceTrails.forEach((trail, id) => {
        const wasVisible = trail.line.visible;
        scene.remove(trail.line);
        trail.line.geometry.dispose();
        trail.line.material.dispose();
        trail.line = createTrailRenderable();
        trail.line.visible = wasVisible;
        scene.add(trail.line);
        if (app.trailsEnabled) {
          rebuildTrailGeometry(id);
        }
      });
      persistTrailPrefs();
      pushMpvOverlayTrailPrefs(
        app.trailsEnabled, app.trailPointTtlMs,
        app.trailRenderMode, app.trailTeleportThreshold
      );
    });
  }

  if (trailTtlSliderEl) {
    trailTtlSliderEl.addEventListener('input', () => {
      const seconds = Number(trailTtlSliderEl.value);
      app.trailPointTtlMs = Math.max(500, seconds * 1000);
      if (trailTtlValEl) trailTtlValEl.textContent = `${seconds.toFixed(1)}s`;
      persistTrailPrefs();
      pushMpvOverlayTrailPrefs(
        app.trailsEnabled, app.trailPointTtlMs,
        app.trailRenderMode, app.trailTeleportThreshold
      );
    });
  }

  if (trailTeleportSliderEl) {
    trailTeleportSliderEl.addEventListener('input', () => {
      const value = Number(trailTeleportSliderEl.value);
      app.trailTeleportThreshold = Math.max(0.05, Math.min(2.0, Number.isFinite(value) ? value : 0.5));
      if (trailTeleportValEl) {
        trailTeleportValEl.textContent = app.trailTeleportThreshold.toFixed(2);
      }
      // Rebuild every visible trail so the new threshold takes effect on
      // already-buffered points (we evaluate breaks at rebuild time from
      // the raw consecutive positions).
      sourceTrails.forEach((_trail, id) => {
        rebuildTrailGeometry(id);
      });
      persistTrailPrefs();
      pushMpvOverlayTrailPrefs(
        app.trailsEnabled, app.trailPointTtlMs,
        app.trailRenderMode, app.trailTeleportThreshold
      );
    });
  }

  if (localeSelectEl) {
    localeSelectEl.addEventListener('change', () => {
      setLocale(localeSelectEl.value || 'auto');
    });
  }

  if (speakerHeatmapSlicesToggleEl) {
    speakerHeatmapSlicesToggleEl.checked = app.speakerHeatmapSlicesEnabled;
    speakerHeatmapSlicesToggleEl.addEventListener('change', () => {
      app.speakerHeatmapSlicesEnabled = speakerHeatmapSlicesToggleEl.checked;
      subscribeSpeakerHeatmap();
      persistEffectiveRenderPrefs();
    });
  }

  if (speakerHeatmapVolumeToggleEl) {
    speakerHeatmapVolumeToggleEl.checked = app.speakerHeatmapVolumeEnabled;
    speakerHeatmapVolumeToggleEl.addEventListener('change', () => {
      app.speakerHeatmapVolumeEnabled = speakerHeatmapVolumeToggleEl.checked;
      subscribeSpeakerHeatmap();
      persistEffectiveRenderPrefs();
    });
  }

  if (speakerHeatmapBandSelectEl) {
    syncSpeakerHeatmapBandSelect();
    speakerHeatmapBandSelectEl.addEventListener('change', () => {
      const nextBandIndex = Number(speakerHeatmapBandSelectEl.value);
      app.speakerHeatmapBandIndex = Math.max(0, Math.round(Number.isFinite(nextBandIndex) ? nextBandIndex : 0));
      syncSpeakerHeatmapBandSelect();
      refreshOverlayLists();
      refreshEffectiveRenderVisibility();
      subscribeSpeakerHeatmap();
      persistEffectiveRenderPrefs();
    });
  }

  if (speakerHeatmapSampleCountInputEl) {
    speakerHeatmapSampleCountInputEl.value = String(app.speakerHeatmapSampleCount);
    speakerHeatmapSampleCountInputEl.addEventListener('change', () => {
      const nextCount = Number(speakerHeatmapSampleCountInputEl.value);
      app.speakerHeatmapSampleCount = Math.max(128, Math.min(20000, Math.round(Number.isFinite(nextCount) ? nextCount : 3072)));
      speakerHeatmapSampleCountInputEl.value = String(app.speakerHeatmapSampleCount);
      subscribeSpeakerHeatmap();
      persistEffectiveRenderPrefs();
    });
  }

  if (speakerHeatmapMaxSphereSizeSliderEl) {
    speakerHeatmapMaxSphereSizeSliderEl.value = String(app.speakerHeatmapMaxSphereSize);
    if (speakerHeatmapMaxSphereSizeValEl) {
      speakerHeatmapMaxSphereSizeValEl.textContent = app.speakerHeatmapMaxSphereSize.toFixed(3);
    }
    speakerHeatmapMaxSphereSizeSliderEl.addEventListener('input', () => {
      const nextSize = Number(speakerHeatmapMaxSphereSizeSliderEl.value);
      app.speakerHeatmapMaxSphereSize = Math.max(0.01, Math.min(0.2, Number.isFinite(nextSize) ? nextSize : 0.062));
      if (speakerHeatmapMaxSphereSizeValEl) {
        speakerHeatmapMaxSphereSizeValEl.textContent = app.speakerHeatmapMaxSphereSize.toFixed(3);
      }
      subscribeSpeakerHeatmap();
      persistEffectiveRenderPrefs();
    });
  }

  // Seed Rust with the current trail prefs so they're already stashed for
  // the first mpv overlay (re)connect — even if the user never touches the
  // trail UI this session.
  pushMpvOverlayTrailPrefs(
    app.trailsEnabled, app.trailPointTtlMs,
    app.trailRenderMode, app.trailTeleportThreshold
  );
}
