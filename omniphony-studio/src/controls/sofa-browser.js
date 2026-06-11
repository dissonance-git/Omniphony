// SOFA HRTF database browser: navigates the Apache index of
// sofacoustics.org/data via the `sofa_browse` Tauri command, downloads a
// chosen .sofa with `sofa_download`, then activates it through the existing
// `control_hrir_source` command (`sofa:<local path>`).
//
// State is one percent-encoded relative path ("" = root). Starts in
// "database/", where the per-subject HRTF sets live.

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

const el = (id) => document.getElementById(id);

let currentPath = 'database/';
let busy = false;
// Remote path (currentPath + href) of the currently active .sofa, so the
// entry stays visibly marked while scrolling / navigating / reopening.
let activeRemotePath = '';
// Browser mode: 'local' (default — already-downloaded files, no network)
// or 'remote' (sofacoustics.org). Going online asks for confirmation once
// per session.
let mode = 'local';
let onlineConsent = false;
// Active local .sofa path, fed from the renderer state broadcast
// (binaural.hrtfSofaPath) so the highlight survives Studio restarts.
let activeSofaPath = '';

export function setActiveSofaPath(path) {
  activeSofaPath = typeof path === 'string' ? path : '';
}

const fmtMB = (b) => `${(b / (1024 * 1024)).toFixed(1)} MB`;

function setDownloadUiVisible(visible) {
  const row = el('sofaDlRow');
  if (row) row.style.display = visible ? 'flex' : 'none';
  if (visible) {
    const bar = el('sofaDlBar');
    const txt = el('sofaDlText');
    if (bar) bar.style.width = '0%';
    if (txt) txt.textContent = '';
  }
}

function onDownloadProgress({ bytes, total }) {
  const bar = el('sofaDlBar');
  const txt = el('sofaDlText');
  if (total) {
    const pct = Math.min(100, (bytes / total) * 100);
    if (bar) bar.style.width = `${pct.toFixed(1)}%`;
    if (txt) txt.textContent = `${fmtMB(bytes)} / ${fmtMB(total)}`;
  } else {
    // Unknown size: full-width pulse + byte counter.
    if (bar) { bar.style.width = '100%'; bar.style.opacity = '0.35'; }
    if (txt) txt.textContent = fmtMB(bytes);
  }
}

function setStatus(text, isError = false) {
  const n = el('sofaBrowserStatus');
  if (!n) return;
  n.textContent = text || '';
  n.style.color = isError ? '#ff7a7a' : '#c9d6e2';
}

function renderCrumbs() {
  const n = el('sofaBrowserCrumbs');
  if (!n) return;
  n.textContent = '';
  const mk = (label, path) => {
    const a = document.createElement('a');
    a.textContent = label;
    a.href = '#';
    a.style.color = '#7fb3e8';
    a.addEventListener('click', (e) => {
      e.preventDefault();
      navigate(path);
    });
    return a;
  };
  n.appendChild(mk('data', ''));
  let acc = '';
  for (const seg of currentPath.split('/').filter(Boolean)) {
    acc += `${seg}/`;
    n.appendChild(document.createTextNode(' / '));
    n.appendChild(mk(decodeURIComponent(seg), acc));
  }
}

function renderList(entries) {
  const list = el('sofaBrowserList');
  if (!list) return;
  list.textContent = '';
  if (currentPath) {
    const up = document.createElement('div');
    up.textContent = '⬑ ..';
    up.style.cssText = 'cursor:pointer;padding:0.15rem 0.3rem;color:#8fa6bd;';
    up.addEventListener('click', () => {
      const parts = currentPath.split('/').filter(Boolean);
      parts.pop();
      navigate(parts.length ? `${parts.join('/')}/` : '');
    });
    list.appendChild(up);
  }
  if (!entries.length) {
    const empty = document.createElement('div');
    empty.textContent = '(no subfolders or .sofa files here)';
    empty.style.cssText = 'padding:0.15rem 0.3rem;color:#777;';
    list.appendChild(empty);
    return;
  }
  let activeRow = null;
  for (const entry of entries) {
    const row = document.createElement('div');
    const isActive = !entry.dir && currentPath + entry.href === activeRemotePath;
    row.dataset.sofaPath = entry.dir ? '' : currentPath + entry.href;
    row.style.cssText =
      'display:flex;justify-content:space-between;gap:0.5rem;cursor:pointer;' +
      'padding:0.15rem 0.3rem;border-radius:4px;';
    if (isActive) {
      row.style.background = 'rgba(86,156,255,0.22)';
      row.style.boxShadow = 'inset 2px 0 0 #569cff';
      activeRow = row;
    }
    row.addEventListener('mouseenter', () => {
      if (row.dataset.sofaPath !== activeRemotePath || !row.dataset.sofaPath) {
        row.style.background = 'rgba(255,255,255,0.06)';
      }
    });
    row.addEventListener('mouseleave', () => {
      row.style.background =
        row.dataset.sofaPath && row.dataset.sofaPath === activeRemotePath
          ? 'rgba(86,156,255,0.22)'
          : '';
    });
    const name = document.createElement('span');
    name.textContent = entry.dir
      ? `📁 ${entry.name}/`
      : `🎧 ${entry.name}${isActive ? '  ✓ active' : ''}`;
    name.style.wordBreak = 'break-all';
    if (isActive) name.style.color = '#9cc4ff';
    const size = document.createElement('span');
    size.textContent = entry.dir ? '' : entry.size;
    size.style.cssText = 'color:#8fa6bd;white-space:nowrap;';
    row.append(name, size);
    row.addEventListener('click', () => {
      if (entry.dir) {
        navigate(currentPath + entry.href);
      } else {
        downloadAndActivate(entry);
      }
    });
    list.appendChild(row);
  }
  // Bring the active file back into view when (re)entering its folder.
  if (activeRow) activeRow.scrollIntoView({ block: 'nearest' });
}

function setChrome() {
  const title = el('sofaBrowserTitle');
  const toggle = el('sofaSourceToggleBtn');
  const crumbs = el('sofaBrowserCrumbs');
  if (mode === 'local') {
    if (title) title.textContent = 'SOFA HRTF files — downloaded';
    if (toggle) { toggle.textContent = 'Online database…'; toggle.style.display = ''; }
    if (crumbs) crumbs.textContent = '';
  } else {
    if (title) title.textContent = 'SOFA HRTF database — sofacoustics.org';
    if (toggle) { toggle.textContent = '← Local files'; toggle.style.display = ''; }
  }
}

async function showLocal() {
  mode = 'local';
  setChrome();
  setStatus('');
  const list = el('sofaBrowserList');
  if (!list) return;
  list.textContent = 'Loading…';
  let files = [];
  try {
    files = await invoke('sofa_list_local', {});
  } catch (e) {
    setStatus(String(e), true);
    return;
  }
  list.textContent = '';
  if (!files.length) {
    const empty = document.createElement('div');
    empty.textContent = 'No downloaded SOFA files yet — use “Online database…” to fetch some.';
    empty.style.cssText = 'padding:0.3rem;color:#8fa6bd;';
    list.appendChild(empty);
    return;
  }
  let activeRow = null;
  for (const f of files) {
    const row = document.createElement('div');
    const isActive = f.path === activeSofaPath;
    row.style.cssText =
      'display:flex;justify-content:space-between;gap:0.5rem;cursor:pointer;' +
      'padding:0.15rem 0.3rem;border-radius:4px;';
    if (isActive) {
      row.style.background = 'rgba(86,156,255,0.22)';
      row.style.boxShadow = 'inset 2px 0 0 #569cff';
      activeRow = row;
    }
    const name = document.createElement('span');
    name.textContent = `🎧 ${f.name}${isActive ? '  ✓ active' : ''}`;
    name.style.wordBreak = 'break-all';
    if (isActive) name.style.color = '#9cc4ff';
    const size = document.createElement('span');
    size.textContent = f.size;
    size.style.cssText = 'color:#8fa6bd;white-space:nowrap;';
    row.append(name, size);
    row.addEventListener('click', async () => {
      if (busy) return;
      busy = true;
      setStatus('Activating…');
      try {
        await invoke('control_hrir_source', { value: `sofa:${f.path}` });
        const src = el('binauralHrirSource');
        if (src) src.value = 'sofa';
        activeSofaPath = f.path;
        setStatus(`✓ Active: ${f.name}`);
        await showLocal(); // re-render to move the highlight
      } catch (e) {
        setStatus(String(e), true);
      } finally {
        busy = false;
      }
    });
    list.appendChild(row);
  }
  if (activeRow) activeRow.scrollIntoView({ block: 'nearest' });
}

function showOnlineConfirm() {
  setChrome();
  const toggle = el('sofaSourceToggleBtn');
  if (toggle) toggle.style.display = 'none';
  const list = el('sofaBrowserList');
  if (!list) return;
  list.textContent = '';
  const pane = document.createElement('div');
  pane.style.cssText = 'display:grid;gap:0.5rem;padding:0.5rem;';
  const msg = document.createElement('div');
  msg.style.cssText = 'font-size:0.78rem;color:#c9d6e2;line-height:1.4;';
  msg.textContent =
    'This will connect to sofacoustics.org (the public SOFA conventions '
    + 'database) to browse and download HRTF files over the internet. '
    + 'Nothing is sent besides the directory and file requests.';
  const actions = document.createElement('div');
  actions.style.cssText = 'display:flex;gap:0.5rem;justify-content:flex-end;';
  const cancel = document.createElement('button');
  cancel.type = 'button';
  cancel.className = 'ui-btn';
  cancel.textContent = 'Cancel';
  cancel.addEventListener('click', () => showLocal());
  const go = document.createElement('button');
  go.type = 'button';
  go.className = 'ui-btn ui-btn-primary';
  go.textContent = 'Connect';
  go.addEventListener('click', () => {
    onlineConsent = true;
    mode = 'remote';
    setChrome();
    navigate(currentPath);
  });
  actions.append(cancel, go);
  pane.append(msg, actions);
  list.appendChild(pane);
}

async function navigate(path) {
  if (busy) return;
  busy = true;
  currentPath = path;
  renderCrumbs();
  setStatus('Loading…');
  try {
    const entries = await invoke('sofa_browse', { path });
    renderList(entries);
    setStatus('');
  } catch (e) {
    setStatus(String(e), true);
  } finally {
    busy = false;
  }
}

async function downloadAndActivate(entry) {
  if (busy) return;
  busy = true;
  setStatus(`Downloading ${entry.name} (${entry.size})…`);
  setDownloadUiVisible(true);
  try {
    const remotePath = currentPath + entry.href;
    const localPath = await invoke('sofa_download', { path: remotePath });
    setStatus('Activating…');
    await invoke('control_hrir_source', { value: `sofa:${localPath}` });
    // Reflect the selection locally; the state broadcast will confirm it.
    const src = el('binauralHrirSource');
    if (src) src.value = 'sofa';
    activeRemotePath = remotePath;
    activeSofaPath = localPath;
    setStatus(`✓ Active: ${entry.name}`);
    // Re-render so the previous highlight moves to the new file.
    const listEl = el('sofaBrowserList');
    if (listEl) {
      for (const row of listEl.children) {
        const p = row.dataset ? row.dataset.sofaPath : '';
        const on = p && p === activeRemotePath;
        row.style.background = on ? 'rgba(86,156,255,0.22)' : '';
        row.style.boxShadow = on ? 'inset 2px 0 0 #569cff' : '';
        const nameEl = row.firstChild;
        if (nameEl && nameEl.textContent && nameEl.textContent.startsWith('🎧')) {
          nameEl.textContent = nameEl.textContent.replace('  ✓ active', '');
          if (on) nameEl.textContent += '  ✓ active';
          nameEl.style.color = on ? '#9cc4ff' : '';
        }
      }
    }
  } catch (e) {
    const msg = String(e);
    setStatus(msg.includes('cancelled') ? 'Download cancelled.' : msg, !msg.includes('cancelled'));
  } finally {
    setDownloadUiVisible(false);
    busy = false;
  }
}

export function initSofaBrowser() {
  const btn = el('sofaBrowseBtn');
  const modal = el('sofaBrowserModal');
  const close = el('sofaBrowserClose');
  if (!btn || !modal) return;
  btn.addEventListener('click', () => {
    modal.classList.add('open');
    showLocal();
  });
  const sourceToggle = el('sofaSourceToggleBtn');
  if (sourceToggle) {
    sourceToggle.addEventListener('click', () => {
      if (busy) return;
      if (mode === 'remote') {
        showLocal();
      } else if (onlineConsent) {
        mode = 'remote';
        setChrome();
        navigate(currentPath);
      } else {
        showOnlineConfirm();
      }
    });
  }
  if (close) {
    close.addEventListener('click', () => modal.classList.remove('open'));
  }
  modal.addEventListener('click', (e) => {
    if (e.target === modal) modal.classList.remove('open');
  });
  const cancel = el('sofaDlCancel');
  if (cancel) {
    // Deliberately not gated by `busy`: cancelling is only meaningful while
    // a download is in flight.
    cancel.addEventListener('click', () => {
      invoke('sofa_download_cancel').catch((e) => console.error('[sofa] cancel', e));
    });
  }
  listen('sofa:download_progress', ({ payload }) => {
    if (payload && typeof payload === 'object') onDownloadProgress(payload);
  });
}
