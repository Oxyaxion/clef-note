# Partitions

Partitions are independent note workspaces. Each partition lives in its own sub-directory under the `--partitions` root and can optionally be synced to a **different** git repository.

Which sub-directories are partitions is declared in a single **`partition.toml`** at the root of the partitions directory. A sub-directory is only treated as a partition if its slug appears there — directories not listed are ignored.

```toml
# <partitions root>/partition.toml

[notes]
name = "Notes"

[work]
name = "Work"
```

The slug (the `[…]` key) is the sub-directory name; `name` is the display label and defaults to the slug if omitted.

## Creating a partition

Open the sidebar, click the partition name in the top-left, then **New partition**. The server creates the sub-directory and adds an entry to the root `partition.toml` automatically. No restart required.

You can also create one manually by adding a section to the root `partition.toml` and restarting the server. The sub-directory is created on startup if it does not exist yet.

## Switching partitions

- **Sidebar** — click the partition name in the top-left corner to open the switcher.
- **Ctrl+K** — type `>` to enter command mode, then search for "Switch to:".

## Configuring git sync per partition

Each partition can be synced to a separate remote. The git token is kept in `clef-note.toml` (outside all partition directories) so it is never committed to any repository.

**Step 1 — add a `[<slug>.sync]` block to the root `partition.toml`:**

```toml
# <partitions root>/partition.toml

[work]
name = "Work"

[work.sync]
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

[partition_git_tokens]
notes = "ghp_personal_token_xxxx"   # key = partition slug
work  = "ghp_work_token_yyyy"
```

**Step 3 — restart** — the initial sync for each configured partition runs at startup. Subsequent syncs follow `interval_minutes` or can be triggered manually from **Settings → Git Sync → Sync now**.

## Full example: two partitions, two git remotes

```
/home/user/
  clef-note.toml               ← password + tokens (chmod 600)

/home/user/clef-notes/              ← --partitions root
  partition.toml             ← declares the "notes" and "work" partitions
  notes/
    Journal.md
    .assets/
  work/
    Projects/
      Alpha.md
    .assets/
```

```toml
# clef-note.toml
password = "$argon2id$..."

[partition_git_tokens]
notes = "ghp_personal_xxxx"
work  = "ghp_work_yyyy"
```

```toml
# /home/user/clef-notes/partition.toml

[notes]
name = "Notes"

[notes.sync]
remote           = "https://github.com/you/personal-notes.git"
branch           = "main"
interval_minutes = 60

[work]
name = "Work"

[work.sync]
remote           = "https://github.com/you/work-notes.git"
branch           = "main"
interval_minutes = 15
```

The two repositories are fully independent — no cross-contamination, no shared history.
