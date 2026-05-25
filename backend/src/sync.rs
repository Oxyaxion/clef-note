//! Git-based synchronisation with remote repositories.
//! HTTPS + token authentication only (GitHub, Gitea, Forgejo, GitLab).

use std::path::Path;
use std::sync::{Arc, Mutex};

use chrono::Utc;
use git2::{
    build::CheckoutBuilder, Cred, FetchOptions, IndexAddOption, MergeAnalysis, PushOptions,
    RemoteCallbacks, Repository, Signature,
};
use serde::Serialize;
use tracing::{error, info, warn};

use crate::config::SyncConfig;

#[derive(Debug, Clone, Serialize, Default)]
pub struct SyncStatus {
    pub configured: bool,
    pub last_sync_at: Option<String>,
    pub last_error: Option<String>,
}

pub type SharedSyncStatus = Arc<Mutex<SyncStatus>>;

pub fn new_status(configured: bool) -> SharedSyncStatus {
    Arc::new(Mutex::new(SyncStatus { configured, ..Default::default() }))
}

// ── Auth ─────────────────────────────────────────────────────────────────────

fn fetch_opts(token: String) -> FetchOptions<'static> {
    let mut cb = RemoteCallbacks::new();
    cb.credentials(move |_, _, _| Cred::userpass_plaintext("oauth2", &token));
    let mut opts = FetchOptions::new();
    opts.remote_callbacks(cb);
    opts
}

fn push_opts(token: String) -> PushOptions<'static> {
    let mut cb = RemoteCallbacks::new();
    cb.credentials(move |_, _, _| Cred::userpass_plaintext("oauth2", &token));
    let mut opts = PushOptions::new();
    opts.remote_callbacks(cb);
    opts
}

/// Strip the token from any error message before logging or surfacing it.
fn sanitize(msg: &str, token: &str) -> String {
    msg.replace(token, "[REDACTED]")
}

// ── Repo setup ───────────────────────────────────────────────────────────────

fn open_or_init(storage: &Path) -> Result<Repository, git2::Error> {
    Repository::open(storage).or_else(|_| Repository::init(storage))
}

fn ensure_remote(repo: &Repository, url: &str) -> Result<(), git2::Error> {
    match repo.find_remote("origin") {
        Ok(r) if r.url() == Some(url) => {}
        Ok(_) => {
            repo.remote_delete("origin")?;
            repo.remote("origin", url)?;
        }
        Err(_) => {
            repo.remote("origin", url)?;
        }
    }
    Ok(())
}

/// Ensure clef-note.toml is in .gitignore (safety net even if it lives outside storage/).
fn ensure_gitignore(storage: &Path) {
    let path = storage.join(".gitignore");
    let current = std::fs::read_to_string(&path).unwrap_or_default();
    if current.contains("clef-note.toml") {
        return;
    }
    let mut content = current;
    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str("clef-note.toml\n");
    if let Err(e) = std::fs::write(&path, &content) {
        warn!("sync: failed to update .gitignore: {e}");
    }
}

// ── Staging & commits ────────────────────────────────────────────────────────

fn sig(cfg: &SyncConfig) -> Result<Signature<'static>, git2::Error> {
    let name = cfg.author_name.as_deref().unwrap_or("clef-note");
    let email = cfg.author_email.as_deref().unwrap_or("sync@local");
    Signature::now(name, email)
}

/// Stage all changes (equivalent to `git add -A`). Returns true if anything changed vs HEAD.
fn stage_all(repo: &Repository) -> Result<bool, git2::Error> {
    let mut index = repo.index()?;
    index.update_all(["."].iter(), None)?; // mods + deletes of tracked files
    index.add_all(["."].iter(), IndexAddOption::DEFAULT, None)?; // new untracked files
    index.write()?;

    match repo.head() {
        Ok(head) => {
            let index = repo.index()?;
            let head_tree = head.peel_to_tree()?;
            let diff = repo.diff_tree_to_index(Some(&head_tree), Some(&index), None)?;
            Ok(diff.deltas().count() > 0)
        }
        // No HEAD yet (empty repo): any staged file counts as a change.
        Err(_) => Ok(repo.index()?.len() > 0),
    }
}

/// Commit if there are staged changes. Returns Some(oid) on success, None if nothing to commit.
fn commit_if_changed(
    repo: &Repository,
    cfg: &SyncConfig,
    message: &str,
) -> Result<Option<git2::Oid>, git2::Error> {
    if !stage_all(repo)? {
        return Ok(None);
    }
    let sig = sig(cfg)?;
    let mut index = repo.index()?;
    let tree_oid = index.write_tree()?;
    let tree = repo.find_tree(tree_oid)?;
    let oid = match repo.head() {
        Ok(head) => {
            let parent = head.peel_to_commit()?;
            repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&parent])?
        }
        Err(_) => repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[])?,
    };
    Ok(Some(oid))
}

// ── Remote interaction ───────────────────────────────────────────────────────

fn do_fetch(repo: &Repository, cfg: &SyncConfig) -> Result<(), git2::Error> {
    let mut remote = repo.find_remote("origin")?;
    let mut opts = fetch_opts(cfg.token.clone());
    remote.fetch(&[cfg.branch.as_str()], Some(&mut opts), None)?;
    Ok(())
}

fn remote_tip_oid(repo: &Repository, branch: &str) -> Option<git2::Oid> {
    repo.find_reference(&format!("refs/remotes/origin/{branch}"))
        .ok()
        .and_then(|r| r.peel_to_commit().ok())
        .map(|c| c.id())
}

fn apply_fast_forward(
    repo: &Repository,
    branch: &str,
    target_oid: git2::Oid,
) -> Result<(), git2::Error> {
    let target = repo.find_commit(target_oid)?;
    // create or force-update local branch to the remote tip
    repo.branch(branch, &target, true)?;
    repo.set_head(&format!("refs/heads/{branch}"))?;
    repo.checkout_head(Some(CheckoutBuilder::default().force()))?;
    Ok(())
}

fn do_push(repo: &Repository, cfg: &SyncConfig) -> Result<(), git2::Error> {
    let mut remote = repo.find_remote("origin")?;
    let refspec = format!("refs/heads/{}:refs/heads/{}", cfg.branch, cfg.branch);
    let mut opts = push_opts(cfg.token.clone());
    remote.push(&[refspec.as_str()], Some(&mut opts))?;
    Ok(())
}

// ── Conflict resolution ───────────────────────────────────────────────────────

/// Called when `repo.merge()` leaves the index with conflicts.
/// Strategy: keep local ("ours") version of every conflicted file,
/// and write a "Conflict - <name>.md" note so the user can review.
fn resolve_conflicts(
    repo: &Repository,
    cfg: &SyncConfig,
    storage: &Path,
    remote_oid: git2::Oid,
) -> Result<(), git2::Error> {
    struct Entry {
        path: String,
        ours: Vec<u8>,
        theirs: Vec<u8>,
    }

    let entries: Vec<Entry> = {
        let index = repo.index()?;
        index
            .conflicts()?
            .filter_map(|c| c.ok())
            .filter_map(|c| {
                let path_bytes = c.our.as_ref().or(c.their.as_ref())?.path.clone();
                let path = String::from_utf8(path_bytes).ok()?;
                let ours = c
                    .our
                    .as_ref()
                    .and_then(|e| repo.find_blob(e.id).ok())
                    .map(|b| b.content().to_owned())
                    .unwrap_or_default();
                let theirs = c
                    .their
                    .as_ref()
                    .and_then(|e| repo.find_blob(e.id).ok())
                    .map(|b| b.content().to_owned())
                    .unwrap_or_default();
                Some(Entry { path, ours, theirs })
            })
            .collect()
    };

    for entry in &entries {
        // Restore our version on disk (overwrite git's conflict-marker file).
        let full_path = storage.join(&entry.path);
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        std::fs::write(&full_path, &entry.ours).ok();

        // For markdown notes, also write a dedicated conflict note.
        if entry.path.starts_with("notes/") && entry.path.ends_with(".md") {
            let relative = &entry.path["notes/".len()..entry.path.len() - 3];
            let conflict_path = storage
                .join("notes")
                .join(format!("Conflict - {}.md", relative.replace('/', " - ")));
            let content = format!(
                "---\ntags: [conflict]\n---\n\n\
                > Sync conflict on `{relative}`. \
                Local version was kept. Review both versions and delete this note when resolved.\n\n\
                ## Local version\n\n\
                {}\n\n---\n\n## Remote version\n\n\
                {}",
                String::from_utf8_lossy(&entry.ours),
                String::from_utf8_lossy(&entry.theirs),
            );
            std::fs::write(&conflict_path, &content).ok();
        }
    }

    warn!("sync: {} conflict(s) resolved (local kept, conflict notes created)", entries.len());

    // Rebuild the index cleanly from HEAD, then re-stage working directory.
    // This clears all conflict stages (1/2/3) and avoids conflict markers in the tree.
    let head_tree = repo.head()?.peel_to_tree()?;
    let mut index = repo.index()?;
    index.read_tree(&head_tree)?;
    index.update_all(["."].iter(), None)?;
    index.add_all(["."].iter(), IndexAddOption::DEFAULT, None)?;
    index.write()?;

    // Merge commit: two parents (local HEAD + remote tip).
    let sig = sig(cfg)?;
    let head_commit = repo.head()?.peel_to_commit()?;
    let remote_commit = repo.find_commit(remote_oid)?;
    let tree_oid = index.write_tree()?;
    let tree = repo.find_tree(tree_oid)?;
    repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        &format!("sync: merge remote/{} (conflicts resolved)", cfg.branch),
        &tree,
        &[&head_commit, &remote_commit],
    )?;
    repo.cleanup_state()?;

    Ok(())
}

// ── Main sync logic ───────────────────────────────────────────────────────────

fn sync_blocking(cfg: &SyncConfig, storage: &Path) -> Result<(), String> {
    let e = |err: git2::Error| sanitize(&err.to_string(), &cfg.token);

    let repo = open_or_init(storage).map_err(e)?;
    ensure_gitignore(storage);
    ensure_remote(&repo, &cfg.remote).map_err(e)?;

    // 1. Commit any pending local changes before touching remote state.
    let ts = Utc::now().format("%Y-%m-%d %H:%M UTC");
    commit_if_changed(&repo, cfg, &format!("sync: {ts}")).map_err(e)?;

    // 2. Fetch remote.
    do_fetch(&repo, cfg).map_err(|err| sanitize(&err.to_string(), &cfg.token))?;

    // 3. Integrate remote changes.
    if let Some(remote_oid) = remote_tip_oid(&repo, &cfg.branch) {
        let remote_ac = repo.find_annotated_commit(remote_oid).map_err(e)?;
        let (analysis, _) = repo.merge_analysis(&[&remote_ac]).map_err(e)?;

        if analysis.contains(MergeAnalysis::ANALYSIS_UP_TO_DATE) {
            info!("sync: remote is up-to-date");
            // Local may still be ahead — fall through to push.
        } else if analysis.contains(MergeAnalysis::ANALYSIS_FASTFORWARD)
            || analysis.contains(MergeAnalysis::ANALYSIS_UNBORN)
        {
            apply_fast_forward(&repo, &cfg.branch, remote_oid).map_err(e)?;
            info!("sync: fast-forwarded to remote");
        } else if analysis.contains(MergeAnalysis::ANALYSIS_NORMAL) {
            // Merge needed (diverged histories or initial setup with both sides having content).
            info!("sync: merging with remote");
            let remote_ac2 = repo.find_annotated_commit(remote_oid).map_err(e)?;
            repo.merge(&[&remote_ac2], None, None).map_err(e)?;

            if repo.index().map_err(e)?.has_conflicts() {
                resolve_conflicts(&repo, cfg, storage, remote_oid).map_err(e)?;
            } else {
                // Clean merge — just commit.
                commit_if_changed(&repo, cfg, &format!("sync: merge remote/{}", cfg.branch))
                    .map_err(e)?;
                repo.cleanup_state().map_err(e)?;
                info!("sync: clean merge");
            }
        }
    } else {
        info!("sync: remote branch '{}' not found — will push initial content", cfg.branch);
    }

    // 4. Push local commits to remote.
    do_push(&repo, cfg).map_err(|err| sanitize(&err.to_string(), &cfg.token))?;
    info!("sync: pushed to remote/{}", cfg.branch);

    Ok(())
}

pub async fn run_sync(cfg: &SyncConfig, storage: &Path, status: &SharedSyncStatus) {
    let cfg = cfg.clone();
    let storage = storage.to_path_buf();

    let result = tokio::task::spawn_blocking(move || sync_blocking(&cfg, &storage)).await;

    match result {
        Ok(Ok(())) => {
            if let Ok(mut s) = status.lock() {
                s.last_sync_at = Some(Utc::now().to_rfc3339());
                s.last_error = None;
            }
        }
        Ok(Err(msg)) => {
            error!("sync error: {msg}");
            if let Ok(mut s) = status.lock() {
                s.last_error = Some(msg);
            }
        }
        Err(e) => {
            error!("sync task panicked: {e}");
            if let Ok(mut s) = status.lock() {
                s.last_error = Some(format!("internal error: {e}"));
            }
        }
    }
}
