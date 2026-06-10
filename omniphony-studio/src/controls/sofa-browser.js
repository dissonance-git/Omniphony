// SOFA HRTF database browser: navigates the Apache index of
// sofacoustics.org/data via the `sofa_browse` Tauri command, downloads a
// chosen .sofa with `sofa_download`, then activates it through the existing
// `control_hrir_source` command (`sofa:<local path>`).
//
// State is one percent-encoded relative path ("" = root). Starts in
// "database/", where the per-subject HRTF sets live.

import { invoke } from '@tauri-apps/api/core';

const el = (id) => document.getElementById(id);

let currentPath = 'database/';
let busy = false;
// Remote path (currentPath + href) of the currently active .sofa, so the
// entry stays visibly marked while scrolling / navigating / reopening.
let activeRemotePath = '';

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
  try {
    const remotePath = currentPath + entry.href;
    const localPath = await invoke('sofa_download', { path: remotePath });
    setStatus('Activating…');
    await invoke('control_hrir_source', { value: `sofa:${localPath}` });
    // Reflect the selection locally; the state broadcast will confirm it.
    const src = el('binauralHrirSource');
    if (src) src.value = 'sofa';
    activeRemotePath = remotePath;
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
    setStatus(String(e), true);
  } finally {
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
    navigate(currentPath);
  });
  if (close) {
    close.addEventListener('click', () => modal.classList.remove('open'));
  }
  modal.addEventListener('click', (e) => {
    if (e.target === modal) modal.classList.remove('open');
  });
}
