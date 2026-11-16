//! What the chats used. OmniHub's own chats keep a small ledger per day.
//! Claude Code's use is added up from the token counts in its transcripts,
//! with what the same tokens would cost on the API (a subscription doesn't
//! bill per token; it has limits instead, which Claude Code notes in a
//! transcript when one is reached). Codex records its token counts and the
//! plan's rate limits (how much of each window is used, when it resets) in
//! its session files. Files are read once and then only their new lines.

use std::collections::{BTreeMap, HashMap};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use chrono::{Local, TimeZone};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::ts;

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Tokens {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
    /// US dollars (for Claude Code: what it would cost on the API).
    pub cost: f64,
}

impl Tokens {
    pub fn add(&mut self, o: &Tokens) {
        self.input += o.input;
        self.output += o.output;
        self.cache_read += o.cache_read;
        self.cache_write += o.cache_write;
        self.cost += o.cost;
    }

    pub fn total(&self) -> u64 {
        self.input + self.output + self.cache_read + self.cache_write
    }
}

/// API prices (per million tokens: input, output, cache read, cache write)
/// for any Claude model a transcript names, current or older.
pub fn claude_price(model: &str) -> (f64, f64, f64, f64) {
    if crate::claude::MODELS.iter().any(|(m, _)| *m == model) {
        return crate::claude::price(model);
    }
    let m = model.to_ascii_lowercase();
    let older = |p: f64, o: f64| (p, o, p / 10.0, p * 1.25);
    if m.contains("fable") {
        crate::claude::price("claude-fable-5-1")
    } else if m.contains("opus") {
        if m.contains("opus-4-5") || m.contains("opus-4-6") || m.contains("opus-4-7") {
            older(5.0, 25.0)
        } else if m.contains("opus-4") || m.contains("3-opus") {
            older(15.0, 75.0)
        } else {
            crate::claude::price("claude-opus-5-5")
        }
    } else if m.contains("sonnet") {
        if m.contains("sonnet-4") || m.contains("3-7-sonnet") || m.contains("3-5-sonnet") {
            older(3.0, 15.0)
        } else {
            crate::claude::price("claude-sonnet-5-5")
        }
    } else if m.contains("haiku") {
        if m.contains("haiku-4-5") {
            older(1.0, 5.0)
        } else if m.contains("3-5-haiku") {
            older(0.8, 4.0)
        } else {
            crate::claude::price("claude-haiku-5-5")
        }
    } else {
        crate::claude::price(crate::claude::DEFAULT_MODEL)
    }
}

/// A limit Claude Code noted as reached.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct LimitSeen {
    at: i64,
    kind: String,
    status: String,
    resets_at: Option<i64>,
}

/// One file, read up to `offset`: its tokens by UTC hour.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct Scan {
    len: u64,
    modified: i64,
    offset: u64,
    /// Claude Code writes a response once per content block with the same
    /// usage; only the first of a run of the same message id counts.
    last_id: String,
    /// Codex's running totals, to count only what each report adds.
    totals: [u64; 3],
    hours: BTreeMap<i64, Tokens>,
    limit: Option<LimitSeen>,
    /// Codex: when its rate limits were last reported, and what they were.
    rate_at: i64,
    rate: Value,
}

fn claude_line(o: &Value, s: &mut Scan) {
    let at = ts(&o["timestamp"]);
    if let Some(q) = o.get("quotaLimits").filter(|q| q.is_object()) {
        if s.limit.as_ref().is_none_or(|l| at >= l.at) {
            s.limit = Some(LimitSeen { at, kind: q["rateLimitType"].as_str().unwrap_or_default().to_string(), status: q["status"].as_str().unwrap_or_default().to_string(), resets_at: q["resetsAt"].as_i64() });
        }
    }
    if o["type"] != "assistant" {
        return;
    }
    let m = &o["message"];
    let u = &m["usage"];
    if !u.is_object() {
        return;
    }
    let id = m["id"].as_str().unwrap_or_default();
    if !id.is_empty() && id == s.last_id {
        return;
    }
    s.last_id = id.to_string();
    let model = m["model"].as_str().unwrap_or_default();
    if model.is_empty() || model.starts_with('<') {
        return;
    }
    let n = |k: &str| u[k].as_u64().unwrap_or(0);
    let mut t = Tokens { input: n("input_tokens"), output: n("output_tokens"), cache_read: n("cache_read_input_tokens"), cache_write: n("cache_creation_input_tokens"), cost: 0.0 };
    let (i, out, r, w) = claude_price(model);
    t.cost = (t.input as f64 * i + t.output as f64 * out + t.cache_read as f64 * r + t.cache_write as f64 * w) / 1e6;
    if let Some(web) = u["server_tool_use"]["web_search_requests"].as_u64() {
        t.cost += web as f64 * 0.01;
    }
    s.hours.entry(at - at.rem_euclid(3600)).or_default().add(&t);
}

fn codex_line(o: &Value, s: &mut Scan) {
    let p = &o["payload"];
    if o["type"] != "event_msg" || p["type"] != "token_count" {
        return;
    }
    let at = ts(&o["timestamp"]);
    if p["rate_limits"].is_object() && at >= s.rate_at {
        s.rate_at = at;
        s.rate = p["rate_limits"].clone();
    }
    let u = &p["info"]["total_token_usage"];
    if !u.is_object() {
        return;
    }
    let n = |k: &str| u[k].as_u64().unwrap_or(0);
    let now = [n("input_tokens"), n("cached_input_tokens"), n("output_tokens")];
    let prev = if now.iter().zip(s.totals.iter()).any(|(a, b)| a < b) { [0; 3] } else { s.totals };
    let d = [now[0] - prev[0], now[1] - prev[1], now[2] - prev[2]];
    s.totals = now;
    if d.iter().all(|x| *x == 0) {
        return;
    }
    let t = Tokens { input: d[0].saturating_sub(d[1]), cache_read: d[1], output: d[2], cache_write: 0, cost: 0.0 };
    s.hours.entry(at - at.rem_euclid(3600)).or_default().add(&t);
}

fn modified(m: &std::fs::Metadata) -> i64 {
    m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs() as i64)
}

/// Bring one file's scan up to date, reading only lines added since.
fn update(path: &Path, s: &mut Scan, line: fn(&Value, &mut Scan)) {
    let Ok(meta) = std::fs::metadata(path) else { return };
    let (len, m) = (meta.len(), modified(&meta));
    if len == s.len && m == s.modified {
        return;
    }
    if path.extension().is_some_and(|e| e == "zst") {
        *s = Scan::default();
        let (text, _) = super::codex::read_text(path, usize::MAX);
        for l in text.lines() {
            if let Ok(o) = serde_json::from_str::<Value>(l) {
                line(&o, s);
            }
        }
    } else {
        if len < s.offset {
            *s = Scan::default();
        }
        let Ok(mut f) = std::fs::File::open(path) else { return };
        if f.seek(SeekFrom::Start(s.offset)).is_err() {
            return;
        }
        let mut buf = Vec::new();
        if f.take(len - s.offset).read_to_end(&mut buf).is_err() {
            return;
        }
        let Some(end) = buf.iter().rposition(|b| *b == b'\n') else { return };
        for l in buf[..end].split(|b| *b == b'\n') {
            if let Ok(o) = serde_json::from_slice::<Value>(l) {
                line(&o, s);
            }
        }
        s.offset += end as u64 + 1;
    }
    s.len = len;
    s.modified = m;
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct Scans {
    claude: HashMap<String, Scan>,
    codex: HashMap<String, Scan>,
}

/// The day ledger of OmniHub's own chats.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OwnDay {
    pub claude: Tokens,
    pub openai: Tokens,
}

pub struct Usage {
    cache_file: PathBuf,
    ledger_file: PathBuf,
    scans: Mutex<Scans>,
    ledger: Mutex<BTreeMap<String, OwnDay>>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Day {
    pub date: String,
    #[serde(flatten)]
    pub tokens: Tokens,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Period {
    pub today: Tokens,
    pub week: Tokens,
    pub month: Tokens,
    /// The last 30 days with any use, oldest first.
    pub days: Vec<Day>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Limit {
    pub name: String,
    /// Codex says how much of the window is used; Claude Code only notes a
    /// limit once it's reached.
    pub used_percent: Option<f64>,
    pub reached: bool,
    pub resets_at: Option<i64>,
}

/// About where the current 5-hour window began, by first use.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Window {
    pub start: i64,
    pub end: i64,
    pub tokens: Tokens,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentUsage {
    /// Whether any of its files were found on this PC.
    pub found: bool,
    pub period: Period,
    pub window: Option<Window>,
    pub limits: Vec<Limit>,
    /// When the limits were last reported.
    pub limits_at: Option<i64>,
    pub plan: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnUsage {
    pub claude: Period,
    pub openai: Period,
    /// The mod maker's projects (all time).
    pub maker: Tokens,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub omni: OwnUsage,
    pub claude_code: AgentUsage,
    pub codex: AgentUsage,
}

fn local_date(at: i64) -> String {
    Local.timestamp_opt(at, 0).single().map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default()
}

fn period(days: &BTreeMap<String, Tokens>, now: i64) -> Period {
    let today = local_date(now);
    let back = |n: i64| local_date(now - n * 86_400);
    let (week_from, month_from) = (back(6), back(29));
    let mut p = Period::default();
    for (d, t) in days {
        if *d >= month_from {
            p.month.add(t);
            p.days.push(Day { date: d.clone(), tokens: *t });
        }
        if *d >= week_from {
            p.week.add(t);
        }
        if *d == today {
            p.today.add(t);
        }
    }
    p
}

fn by_day(hours: &BTreeMap<i64, Tokens>) -> BTreeMap<String, Tokens> {
    let mut d: BTreeMap<String, Tokens> = BTreeMap::new();
    for (h, t) in hours {
        d.entry(local_date(*h)).or_default().add(t);
    }
    d
}

/// The 5-hour window in use now, if any: windows start at the first use
/// after the last one ended.
fn window(hours: &BTreeMap<i64, Tokens>, now: i64) -> Option<Window> {
    const SPAN: i64 = 5 * 3600;
    let mut start: Option<i64> = None;
    for (h, t) in hours.range(now - 2 * SPAN..) {
        if t.total() > 0 && start.is_none_or(|s| *h >= s + SPAN) {
            start = Some(*h);
        }
    }
    let s = start.filter(|s| now < s + SPAN)?;
    let mut tokens = Tokens::default();
    for (_, t) in hours.range(s..s + SPAN) {
        tokens.add(t);
    }
    Some(Window { start: s, end: s + SPAN, tokens })
}

fn limit_name(kind: &str) -> String {
    match kind {
        "five_hour" => "5-hour limit".into(),
        "seven_day" => "Weekly limit".into(),
        "seven_day_opus" => "Weekly Opus limit".into(),
        "seven_day_sonnet" => "Weekly Sonnet limit".into(),
        "" => "Usage limit".into(),
        k => k.replace('_', " "),
    }
}

fn window_name(minutes: Option<i64>) -> String {
    match minutes {
        Some(300) => "5-hour limit".into(),
        Some(10_080) => "Weekly limit".into(),
        Some(m) if m % 1440 == 0 => format!("{}-day limit", m / 1440),
        Some(m) if m % 60 == 0 => format!("{}-hour limit", m / 60),
        Some(m) => format!("{m}-minute limit"),
        None => "Usage limit".into(),
    }
}

impl Usage {
    pub fn new(dir: &Path) -> Usage {
        let cache_file = dir.join("usage-scan.json");
        let ledger_file = dir.join("usage.json");
        let scans = std::fs::read(&cache_file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        let ledger = std::fs::read(&ledger_file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        Usage { cache_file, ledger_file, scans: Mutex::new(scans), ledger: Mutex::new(ledger) }
    }

    /// Add what one of OmniHub's own replies used.
    pub fn record(&self, claude: Option<Tokens>, openai: Option<Tokens>) {
        let mut l = self.ledger.lock();
        let day = l.entry(local_date(crate::db::now())).or_default();
        if let Some(t) = claude {
            day.claude.add(&t);
        }
        if let Some(t) = openai {
            day.openai.add(&t);
        }
        if let Ok(b) = serde_json::to_vec(&*l) {
            let tmp = self.ledger_file.with_extension("json.tmp");
            if std::fs::write(&tmp, b).is_ok() {
                let _ = std::fs::rename(&tmp, &self.ledger_file);
            }
        }
    }

    pub fn report(&self, claude_home: Option<&Path>, codex_home: Option<&Path>, maker: Tokens, now: i64) -> Report {
        let mut r = Report::default();
        {
            let l = self.ledger.lock();
            let claude: BTreeMap<String, Tokens> = l.iter().map(|(d, v)| (d.clone(), v.claude)).collect();
            let openai: BTreeMap<String, Tokens> = l.iter().map(|(d, v)| (d.clone(), v.openai)).collect();
            r.omni = OwnUsage { claude: period(&claude, now), openai: period(&openai, now), maker };
        }
        let mut scans = self.scans.lock();
        if let Some(h) = claude_home {
            let files: Vec<PathBuf> = walkdir::WalkDir::new(h.join("projects")).max_depth(4).into_iter().flatten().filter(|e| e.file_type().is_file() && e.path().extension().is_some_and(|x| x == "jsonl")).map(|e| e.into_path()).collect();
            let keys: std::collections::HashSet<String> = files.iter().map(|p| p.to_string_lossy().into_owned()).collect();
            scans.claude.retain(|k, _| keys.contains(k));
            let mut hours: BTreeMap<i64, Tokens> = BTreeMap::new();
            let mut limit: Option<LimitSeen> = None;
            for p in &files {
                let s = scans.claude.entry(p.to_string_lossy().into_owned()).or_default();
                update(p, s, claude_line);
                for (h, t) in &s.hours {
                    hours.entry(*h).or_default().add(t);
                }
                if let Some(l) = &s.limit {
                    if limit.as_ref().is_none_or(|x| l.at > x.at) {
                        limit = Some(l.clone());
                    }
                }
            }
            r.claude_code.found = !files.is_empty();
            r.claude_code.period = period(&by_day(&hours), now);
            r.claude_code.window = window(&hours, now);
            if let Some(l) = limit.filter(|l| l.resets_at.is_some_and(|t| t > now)) {
                r.claude_code.limits_at = Some(l.at);
                r.claude_code.limits.push(Limit { name: limit_name(&l.kind), used_percent: None, reached: l.status == "rejected", resets_at: l.resets_at });
            }
        }
        if let Some(h) = codex_home {
            let files = super::codex::files(h);
            let keys: std::collections::HashSet<String> = files.iter().map(|p| p.to_string_lossy().into_owned()).collect();
            scans.codex.retain(|k, _| keys.contains(k));
            let mut hours: BTreeMap<i64, Tokens> = BTreeMap::new();
            let (mut rate_at, mut rate) = (0, Value::Null);
            for p in &files {
                let s = scans.codex.entry(p.to_string_lossy().into_owned()).or_default();
                update(p, s, codex_line);
                for (h, t) in &s.hours {
                    hours.entry(*h).or_default().add(t);
                }
                if s.rate.is_object() && s.rate_at > rate_at {
                    (rate_at, rate) = (s.rate_at, s.rate.clone());
                }
            }
            r.codex.found = !files.is_empty();
            r.codex.period = period(&by_day(&hours), now);
            if rate.is_object() {
                r.codex.limits_at = Some(rate_at);
                r.codex.plan = rate["plan_type"].as_str().map(String::from);
                for w in [&rate["primary"], &rate["secondary"]] {
                    if !w.is_object() {
                        continue;
                    }
                    let resets_at = w["resets_at"].as_i64().or_else(|| w["resets_in_seconds"].as_i64().map(|s| rate_at + s));
                    // A window that has reset since says nothing about now.
                    let fresh = resets_at.is_none_or(|t| t > now);
                    let used = w["used_percent"].as_f64().filter(|_| fresh);
                    r.codex.limits.push(Limit { name: window_name(w["window_minutes"].as_i64()), used_percent: used, reached: used.is_some_and(|u| u >= 100.0), resets_at: resets_at.filter(|_| fresh) });
                }
            }
        }
        if let Ok(b) = serde_json::to_vec(&*scans) {
            let tmp = self.cache_file.with_extension("json.tmp");
            if std::fs::write(&tmp, b).is_ok() {
                let _ = std::fs::rename(&tmp, &self.cache_file);
            }
        }
        r
    }
}
