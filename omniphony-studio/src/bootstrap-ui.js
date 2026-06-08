import { mountAudioPanel } from './ui/audio-panel.js';
import { mountInputPanel } from './ui/input-panel.js';
import { mountRendererPanel } from './ui/renderer-panel.js';
import { initSidePanels } from './ui/side-panels.js';
import { wireInlineHelpFromMarkup } from './controls/inline-help.js';

mountAudioPanel();
mountInputPanel();
mountRendererPanel();
initSidePanels();
// Wire the inline parameter help across every panel (static index.html markup
// plus the just-mounted panels). Idempotent, so the renderer panel's own
// mount-time pass is harmless to repeat here.
wireInlineHelpFromMarkup(document);
