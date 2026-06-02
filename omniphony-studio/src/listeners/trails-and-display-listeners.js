import { app, sourceMeshes, sourceTrails, speakerLabels, speakerLevels, speakerMeshes } from '../state.js';
import { setLocale } from '../i18n.js';
import { persistTrailPrefs, persistEffectiveRenderPrefs, refreshEffectiveRenderVisibility } from '../controls/room-geometry.js';
import { rebuildTrailGeometry, createTrailRenderable } from '../trails.js';
import { scene } from '../scene/setup.js';
import { applySpeakerLevel, updateSourceDecorations, updateSourceSelectionStyles, applyObjectsVisibility } from '../sources.js';
import { refreshOverlayLists, updateSpeakerVisualsFromState } from '../speakers.js';
import { subscribeSpeakerHeatmap, syncSpeakerHeatmapBandSelect } from '../scene/speaker-heatmap.js';
import { refreshObjectEnergyVolume } from '../scene/object-energy-volume.js';
import { clampVolumeGamma, colormapIndex } from '../scene/object-energy-shared.js';
import {
  getMpvOverlayStatus,
  setMpvOverlayEnabled,
  pushMpvOverlayTrailPrefs
} from '../mpvOverlay.js';
import { invoke } from '@tauri-apps/api/core';

export function setupTrailsAndDisplayListeners() {
  const trailToggleEl = document.getElementById('trailToggle');
  const effectiveRenderToggleEl = document.getElementById('effectiveRenderToggle');
  const showObjectsToggleEl = document.getElementById('showObjectsToggle');
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
  const objectEnergyHeatmapToggleEl = document.getElementById('objectEnergyHeatmapToggle');
  const objectEnergyColormapEl = document.getElementById('objectEnergyColormap');
  const objectEnergyVolumeMixSliderEl = document.getElementById('objectEnergyVolumeMixSlider');
  const objectEnergyVolumeMixValEl = document.getElementById('objectEnergyVolumeMixVal');
  const objectEnergyVolumeGammaAccumulateSliderEl = document.getElementById('objectEnergyVolumeGammaAccumulateSlider');
  const objectEnergyVolumeGammaAccumulateValEl = document.getElementById('objectEnergyVolumeGammaAccumulateVal');
  const objectEnergyVolumeGammaMipSliderEl = document.getElementById('objectEnergyVolumeGammaMipSlider');
  const objectEnergyVolumeGammaMipValEl = document.getElementById('objectEnergyVolumeGammaMipVal');
  const objectEnergyHeatmapResolutionSliderEl = document.getElementById('objectEnergyHeatmapResolutionSlider');
  const objectEnergyHeatmapResolutionValEl = document.getElementById('objectEnergyHeatmapResolutionVal');
  const objectEnergyHeatmapRadiusSliderEl = document.getElementById('objectEnergyHeatmapRadiusSlider');
  const objectEnergyHeatmapRadiusValEl = document.getElementById('objectEnergyHeatmapRadiusVal');
  const objectEnergyHeatmapOpacitySliderEl = document.getElementById('objectEnergyHeatmapOpacitySlider');
  const objectEnergyHeatmapOpacityValEl = document.getElementById('objectEnergyHeatmapOpacityVal');
  const mpvOverlayToggleEl = document.getElementById('mpvOverlayToggle');

  if (mpvOverlayToggleEl) {
    mpvOverlayToggleEl.checked = getMpvOverlayStatus().enabled;
    mpvOverlayToggleEl.addEventListener('change', async () => {
      await setMpvOverlayEnabled(mpvOverlayToggleEl.checked);
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

  if (showObjectsToggleEl) {
    showObjectsToggleEl.checked = app.objectsVisible !== false;
    showObjectsToggleEl.addEventListener('change', () => {
      app.objectsVisible = showObjectsToggleEl.checked;
      applyObjectsVisibility();
      invoke('mpv_overlay_set_objects', { visible: app.objectsVisible }).catch(() => {});
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
      // Mirror the toggle to the in-process mpv overlay (OSC control).
      invoke('mpv_overlay_set_labels', { enabled: app.objectLabelsEnabled }).catch(() => {});
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

  // Force the next refresh tick to rebuild immediately rather than wait out the
  // throttle window, so UI changes feel instant.
  const refreshObjectEnergyHeatmapNow = () => {
    app.lastObjectEnergyHeatmapAt = 0;
    refreshObjectEnergyVolume(performance.now());
  };

  if (objectEnergyHeatmapToggleEl) {
    objectEnergyHeatmapToggleEl.checked = app.objectEnergyHeatmapEnabled;
    objectEnergyHeatmapToggleEl.addEventListener('change', () => {
      app.objectEnergyHeatmapEnabled = objectEnergyHeatmapToggleEl.checked;
      refreshObjectEnergyHeatmapNow();
      persistEffectiveRenderPrefs();
    });
  }

  // Colour gradient — applies to the Studio 3D volume and the mpv overlay.
  if (objectEnergyColormapEl) {
    objectEnergyColormapEl.value = app.objectEnergyColormap;
    objectEnergyColormapEl.addEventListener('change', () => {
      app.objectEnergyColormap = objectEnergyColormapEl.value;
      refreshObjectEnergyHeatmapNow();
      invoke('mpv_overlay_set_heatmap_colormap', { colormap: colormapIndex(app.objectEnergyColormap) }).catch(() => {});
      persistEffectiveRenderPrefs();
    });
  }

  // Mix slider: 0 = pure accumulate … 1 = pure peak. Both components render at
  // once and are blended in the shader.
  if (objectEnergyVolumeMixSliderEl) {
    objectEnergyVolumeMixSliderEl.value = String(app.objectEnergyVolumeMix);
    if (objectEnergyVolumeMixValEl) {
      objectEnergyVolumeMixValEl.textContent = app.objectEnergyVolumeMix.toFixed(2);
    }
    objectEnergyVolumeMixSliderEl.addEventListener('input', () => {
      const next = Number(objectEnergyVolumeMixSliderEl.value);
      app.objectEnergyVolumeMix = Math.max(0, Math.min(1, Number.isFinite(next) ? next : 0));
      if (objectEnergyVolumeMixValEl) {
        objectEnergyVolumeMixValEl.textContent = app.objectEnergyVolumeMix.toFixed(2);
      }
      refreshObjectEnergyHeatmapNow();
      persistEffectiveRenderPrefs();
    });
  }

  // Each component keeps its own γ (own range): peak weighs one sample, accumulate
  // integrates the whole ray, so a shared value isn't comparable.
  const bindVolumeGamma = (sliderEl, valEl, field, projection, digits) => {
    if (!sliderEl) return;
    sliderEl.value = String(app[field]);
    if (valEl) valEl.textContent = app[field].toFixed(digits);
    sliderEl.addEventListener('input', () => {
      app[field] = clampVolumeGamma(projection, sliderEl.value);
      if (valEl) valEl.textContent = app[field].toFixed(digits);
      refreshObjectEnergyHeatmapNow();
      persistEffectiveRenderPrefs();
    });
  };
  bindVolumeGamma(objectEnergyVolumeGammaAccumulateSliderEl, objectEnergyVolumeGammaAccumulateValEl,
    'objectEnergyVolumeGammaAccumulate', 'accumulate', 1);
  bindVolumeGamma(objectEnergyVolumeGammaMipSliderEl, objectEnergyVolumeGammaMipValEl,
    'objectEnergyVolumeGammaMip', 'mip', 2);

  if (objectEnergyHeatmapResolutionSliderEl) {
    objectEnergyHeatmapResolutionSliderEl.value = String(app.objectEnergyHeatmapResolution);
    if (objectEnergyHeatmapResolutionValEl) {
      objectEnergyHeatmapResolutionValEl.textContent = String(app.objectEnergyHeatmapResolution);
    }
    objectEnergyHeatmapResolutionSliderEl.addEventListener('input', () => {
      const next = Number(objectEnergyHeatmapResolutionSliderEl.value);
      app.objectEnergyHeatmapResolution = Math.max(8, Math.min(64, Math.round(Number.isFinite(next) ? next : 24)));
      if (objectEnergyHeatmapResolutionValEl) {
        objectEnergyHeatmapResolutionValEl.textContent = String(app.objectEnergyHeatmapResolution);
      }
      refreshObjectEnergyHeatmapNow();
      persistEffectiveRenderPrefs();
    });
  }

  if (objectEnergyHeatmapRadiusSliderEl) {
    objectEnergyHeatmapRadiusSliderEl.value = String(app.objectEnergyHeatmapFalloffRadius);
    if (objectEnergyHeatmapRadiusValEl) {
      objectEnergyHeatmapRadiusValEl.textContent = app.objectEnergyHeatmapFalloffRadius.toFixed(2);
    }
    objectEnergyHeatmapRadiusSliderEl.addEventListener('input', () => {
      const next = Number(objectEnergyHeatmapRadiusSliderEl.value);
      app.objectEnergyHeatmapFalloffRadius = Math.max(0.02, Math.min(0.5, Number.isFinite(next) ? next : 0.12));
      if (objectEnergyHeatmapRadiusValEl) {
        objectEnergyHeatmapRadiusValEl.textContent = app.objectEnergyHeatmapFalloffRadius.toFixed(2);
      }
      refreshObjectEnergyHeatmapNow();
      persistEffectiveRenderPrefs();
    });
  }

  if (objectEnergyHeatmapOpacitySliderEl) {
    objectEnergyHeatmapOpacitySliderEl.value = String(app.objectEnergyHeatmapOpacity);
    if (objectEnergyHeatmapOpacityValEl) {
      objectEnergyHeatmapOpacityValEl.textContent = app.objectEnergyHeatmapOpacity.toFixed(2);
    }
    objectEnergyHeatmapOpacitySliderEl.addEventListener('input', () => {
      const next = Number(objectEnergyHeatmapOpacitySliderEl.value);
      app.objectEnergyHeatmapOpacity = Math.max(0.05, Math.min(1.0, Number.isFinite(next) ? next : 0.55));
      if (objectEnergyHeatmapOpacityValEl) {
        objectEnergyHeatmapOpacityValEl.textContent = app.objectEnergyHeatmapOpacity.toFixed(2);
      }
      refreshObjectEnergyHeatmapNow();
      persistEffectiveRenderPrefs();
    });
  }

  // No startup seed: the mpv overlay display prefs (enable / labels / trails)
  // are owned and persisted by orender now and loaded at its startup. Studio
  // only drives them live via the on-change handlers above; seeding here would
  // override orender's persisted state at connect.
}
