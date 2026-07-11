// MCP Streamable HTTP handler for clef-note.
//
// Protocol: JSON-RPC 2.0 over HTTP POST (spec 2025-03-26).
// Route:    POST /mcp  — protected by the existing bearer-token middleware.
//
// Claude.ai / Gemini / Cursor connect by pointing their MCP integration at:
//   https://<your-domain>/mcp   (Authorization: Bearer <api_key>)
//
// Tools:
//   list_partitions — enumerate partitions and which one is active
//   list_notes      — enumerate all notes in a partition
//   get_note        — read a note's full Markdown (including frontmatter)
//   write_note      — create or overwrite a note (stamps lastModified)
//   search_notes    — full-text search returning snippets
//   query_notes     — DSL filter (#tag, status:, area:, recent:N, …)
//
// All tools except list_partitions accept an optional "partition" argument
// (slug). When omitted, the server's currently active partition is used —
// the same one the web UI's partition switcher controls. That default is
// process-wide mutable state shared with the web UI, so relying on it is a
// race if someone switches partitions concurrently; pass an explicit slug
// for anything beyond a one-off read.

use std::sync::Arc;

use axum::{
    extract::State,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use serde_json::{Value, json};

use crate::{AppState, partitions::PartitionState};

/// `POST /mcp` — entry point wired into the Axum router.
pub async fn handle(State(state): State<Arc<AppState>>, body: String) -> Response {
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
                out.push(respond(&state, m).await);
            }
        }
        return json_resp(json!(out));
    }

    // Notification (no "id") → 202, no body.
    if msg.get("id").is_none() {
        return StatusCode::ACCEPTED.into_response();
    }

    json_resp(respond(&state, &msg).await)
}

async fn respond(state: &Arc<AppState>, msg: &Value) -> Value {
    let id = msg["id"].clone();
    let method = msg["method"].as_str().unwrap_or("");
    let params = msg.get("params");

    match dispatch(state, method, params).await {
        Ok(result) => json!({"jsonrpc":"2.0","result":result,"id":id}),
        Err(err)   => json!({"jsonrpc":"2.0","error":{"code":-32000,"message":err},"id":id}),
    }
}

async fn dispatch(
    state: &Arc<AppState>,
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
            call_tool(state, name, args).await
        }

        "ping" => Ok(json!({})),

        _ => Err(format!("method not found: {method}")),
    }
}

const PARTITION_ARG_DESC: &str = "Partition slug to operate on (see list_partitions). Defaults to the server's currently active partition if omitted.";

fn tool_schemas() -> Value {
    json!([
        {
            "name": "list_partitions",
            "description": "List all partitions (vaults) and mark which one is currently active on the server.",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "list_notes",
            "description": "List all note names in a partition.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "partition": { "type": "string", "description": PARTITION_ARG_DESC }
                }
            }
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
                    },
                    "partition": { "type": "string", "description": PARTITION_ARG_DESC }
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
                    },
                    "partition": { "type": "string", "description": PARTITION_ARG_DESC }
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
                    "query": { "type": "string", "description": "Search terms" },
                    "partition": { "type": "string", "description": PARTITION_ARG_DESC }
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
                    },
                    "partition": { "type": "string", "description": PARTITION_ARG_DESC }
                },
                "required": ["query"]
            }
        }
    ])
}

/// Resolves the partition named by args["partition"], falling back to the
/// server's currently active partition when the argument is absent.
async fn resolve_partition(state: &Arc<AppState>, args: &Value) -> Result<Arc<PartitionState>, String> {
    let slug = match args.get("partition").and_then(Value::as_str) {
        Some(s) => s.to_string(),
        None => state.active_partition.read().await.clone(),
    };
    state.partitions.read().await
        .get(&slug)
        .cloned()
        .ok_or_else(|| format!("unknown partition: {slug}"))
}

async fn call_tool(state: &Arc<AppState>, name: &str, args: &Value) -> Result<Value, String> {
    if name == "list_partitions" {
        let active = state.active_partition.read().await.clone();
        let partitions = state.partitions.read().await;
        let mut infos: Vec<Value> = partitions.values().map(|p| json!({
            "slug": p.slug,
            "name": p.name.read().unwrap().clone(),
            "active": p.slug == active,
        })).collect();
        infos.sort_by(|a, b| a["slug"].as_str().cmp(&b["slug"].as_str()));
        return Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&infos).unwrap_or_default() }] }));
    }

    let vault = resolve_partition(state, args).await?;

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

            text_ok(&format!("Saved '{note_name}' in partition '{}'.", vault.slug))
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
    use std::collections::HashMap;

    async fn make_state(dir: &std::path::Path) -> Arc<AppState> {
        make_state_with_slug(dir, "test").await
    }

    async fn make_state_with_slug(dir: &std::path::Path, slug: &str) -> Arc<AppState> {
        let partition = crate::partitions::init(
            slug.to_string(),
            "Test".to_string(),
            dir.to_path_buf(),
            None,
        ).await;
        let mut map = HashMap::new();
        map.insert(slug.to_string(), Arc::new(partition));

        Arc::new(AppState {
            root_path: dir.to_path_buf(),
            partitions: tokio::sync::RwLock::new(map),
            active_partition: tokio::sync::RwLock::new(slug.to_string()),
            password_hash: String::new(),
            sessions: crate::session::SessionStore::new(),
            api_key: None,
            login_guard: crate::auth::LoginGuard::new(),
            oidc_client: None,
        })
    }

    #[tokio::test]
    async fn initialize_returns_protocol_version() {
        let dir = tempfile::tempdir().unwrap();
        let s = make_state(dir.path()).await;
        let r = dispatch(&s, "initialize", None).await.unwrap();
        assert_eq!(r["protocolVersion"], "2024-11-05");
        assert!(r["capabilities"]["tools"].is_object());
    }

    #[tokio::test]
    async fn tools_list_has_all_tools() {
        let dir = tempfile::tempdir().unwrap();
        let s = make_state(dir.path()).await;
        let r = dispatch(&s, "tools/list", None).await.unwrap();
        let names: Vec<&str> = r["tools"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|t| t["name"].as_str())
            .collect();
        for expected in ["list_partitions", "list_notes", "get_note", "write_note", "search_notes", "query_notes"] {
            assert!(names.contains(&expected), "missing tool: {expected}");
        }
    }

    #[tokio::test]
    async fn write_then_get_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let s = make_state(dir.path()).await;

        let w = call_tool(&s, "write_note", &json!({"name":"LLM/Test","content":"# Hello\nworld"}))
            .await
            .unwrap();
        assert!(w["content"][0]["text"].as_str().unwrap().contains("Saved"));

        let g = call_tool(&s, "get_note", &json!({"name":"LLM/Test"})).await.unwrap();
        let text = g["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("# Hello") && text.contains("world"));
    }

    #[tokio::test]
    async fn list_notes_reflects_writes() {
        let dir = tempfile::tempdir().unwrap();
        let s = make_state(dir.path()).await;

        call_tool(&s, "write_note", &json!({"name":"A","content":"alpha"})).await.unwrap();
        call_tool(&s, "write_note", &json!({"name":"B","content":"beta"})).await.unwrap();

        let r = call_tool(&s, "list_notes", &json!({})).await.unwrap();
        let text = r["content"][0]["text"].as_str().unwrap();
        assert!(text.contains('A') && text.contains('B'));
    }

    #[tokio::test]
    async fn get_note_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let s = make_state(dir.path()).await;
        let e = call_tool(&s, "get_note", &json!({"name":"Missing"})).await.unwrap_err();
        assert!(e.contains("not found"));
    }

    #[tokio::test]
    async fn rejects_path_traversal() {
        let dir = tempfile::tempdir().unwrap();
        let s = make_state(dir.path()).await;
        let e = call_tool(&s, "get_note", &json!({"name":"../secret"})).await.unwrap_err();
        assert!(e.contains("invalid"));
    }

    #[tokio::test]
    async fn unknown_method_errors() {
        let dir = tempfile::tempdir().unwrap();
        let s = make_state(dir.path()).await;
        assert!(dispatch(&s, "nonexistent", None).await.is_err());
    }

    #[tokio::test]
    async fn ping() {
        let dir = tempfile::tempdir().unwrap();
        let s = make_state(dir.path()).await;
        assert_eq!(dispatch(&s, "ping", None).await.unwrap(), json!({}));
    }

    #[tokio::test]
    async fn search_notes_no_results() {
        let dir = tempfile::tempdir().unwrap();
        let s = make_state(dir.path()).await;
        let r = call_tool(&s, "search_notes", &json!({"query":"zzznomatch"})).await.unwrap();
        assert!(r["content"][0]["text"].as_str().unwrap().contains("No results"));
    }

    #[tokio::test]
    async fn list_partitions_marks_active() {
        let dir = tempfile::tempdir().unwrap();
        let s = make_state_with_slug(dir.path(), "notes").await;
        let r = call_tool(&s, "list_partitions", &json!({})).await.unwrap();
        let text = r["content"][0]["text"].as_str().unwrap();
        let parsed: Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed[0]["slug"], "notes");
        assert_eq!(parsed[0]["active"], true);
    }

    #[tokio::test]
    async fn unknown_partition_arg_errors() {
        let dir = tempfile::tempdir().unwrap();
        let s = make_state(dir.path()).await;
        let e = call_tool(&s, "list_notes", &json!({"partition": "nope"})).await.unwrap_err();
        assert!(e.contains("unknown partition"));
    }

    #[tokio::test]
    async fn explicit_partition_overrides_active() {
        let root = tempfile::tempdir().unwrap();
        let a_dir = root.path().join("a");
        let b_dir = root.path().join("b");
        tokio::fs::create_dir_all(&a_dir).await.unwrap();
        tokio::fs::create_dir_all(&b_dir).await.unwrap();

        let pa = crate::partitions::init("a".into(), "A".into(), a_dir, None).await;
        let pb = crate::partitions::init("b".into(), "B".into(), b_dir, None).await;
        let mut map = HashMap::new();
        map.insert("a".to_string(), Arc::new(pa));
        map.insert("b".to_string(), Arc::new(pb));

        let s = Arc::new(AppState {
            root_path: root.path().to_path_buf(),
            partitions: tokio::sync::RwLock::new(map),
            active_partition: tokio::sync::RwLock::new("a".to_string()),
            password_hash: String::new(),
            sessions: crate::session::SessionStore::new(),
            api_key: None,
            login_guard: crate::auth::LoginGuard::new(),
            oidc_client: None,
        });

        call_tool(&s, "write_note", &json!({"name":"OnlyInB","content":"x","partition":"b"})).await.unwrap();

        let default_list = call_tool(&s, "list_notes", &json!({})).await.unwrap();
        assert!(!default_list["content"][0]["text"].as_str().unwrap().contains("OnlyInB"));

        let b_list = call_tool(&s, "list_notes", &json!({"partition":"b"})).await.unwrap();
        assert!(b_list["content"][0]["text"].as_str().unwrap().contains("OnlyInB"));
    }
}
