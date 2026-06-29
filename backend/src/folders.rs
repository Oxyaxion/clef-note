//! Folder rename within the active partition.

use std::sync::Arc;

use axum::{extract::{Path, State}, http::StatusCode, Json};
use regex::Regex;
use serde::Deserialize;

use crate::{AppState, partitions::ActivePartition};

#[derive(Deserialize)]
pub struct RenameFolderBody {
    pub new_path: String,
}

/// PATCH /api/folders/{*path}
///
/// Renames a folder: moves the directory on disk, updates all DB entries whose
/// name starts with the old prefix, and rewrites `[[old/path]]` wiki links in
/// every note.
pub async fn rename_folder(
    State(_state): State<Arc<AppState>>,
    ActivePartition(vault): ActivePartition,
    Path(old_path): Path<String>,
    Json(body): Json<RenameFolderBody>,
) -> Result<StatusCode, StatusCode> {
    let new_path = body.new_path.trim().to_string();

    if !is_safe_path(&old_path) || !is_safe_path(&new_path) || old_path == new_path {
        return Err(StatusCode::BAD_REQUEST);
    }
    // Prevent dropping a folder into one of its own descendants.
    if new_path.starts_with(&format!("{old_path}/")) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let storage = vault.storage_path.clone();
    let old_dir = storage.join(&old_path);
    let new_dir = storage.join(&new_path);

    if !old_dir.is_dir() {
        return Err(StatusCode::NOT_FOUND);
    }
    if new_dir.exists() {
        return Err(StatusCode::CONFLICT);
    }

    if let Some(parent) = new_dir.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(to_500)?;
    }
    tokio::fs::rename(&old_dir, &new_dir).await.map_err(to_500)?;

    let db = vault.db.clone();
    let new_index = tokio::task::spawn_blocking({
        let storage = storage.clone();
        let old_path = old_path.clone();
        let new_path = new_path.clone();
        move || rename_folder_blocking(&storage, &old_path, &new_path, &db)
    })
    .await
    .map_err(to_500)?;

    *vault.backlink_index.write().await = new_index;

    Ok(StatusCode::NO_CONTENT)
}

fn to_500<E: std::fmt::Debug>(e: E) -> StatusCode {
    tracing::error!("internal error: {e:?}");
    StatusCode::INTERNAL_SERVER_ERROR
}

/// A valid folder path: non-empty, no backslashes, no empty/hidden/`..` segments.
pub(crate) fn is_safe_path(path: &str) -> bool {
    !path.is_empty()
        && !path.contains('\\')
        && path
            .split('/')
            .all(|seg| !seg.is_empty() && seg != ".." && !seg.starts_with('.'))
}

/// Update the DB and rewrite wiki links after the directory has already been
/// renamed on disk. Returns a rebuilt backlink index.
fn rename_folder_blocking(
    storage: &std::path::Path,
    old_prefix: &str,
    new_prefix: &str,
    db: &crate::db::Db,
) -> crate::backlinks::BacklinkIndex {
    // Collect DB entries that need renaming (names still carry the old prefix).
    let prefix_slash = format!("{old_prefix}/");
    let old_names: Vec<String> = db
        .list_all_meta()
        .into_iter()
        .filter(|n| n.name.starts_with(&prefix_slash))
        .map(|n| n.name)
        .collect();

    for old_name in &old_names {
        let new_name = format!("{new_prefix}{}", &old_name[old_prefix.len()..]);
        db.rename(old_name, &new_name);
    }

    // Rewrite [[old_prefix/path]] → [[new_prefix/path]] in every note on disk.
    // The trailing '/' in the pattern ensures bare [[old_prefix]] links are not
    // touched (they refer to a note, not the folder we just moved).
    let escaped = regex::escape(old_prefix);
    let re = Regex::new(&format!(r"\[\[({}/[^\]|]*)(\|[^\]]*)?]]", escaped)).ok();

    let mut index = crate::backlinks::BacklinkIndex::default();

    for entry in walkdir::WalkDir::new(storage)
        .into_iter()
        .filter_entry(|e| !e.file_name().to_str().is_some_and(|s| s.starts_with('.')))
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(path) else { continue };
        let Ok(rel) = path.strip_prefix(storage) else { continue };
        let note_name = rel.with_extension("").to_string_lossy().replace('\\', "/");

        let final_content = match &re {
            Some(re) if content.contains(&format!("[[{old_prefix}/")) => {
                let updated = re
                    .replace_all(&content, |caps: &regex::Captures| {
                        let old_link = &caps[1];
                        let alias = caps.get(2).map(|m| m.as_str()).unwrap_or("");
                        let new_link =
                            format!("{new_prefix}{}", &old_link[old_prefix.len()..]);
                        format!("[[{new_link}{alias}]]")
                    })
                    .into_owned();
                if updated != content && std::fs::write(path, &updated).is_ok() {
                    let parsed = crate::frontmatter::parse_note(&updated);
                    db.upsert(&note_name, &parsed, crate::notes::read_mtime(path));
                    updated
                } else {
                    content
                }
            }
            _ => content,
        };

        index.update_note(&note_name, &final_content);
    }

    index
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn tmpdir() -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir()
            .join(format!("cn-folders-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(dir: &Path, rel: &str, content: &str) {
        let path = dir.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content.as_bytes()).unwrap();
    }

    fn read(dir: &Path, rel: &str) -> String {
        std::fs::read_to_string(dir.join(rel)).unwrap()
    }

    fn make_db(notes: &[(&str, &str)]) -> crate::db::Db {
        let db = crate::db::Db::new();
        for (name, content) in notes {
            let parsed = crate::frontmatter::parse_note(content);
            db.upsert(name, &parsed, 0);
        }
        db
    }

    fn db_names(db: &crate::db::Db) -> HashSet<String> {
        db.list_all_meta().into_iter().map(|n| n.name).collect()
    }

    /// Simulate what the async handler does: rename the dir on disk, then run
    /// the blocking half.
    fn run(dir: &Path, old_prefix: &str, new_prefix: &str, db: &crate::db::Db) {
        let old_dir = dir.join(old_prefix);
        let new_dir = dir.join(new_prefix);
        if let Some(p) = new_dir.parent() {
            std::fs::create_dir_all(p).unwrap();
        }
        if old_dir.exists() {
            std::fs::rename(&old_dir, &new_dir).unwrap();
        }
        rename_folder_blocking(dir, old_prefix, new_prefix, db);
    }

    // ── is_safe_path ──────────────────────────────────────────────────────────

    #[test]
    fn safe_path_accepts_simple() {
        assert!(is_safe_path("Dev"));
        assert!(is_safe_path("a-b_c.d"));
    }

    #[test]
    fn safe_path_accepts_nested() {
        assert!(is_safe_path("Dev/Rust"));
        assert!(is_safe_path("a/b/c"));
    }

    #[test]
    fn safe_path_rejects_empty() {
        assert!(!is_safe_path(""));
    }

    #[test]
    fn safe_path_rejects_traversal() {
        assert!(!is_safe_path(".."));
        assert!(!is_safe_path("Dev/../etc"));
        assert!(!is_safe_path("../outside"));
    }

    #[test]
    fn safe_path_rejects_hidden_segments() {
        assert!(!is_safe_path(".git"));
        assert!(!is_safe_path("Dev/.hidden"));
    }

    #[test]
    fn safe_path_rejects_backslash() {
        assert!(!is_safe_path("Dev\\Sub"));
    }

    #[test]
    fn safe_path_rejects_double_slash() {
        assert!(!is_safe_path("Dev//Sub"));
    }

    // ── rename_folder_blocking ────────────────────────────────────────────────

    #[test]
    fn renames_notes_in_db() {
        let dir = tmpdir();
        write(&dir, "Dev/Rust.md", "");
        write(&dir, "Dev/Go.md", "");
        write(&dir, "Other/Note.md", "");
        let db = make_db(&[("Dev/Rust", ""), ("Dev/Go", ""), ("Other/Note", "")]);

        run(&dir, "Dev", "Engineering", &db);

        let names = db_names(&db);
        assert!(names.contains("Engineering/Rust"), "Engineering/Rust should exist");
        assert!(names.contains("Engineering/Go"), "Engineering/Go should exist");
        assert!(names.contains("Other/Note"), "unrelated note unchanged");
        assert!(!names.contains("Dev/Rust"), "Dev/Rust should be gone");
        assert!(!names.contains("Dev/Go"), "Dev/Go should be gone");
    }

    #[test]
    fn rewrites_wiki_links_and_aliases() {
        let dir = tmpdir();
        write(&dir, "Dev/Note.md", "# Note");
        write(&dir, "Ref.md", "See [[Dev/Note]] and [[Dev/Note|alias]].");
        let db = make_db(&[
            ("Dev/Note", "# Note"),
            ("Ref", "See [[Dev/Note]] and [[Dev/Note|alias]]."),
        ]);

        run(&dir, "Dev", "Engineering", &db);

        assert_eq!(
            read(&dir, "Ref.md"),
            "See [[Engineering/Note]] and [[Engineering/Note|alias]]."
        );
        assert!(db_names(&db).contains("Engineering/Note"));
    }

    #[test]
    fn does_not_touch_bare_link_matching_folder_name() {
        let dir = tmpdir();
        write(&dir, "Dev/Note.md", "# Note");
        write(&dir, "Dev.md", "# standalone");
        write(&dir, "Ref.md", "[[Dev]] and [[Dev/Note]]");
        let db = make_db(&[
            ("Dev/Note", "# Note"),
            ("Dev", "# standalone"),
            ("Ref", "[[Dev]] and [[Dev/Note]]"),
        ]);

        run(&dir, "Dev", "Engineering", &db);

        let content = read(&dir, "Ref.md");
        assert!(content.contains("[[Dev]]"), "bare [[Dev]] must be untouched");
        assert!(content.contains("[[Engineering/Note]]"), "folder link must update");
        assert!(db_names(&db).contains("Dev"), "standalone Dev note must stay");
    }

    #[test]
    fn does_not_rewrite_different_folder_with_shared_prefix() {
        let dir = tmpdir();
        write(&dir, "Dev/Note.md", "");
        write(&dir, "DevOps/Note.md", "");
        write(&dir, "Ref.md", "[[Dev/Note]] and [[DevOps/Note]]");
        let db = make_db(&[
            ("Dev/Note", ""),
            ("DevOps/Note", ""),
            ("Ref", "[[Dev/Note]] and [[DevOps/Note]]"),
        ]);

        run(&dir, "Dev", "Engineering", &db);

        let content = read(&dir, "Ref.md");
        assert!(content.contains("[[Engineering/Note]]"), "Dev/ link updated");
        assert!(content.contains("[[DevOps/Note]]"), "DevOps/ link untouched");
        assert!(db_names(&db).contains("DevOps/Note"), "DevOps note unchanged");
    }

    #[test]
    fn handles_nested_folder() {
        let dir = tmpdir();
        write(&dir, "A/B/Note.md", "# Note");
        write(&dir, "Ref.md", "[[A/B/Note]]");
        let db = make_db(&[("A/B/Note", "# Note"), ("Ref", "[[A/B/Note]]")]);

        run(&dir, "A/B", "X/B", &db);

        assert!(db_names(&db).contains("X/B/Note"));
        assert!(!db_names(&db).contains("A/B/Note"));
        assert_eq!(read(&dir, "Ref.md"), "[[X/B/Note]]");
    }

    #[test]
    fn rewrites_multiple_links_in_one_note() {
        let dir = tmpdir();
        write(&dir, "Src/A.md", "");
        write(&dir, "Src/B.md", "");
        write(&dir, "Ref.md", "[[Src/A]] and [[Src/B]]");
        let db = make_db(&[
            ("Src/A", ""),
            ("Src/B", ""),
            ("Ref", "[[Src/A]] and [[Src/B]]"),
        ]);

        run(&dir, "Src", "Dst", &db);

        assert_eq!(read(&dir, "Ref.md"), "[[Dst/A]] and [[Dst/B]]");
    }

    #[test]
    fn notes_in_subfolders_renamed_correctly() {
        let dir = tmpdir();
        write(&dir, "Proj/Sub/Deep.md", "");
        write(&dir, "Ref.md", "[[Proj/Sub/Deep]]");
        let db = make_db(&[("Proj/Sub/Deep", ""), ("Ref", "[[Proj/Sub/Deep]]")]);

        run(&dir, "Proj", "Archive", &db);

        assert!(db_names(&db).contains("Archive/Sub/Deep"));
        assert_eq!(read(&dir, "Ref.md"), "[[Archive/Sub/Deep]]");
    }

    #[test]
    fn notes_outside_folder_unchanged() {
        let dir = tmpdir();
        write(&dir, "Dev/Note.md", "[[Other/Note]]");
        write(&dir, "Other/Note.md", "# Other");
        let db = make_db(&[("Dev/Note", "[[Other/Note]]"), ("Other/Note", "# Other")]);

        run(&dir, "Dev", "Engineering", &db);

        assert!(db_names(&db).contains("Other/Note"), "Other/Note must stay");
        assert_eq!(read(&dir, "Other/Note.md"), "# Other", "Other/Note content unchanged");
    }
}
