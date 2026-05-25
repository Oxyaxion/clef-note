use chrono::Local;
use gray_matter::{engine::YAML, Matter, Pod};
use regex::Regex;
use serde_json::{Map, Value};
use std::sync::OnceLock;

pub struct ParsedNote {
    pub frontmatter: Value,
    pub body: String,
    pub title: Option<String>,
    pub tags: Vec<String>,
    pub date: Option<String>,
    pub status: Option<String>,
    pub aliases: Vec<String>,
    pub note_type: Option<String>,
    pub due: Option<String>,
    pub url: Option<String>,
    pub author: Option<String>,
    pub rating: Option<i64>,
    pub pinned: bool,
    pub locked: bool,
    pub area: Option<String>,
    pub priority: Option<String>,
    pub project: Option<String>,
    pub last_modified: Option<String>,
}

static INLINE_TAG_RE: OnceLock<Regex> = OnceLock::new();
static INLINE_CODE_RE: OnceLock<Regex> = OnceLock::new();
static FM_BLOCK_RE: OnceLock<Regex> = OnceLock::new();
static LM_LINE_RE: OnceLock<Regex> = OnceLock::new();

fn pod_to_value(pod: &Pod) -> Value {
    match pod {
        Pod::Null => Value::Null,
        Pod::Boolean(b) => Value::Bool(*b),
        Pod::Integer(i) => Value::Number((*i).into()),
        Pod::Float(f) => serde_json::Number::from_f64(*f)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        Pod::String(s) => Value::String(s.clone()),
        Pod::Array(arr) => Value::Array(arr.iter().map(pod_to_value).collect()),
        Pod::Hash(map) => {
            let mut obj = Map::new();
            for (k, v) in map {
                obj.insert(k.clone(), pod_to_value(v));
            }
            Value::Object(obj)
        }
    }
}

fn fm_str(fm: &Value, key: &str) -> Option<String> {
    fm.get(key).and_then(|v| v.as_str()).map(String::from)
}

fn extract_h1(body: &str) -> Option<String> {
    body.lines()
        .next()
        .and_then(|line| line.strip_prefix("# "))
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
}

pub fn parse_note(content: &str) -> ParsedNote {
    let matter = Matter::<YAML>::new();
    let (fm_data, raw_body) = match matter.parse(content) {
        Ok(parsed) => (parsed.data, parsed.content),
        Err(_) => (None, content.to_string()),
    };

    let frontmatter = fm_data.as_ref().map(pod_to_value).unwrap_or_default();
    let body = raw_body.trim_start_matches('\n').to_string();

    let title = fm_str(&frontmatter, "title").or_else(|| extract_h1(&body));
    let date = fm_str(&frontmatter, "date");
    let status = fm_str(&frontmatter, "status");

    let mut tags: Vec<String> = match frontmatter.get("tags") {
        Some(Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        Some(Value::String(s)) => vec![s.clone()],
        _ => vec![],
    };

    // Extract inline #hashtags from body, skipping code blocks and inline code.
    // Line-by-line fence tracking handles CRLF, missing trailing newline, and
    // all language specifiers more reliably than a single regex.
    let body_no_fence = {
        let mut out = String::with_capacity(body.len());
        let mut in_fence = false;
        for line in body.lines() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
                in_fence = !in_fence;
            } else if !in_fence {
                out.push_str(line);
                out.push('\n');
            }
        }
        out
    };
    let inline_code_re = INLINE_CODE_RE
        .get_or_init(|| Regex::new(r"`[^`\n]+`").unwrap());
    let body_clean = inline_code_re.replace_all(&body_no_fence, " ");
    let re = INLINE_TAG_RE
        .get_or_init(|| Regex::new(r"(?:^|\s)#([a-zA-Z0-9_-]+)").unwrap());
    for cap in re.captures_iter(&body_clean) {
        let tag = cap[1].to_string();
        if !tags.contains(&tag) {
            tags.push(tag);
        }
    }
    tags.sort();

    let aliases: Vec<String> = match frontmatter.get("aliases") {
        Some(Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        Some(Value::String(s)) => vec![s.clone()],
        _ => vec![],
    };

    let note_type = fm_str(&frontmatter, "type");
    let due = fm_str(&frontmatter, "due");
    let url = fm_str(&frontmatter, "url");
    let author = fm_str(&frontmatter, "author");

    let rating = frontmatter
        .get("rating")
        .and_then(|v| v.as_i64());

    let pinned = frontmatter
        .get("pinned")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let locked = frontmatter
        .get("locked")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let area = fm_str(&frontmatter, "area");
    let priority = fm_str(&frontmatter, "priority");
    let project = fm_str(&frontmatter, "project");
    let last_modified = fm_str(&frontmatter, "lastModified");

    ParsedNote {
        frontmatter,
        body,
        title,
        tags,
        date,
        status,
        aliases,
        note_type,
        due,
        url,
        author,
        rating,
        pinned,
        locked,
        area,
        priority,
        project,
        last_modified,
    }
}

/// If `content` already has a YAML frontmatter block, update (or insert) the
/// `lastModified` field with today's date. Notes without frontmatter are
/// returned unchanged.
pub fn stamp_last_modified(content: &str) -> String {
    let fm_re = FM_BLOCK_RE
        .get_or_init(|| Regex::new(r"(?s)^---\n(.*?\n)---\n").unwrap());
    let lm_re = LM_LINE_RE
        .get_or_init(|| Regex::new(r"(?m)^lastModified:.*$").unwrap());

    let today = Local::now().format("%Y-%m-%d").to_string();
    let new_line = format!("lastModified: {today}");

    let Some(m) = fm_re.find(content) else {
        return content.to_string();
    };

    let fm_block = &content[m.start()..m.end()];
    let rest = &content[m.end()..];

    if lm_re.is_match(fm_block) {
        let updated_fm = lm_re.replace(fm_block, new_line.as_str());
        format!("{updated_fm}{rest}")
    } else {
        // Append field before the closing ---
        let closing = fm_block.len() - 4; // strip trailing "---\n"
        format!("{}{}\n---\n{}", &fm_block[..closing], new_line, rest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tags_for(content: &str) -> Vec<String> {
        parse_note(content).tags
    }

    #[test]
    fn color_code_in_fenced_block_not_a_tag() {
        let note = "---\ntags: []\n---\n\n```html\n.foo { color: #f00000; }\n```\n";
        let tags = tags_for(note);
        assert!(!tags.contains(&"f00000".to_string()), "color code was treated as a tag: {:?}", tags);
    }

    #[test]
    fn color_code_in_inline_code_not_a_tag() {
        let note = "---\ntags: []\n---\n\nUse `#f00000` for red.\n";
        let tags = tags_for(note);
        assert!(!tags.contains(&"f00000".to_string()), "inline color was treated as a tag: {:?}", tags);
    }

    #[test]
    fn real_hashtag_still_extracted() {
        let note = "---\ntags: []\n---\n\nThis note is about #design and #css.\n";
        let tags = tags_for(note);
        assert!(tags.contains(&"design".to_string()));
        assert!(tags.contains(&"css".to_string()));
    }

    #[test]
    fn color_code_no_frontmatter() {
        let note = "```css\n.foo { color: #f00000; }\n```\n";
        let tags = tags_for(note);
        assert!(!tags.contains(&"f00000".to_string()), "color code was treated as a tag: {:?}", tags);
    }

    #[test]
    fn color_code_no_trailing_newline() {
        // No \n after closing fence — common when file ends immediately
        let note = "---\ntags: []\n---\n\n```css\ncolor: #abc123;\n```";
        let tags = tags_for(note);
        assert!(!tags.contains(&"abc123".to_string()), "color code without trailing newline treated as tag: {:?}", tags);
    }

    #[test]
    fn tilde_fence_also_excluded() {
        let note = "---\ntags: []\n---\n\n~~~bash\nexport COLOR=#ff0000\n~~~\n";
        let tags = tags_for(note);
        assert!(!tags.contains(&"ff0000".to_string()), "color in tilde fence treated as tag: {:?}", tags);
    }

    #[test]
    fn crlf_line_endings() {
        let note = "---\r\ntags: []\r\n---\r\n\r\n```css\r\ncolor: #f00000;\r\n```\r\n";
        let tags = tags_for(note);
        assert!(!tags.contains(&"f00000".to_string()), "color with CRLF treated as tag: {:?}", tags);
    }

    #[test]
    fn multiline_code_block_color() {
        let note = "---\ntags: []\n---\n\n# Title\n\n```css\n:root {\n  --primary: #3b82f6;\n  --secondary: #f00000;\n}\n```\n\nSome text with #realtag\n";
        let tags = tags_for(note);
        assert!(!tags.contains(&"3b82f6".to_string()), "hex color treated as tag: {:?}", tags);
        assert!(!tags.contains(&"f00000".to_string()), "hex color treated as tag: {:?}", tags);
        assert!(tags.contains(&"realtag".to_string()), "real tag missing: {:?}", tags);
    }
}
