// Headphone L/R channel rows for binaural mode — same rendering as the
// speaker rows (same DOM classes, same meter pipeline), shown in place of
// the speakers list. The two rows ride the first two speaker param slots:
// meters come from /omniphony/meter/speaker/0|1 and M/S drive
// speaker mute 0/1, which the renderer applies to the ears in binaural.
//
// Position icon: a top-view headphone glyph with the active cup filled
// (left cup for L, right for R). No crossover/filter glyph — there is no
// per-ear crossover.

import { toggleMute, toggleSolo, updateMeterUI, getSoloTarget } from '../mute-solo.js';
import { updateItemClasses } from '../flush.js';
import { speakerMuted } from '../state.js';

const entries = new Map(); // '0' | '1' → row entry

function headphoneSvg(side) {
  const fillL = side === 'L' ? '#8cd6ff' : 'none';
  const fillR = side === 'R' ? '#8cd6ff' : 'none';
  return `<svg viewBox="0 0 20 20" width="100%" height="100%" aria-hidden="true">
    <path d="M3 12 a7 7 0 0 1 14 0" fill="none" stroke="#9eb4c8" stroke-width="1.6"/>
    <rect x="2" y="11" width="4" height="6" rx="1.2" fill="${fillL}" stroke="#9eb4c8" stroke-width="1.2"/>
    <rect x="14" y="11" width="4" height="6" rx="1.2" fill="${fillR}" stroke="#9eb4c8" stroke-width="1.2"/>
  </svg>`;
}

function createChannelRow(side, id) {
  const root = document.createElement('div');
  root.className = 'info-item speaker-item';

  const idStrip = document.createElement('div');
  idStrip.className = 'id-strip flip';
  const idText = document.createElement('span');
  idText.textContent = side;
  idStrip.appendChild(idText);

  const content = document.createElement('div');
  content.className = 'speaker-content';

  const level = document.createElement('div');
  level.className = 'meter-row speaker-meter-row';

  const positionIcon = document.createElement('span');
  positionIcon.className = 'speaker-position-icon';
  positionIcon.innerHTML = headphoneSvg(side);
  level.appendChild(positionIcon);

  const levelText = document.createElement('div');
  levelText.className = 'fixed-metric';
  level.appendChild(levelText);

  const meterBar = document.createElement('div');
  meterBar.className = 'meter-bar level-meter';
  const meterFill = document.createElement('div');
  meterFill.className = 'meter-fill';
  const peakCursor = document.createElement('div');
  peakCursor.className = 'meter-peak';
  meterBar.appendChild(meterFill);
  meterBar.appendChild(peakCursor);

  const controlsRow = document.createElement('div');
  controlsRow.className = 'speaker-meter-actions';
  const muteBtn = document.createElement('button');
  muteBtn.type = 'button';
  muteBtn.className = 'toggle-btn';
  muteBtn.textContent = 'M';
  muteBtn.addEventListener('click', (e) => {
    e.preventDefault();
    toggleMute('speaker', id);
  });
  const soloBtn = document.createElement('button');
  soloBtn.type = 'button';
  soloBtn.className = 'toggle-btn';
  soloBtn.textContent = 'S';
  soloBtn.addEventListener('click', (e) => {
    e.preventDefault();
    toggleSolo('speaker', id);
  });
  controlsRow.append(muteBtn, soloBtn);

  level.appendChild(meterBar);
  level.appendChild(controlsRow);
  content.appendChild(level);
  root.append(idStrip, content);

  return { root, levelText, meterFill, peakCursor, muteBtn, soloBtn };
}

/** Build the two rows into #hpChannelsList (idempotent). */
export function initHeadphoneChannels() {
  const list = document.getElementById('hpChannelsList');
  if (!list || entries.size) return;
  for (const [side, id] of [['L', '0'], ['R', '1']]) {
    const entry = createChannelRow(side, id);
    entries.set(id, entry);
    list.appendChild(entry.root);
  }
}

/** Meter update for ear `index` (0|1); same pipeline as the speaker rows. */
export function updateHeadphoneMeter(index, level) {
  const entry = entries.get(String(index));
  if (!entry) return;
  updateMeterUI(entry, level, 'speaker', `hp${index}`);
}

/** Mirror the mute/solo state onto the L/R buttons (same classes as rows). */
export function updateHeadphoneControlsUI() {
  const soloTarget = getSoloTarget('speaker');
  entries.forEach((entry, id) => {
    entry.muteBtn.classList.toggle('active', speakerMuted.has(id));
    entry.soloBtn.classList.toggle('active', soloTarget === id);
    updateItemClasses(entry, speakerMuted.has(id), Boolean(soloTarget && soloTarget !== id));
  });
}
