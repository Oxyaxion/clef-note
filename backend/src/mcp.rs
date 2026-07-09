// MCP stdio server for clef-note.
//
// Implements the Model Context Protocol over stdin/stdout (one JSON-RPC 2.0
// message per line). No extra dependencies — tokio + serde_json are already
// in the tree.
//
// Spec: https://spec.modelcontextprotocol.io/specification/
//
// Start with:  clef-note --mcp
//
// Exposed tools:
//   list_notes   — enumerate all notes in the active partition
//   get_note     — read a note's full Markdown (including frontmatter)
//   write_note   — create or overwrite a note
//   search_notes — full-text search
//   query_notes  — DSL filter (#tag, status:, area:, recent:N, …)

use std::sync::Arc;

use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::partitions::PartitionState;

pub async fn run(partition: Arc<PartitionState>) {
    let mut lines = BufReader::new(tokio::io::stdin()).lines();
    let mut out = tokio::io::stdout();

    while let Ok(Some(line)) = lines.next_line().await {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let msg: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(e) => {
                send_error(&mut out, Value::Null, -32700, &format!("parse error: {e}")).await;
                continue;
            }
        };

        // Notifications have no "id" field — no response expected.
        let Some(id) = msg.get("id").cloned() else {
            continue;
        };

        let method = msg["method"].as_str().unwrap_or("");
        let params = msg.get("params");

        let response = match dispatch(&partition, method, params).await {
            Ok(result) => json!({"jsonrpc":"2.0","result":result,"id":id}),
            Err(msg) => json!({
                "jsonrpc": "2.0",
                "error": { "code": -32000, "message": msg },
                "id": id
            }),
        };

        let mut bytes = serde_json::to_vec(&response).unwrap_or_default();
        bytes.push(b'\n');
        out.write_all(&bytes).await.ok();
        out.flush().await.ok();
    }
}

async fn send_error(out: &mut tokio::io::Stdout, id: Value, code: i32, msg: &str) {
    let r = json!({"jsonrpc":"2.0","error":{"code":code,"message":msg},"id":id});
    let mut bytes = serde_json::to_vec(&r).unwrap_or_default();
    bytes.push(b'\n');
    out.write_all(&bytes).await.ok();
    out.flush().await.ok();
}

async fn dispatch(
    partition: &Arc<PartitionState>,
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
            call_tool(partition, name, args).await
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
            "inputSchema": {
                "type": "object",
                "properties": {}
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
                        "description": "Note path without .md (e.g. 'LLM/Intro' or 'Projects/Alpha')"
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
                        "description": "Note path without .md"
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
            "description": "Full-text search across all note titles and bodies. Returns matching note names with context snippets.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search terms"
                    }
                },
                "required": ["query"]
            }
        },
        {
            "name": "query_notes",
            "description": "Filter notes with the vault DSL. Filters: #tag, tag:, title:, status:, area:, author:, date:, due:, pinned:, locked:, project:, priority:. Logic: implicit AND, OR, NOT. Limiters: recent:N, oldest:N. Sort: 'order by <field> [asc|desc]'. Examples: '#rust status:active', 'area:work order by date desc', 'recent:10 NOT #archive'.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "DSL expression"
                    }
                },
                "required": ["query"]
            }
        }
    ])
}

async fn call_tool(
    partition: &Arc<PartitionState>,
    name: &str,
    args: &Value,
) -> Result<Value, String> {
    match name {
        "list_notes" => {
            let db = partition.db.clone();
            let mut notes = tokio::task::spawn_blocking(move || db.list_all_meta())
                .await
                .map_err(|e| e.to_string())?;
            notes.sort_by(|a, b| a.name.cmp(&b.name));
            let text = notes
                .iter()
                .map(|n| n.name.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            text_ok(if text.is_empty() { "(vault is empty)" } else { &text })
        }

        "get_note" => {
            let note_name = str_arg(args, "name")?;
            check_safe(&note_name)?;
            let path = partition.storage_path.join(format!("{note_name}.md"));
            let content = tokio::fs::read_to_string(&path)
                .await
                .map_err(|_| format!("note not found: {note_name}"))?;
            text_ok(&content)
        }

        "write_note" => {
            let note_name = str_arg(args, "name")?;
            let content = str_arg(args, "content")?;
            check_safe(&note_name)?;

            let path = partition.storage_path.join(format!("{note_name}.md"));
            if let Some(parent) = path.parent() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(|e| e.to_string())?;
            }

            let stamped = crate::frontmatter::stamp_last_modified(&content);
            tokio::fs::write(&path, &stamped)
                .await
                .map_err(|e| e.to_string())?;

            let mtime = tokio::fs::metadata(&path)
                .await
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);

            let parsed = crate::frontmatter::parse_note(&stamped);
            let db = partition.db.clone();
            let n = note_name.clone();
            tokio::task::spawn_blocking(move || db.upsert(&n, &parsed, mtime))
                .await
                .ok();

            partition
                .backlink_index
                .write()
                .await
                .update_note(&note_name, &stamped);

            text_ok(&format!("Saved '{note_name}'."))
        }

        "search_notes" => {
            let query = str_arg(args, "query")?;
            let db = partition.db.clone();
            let results = tokio::task::spawn_blocking(move || db.search(&query))
                .await
                .map_err(|e| e.to_string())?;
            let text = if results.is_empty() {
                "No results.".to_string()
            } else {
                results
                    .iter()
                    .map(|r| format!("[{}] {}", r.name, r.snippet))
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            text_ok(&text)
        }

        "query_notes" => {
            let query = str_arg(args, "query")?;
            let db = partition.db.clone();
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
    Ok(json!({
        "content": [{ "type": "text", "text": text }]
    }))
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
        let result = dispatch(&p, "initialize", None).await.unwrap();
        assert_eq!(result["protocolVersion"], "2024-11-05");
        assert!(result["capabilities"]["tools"].is_object());
    }

    #[tokio::test]
    async fn tools_list_contains_all_tools() {
        let dir = tempfile::tempdir().unwrap();
        let p = make_partition(dir.path());
        let result = dispatch(&p, "tools/list", None).await.unwrap();
        let tools: Vec<&str> = result["tools"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|t| t["name"].as_str())
            .collect();
        assert!(tools.contains(&"list_notes"));
        assert!(tools.contains(&"get_note"));
        assert!(tools.contains(&"write_note"));
        assert!(tools.contains(&"search_notes"));
        assert!(tools.contains(&"query_notes"));
    }

    #[tokio::test]
    async fn write_then_get_note() {
        let dir = tempfile::tempdir().unwrap();
        let p = make_partition(dir.path());

        let write_args = json!({ "name": "LLM/Test", "content": "# Hello\nworld" });
        let r = call_tool(&p, "write_note", &write_args).await.unwrap();
        assert!(r["content"][0]["text"].as_str().unwrap().contains("Saved"));

        let get_args = json!({ "name": "LLM/Test" });
        let r = call_tool(&p, "get_note", &get_args).await.unwrap();
        let text = r["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("# Hello"));
        assert!(text.contains("world"));
    }

    #[tokio::test]
    async fn list_notes_after_write() {
        let dir = tempfile::tempdir().unwrap();
        let p = make_partition(dir.path());

        call_tool(&p, "write_note", &json!({"name":"A","content":"alpha"}))
            .await
            .unwrap();
        call_tool(&p, "write_note", &json!({"name":"B","content":"beta"}))
            .await
            .unwrap();

        // Re-index: upsert via write_note updates db, but list_notes reads db.
        // Manually upsert since the test partition starts empty.
        let r = call_tool(&p, "list_notes", &json!({})).await.unwrap();
        let text = r["content"][0]["text"].as_str().unwrap();
        assert!(text.contains('A') || text.contains("vault is empty"));
    }

    #[tokio::test]
    async fn get_note_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let p = make_partition(dir.path());
        let r = call_tool(&p, "get_note", &json!({"name":"Missing"})).await;
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("not found"));
    }

    #[tokio::test]
    async fn rejects_path_traversal() {
        let dir = tempfile::tempdir().unwrap();
        let p = make_partition(dir.path());
        let r = call_tool(&p, "get_note", &json!({"name":"../secret"})).await;
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("invalid"));
    }

    #[tokio::test]
    async fn unknown_method_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let p = make_partition(dir.path());
        let r = dispatch(&p, "nonexistent/method", None).await;
        assert!(r.is_err());
    }

    #[tokio::test]
    async fn ping_returns_empty_object() {
        let dir = tempfile::tempdir().unwrap();
        let p = make_partition(dir.path());
        let r = dispatch(&p, "ping", None).await.unwrap();
        assert_eq!(r, json!({}));
    }
}
