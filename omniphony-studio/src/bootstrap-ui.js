import { mountAudioPanel } from './ui/audio-panel.js';
import { mountInputPanel } from './ui/input-panel.js';
import { mountRendererPanel } from './ui/renderer-panel.js';
import { initSidePanels } from './ui/side-panels.js';

mountAudioPanel();
mountInputPanel();
mountRendererPanel();
initSidePanels();
