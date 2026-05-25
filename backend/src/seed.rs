use std::path::Path;

const HOME_MD:      &str = include_str!("defaults/Home.md");
const HELP_MD:      &str = include_str!("defaults/Help.md");
const RISOTTO_MD:   &str = include_str!("defaults/Recipes/Mushroom Risotto.md");
const CARBONARA_MD: &str = include_str!("defaults/Recipes/Pasta Carbonara.md");
const LINUX_MD:     &str = include_str!("defaults/Dev/Linux Commands.md");
const GIT_MD:       &str = include_str!("defaults/Dev/Git Cheatsheet.md");
const PRAGPROG_MD:  &str = include_str!("defaults/Books/The Pragmatic Programmer.md");
const ATOMIC_MD:    &str = include_str!("defaults/Books/Atomic Habits.md");
const BLOG_MD:      &str = include_str!("defaults/Projects/Blog Redesign.md");
const LISBON_MD:    &str = include_str!("defaults/Travel/Lisbon 2026.md");
const MAY_MD:       &str = include_str!("defaults/Journal/May 2026.md");

/// Returns true if at least one `.md` file exists under `storage_dir`,
/// excluding hidden directories (`.assets`, `.drawings`, `.git`, …).
fn has_any_notes(storage_dir: &Path) -> bool {
    walkdir::WalkDir::new(storage_dir)
        .into_iter()
        .filter_entry(|e| {
            !e.file_name()
                .to_str()
                .map_or(false, |s| s.starts_with('.'))
        })
        .filter_map(|e| e.ok())
        .any(|e| e.path().extension().and_then(|x| x.to_str()) == Some("md"))
}

pub async fn seed_defaults(storage_dir: &Path) {
    if has_any_notes(storage_dir) {
        // Notes already exist — only ensure the help note is available.
        seed_file(storage_dir, "Help.md", HELP_MD).await;
    } else {
        // Fresh storage — seed all example notes.
        seed_file(storage_dir, "Home.md", HOME_MD).await;
        seed_file(storage_dir, "Help.md", HELP_MD).await;
        seed_file(storage_dir, "Recipes/Mushroom Risotto.md", RISOTTO_MD).await;
        seed_file(storage_dir, "Recipes/Pasta Carbonara.md", CARBONARA_MD).await;
        seed_file(storage_dir, "Dev/Linux Commands.md", LINUX_MD).await;
        seed_file(storage_dir, "Dev/Git Cheatsheet.md", GIT_MD).await;
        seed_file(storage_dir, "Books/The Pragmatic Programmer.md", PRAGPROG_MD).await;
        seed_file(storage_dir, "Books/Atomic Habits.md", ATOMIC_MD).await;
        seed_file(storage_dir, "Projects/Blog Redesign.md", BLOG_MD).await;
        seed_file(storage_dir, "Travel/Lisbon 2026.md", LISBON_MD).await;
        seed_file(storage_dir, "Journal/May 2026.md", MAY_MD).await;
    }
}

async fn seed_file(storage_dir: &Path, filename: &str, content: &str) {
    let path = storage_dir.join(filename);
    if path.exists() {
        return;
    }
    if let Some(parent) = path.parent() {
        if let Err(e) = tokio::fs::create_dir_all(parent).await {
            tracing::warn!("seed: failed to create dir for {filename}: {e}");
            return;
        }
    }
    if let Err(e) = tokio::fs::write(&path, content).await {
        tracing::warn!("seed: failed to write {filename}: {e}");
    }
}
