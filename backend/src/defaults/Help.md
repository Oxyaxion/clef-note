---
alias: Help about queries
tags:
  - help
  - frontmatter
title: Help
---

# Help

## Frontmatter

The frontmatter is the YAML block at the beginning of a note (between the `---`). It stores the note's metadata: tags, status, date, author, etc.

```yaml
---
title: My note
status: active
tags:
  - dev
  - rust
date: 2025-04-28
area: pro
priority: high
project: Clef Note
rating: 4
pinned: false
locked: false
---
```

**Dynamic queries filter notes based on these fields as well as file modification dates.** Each `field:value` in a query maps directly to a frontmatter field.

Available fields: `title`, `status`, `date`, `due`, `type`, `area`, `author`, `rating`, `url`, `alias`, `pinned`, `locked`, `tags`, `priority`, `project`, `lastModified`.

All fields are optional. The `title` field has a special behavior: if the note starts with a `# H1`, it is automatically used as the title (visible in the sidebar and search results). A `title:` manually set in the frontmatter takes priority over the H1.

### Automatic `lastModified` field

If a note has a `lastModified` field in its frontmatter, it is **automatically updated to today's date on every save** (format `YYYY-MM-DD`). No manual action required.

To enable tracking on a note, add the field once:

```yaml
---
title: My note
lastModified: 2026-01-01
---
```

> Notes without frontmatter are not modified. The field is not injected automatically on bare notes.

---

## Dynamic Queries

`{}` blocks (insertable via `/` → **Dynamic Query** or `/?`) filter notes in real time. The same queries work in **Ctrl+K** with the `?` prefix.

> **Bare word** (`Rust`, `meeting`…): searches in the resolved title **and** in the full path. So a file at `Dev/Free-Infra` is found by `free` even if it has a different `title:` in its frontmatter.

> **`today` keyword**: In any value position, `today` is replaced at display time by the current ISO date (`YYYY-MM-DD`). This makes date-relative queries evergreen — e.g. `due<=:today` always means "due by today" without manual updates.

> **Has / not-has a field**: `due:` (empty value) matches all notes that have a `due` date set. `NOT due:` matches notes without one. This works for any prefix field: `NOT date:`, `NOT lastModified:`, etc.

> `date` **vs modification date**: `date:` filters on the date manually written in the frontmatter (event date, publication date…). `recent:` and `oldest:` rely on the file's last modification date on disk — these are two independent things.

> `depth:` counts the number of `/` in the note path. `depth:0` = root-level notes. Combined with `path:`, it restricts to direct children only: `path:Work/Projects/ depth:2` matches `Work/Projects/Alpha` but not `Work/Projects/Alpha/Tasks`. The depth of direct children = number of `/` in the folder prefix.

> `lastModified:` **vs** `recent:`: `lastModified:` filters by a specific date or period (`lastModified:2026-05` = all notes saved in May 2026). `recent:n` gives a relative ranking (the N most recent) with no date constraint — the two are complementary.

> **Tag prefix**: `#prog` finds all notes whose tag starts with `prog` (`programming`, `progress`…).

> `priority` accepts the values `high`, `medium`, `low`. Sorting with `order by priority` uses semantic order (high → medium → low), not alphabetical.

### Filters

| Token | Searches in | Behavior | Example |
| --- | --- | --- | --- |
| `word` | Title **or** filename | Substring | `Rust` `meeting 2025` |
| `#value` | Tags | Prefix | `#prog` → matches `#programming` |
| `tag:value` | Tags | Prefix | `tag:prog` |
| `title:value` | Frontmatter title → H1 → filename (last segment) | Substring | `title:meeting` |
| `path:value` | Full file path | Substring | `path:Work/Projects` |
| `name:value` | Filename only (last segment) | Substring | `name:meeting` |
| `depth:n` | Number of `/` in the path | Exact | `depth:0` `depth:3` |
| `status:value` | Frontmatter `status` | Exact | `status:active` |
| `type:value` | Frontmatter `type` | Exact | `type:note` `type:book` |
| `area:value` | Frontmatter `area` | Substring | `area:pro` |
| `priority:value` | Frontmatter `priority` | Exact | `priority:high` |
| `project:value` | Frontmatter `project` | Substring | `project:Clef` |
| `author:value` | Frontmatter `author` | Substring | `author:Smith` |
| `rating:n` | Frontmatter `rating` | Exact | `rating:5` |
| `date:prefix` | Frontmatter `date` (semantic date, written by the user) | Prefix | `date:2025` `date:2025-04` |
| `date>=:YYYY-MM-DD` | Frontmatter `date` | On or after | `date>=:2026-01-01` |
| `date<=:YYYY-MM-DD` | Frontmatter `date` | On or before | `date<=:2026-12-31` |
| `due:prefix` | Frontmatter `due` | Prefix | `due:2025-05` |
| `due>=:YYYY-MM-DD` | Frontmatter `due` | On or after | `due>=:2026-06-01` |
| `due<=:YYYY-MM-DD` | Frontmatter `due` | On or before | `due<=:2026-06-30` |
| `url:value` | Frontmatter `url` | Substring | `url:github.com` |
| `alias:value` | Frontmatter `aliases` | Exact | `alias:my-shortcut` |
| `pinned:true/false` | Frontmatter `pinned` | Exact | `pinned:true` |
| `locked:true/false` | Frontmatter `locked` | Exact | `locked:true` |
| `lastModified:prefix` | Frontmatter `lastModified` (auto-updated on save) | Prefix | `lastModified:2026-05` |
| `recent:n` | **File modification** date | The n most recent | `recent:10` |
| `oldest:n` | **File modification** date | The n oldest | `oldest:5` |

---

### Boolean Operators

Tokens are implicitly combined with **AND**.

```
#work status:active
```

↳ notes with the *work* tag **and** the *active* status

```
#work OR #personal
```

↳ notes with *work* **or** *personal*

```
NOT status:archived
```

↳ all notes except archived ones

```
#work OR #personal AND NOT status:done
```

↳ AND has higher precedence than OR: `#work OR (#personal AND NOT status:done)`

---

### Sorting — `order by`

```
order by <field> [asc|desc|reverse]
```

Sorts results by a field. The default direction is `asc`. `reverse` is an alias for `desc`.

| Field | Sorts on |
| --- | --- |
| `name` | Filename (full path) |
| `title` | Resolved title (frontmatter → H1 → name) |
| `date` | Frontmatter `date` (semantic date) |
| `modified` | File modification date |
| `due` | Frontmatter `due` |
| `status` | Frontmatter `status` |
| `rating` | Frontmatter `rating` |
| `area` | Frontmatter `area` |
| `author` | Frontmatter `author` |
| `priority` | Frontmatter `priority` (high → medium → low) |
| `project` | Frontmatter `project` |
| `lastModified` | Frontmatter `lastModified` (auto-updated on save) |

Notes without a value for the sorted field appear last, regardless of direction.

**Combined with** `recent:n` **/** `oldest:n`: `recent:` or `oldest:` selects the N notes first (by modification date), then `order by` re-sorts that subset.

---

### Display — `print`

```
... print <field> [field2 ...]
```

Controls which columns are shown in results. Default: title, path, and metadata (tags, date, status).

Available fields: `name`, `title`, `tags`, `date`, `status`, `area`, `author`, `due`, `rating`, `url`, `priority`, `project`

The first navigable field (`name` or `title`) becomes the clickable link. The others are displayed as metadata. `print` must always be placed **last** in the query.

---

### Special Queries

| Query | Result |
| --- | --- |
| `#` | Cloud of all tags (click to filter) |
| `status:` | All existing status values |
| `area:` | Same for any frontmatter field |

Typing `field:` without a value displays existing values as clickable chips.

---

### Complete Examples

```
type:meeting date:2025-04
```

↳ All meetings in April 2025

```
#prog Rust
```

↳ Notes tagged `prog*` whose title **or filename** contains `Rust`

```
status:todo OR status:active NOT #someday
```

↳ Ongoing tasks, without the *someday* tag

```
recent:10
```

↳ The 10 most recently modified notes

```
priority:high area:pro
```

↳ Urgent work items

```
recent:10 order by title
```

↳ The 10 most recent notes, sorted alphabetically by title

```
#journal order by date desc
```

↳ All notes tagged *journal*, from most recent to oldest (frontmatter date)

```
status:active order by priority
```

↳ Active tasks sorted by priority (urgent first)

```
due<=:today NOT status:done order by due asc
```

↳ Overdue or due today, not done — `today` updates automatically every day

```
due>=:today NOT status:done order by due asc
```

↳ Upcoming tasks from today, sorted by nearest deadline

```
due>=:today due<=:2026-06-30 NOT status:done order by priority
```

↳ Tasks due this month, not done, sorted by priority

```
NOT due: status:todo
```

↳ Todo items that have no deadline yet — the backlog to schedule

```
date:2026-06 order by date asc
```

↳ All events/notes in June 2026, sorted chronologically

```
area:pro type:meeting recent:20 order by date desc print title date
```

↳ The 20 most recent pro meetings, sorted by date, displaying only title and date

```
path:Work/Company/Finance/ depth:3 order by name
```

↳ Direct children of `Work/Company/Finance/` only (excludes sub-folders)

---

### Index Pages

Setting `type: index` in the frontmatter turns a note into a **dashboard page**.

```yaml
---
title: Work
type: index
---
```

Index pages have two visual differences:

- **Sidebar**: they appear in a dedicated section at the top, above the regular note tree, with a grid icon (⊞).
- **Editor**: the H1 title is centered and larger; dynamic query blocks are displayed in a **two-column grid** instead of a single vertical list.

Everything else stays standard Markdown — you can freely mix headings, text, and `{}` query blocks. The file is stored and edited like any other note.

**Typical layout:**

```
---
title: Work
type: index
---

# Work

## Active projects
{project: status:active order by priority}

## Recent notes
{recent:10 order by title}
```

Query blocks placed consecutively flow into the two-column grid; headings and paragraphs always span the full width.

---

### Locking a Note (Read-Only)

The padlock icon in the title bar toggles a note between **editable** and **read-only** mode.

- Click the open padlock → the note is locked. `locked: true` is written to the frontmatter and the editor becomes non-editable.
- Click the closed padlock (orange) → the note is unlocked and editable again.

The locked state is stored directly in the frontmatter, so it persists across sessions and is queryable:

```
locked:true                          → all locked notes
locked:false                         → all editable notes
locked:true order by lastModified    → locked notes, most recently saved first
```

---

## Keyboard Shortcuts

### Navigation

| Shortcut | Action |
| --- | --- |
| `Ctrl+K` | Command palette (search, export, theme…) |
| `Ctrl+Shift+H` | Go to home page (set in Settings → General) |
| `Ctrl+Shift+P` | Navigate back in history |
| `Ctrl+Shift+N` | Navigate forward in history |

### Editor

| Shortcut | Action |
| --- | --- |
| `/` | Block menu (headings, lists, code, query…) |
| `/?` | Insert a dynamic query block directly |
| `[[` | Wiki link to another note |
| `:shortcode:` | Insert emoji — `:smile:` → 😊, `:rocket:` → 🚀 |
| `Ctrl+Enter` | Exit current blockquote or code block |
| `Ctrl+S` | Save (auto-save is also active) |
| `Ctrl+B / I` | Bold / Italic |
| `Ctrl+Z` | Undo |

---

## Emoji Shortcodes

Type `:shortcode:` anywhere in the editor and it is replaced automatically when you close it with `:`.

You can also search shortcodes via the `/` slash-command menu (type `/smile` for example).

**Smileys & faces**

| Shortcode | Emoji | | Shortcode | Emoji |
| --- | --- | --- | --- | --- |
| `:smile:` | 😊 | | `:grin:` | 😁 |
| `:laughing:` | 😆 | | `:joy:` | 😂 |
| `:wink:` | 😉 | | `:heart_eyes:` | 😍 |
| `:thinking:` | 🤔 | | `:cry:` | 😢 |
| `:angry:` | 😠 | | `:scream:` | 😱 |
| `:cool:` | 😎 | | `:nerd:` | 🤓 |
| `:partying:` | 🥳 | | `:sleeping:` | 😴 |
| `:ghost:` | 👻 | | `:robot:` | 🤖 |

**Status & actions**

| Shortcode | Emoji | | Shortcode | Emoji |
| --- | --- | --- | --- | --- |
| `:check:` / `:done:` | ✅ | | `:x:` / `:no:` | ❌ |
| `:warning:` | ⚠️ | | `:question:` | ❓ |
| `:bulb:` / `:idea:` | 💡 | | `:fire:` | 🔥 |
| `:rocket:` | 🚀 | | `:star:` | ⭐ |
| `:tada:` / `:party:` | 🎉 | | `:trophy:` | 🏆 |
| `:heart:` | ❤️ | | `:thumbsup:` | 👍 |
| `:bug:` | 🐛 | | `:zap:` | ⚡ |
| `:clock:` | ⏰ | | `:calendar:` | 📅 |

---

## Date Format

The format used by `/date` and `/timestamp` can be configured in **Settings → General → Date format**:

| Option | Example |
| --- | --- |
| Full | Wednesday, December 25, 2025 |
| European | 25/12/2025 |
| ISO | 2025-12-25 |
| American | 12/25/2025 |
