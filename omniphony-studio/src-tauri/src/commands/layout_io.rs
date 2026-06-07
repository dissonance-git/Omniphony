//! Layout selection, import/export and the native file-dialog pickers (layouts,
//! evaluation artifacts, bridge & orender executables).
//!
//! Unlike most command modules these touch the shared [`AppState`] layout list
//! and the local filesystem rather than only forwarding over OSC.
//!
//! [`AppState`]: crate::app_state::AppState

use crate::layouts::{self, Layout};
use crate::SharedState;
use rfd::FileDialog;
use std::path::Path;
use tauri::State;

#[tauri::command]
pub fn select_layout(state: State<SharedState>, key: String) -> bool {
    let mut s = state.inner.lock().unwrap();
    let exists = s.layouts.iter().any(|l| l.key == key);
    if exists {
        s.selected_layout_key = Some(key);
    }
    exists
}

#[tauri::command]
pub fn import_layout_from_path(
    state: State<SharedState>,
    path: String,
) -> Result<serde_json::Value, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("empty layout path".to_string());
    }
    let mut layout = layouts::load_layout_file(Path::new(trimmed))
        .ok_or_else(|| "failed to parse layout file".to_string())?;

    let mut s = state.inner.lock().unwrap();
    let base_key = layout.key.clone();
    let mut suffix = 1usize;
    while s.layouts.iter().any(|l| l.key == layout.key) {
        layout.key = format!("{base_key}-{}", suffix);
        suffix += 1;
    }
    s.selected_layout_key = Some(layout.key.clone());
    s.layouts.push(layout);
    s.layouts
        .sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    Ok(serde_json::json!({
        "layouts": s.layouts,
        "selectedLayoutKey": s.selected_layout_key
    }))
}

#[tauri::command]
pub fn pick_import_layout_path() -> Option<String> {
    FileDialog::new()
        .add_filter("Layout", &["json", "yaml", "yml"])
        .pick_file()
        .map(|path| path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn pick_export_layout_path(suggested_name: Option<String>) -> Option<String> {
    let file_name = suggested_name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| {
            let lowered = s.to_ascii_lowercase();
            if lowered.ends_with(".yaml") || lowered.ends_with(".yml") || lowered.ends_with(".json")
            {
                s.to_string()
            } else {
                format!("{s}.yaml")
            }
        })
        .unwrap_or_else(|| "layout.yaml".to_string());

    FileDialog::new()
        .add_filter("Layout YAML", &["yaml", "yml"])
        .add_filter("Layout JSON", &["json"])
        .set_file_name(&file_name)
        .save_file()
        .map(|path| path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn pick_import_evaluation_artifact_path() -> Option<String> {
    FileDialog::new()
        .add_filter("Omniphony evaluator", &["oevl"])
        .pick_file()
        .map(|path| path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn pick_export_evaluation_artifact_path(suggested_name: Option<String>) -> Option<String> {
    let file_name = suggested_name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| {
            let lowered = s.to_ascii_lowercase();
            if lowered.ends_with(".oevl") {
                s.to_string()
            } else {
                format!("{s}.oevl")
            }
        })
        .unwrap_or_else(|| "evaluation.oevl".to_string());

    FileDialog::new()
        .add_filter("Omniphony evaluator", &["oevl"])
        .set_file_name(&file_name)
        .save_file()
        .map(|path| path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn pick_bridge_path() -> Option<String> {
    FileDialog::new()
        .set_title("Select bridge library")
        .pick_file()
        .map(|path| path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn pick_orender_path() -> Option<String> {
    FileDialog::new()
        .set_title("Select orender executable")
        .pick_file()
        .map(|path| path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn export_layout_to_path(path: String, layout: Layout) -> Result<(), String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("empty export path".to_string());
    }

    layouts::save_layout_file(Path::new(trimmed), &layout)
}
