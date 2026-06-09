# Git Sync

Clef Note can synchronise a partition's directory with any git remote that supports HTTPS token authentication (GitHub, Gitea, Forgejo, GitLab).

## How it works

1. On every sync cycle, all local changes are committed.
2. Remote changes are fetched and integrated (fast-forward when possible, merge otherwise).
3. **Conflicts** keep the local version. A `Conflict - <note name>.md` file is created so you can review both versions.
4. The result is pushed to the remote.

Sync runs automatically at startup and on the configured interval. It can also be triggered manually from **Settings → Git Sync → Sync now**.

## Setup

**1. Create a token**

- GitHub: Settings → Developer settings → Personal access tokens → Fine-grained → Contents: Read and write
- Gitea / Forgejo: User Settings → Applications → Generate token

**2. Configure the partition** — see [PARTITIONS.md → Configuring git sync per partition](PARTITIONS.md#configuring-git-sync-per-partition).

## Security notes

- Tokens are read from `clef-note.toml` and kept in memory only — they never appear in `.git/config`, commit messages, or logs.
- `clef-note.toml` lives **outside** all partition directories and is therefore never included in any git repository.
- Restrict config file permissions: `chmod 600 clef-note.toml`.
