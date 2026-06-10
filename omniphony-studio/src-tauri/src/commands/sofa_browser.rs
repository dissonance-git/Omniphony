//! Browse the sofacoustics.org SOFA database and download HRTF files.
//!
//! The site serves plain Apache directory indexes under a fixed root; we list
//! entries (subdirectories + `.sofa` files), download a chosen file into the
//! app data dir, and the frontend then activates it through the existing
//! `control_hrir_source` command (`sofa:<local path>`).

use std::io::Read;
use std::path::PathBuf;

use tauri::Manager;

/// Fixed browse root. `path` arguments are relative to this and sanitised —
/// the browser can never escape it.
const ROOT: &str = "https://sofacoustics.org/data/";

/// Hard cap on a downloaded file (some database entries are huge ambisonics
/// sets; HRIR files are a few MB).
const MAX_DOWNLOAD_BYTES: u64 = 512 * 1024 * 1024;

#[derive(serde::Serialize)]
pub struct SofaEntry {
    /// Percent-encoded path segment as found in the index (append to the
    /// current path for navigation/download).
    pub href: String,
    /// Human-readable (percent-decoded) name.
    pub name: String,
    pub dir: bool,
    /// Size column as shown by the index ("1.6M", "-" for dirs).
    pub size: String,
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Reject anything that could walk out of the root.
fn sanitize(rel: &str) -> Result<String, String> {
    let decoded = percent_decode(rel);
    if rel.starts_with('/') || rel.contains("://") {
        return Err("invalid path".into());
    }
    for seg in decoded.split('/') {
        if seg == ".." {
            return Err("invalid path".into());
        }
    }
    Ok(rel.to_string())
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(std::time::Duration::from_secs(10))
        .timeout_read(std::time::Duration::from_secs(60))
        .build()
}

fn parse_index(html: &str) -> Vec<SofaEntry> {
    let mut entries = Vec::new();
    for line in html.lines() {
        let Some(h0) = line.find("href=\"") else {
            continue;
        };
        let rest = &line[h0 + 6..];
        let Some(h1) = rest.find('"') else { continue };
        let href = &rest[..h1];
        // Skip sort links, parent/absolute links, and external URLs.
        if href.is_empty()
            || href.starts_with('?')
            || href.starts_with('/')
            || href.starts_with('#')
            || href.contains("://")
            || href == "../"
        {
            continue;
        }
        let dir = href.ends_with('/');
        let name = percent_decode(href.trim_end_matches('/'));
        // Files: only .sofa is loadable; hide the rest (docs, meshes, csv…).
        if !dir && !name.to_ascii_lowercase().ends_with(".sofa") {
            continue;
        }
        // Size = second right-aligned cell of the row (first is the date).
        let mut sizes = line
            .split("<td align=\"right\">")
            .skip(2)
            .map(|c| c.split('<').next().unwrap_or("").trim().to_string());
        let size = sizes.next().unwrap_or_default();
        entries.push(SofaEntry {
            href: href.to_string(),
            name,
            dir,
            size,
        });
    }
    // Directories first, then files, each alphabetically.
    entries.sort_by(|a, b| b.dir.cmp(&a.dir).then(a.name.cmp(&b.name)));
    entries
}

/// List one directory of the SOFA database. `path` is the percent-encoded
/// path relative to the root ("" = root, "database/hutubs/" …).
#[tauri::command]
pub async fn sofa_browse(path: String) -> Result<Vec<SofaEntry>, String> {
    let rel = sanitize(&path)?;
    tauri::async_runtime::spawn_blocking(move || {
        let url = format!("{ROOT}{rel}");
        let body = agent()
            .get(&url)
            .call()
            .map_err(|e| format!("fetch {url}: {e}"))?
            .into_string()
            .map_err(|e| format!("read {url}: {e}"))?;
        Ok(parse_index(&body))
    })
    .await
    .map_err(|e| format!("task failed: {e}"))?
}

/// Download one `.sofa` file into `<app data dir>/hrtf/` and return its local
/// path. The file name flattens the relative path so different databases
/// cannot collide.
#[tauri::command]
pub async fn sofa_download(app: tauri::AppHandle, path: String) -> Result<String, String> {
    let rel = sanitize(&path)?;
    if !percent_decode(&rel).to_ascii_lowercase().ends_with(".sofa") {
        return Err("not a .sofa file".into());
    }
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("no app data dir: {e}"))?
        .join("hrtf");
    tauri::async_runtime::spawn_blocking(move || {
        std::fs::create_dir_all(&dir).map_err(|e| format!("create {dir:?}: {e}"))?;
        let flat = percent_decode(&rel).replace('/', "_");
        let dest: PathBuf = dir.join(flat);
        let url = format!("{ROOT}{rel}");
        let resp = agent()
            .get(&url)
            .call()
            .map_err(|e| format!("fetch {url}: {e}"))?;
        let mut reader = resp.into_reader().take(MAX_DOWNLOAD_BYTES);
        let tmp = dest.with_extension("part");
        let mut file =
            std::fs::File::create(&tmp).map_err(|e| format!("create {tmp:?}: {e}"))?;
        std::io::copy(&mut reader, &mut file).map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            format!("download {url}: {e}")
        })?;
        std::fs::rename(&tmp, &dest).map_err(|e| format!("finalize {dest:?}: {e}"))?;
        Ok(dest.to_string_lossy().into_owned())
    })
    .await
    .map_err(|e| format!("task failed: {e}"))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_apache_index_rows() {
        let html = r#"
<tr><td><img alt="[PARENTDIR]"></td><td><a href="/data/">Parent Directory</a></td><td>&nbsp;</td><td align="right">  - </td></tr>
<tr><td><img alt="[DIR]"></td><td><a href="hutubs/">hutubs/</a></td><td align="right">2020-01-01 10:00  </td><td align="right">  - </td></tr>
<tr><td><img alt="[FILE]"></td><td><a href="pp1_HRIRs_measured.sofa">pp1_HRIRs_measured.sofa</a></td><td align="right">2020-01-01 10:00  </td><td align="right">1.6M</td></tr>
<tr><td><img alt="[FILE]"></td><td><a href="Documentation.pdf">Documentation.pdf</a></td><td align="right">2020-01-01 10:00  </td><td align="right">2M</td></tr>
<tr><th><a href="?C=N;O=D">Name</a></th></tr>
"#;
        let entries = parse_index(html);
        assert_eq!(entries.len(), 2);
        assert!(entries[0].dir && entries[0].name == "hutubs");
        assert!(!entries[1].dir && entries[1].name == "pp1_HRIRs_measured.sofa");
        assert_eq!(entries[1].size, "1.6M");
    }

    #[test]
    fn sanitize_rejects_escapes() {
        assert!(sanitize("../etc/").is_err());
        assert!(sanitize("a/%2e%2e/b").is_err());
        assert!(sanitize("/abs").is_err());
        assert!(sanitize("http://x").is_err());
        assert!(sanitize("database/hutubs/").is_ok());
    }

    #[test]
    fn decodes_percent_names() {
        assert_eq!(percent_decode("aachen%20(high-resolution)"), "aachen (high-resolution)");
    }
}
