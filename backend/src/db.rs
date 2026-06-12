use std::collections::{HashMap, HashSet};
use std::sync::{OnceLock, RwLock};

use serde::Serialize;

use crate::frontmatter::ParsedNote;

// ── Public types ──────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Clone)]
pub struct NoteMeta {
    pub name: String,
    pub pinned: bool,
    pub is_template: bool,
    pub is_index: bool,
    pub has_frontmatter: bool,
}

#[derive(Debug, Serialize, Clone)]
pub struct NoteRow {
    pub name: String,
    pub title: Option<String>,
    pub date: Option<String>,
    pub status: Option<String>,
    pub tags: Vec<String>,
    pub modified_at: i64,
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

#[derive(Debug, Serialize, Clone)]
pub struct SearchResult {
    pub name: String,
    pub title: Option<String>,
    pub snippet: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct TagCount {
    pub tag: String,
    pub count: usize,
}

#[derive(Debug, Serialize, Clone)]
pub struct NoteStub {
    pub name: String,
    pub title: Option<String>,
    pub body_len: usize,
}

// ── Internal storage ──────────────────────────────────────────────────────────

struct StoredNote {
    row: NoteRow,
    body: String,
    has_frontmatter: bool,
}

// ── Note index ────────────────────────────────────────────────────────────────

pub struct Db(RwLock<HashMap<String, StoredNote>>);

impl Db {
    pub fn new() -> Self {
        Db(RwLock::new(HashMap::new()))
    }

    pub fn upsert(&self, name: &str, parsed: &ParsedNote, modified_at: i64) {
        let row = NoteRow {
            name: name.to_string(),
            title: parsed.title.clone(),
            date: parsed.date.clone(),
            status: parsed.status.clone(),
            tags: parsed.tags.clone(),
            modified_at,
            aliases: parsed.aliases.clone(),
            note_type: parsed.note_type.clone(),
            due: parsed.due.clone(),
            url: parsed.url.clone(),
            author: parsed.author.clone(),
            rating: parsed.rating,
            pinned: parsed.pinned,
            locked: parsed.locked,
            area: parsed.area.clone(),
            priority: parsed.priority.clone(),
            project: parsed.project.clone(),
            last_modified: parsed.last_modified.clone(),
        };
        let has_frontmatter = parsed.frontmatter
            .as_object()
            .map(|m| !m.is_empty())
            .unwrap_or(false);
        self.0.write().unwrap().insert(
            name.to_string(),
            StoredNote { row, body: parsed.body.clone(), has_frontmatter },
        );
    }

    pub fn delete(&self, name: &str) {
        self.0.write().unwrap().remove(name);
    }

    pub fn rename(&self, old_name: &str, new_name: &str) {
        let mut index = self.0.write().unwrap();
        if let Some(mut stored) = index.remove(old_name) {
            stored.row.name = new_name.to_string();
            index.insert(new_name.to_string(), stored);
        }
    }

    pub fn search(&self, q: &str) -> Vec<SearchResult> {
        if q.trim().is_empty() {
            return vec![];
        }
        let lower_q = q.to_lowercase();
        let index = self.0.read().unwrap();
        let mut results: Vec<SearchResult> = index
            .values()
            .filter(|n| {
                let title_hay = n.row.title.as_deref().unwrap_or(&n.row.name).to_lowercase();
                title_hay.contains(&lower_q) || n.body.to_lowercase().contains(&lower_q)
            })
            .map(|n| SearchResult {
                name: n.row.name.clone(),
                title: n.row.title.clone(),
                snippet: make_snippet(&n.body, q),
            })
            .collect();
        results.sort_by(|a, b| a.name.cmp(&b.name));
        results
    }

    pub fn list_tags(&self) -> Vec<TagCount> {
        let index = self.0.read().unwrap();
        let mut counts: HashMap<String, usize> = HashMap::new();
        for note in index.values() {
            for tag in &note.row.tags {
                *counts.entry(tag.clone()).or_insert(0) += 1;
            }
        }
        let mut tags: Vec<TagCount> = counts
            .into_iter()
            .map(|(tag, count)| TagCount { tag, count })
            .collect();
        tags.sort_by(|a, b| b.count.cmp(&a.count).then(a.tag.cmp(&b.tag)));
        tags
    }

    pub fn get_aliases(&self) -> HashMap<String, String> {
        let index = self.0.read().unwrap();
        let mut map = HashMap::new();
        for note in index.values() {
            for alias in &note.row.aliases {
                map.insert(alias.clone(), note.row.name.clone());
            }
        }
        map
    }

    pub fn get_note_aliases(&self, name: &str) -> Vec<String> {
        self.0
            .read()
            .unwrap()
            .get(name)
            .map(|n| n.row.aliases.clone())
            .unwrap_or_default()
    }

    pub fn list_all_meta(&self) -> Vec<NoteMeta> {
        let index = self.0.read().unwrap();
        index.values().map(|n| {
            let is_template = n.row.note_type.as_deref() == Some("template");
            let is_index    = n.row.note_type.as_deref() == Some("index");
            NoteMeta {
                name: n.row.name.clone(),
                pinned: n.row.pinned,
                is_template,
                is_index,
                has_frontmatter: n.has_frontmatter,
            }
        }).collect()
    }

    pub fn get_field_values(&self, field: &str) -> Vec<String> {
        let index = self.0.read().unwrap();
        let mut values: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for note in index.values() {
            let val: Option<String> = match field {
                "status" => note.row.status.clone(),
                "area"   => note.row.area.clone(),
                "author" => note.row.author.clone(),
                "type"   => note.row.note_type.clone(),
                "due"    => note.row.due.clone(),
                "url"    => note.row.url.clone(),
                "rating"   => note.row.rating.map(|r| r.to_string()),
                "priority"     => note.row.priority.clone(),
                "project"      => note.row.project.clone(),
                "lastModified" => note.row.last_modified.clone(),
                _              => None,
            };
            if let Some(v) = val && !v.is_empty() {
                values.insert(v);
            }
        }
        values.into_iter().collect()
    }

    pub fn get_media_usage(&self) -> (HashSet<String>, HashSet<String>) {
        static ASSET_RE: OnceLock<regex::Regex> = OnceLock::new();
        static DRAWING_RE: OnceLock<regex::Regex> = OnceLock::new();
        let asset_re = ASSET_RE.get_or_init(|| regex::Regex::new(r#"/assets/([^)\s"'\n>]+)"#).unwrap());
        let drawing_re = DRAWING_RE.get_or_init(|| regex::Regex::new(r"(?m)^```drawing\r?\n(\S+)").unwrap());

        let index = self.0.read().unwrap();
        let mut used_assets = HashSet::new();
        let mut used_drawings = HashSet::new();
        for note in index.values() {
            for cap in asset_re.captures_iter(&note.body) {
                used_assets.insert(cap[1].to_string());
            }
            for cap in drawing_re.captures_iter(&note.body) {
                used_drawings.insert(cap[1].to_string());
            }
        }
        (used_assets, used_drawings)
    }

    /// Returns notes whose body length (bytes) is <= `max_bytes`, sorted by body length ascending.
    pub fn stubs(&self, max_bytes: usize) -> Vec<NoteStub> {
        let index = self.0.read().unwrap();
        let mut result: Vec<NoteStub> = index
            .values()
            .filter(|n| n.body.len() <= max_bytes)
            .map(|n| NoteStub {
                name: n.row.name.clone(),
                title: n.row.title.clone(),
                body_len: n.body.len(),
            })
            .collect();
        result.sort_by(|a, b| a.body_len.cmp(&b.body_len).then(a.name.cmp(&b.name)));
        result
    }

    pub fn query_notes(&self, q: &str) -> Vec<NoteRow> {
        let index = self.0.read().unwrap();
        let pq = parse_query(q);

        let mut results: Vec<&NoteRow> = index
            .values()
            .map(|n| &n.row)
            .filter(|note| {
                let or_match = pq.or_groups.is_empty()
                    || pq.or_groups.iter().any(|group| group.iter().all(|p| p.eval(note)));
                let not_match = pq.global_not.iter().all(|p| p.eval(note));
                or_match && not_match
            })
            .collect();

        // recent:N / oldest:N limit by modification time before any order by
        if let Some(n) = pq.recent {
            results.sort_by_key(|r| std::cmp::Reverse(r.modified_at));
            results.truncate(n);
        } else if let Some(n) = pq.oldest {
            results.sort_by_key(|r| r.modified_at);
            results.truncate(n);
        }

        // order by overrides the final sort (after potential recent/oldest truncation)
        if let Some(ref ob) = pq.order_by {
            results.sort_by(|a, b| compare_by_field(a, b, &ob.field, ob.desc));
        } else if pq.recent.is_none() && pq.oldest.is_none() {
            results.sort_by(|a, b| a.name.cmp(&b.name));
        }

        results.into_iter().cloned().collect()
    }
}

// ── Snippet helper ────────────────────────────────────────────────────────────

fn make_snippet(body: &str, query: &str) -> String {
    let lower_body = body.to_lowercase();
    let lower_q = query.to_lowercase();
    if let Some(pos) = lower_body.find(&lower_q) {
        let start = pos.saturating_sub(80);
        let end = (pos + lower_q.len() + 80).min(body.len());
        let start = body.char_indices().map(|(i, _)| i).filter(|&i| i <= start).next_back().unwrap_or(0);
        let end = body.char_indices().map(|(i, _)| i).find(|&i| i >= end).unwrap_or(body.len());
        let prefix = if start > 0 { "…" } else { "" };
        let suffix = if end < body.len() { "…" } else { "" };
        let raw = body[start..end].trim();
        let clean = raw
            .trim_start_matches(['#', '-', '*', '>', ' ', '\t'])
            .trim();
        format!("{}{}{}", prefix, clean, suffix)
    } else {
        body.chars().take(160).collect()
    }
}

// ── Query DSL ─────────────────────────────────────────────────────────────────

/// Comparison operator for date range filters (`due>=:`, `date<:`, etc.).
enum CmpOp { Lt, Lte, Gt, Gte }

/// A single filter predicate, with a `not` flag for negation.
enum Pred {
    Tag(String, bool),        // prefix match on any tag (lowercase)
    Status(String, bool),     // exact match
    DatePrefix(String, bool), // starts_with
    DateCmp(CmpOp, String, bool), // lexicographic date comparison (ISO)
    Title(String, bool),      // substring on title field only (falls back to name if no title)
    Text(String, bool),       // bare word: substring on title OR name (lowercase)
    Path(String, bool),       // substring on full path (lowercase)
    FileName(String, bool),   // substring on last segment only (lowercase)
    NoteType(String, bool),   // exact match
    DuePrefix(String, bool),  // starts_with
    DueCmp(CmpOp, String, bool),  // lexicographic date comparison (ISO)
    Area(String, bool),       // substring (lowercase)
    Author(String, bool),     // substring (lowercase)
    Rating(i64, bool),        // exact match
    Alias(String, bool),      // exact match (lowercase)
    Url(String, bool),        // substring (lowercase)
    Pinned(bool),             // expected value (NOT already folded in at parse time)
    Locked(bool),
    Priority(String, bool),        // exact match (high/medium/low)
    Project(String, bool),         // substring (lowercase)
    LastModified(String, bool),    // starts_with
    Depth(usize, bool),            // exact slash count in name
}

impl Pred {
    fn eval(&self, note: &NoteRow) -> bool {
        match self {
            Pred::Tag(prefix, not) => {
                let has = note.tags.iter().any(|t| t.to_lowercase().starts_with(prefix.as_str()));
                if *not { !has } else { has }
            }
            Pred::Status(val, not) => {
                let m = note.status.as_deref() == Some(val.as_str());
                if *not { !m } else { m }
            }
            Pred::DatePrefix(prefix, not) => {
                let m = note.date.as_deref().is_some_and(|d| d.starts_with(prefix.as_str()));
                if *not { !m } else { m }
            }
            Pred::Title(pattern, not) => {
                let segment = note.name.split('/').next_back().unwrap_or(&note.name);
                let hay = note.title.as_deref().unwrap_or(segment).to_lowercase();
                let m = hay.contains(pattern.as_str());
                if *not { !m } else { m }
            }
            Pred::Text(pattern, not) => {
                let title_m = note.title.as_deref()
                    .is_some_and(|t| t.to_lowercase().contains(pattern.as_str()));
                let name_m = note.name.to_lowercase().contains(pattern.as_str());
                let m = title_m || name_m;
                if *not { !m } else { m }
            }
            Pred::Path(pattern, not) => {
                let m = note.name.to_lowercase().contains(pattern.as_str());
                if *not { !m } else { m }
            }
            Pred::FileName(pattern, not) => {
                let segment = note.name.split('/').next_back().unwrap_or(&note.name).to_lowercase();
                let m = segment.contains(pattern.as_str());
                if *not { !m } else { m }
            }
            Pred::NoteType(val, not) => {
                let m = note.note_type.as_deref() == Some(val.as_str());
                if *not { !m } else { m }
            }
            Pred::DuePrefix(prefix, not) => {
                let m = note.due.as_deref().is_some_and(|d| d.starts_with(prefix.as_str()));
                if *not { !m } else { m }
            }
            Pred::DueCmp(op, threshold, not) => {
                let m = note.due.as_deref().is_some_and(|d| cmp_str(d, threshold, op));
                if *not { !m } else { m }
            }
            Pred::DateCmp(op, threshold, not) => {
                let m = note.date.as_deref().is_some_and(|d| cmp_str(d, threshold, op));
                if *not { !m } else { m }
            }
            Pred::Area(pattern, not) => {
                let m = note.area.as_deref().is_some_and(|s| s.to_lowercase().contains(pattern.as_str()));
                if *not { !m } else { m }
            }
            Pred::Author(pattern, not) => {
                let m = note.author.as_deref().is_some_and(|s| s.to_lowercase().contains(pattern.as_str()));
                if *not { !m } else { m }
            }
            Pred::Rating(val, not) => {
                let m = note.rating == Some(*val);
                if *not { !m } else { m }
            }
            Pred::Alias(val, not) => {
                let m = note.aliases.iter().any(|a| a.to_lowercase() == val.as_str());
                if *not { !m } else { m }
            }
            Pred::Url(pattern, not) => {
                let m = note.url.as_deref().is_some_and(|s| s.to_lowercase().contains(pattern.as_str()));
                if *not { !m } else { m }
            }
            Pred::Pinned(expected) => note.pinned == *expected,
            Pred::Locked(expected) => note.locked == *expected,
            Pred::Priority(val, not) => {
                let m = note.priority.as_deref() == Some(val.as_str());
                if *not { !m } else { m }
            }
            Pred::Project(pattern, not) => {
                let m = note.project.as_deref().is_some_and(|s| s.to_lowercase().contains(pattern.as_str()));
                if *not { !m } else { m }
            }
            Pred::LastModified(prefix, not) => {
                let m = note.last_modified.as_deref().is_some_and(|d| d.starts_with(prefix.as_str()));
                if *not { !m } else { m }
            }
            Pred::Depth(n, not) => {
                let count = note.name.chars().filter(|&c| c == '/').count();
                let m = count == *n;
                if *not { !m } else { m }
            }
        }
    }
}

fn cmp_str(val: &str, threshold: &str, op: &CmpOp) -> bool {
    match op {
        CmpOp::Lt  => val < threshold,
        CmpOp::Lte => val <= threshold,
        CmpOp::Gt  => val > threshold,
        CmpOp::Gte => val >= threshold,
    }
}

// ── Order by ─────────────────────────────────────────────────────────────────

struct OrderBy {
    field: String,
    desc: bool,
}

fn compare_by_field(a: &NoteRow, b: &NoteRow, field: &str, desc: bool) -> std::cmp::Ordering {
    match field {
        "name" => {
            let cmp = a.name.cmp(&b.name);
            if desc { cmp.reverse() } else { cmp }
        }
        "title" => {
            let ta = a.title.as_deref().unwrap_or(&a.name);
            let tb = b.title.as_deref().unwrap_or(&b.name);
            let cmp = ta.cmp(tb);
            if desc { cmp.reverse() } else { cmp }
        }
        "date"             => cmp_opt_str(a.date.as_deref(), b.date.as_deref(), desc),
        "modified" | "modified_at" => {
            let cmp = a.modified_at.cmp(&b.modified_at);
            if desc { cmp.reverse() } else { cmp }
        }
        "due"              => cmp_opt_str(a.due.as_deref(), b.due.as_deref(), desc),
        "status"           => cmp_opt_str(a.status.as_deref(), b.status.as_deref(), desc),
        "rating"           => cmp_opt_i64(a.rating, b.rating, desc),
        "area"             => cmp_opt_str(a.area.as_deref(), b.area.as_deref(), desc),
        "author"           => cmp_opt_str(a.author.as_deref(), b.author.as_deref(), desc),
        "priority"         => cmp_priority(a.priority.as_deref(), b.priority.as_deref(), desc),
        "project"          => cmp_opt_str(a.project.as_deref(), b.project.as_deref(), desc),
        "lastModified"     => cmp_opt_str(a.last_modified.as_deref(), b.last_modified.as_deref(), desc),
        _                  => std::cmp::Ordering::Equal,
    }
}

// None always sorts last, regardless of direction.
fn cmp_opt_str(a: Option<&str>, b: Option<&str>, desc: bool) -> std::cmp::Ordering {
    match (a, b) {
        (Some(a), Some(b)) => {
            let cmp = a.cmp(b);
            if desc { cmp.reverse() } else { cmp }
        }
        (Some(_), None)    => std::cmp::Ordering::Less,
        (None, Some(_))    => std::cmp::Ordering::Greater,
        (None, None)       => std::cmp::Ordering::Equal,
    }
}

fn cmp_opt_i64(a: Option<i64>, b: Option<i64>, desc: bool) -> std::cmp::Ordering {
    match (a, b) {
        (Some(a), Some(b)) => {
            let cmp = a.cmp(&b);
            if desc { cmp.reverse() } else { cmp }
        }
        (Some(_), None)    => std::cmp::Ordering::Less,
        (None, Some(_))    => std::cmp::Ordering::Greater,
        (None, None)       => std::cmp::Ordering::Equal,
    }
}

fn cmp_priority(a: Option<&str>, b: Option<&str>, desc: bool) -> std::cmp::Ordering {
    fn rank(p: Option<&str>) -> Option<u8> {
        match p {
            Some("high")   => Some(0),
            Some("medium") => Some(1),
            Some("low")    => Some(2),
            _              => None,
        }
    }
    match (rank(a), rank(b)) {
        (Some(ra), Some(rb)) => {
            let cmp = ra.cmp(&rb);
            if desc { cmp.reverse() } else { cmp }
        }
        (Some(_), None)  => std::cmp::Ordering::Less,
        (None, Some(_))  => std::cmp::Ordering::Greater,
        (None, None)     => std::cmp::Ordering::Equal,
    }
}

/// Strip a trailing comparison operator from a field key and return (base_key, op).
/// Checks `<=` / `>=` before `<` / `>` to avoid greedy single-char match.
fn parse_cmp_op(k: &str) -> (&str, Option<CmpOp>) {
    if let Some(f) = k.strip_suffix("<=") { return (f, Some(CmpOp::Lte)); }
    if let Some(f) = k.strip_suffix(">=") { return (f, Some(CmpOp::Gte)); }
    if let Some(f) = k.strip_suffix('<')  { return (f, Some(CmpOp::Lt));  }
    if let Some(f) = k.strip_suffix('>')  { return (f, Some(CmpOp::Gt));  }
    (k, None)
}

// ── Query parser ──────────────────────────────────────────────────────────────

struct ParsedQuery {
    or_groups: Vec<Vec<Pred>>,
    global_not: Vec<Pred>,
    recent: Option<usize>,
    oldest: Option<usize>,
    order_by: Option<OrderBy>,
}

/// Parse a DSL query string into OR-groups of AND-predicates.
///
/// Precedence: AND binds tighter than OR.
///   `A OR B AND C`  →  `[[A], [B, C]]`  →  A OR (B AND C)
fn parse_query(q: &str) -> ParsedQuery {
    let mut or_groups: Vec<Vec<Pred>> = vec![vec![]];
    let mut global_not: Vec<Pred> = vec![];
    let mut pending_not = false;
    let mut pending_or = false;
    let mut recent: Option<usize> = None;
    let mut oldest: Option<usize> = None;
    let mut order_by: Option<OrderBy> = None;

    #[derive(PartialEq)]
    enum ObState { Idle, ExpectBy, ExpectField, ExpectDir }
    let mut ob_state = ObState::Idle;
    let mut ob_field = String::new();

    for token in q.split_whitespace() {
        let upper = token.to_ascii_uppercase();

        match ob_state {
            ObState::ExpectBy => {
                ob_state = if upper == "BY" { ObState::ExpectField } else { ObState::Idle };
                continue;
            }
            ObState::ExpectField => {
                ob_field = token.to_lowercase();
                ob_state = ObState::ExpectDir;
                continue;
            }
            ObState::ExpectDir => {
                let desc = matches!(upper.as_str(), "DESC" | "REVERSE" | "REVERSED");
                if matches!(upper.as_str(), "ASC" | "DESC" | "REVERSE" | "REVERSED") {
                    order_by = Some(OrderBy { field: ob_field.clone(), desc });
                    ob_state = ObState::Idle;
                    continue;
                }
                // No direction token — commit with default asc and fall through
                order_by = Some(OrderBy { field: ob_field.clone(), desc: false });
                ob_state = ObState::Idle;
            }
            ObState::Idle => {}
        }

        match upper.as_str() {
            "OR"    => { pending_or  = true; continue; }
            "AND"   => {                     continue; }
            "NOT"   => { pending_not = true; continue; }
            "ORDER" => { ob_state = ObState::ExpectBy; continue; }
            _ => {}
        }

        if pending_or {
            or_groups.push(vec![]);
            pending_or = false;
        }

        let not = std::mem::replace(&mut pending_not, false);

        let pred = if let Some(v) = token.strip_prefix('#') {
            Pred::Tag(v.to_lowercase(), not)
        } else if let Some((k, v)) = token.split_once(':') {
            let (base_k, cmp_op) = parse_cmp_op(k);
            match base_k.to_lowercase().as_str() {
                "tag"    => Pred::Tag(v.to_lowercase(), not),
                "status" => Pred::Status(v.to_string(), not),
                "date"   => match cmp_op {
                    Some(op) => Pred::DateCmp(op, v.to_string(), not),
                    None     => Pred::DatePrefix(v.to_string(), not),
                },
                "title"  => Pred::Title(v.to_lowercase(), not),
                "path" => Pred::Path(v.to_lowercase(), not),
                "name" => Pred::FileName(v.to_lowercase(), not),
                "type"   => Pred::NoteType(v.to_string(), not),
                "due"    => match cmp_op {
                    Some(op) => Pred::DueCmp(op, v.to_string(), not),
                    None     => Pred::DuePrefix(v.to_string(), not),
                },
                "area"   => Pred::Area(v.to_lowercase(), not),
                "author" => Pred::Author(v.to_lowercase(), not),
                "rating" => match v.parse::<i64>() {
                    Ok(n)  => Pred::Rating(n, not),
                    Err(_) => continue,
                },
                "alias"    => Pred::Alias(v.to_lowercase(), not),
                "url"      => Pred::Url(v.to_lowercase(), not),
                "priority"     => Pred::Priority(v.to_lowercase(), not),
                "project"      => Pred::Project(v.to_lowercase(), not),
                "lastmodified" => Pred::LastModified(v.to_string(), not),
                "depth" => match v.parse::<usize>() {
                    Ok(n)  => Pred::Depth(n, not),
                    Err(_) => continue,
                },
                "pinned" => match v {
                    "true"  => Pred::Pinned(!not),
                    "false" => Pred::Pinned(not),
                    _       => continue,
                },
                "locked" => match v {
                    "true"  => Pred::Locked(!not),
                    "false" => Pred::Locked(not),
                    _       => continue,
                },
                "recent" => {
                    recent = Some(if v.is_empty() { usize::MAX } else if let Ok(n) = v.parse() { n } else { continue });
                    continue;
                }
                "oldest" => {
                    oldest = Some(if v.is_empty() { usize::MAX } else if let Ok(n) = v.parse() { n } else { continue });
                    continue;
                }
                _ => continue,
            }
        } else {
            // Bare token → title OR name search
            Pred::Text(token.to_lowercase(), not)
        };

        // NOT predicates are global exclusions — applied after OR-group evaluation
        // so `A OR B AND NOT C` means `(A OR B) AND NOT C`, not `A OR (B AND NOT C)`.
        if not {
            global_not.push(pred);
        } else {
            or_groups.last_mut().unwrap().push(pred);
        }
    }

    // Flush a pending order field with no direction token
    if ob_state == ObState::ExpectDir && !ob_field.is_empty() {
        order_by = Some(OrderBy { field: ob_field, desc: false });
    }

    // Drop empty groups (e.g. from a leading OR)
    let or_groups = or_groups.into_iter().filter(|g| !g.is_empty()).collect();
    ParsedQuery { or_groups, global_not, recent, oldest, order_by }
}

// ── Indexing ──────────────────────────────────────────────────────────────────

/// Walk `notes_dir`, parse every `.md` file and insert it into `db`.
/// Hidden directories (names starting with `.`) are skipped.
pub fn index_dir(db: &Db, notes_dir: &std::path::Path) {
    for entry in walkdir::WalkDir::new(notes_dir)
        .into_iter()
        .filter_entry(|e| !e.file_name().to_str().map_or(false, |s| s.starts_with('.')))
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let Ok(rel) = path.strip_prefix(notes_dir) else { continue };
        let name = rel.with_extension("").to_string_lossy().replace('\\', "/");
        if let Ok(content) = std::fs::read_to_string(path) {
            let parsed = crate::frontmatter::parse_note(&content);
            db.upsert(&name, &parsed, crate::notes::read_mtime(path));
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Helpers ───────────────────────────────────────────────────────────────

    /// Default NoteRow — every field is empty/false/None/0.
    fn r(name: &str) -> NoteRow {
        NoteRow {
            name: name.to_string(),
            title: None,
            date: None,
            status: None,
            tags: vec![],
            modified_at: 0,
            aliases: vec![],
            note_type: None,
            due: None,
            url: None,
            author: None,
            rating: None,
            pinned: false,
            locked: false,
            area: None,
            priority: None,
            project: None,
            last_modified: None,
        }
    }

    /// Insert a pre-built row directly into the DB (bypasses ParsedNote).
    fn insert(db: &Db, row: NoteRow) {
        db.0.write().unwrap().insert(
            row.name.clone(),
            StoredNote { row, body: String::new() },
        );
    }

    /// Run a query and return note names in the order produced by the engine.
    fn names(db: &Db, q: &str) -> Vec<String> {
        db.query_notes(q).into_iter().map(|r| r.name).collect()
    }

    /// Like names(), but sort the result for order-independent assertions.
    fn sorted_names(db: &Db, q: &str) -> Vec<String> {
        let mut v = names(db, q);
        v.sort();
        v
    }

    // ── Tag ───────────────────────────────────────────────────────────────────

    #[test]
    fn tag_hash_basic() {
        let db = Db::new();
        insert(&db, NoteRow { tags: vec!["work".into()], ..r("a") });
        insert(&db, NoteRow { tags: vec!["personal".into()], ..r("b") });
        insert(&db, NoteRow { tags: vec!["work".into(), "urgent".into()], ..r("c") });

        assert_eq!(sorted_names(&db, "#work"), vec!["a", "c"]);
    }

    #[test]
    fn tag_key_value_syntax() {
        let db = Db::new();
        insert(&db, NoteRow { tags: vec!["work".into()], ..r("a") });
        insert(&db, NoteRow { tags: vec!["personal".into()], ..r("b") });

        assert_eq!(sorted_names(&db, "tag:work"), vec!["a"]);
    }

    #[test]
    fn tag_prefix_match() {
        // starts_with: "#work" matches tags "work" and "work/meeting"
        let db = Db::new();
        insert(&db, NoteRow { tags: vec!["work/meeting".into()], ..r("a") });
        insert(&db, NoteRow { tags: vec!["work".into()], ..r("b") });
        insert(&db, NoteRow { tags: vec!["personal".into()], ..r("c") });

        assert_eq!(sorted_names(&db, "#work"), vec!["a", "b"]);
    }

    #[test]
    fn tag_case_insensitive() {
        let db = Db::new();
        insert(&db, NoteRow { tags: vec!["Work".into()], ..r("a") });
        insert(&db, NoteRow { tags: vec!["WORK".into()], ..r("b") });
        insert(&db, NoteRow { tags: vec!["other".into()], ..r("c") });

        assert_eq!(sorted_names(&db, "#work"), vec!["a", "b"]);
    }

    #[test]
    fn tag_no_match_returns_empty() {
        let db = Db::new();
        insert(&db, NoteRow { tags: vec!["personal".into()], ..r("a") });

        assert!(sorted_names(&db, "#work").is_empty());
    }

    // ── Status ────────────────────────────────────────────────────────────────

    #[test]
    fn status_exact_match() {
        let db = Db::new();
        insert(&db, NoteRow { status: Some("active".into()), ..r("a") });
        insert(&db, NoteRow { status: Some("done".into()), ..r("b") });
        insert(&db, NoteRow { ..r("c") });

        assert_eq!(sorted_names(&db, "status:active"), vec!["a"]);
    }

    #[test]
    fn status_no_partial_match() {
        // status: is exact, not substring
        let db = Db::new();
        insert(&db, NoteRow { status: Some("active".into()), ..r("a") });
        insert(&db, NoteRow { status: Some("inactive".into()), ..r("b") });

        assert_eq!(sorted_names(&db, "status:active"), vec!["a"]);
    }

    // ── Date ──────────────────────────────────────────────────────────────────

    #[test]
    fn date_month_prefix() {
        let db = Db::new();
        insert(&db, NoteRow { date: Some("2025-01-15".into()), ..r("a") });
        insert(&db, NoteRow { date: Some("2025-02-20".into()), ..r("b") });
        insert(&db, NoteRow { date: Some("2024-12-01".into()), ..r("c") });
        insert(&db, NoteRow { ..r("d") });

        assert_eq!(sorted_names(&db, "date:2025-01"), vec!["a"]);
    }

    #[test]
    fn date_year_prefix_matches_all_months() {
        let db = Db::new();
        insert(&db, NoteRow { date: Some("2025-01-15".into()), ..r("a") });
        insert(&db, NoteRow { date: Some("2025-09-01".into()), ..r("b") });
        insert(&db, NoteRow { date: Some("2024-12-01".into()), ..r("c") });

        assert_eq!(sorted_names(&db, "date:2025"), vec!["a", "b"]);
    }

    // ── Text / Title / Path / Name ────────────────────────────────────────────

    #[test]
    fn bare_word_matches_note_name() {
        let db = Db::new();
        insert(&db, NoteRow { ..r("Meeting Notes") });
        insert(&db, NoteRow { ..r("Daily Journal") });
        insert(&db, NoteRow { ..r("Ideas") });

        assert_eq!(sorted_names(&db, "meeting"), vec!["Meeting Notes"]);
    }

    #[test]
    fn bare_word_matches_title_field() {
        let db = Db::new();
        insert(&db, NoteRow { title: Some("Project Overview".into()), ..r("a") });
        insert(&db, NoteRow { title: Some("Daily Journal".into()), ..r("b") });
        insert(&db, NoteRow { ..r("c") });

        assert_eq!(sorted_names(&db, "project"), vec!["a"]);
    }

    #[test]
    fn bare_word_searches_full_path() {
        let db = Db::new();
        // "work" is in the folder name, not the file name
        insert(&db, NoteRow { ..r("Work/Note") });
        insert(&db, NoteRow { ..r("Personal/Note") });

        assert_eq!(sorted_names(&db, "work"), vec!["Work/Note"]);
    }

    #[test]
    fn title_filter_uses_basename_fallback() {
        let db = Db::new();
        // No title → falls back to basename for title: matching
        insert(&db, NoteRow { ..r("Work/Notes") });        // folder=Work, basename=Notes
        insert(&db, NoteRow { ..r("Work/Meeting") });      // basename=Meeting
        insert(&db, NoteRow { title: Some("Work Plan".into()), ..r("x") });

        // title:work → basename "Notes" ≠ work, "Meeting" ≠ work, title "Work Plan" contains work
        assert_eq!(sorted_names(&db, "title:work"), vec!["x"]);
        // title:notes → basename "Notes" contains "notes"
        assert_eq!(sorted_names(&db, "title:notes"), vec!["Work/Notes"]);
    }

    #[test]
    fn title_vs_path_difference() {
        let db = Db::new();
        // Full path has "work", but basename is "Note"
        insert(&db, NoteRow { ..r("Work/Note") });

        // path:work → full path "work/note" contains "work" → matches
        assert_eq!(sorted_names(&db, "path:work"), vec!["Work/Note"]);
        // title:work → basename "note" does not contain "work" → no match
        assert!(sorted_names(&db, "title:work").is_empty());
    }

    #[test]
    fn path_filter_full_path_match() {
        let db = Db::new();
        insert(&db, NoteRow { ..r("Work/Meeting Notes") });
        insert(&db, NoteRow { ..r("Personal/Journal") });
        insert(&db, NoteRow { ..r("Work/Planning") });

        assert_eq!(sorted_names(&db, "path:work"), vec!["Work/Meeting Notes", "Work/Planning"]);
    }

    #[test]
    fn name_filter_last_segment_only() {
        let db = Db::new();
        insert(&db, NoteRow { ..r("Work/Meeting") });     // basename = Meeting
        insert(&db, NoteRow { ..r("Meeting/Topic") });    // basename = Topic (folder is Meeting)
        insert(&db, NoteRow { ..r("Other") });

        // name: matches last segment → only "Work/Meeting" has basename "meeting"
        assert_eq!(sorted_names(&db, "name:meeting"), vec!["Work/Meeting"]);
    }

    #[test]
    fn path_vs_name_distinction() {
        let db = Db::new();
        insert(&db, NoteRow { ..r("Work/Meeting") });
        insert(&db, NoteRow { ..r("Meeting/Topic") });

        // path:meeting → full paths "work/meeting" and "meeting/topic" both contain "meeting"
        assert_eq!(sorted_names(&db, "path:meeting"), vec!["Meeting/Topic", "Work/Meeting"]);
        // name:meeting → basenames: "meeting" matches, "topic" does not
        assert_eq!(sorted_names(&db, "name:meeting"), vec!["Work/Meeting"]);
    }

    // ── Type ──────────────────────────────────────────────────────────────────

    #[test]
    fn type_exact_match() {
        let db = Db::new();
        insert(&db, NoteRow { note_type: Some("template".into()), ..r("t") });
        insert(&db, NoteRow { note_type: Some("journal".into()), ..r("j") });
        insert(&db, NoteRow { ..r("normal") });

        assert_eq!(sorted_names(&db, "type:template"), vec!["t"]);
    }

    // ── Due ───────────────────────────────────────────────────────────────────

    #[test]
    fn due_prefix_match() {
        let db = Db::new();
        insert(&db, NoteRow { due: Some("2025-06-01".into()), ..r("a") });
        insert(&db, NoteRow { due: Some("2025-07-15".into()), ..r("b") });
        insert(&db, NoteRow { due: Some("2026-01-01".into()), ..r("c") });
        insert(&db, NoteRow { ..r("d") });

        assert_eq!(sorted_names(&db, "due:2025-06"), vec!["a"]);
        assert_eq!(sorted_names(&db, "due:2025"), vec!["a", "b"]);
    }

    // ── Area / Author / Rating ────────────────────────────────────────────────

    #[test]
    fn area_substring_match() {
        let db = Db::new();
        insert(&db, NoteRow { area: Some("pro".into()), ..r("a") });
        insert(&db, NoteRow { area: Some("perso".into()), ..r("b") });
        insert(&db, NoteRow { area: Some("professional".into()), ..r("c") }); // "pro" is a substring
        insert(&db, NoteRow { ..r("d") });

        assert_eq!(sorted_names(&db, "area:pro"), vec!["a", "c"]);
    }

    #[test]
    fn author_substring_case_insensitive() {
        let db = Db::new();
        insert(&db, NoteRow { author: Some("Martin".into()), ..r("a") });
        insert(&db, NoteRow { author: Some("Hunt".into()), ..r("b") });
        insert(&db, NoteRow { ..r("c") });

        assert_eq!(sorted_names(&db, "author:martin"), vec!["a"]);
        assert_eq!(sorted_names(&db, "author:MARTIN"), vec!["a"]);
    }

    #[test]
    fn rating_exact_match() {
        let db = Db::new();
        insert(&db, NoteRow { rating: Some(5), ..r("a") });
        insert(&db, NoteRow { rating: Some(3), ..r("b") });
        insert(&db, NoteRow { rating: Some(5), ..r("c") });
        insert(&db, NoteRow { ..r("d") });

        assert_eq!(sorted_names(&db, "rating:5"), vec!["a", "c"]);
    }

    // ── Alias ─────────────────────────────────────────────────────────────────

    #[test]
    fn alias_exact_match() {
        let db = Db::new();
        insert(&db, NoteRow { aliases: vec!["myalias".into(), "short".into()], ..r("a") });
        insert(&db, NoteRow { aliases: vec!["otheralias".into()], ..r("b") });
        insert(&db, NoteRow { ..r("c") });

        assert_eq!(sorted_names(&db, "alias:myalias"), vec!["a"]);
        assert_eq!(sorted_names(&db, "alias:short"),   vec!["a"]);
    }

    #[test]
    fn alias_case_insensitive() {
        let db = Db::new();
        insert(&db, NoteRow { aliases: vec!["MyAlias".into()], ..r("a") });

        assert_eq!(sorted_names(&db, "alias:myalias"), vec!["a"]);
        assert_eq!(sorted_names(&db, "alias:MYALIAS"), vec!["a"]);
    }

    // ── URL ───────────────────────────────────────────────────────────────────

    #[test]
    fn url_substring_match() {
        let db = Db::new();
        insert(&db, NoteRow { url: Some("https://example.com/resource".into()), ..r("a") });
        insert(&db, NoteRow { url: Some("https://other.org".into()), ..r("b") });
        insert(&db, NoteRow { ..r("c") });

        assert_eq!(sorted_names(&db, "url:example.com"), vec!["a"]);
    }

    // ── Pinned / Locked ───────────────────────────────────────────────────────

    #[test]
    fn pinned_true_filter() {
        let db = Db::new();
        insert(&db, NoteRow { pinned: true, ..r("a") });
        insert(&db, NoteRow { pinned: false, ..r("b") });
        insert(&db, NoteRow { ..r("c") });  // default pinned=false

        assert_eq!(sorted_names(&db, "pinned:true"),  vec!["a"]);
        assert_eq!(sorted_names(&db, "pinned:false"), vec!["b", "c"]);
    }

    #[test]
    fn locked_true_filter() {
        let db = Db::new();
        insert(&db, NoteRow { locked: true, ..r("a") });
        insert(&db, NoteRow { ..r("b") });

        assert_eq!(sorted_names(&db, "locked:true"),  vec!["a"]);
        assert_eq!(sorted_names(&db, "locked:false"), vec!["b"]);
    }

    // ── Priority ──────────────────────────────────────────────────────────────

    #[test]
    fn priority_exact_match() {
        let db = Db::new();
        insert(&db, NoteRow { priority: Some("high".into()),   ..r("a") });
        insert(&db, NoteRow { priority: Some("medium".into()), ..r("b") });
        insert(&db, NoteRow { priority: Some("low".into()),    ..r("c") });
        insert(&db, NoteRow { ..r("d") });

        assert_eq!(sorted_names(&db, "priority:high"),   vec!["a"]);
        assert_eq!(sorted_names(&db, "priority:medium"), vec!["b"]);
        assert_eq!(sorted_names(&db, "priority:low"),    vec!["c"]);
    }

    // ── Project ───────────────────────────────────────────────────────────────

    #[test]
    fn project_substring_match() {
        let db = Db::new();
        insert(&db, NoteRow { project: Some("alpha".into()),    ..r("a") });
        insert(&db, NoteRow { project: Some("beta".into()),     ..r("b") });
        insert(&db, NoteRow { project: Some("alpha-v2".into()), ..r("c") });
        insert(&db, NoteRow { ..r("d") });

        assert_eq!(sorted_names(&db, "project:alpha"), vec!["a", "c"]);
    }

    // ── lastModified ──────────────────────────────────────────────────────────

    #[test]
    fn last_modified_prefix_match() {
        let db = Db::new();
        insert(&db, NoteRow { last_modified: Some("2025-05-01".into()), ..r("a") });
        insert(&db, NoteRow { last_modified: Some("2025-06-15".into()), ..r("b") });
        insert(&db, NoteRow { ..r("c") });

        // key is case-insensitive in the parser: "lastModified" → "lastmodified"
        assert_eq!(sorted_names(&db, "lastModified:2025-05"), vec!["a"]);
        assert_eq!(sorted_names(&db, "lastmodified:2025-05"), vec!["a"]);
    }

    // ── Depth ─────────────────────────────────────────────────────────────────

    #[test]
    fn depth_zero_root_notes() {
        let db = Db::new();
        insert(&db, NoteRow { ..r("Home") });
        insert(&db, NoteRow { ..r("Sub/Note") });
        insert(&db, NoteRow { ..r("Deep/Sub/Note") });

        assert_eq!(sorted_names(&db, "depth:0"), vec!["Home"]);
    }

    #[test]
    fn depth_one_single_subfolder() {
        let db = Db::new();
        insert(&db, NoteRow { ..r("Home") });
        insert(&db, NoteRow { ..r("Sub/Note A") });
        insert(&db, NoteRow { ..r("Sub/Note B") });
        insert(&db, NoteRow { ..r("Deep/Sub/Note") });

        assert_eq!(sorted_names(&db, "depth:1"), vec!["Sub/Note A", "Sub/Note B"]);
    }

    #[test]
    fn depth_two_nested_notes() {
        let db = Db::new();
        insert(&db, NoteRow { ..r("Home") });
        insert(&db, NoteRow { ..r("Sub/Note") });
        insert(&db, NoteRow { ..r("Deep/Sub/Note") });

        assert_eq!(sorted_names(&db, "depth:2"), vec!["Deep/Sub/Note"]);
    }

    // ── Logic: AND (implicit), OR, NOT ───────────────────────────────────────

    #[test]
    fn implicit_and_all_predicates_must_match() {
        let db = Db::new();
        insert(&db, NoteRow { tags: vec!["work".into()], status: Some("active".into()), ..r("a") });
        insert(&db, NoteRow { tags: vec!["work".into()], status: Some("done".into()),   ..r("b") });
        insert(&db, NoteRow { tags: vec!["other".into()], status: Some("active".into()), ..r("c") });

        assert_eq!(sorted_names(&db, "#work status:active"), vec!["a"]);
    }

    #[test]
    fn explicit_and_keyword_is_noop() {
        let db = Db::new();
        insert(&db, NoteRow { tags: vec!["work".into()], status: Some("active".into()), ..r("a") });
        insert(&db, NoteRow { tags: vec!["work".into()], status: Some("done".into()),   ..r("b") });

        assert_eq!(sorted_names(&db, "#work AND status:active"), vec!["a"]);
    }

    #[test]
    fn multiple_tag_filters_all_must_match() {
        let db = Db::new();
        insert(&db, NoteRow { tags: vec!["work".into(), "urgent".into()], ..r("a") });
        insert(&db, NoteRow { tags: vec!["work".into()], ..r("b") });
        insert(&db, NoteRow { tags: vec!["urgent".into()], ..r("c") });

        assert_eq!(sorted_names(&db, "#work #urgent"), vec!["a"]);
    }

    #[test]
    fn or_operator() {
        let db = Db::new();
        insert(&db, NoteRow { tags: vec!["work".into()], ..r("a") });
        insert(&db, NoteRow { tags: vec!["personal".into()], ..r("b") });
        insert(&db, NoteRow { tags: vec!["other".into()], ..r("c") });

        assert_eq!(sorted_names(&db, "#work OR #personal"), vec!["a", "b"]);
    }

    #[test]
    fn three_way_or() {
        let db = Db::new();
        insert(&db, NoteRow { tags: vec!["a".into()], ..r("nota") });
        insert(&db, NoteRow { tags: vec!["b".into()], ..r("notb") });
        insert(&db, NoteRow { tags: vec!["c".into()], ..r("notc") });
        insert(&db, NoteRow { tags: vec!["d".into()], ..r("notd") });

        assert_eq!(sorted_names(&db, "#a OR #b OR #c"), vec!["nota", "notb", "notc"]);
    }

    #[test]
    fn or_and_precedence() {
        // `A OR B C` → A OR (B AND C)
        let db = Db::new();
        insert(&db, NoteRow { tags: vec!["work".into()],    status: Some("done".into()),   ..r("a") }); // A only
        insert(&db, NoteRow { tags: vec!["journal".into()], status: Some("active".into()), ..r("b") }); // B AND C
        insert(&db, NoteRow { tags: vec!["journal".into()], status: Some("done".into()),   ..r("c") }); // B not C
        insert(&db, NoteRow { tags: vec!["other".into()],   status: Some("active".into()), ..r("d") }); // neither

        assert_eq!(sorted_names(&db, "#work OR #journal status:active"), vec!["a", "b"]);
    }

    #[test]
    fn not_creates_global_exclusion() {
        let db = Db::new();
        insert(&db, NoteRow { tags: vec!["work".into()], status: Some("active".into()), ..r("a") });
        insert(&db, NoteRow { tags: vec!["work".into()], status: Some("done".into()),   ..r("b") });
        insert(&db, NoteRow { tags: vec!["other".into()], ..r("c") });

        // NOT status:done → b and c are "not done", but we also filter #work → only a
        assert_eq!(sorted_names(&db, "#work NOT status:done"), vec!["a"]);
    }

    #[test]
    fn not_applies_globally_across_or_groups() {
        // `A OR B NOT C` → (A OR B) AND NOT C
        let db = Db::new();
        insert(&db, NoteRow { tags: vec!["work".into()],     status: Some("done".into()),   ..r("a") }); // OR group A, but globally excluded
        insert(&db, NoteRow { tags: vec!["personal".into()], status: Some("active".into()), ..r("b") }); // OR group B, not excluded
        insert(&db, NoteRow { tags: vec!["work".into()],     status: Some("active".into()), ..r("c") }); // OR group A, not excluded

        assert_eq!(sorted_names(&db, "#work OR #personal NOT status:done"), vec!["b", "c"]);
    }

    #[test]
    fn not_tag_excludes_matching_notes() {
        let db = Db::new();
        insert(&db, NoteRow { tags: vec!["work".into()], ..r("a") });
        insert(&db, NoteRow { tags: vec!["personal".into()], ..r("b") });
        insert(&db, NoteRow { ..r("c") });

        assert_eq!(sorted_names(&db, "NOT #work"), vec!["b", "c"]);
    }

    // ── Empty query ───────────────────────────────────────────────────────────

    #[test]
    fn empty_query_returns_all_sorted_by_name() {
        let db = Db::new();
        insert(&db, NoteRow { ..r("c") });
        insert(&db, NoteRow { ..r("a") });
        insert(&db, NoteRow { ..r("b") });

        assert_eq!(names(&db, ""), vec!["a", "b", "c"]);
    }

    // ── recent / oldest ───────────────────────────────────────────────────────

    #[test]
    fn recent_returns_n_most_recently_modified() {
        let db = Db::new();
        insert(&db, NoteRow { modified_at: 1, ..r("old") });
        insert(&db, NoteRow { modified_at: 3, ..r("new") });
        insert(&db, NoteRow { modified_at: 2, ..r("mid") });

        // Returned in descending modified_at order
        assert_eq!(names(&db, "recent:2"), vec!["new", "mid"]);
    }

    #[test]
    fn oldest_returns_n_least_recently_modified() {
        let db = Db::new();
        insert(&db, NoteRow { modified_at: 1, ..r("old") });
        insert(&db, NoteRow { modified_at: 3, ..r("new") });
        insert(&db, NoteRow { modified_at: 2, ..r("mid") });

        // Returned in ascending modified_at order
        assert_eq!(names(&db, "oldest:2"), vec!["old", "mid"]);
    }

    #[test]
    fn recent_larger_than_count_returns_all() {
        let db = Db::new();
        insert(&db, NoteRow { modified_at: 1, ..r("a") });
        insert(&db, NoteRow { modified_at: 2, ..r("b") });

        assert_eq!(names(&db, "recent:100").len(), 2);
    }

    #[test]
    fn recent_with_tag_filter() {
        let db = Db::new();
        insert(&db, NoteRow { tags: vec!["work".into()], modified_at: 3, ..r("b") });
        insert(&db, NoteRow { tags: vec!["work".into()], modified_at: 1, ..r("a") });
        insert(&db, NoteRow { tags: vec!["work".into()], modified_at: 2, ..r("mid") });
        insert(&db, NoteRow { tags: vec!["other".into()], modified_at: 99, ..r("x") });

        // Most-recent 2 among work notes: b(3) and mid(2)
        assert_eq!(names(&db, "#work recent:2"), vec!["b", "mid"]);
    }

    // ── order by ──────────────────────────────────────────────────────────────

    #[test]
    fn order_by_name_asc() {
        let db = Db::new();
        insert(&db, NoteRow { ..r("charlie") });
        insert(&db, NoteRow { ..r("alpha") });
        insert(&db, NoteRow { ..r("beta") });

        assert_eq!(names(&db, "order by name"), vec!["alpha", "beta", "charlie"]);
    }

    #[test]
    fn order_by_name_desc() {
        let db = Db::new();
        insert(&db, NoteRow { ..r("charlie") });
        insert(&db, NoteRow { ..r("alpha") });
        insert(&db, NoteRow { ..r("beta") });

        assert_eq!(names(&db, "order by name desc"), vec!["charlie", "beta", "alpha"]);
    }

    #[test]
    fn order_by_date_asc_none_last() {
        let db = Db::new();
        insert(&db, NoteRow { date: Some("2025-03-01".into()), ..r("c") });
        insert(&db, NoteRow { date: Some("2025-01-15".into()), ..r("a") });
        insert(&db, NoteRow { date: Some("2025-02-10".into()), ..r("b") });
        insert(&db, NoteRow { ..r("no_date") }); // None → sorted last

        assert_eq!(names(&db, "order by date"), vec!["a", "b", "c", "no_date"]);
    }

    #[test]
    fn order_by_date_desc_none_last() {
        // None always sorts last, even in descending order
        let db = Db::new();
        insert(&db, NoteRow { date: Some("2025-03-01".into()), ..r("c") });
        insert(&db, NoteRow { date: Some("2025-01-15".into()), ..r("a") });
        insert(&db, NoteRow { date: Some("2025-02-10".into()), ..r("b") });
        insert(&db, NoteRow { ..r("no_date") });

        assert_eq!(names(&db, "order by date desc"), vec!["c", "b", "a", "no_date"]);
    }

    #[test]
    fn order_by_priority_high_first() {
        let db = Db::new();
        insert(&db, NoteRow { priority: Some("low".into()),    ..r("low") });
        insert(&db, NoteRow { priority: Some("high".into()),   ..r("high") });
        insert(&db, NoteRow { priority: Some("medium".into()), ..r("medium") });
        insert(&db, NoteRow { ..r("none") });

        assert_eq!(names(&db, "order by priority"), vec!["high", "medium", "low", "none"]);
    }

    #[test]
    fn order_by_rating_desc_none_last() {
        // None always sorts last, even in descending order
        let db = Db::new();
        insert(&db, NoteRow { rating: Some(3), ..r("three") });
        insert(&db, NoteRow { rating: Some(5), ..r("five") });
        insert(&db, NoteRow { rating: Some(4), ..r("four") });
        insert(&db, NoteRow { ..r("none") });

        assert_eq!(names(&db, "order by rating desc"), vec!["five", "four", "three", "none"]);
    }

    #[test]
    fn order_by_priority_desc_none_last() {
        let db = Db::new();
        insert(&db, NoteRow { priority: Some("high".into()),   ..r("high") });
        insert(&db, NoteRow { priority: Some("medium".into()), ..r("medium") });
        insert(&db, NoteRow { priority: Some("low".into()),    ..r("low") });
        insert(&db, NoteRow { ..r("none") });

        // desc: low first, then medium, high, None last
        assert_eq!(names(&db, "order by priority desc"), vec!["low", "medium", "high", "none"]);
    }

    #[test]
    fn order_by_modified_asc() {
        let db = Db::new();
        insert(&db, NoteRow { modified_at: 30, ..r("c") });
        insert(&db, NoteRow { modified_at: 10, ..r("a") });
        insert(&db, NoteRow { modified_at: 20, ..r("b") });

        assert_eq!(names(&db, "order by modified"), vec!["a", "b", "c"]);
    }

    #[test]
    fn order_by_without_direction_defaults_to_asc() {
        let db = Db::new();
        insert(&db, NoteRow { ..r("z") });
        insert(&db, NoteRow { ..r("a") });
        insert(&db, NoteRow { ..r("m") });

        assert_eq!(names(&db, "order by name"), vec!["a", "m", "z"]);
    }

    // ── recent + order by combination ─────────────────────────────────────────

    #[test]
    fn recent_then_order_by_re_sorts() {
        // recent:2 picks the 2 newest, then order by name re-sorts them
        let db = Db::new();
        insert(&db, NoteRow { modified_at: 3, ..r("charlie") });
        insert(&db, NoteRow { modified_at: 1, ..r("alpha") });
        insert(&db, NoteRow { modified_at: 2, ..r("beta") });

        // recent:2 → charlie(3), beta(2); order by name → beta, charlie
        assert_eq!(names(&db, "recent:2 order by name"), vec!["beta", "charlie"]);
    }

    // ── Complex combinations ──────────────────────────────────────────────────

    #[test]
    fn tag_status_order_combined() {
        let db = Db::new();
        insert(&db, NoteRow { tags: vec!["work".into()], status: Some("active".into()), modified_at: 2, ..r("b") });
        insert(&db, NoteRow { tags: vec!["work".into()], status: Some("active".into()), modified_at: 1, ..r("a") });
        insert(&db, NoteRow { tags: vec!["work".into()], status: Some("done".into()),   modified_at: 3, ..r("c") });
        insert(&db, NoteRow { tags: vec!["other".into()], status: Some("active".into()), ..r("d") });

        assert_eq!(names(&db, "#work status:active order by name"), vec!["a", "b"]);
    }

    #[test]
    fn tag_and_date_filter() {
        let db = Db::new();
        insert(&db, NoteRow { tags: vec!["journal".into()], date: Some("2025-01-10".into()), ..r("jan") });
        insert(&db, NoteRow { tags: vec!["journal".into()], date: Some("2025-02-15".into()), ..r("feb") });
        insert(&db, NoteRow { tags: vec!["meeting".into()], date: Some("2025-01-05".into()), ..r("mtg") });

        assert_eq!(sorted_names(&db, "#journal date:2025-01"), vec!["jan"]);
    }

    #[test]
    fn or_groups_with_not_and_order() {
        let db = Db::new();
        insert(&db, NoteRow { tags: vec!["work".into()],     status: Some("active".into()), ..r("wa") });
        insert(&db, NoteRow { tags: vec!["work".into()],     status: Some("done".into()),   ..r("wd") });
        insert(&db, NoteRow { tags: vec!["personal".into()], status: Some("active".into()), ..r("pa") });
        insert(&db, NoteRow { tags: vec!["personal".into()], status: Some("done".into()),   ..r("pd") });

        // (#work OR #personal) AND NOT status:done → wa and pa
        assert_eq!(names(&db, "#work OR #personal NOT status:done order by name"), vec!["pa", "wa"]);
    }

    // ── Edge cases ────────────────────────────────────────────────────────────

    #[test]
    fn invalid_rating_token_silently_ignored() {
        // parser does `Err(_) => continue` → predicate discarded → all notes pass
        let db = Db::new();
        insert(&db, NoteRow { rating: Some(5), ..r("a") });
        insert(&db, NoteRow { ..r("b") });

        assert_eq!(sorted_names(&db, "rating:not_a_number"), vec!["a", "b"]);
    }

    #[test]
    fn unknown_key_value_silently_ignored() {
        let db = Db::new();
        insert(&db, NoteRow { ..r("a") });
        insert(&db, NoteRow { ..r("b") });

        assert_eq!(sorted_names(&db, "unknownfield:value"), vec!["a", "b"]);
    }

    #[test]
    fn invalid_depth_token_silently_ignored() {
        let db = Db::new();
        insert(&db, NoteRow { ..r("a") });

        assert_eq!(sorted_names(&db, "depth:notanumber"), vec!["a"]);
    }

    #[test]
    fn not_pinned_true_equivalent_to_pinned_false() {
        let db = Db::new();
        insert(&db, NoteRow { pinned: true,  ..r("pinned") });
        insert(&db, NoteRow { pinned: false, ..r("unpinned") });

        assert_eq!(sorted_names(&db, "NOT pinned:true"),  vec!["unpinned"]);
        assert_eq!(sorted_names(&db, "NOT pinned:false"), vec!["pinned"]);
    }

    #[test]
    fn whitespace_only_query_returns_all() {
        let db = Db::new();
        insert(&db, NoteRow { ..r("a") });
        insert(&db, NoteRow { ..r("b") });

        assert_eq!(names(&db, "   "), vec!["a", "b"]);
    }

    #[test]
    fn bare_word_case_insensitive() {
        let db = Db::new();
        insert(&db, NoteRow { ..r("Meeting Notes") });
        insert(&db, NoteRow { ..r("other") });

        assert_eq!(sorted_names(&db, "MEETING"), vec!["Meeting Notes"]);
    }

    #[test]
    fn status_is_case_sensitive() {
        // status: uses exact match, no lowercasing
        let db = Db::new();
        insert(&db, NoteRow { status: Some("Active".into()), ..r("a") });
        insert(&db, NoteRow { status: Some("active".into()), ..r("b") });

        assert_eq!(sorted_names(&db, "status:active"), vec!["b"]);
        assert_eq!(sorted_names(&db, "status:Active"), vec!["a"]);
    }

    // ── Has / not-has field filters ───────────────────────────────────────────
    // `due:` (empty prefix) = "has a due date"; `NOT due:` = "has no due date".
    // Same logic applies to any prefix field (date:, lastModified:, etc.).

    #[test]
    fn due_colon_empty_matches_notes_with_any_due() {
        let db = Db::new();
        insert(&db, NoteRow { due: Some("2026-06-01".into()), ..r("has") });
        insert(&db, NoteRow { ..r("no") });

        assert_eq!(sorted_names(&db, "due:"), vec!["has"]);
    }

    #[test]
    fn not_due_colon_empty_matches_notes_without_due() {
        let db = Db::new();
        insert(&db, NoteRow { due: Some("2026-06-01".into()), ..r("has") });
        insert(&db, NoteRow { ..r("no") });

        assert_eq!(sorted_names(&db, "NOT due:"), vec!["no"]);
    }

    #[test]
    fn date_colon_empty_matches_notes_with_any_date() {
        let db = Db::new();
        insert(&db, NoteRow { date: Some("2026-01-01".into()), ..r("has") });
        insert(&db, NoteRow { ..r("no") });

        assert_eq!(sorted_names(&db, "date:"), vec!["has"]);
        assert_eq!(sorted_names(&db, "NOT date:"), vec!["no"]);
    }

    // ── Date comparison operators ─────────────────────────────────────────────

    #[test]
    fn due_gte_matches_on_or_after() {
        let db = Db::new();
        insert(&db, NoteRow { due: Some("2026-05-01".into()), ..r("past") });
        insert(&db, NoteRow { due: Some("2026-06-06".into()), ..r("today") });
        insert(&db, NoteRow { due: Some("2026-07-01".into()), ..r("future") });
        insert(&db, NoteRow { ..r("none") });

        assert_eq!(sorted_names(&db, "due>=:2026-06-06"), vec!["future", "today"]);
    }

    #[test]
    fn due_gt_excludes_exact_match() {
        let db = Db::new();
        insert(&db, NoteRow { due: Some("2026-06-06".into()), ..r("today") });
        insert(&db, NoteRow { due: Some("2026-06-07".into()), ..r("tomorrow") });
        insert(&db, NoteRow { ..r("none") });

        assert_eq!(sorted_names(&db, "due>:2026-06-06"), vec!["tomorrow"]);
    }

    #[test]
    fn due_lte_matches_on_or_before() {
        let db = Db::new();
        insert(&db, NoteRow { due: Some("2026-05-01".into()), ..r("past") });
        insert(&db, NoteRow { due: Some("2026-06-06".into()), ..r("today") });
        insert(&db, NoteRow { due: Some("2026-07-01".into()), ..r("future") });
        insert(&db, NoteRow { ..r("none") });

        assert_eq!(sorted_names(&db, "due<=:2026-06-06"), vec!["past", "today"]);
    }

    #[test]
    fn due_lt_excludes_exact_match() {
        let db = Db::new();
        insert(&db, NoteRow { due: Some("2026-06-05".into()), ..r("yesterday") });
        insert(&db, NoteRow { due: Some("2026-06-06".into()), ..r("today") });
        insert(&db, NoteRow { ..r("none") });

        assert_eq!(sorted_names(&db, "due<:2026-06-06"), vec!["yesterday"]);
    }

    #[test]
    fn due_range_between_two_dates() {
        let db = Db::new();
        insert(&db, NoteRow { due: Some("2026-06-01".into()), ..r("a") });
        insert(&db, NoteRow { due: Some("2026-06-15".into()), ..r("b") });
        insert(&db, NoteRow { due: Some("2026-07-01".into()), ..r("c") });
        insert(&db, NoteRow { due: Some("2026-05-31".into()), ..r("d") });
        insert(&db, NoteRow { ..r("none") });

        assert_eq!(sorted_names(&db, "due>=:2026-06-01 due<=:2026-06-30"), vec!["a", "b"]);
    }

    #[test]
    fn due_cmp_excludes_notes_without_due() {
        let db = Db::new();
        insert(&db, NoteRow { due: Some("2026-06-01".into()), ..r("has_due") });
        insert(&db, NoteRow { ..r("no_due") });

        // Notes without a due field never match a comparison predicate
        assert_eq!(sorted_names(&db, "due>=:2026-01-01"), vec!["has_due"]);
        assert_eq!(sorted_names(&db, "due<=:2099-12-31"), vec!["has_due"]);
    }

    #[test]
    fn date_cmp_operators() {
        let db = Db::new();
        insert(&db, NoteRow { date: Some("2025-03-01".into()), ..r("old") });
        insert(&db, NoteRow { date: Some("2026-06-06".into()), ..r("now") });
        insert(&db, NoteRow { date: Some("2026-12-01".into()), ..r("future") });
        insert(&db, NoteRow { ..r("none") });

        assert_eq!(sorted_names(&db, "date>=:2026-06-06"), vec!["future", "now"]);
        assert_eq!(sorted_names(&db, "date<:2026-06-06"),  vec!["old"]);
    }

    #[test]
    fn due_cmp_combined_with_status_filter() {
        let db = Db::new();
        insert(&db, NoteRow { due: Some("2026-06-01".into()), status: Some("todo".into()),   ..r("a") });
        insert(&db, NoteRow { due: Some("2026-06-01".into()), status: Some("done".into()),   ..r("b") });
        insert(&db, NoteRow { due: Some("2026-07-01".into()), status: Some("todo".into()),   ..r("c") });
        insert(&db, NoteRow { ..r("none") });

        // Overdue todos only
        assert_eq!(sorted_names(&db, "due<=:2026-06-30 status:todo"), vec!["a"]);
    }
}
