use std::collections::HashMap;

use serde::Deserialize;

use crate::config::SyncConfig;

/// Config for a single partition, as declared in the root `partition.toml`.
#[derive(Deserialize, Default)]
pub struct PartitionConfig {
    pub name: Option<String>,
    pub sync: Option<SyncConfig>,
}

/// The root manifest: an explicit map of partition slug → config.
/// A sub-directory is treated as a partition only if its slug appears here.
pub type Manifest = HashMap<String, PartitionConfig>;

/// Path to the root manifest, `<root>/partition.toml`.
pub fn manifest_path(root: &std::path::Path) -> std::path::PathBuf {
    root.join("partition.toml")
}

/// Load and parse the root manifest. Returns an empty manifest if the file is
/// missing, and logs a warning (then returns empty) if it fails to parse.
pub fn load_manifest(root: &std::path::Path) -> Manifest {
    let path = manifest_path(root);
    let raw = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return Manifest::default(),
    };
    match toml::from_str(&raw) {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!("failed to parse {}: {e}", path.display());
            Manifest::default()
        }
    }
}

/// One-time migration: if the root manifest is absent but legacy per-directory
/// `partition.toml` files exist, fold them into a single root manifest. The
/// legacy files are left in place (harmless) and no longer read.
pub fn migrate_legacy(root: &std::path::Path) {
    let manifest = manifest_path(root);
    if manifest.exists() {
        return;
    }
    let Ok(rd) = std::fs::read_dir(root) else {
        return;
    };
    let mut found: Vec<(String, PartitionConfig)> = Vec::new();
    for entry in rd.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let slug = match path.file_name().and_then(|n| n.to_str()) {
            Some(s) if !s.starts_with('.') => s.to_string(),
            _ => continue,
        };
        let Ok(raw) = std::fs::read_to_string(path.join("partition.toml")) else {
            continue;
        };
        if let Ok(cfg) = toml::from_str::<PartitionConfig>(&raw) {
            found.push((slug, cfg));
        }
    }
    if found.is_empty() {
        return;
    }
    found.sort_by(|a, b| a.0.cmp(&b.0));

    let mut out =
        String::from("# Partitions manifest — declares which sub-directories are partitions.\n# See PARTITIONS.md. (Auto-migrated from per-directory partition.toml files.)\n");
    for (slug, cfg) in &found {
        let name = cfg.name.clone().unwrap_or_else(|| slug.clone());
        out.push_str(&format!("\n[{slug}]\nname = \"{}\"\n", escape(&name)));
        if let Some(sync) = &cfg.sync {
            out.push_str(&format!("\n[{slug}.sync]\n"));
            out.push_str(&format!("remote = \"{}\"\n", escape(&sync.remote)));
            out.push_str(&format!("branch = \"{}\"\n", escape(&sync.branch)));
            if let Some(i) = sync.interval_minutes {
                out.push_str(&format!("interval_minutes = {i}\n"));
            }
            if let Some(a) = &sync.author_name {
                out.push_str(&format!("author_name = \"{}\"\n", escape(a)));
            }
            if let Some(a) = &sync.author_email {
                out.push_str(&format!("author_email = \"{}\"\n", escape(a)));
            }
        }
    }
    if std::fs::write(&manifest, out).is_ok() {
        tracing::info!(
            "migrated {} legacy partition.toml file(s) into {}",
            found.len(),
            manifest.display()
        );
    }
}

// ── Manifest editing (format-preserving) ────────────────────────────────────
// These operate on the raw file text so hand-written comments and [slug.sync]
// blocks survive create / rename / delete from the UI.

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Return the top-level table key of a header line like `[notes]` or
/// `[notes.sync]`, or `None` if the line is not a regular table header.
fn header_top_key(line: &str) -> Option<&str> {
    let rest = line.trim_start().strip_prefix('[')?;
    if rest.starts_with('[') {
        return None; // array-of-tables [[...]]
    }
    let inner = &rest[..rest.find(']')?];
    Some(inner.split('.').next().unwrap_or("").trim())
}

/// Set (or insert) the `name` of partition `slug` in the manifest `content`,
/// appending a fresh `[slug]` section if it does not yet exist.
pub fn set_name(content: &str, slug: &str, name: &str) -> String {
    let name_line = format!("name = \"{}\"", escape(name));
    let header = format!("[{slug}]");

    let Some(h) = content.lines().position(|l| l.trim() == header) else {
        // No existing section — append a fresh one.
        let mut out = content.to_string();
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&format!("[{slug}]\n{name_line}\n"));
        return out;
    };

    let mut lines: Vec<String> = content.lines().map(str::to_string).collect();
    // The [slug] table body runs until the next table header.
    let end = (h + 1..lines.len())
        .find(|&i| lines[i].trim_start().starts_with('['))
        .unwrap_or(lines.len());
    let name_pos = (h + 1..end).find(|&i| {
        let t = lines[i].trim_start();
        t.starts_with("name") && t["name".len()..].trim_start().starts_with('=')
    });
    match name_pos {
        Some(i) => lines[i] = name_line,
        None => lines.insert(h + 1, name_line),
    }
    let mut out = lines.join("\n");
    if content.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// Remove partition `slug` — its `[slug]` table and any `[slug.*]` sub-tables —
/// from the manifest `content`.
pub fn remove(content: &str, slug: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    let mut skipping = false;
    for line in content.lines() {
        if let Some(top) = header_top_key(line) {
            skipping = top == slug;
        }
        if !skipping {
            out.push(line);
        }
    }
    let mut s = out.join("\n");
    while s.ends_with('\n') {
        s.pop();
    }
    if content.ends_with('\n') && !s.is_empty() {
        s.push('\n');
    }
    s
}
