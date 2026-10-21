//! Live TUI for Pedro's Factory: one tab per project, an agent tree per orchestrator session.
//! Run: cargo run --release -- [PATH ...]   (PATH = a project with .factory/ or a folder to scan)

use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Paragraph, Tabs};
use ratatui::Frame;
use serde_json::Value;

const SCAN_DEPTH: usize = 3;
const SKIP_DIRS: &[&str] = &["node_modules", "Pictures", "Videos", "Music", "Downloads", "Documents", "Library", "target"];
const ROOT_WINDOW: u64 = 3600; // orchestrator sessions touched within this many seconds get a tree
const RUNNING_S: u64 = 30; // transcript written this recently -> running
const CARD_H: u16 = 5;
const INDENT: u16 = 4;

// Everforest dark (medium)
const BG0: Color = Color::Rgb(0x2d, 0x35, 0x3b);
const BG1: Color = Color::Rgb(0x34, 0x3f, 0x44);
const BG3: Color = Color::Rgb(0x47, 0x52, 0x58);
const FG: Color = Color::Rgb(0xd3, 0xc6, 0xaa);
const RED: Color = Color::Rgb(0xe6, 0x7e, 0x80);
const ORANGE: Color = Color::Rgb(0xe6, 0x98, 0x75);
const YELLOW: Color = Color::Rgb(0xdb, 0xbc, 0x7f);
const GREEN: Color = Color::Rgb(0xa7, 0xc0, 0x80);
const AQUA: Color = Color::Rgb(0x83, 0xc0, 0x92);
const BLUE: Color = Color::Rgb(0x7f, 0xbb, 0xb3);
const PURPLE: Color = Color::Rgb(0xd6, 0x99, 0xb6);
const GREY: Color = Color::Rgb(0x85, 0x92, 0x89);
const GREY2: Color = Color::Rgb(0x9d, 0xa9, 0xa0);
const BG_GREEN: Color = Color::Rgb(0x42, 0x50, 0x47);
const BG_RED: Color = Color::Rgb(0x51, 0x40, 0x45);
const BG_YELLOW: Color = Color::Rgb(0x4d, 0x4c, 0x43);
const BG_BLUE: Color = Color::Rgb(0x3a, 0x51, 0x5d);

// ---------- data ----------

/// One transcript, read incrementally from `offset` on every refresh.
#[derive(Default)]
struct Transcript {
    offset: u64,
    cwd: String,
    model: String,
    title: String,
    prompt: String,
    cost: f64,
    first_ts: u64,
    last_ts: u64,
    last_msg_id: String,
    spawned: Vec<String>,
    events: VecDeque<(u64, String)>,
    popped: u64, // events dropped from the front, so `popped + index` is a stable event id
    waiting: bool,
    blocked: bool,
    last: String,
}

/// $/MTok (input, output, cache read). Cache writes are billed at 1.25x input.
/// shortcut: API list prices from the claude-api skill (2026-10); a Pro/Max plan is not billed this way.
fn price(model: &str) -> (f64, f64, f64) {
    if model.contains("fable") || model.contains("mythos") {
        (10.0, 50.0, 0.25)
    } else if model.contains("opus") {
        (4.0, 20.0, 0.20)
    } else if model.contains("sonnet") {
        (2.0, 10.0, 0.20)
    } else if model.contains("haiku") {
        (0.10, 0.50, 0.01)
    } else {
        (0.0, 0.0, 0.0)
    }
}

/// "2026-10-20T02:12:19.215Z" -> unix seconds.
fn parse_ts(s: &str) -> Option<u64> {
    let n = |a: usize, b: usize| s.get(a..b)?.parse::<i64>().ok();
    let (y, m, d) = (n(0, 4)?, n(5, 7)?, n(8, 10)?);
    let (hh, mm, ss) = (n(11, 13)?, n(14, 16)?, n(17, 19)?);
    // days from civil (Howard Hinnant)
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * ((m + 9) % 12) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    u64::try_from(days * 86400 + hh * 3600 + mm * 60 + ss).ok()
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn mtime(p: &Path) -> u64 {
    fs::metadata(p).and_then(|m| m.modified()).ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0)
}

fn one_line(s: &str, max: usize) -> String {
    let s: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if s.chars().count() > max { s.chars().take(max.saturating_sub(1)).collect::<String>() + "…" } else { s }
}

fn summarize_tool(name: &str, input: &Value) -> String {
    for key in ["command", "file_path", "pattern", "description", "query", "url", "prompt"] {
        if let Some(v) = input.get(key).and_then(Value::as_str) {
            let v = if key == "file_path" { Path::new(v).file_name().and_then(|f| f.to_str()).unwrap_or(v) } else { v };
            return format!("{name} {}", v.trim()); // full text; cut only when drawn
        }
    }
    name.to_string()
}

fn prompt_text(content: &Value) -> Option<String> {
    let text = match content {
        Value::String(s) => s.clone(),
        Value::Array(items) => items.iter().find_map(|i| i.get("text").and_then(Value::as_str)).unwrap_or("").to_string(),
        _ => return None,
    };
    if let (Some(a), Some(b)) = (text.find("<command-args>"), text.find("</command-args>")) {
        // "/factory:build <feature>" is the task; argless commands like /reload-plugins are not
        let args = text[a + 14..b].trim();
        let name = text.find("<command-name>").and_then(|i| text[i + 14..].split('<').next()).unwrap_or("");
        return (!args.is_empty()).then(|| format!("{name} {args}"));
    }
    (!text.trim().is_empty() && !text.starts_with('<')).then_some(text)
}

impl Transcript {
    fn update(&mut self, path: &Path) {
        let Ok(mut f) = fs::File::open(path) else { return };
        let len = f.metadata().map(|m| m.len()).unwrap_or(0);
        if len < self.offset {
            *self = Transcript::default(); // file was rewritten
        }
        if len == self.offset || f.seek(SeekFrom::Start(self.offset)).is_err() {
            return;
        }
        let mut buf = Vec::new();
        if f.read_to_end(&mut buf).is_err() {
            return;
        }
        let Some(end) = buf.iter().rposition(|&b| b == b'\n') else { return }; // wait for a full line
        self.offset += end as u64 + 1;
        for line in buf[..end].split(|&b| b == b'\n') {
            if let Ok(v) = serde_json::from_slice::<Value>(line) {
                self.apply(&v);
            }
        }
    }

    fn apply(&mut self, v: &Value) {
        if self.cwd.is_empty() {
            if let Some(c) = v.get("cwd").and_then(Value::as_str) {
                self.cwd = c.to_string();
            }
        }
        let ts = v.get("timestamp").and_then(Value::as_str).and_then(parse_ts);
        if let Some(ts) = ts {
            if self.first_ts == 0 {
                self.first_ts = ts;
            }
            self.last_ts = ts;
        }
        let ts = ts.unwrap_or(self.last_ts);
        match v.get("type").and_then(Value::as_str) {
            Some("ai-title") => self.title = v["aiTitle"].as_str().unwrap_or("").to_string(),
            Some("user") => {
                self.waiting = false;
                if self.prompt.is_empty() && v.get("isMeta").is_none() {
                    if let Some(p) = prompt_text(&v["message"]["content"]) {
                        self.prompt = one_line(&p, 300);
                    }
                }
            }
            Some("assistant") => {
                let msg = &v["message"];
                if let Some(m) = msg["model"].as_str().filter(|m| !m.starts_with('<')) {
                    self.model = m.to_string();
                }
                // Claude Code writes one line per content block, each repeating the message usage.
                let id = msg["id"].as_str().unwrap_or("");
                if id != self.last_msg_id {
                    self.last_msg_id = id.to_string();
                    let u = &msg["usage"];
                    let t = |k: &str| u[k].as_f64().unwrap_or(0.0);
                    let (pi, po, pr) = price(&self.model);
                    self.cost += (t("input_tokens") * pi
                        + t("output_tokens") * po
                        + t("cache_creation_input_tokens") * pi * 1.25
                        + t("cache_read_input_tokens") * pr)
                        / 1e6;
                }
                for item in msg["content"].as_array().into_iter().flatten() {
                    match item["type"].as_str() {
                        Some("tool_use") => {
                            let name = item["name"].as_str().unwrap_or("?");
                            if name == "Agent" || name == "Task" {
                                self.spawned.push(item["id"].as_str().unwrap_or("").to_string());
                            }
                            self.waiting = matches!(name, "AskUserQuestion" | "ExitPlanMode");
                            self.last = summarize_tool(name, &item["input"]);
                            self.push(ts, format!("▸ {}", self.last));
                        }
                        Some("text") => {
                            let text = item["text"].as_str().unwrap_or("");
                            if !text.trim().is_empty() {
                                self.blocked = text.contains("STATUS: BLOCKED");
                                self.push(ts, format!("✎ {}", text.trim()));
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    fn push(&mut self, ts: u64, line: String) {
        self.events.push_back((ts, line));
        if self.events.len() > 300 {
            self.events.pop_front();
            self.popped += 1;
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Status {
    Running,
    NeedsYou,
    Blocked,
    Idle,
    Done,
}

struct Node {
    path: PathBuf,
    name: String,
    model: String,
    task: String,
    status: Status,
    cost: f64,
    secs: u64,
    last: String,
    children: Vec<usize>,
}

struct Job {
    id: String,
    stage: String,
    round: i64,
    wps: (usize, usize),
    verdicts: String,
    live: bool,
}

struct Project {
    root: PathBuf,
    nodes: Vec<Node>,
    roots: Vec<usize>,
    jobs: Vec<Job>,
}

fn find_projects(bases: &[PathBuf]) -> Vec<PathBuf> {
    fn walk(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
        if dir.join(".factory").is_dir() {
            out.push(dir.to_path_buf());
        }
        if depth >= SCAN_DEPTH {
            return;
        }
        let Ok(rd) = fs::read_dir(dir) else { return };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if e.file_type().map(|t| t.is_dir()).unwrap_or(false) && !name.starts_with('.') && !SKIP_DIRS.contains(&name.as_str()) {
                walk(&e.path(), depth + 1, out);
            }
        }
    }
    let mut out = Vec::new();
    for b in bases {
        walk(b, 0, &mut out);
    }
    out.sort_by_key(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()));
    out.dedup();
    out
}

fn load_pinned(config: &Path) -> Vec<PathBuf> {
    fs::read_to_string(config)
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(PathBuf::from)
        .collect()
}

fn save_pinned(config: &Path, pinned: &[PathBuf]) -> std::io::Result<()> {
    if let Some(dir) = config.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut text = String::from("# Projects factory-dash shows as tabs, one path per line. Edit here or press a in the app.\n");
    for p in pinned {
        text.push_str(&format!("{}\n", p.display()));
    }
    let tmp = config.with_extension("tmp");
    fs::write(&tmp, text)?;
    fs::rename(tmp, config) // atomic, so a crash never leaves half a list
}

fn newest_mtime(dir: &Path) -> u64 {
    let mut newest = mtime(dir);
    for e in fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = e.path();
        newest = newest.max(if p.is_dir() { newest_mtime(&p) } else { mtime(&p) });
    }
    newest
}

fn read_verdicts(job_dir: &Path) -> String {
    let last = fs::read_dir(job_dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| e.file_name().to_str()?.strip_prefix("round-")?.parse::<u32>().ok())
        .max();
    let Some(n) = last else { return String::new() };
    let mut out: Vec<String> = fs::read_dir(job_dir.join(format!("round-{n}")))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_str()?.strip_suffix(".md")?.to_string();
            let head: String = fs::read_to_string(e.path()).ok()?.chars().take(200).collect();
            let verdict = head.split("VERDICT:").nth(1)?.split_whitespace().next()?.trim_matches('*').to_string();
            Some(format!("{name}:{verdict}"))
        })
        .collect();
    out.sort();
    out.join(" ")
}

fn load_jobs(root: &Path) -> Vec<Job> {
    let dir = root.join(".factory").join("jobs");
    let mut jobs: Vec<Job> = fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let v: Value = serde_json::from_str(&fs::read_to_string(e.path().join("job.json")).ok()?).ok()?;
            let wps = v["work_packages"].as_array().cloned().unwrap_or_default();
            let done = wps.iter().filter(|w| matches!(w["status"].as_str(), Some("merged" | "done"))).count();
            Some(Job {
                id: v["id"].as_str().map(String::from).unwrap_or_else(|| e.file_name().to_string_lossy().to_string()),
                stage: v["stage"].as_str().unwrap_or("?").to_string(),
                round: v["round"].as_i64().unwrap_or(0),
                wps: (done, wps.len()),
                verdicts: read_verdicts(&e.path()),
                live: now().saturating_sub(newest_mtime(&e.path())) < 120,
            })
        })
        .collect();
    jobs.sort_by(|a, b| b.id.cmp(&a.id));
    jobs
}

/// Longest project root that contains `cwd`.
fn owner<'a>(cwd: &str, roots: &'a [PathBuf]) -> Option<&'a PathBuf> {
    roots.iter().filter(|r| Path::new(cwd).starts_with(r)).max_by_key(|r| r.as_os_str().len())
}

fn short_agent(t: &str) -> String {
    t.rsplit(':').next().unwrap_or(t).to_string()
}

fn short_model(m: &str) -> String {
    // claude-opus-5-5 -> Opus 5.5
    let parts: Vec<&str> = m.trim_start_matches("claude-").split('-').collect();
    match parts.as_slice() {
        [fam, rest @ ..] if !rest.is_empty() => {
            let mut f = fam.to_string();
            f[..1].make_ascii_uppercase();
            format!("{f} {}", rest.join("."))
        }
        _ => m.to_string(),
    }
}

struct Store {
    projects_dir: PathBuf,
    cache: HashMap<PathBuf, Transcript>,
}

impl Store {
    fn transcript(&mut self, p: &Path) -> &Transcript {
        let t = self.cache.entry(p.to_path_buf()).or_default();
        t.update(p);
        t
    }

    fn build(&mut self, roots: &[PathBuf]) -> Vec<Project> {
        let mut projects: Vec<Project> =
            roots.iter().map(|r| Project { root: r.clone(), nodes: vec![], roots: vec![], jobs: load_jobs(r) }).collect();
        let now = now();
        let mut sessions: Vec<(PathBuf, u64)> = fs::read_dir(&self.projects_dir)
            .into_iter()
            .flatten()
            .flatten()
            .flat_map(|d| fs::read_dir(d.path()).into_iter().flatten().flatten())
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "jsonl"))
            .map(|p| {
                let m = mtime(&p);
                (p, m)
            })
            .filter(|(_, m)| now.saturating_sub(*m) < ROOT_WINDOW)
            .collect();
        sessions.sort_by_key(|(_, m)| std::cmp::Reverse(*m));

        for (path, _) in sessions {
            let cwd = self.transcript(&path).cwd.clone();
            let Some(pi) = owner(&cwd, roots).and_then(|r| roots.iter().position(|x| x == r)) else { continue };
            let root_idx = self.add_node(&mut projects[pi], &path, None);
            projects[pi].roots.push(root_idx);

            // Subagents live in <session>/subagents/agent-*.jsonl with a .meta.json beside them.
            let sub_dir = path.with_extension("").join("subagents");
            let mut subs: Vec<(PathBuf, Value)> = fs::read_dir(&sub_dir)
                .into_iter()
                .flatten()
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|x| x == "jsonl"))
                .map(|p| {
                    let meta = fs::read_to_string(p.with_extension("meta.json")).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(Value::Null);
                    (p, meta)
                })
                .collect();
            subs.sort_by_key(|(p, _)| self.transcript(p).first_ts);
            let mut by_tool_id: HashMap<String, usize> = HashMap::new();
            for id in &self.transcript(&path).spawned {
                by_tool_id.insert(id.clone(), root_idx);
            }
            let proj = &mut projects[pi];
            let mut pending: Vec<(PathBuf, Value)> = subs;
            // Attach children whose spawning tool call is known; repeat for deeper levels.
            loop {
                let before = pending.len();
                let mut rest = Vec::new();
                for (p, meta) in pending {
                    let tid = meta["toolUseId"].as_str().unwrap_or("").to_string();
                    let parent = by_tool_id.get(&tid).copied().or((meta["spawnDepth"].as_u64().unwrap_or(1) <= 1).then_some(root_idx));
                    match parent {
                        Some(parent) => {
                            let idx = self.add_node(proj, &p, Some(&meta));
                            proj.nodes[parent].children.push(idx);
                            for id in &self.transcript(&p).spawned {
                                by_tool_id.insert(id.clone(), idx);
                            }
                        }
                        None => rest.push((p, meta)),
                    }
                }
                pending = rest;
                if pending.is_empty() || pending.len() == before {
                    for (p, meta) in pending {
                        let idx = self.add_node(proj, &p, Some(&meta)); // orphan: hang it off the root
                        proj.nodes[root_idx].children.push(idx);
                    }
                    break;
                }
            }
        }
        projects
    }

    fn add_node(&mut self, proj: &mut Project, path: &Path, meta: Option<&Value>) -> usize {
        let age = now().saturating_sub(mtime(path));
        let t = self.transcript(path);
        let finished = meta.is_some_and(|m| m["finished"].as_bool() == Some(true));
        let status = if t.waiting && !finished {
            Status::NeedsYou
        } else if finished {
            if t.blocked { Status::Blocked } else { Status::Done }
        } else if age < RUNNING_S {
            Status::Running
        } else {
            Status::Idle
        };
        let (name, task) = match meta {
            Some(m) => (short_agent(m["agentType"].as_str().unwrap_or("agent")), m["description"].as_str().unwrap_or("").to_string()),
            None => ("orchestrator".to_string(), if t.prompt.is_empty() { t.title.clone() } else { t.prompt.clone() }),
        };
        proj.nodes.push(Node {
            path: path.to_path_buf(),
            name,
            model: short_model(&t.model),
            task,
            status,
            cost: t.cost,
            secs: t.last_ts.saturating_sub(t.first_ts),
            last: t.last.clone(),
            children: vec![],
        });
        proj.nodes.len() - 1
    }
}

// ---------- ui ----------

struct Row {
    node: usize,
    depth: usize,
    lasts: Vec<bool>, // lasts[k]: is the ancestor at depth k+1 (or self, at k = depth-1) the last child
}

struct App {
    bases: Vec<PathBuf>,
    args_mode: bool,           // paths given on the command line: show exactly those, save nothing
    config: PathBuf,           // pinned projects, one path per line
    pinned: Vec<PathBuf>,
    discovered: Vec<PathBuf>,  // every factory project found under `bases`
    active: HashSet<PathBuf>,  // projects with an orchestrator session in the window
    temporary: bool,           // first run (no saved list yet): tabs are the active projects
    store: Store,
    projects: Vec<Project>,    // the open tabs
    tab: usize,
    current: Option<PathBuf>,  // root of the open tab, so it survives tabs being added or removed
    picker: Option<usize>,     // cursor in the project picker while it is open
    notice: Option<String>,
    selected: HashMap<PathBuf, PathBuf>,
    cursor: usize,
    scroll: usize,
    collapsed: HashSet<PathBuf>,
    frame: usize,
    feed_focus: bool,
    feed_cursor: Option<u64>, // event id in the selected agent's feed; None follows the newest
    expanded: HashSet<u64>, // event ids shown in full
    expand_scroll: usize, // rows scrolled inside an expansion taller than the panel
    expand_max: usize,    // set when drawing: how far that expansion can scroll
}

impl App {
    fn refresh(&mut self, rescan: bool) {
        if rescan {
            self.discovered = find_projects(&self.bases);
        }
        // Build every known project, so the picker and the no-pins fallback know what is active.
        // shortcut: reloads every discovered project's jobs each second; fine for tens of projects
        let mut all_roots = self.discovered.clone();
        all_roots.extend(self.pinned.iter().filter(|p| !self.discovered.contains(p)).cloned());
        let mut all = self.store.build(&all_roots);
        self.active = all.iter().filter(|p| !p.roots.is_empty()).map(|p| p.root.clone()).collect();
        // a saved empty list means the user closed every tab; only a missing file falls back
        self.temporary = !self.args_mode && self.pinned.is_empty() && !self.config.exists();
        let open: Vec<PathBuf> = if self.args_mode {
            self.discovered.clone()
        } else if self.temporary {
            self.discovered.iter().filter(|p| self.active.contains(*p)).cloned().collect()
        } else {
            self.pinned.clone()
        };
        self.projects = open.iter().filter_map(|r| all.iter().position(|p| &p.root == r).map(|i| all.swap_remove(i))).collect();
        self.tab = self
            .current
            .as_ref()
            .and_then(|c| self.projects.iter().position(|p| &p.root == c))
            .unwrap_or(self.tab)
            .min(self.projects.len().saturating_sub(1));
        self.current = self.projects.get(self.tab).map(|p| p.root.clone());
        self.frame += 1;
    }

    fn switch_tab(&mut self, delta: isize) {
        let n = self.projects.len().max(1) as isize;
        self.tab = (self.tab as isize + delta).rem_euclid(n) as usize;
        self.current = self.projects.get(self.tab).map(|p| p.root.clone());
        self.feed_cursor = None;
        self.expanded.clear();
    }

    /// Pin or unpin a project, save the list, and open its tab when pinning.
    fn toggle_pin(&mut self, root: &Path) {
        if let Some(i) = self.pinned.iter().position(|p| p == root) {
            self.pinned.remove(i);
        } else {
            self.pinned.push(root.to_path_buf());
            self.current = Some(root.to_path_buf());
        }
        if self.args_mode {
            self.notice = Some("paths given on the command line: changes last for this run only".into());
        } else if let Err(e) = save_pinned(&self.config, &self.pinned) {
            self.notice = Some(format!("could not save {}: {e}", self.config.display()));
        }
        self.refresh(false);
    }

    fn rows(&self) -> Vec<Row> {
        let Some(p) = self.projects.get(self.tab) else { return vec![] };
        let mut out = Vec::new();
        fn visit(app: &App, p: &Project, idx: usize, depth: usize, lasts: Vec<bool>, out: &mut Vec<Row>) {
            out.push(Row { node: idx, depth, lasts: lasts.clone() });
            if app.collapsed.contains(&p.nodes[idx].path) {
                return;
            }
            let kids = &p.nodes[idx].children;
            for (i, &c) in kids.iter().enumerate() {
                let mut l = lasts.clone();
                l.push(i + 1 == kids.len());
                visit(app, p, c, depth + 1, l, out);
            }
        }
        for &r in &p.roots {
            visit(self, p, r, 0, vec![], &mut out);
        }
        out
    }

    fn sync_cursor(&mut self, rows: &[Row]) {
        // keep the selection on the same agent across refreshes
        if let Some(p) = self.projects.get(self.tab) {
            if let Some(i) = self.selected.get(&p.root).and_then(|path| rows.iter().position(|r| &p.nodes[r.node].path == path)) {
                self.cursor = i;
            }
        }
        self.cursor = self.cursor.min(rows.len().saturating_sub(1));
    }

    fn select(&mut self, rows: &[Row], i: usize) {
        self.feed_cursor = None;
        self.expanded.clear();
        self.cursor = i.min(rows.len().saturating_sub(1));
        if let (Some(p), Some(r)) = (self.projects.get(self.tab), rows.get(self.cursor)) {
            self.selected.insert(p.root.clone(), p.nodes[r.node].path.clone());
        }
    }

    fn selected_node<'a>(&'a self, rows: &[Row]) -> Option<(&'a Node, &'a Transcript)> {
        let n = &self.projects.get(self.tab)?.nodes[rows.get(self.cursor)?.node];
        Some((n, self.store.cache.get(&n.path)?))
    }

    /// The highlighted feed event (the newest one while following).
    fn feed_id(&self, rows: &[Row]) -> Option<u64> {
        let (_, t) = self.selected_node(rows)?;
        let last = (t.popped + t.events.len() as u64).checked_sub(1)?;
        Some(self.feed_cursor.unwrap_or(last).clamp(t.popped, last))
    }

    /// Move the feed highlight by `delta` events (negative = older). A step first scrolls
    /// through the highlighted event's expansion when it is taller than the panel.
    fn move_feed(&mut self, rows: &[Row], delta: i64) {
        let Some(cur) = self.feed_id(rows) else { return };
        if self.expanded.contains(&cur) {
            if delta == 1 && self.expand_scroll < self.expand_max {
                self.expand_scroll += 1;
                return;
            }
            if delta == -1 && self.expand_scroll > 0 {
                self.expand_scroll -= 1;
                return;
            }
        }
        let Some((_, t)) = self.selected_node(rows) else { return };
        let last = t.popped + t.events.len() as u64 - 1;
        self.feed_cursor = Some((cur as i64).saturating_add(delta).clamp(t.popped as i64, last as i64) as u64);
        self.expand_scroll = 0;
    }

    fn toggle_expand(&mut self, rows: &[Row]) {
        let Some(cur) = self.feed_id(rows) else { return };
        self.feed_cursor = Some(cur); // stop following so new events don't move it
        if !self.expanded.remove(&cur) {
            self.expanded.insert(cur);
        }
        self.expand_scroll = 0;
    }
}

/// Greedy word wrap; words longer than `width` are split.
fn wrap(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut out = Vec::new();
    for para in text.replace('\t', "    ").lines() {
        let mut line = String::new();
        for word in para.split(' ') {
            let mut word: Vec<char> = word.chars().collect();
            while word.len() > width {
                if !line.is_empty() {
                    out.push(std::mem::take(&mut line));
                }
                out.push(word.drain(..width).collect());
            }
            let len = line.chars().count();
            if len > 0 && len + 1 + word.len() > width {
                out.push(std::mem::take(&mut line));
            }
            if !line.is_empty() {
                line.push(' ');
            }
            line.extend(word);
        }
        out.push(line);
    }
    out
}

/// Pad a line with spaces to `width` so its background fills the row.
fn fill(mut line: Line<'static>, width: usize, bg: Color) -> Line<'static> {
    let gap = width.saturating_sub(line.width());
    line.push_span(Span::raw(" ".repeat(gap)));
    line.style(Style::new().bg(bg))
}

fn badge(s: Status, frame: usize) -> Span<'static> {
    const SPIN: [&str; 4] = ["◐", "◓", "◑", "◒"];
    let (text, fg, bg) = match s {
        Status::Running => (format!(" {} Running ", SPIN[frame % 4]), BLUE, BG_BLUE),
        Status::NeedsYou => (" ! Needs you ".into(), ORANGE, BG_RED),
        Status::Blocked => (" ■ Blocked ".into(), YELLOW, BG_YELLOW),
        Status::Idle => (" ○ Idle ".into(), GREY, BG1),
        Status::Done => (" ✓ Done ".into(), GREEN, BG_GREEN),
    };
    Span::styled(text, Style::new().fg(fg).bg(bg).add_modifier(Modifier::BOLD))
}

fn fmt_secs(s: u64) -> String {
    match s {
        0..=59 => format!("{s}s"),
        60..=3599 => format!("{}m{:02}s", s / 60, s % 60),
        _ => format!("{}h{:02}m", s / 3600, s % 3600 / 60),
    }
}

/// `left` then `right` pushed to the far edge of `width`.
fn spread<'a>(mut left: Vec<Span<'a>>, right: Vec<Span<'a>>, width: u16) -> Line<'a> {
    let used: usize = left.iter().chain(&right).map(|s| s.width()).sum();
    left.push(Span::raw(" ".repeat((width as usize).saturating_sub(used))));
    left.extend(right);
    Line::from(left)
}

fn draw_card(f: &mut Frame, area: Rect, n: &Node, selected: bool, collapsed: bool, frame: usize) {
    let border = if selected { Style::new().fg(GREEN).add_modifier(Modifier::BOLD) } else { Style::new().fg(BG3) };
    // selection is shown by the border and name only: a fill can't follow rounded corners in a terminal
    let block = Block::bordered().border_type(BorderType::Rounded).border_style(border);
    let inner = block.inner(area);
    f.render_widget(block, area);
    let w = inner.width;
    let name_style = match (n.name == "orchestrator", selected) {
        (true, true) => Style::new().fg(PURPLE).add_modifier(Modifier::BOLD),
        (true, false) => Style::new().fg(PURPLE),
        (false, true) => Style::new().fg(FG).add_modifier(Modifier::BOLD),
        (false, false) => Style::new().fg(GREY2),
    };
    let title = spread(
        vec![
            Span::styled(format!(" {} ", n.name), name_style),
            Span::styled(n.model.clone(), Style::new().fg(GREY)),
        ],
        vec![badge(n.status, frame)],
        w,
    );
    let task = Line::from(Span::styled(format!(" {}", one_line(&n.task, w.saturating_sub(2) as usize)), Style::new().fg(FG)));
    let kids = if n.children.is_empty() { String::new() } else { format!("{} {} ", n.children.len(), if collapsed { "▸" } else { "▾" }) };
    let meta = format!(" ${:.2} · {} ", n.cost, fmt_secs(n.secs));
    let room = (w as usize).saturating_sub(meta.chars().count() + kids.chars().count() + 1);
    let foot = spread(
        vec![
            Span::styled(meta, Style::new().fg(YELLOW)),
            Span::styled(one_line(&n.last, room), Style::new().fg(GREY)),
        ],
        vec![Span::styled(kids, Style::new().fg(AQUA))],
        w,
    );
    f.render_widget(Paragraph::new(vec![title, task, foot]), inner);
}

fn draw_tree(f: &mut Frame, area: Rect, app: &mut App, rows: &[Row]) {
    let visible = (area.height / CARD_H).max(1) as usize;
    if app.cursor < app.scroll {
        app.scroll = app.cursor;
    } else if app.cursor >= app.scroll + visible {
        app.scroll = app.cursor + 1 - visible;
    }
    let Some(p) = app.projects.get(app.tab) else { return };
    if rows.is_empty() {
        let msg = format!(" No agents in the last {} min. Start one with /factory:build <feature>.", ROOT_WINDOW / 60);
        f.render_widget(Paragraph::new(msg).style(Style::new().fg(GREY)), area);
        return;
    }
    let line = Style::new().fg(BLUE);
    for (slot, (i, row)) in rows.iter().enumerate().skip(app.scroll).take(visible).enumerate() {
        let y = area.y + slot as u16 * CARD_H;
        let x = area.x + row.depth as u16 * INDENT;
        let width = area.width.saturating_sub(row.depth as u16 * INDENT);
        if width < 20 {
            continue;
        }
        let buf = f.buffer_mut();
        for dy in 0..CARD_H {
            for (lvl, &last) in row.lasts.iter().enumerate() {
                let gx = area.x + lvl as u16 * INDENT + 1;
                let own = lvl + 1 == row.depth;
                let sym = match (own, dy, last) {
                    (true, 0..=1, _) => "│",
                    (true, 2, true) => "╰──",
                    (true, 2, false) => "├──",
                    (_, _, false) => "│",
                    _ => " ",
                };
                buf.set_string(gx, y + dy, sym, line);
            }
        }
        let n = &p.nodes[row.node];
        draw_card(f, Rect::new(x, y, width, CARD_H), n, i == app.cursor && !app.feed_focus, app.collapsed.contains(&n.path), app.frame);
    }
}

fn key_line(keys: &[(&str, &str)], notice: Option<&String>) -> Line<'static> {
    let mut spans: Vec<Span> = keys
        .iter()
        .flat_map(|(k, d)| [Span::styled(format!(" {k} "), Style::new().fg(BG0).bg(GREEN)), Span::styled(format!(" {d}  "), Style::new().fg(GREY))])
        .collect();
    if let Some(n) = notice {
        spans.push(Span::styled(n.clone(), Style::new().fg(ORANGE)));
    }
    Line::from(spans)
}

const LOGO: &[&str] = &[
    "⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢸⣿⡇⠀⢸⣿",
    "⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢸⣿⡇⠀⢸⣿",
    "⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢸⣿⡇⠀⢸⣿",
    "⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⡀⠀⠀⠀⠀⠀⢸⣿⡇⠀⢸⣿",
    "⢠⣷⡀⠀⠀⠀⠀⣼⣦⠀⠀⠀⠀⢠⣷⣄⠀⠀⠀⠀⢸⣿⡇⠀⢸⣿",
    "⢸⣿⣿⣆⠀⠀⠀⣿⣿⣷⡀⠀⠀⢸⣿⣿⣧⠀⠀⠀⢸⣿⡇⠀⢸⣿",
    "⣾⣿⣿⣿⣷⡀⢠⣿⣿⣿⣿⣆⠀⣼⣿⣿⣿⣷⣄⠀⢸⣿⡇⠀⢸⣿",
    "⣿⣿⣿⣿⣿⣿⣾⣿⣿⣿⣿⣿⣷⣿⣿⣿⣿⣿⣿⣶⣼⣿⣧⣤⣾⣿",
    "⣿⡟⠉⠉⢹⣿⡏⠉⠉⢻⣿⠉⠉⠉⣿⣿⠉⠉⢹⣿⣿⣿⣿⣿⣿⣿",
    "⣿⣷⣶⣶⣾⣿⣷⣶⣶⣾⣿⣶⣶⣶⣿⣿⣶⣶⣾⣿⣿⣿⣿⣿⣿⣿",
    "⣿⡏⠉⠉⢹⣿⡏⠉⠉⢹⣿⠉⠉⠉⣿⣿⠉⠉⢹⣿⣿⣿⣿⣿⣿⣿",
    "⣿⣿⣶⣶⣾⣿⣷⣶⣶⣿⣿⣷⣶⣾⣿⣿⣶⣶⣾⣿⣿⣿⣿⣿⣿⣿",
];

/// Shown instead of the dashboard while no project tab is open: logo, hint, and only the keys that work here.
fn draw_home(f: &mut Frame, app: &App) {
    let [body, foot] = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(f.area());
    let hint = if app.args_mode {
        "No .factory/ projects under the paths you gave.".to_string()
    } else if app.temporary {
        format!("Nothing active in the last {} min. Press a to pick the projects you want as tabs.", ROOT_WINDOW / 60)
    } else {
        "No tabs open. Press a to pick the projects you want as tabs.".to_string()
    };
    let mut lines: Vec<Line> = Vec::new();
    // drop the art when it would not fit; the title and hint still show
    if body.height as usize >= LOGO.len() + 4 && body.width >= 30 {
        lines.extend(LOGO.iter().map(|l| Line::styled(*l, Style::new().fg(GREEN))));
        lines.push(Line::raw(""));
    }
    lines.push(Line::styled("Pedro's Factory", Style::new().fg(FG).add_modifier(Modifier::BOLD)));
    lines.push(Line::raw(""));
    lines.push(Line::styled(hint, Style::new().fg(GREY)));
    let top = body.y + body.height.saturating_sub(lines.len() as u16) / 2;
    let area = Rect { y: top, height: body.bottom() - top, ..body };
    f.render_widget(Paragraph::new(lines).alignment(ratatui::layout::Alignment::Center), area);
    let keys: &[(&str, &str)] = if app.args_mode { &[("r", "refresh"), ("q", "quit")] } else { &[("a", "projects"), ("r", "refresh"), ("q", "quit")] };
    f.render_widget(Paragraph::new(key_line(keys, app.notice.as_ref())), foot);
}

fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();
    f.render_widget(Block::new().style(Style::new().bg(BG0).fg(FG)), area);
    if app.projects.is_empty() {
        return draw_home(f, app);
    }
    let [top, body, foot] = Layout::vertical([Constraint::Length(2), Constraint::Min(5), Constraint::Length(1)]).areas(area);

    // top bar: tabs on the left, counts on the right
    let titles: Vec<Line> = app
        .projects
        .iter()
        .map(|p| {
            let name = p.root.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let live = p.nodes.iter().any(|n| n.status == Status::Running);
            let attention = p.nodes.iter().any(|n| n.status == Status::NeedsYou);
            let dot = if attention { Span::styled("● ", Style::new().fg(ORANGE)) } else if live { Span::styled("● ", Style::new().fg(GREEN)) } else { Span::raw("") };
            // unpinned tabs (shown because nothing is pinned) are italic
            let style = if app.temporary { Style::new().add_modifier(Modifier::ITALIC) } else { Style::new() };
            Line::from(vec![dot, Span::styled(name, style)])
        })
        .collect();
    let rows = app.rows();
    app.sync_cursor(&rows);
    let (agents, running, needs, cost) = app.projects.get(app.tab).map_or((0, 0, 0, 0.0), |p| {
        (
            p.nodes.len(),
            p.nodes.iter().filter(|n| n.status == Status::Running).count(),
            p.nodes.iter().filter(|n| matches!(n.status, Status::NeedsYou | Status::Blocked)).count(),
            p.nodes.iter().map(|n| n.cost).sum::<f64>(),
        )
    });
    let mut stats = vec![
        Span::styled(format!("{agents} agents  "), Style::new().fg(GREY)),
        Span::styled(format!("{running} running  "), Style::new().fg(BLUE).add_modifier(Modifier::BOLD)),
    ];
    if needs > 0 {
        stats.push(Span::styled(format!(" {needs} needs you "), Style::new().fg(ORANGE).bg(BG_RED).add_modifier(Modifier::BOLD)));
        stats.push(Span::raw("  "));
    }
    stats.push(Span::styled(format!("${cost:.2} "), Style::new().fg(FG).add_modifier(Modifier::BOLD)));
    let stats = Line::from(stats);
    let [brand, tabs_area, stats_area] =
        Layout::horizontal([Constraint::Length(12), Constraint::Min(10), Constraint::Length(stats.width() as u16)]).areas(top);
    f.render_widget(Paragraph::new(Span::styled(" ✦ Agents", Style::new().fg(GREEN).add_modifier(Modifier::BOLD))), brand);
    f.render_widget(
        Tabs::new(titles)
            .select(app.tab)
            .style(Style::new().fg(GREY))
            .highlight_style(Style::new().fg(BG0).bg(GREEN).add_modifier(Modifier::BOLD))
            .divider(Span::styled("│", Style::new().fg(BG3))),
        tabs_area,
    );
    f.render_widget(Paragraph::new(stats), stats_area);

    let [tree_area, side] = Layout::horizontal([Constraint::Percentage(62), Constraint::Percentage(38)]).areas(body);
    draw_tree(f, tree_area.inner(ratatui::layout::Margin::new(1, 0)), app, &rows);

    // same slots in the same order in both panels: focus, move, jump, act, then app-wide keys
    let keys: &[(&str, &str)] = if app.feed_focus {
        &[("tab", "tree"), ("j/k", "event"), ("g/G", "oldest/newest"), ("enter", "expand"), ("h/l", "project"), ("a", "projects"), ("r", "refresh"), ("q", "quit")]
    } else {
        &[("tab", "activity"), ("j/k", "agent"), ("g/G", "top/bottom"), ("enter", "fold"), ("h/l", "project"), ("a", "projects"), ("r", "refresh"), ("q", "quit")]
    };
    f.render_widget(Paragraph::new(key_line(keys, app.notice.as_ref())), foot);

    // side: jobs, then the selected agent's activity
    let Some(p) = app.projects.get(app.tab) else { return };
    let job_h = (p.jobs.len().min(6) as u16 + 2).max(3);
    let [jobs_area, feed_area] = Layout::vertical([Constraint::Length(job_h), Constraint::Min(3)]).areas(side);
    let panel = |title: &str, color: Color| {
        Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Style::new().fg(BG3))
            .title(Span::styled(format!(" {title} "), Style::new().fg(color).add_modifier(Modifier::BOLD)))
    };
    let job_lines: Vec<Line> = if p.jobs.is_empty() {
        vec![Line::styled(" no jobs yet", Style::new().fg(GREY))]
    } else {
        p.jobs
            .iter()
            .take(6)
            .map(|j| {
                let stage_color = match j.stage.as_str() {
                    "needs-human" => RED,
                    "merged" | "approved" | "done" => GREEN,
                    _ => YELLOW,
                };
                Line::from(vec![
                    Span::styled(if j.live { "● " } else { "  " }, Style::new().fg(GREEN)),
                    Span::styled(format!("{} ", j.id), Style::new().fg(FG)),
                    Span::styled(format!("{} ", j.stage), Style::new().fg(stage_color).add_modifier(Modifier::BOLD)),
                    Span::styled(format!("r{} {}/{} WPs ", j.round, j.wps.0, j.wps.1), Style::new().fg(GREY)),
                    Span::styled(j.verdicts.clone(), Style::new().fg(AQUA)),
                ])
            })
            .collect()
    };
    f.render_widget(Paragraph::new(job_lines).block(panel("Jobs", YELLOW)), jobs_area);

    let sel = rows.get(app.cursor).map(|r| &p.nodes[r.node]);
    let title = sel.map_or("Activity".to_string(), |n| format!("Activity · {}", n.name));
    let feed_block = panel(&title, AQUA).border_style(Style::new().fg(if app.feed_focus { AQUA } else { BG3 }));
    let inner = feed_block.inner(feed_area);
    let now = now();
    let (h, w) = (inner.height as usize, inner.width as usize);
    let mut out: Vec<Line> = Vec::new();
    let (mut r0, mut r1) = (0, 0); // rows of the highlighted event, expansion included
    let mut expand_max = 0;
    if let Some(t) = sel.and_then(|n| app.store.cache.get(&n.path)) {
        let n = t.events.len();
        let cur = app.feed_cursor.map_or(n.saturating_sub(1), |id| (id.saturating_sub(t.popped) as usize).min(n.saturating_sub(1)));
        for (i, (ts, text)) in t.events.iter().enumerate() {
            let selected = app.feed_focus && i == cur;
            if i == cur {
                r0 = out.len();
            }
            let color = if text.starts_with('✎') { FG } else { GREY };
            let time = Span::styled(format!("{:>6} ", fmt_secs(now.saturating_sub(*ts))), Style::new().fg(if selected { GREY } else { BG3 }));
            let header_bg = if selected { BG3 } else { BG1 };
            if app.expanded.contains(&(t.popped + i as u64)) {
                // the message itself grows: first wrapped line on the event's row, the rest under it
                let body: String = text.chars().skip(2).collect(); // drop the "▸ " / "✎ " marker
                let mut parts = wrap(body.trim(), w.saturating_sub(9)).into_iter();
                let first = Line::from(vec![time, Span::styled("▾ ", Style::new().fg(AQUA)), Span::styled(parts.next().unwrap_or_default(), Style::new().fg(color))]);
                out.push(fill(first, w, header_bg));
                for l in parts {
                    out.push(fill(Line::from(vec![Span::raw(" ".repeat(9)), Span::styled(l, Style::new().fg(color))]), w, BG1));
                }
            } else {
                let line = Line::from(vec![time, Span::styled(one_line(text, w.saturating_sub(8)), Style::new().fg(color))]);
                out.push(if selected { fill(line, w, BG3) } else { line });
            }
            if selected {
                out[r0] = std::mem::take(&mut out[r0]).patch_style(Style::new().add_modifier(Modifier::BOLD));
            }
            if i == cur {
                r1 = out.len() - 1;
            }
        }
    }
    // stick to the newest rows unless the highlight was moved above them
    let mut start = out.len().saturating_sub(h);
    if app.feed_focus {
        let block = r1 + 1 - r0;
        if block > h {
            expand_max = block - h;
            start = r0 + app.expand_scroll.min(expand_max);
        } else if r0 < start {
            start = r0;
        }
    }
    app.expand_max = expand_max;
    let visible: Vec<Line> = out.into_iter().skip(start).take(h).collect();
    f.render_widget(Paragraph::new(visible).block(feed_block), feed_area);

}

fn tilde(p: &Path) -> String {
    match std::env::var_os("HOME").map(PathBuf::from) {
        Some(h) if p.starts_with(&h) => format!("~/{}", p.strip_prefix(&h).unwrap_or(p).display()),
        _ => p.display().to_string(),
    }
}

/// Projects the picker offers: everything discovered, plus pinned ones outside the scan.
fn picker_items(app: &App) -> Vec<PathBuf> {
    let mut items = app.discovered.clone();
    items.extend(app.pinned.iter().filter(|p| !app.discovered.contains(p)).cloned());
    items
}

fn draw_picker(f: &mut Frame, app: &App) {
    let Some(cursor) = app.picker else { return };
    let items = picker_items(app);
    let area = f.area();
    let w = area.width.saturating_sub(8).min(96);
    let h = (items.len().max(1) as u16 + 2).min(area.height.saturating_sub(4));
    let pop = Rect::new(area.x + (area.width - w) / 2, area.y + (area.height - h) / 2, w, h);
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(GREEN))
        .style(Style::new().bg(BG0))
        .title(Span::styled(" Projects ", Style::new().fg(GREEN).add_modifier(Modifier::BOLD)))
        .title_bottom(Span::styled(" enter pin/unpin · esc close ", Style::new().fg(GREY)));
    let inner = block.inner(pop);
    let rows = inner.height as usize;
    let start = cursor.saturating_sub(rows.saturating_sub(1));
    let lines: Vec<Line> = if items.is_empty() {
        vec![Line::styled(" No .factory/ projects found under your home folder.", Style::new().fg(GREY))]
    } else {
        items
            .iter()
            .enumerate()
            .skip(start)
            .take(rows)
            .map(|(i, root)| {
                let pinned = app.pinned.contains(root);
                let name = root.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                let line = Line::from(vec![
                    Span::styled(if pinned { " ✓ " } else { "   " }, Style::new().fg(GREEN).add_modifier(Modifier::BOLD)),
                    Span::styled(format!("{name:<24} "), Style::new().fg(if pinned { FG } else { GREY2 }).add_modifier(Modifier::BOLD)),
                    Span::styled(if app.active.contains(root) { "● active  " } else { "          " }, Style::new().fg(GREEN)),
                    Span::styled(tilde(root), Style::new().fg(GREY)),
                ]);
                if i == cursor { fill(line, inner.width as usize, BG3) } else { line }
            })
            .collect()
    };
    f.render_widget(ratatui::widgets::Clear, pop);
    f.render_widget(Paragraph::new(lines).block(block), pop);
}

fn main() -> std::io::Result<()> {
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    if std::env::args().nth(1).is_some_and(|a| a == "--version" || a == "-V") {
        println!("factory-dash {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    let args: Vec<PathBuf> = std::env::args().skip(1).map(PathBuf::from).collect();
    let bases = if args.is_empty() { vec![home.clone()] } else { args.iter().map(|p| fs::canonicalize(p).unwrap_or(p.clone())).collect() };
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".config"))
        .join("factory-dash")
        .join("projects");
    let mut app = App {
        bases,
        args_mode: !args.is_empty(),
        pinned: load_pinned(&config),
        config,
        discovered: vec![],
        active: HashSet::new(),
        temporary: false,
        current: None,
        picker: None,
        notice: None,
        store: Store { projects_dir: home.join(".claude").join("projects"), cache: HashMap::new() },
        projects: vec![],
        tab: 0,
        selected: HashMap::new(),
        cursor: 0,
        scroll: 0,
        collapsed: HashSet::new(),
        frame: 0,
        feed_focus: false,
        feed_cursor: None,
        expanded: HashSet::new(),
        expand_scroll: 0,
        expand_max: 0,
    };
    app.refresh(true);

    let mut terminal = ratatui::init();
    let mut last_refresh = Instant::now();
    let mut last_scan = Instant::now();
    let result = loop {
        if let Err(e) = terminal.draw(|f| {
            draw(f, &mut app);
            draw_picker(f, &app);
        }) {
            break Err(e);
        }
        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(k) = event::read()? {
                if k.kind != KeyEventKind::Press {
                    continue;
                }
                let rows = app.rows();
                let feed = app.feed_focus;
                app.notice = None;
                if let Some(c) = app.picker {
                    let items = picker_items(&app);
                    let last = items.len().saturating_sub(1);
                    match k.code {
                        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('a') => app.picker = None,
                        KeyCode::Char('j') | KeyCode::Down => app.picker = Some((c + 1).min(last)),
                        KeyCode::Char('k') | KeyCode::Up => app.picker = Some(c.saturating_sub(1)),
                        KeyCode::Char('g') | KeyCode::Home => app.picker = Some(0),
                        KeyCode::Char('G') | KeyCode::End => app.picker = Some(last),
                        KeyCode::Enter | KeyCode::Char(' ') => {
                            if let Some(root) = items.get(c) {
                                app.toggle_pin(root);
                            }
                        }
                        _ => {}
                    }
                    continue;
                }
                match k.code {
                    KeyCode::Char('q') => break Ok(()),
                    KeyCode::Esc if feed => app.feed_focus = false,
                    KeyCode::Esc => break Ok(()),
                    KeyCode::Tab | KeyCode::BackTab => app.feed_focus = !feed,
                    KeyCode::Char('l') | KeyCode::Right => app.switch_tab(1),
                    KeyCode::Char('h') | KeyCode::Left => app.switch_tab(-1),
                    KeyCode::Char('a') if app.args_mode => app.notice = Some("showing the paths you gave; run factory-dash with no paths to pick projects".into()),
                    KeyCode::Char('a') => {
                        app.refresh(true);
                        app.picker = Some(0);
                    }
                    KeyCode::Char('j') | KeyCode::Down if feed => app.move_feed(&rows, 1),
                    KeyCode::Char('k') | KeyCode::Up if feed => app.move_feed(&rows, -1),
                    KeyCode::Char('g') | KeyCode::Home if feed => app.move_feed(&rows, i64::MIN),
                    KeyCode::Char('G') | KeyCode::End if feed => {
                        app.feed_cursor = None;
                        app.expand_scroll = 0;
                    }
                    KeyCode::Enter | KeyCode::Char(' ') if feed => app.toggle_expand(&rows),
                    KeyCode::Char('j') | KeyCode::Down => app.select(&rows, app.cursor + 1),
                    KeyCode::Char('k') | KeyCode::Up => app.select(&rows, app.cursor.saturating_sub(1)),
                    KeyCode::Char('g') | KeyCode::Home => app.select(&rows, 0),
                    KeyCode::Char('G') | KeyCode::End => app.select(&rows, usize::MAX),
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        if let (Some(p), Some(r)) = (app.projects.get(app.tab), rows.get(app.cursor)) {
                            let path = p.nodes[r.node].path.clone();
                            if !app.collapsed.remove(&path) {
                                app.collapsed.insert(path);
                            }
                        }
                    }
                    KeyCode::Char('r') => app.refresh(true),
                    _ => {}
                }
            }
        }
        if last_refresh.elapsed() >= Duration::from_secs(1) {
            let rescan = last_scan.elapsed() >= Duration::from_secs(30);
            if rescan {
                last_scan = Instant::now();
            }
            app.refresh(rescan);
            last_refresh = Instant::now();
        }
    };
    ratatui::restore();
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_timestamps() {
        assert_eq!(parse_ts("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(parse_ts("2026-10-20T02:12:19.215Z"), Some(1_792_462_339));
        assert_eq!(parse_ts("garbage"), None);
    }

    #[test]
    fn counts_cost_once_per_message_and_tracks_spawns() {
        let mut t = Transcript::default();
        let usage = r#"{"input_tokens":1000000,"output_tokens":0,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}"#;
        for block in [r#"{"type":"text","text":"hi"}"#, r#"{"type":"tool_use","id":"toolu_1","name":"Agent","input":{"description":"Build WP1"}}"#] {
            let line = format!(r#"{{"type":"assistant","cwd":"/p","timestamp":"2026-10-20T02:12:19Z","message":{{"id":"m1","model":"claude-opus-5-5","usage":{usage},"content":[{block}]}}}}"#);
            t.apply(&serde_json::from_str(&line).unwrap());
        }
        assert!((t.cost - 4.0).abs() < 1e-9); // 1M input tokens of Opus 5.5, counted once
        assert_eq!(t.spawned, vec!["toolu_1"]);
        assert_eq!(t.last, "Agent Build WP1");
        assert_eq!(short_model(&t.model), "Opus 5.5");
    }

    #[test]
    fn keeps_full_text_and_stable_ids() {
        let mut t = Transcript::default();
        let cmd = format!("echo {}\nls", "x".repeat(500));
        t.apply(&serde_json::from_value(serde_json::json!({"type":"assistant","message":{"id":"a","content":[{"type":"tool_use","id":"t","name":"Bash","input":{"command":cmd}}]}})).unwrap());
        assert_eq!(t.events[0].1, format!("▸ Bash {cmd}"));
        for i in 0..304 {
            t.push(0, i.to_string());
        }
        assert_eq!((t.events.len(), t.popped), (300, 5));
        assert_eq!(t.events[0].1, "4"); // id popped + 0 = 5 is the 6th event pushed
    }

    #[test]
    fn wraps_words_and_splits_long_ones() {
        assert_eq!(wrap("aa bb cc", 5), vec!["aa bb", "cc"]);
        assert_eq!(wrap("abcdefgh x", 3), vec!["abc", "def", "gh", "x"]);
        assert_eq!(wrap("one\n\ntwo", 10), vec!["one", "", "two"]);
    }

    #[test]
    fn pinned_list_round_trips() {
        let dir = std::env::temp_dir().join(format!("factory-dash-test-{}", std::process::id()));
        let config = dir.join("factory-dash").join("projects");
        let pinned = vec![PathBuf::from("/home/me/a"), PathBuf::from("/home/me/b c")];
        save_pinned(&config, &pinned).unwrap();
        assert_eq!(load_pinned(&config), pinned); // header comment is skipped
        assert!(load_pinned(&dir.join("missing")).is_empty());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn needs_you_until_user_replies() {
        let mut t = Transcript::default();
        t.apply(&serde_json::from_str(r#"{"type":"assistant","message":{"id":"a","content":[{"type":"tool_use","id":"x","name":"AskUserQuestion","input":{}}]}}"#).unwrap());
        assert!(t.waiting);
        t.apply(&serde_json::from_str(r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"x"}]}}"#).unwrap());
        assert!(!t.waiting);
    }
}
