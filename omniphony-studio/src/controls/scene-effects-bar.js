// Floating quick-toggle bar pinned over the bottom of the 3D view. Each icon
// button mirrors one of the main display toggles in the renderer panel: it
// drives the source checkbox (dispatching `change`, so every existing handler
// and state update runs exactly as if the panel checkbox were clicked) and
// reflects that checkbox's state with an `.active` style. No effect logic lives
// here — this is purely a convenience surface over the existing controls.

// Button id -> source checkbox id. Order defines left-to-right layout.
const SCENE_FX = [
  { btn: 'fxGridBtn', src: 'vbapCartesianGridToggleBtn' },
  { btn: 'fxObjectsBtn', src: 'showObjectsToggle' },
  { btn: 'fxLabelsBtn', src: 'objectLabelsToggle' },
  { btn: 'fxTrailsBtn', src: 'trailToggle' },
  { btn: 'fxFieldBtn', src: 'objectEnergyHeatmapToggle' },
  { btn: 'fxHeatmapBtn', src: 'speakerHeatmapVolumeToggle' },
  { btn: 'fxMpvBtn', src: 'mpvOverlayToggle' },
];

function reflect(entry) {
  const btn = document.getElementById(entry.btn);
  const src = document.getElementById(entry.src);
  if (!btn || !src) return;
  const on = !!src.checked;
  btn.classList.toggle('active', on);
  btn.setAttribute('aria-pressed', on ? 'true' : 'false');
}

/** Re-read every source checkbox and update its button. Safe to call any time
 *  (e.g. after the display panel is re-rendered from persisted state). */
export function syncSceneEffectsBar() {
  for (const e of SCENE_FX) reflect(e);
}

/** Wire the bar's buttons to their source checkboxes. Call after the renderer/
 *  display panel listeners are installed so the dispatched `change` reaches the
 *  existing handlers. */
export function initSceneEffectsBar() {
  for (const entry of SCENE_FX) {
    const btn = document.getElementById(entry.btn);
    const src = document.getElementById(entry.src);
    if (!btn || !src) continue;
    btn.addEventListener('click', () => {
      src.checked = !src.checked;
      src.dispatchEvent(new Event('change', { bubbles: true }));
      reflect(entry);
    });
    // Keep the button in sync when the panel checkbox is toggled directly (the
    // dispatched event above also lands here, which is harmless/idempotent).
    src.addEventListener('change', () => reflect(entry));
    reflect(entry);
  }
}
