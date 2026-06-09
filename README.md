<div align="center">
  <img src="clef-note-logo.png" alt="Clef Note" width="200" />
  <h1>Clef Note</h1>
  <p><em>A fast and lightweight, keyboard-first, self-hosted markdown editor — plain files, live queries, one binary, zero lock-in.</em></p>
</div>

<br />

- Self-hosted, WYSIWYG editor, note-taking backed by plain `.md` files with frontmatter, no database, no lock-in.
- Move your storage folder anywhere, open notes in any editor.
- Clean and minimal UI.
- Minimal footprint: one binary, ~20 MB on disk and just a few dozen megabytes of RAM for 5000 notes.
- Fast search even with thousands of notes.
- Keyboard-first: `/` for blocks, `Ctrl+K` for commands.
- Write live query blocks directly in your notes: `{area:pro status:active order by priority}`, `{path:Work/ depth:2 order by name and not project print name}` — results update in real time.
- All the modern features: export notes / copy-paste images / Excalidraw drawings.
- Responsive for smartphone.
- Read-only mode.
- API to query from the CLI (`scripts/cn`). See [API.md](API.md) for the full reference.
- OpenAI-compatible endpoint to plug in an LLM.
- **Partitions** — organize your notes into independent workspaces, each optionally synced to a different git repository.
- **Shared links** — share any note as a public read-only link, with optional password and expiry.
- **Git sync** — push notes to any GitHub / Gitea / Forgejo / GitLab repository (HTTPS token).
- **OIDC authentication** — delegate login to Authelia, Authentik, Keycloak or any OIDC-compliant provider.

## Road map

- Dashboard system
- Any ideas?

---

## Stack

| Layer    | Technology                         |
|----------|------------------------------------|
| Backend  | Rust + Axum                        |
| Frontend | SvelteKit (vanilla CSS)            |
| Editor   | TipTap (ProseMirror)               |
| Storage  | Flat `.md` files (in-memory index) |

---

## Getting Started

### Download pre-built binary

The easiest way to get started — no Rust or Node.js required.

Download the latest binary for your platform from the [Releases page](https://github.com/Oxyaxion/clef-note/releases):

- `clef-note-linux-x86_64` — Linux
- `clef-note-freebsd-x86_64` — FreeBSD

```bash
chmod +x clef-note-linux-x86_64
./clef-note-linux-x86_64 --config clef-note.toml
```

### Build from source

#### Prerequisites

- [Rust toolchain](https://rustup.rs) — for the backend
- Node.js ≥ 20 — only needed to build the frontend

### Production build

The backend embeds the entire frontend at compile time into a single binary.

```bash
# 1. Build the frontend
cd frontend && npm install && npm run build

# 2. Compile the backend (embeds frontend/build/ automatically)
cd ../backend && cargo build --release

# 3. Run — serves UI + API on http://localhost:3000
./target/release/clef-note --config /path/clef-note.toml
```

No Node.js needed at runtime — `clef-note` is self-contained.

### Development

Run the backend and frontend in two separate terminals:

```bash
# backend (API on http://localhost:3000)
cd backend && cargo run -- --config /path/clef-note.toml --partitions /path/partitions

# frontend (UI on http://localhost:5173, proxies API to :3000)
cd frontend && npm install && npm run dev
```

Open `http://localhost:5173`.

### Run as a systemd service

Create `/etc/systemd/system/clef-note.service`:

```ini
[Unit]
Description=Clef Note
After=network.target

[Service]
User=clef-note
WorkingDirectory=/opt/clef-note
ExecStart=/opt/clef-note/clef-note
Restart=on-failure

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now clef-note
sudo journalctl -u clef-note -f    # follow logs
```

The `clef-note` binary serves both the frontend and the API on port `3000` — no nginx, no Node.js required in production.

### FreeBSD rc.d service

Create `/usr/local/etc/rc.d/clef-note`:

```sh
#!/bin/sh
# PROVIDE: clef-note
# REQUIRE: NETWORKING
# KEYWORD: shutdown

. /etc/rc.subr

name="clef_note"
rcvar="${name}_enable"
procname="/usr/local/sbin/clef-note/clef-note"
pidfile="/var/run/${name}.pid"
clef_note_config="/usr/local/etc/clef-note/clef-note.toml"

load_rc_config ${name}

command="/usr/sbin/daemon"
command_args="-P ${pidfile} -r -f ${procname} --config ${clef_note_config}"

run_rc_command "$1"
```

```bash
chmod +x /usr/local/etc/rc.d/clef-note
echo 'clef_note_enable="YES"' >> /etc/rc.conf
service clef-note start
```

---

## Configuration

All configuration lives in `clef-note.toml`, looked up in the parent of the `--partitions` directory by default. A template is provided at [`clef-note.toml.example`](clef-note.toml.example).

```toml
# Required unless [oidc] is configured — hash with: ./clef-note --hash-password "yourpassword"
password = "$argon2id$v=19$..."

# Optional
# partitions = "/mnt/notes"   # default: ../partitions relative to the binary
# api_key    = ""             # CLI/REST access — openssl rand -hex 32

# Git sync tokens — one entry per partition (folder name = key).
# Kept here, outside all partition directories, so tokens are never
# accidentally committed to a git repository.
# [vault_tokens]
# notes = "ghp_xxxx"
# work  = "ghp_yyyy"

# OIDC — optional. When configured, password login is disabled.
# [oidc]
# issuer_url    = "https://auth.example.com"
# client_id     = "clef-note"
# client_secret = "..."
# redirect_uri  = "https://notes.example.com/auth/oidc/callback"
# allowed_email = "user@example.com"   # restrict to a single user
# provider_name = "Authelia"           # label on the login button (optional)
# disable_password_login = true        # hide password form when OIDC is active (optional)
```

**CLI flags** (override the config file):

```
--partitions <path>          Root directory that contains all partition sub-folders
--port       <port>          Listening port (default: 3000)
--config     <path>          Path to clef-note.toml
--hash-password <plaintext>  Print Argon2 hash and exit
```

### Storage layout

Notes live inside named partition sub-directories. A default `notes` partition is created automatically on first run. The `clef-note.toml` config file sits **outside** all partitions so it is never included in any git repository.

```
/home/user/
  clef-note.toml            ← global config (password, tokens) — never git-tracked

/home/user/notes/           ← --partitions root
  settings.json             ← global UI settings (theme, font, …)
  notes/                    ← partition "Notes" (created on first run)
    partition.toml
    Meeting.md
    Work/
      Project.md
    .assets/
    .drawings/
  work/                     ← partition "Work" (created from the UI)
    partition.toml
    .assets/
    .drawings/
```

Each partition is a completely independent namespace: notes, search results, backlinks, assets and drawings are all scoped to the active partition. Switching partition reloads the entire note list.

To import existing notes into a partition, copy your `.md` files into the partition sub-directory — they appear automatically on next startup.

### Authentication

| Client | Mechanism |
|--------|-----------|
| Web UI — password mode | Password → session token (localStorage, 30-day TTL) |
| Web UI — OIDC mode | Authorization Code + PKCE via external provider |
| CLI (`scripts/cn`) | `CN_KEY` env var = `api_key` from config |

Authentication is **global across all partitions** — one login gives access to every partition. Sessions expire after 30 days or on sign out.

**OIDC mode** — add an `[oidc]` section to `clef-note.toml` (see above). By default both the OIDC button and the password form are shown, so you can keep a fallback. Set `disable_password_login = true` to show the OIDC button only.

Any provider that exposes a standard OIDC discovery endpoint (`/.well-known/openid-configuration`) should work.

#### Authelia setup

Add a client entry to your Authelia configuration:

```yaml
identity_providers:
  oidc:
    clients:
      - client_id: 'clef-note'
        client_name: 'Clef Note'
        client_secret: '$pbkdf2-sha512$...'   # hash with: authelia crypto hash generate pbkdf2 --variant sha512
        public: false
        authorization_policy: 'one_factor'
        require_pkce: true
        pkce_challenge_method: 'S256'
        token_endpoint_auth_method: 'client_secret_basic'   # required — clef-note uses Basic auth by default
        redirect_uris:
          - 'https://notes.example.com/auth/oidc/callback'
        scopes:
          - 'openid'
          - 'profile'
          - 'email'
        grant_types:
          - 'authorization_code'
        response_types:
          - 'code'
        consent_mode: implicit
```

Then in `clef-note.toml`:

```toml
[oidc]
issuer_url    = "https://auth.example.com"
client_id     = "clef-note"
client_secret = "your-plain-text-secret"   # the un-hashed value used above
redirect_uri  = "https://notes.example.com/auth/oidc/callback"
allowed_email = "user@example.com"
provider_name = "Authelia"
disable_password_login = true
```

> **Note:** `token_endpoint_auth_method: 'client_secret_basic'` is required. Authelia defaults to `client_secret_post` for new clients but clef-note (and most OIDC libraries) use `client_secret_basic`, which is the method recommended by RFC 6749.

---

## Partitions

Partitions are independent note workspaces. Each partition lives in its own sub-directory under the `--partitions` root and can optionally be synced to a **different** git repository.

### Creating a partition

Open the sidebar, click the partition name in the top-left, then **New partition**. The server creates the sub-directory and `partition.toml` automatically. No restart required.

You can also create a partition manually by adding a `partition.toml` to any sub-directory and restarting the server:

```toml
# notes/partition.toml
name = "Notes"
```

### Switching partitions

- **Sidebar** — click the partition name in the top-left corner to open the switcher.
- **Ctrl+K** — type `>` to enter command mode, then search for "Switch to:".

### Configuring git sync per partition

Each partition can be synced to a separate remote. The git token is kept in `clef-note.toml` (outside all partition directories) so it is never committed to any repository.

**Step 1 — add the sync block to `partition.toml`** (inside the partition folder, safe to commit):

```toml
# /home/user/notes/work/partition.toml
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

[vault_tokens]
notes = "ghp_personal_token_xxxx"   # key = partition folder name
work  = "ghp_work_token_yyyy"
```

**Step 3 — restart** — the initial sync for each configured partition runs at startup. Subsequent syncs follow `interval_minutes` or can be triggered manually from **Settings → Git Sync → Sync now**.

### Full example: two partitions, two git remotes

```
/home/user/
  clef-note.toml               ← password + tokens (chmod 600)

/home/user/notes/              ← --partitions root
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

[vault_tokens]
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

---

## Git Sync

Clef Note can synchronise a partition's directory with any git remote that supports HTTPS token authentication (GitHub, Gitea, Forgejo, GitLab).

### How it works

1. On every sync cycle, all local changes are committed.
2. Remote changes are fetched and integrated (fast-forward when possible, merge otherwise).
3. **Conflicts** keep the local version. A `Conflict - <note name>.md` file is created so you can review both versions.
4. The result is pushed to the remote.

Sync runs automatically at startup and on the configured interval. It can also be triggered manually from **Settings → Git Sync → Sync now**.

### Setup

**1. Create a token**

- GitHub: Settings → Developer settings → Personal access tokens → Fine-grained → Contents: Read and write
- Gitea / Forgejo: User Settings → Applications → Generate token

**2. Configure the partition** — see [Partitions → Configuring git sync per partition](#configuring-git-sync-per-partition) above.

### Security notes

- Tokens are read from `clef-note.toml` and kept in memory only — they never appear in `.git/config`, commit messages, or logs.
- `clef-note.toml` lives **outside** all partition directories and is therefore never included in any git repository.
- Restrict config file permissions: `chmod 600 clef-note.toml`.

---

## API & CLI

The REST API is documented in **[API.md](API.md)**, including:

- Quick start: getting your API key, environment variables
- `cn` CLI script commands and examples (`ls`, `get`, `put`, `rm`, `mv`, `query`, `search`, `shares`, `key`)
- Full endpoint reference (notes, search, assets, shared notes, git sync)
- Query DSL filter/sort/logic reference

---

## Keyboard Shortcuts

### Navigation

| Shortcut       | Action                                              |
|----------------|-----------------------------------------------------|
| `Ctrl+K`         | Open command palette                          |
| `Ctrl+Shift+H`   | Go to home page (set in Settings → General)   |
| `Ctrl+Shift+P`   | Navigate back in history                      |
| `Ctrl+Shift+N`   | Navigate forward in history                   |

### Editor

| Shortcut       | Action                        |
|----------------|-------------------------------|
| `/`            | Open slash-command menu       |
| `[[`           | Start a WikiLink              |
| `:shortcode:`  | Insert emoji (`:smile:` → 😊) |
| `Ctrl+S`       | Save (auto-save also active)  |
| `Ctrl+B/I`     | Bold / Italic                 |
| `Ctrl+Z`       | Undo                          |

---

## License

MIT
