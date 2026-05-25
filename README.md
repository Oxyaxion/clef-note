<div align="center">
  <img src="clef-note-logo.png" alt="Clef Note" width="200" />
  <h1>Clef Note</h1>
  <p><em>A lightweight, fast, queryable, keyboard-oriented markdown note-taking app with a minimal footprint.</em></p>
</div>

<br />

- Self-hosted, WYSIWYG editor, note-taking backed by plain `.md` files with frontmatter, no database, no lock-in.
- Move your `storage/` folder anywhere, open notes in any editor.
- Clean and minimal UI.
- Minimal footprint : one binary, ~15 MB on disk and just a few dozen megabytes of RAM for 5000 notes.
- Fast search even with thousands of notes.
- Keyboard-first: `/` for blocks, `Ctrl+K` for commands.
- Write live query blocks directly in your notes: `{area:pro status:active order by priority}`, `{path:Work/ depth:2 order by name and not project print name}` : Results update in real time.
- All the modern features : Export notes / Copy Paste images / Excalidraw ...
- Responsive for smartphone.
- Read only mode.
- API to queries from the CLI (`scripts/an`).
- OpenAI compatible endpoint to plug with an LLM.

## Road map

- OpenID Authentication
- Dashboard system
- git synchronisation
- Any ideas ?

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
cd backend && cargo run -- --config /path/clef-note.toml --storage /path/storage 

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

name="clef-note"
rcvar="clef-note_enable"
command="/opt/clef-note/clef-note"
pidfile="/var/run/${name}.pid"

load_rc_config $name
run_rc_command "$1"
```

```bash
chmod +x /usr/local/etc/rc.d/clef-note
echo 'clef-note_enable="YES"' >> /etc/rc.conf
service clef-note start
```

---

## Configuration

All configuration lives in `clef-note.toml`, looked up next to the binary by default. A template is provided at [`clef-note.toml.example`](clef-note.toml.example).

```toml
# Required — hash with: ./clef-note --hash-password "yourpassword"
password = "$argon2id$v=19$..."

# Optional
# storage = "/mnt/notes"   # default: ../storage relative to the binary
# api_key = ""             # CLI/REST access — openssl rand -hex 32
```

**CLI flags** (override the config file):

```
--storage <path>             Override the notes storage directory
--port    <port>             Listening port (default: 3000)
--config  <path>             Path to clef-note.toml
--hash-password <plaintext>  Print Argon2 hash and exit
```

### Storage layout

Data lives under the storage directory:

```
storage/
  notes/       ← Markdown files (sub-directories supported)
  assets/      ← uploaded images
  drawings/    ← Excalidraw diagrams (.excalidraw)
```

A note named `work/meeting` is stored as `storage/notes/work/meeting.md`. The folder can be moved freely — no database, no lock-in.

To import existing notes, copy your `.md` files into `storage/notes/` — they will appear automatically on next startup.

### Authentication

| Client | Mechanism |
|--------|-----------|
| Web UI | Password → session token (localStorage, 30-day TTL) |
| CLI (`scripts/an`) | `AN_KEY` env var = `api_key` from config |

The web UI shows a login page on first visit. Sessions expire after 30 days or on sign out.

---

## API Reference

All endpoints are served by the Rust backend on port `3000`.

| Method   | Path                  | Description                          |
|----------|-----------------------|--------------------------------------|
| `GET`    | `/notes`              | List all notes                       |
| `GET`    | `/notes/{*name}`      | Read note (`{name, content, frontmatter}`) |
| `PUT`    | `/notes/{*name}`      | Create or overwrite note (`{content}`) |
| `PATCH`  | `/notes/{*name}`      | Rename note (`{new_name}`)           |
| `DELETE` | `/notes/{*name}`      | Delete note                          |
| `GET`    | `/backlinks/{*name}`  | Backlinks for a note                 |
| `POST`   | `/assets`             | Upload image (multipart), returns `{url}` |
| `GET`    | `/api/search?q=`      | Full-text search                     |
| `GET`    | `/api/query?q=`       | DSL metadata query                   |

### Query DSL examples

Filters are combined with implicit AND. Tokens are whitespace-separated.

```
#work status:active                        → tagged "work" AND status "active"
#work OR #perso                            → tagged "work" or "perso"
NOT status:archived                        → all notes except archived
recent:10                                  → 10 most recently modified notes
oldest:5                                   → 5 least recently modified notes
date:2025-04 type:meeting                  → meetings from April 2025
#journal order by date desc                → journal notes, newest frontmatter date first
recent:10 order by title                   → 10 freshest notes, sorted alphabetically
status:active order by due                 → active tasks sorted by due date
area:pro type:meeting print title date     → meetings with only title and date columns
```

**Filters:** bare word, `#tag`, `tag:`, `title:`, `path:`, `name:`, `depth:`, `status:`, `type:`, `area:`, `author:`, `rating:`, `date:`, `due:`, `lastModified:`, `url:`, `alias:`, `pinned:`, `priority:`, `project:`

**Limiters:** `recent:n`, `oldest:n` — by filesystem modification date

**Sort:** `order by <field> [asc|desc]` — fields: `name`, `title`, `date`, `modified`, `due`, `status`, `rating`, `area`, `author`, `priority`, `project`, `lastModified`

**Columns:** `print <field> [field2 …]` — fields: `name`, `title`, `tags`, `date`, `status`, `area`, `author`, `due`, `rating`, `url`, `priority`, `project`

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
