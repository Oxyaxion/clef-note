# Clef Note — API & CLI

Clef Note exposes a REST API on the same port as the web UI (default `:3000`). It is useful for scripting, automation, or integrating with other tools.

---

## Quick start

### 1. Set up your API key

The API key is defined in `clef-note.toml`. Generate one and add it:

```bash
# Generate a key
openssl rand -hex 32

# Add it to clef-note.toml
# api_key = "the_generated_value"
```

Then restart the backend. The key will appear in **Settings → Security** where you can reveal and copy it.

### 2. Configure your environment

```bash
export CN_URL=http://localhost:3000   # or https://notes.example.com
export CN_KEY=<your-api-key>
```

Add these to your shell profile (`~/.bashrc`, `~/.zshrc`, …) to make them permanent.

---

## `cn` — command-line script

`scripts/cn` is a small Bash script that wraps the API. It requires `curl` and `jq`.

```bash
# Copy to somewhere in your PATH
cp scripts/cn ~/.local/bin/cn
chmod +x ~/.local/bin/cn
```

### Commands

```
cn ls                       List all notes (★ = pinned)
cn get <name>               Print note content to stdout
cn put <name>               Create or overwrite a note from stdin
cn rm  <name>               Delete a note
cn mv  <old> <new>          Rename a note
cn query '<q>'              DSL metadata query
cn search '<text>'          Full-text content search
cn shares                   List all shared links
cn shares rm <slug>         Delete a shared link
cn key                      Show the API key
```

### Examples

```bash
# List all notes
cn ls

# Read a note
cn get "Work/Meeting notes"

# Create or overwrite a note
echo "# Ideas\n\n- something" | cn put "scratch"

# Pipe from a file
cn put "Work/Report" < report.md

# Rename a note
cn mv "scratch" "Notes/scratch"

# Delete
cn rm "Notes/scratch"

# Query with the DSL
cn query "#work status:active"
cn query "area:pro type:meeting recent:5"
cn query "#journal order by date desc"

# Full-text search
cn search "kubernetes"

# List public share links
cn shares

# Delete a share link
cn shares rm my-note-a1b2c3
```

### Environment variables

| Variable | Description | Default |
|---|---|---|
| `CN_URL` | Backend URL | `http://localhost:3000` |
| `CN_KEY` | API key | *(none)* |
| `CLEF_NOTE_KEY` | Fallback for `CN_KEY` | *(none)* |

---

## REST API

All authenticated endpoints require either:
- `Authorization: Bearer <api_key>` header, or
- a valid session cookie (web UI login)

### Notes

| Method | Path | Description |
|---|---|---|
| `GET` | `/notes` | List all notes |
| `GET` | `/notes/{*name}` | Read a note |
| `PUT` | `/notes/{*name}` | Create or overwrite a note |
| `PATCH` | `/notes/{*name}` | Rename a note |
| `DELETE` | `/notes/{*name}` | Delete a note |
| `GET` | `/backlinks/{*name}` | List notes that link to this note |

Note names with `/` are path-separated (e.g. `Work/Meeting`). Each segment must be URL-encoded individually — spaces become `%20`.

```bash
# List all notes
curl -s "$CN_URL/notes" \
  -H "Authorization: Bearer $CN_KEY" | jq '.[].name'

# Read a note
curl -s "$CN_URL/notes/Work%2FMeeting%20notes" \
  -H "Authorization: Bearer $CN_KEY" | jq -r '.content'

# Create or overwrite
curl -s -X PUT "$CN_URL/notes/scratch" \
  -H "Authorization: Bearer $CN_KEY" \
  -H "Content-Type: application/json" \
  -d '{"content": "# Hello\n\nWorld"}'

# Rename
curl -s -X PATCH "$CN_URL/notes/scratch" \
  -H "Authorization: Bearer $CN_KEY" \
  -H "Content-Type: application/json" \
  -d '{"new_name": "Notes/scratch"}'

# Delete
curl -s -X DELETE "$CN_URL/notes/scratch" \
  -H "Authorization: Bearer $CN_KEY"
```

**`GET /notes/{*name}` response:**

```json
{
  "name": "Work/Meeting notes",
  "content": "# Meeting notes\n\nAgenda: …",
  "frontmatter": {
    "title": "Meeting notes",
    "date": "2025-06-01",
    "status": "active",
    "tags": ["work", "meeting"]
  }
}
```

The `content` field contains the note body **without** the YAML frontmatter block.

---

### Search

| Method | Path | Description |
|---|---|---|
| `GET` | `/api/search?q=<text>` | Full-text search |
| `GET` | `/api/query?q=<dsl>` | DSL metadata query |
| `GET` | `/api/tags` | List all tags with note counts |

```bash
# Full-text search
curl -s "$CN_URL/api/search?q=kubernetes" \
  -H "Authorization: Bearer $CN_KEY" | jq '.[].name'

# DSL query
curl -s "$CN_URL/api/query?q=%23work%20status%3Aactive" \
  -H "Authorization: Bearer $CN_KEY" | jq '.[].name'

# All tags
curl -s "$CN_URL/api/tags" \
  -H "Authorization: Bearer $CN_KEY" | jq '.[] | "\(.count)  \(.tag)"' -r
```

---

### Media

| Method | Path | Description |
|---|---|---|
| `POST` | `/assets` | Upload an image (multipart/form-data) |
| `GET` | `/api/assets` | List uploaded images |
| `DELETE` | `/assets/{filename}` | Delete an image |
| `GET` | `/assets/{filename}` | Serve an image *(public, no auth)* |

```bash
# Upload an image
curl -s -X POST "$CN_URL/assets" \
  -H "Authorization: Bearer $CN_KEY" \
  -F "file=@screenshot.png" | jq -r '.url'

# List all assets
curl -s "$CN_URL/api/assets" \
  -H "Authorization: Bearer $CN_KEY" | jq '.[].name'
```

---

### Shared notes (public links)

Shared notes are created from the web UI (`Ctrl+K` → **Share note…**). The public endpoint requires **no authentication**.

| Method | Path | Description |
|---|---|---|
| `GET` | `/api/shared/{slug}` | Shared note as JSON |
| `GET` | `/api/shared/{slug}?raw=1` | Shared note as plain markdown |
| `GET` | `/api/shares` | List all shares *(authenticated)* |
| `POST` | `/api/shares` | Create a share *(authenticated)* |
| `PATCH` | `/api/shares/{slug}` | Update expiry / password *(authenticated)* |
| `DELETE` | `/api/shares/{slug}` | Delete a share *(authenticated)* |

```bash
# Fetch a public shared note as plain markdown
curl "https://notes.example.com/api/shared/my-note-a1b2c3?raw=1"

# With password
curl "https://notes.example.com/api/shared/my-note-a1b2c3?raw=1" \
  -H "X-Share-Password: secret"

# Download to a file
curl -o note.md "https://notes.example.com/api/shared/my-note-a1b2c3?raw=1"

# List your shares
curl -s "$CN_URL/api/shares" \
  -H "Authorization: Bearer $CN_KEY" | jq '.[] | "\(.note)  /shared/\(.slug)"' -r
```

**Status codes for `/api/shared/{slug}`:**

| Code | Meaning |
|---|---|
| `200` | OK |
| `401` | Password required or wrong (send `X-Share-Password` header) |
| `404` | Share not found |
| `410` | Share has expired |

> **Note:** Internal constructs are stripped before serving — wiki links, query blocks, drawing blocks and `![[image]]` embeds are removed or converted to plain text. The original note is never modified.

---

### Git sync

| Method | Path | Description |
|---|---|---|
| `GET` | `/api/sync/status` | Sync status |
| `POST` | `/api/sync` | Trigger an immediate sync |

```bash
curl -s "$CN_URL/api/sync/status" \
  -H "Authorization: Bearer $CN_KEY" | jq '.'

curl -s -X POST "$CN_URL/api/sync" \
  -H "Authorization: Bearer $CN_KEY"
```

---

## Query DSL reference

The DSL is used by `cn query` and by `GET /api/query?q=`. Filters are implicitly AND-ed.

### Filters

| Filter | Example | Description |
|---|---|---|
| Bare word | `rust` | Title or name contains word |
| `#tag` | `#work` | Has tag |
| `title:` | `title:meeting` | Title contains |
| `status:` | `status:active` | Frontmatter status field |
| `area:` | `area:pro` | Frontmatter area field |
| `type:` | `type:meeting` | Frontmatter type field |
| `path:` | `path:Work/` | Note path starts with |
| `depth:` | `depth:2` | Max folder depth |
| `date:` | `date:2025-06` | Frontmatter date (prefix match) |
| `due:` | `due:2025-07-01` | Frontmatter due date |
| `author:` | `author:alice` | Frontmatter author |
| `rating:` | `rating:5` | Frontmatter rating |
| `pinned:` | `pinned:true` | Pinned notes |
| `priority:` | `priority:high` | Frontmatter priority |
| `project:` | `project:clef` | Frontmatter project |

### Logic

```
#work status:active            AND (implicit)
#work OR #perso                OR
NOT status:archived            NOT
(#work OR #perso) NOT archived combine freely
```

### Limiters

```
recent:10                      10 most recently modified notes
oldest:5                       5 least recently modified notes
```

### Sort

```
order by date desc
order by title
order by due
order by priority asc
```

Available sort fields: `name`, `title`, `date`, `modified`, `due`, `status`, `rating`, `area`, `author`, `priority`, `project`

Notes without the sorted field always appear last, regardless of direction.

### Column selection (JSON output only)

```
print title date status
print name tags area author
```

### Practical examples

```
#work status:active order by due
#journal order by date desc recent:20
area:pro NOT status:archived
path:Work/ depth:2 type:meeting date:2025
rating:5 order by title
pinned:true
```
