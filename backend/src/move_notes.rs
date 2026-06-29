use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;

use axum::{Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};

use crate::AppState;

/// Record of a single moved note (source name → destination name in the target).
#[derive(Debug, Clone, Serialize)]
pub struct MovedNote {
    pub from: String,
    pub to: String,
}

// ── HTTP handler ────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct MoveRequest {
    /// Slug of the destination partition.
    pub target_slug: String,
    /// Note name (no `.md`) or folder path, relative to the active partition.
    pub source_path: String,
    /// When true, `source_path` is a folder and every note beneath it is moved.
    #[serde(default)]
    pub is_folder: bool,
}

#[derive(Serialize)]
pub struct MoveResponse {
    pub moved: Vec<MovedNote>,
}

type ApiError = (StatusCode, &'static str);

/// Move a note (or a whole folder) from the active partition to another one.
/// The active partition is always the source.
pub async fn move_to_partition(
    State(state): State<Arc<AppState>>,
    Json(req): Json<MoveRequest>,
) -> Result<Json<MoveResponse>, ApiError> {
    if !crate::notes::is_safe_note_name(&req.source_path) {
        return Err((StatusCode::BAD_REQUEST, "invalid source path"));
    }

    let active_slug = state.active_partition.read().await.clone();
    if req.target_slug == active_slug {
        return Err((StatusCode::BAD_REQUEST, "source and target are the same partition"));
    }

    let (source, target) = {
        let partitions = state.partitions.read().await;
        let source = partitions
            .get(&active_slug)
            .cloned()
            .ok_or((StatusCode::INTERNAL_SERVER_ERROR, "active partition not found"))?;
        let target = partitions
            .get(&req.target_slug)
            .cloned()
            .ok_or((StatusCode::NOT_FOUND, "target partition not found"))?;
        (source, target)
    };

    // Build the list of note names to move.
    let notes_to_move: Vec<String> = if req.is_folder {
        enumerate_folder_notes(&source.storage_path, &req.source_path)
    } else {
        let path = source.storage_path.join(format!("{}.md", req.source_path));
        if !path.exists() {
            return Err((StatusCode::NOT_FOUND, "note not found"));
        }
        vec![req.source_path.clone()]
    };
    if notes_to_move.is_empty() {
        return Err((StatusCode::NOT_FOUND, "nothing to move"));
    }

    // Names already present in the target — used to resolve collisions.
    let reserved: HashSet<String> =
        target.db.list_all_meta().into_iter().map(|m| m.name).collect();

    // All filesystem work happens off the async runtime, sequentially, so that
    // `reserved` stays consistent as each note claims its destination name.
    let src_path = source.storage_path.clone();
    let tgt_path = target.storage_path.clone();
    let moved = tokio::task::spawn_blocking(move || {
        let mut reserved = reserved;
        let mut moved = Vec::new();
        for name in notes_to_move {
            match move_one_note(&src_path, &tgt_path, &name, &mut reserved) {
                Ok(m) => moved.push(m),
                Err(e) => tracing::error!("move failed for note '{name}': {e}"),
            }
        }
        moved
    })
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "move task failed"))?;

    // Update both partitions' in-memory indexes for the affected notes.
    for m in &moved {
        source.db.delete(&m.from);
        source.backlink_index.write().await.remove_note(&m.from);

        let path = target.storage_path.join(format!("{}.md", m.to));
        if let Ok(content) = tokio::fs::read_to_string(&path).await {
            let parsed = crate::frontmatter::parse_note(&content);
            target.db.upsert(&m.to, &parsed, crate::notes::read_mtime(&path));
            target.backlink_index.write().await.update_note(&m.to, &content);
        }
    }

    Ok(Json(MoveResponse { moved }))
}

// ── Filesystem operations ───────────────────────────────────────────────────────

/// List note names (relative to `source_dir`, no `.md`) located under `folder/`.
/// Hidden directories (`.assets`, `.drawings`) are skipped, matching `index_dir`.
fn enumerate_folder_notes(source_dir: &Path, folder: &str) -> Vec<String> {
    let base = source_dir.join(folder);
    let mut out = Vec::new();
    for entry in walkdir::WalkDir::new(&base)
        .into_iter()
        .filter_entry(|e| !e.file_name().to_str().is_some_and(|s| s.starts_with('.')))
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        if let Ok(rel) = path.strip_prefix(source_dir) {
            out.push(rel.with_extension("").to_string_lossy().replace('\\', "/"));
        }
    }
    out
}

/// File names (with extension) currently present in `dir`.
fn dir_filenames(dir: &Path) -> HashSet<String> {
    let mut out = HashSet::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            if let Some(name) = entry.file_name().to_str() {
                out.insert(name.to_string());
            }
        }
    }
    out
}

/// Copy an asset into the target `.assets/`. Returns `Some(new_name)` only when
/// a content clash forced a rename; `None` when copied as-is, reused, or missing.
fn copy_asset(src_assets: &Path, dst_assets: &Path, name: &str) -> std::io::Result<Option<String>> {
    let src = src_assets.join(name);
    if !src.exists() {
        return Ok(None);
    }
    let data = std::fs::read(&src)?;
    std::fs::create_dir_all(dst_assets)?;
    let dst = dst_assets.join(name);
    if dst.exists() {
        if std::fs::read(&dst)? == data {
            return Ok(None); // identical — reuse
        }
        let new = next_available_asset_name(&dir_filenames(dst_assets), name);
        std::fs::write(dst_assets.join(&new), &data)?;
        return Ok(Some(new));
    }
    std::fs::write(&dst, &data)?;
    Ok(None)
}

/// Copy a drawing (`.excalidraw` + optional `.svg`) into the target `.drawings/`.
/// Keyed on the drawing's base name. Returns `Some(new_base)` on a content clash.
fn copy_drawing(src_draw: &Path, dst_draw: &Path, base: &str) -> std::io::Result<Option<String>> {
    let src_exc = src_draw.join(format!("{base}.excalidraw"));
    if !src_exc.exists() {
        return Ok(None);
    }
    let exc_data = std::fs::read(&src_exc)?;
    std::fs::create_dir_all(dst_draw)?;
    let dst_exc = dst_draw.join(format!("{base}.excalidraw"));

    let target_base = if dst_exc.exists() {
        if std::fs::read(&dst_exc)? == exc_data {
            return Ok(None); // identical — reuse
        }
        let taken: HashSet<String> = dir_filenames(dst_draw)
            .iter()
            .filter_map(|f| f.strip_suffix(".excalidraw").map(str::to_string))
            .collect();
        next_available_note_name(&taken, base) // single segment → appends _vN
    } else {
        base.to_string()
    };

    std::fs::write(dst_draw.join(format!("{target_base}.excalidraw")), &exc_data)?;
    let src_svg = src_draw.join(format!("{base}.svg"));
    if src_svg.exists() {
        let svg = std::fs::read(&src_svg)?;
        std::fs::write(dst_draw.join(format!("{target_base}.svg")), &svg)?;
    }
    Ok(if target_base != base { Some(target_base) } else { None })
}

/// Move one note from `source_dir` to `target_dir`. `note_name` is relative,
/// without the `.md` extension. `reserved` holds note names already present (or
/// already claimed in this batch) in the target; the chosen destination name is
/// inserted. Referenced assets/drawings are copied to the target (never removed
/// from the source); on a clash with differing content the copy is renamed and
/// the body rewritten. The source `.md` is deleted. Returns the move record.
fn move_one_note(
    source_dir: &Path,
    target_dir: &Path,
    note_name: &str,
    reserved: &mut HashSet<String>,
) -> std::io::Result<MovedNote> {
    let src_note = source_dir.join(format!("{note_name}.md"));
    let content = std::fs::read_to_string(&src_note)?;

    let dest = next_available_note_name(reserved, note_name);
    reserved.insert(dest.clone());

    let (assets, drawings) = referenced_media(&content);

    let mut asset_renames = HashMap::new();
    let src_assets = source_dir.join(".assets");
    let dst_assets = target_dir.join(".assets");
    for name in assets {
        if let Some(new) = copy_asset(&src_assets, &dst_assets, &name)? {
            asset_renames.insert(name, new);
        }
    }

    let mut drawing_renames = HashMap::new();
    let src_draw = source_dir.join(".drawings");
    let dst_draw = target_dir.join(".drawings");
    for base in drawings {
        if let Some(new) = copy_drawing(&src_draw, &dst_draw, &base)? {
            drawing_renames.insert(base, new);
        }
    }

    let new_content = rewrite_refs(&content, &asset_renames, &drawing_renames);

    let dst_note = target_dir.join(format!("{dest}.md"));
    if let Some(parent) = dst_note.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&dst_note, new_content.as_bytes())?;
    std::fs::remove_file(&src_note)?;

    Ok(MovedNote { from: note_name.to_string(), to: dest })
}

// ── Pure helpers (unit-tested) ──────────────────────────────────────────────────

/// Find a free note name in the target partition. If `desired` is not taken,
/// it is returned as-is. Otherwise `_v2`, `_v3`… is appended to the **last path
/// segment** until a free name is found (e.g. `A/B/Note` → `A/B/Note_v2`).
/// Names are relative paths without the `.md` extension.
fn next_available_note_name(existing: &HashSet<String>, desired: &str) -> String {
    if !existing.contains(desired) {
        return desired.to_string();
    }
    let (dir, base) = match desired.rfind('/') {
        Some(i) => (&desired[..=i], &desired[i + 1..]),
        None => ("", desired),
    };
    for n in 2.. {
        let candidate = format!("{dir}{base}_v{n}");
        if !existing.contains(&candidate) {
            return candidate;
        }
    }
    unreachable!()
}

/// Find a free asset filename in the target's `.assets/`. The `_vN` suffix is
/// inserted before the extension (e.g. `image.png` → `image_v2.png`).
fn next_available_asset_name(existing: &HashSet<String>, desired: &str) -> String {
    if !existing.contains(desired) {
        return desired.to_string();
    }
    let (stem, ext) = match desired.rfind('.') {
        Some(i) => (&desired[..i], &desired[i..]), // ext includes the dot
        None => (desired, ""),
    };
    for n in 2.. {
        let candidate = format!("{stem}_v{n}{ext}");
        if !existing.contains(&candidate) {
            return candidate;
        }
    }
    unreachable!()
}

/// Extract referenced media from a note body. Returns `(assets, drawings)` where
/// assets are filenames referenced via `/assets/<name>` and drawings are names
/// referenced via a ```` ```drawing ```` fenced block. Mirrors `Db::get_media_usage`.
fn referenced_media(content: &str) -> (Vec<String>, Vec<String>) {
    use std::sync::OnceLock;
    static ASSET_RE: OnceLock<regex::Regex> = OnceLock::new();
    static DRAWING_RE: OnceLock<regex::Regex> = OnceLock::new();
    let asset_re = ASSET_RE.get_or_init(|| regex::Regex::new(r#"/assets/([^)\s"'\n>]+)"#).unwrap());
    let drawing_re = DRAWING_RE.get_or_init(|| regex::Regex::new(r"(?m)^```drawing\r?\n(\S+)").unwrap());

    let collect = |re: &regex::Regex| -> Vec<String> {
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        for cap in re.captures_iter(content) {
            let v = cap[1].to_string();
            if seen.insert(v.clone()) {
                out.push(v);
            }
        }
        out
    };

    (collect(asset_re), collect(drawing_re))
}

/// Rewrite media references in a note body after assets/drawings were copied to
/// the target under new names. `asset_renames` maps old `/assets/` filenames to
/// new ones; `drawing_renames` maps old drawing block names to new ones. Matching
/// is token-exact (a captured `a.png` never matches `a.png.bak`).
fn rewrite_refs(
    content: &str,
    asset_renames: &HashMap<String, String>,
    drawing_renames: &HashMap<String, String>,
) -> String {
    use std::sync::OnceLock;
    static ASSET_RE: OnceLock<regex::Regex> = OnceLock::new();
    static DRAWING_RE: OnceLock<regex::Regex> = OnceLock::new();
    let asset_re = ASSET_RE.get_or_init(|| regex::Regex::new(r#"/assets/([^)\s"'\n>]+)"#).unwrap());
    let drawing_re = DRAWING_RE.get_or_init(|| regex::Regex::new(r"(?m)^(```drawing\r?\n)(\S+)").unwrap());

    let out = if asset_renames.is_empty() {
        content.to_string()
    } else {
        asset_re
            .replace_all(content, |caps: &regex::Captures| {
                let name = &caps[1];
                let mapped = asset_renames.get(name).map(String::as_str).unwrap_or(name);
                format!("/assets/{mapped}")
            })
            .into_owned()
    };

    if drawing_renames.is_empty() {
        out
    } else {
        drawing_re
            .replace_all(&out, |caps: &regex::Captures| {
                let prefix = &caps[1];
                let name = &caps[2];
                let mapped = drawing_renames.get(name).map(String::as_str).unwrap_or(name);
                format!("{prefix}{mapped}")
            })
            .into_owned()
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn set(names: &[&str]) -> HashSet<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    // ── Filesystem test harness ─────────────────────────────────────────────

    static TMP_COUNTER: AtomicU32 = AtomicU32::new(0);

    /// Create a unique temporary directory for a test.
    fn tmpdir() -> PathBuf {
        let n = TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("cn-move-test-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Write `content` to `dir/rel`, creating parent directories.
    fn write_file(dir: &Path, rel: &str, content: &[u8]) {
        let path = dir.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }

    fn read_file(dir: &Path, rel: &str) -> String {
        std::fs::read_to_string(dir.join(rel)).unwrap()
    }

    #[test]
    fn move_one_note_moves_markdown() {
        let src = tmpdir();
        let dst = tmpdir();
        write_file(&src, "MyNote.md", b"hello");
        let mut reserved = set(&[]);

        let moved = move_one_note(&src, &dst, "MyNote", &mut reserved).unwrap();

        assert_eq!(moved.from, "MyNote");
        assert_eq!(moved.to, "MyNote");
        assert!(!src.join("MyNote.md").exists(), "source note should be deleted");
        assert_eq!(read_file(&dst, "MyNote.md"), "hello");
    }

    #[test]
    fn move_one_note_suffixes_on_collision() {
        let src = tmpdir();
        let dst = tmpdir();
        write_file(&src, "MyNote.md", b"new");
        write_file(&dst, "MyNote.md", b"existing");
        let mut reserved = set(&["MyNote"]);

        let moved = move_one_note(&src, &dst, "MyNote", &mut reserved).unwrap();

        assert_eq!(moved.to, "MyNote_v2");
        assert_eq!(read_file(&dst, "MyNote.md"), "existing", "existing note untouched");
        assert_eq!(read_file(&dst, "MyNote_v2.md"), "new");
    }

    #[test]
    fn move_one_note_copies_referenced_asset_without_deleting_source() {
        let src = tmpdir();
        let dst = tmpdir();
        write_file(&src, "Note.md", b"![x](/assets/pic.png)");
        write_file(&src, ".assets/pic.png", b"PNGDATA");
        let mut reserved = set(&[]);

        move_one_note(&src, &dst, "Note", &mut reserved).unwrap();

        assert_eq!(read_file(&dst, ".assets/pic.png"), "PNGDATA");
        assert!(src.join(".assets/pic.png").exists(), "source asset kept");
        assert_eq!(read_file(&dst, "Note.md"), "![x](/assets/pic.png)");
    }

    #[test]
    fn move_one_note_reuses_identical_target_asset() {
        let src = tmpdir();
        let dst = tmpdir();
        write_file(&src, "Note.md", b"![x](/assets/pic.png)");
        write_file(&src, ".assets/pic.png", b"SAME");
        write_file(&dst, ".assets/pic.png", b"SAME");
        let mut reserved = set(&[]);

        move_one_note(&src, &dst, "Note", &mut reserved).unwrap();

        assert!(!dst.join(".assets/pic_v2.png").exists(), "no rename when content matches");
        assert_eq!(read_file(&dst, "Note.md"), "![x](/assets/pic.png)");
    }

    #[test]
    fn move_one_note_renames_clashing_asset_and_rewrites_body() {
        let src = tmpdir();
        let dst = tmpdir();
        write_file(&src, "Note.md", b"![x](/assets/pic.png)");
        write_file(&src, ".assets/pic.png", b"NEW-CONTENT");
        write_file(&dst, ".assets/pic.png", b"OLD-CONTENT");
        let mut reserved = set(&[]);

        move_one_note(&src, &dst, "Note", &mut reserved).unwrap();

        assert_eq!(read_file(&dst, ".assets/pic.png"), "OLD-CONTENT", "existing asset untouched");
        assert_eq!(read_file(&dst, ".assets/pic_v2.png"), "NEW-CONTENT");
        assert_eq!(read_file(&dst, "Note.md"), "![x](/assets/pic_v2.png)");
    }

    #[test]
    fn move_one_note_copies_drawing_with_preview() {
        let src = tmpdir();
        let dst = tmpdir();
        write_file(&src, "Note.md", b"```drawing\nsketch\n```");
        write_file(&src, ".drawings/sketch.excalidraw", b"{}");
        write_file(&src, ".drawings/sketch.svg", b"<svg/>");
        let mut reserved = set(&[]);

        move_one_note(&src, &dst, "Note", &mut reserved).unwrap();

        assert_eq!(read_file(&dst, ".drawings/sketch.excalidraw"), "{}");
        assert_eq!(read_file(&dst, ".drawings/sketch.svg"), "<svg/>");
    }

    #[test]
    fn enumerate_folder_notes_lists_nested_notes_and_skips_hidden() {
        let src = tmpdir();
        write_file(&src, "A/B/One.md", b"1");
        write_file(&src, "A/B/Two.md", b"2");
        write_file(&src, "A/Other.md", b"o");
        write_file(&src, "Root.md", b"r");
        write_file(&src, "A/B/.assets/img.png", b"i");

        let mut names = enumerate_folder_notes(&src, "A/B");
        names.sort();

        assert_eq!(names, vec!["A/B/One".to_string(), "A/B/Two".to_string()]);
    }

    #[test]
    fn note_name_free_returns_desired() {
        let existing = set(&["Other"]);
        assert_eq!(next_available_note_name(&existing, "MyNote"), "MyNote");
    }

    #[test]
    fn note_name_collision_suffixes_v2() {
        let existing = set(&["MyNote"]);
        assert_eq!(next_available_note_name(&existing, "MyNote"), "MyNote_v2");
    }

    #[test]
    fn note_name_collision_skips_to_v3() {
        let existing = set(&["MyNote", "MyNote_v2"]);
        assert_eq!(next_available_note_name(&existing, "MyNote"), "MyNote_v3");
    }

    #[test]
    fn note_name_suffix_applies_to_last_segment() {
        let existing = set(&["A/B/MyNote"]);
        assert_eq!(next_available_note_name(&existing, "A/B/MyNote"), "A/B/MyNote_v2");
    }

    #[test]
    fn asset_name_free_returns_desired() {
        let existing = set(&["other.png"]);
        assert_eq!(next_available_asset_name(&existing, "image.png"), "image.png");
    }

    #[test]
    fn asset_name_collision_suffixes_before_extension() {
        let existing = set(&["image.png"]);
        assert_eq!(next_available_asset_name(&existing, "image.png"), "image_v2.png");
    }

    #[test]
    fn asset_name_collision_no_extension() {
        let existing = set(&["data"]);
        assert_eq!(next_available_asset_name(&existing, "data"), "data_v2");
    }

    #[test]
    fn asset_name_collision_skips_to_v3() {
        let existing = set(&["image.png", "image_v2.png"]);
        assert_eq!(next_available_asset_name(&existing, "image.png"), "image_v3.png");
    }

    #[test]
    fn referenced_media_finds_assets() {
        let content = "Look ![alt](/assets/pic.png) and ![](/assets/sub/photo.jpg).";
        let (assets, drawings) = referenced_media(content);
        assert_eq!(assets, vec!["pic.png".to_string(), "sub/photo.jpg".to_string()]);
        assert!(drawings.is_empty());
    }

    #[test]
    fn referenced_media_finds_drawings() {
        let content = "Text\n\n```drawing\nmy-sketch\n```\n\nmore";
        let (assets, drawings) = referenced_media(content);
        assert!(assets.is_empty());
        assert_eq!(drawings, vec!["my-sketch".to_string()]);
    }

    #[test]
    fn referenced_media_dedupes() {
        let content = "![a](/assets/pic.png) ![b](/assets/pic.png)";
        let (assets, _) = referenced_media(content);
        assert_eq!(assets, vec!["pic.png".to_string()]);
    }

    fn map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect()
    }

    #[test]
    fn rewrite_refs_empty_maps_is_identity() {
        let content = "![a](/assets/pic.png)\n```drawing\nsketch\n```";
        assert_eq!(rewrite_refs(content, &map(&[]), &map(&[])), content);
    }

    #[test]
    fn rewrite_refs_renames_asset() {
        let content = "![a](/assets/pic.png) and ![b](/assets/keep.png)";
        let out = rewrite_refs(content, &map(&[("pic.png", "pic_v2.png")]), &map(&[]));
        assert_eq!(out, "![a](/assets/pic_v2.png) and ![b](/assets/keep.png)");
    }

    #[test]
    fn rewrite_refs_token_exact_does_not_touch_prefix() {
        let content = "![a](/assets/a.png.bak)";
        let out = rewrite_refs(content, &map(&[("a.png", "a_v2.png")]), &map(&[]));
        assert_eq!(out, content);
    }

    #[test]
    fn rewrite_refs_renames_drawing() {
        let content = "x\n```drawing\nsketch\n```\ny";
        let out = rewrite_refs(content, &map(&[]), &map(&[("sketch", "sketch_v2")]));
        assert_eq!(out, "x\n```drawing\nsketch_v2\n```\ny");
    }
}
