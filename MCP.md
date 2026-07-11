# MCP (Model Context Protocol)

Clef Note exposes a [Model Context Protocol](https://modelcontextprotocol.io) Streamable HTTP endpoint, letting LLM agents (Claude.ai, Claude Code, Gemini, Cursor, …) read and write your notes directly.

## How it works

| Method | Path | Description |
|---|---|---|
| `POST` | `/mcp` | JSON-RPC 2.0 endpoint (single request, batch array, or notification) |

Same port, same HTTPS, same `Authorization: Bearer <api_key>` auth as the rest of the [REST API](API.md) — no extra setup, no new port to expose. All calls operate on the server's **active partition** (see [PARTITIONS.md](PARTITIONS.md)); there is no per-request partition selection.

## Connecting a client

Point your MCP client's integration settings at:

```
https://<your-domain>/mcp
Authorization: Bearer <api_key>
```

The API key is the same one used for the [REST API](API.md#1-set-up-your-api-key) and CLI — generate it in `clef-note.toml`, then find it under **Settings → Security**.

## Tools

| Tool | Arguments | Description |
|---|---|---|
| `list_notes` | *(none)* | Enumerate all note names in the active partition |
| `get_note` | `name` | Read a note's full Markdown, including frontmatter |
| `write_note` | `name`, `content` | Create or overwrite a note; parent folders are created automatically; stamps `lastModified` |
| `search_notes` | `query` | Full-text search, returns note names with context snippets |
| `query_notes` | `query` | Filter notes with the [Query DSL](API.md#query-dsl-reference) |

## Example

```bash
# List available tools
curl -s -X POST "$CN_URL/mcp" \
  -H "Authorization: Bearer $CN_KEY" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"tools/list","id":1}' | jq '.'

# Call a tool
curl -s -X POST "$CN_URL/mcp" \
  -H "Authorization: Bearer $CN_KEY" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"search_notes","arguments":{"query":"kubernetes"}},"id":2}' | jq '.'
```

## Batch requests and notifications

The endpoint also accepts a JSON array of messages (processed in order, responses returned as an array) and JSON-RPC notifications (a message with no `id`), which are acknowledged with `202 Accepted` and no body — per the MCP Streamable HTTP spec.
