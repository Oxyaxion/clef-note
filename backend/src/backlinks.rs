use std::{collections::HashMap, path::Path, sync::{Arc, OnceLock}};
use axum::{extract::{Path as AxumPath, State}, http::StatusCode, response::Json};
use regex::Regex;
use serde::Serialize;

use crate::{AppState, partitions::ActivePartition};

#[derive(Default)]
pub struct BacklinkIndex {
    // target → sources (notes that link to target)
    reverse: HashMap<String, Vec<String>>,
    // source → targets (what each note links to, for incremental updates)
    forward: HashMap<String, Vec<String>>,
}

fn extract_wikilinks(content: &str) -> Vec<String> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"\[\[([^\]]+)\]\]").unwrap());
    re.captures_iter(content).map(|cap| cap[1].to_string()).collect()
}

impl BacklinkIndex {
    pub async fn build(notes_dir: &Path) -> Self {
        let notes_dir = notes_dir.to_path_buf();
        tokio::task::spawn_blocking(move || {
            let mut index = BacklinkIndex::default();
            for entry in walkdir::WalkDir::new(&notes_dir)
                .into_iter()
                .filter_entry(|e| !e.file_name().to_str().is_some_and(|s| s.starts_with('.')))
                .filter_map(|e| e.ok())
            {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("md") {
                    continue;
                }
                let Ok(rel) = path.strip_prefix(&notes_dir) else { continue };
                let source = rel.with_extension("").to_string_lossy().replace('\\', "/");
                if let Ok(content) = std::fs::read_to_string(path) {
                    let targets = extract_wikilinks(&content);
                    for target in &targets {
                        index.reverse.entry(target.clone()).or_default().push(source.clone());
                    }
                    if !targets.is_empty() {
                        index.forward.insert(source, targets);
                    }
                }
            }
            index
        })
        .await
        .unwrap_or_default()
    }

    /// Incrementally update the index when a single note's content changes.
    pub fn update_note(&mut self, source: &str, content: &str) {
        if let Some(old_targets) = self.forward.remove(source) {
            for target in &old_targets {
                if let Some(sources) = self.reverse.get_mut(target) {
                    sources.retain(|s| s != source);
                }
            }
        }
        let new_targets = extract_wikilinks(content);
        for target in &new_targets {
            self.reverse.entry(target.clone()).or_default().push(source.to_string());
        }
        if !new_targets.is_empty() {
            self.forward.insert(source.to_string(), new_targets);
        }
    }

    /// Remove a deleted note from the index.
    pub fn remove_note(&mut self, source: &str) {
        if let Some(old_targets) = self.forward.remove(source) {
            for target in &old_targets {
                if let Some(sources) = self.reverse.get_mut(target) {
                    sources.retain(|s| s != source);
                }
            }
        }
        self.reverse.remove(source);
    }

    pub fn get(&self, note: &str) -> Vec<String> {
        self.reverse.get(note).cloned().unwrap_or_default()
    }
}

#[derive(Serialize)]
pub struct BacklinksResponse {
    pub note: String,
    pub backlinks: Vec<String>,
}

pub async fn get_backlinks(
    State(_state): State<Arc<AppState>>,
    ActivePartition(vault): ActivePartition,
    AxumPath(name): AxumPath<String>,
) -> Result<Json<BacklinksResponse>, StatusCode> {
    let index = vault.backlink_index.read().await;
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut backlinks: Vec<String> = Vec::new();

    let mut add = |items: Vec<String>| {
        for bl in items {
            if seen.insert(bl.clone()) {
                backlinks.push(bl);
            }
        }
    };

    add(index.get(&name));

    let basename = name.split('/').next_back().unwrap_or(&name);
    if basename != name {
        add(index.get(basename));
    }

    let db = vault.db.clone();
    let name_clone = name.clone();
    let aliases = tokio::task::spawn_blocking(move || db.get_note_aliases(&name_clone))
        .await
        .unwrap_or_default();
    for alias in aliases {
        add(index.get(&alias));
    }

    Ok(Json(BacklinksResponse {
        backlinks,
        note: name,
    }))
}
