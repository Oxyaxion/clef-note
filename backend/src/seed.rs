use std::path::Path;

const HOME_MD: &str = include_str!("defaults/Home.md");
const HELP_MD: &str = include_str!("defaults/Help.md");

fn has_any_notes(storage_dir: &Path) -> bool {
    walkdir::WalkDir::new(storage_dir)
        .into_iter()
        .filter_entry(|e| {
            !e.file_name()
                .to_str()
                .is_some_and(|s| s.starts_with('.'))
        })
        .filter_map(|e| e.ok())
        .any(|e| e.path().extension().and_then(|x| x.to_str()) == Some("md"))
}

pub async fn seed_defaults(storage_dir: &Path) {
    if has_any_notes(storage_dir) {
        return;
    }
    seed_file(storage_dir, "Home.md", HOME_MD).await;
    seed_file(storage_dir, "Help.md", HELP_MD).await;
}

async fn seed_file(storage_dir: &Path, filename: &str, content: &str) {
    let path = storage_dir.join(filename);
    if path.exists() {
        return;
    }
    if let Err(e) = tokio::fs::write(&path, content).await {
        tracing::warn!("seed: failed to write {filename}: {e}");
    }
}
