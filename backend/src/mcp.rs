// MCP Streamable HTTP handler for clef-note.
//
// Protocol: JSON-RPC 2.0 over HTTP POST (spec 2025-03-26).
// Route:    POST /mcp  — protected by the existing bearer-token middleware.
//
// Claude.ai / Gemini / Cursor connect by pointing their MCP integration at:
//   https://<your-domain>/mcp   (Authorization: Bearer <api_key>)
//
// Tools:
//   list_notes   — enumerate all notes in the active partition
//   get_note     — read a note's full Markdown (including frontmatter)
//   write_note   — create or overwrite a note (stamps lastModified)
//   search_notes — full-text search returning snippets
//   query_notes  — DSL filter (#tag, status:, area:, recent:N, …)

use std::sync::Arc;

use axum::{
    extract::State,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use serde_json::{Value, json};

use crate::{AppState, partitions::{ActivePartition, PartitionState}};

/// `POST /mcp` — entry point wired into the Axum router.
pub async fn handle(
    State(_state): State<Arc<AppState>>,
    ActivePartition(vault): ActivePartition,
    body: String,
) -> Response {
    let msg: Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(e) => return json_resp(json!({
            "jsonrpc": "2.0",
            "error": { "code": -32700, "message": format!("parse error: {e}") },
            "id": null
        })),
    };

    // Batch request (array of messages).
    if let Some(msgs) = msg.as_array() {
        let mut out = Vec::new();
        for m in msgs {
            if m.get("id").is_some() {
                out.push(respond(&vault, m).await);
            }
        }
        return json_resp(json!(out));
    }

    // Notification (no "id") → 202, no body.
    if msg.get("id").is_none() {
        return StatusCode::ACCEPTED.into_response();
    }

    json_resp(respond(&vault, &msg).await)
}

async fn respond(vault: &Arc<PartitionState>, msg: &Value) -> Value {
    let id = msg["id"].clone();
    let method = msg["method"].as_str().unwrap_or("");
    let params = msg.get("params");

    match dispatch(vault, method, params).await {
        Ok(result) => json!({"jsonrpc":"2.0","result":result,"id":id}),
        Err(err)   => json!({"jsonrpc":"2.0","error":{"code":-32000,"message":err},"id":id}),
    }
}

async fn dispatch(
    vault: &Arc<PartitionState>,
    method: &str,
    params: Option<&Value>,
) -> Result<Value, String> {
    match method {
        "initialize" => Ok(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": {} },
            "serverInfo": {
                "name": "clef-note",
                "version": env!("CARGO_PKG_VERSION")
            }
        })),

        "tools/list" => Ok(json!({ "tools": tool_schemas() })),

        "tools/call" => {
            let p = params.ok_or("missing params")?;
            let name = p["name"].as_str().ok_or("missing tool name")?;
            let args = p.get("arguments").unwrap_or(&Value::Null);
            call_tool(vault, name, args).await
        }

        "ping" => Ok(json!({})),

        _ => Err(format!("method not found: {method}")),
    }
}

fn tool_schemas() -> Value {
    json!([
        {
            "name": "list_notes",
            "description": "List all note names in the active vault.",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "get_note",
            "description": "Read the full Markdown content of a note (frontmatter included).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Note path without .md extension (e.g. 'LLM/Intro' or 'Projects/Alpha')"
                    }
                },
                "required": ["name"]
            }
        },
        {
            "name": "write_note",
            "description": "Create or overwrite a note. Parent folders are created automatically. Stamps lastModified in frontmatter.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Note path without .md extension"
                    },
                    "content": {
                        "type": "string",
                        "description": "Full Markdown content (optional YAML frontmatter block at the top)"
                    }
                },
                "required": ["name", "content"]
            }
        },
        {
            "name": "search_notes",
            "description": "Full-text search across all note titles and bodies. Returns note names with context snippets.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Search terms" }
                },
                "required": ["query"]
            }
        },
        {
            "name": "query_notes",
            "description": "Filter notes with the vault DSL. Filters: #tag, tag:, title:, status:, area:, author:, date:, due:, pinned:, locked:, project:, priority:. Logic: implicit AND, OR, NOT. Limiters: recent:N, oldest:N. Sort: 'order by <field> [asc|desc]'.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "DSL expression, e.g. '#rust status:active' or 'area:work order by date desc'"
                    }
                },
                "required": ["query"]
            }
        }
    ])
}

async fn call_tool(vault: &Arc<PartitionState>, name: &str, args: &Value) -> Result<Value, String> {
    match name {
        "list_notes" => {
            let db = vault.db.clone();
            let mut notes = tokio::task::spawn_blocking(move || db.list_all_meta())
                .await
                .map_err(|e| e.to_string())?;
            notes.sort_by(|a, b| a.name.cmp(&b.name));
            let text = notes.iter().map(|n| n.name.as_str()).collect::<Vec<_>>().join("\n");
            text_ok(if text.is_empty() { "(vault is empty)" } else { &text })
        }

        "get_note" => {
            let note_name = str_arg(args, "name")?;
            check_safe(&note_name)?;
            let path = vault.storage_path.join(format!("{note_name}.md"));
            let content = tokio::fs::read_to_string(&path)
                .await
                .map_err(|_| format!("note not found: {note_name}"))?;
            text_ok(&content)
        }

        "write_note" => {
            let note_name = str_arg(args, "name")?;
            let content   = str_arg(args, "content")?;
            check_safe(&note_name)?;

            let path = vault.storage_path.join(format!("{note_name}.md"));
            if let Some(parent) = path.parent() {
                tokio::fs::create_dir_all(parent).await.map_err(|e| e.to_string())?;
            }

            let stamped = crate::frontmatter::stamp_last_modified(&content);
            tokio::fs::write(&path, &stamped).await.map_err(|e| e.to_string())?;

            let mtime = tokio::fs::metadata(&path)
                .await
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);

            let parsed = crate::frontmatter::parse_note(&stamped);
            let db = vault.db.clone();
            let n = note_name.clone();
            tokio::task::spawn_blocking(move || db.upsert(&n, &parsed, mtime)).await.ok();
            vault.backlink_index.write().await.update_note(&note_name, &stamped);

            text_ok(&format!("Saved '{note_name}'."))
        }

        "search_notes" => {
            let query = str_arg(args, "query")?;
            let db = vault.db.clone();
            let results = tokio::task::spawn_blocking(move || db.search(&query))
                .await
                .map_err(|e| e.to_string())?;
            let text = if results.is_empty() {
                "No results.".to_string()
            } else {
                results.iter()
                    .map(|r| format!("[{}] {}", r.name, r.snippet))
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            text_ok(&text)
        }

        "query_notes" => {
            let query = str_arg(args, "query")?;
            let db = vault.db.clone();
            let results = tokio::task::spawn_blocking(move || db.query_notes(&query))
                .await
                .map_err(|e| e.to_string())?;
            if results.is_empty() {
                text_ok("No matching notes.")
            } else {
                text_ok(&serde_json::to_string_pretty(&results).unwrap_or_default())
            }
        }

        _ => Err(format!("unknown tool: {name}")),
    }
}

fn str_arg(args: &Value, key: &str) -> Result<String, String> {
    args.get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("missing argument: {key}"))
}

fn check_safe(name: &str) -> Result<(), String> {
    if crate::notes::is_safe_note_name(name) {
        Ok(())
    } else {
        Err(format!("invalid note name: {name}"))
    }
}

fn text_ok(text: &str) -> Result<Value, String> {
    Ok(json!({ "content": [{ "type": "text", "text": text }] }))
}

fn json_resp(body: Value) -> Response {
    (
        [(header::CONTENT_TYPE, "application/json")],
        serde_json::to_string(&body).unwrap_or_default(),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_partition(dir: &std::path::Path) -> Arc<PartitionState> {
        use crate::{backlinks::BacklinkIndex, db::Db, sync};
        Arc::new(PartitionState {
            slug: "test".into(),
            name: std::sync::RwLock::new("Test".into()),
            storage_path: dir.to_path_buf(),
            db: Arc::new(Db::new()),
            backlink_index: tokio::sync::RwLock::new(BacklinkIndex::default()),
            sync_config: None,
            sync_status: sync::new_status(false),
        })
    }

    #[tokio::test]
    async fn initialize_returns_protocol_version() {
        let dir = tempfile::tempdir().unwrap();
        let p = make_partition(dir.path());
        let r = dispatch(&p, "initialize", None).await.unwrap();
        assert_eq!(r["protocolVersion"], "2024-11-05");
        assert!(r["capabilities"]["tools"].is_object());
    }

    #[tokio::test]
    async fn tools_list_has_all_tools() {
        let dir = tempfile::tempdir().unwrap();
        let p = make_partition(dir.path());
        let r = dispatch(&p, "tools/list", None).await.unwrap();
        let names: Vec<&str> = r["tools"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|t| t["name"].as_str())
            .collect();
        for expected in ["list_notes", "get_note", "write_note", "search_notes", "query_notes"] {
            assert!(names.contains(&expected), "missing tool: {expected}");
        }
    }

    #[tokio::test]
    async fn write_then_get_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let p = make_partition(dir.path());

        let w = call_tool(&p, "write_note", &json!({"name":"LLM/Test","content":"# Hello\nworld"}))
            .await
            .unwrap();
        assert!(w["content"][0]["text"].as_str().unwrap().contains("Saved"));

        let g = call_tool(&p, "get_note", &json!({"name":"LLM/Test"})).await.unwrap();
        let text = g["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("# Hello") && text.contains("world"));
    }

    #[tokio::test]
    async fn list_notes_reflects_writes() {
        let dir = tempfile::tempdir().unwrap();
        let p = make_partition(dir.path());

        call_tool(&p, "write_note", &json!({"name":"A","content":"alpha"})).await.unwrap();
        call_tool(&p, "write_note", &json!({"name":"B","content":"beta"})).await.unwrap();

        let r = call_tool(&p, "list_notes", &json!({})).await.unwrap();
        let text = r["content"][0]["text"].as_str().unwrap();
        assert!(text.contains('A') && text.contains('B'));
    }

    #[tokio::test]
    async fn get_note_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let p = make_partition(dir.path());
        let e = call_tool(&p, "get_note", &json!({"name":"Missing"})).await.unwrap_err();
        assert!(e.contains("not found"));
    }

    #[tokio::test]
    async fn rejects_path_traversal() {
        let dir = tempfile::tempdir().unwrap();
        let p = make_partition(dir.path());
        let e = call_tool(&p, "get_note", &json!({"name":"../secret"})).await.unwrap_err();
        assert!(e.contains("invalid"));
    }

    #[tokio::test]
    async fn unknown_method_errors() {
        let dir = tempfile::tempdir().unwrap();
        let p = make_partition(dir.path());
        assert!(dispatch(&p, "nonexistent", None).await.is_err());
    }

    #[tokio::test]
    async fn ping() {
        let dir = tempfile::tempdir().unwrap();
        let p = make_partition(dir.path());
        assert_eq!(dispatch(&p, "ping", None).await.unwrap(), json!({}));
    }

    #[tokio::test]
    async fn search_notes_no_results() {
        let dir = tempfile::tempdir().unwrap();
        let p = make_partition(dir.path());
        let r = call_tool(&p, "search_notes", &json!({"query":"zzznomatch"})).await.unwrap();
        assert!(r["content"][0]["text"].as_str().unwrap().contains("No results"));
    }
}
