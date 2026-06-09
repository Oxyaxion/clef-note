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

> **HTTPS / internet exposure** — clef-note speaks plain HTTP only. If you expose it on the internet, put it behind a reverse proxy that handles TLS termination (Caddy, nginx, Traefik, …). Never expose port 3000 directly.

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

The `clef-note` binary serves both the frontend and the API on port `3000` — no Node.js required at runtime. For internet-facing deployments, use a reverse proxy (Caddy, nginx, …) for TLS termination.

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
# [partition_git_tokens]
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

/home/user/clef-notes/      ← --partitions root
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

See **[PARTITIONS.md](PARTITIONS.md)** for the full partitions reference, including creating, switching, and configuring git sync per partition.

### Authentication

See **[AUTHENTICATION.md](AUTHENTICATION.md)** for the full authentication reference, including OIDC setup and provider-specific guides (Authelia, Authentik, Keycloak).

---

## Git Sync

See **[GIT_SYNC.md](GIT_SYNC.md)** for the full git sync reference, including setup, conflict resolution, and security notes.

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

| Shortcut         | Action                                            |
|------------------|---------------------------------------------------|
| `Ctrl+K`         | Open command palette                              |
| `Ctrl+Shift+H`   | Go to home page (set in Settings → General)       |
| `Ctrl+Shift+C`   | Create a new note                                 |
| `Ctrl+Shift+L`   | Navigate back in history                          |
| `Ctrl+Shift+N`   | Navigate forward in history                       |
| `Ctrl+Shift+F`   | Toggle focus mode                                 |
| `Ctrl+Shift+M`   | Toggle markdown source view                       |

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
