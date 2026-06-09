# Partitions

Partitions are independent note workspaces. Each partition lives in its own sub-directory under the `--partitions` root and can optionally be synced to a **different** git repository.

## Creating a partition

Open the sidebar, click the partition name in the top-left, then **New partition**. The server creates the sub-directory and `partition.toml` automatically. No restart required.

You can also create a partition manually by adding a `partition.toml` to any sub-directory and restarting the server:

```toml
# notes/partition.toml
name = "Notes"
```

## Switching partitions

- **Sidebar** — click the partition name in the top-left corner to open the switcher.
- **Ctrl+K** — type `>` to enter command mode, then search for "Switch to:".

## Configuring git sync per partition

Each partition can be synced to a separate remote. The git token is kept in `clef-note.toml` (outside all partition directories) so it is never committed to any repository.

**Step 1 — add the sync block to `partition.toml`** (inside the partition folder, safe to commit):

```toml
# /home/user/clef-notes/work/partition.toml
name = "Work"

[sync]
remote           = "https://github.com/you/work-notes.git"
branch           = "main"
interval_minutes = 30
author_name      = "clef-note"
author_email     = "sync@local"
```

**Step 2 — add the token to `clef-note.toml`** (outside all partitions, never committed):

```toml
# /home/user/clef-note.toml
password = "$argon2id$..."

[partition_tokens]
notes = "ghp_personal_token_xxxx"   # key = partition folder name
work  = "ghp_work_token_yyyy"
```

**Step 3 — restart** — the initial sync for each configured partition runs at startup. Subsequent syncs follow `interval_minutes` or can be triggered manually from **Settings → Git Sync → Sync now**.

## Full example: two partitions, two git remotes

```
/home/user/
  clef-note.toml               ← password + tokens (chmod 600)

/home/user/clef-notes/              ← --partitions root
  notes/
    partition.toml             ← name="Notes", sync→github.com/you/personal-notes
    Journal.md
    .assets/
  work/
    partition.toml             ← name="Work",  sync→github.com/you/work-notes
    Projects/
      Alpha.md
    .assets/
```

```toml
# clef-note.toml
password = "$argon2id$..."

[partition_tokens]
notes = "ghp_personal_xxxx"
work  = "ghp_work_yyyy"
```

```toml
# notes/partition.toml
name = "Notes"

[sync]
remote           = "https://github.com/you/personal-notes.git"
branch           = "main"
interval_minutes = 60
```

```toml
# work/partition.toml
name = "Work"

[sync]
remote           = "https://github.com/you/work-notes.git"
branch           = "main"
interval_minutes = 15
```

The two repositories are fully independent — no cross-contamination, no shared history.
