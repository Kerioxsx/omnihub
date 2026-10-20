//! The mod maker: Claude, with the user's API key, makes a mod for one of
//! their games. Each mod is a project: a conversation, and a folder of
//! files Claude writes. Claude can look at the game's files (never change
//! them), search the web for the mod loader's documentation, and write only
//! inside the project's own folder. C# mods are built with the .NET SDK
//! only when the user approves. The user installs the result with one click
//! (through the mod manager, so it switches off and removes cleanly).
//!
//! The conversation is kept exactly as the API returned it and only ever
//! appended to, so Claude's thinking stays valid from turn to turn.

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::detect::Engine;
use super::{MakerKind, ModGame, ModHub};
use crate::claude::api::{self, Delta, Usage};
use crate::claude::Claude;
use crate::events::EventBus;

/// Tool rounds Claude gets per message before it pauses.
const MAX_ROUNDS: usize = 60;
const MAX_FILE: usize = 1 << 20;
const MAX_FILES: usize = 300;
const MAX_TOTAL: u64 = 20 << 20;
const READ_CHARS: usize = 60_000;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    #[default]
    Idle,
    Working,
    /// Waiting for the user to approve something (a build).
    Waiting,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Approval {
    pub id: String,
    pub what: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Project {
    pub id: String,
    pub game: String,
    pub game_name: String,
    pub name: String,
    pub kind: MakerKind,
    pub created: i64,
    pub updated: i64,
    pub status: Status,
    /// Claude said the mod is ready (version counts each time).
    pub ready: bool,
    pub version: u32,
    pub summary: String,
    pub how_to_use: String,
    /// The installed mod (its id in the mod manager) and which version it is.
    pub installed: Option<String>,
    pub installed_version: u32,
    pub usage: Usage,
    pub cost: f64,
    pub model: String,
    /// Web search was on when the project started (fixed, so the conversation stays valid).
    pub web: bool,
    pub approval: Option<Approval>,
    pub error: Option<String>,
}

impl Default for Project {
    fn default() -> Self {
        Project {
            id: String::new(),
            game: String::new(),
            game_name: String::new(),
            name: String::new(),
            kind: MakerKind::Files,
            created: 0,
            updated: 0,
            status: Status::Idle,
            ready: false,
            version: 0,
            summary: String::new(),
            how_to_use: String::new(),
            installed: None,
            installed_version: 0,
            usage: Usage::default(),
            cost: 0.0,
            model: String::new(),
            web: true,
            approval: None,
            error: None,
        }
    }
}

/// One line of the conversation as the window shows it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    /// user, text, note, tool, web, notice
    pub kind: String,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tone: Option<String>,
    pub at: i64,
}

fn item(kind: &str, text: impl Into<String>) -> Item {
    Item { kind: kind.into(), text: text.into(), detail: None, tone: None, at: crate::db::now() }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FileInfo {
    pub path: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct View {
    pub project: Project,
    pub items: Vec<Item>,
    pub files: Vec<FileInfo>,
}

/// Builds a C# project into a folder; returns the build's output.
pub type Builder = Arc<dyn Fn(&Path, &Path) -> Result<String, String> + Send + Sync>;

struct Run {
    cancel: Arc<AtomicBool>,
    approval: Option<std::sync::mpsc::Sender<bool>>,
}

pub struct Maker {
    dir: PathBuf,
    events: EventBus,
    claude: Arc<Claude>,
    runs: Mutex<HashMap<String, Run>>,
    /// Serialises writes to a project's files on disk.
    io: Mutex<()>,
    /// Serialises read-change-write of project.json.
    edit: Mutex<()>,
    /// Builds C# mods (replaced in tests).
    pub builder: Mutex<Builder>,
}

fn now() -> i64 {
    crate::db::now()
}

/// What Claude's tools may touch for one project.
struct Area {
    game: PathBuf,
    data: Option<PathBuf>,
    modd: PathBuf,
}

#[derive(Debug)]
enum Target {
    Dir(PathBuf),
    File(PathBuf),
    /// An entry inside a zip-based file (`game/Balatro.exe!/card.lua`).
    Inside(PathBuf, String),
}

impl Area {
    /// `mod/…`, `game/…` or `data/…`, kept inside its folder. `write`:
    /// only `mod/` may be written.
    fn resolve(&self, path: &str, write: bool) -> Result<(Target, String), String> {
        let path = path.trim().trim_start_matches(['/', '\\']);
        let (area, rest) = path.split_once(['/', '\\']).unwrap_or((path, ""));
        let base = match area {
            "mod" => &self.modd,
            "game" if !write => &self.game,
            "data" if !write => self.data.as_ref().ok_or("This game has no data/ folder.")?,
            "game" | "data" => return Err("Only files under mod/ can be written. The game's own files stay as they are; your mod's files replace them when it's installed.".into()),
            _ => return Err("Paths start with mod/, game/ or data/.".into()),
        };
        let (file_part, inner) = match rest.split_once("!/") {
            Some((f, i)) => (f, Some(i.to_string())),
            None => (rest, None),
        };
        let rel = if file_part.is_empty() { String::new() } else { super::install::safe_rel(file_part).ok_or("That path leaves its folder.")? };
        let full = if rel.is_empty() { base.clone() } else { base.join(&rel) };
        let shown = format!("{area}/{rel}");
        if let Some(inner) = inner {
            if write {
                return Err("Files inside archives can't be written.".into());
            }
            return Ok((Target::Inside(full, inner.trim_start_matches('/').to_string()), format!("{shown}!/{inner}")));
        }
        if full.is_dir() {
            Ok((Target::Dir(full), shown))
        } else {
            Ok((Target::File(full), shown))
        }
    }
}

fn is_text(b: &[u8]) -> bool {
    !b[..b.len().min(8192)].contains(&0)
}

/// File endings Claude may not write: programs and scripts Windows runs.
fn blocked_ext(p: &str) -> bool {
    let l = p.to_lowercase();
    [".exe", ".dll", ".asi", ".bat", ".cmd", ".ps1", ".psm1", ".vbs", ".vbe", ".wsf", ".com", ".scr", ".msi", ".lnk", ".reg", ".sys", ".jar", ".hta"].iter().any(|x| l.ends_with(x))
}

fn open_zip(p: &Path) -> Result<zip::ZipArchive<std::fs::File>, String> {
    zip::ZipArchive::new(std::fs::File::open(p).map_err(|e| e.to_string())?).map_err(|_| "That file isn't an archive OmniHub can look inside.".to_string())
}

fn read_inside(p: &Path, inner: &str) -> Result<Vec<u8>, String> {
    let mut z = open_zip(p)?;
    let mut f = z.by_name(inner).map_err(|_| format!("There's no {inner} inside it."))?;
    let mut out = Vec::new();
    (&mut f).take(8 << 20).read_to_end(&mut out).map_err(|e| e.to_string())?;
    Ok(out)
}

/// Lines `start..start+count` of a text, within READ_CHARS.
fn window(text: &str, start: usize, count: usize) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let total = lines.len();
    let start = start.max(1);
    let mut out = String::new();
    let mut last = start.saturating_sub(1);
    for (i, l) in lines.iter().enumerate().skip(start - 1).take(count) {
        if out.len() + l.len() + 1 > READ_CHARS {
            break;
        }
        out.push_str(l);
        out.push('\n');
        last = i + 1;
    }
    if total == 0 {
        return "(empty file)".into();
    }
    let head = if start > 1 || last < total { format!("[lines {start}–{last} of {total}; ask for more with start_line]\n") } else { String::new() };
    head + &out
}

fn list_dir(dir: &Path, shown: &str) -> String {
    let mut entries: Vec<(bool, String, u64)> = std::fs::read_dir(dir).map(|rd| rd.flatten().map(|e| {
        let md = e.metadata().ok();
        (md.as_ref().is_some_and(|m| m.is_dir()), e.file_name().to_string_lossy().into_owned(), md.map(|m| m.len()).unwrap_or(0))
    }).collect()).unwrap_or_default();
    entries.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.to_lowercase().cmp(&b.1.to_lowercase())));
    let n = entries.len();
    let mut out = format!("{shown} ({n} entries)\n");
    for (d, name, size) in entries.into_iter().take(400) {
        if d {
            out.push_str(&format!("{name}/\n"));
        } else {
            out.push_str(&format!("{name}  ({size} bytes)\n"));
        }
    }
    if n > 400 {
        out.push_str(&format!("… and {} more\n", n - 400));
    }
    out
}

fn list_inside(p: &Path, prefix: &str, shown: &str) -> Result<String, String> {
    let z = open_zip(p)?;
    let prefix = prefix.trim_start_matches('/');
    let mut names: Vec<&str> = z.file_names().filter(|n| n.starts_with(prefix)).collect();
    names.sort();
    let n = names.len();
    let mut out = format!("{shown} ({n} entries)\n");
    for name in names.into_iter().take(600) {
        out.push_str(name);
        out.push('\n');
    }
    if n > 600 {
        out.push_str(&format!("… and {} more (narrow the path)\n", n - 600));
    }
    Ok(out)
}

fn search_text(name: &str, text: &str, words: &str, hits: &mut Vec<String>) {
    for (i, l) in text.lines().enumerate() {
        if hits.len() >= 120 {
            return;
        }
        if l.to_lowercase().contains(words) {
            let t: String = l.trim().chars().take(200).collect();
            hits.push(format!("{name}:{}: {t}", i + 1));
        }
    }
}

/// A tool call's inputs, checked: they stream in unvalidated.
fn arg<'a>(input: &'a Value, k: &str) -> Result<&'a str, String> {
    input.get(k).and_then(Value::as_str).ok_or_else(|| format!("Missing the “{k}” text."))
}

fn tools(kind: MakerKind, web: bool, model: &str) -> Vec<Value> {
    let tool = |name: &str, desc: &str, props: Value, required: &[&str]| json!({ "name": name, "description": desc, "eager_input_streaming": true, "input_schema": { "type": "object", "properties": props, "required": required } });
    let mut v = vec![
        tool("list_files", "List a folder: mod/ (your mod), game/ (the game's install folder) or data/ (the game's own mod/data folder, when it has one). Add !/ after a zip-based file to list what's inside it (e.g. game/Balatro.exe!/). Call this before reading, to find the right files.", json!({ "path": { "type": "string", "description": "e.g. game/ or game/BepInEx/ or game/Balatro.exe!/" } }), &["path"]),
        tool("read_file", "Read a text file (game files, logs, docs, or your own mod files). Use it to learn how the game and its loader work before writing code, and to read logs after the user plays. Long files come in parts: pass start_line to read further.", json!({ "path": { "type": "string" }, "start_line": { "type": "integer", "description": "1-based first line (default 1)" }, "max_lines": { "type": "integer", "description": "default 800" } }), &["path"]),
        tool("search_files", "Find text in the files under a folder (or inside a zip-based file), case-insensitive. Use it to find where the game defines something you want to change.", json!({ "path": { "type": "string" }, "text": { "type": "string" } }), &["path", "text"]),
        tool("write_file", "Create or replace a text file of your mod, under mod/. Programs and scripts Windows runs (.exe, .dll, .bat, .ps1…) can't be written.", json!({ "path": { "type": "string", "description": "under mod/" }, "content": { "type": "string", "description": "the whole file" } }), &["path", "content"]),
        tool("edit_file", "Change part of one of your mod's files: old_text must appear exactly once.", json!({ "path": { "type": "string" }, "old_text": { "type": "string" }, "new_text": { "type": "string" } }), &["path", "old_text", "new_text"]),
        tool("delete_file", "Delete one of your mod's files.", json!({ "path": { "type": "string" } }), &["path"]),
        tool("finish", "Call when the mod is complete and ready for the user to install (they press Install). Give a short plain-words summary of what it does, and how to use it in the game (keys, menus). Call it again after each fix.", json!({ "name": { "type": "string", "description": "a short name for the mod" }, "summary": { "type": "string" }, "how_to_use": { "type": "string" } }), &["summary", "how_to_use"]),
    ];
    if kind.compiled() {
        v.push(tool("build", "Build the C# project in mod/ with the .NET SDK (dotnet build). The user approves each build. Returns the compiler's output; fix any errors and build again before calling finish.", json!({}), &[]));
    }
    if web {
        let t = crate::claude::traits(model);
        v.push(json!({ "type": t.web_search, "name": "web_search", "max_uses": 8 }));
        v.push(json!({ "type": t.web_fetch, "name": "web_fetch", "max_uses": 12 }));
    }
    v
}

/// The Unity version, from the start of the game's globalgamemanagers file.
fn unity_version(game: &Path) -> Option<String> {
    let data = std::fs::read_dir(game).ok()?.flatten().find(|e| e.file_name().to_string_lossy().to_lowercase().ends_with("_data"))?.path();
    let mut buf = vec![0u8; 512];
    let n = std::fs::File::open(data.join("globalgamemanagers")).ok()?.read(&mut buf).ok()?;
    let text: String = buf[..n].iter().map(|&b| if b.is_ascii_graphic() { b as char } else { ' ' }).collect();
    text.split_whitespace().find(|w| {
        let parts: Vec<&str> = w.split('.').collect();
        parts.len() == 3 && parts[0].len() == 4 && parts[0].parse::<u32>().is_ok() && parts[2].chars().next().is_some_and(|c| c.is_ascii_digit()) && parts[2].contains(['f', 'p', 'b', 'a'])
    }).map(String::from)
}

fn system_prompt(g: &ModGame, kind: MakerKind, game_dir: &Path, unity: Option<&str>) -> String {
    let loaders = if g.loaders.is_empty() { "none needed".to_string() } else { g.loaders.iter().map(|l| format!("{} ({})", l.label, if l.installed { "installed" } else if l.auto { "OmniHub installs it with the mod" } else { "the user must get it themselves" })).collect::<Vec<_>>().join(", ") };
    let data_line = match &g.data_dir {
        Some(d) => format!("- data/ = the game's own folder for mods and logs ({d}), read-only.\n"),
        None => String::new(),
    };
    let managed = game_dir.join(format!("{}_Data", g.exe.as_deref().unwrap_or("Game").trim_end_matches(".exe"))).join("Managed");
    let guide = match kind {
        MakerKind::Steamodded => "You're making a Balatro mod for Steamodded (smods), loaded by Lovely. Your mod/ folder becomes %AppData%\\Balatro\\Mods\\<mod>\\. It needs a metadata file (mod/<id>.json in Steamodded's current format: id, name, author list, description, prefix, main_file, version, dependencies such as \"Steamodded (>=1.0.0~BETA)\") and the main Lua file it names; art goes in mod/assets/1x and mod/assets/2x (the 2x copy is the 1x art scaled up twice). Balatro's own Lua code is inside the game program: list game/Balatro.exe!/ and read files such as game/Balatro.exe!/card.lua or game/Balatro.exe!/functions/common_events.lua to see how things work. Look up Steamodded's API (its wiki at github.com/Steamodded/smods/wiki) rather than guessing: SMODS.Joker, SMODS.Atlas, SMODS.Consumable, loc_txt, calculate(self, card, context) and its context fields. Read data/Mods/lovely/log/ after the user plays to find errors.",
        MakerKind::Lovely => "You're making a mod for a LÖVE game loaded by Lovely. Your mod/ folder becomes the game's Mods\\<mod>\\ folder in %AppData%. Lovely patches go in mod/lovely.toml or mod/lovely/*.toml (pattern or regex patches that insert Lua into the game's files; look up Lovely's patch format on github.com/ethangreen-dev/lovely-injector). The game's Lua is inside its program: list game/<program>.exe!/ to read it.",
        MakerKind::ShvdnScript => "You're making a GTA V story-mode script for Script Hook V .NET (SHVDN, API v3), which compiles .cs files itself when the game starts. Your mod/ files go into the game's scripts\\ folder: write one or more C# files such as mod/MyMod.cs (a public class deriving from GTA.Script, with Tick and KeyDown handlers; using GTA; using GTA.Math; using GTA.Native;) and an optional .ini for settings. Look up the SHVDN v3 API (github.com/scripthookvdotnet/scripthookvdotnet and its wiki, and the native function database at nativedb.dotindustries.dev) before using a function. Pick keys that GTA doesn't already use, and show the player what the mod does (GTA.UI.Notification.Show). After the user plays, read game/ScriptHookVDotNet.log for errors. Story mode only: nothing that works in GTA Online.",
        MakerKind::Ue4ssLua => "You're making a Lua mod for UE4SS (an Unreal Engine script system). Your mod/ folder becomes ue4ss\\Mods\\<mod>\\ next to the game's program; the entry point is mod/Scripts/main.lua (OmniHub adds enabled.txt). Use UE4SS's Lua API (RegisterHook, NotifyOnNewObject, FindFirstOf, FindAllOf, RegisterKeyBind, ExecuteWithDelay; docs at docs.ue4ss.com). Class and property names come from the game: if a UE4SS object dump exists under the program folder, read it; otherwise ask the user to run the game once with UE4SS and use its live view. Read UE4SS.log next to the game's program after the user plays.",
        MakerKind::BepInExPlugin => "You're making a BepInEx 5 plugin (C#) for a Unity game. Write a small SDK-style project in mod/: one .csproj (Sdk=\"Microsoft.NET.Sdk\", TargetFramework netstandard2.1 — or net472 for older Unity — with PackageReferences only to BepInEx.Core 5.*, BepInEx.PluginInfoProps, UnityEngine.Modules (the game's Unity version) and HarmonyX, restored from https://nuget.bepinex.dev/v3/index.json via RestoreAdditionalProjectSources) and the plugin's .cs files (a class deriving from BaseUnityPlugin with [BepInPlugin], Harmony patches where needed). Reference the game's own code with <Reference Include=\"Assembly-CSharp\"><HintPath>…</HintPath><Private>false</Private></Reference> using the Managed folder below. No build steps, targets or scripts in the project: the build refuses them. Call build, fix every error, then finish. Read game/BepInEx/LogOutput.log after the user plays.",
        MakerKind::MelonMod => "You're making a MelonLoader mod (C#) for a Unity game. Write a small SDK-style project in mod/: one .csproj (Sdk=\"Microsoft.NET.Sdk\") referencing MelonLoader and the game's assemblies with HintPaths (game/MelonLoader/net6/MelonLoader.dll or net35; IL2CPP games: game/MelonLoader/Il2CppAssemblies/ after the game ran once with MelonLoader) and Private=false, and a class deriving from MelonMod with the MelonInfo and MelonGame attributes. No build steps, targets or scripts in the project: the build refuses them. Call build, fix every error, then finish. Read game/MelonLoader/Latest.log after the user plays.",
        MakerKind::Files => "OmniHub doesn't know a script loader for this game, so mods are files: your mod/ folder is copied into the game's folder with the same paths (mod/Data/x.json replaces game/Data/x.json). Change the game's config, data, text or other plain files: read the game's file first, copy it into mod/ at the same path and change only what's needed.",
    };
    let unity_line = unity.map(|u| format!("- Unity version: {u}\n")).unwrap_or_default();
    let managed_line = if kind == MakerKind::BepInExPlugin { format!("- The game's .NET assemblies (for HintPaths): {}\n", managed.display()) } else { String::new() };
    format!(
        "You are Claude, working inside OmniHub (a Windows app) to make a mod for the user's copy of {name}. The user is a player, not necessarily a programmer: talk to them in short, plain sentences, and in their language.\n\n\
        The game:\n- {name} ({engine}{steam}), installed at {dir}\n- Mods: {method}. Loaders: {loaders}.\n{unity_line}{managed_line}- Anti-cheat: {ac}\n\n\
        Your tools see three folders:\n- mod/ = your mod's own folder. You write here (write_file, edit_file, delete_file); nothing else can be written.\n- game/ = the game's install folder, read-only.\n{data_line}\n\
        What to make: {guide}\n\n\
        How to work:\n1. If the request is unclear, ask one short question first. Otherwise look before you write: list and read the game's files and the loader's documentation (search the web when you're unsure of an API rather than guessing). Keep reading focused: open the files that matter, don't dump whole folders.\n2. Write the mod: small, clean, commented code that does exactly what was asked. Prefer adding over replacing the game's files.\n3. When it's complete, call finish. Tell the user in one or two sentences what you made and that they can press Install, then play.\n4. When the user reports a problem, read the logs, fix it, and call finish again.\n\n\
        Rules: make mods for single-player or offline play, or harmless cosmetic and quality-of-life changes. Never make cheats or anything that gives an advantage over other players online, never disable, bypass or hide from anti-cheat or DRM, and never make anything that harms the PC, collects data, or downloads or runs programs. If asked, say plainly that you won't, and offer an offline alternative. Don't copy copyrighted art or code from elsewhere into the mod; write your own.",
        name = g.name,
        engine = g.engine_label,
        steam = g.app_id.map(|a| format!(", Steam app {a}")).unwrap_or_default(),
        dir = game_dir.display(),
        method = g.method_label,
        ac = if g.anti_cheat.is_empty() { "none found".to_string() } else { g.anti_cheat.join(", ") },
    )
}

/// Safety checks on a C# project before it's built: no build steps, scripts,
/// imported targets or packages beyond the usual modding ones.
pub fn check_project(dir: &Path) -> Result<PathBuf, String> {
    let files = super::install::files_in(dir);
    let projects: Vec<&String> = files.iter().filter(|f| f.to_lowercase().ends_with(".csproj")).collect();
    let [proj] = projects.as_slice() else { return Err(format!("mod/ needs exactly one .csproj (it has {}).", projects.len())) };
    for f in &files {
        let l = f.to_lowercase();
        let name = l.rsplit('/').next().unwrap_or(&l);
        if l.ends_with(".props") || l.ends_with(".targets") || name == "nuget.config" || name == "global.json" || name.starts_with("directory.build") || name.starts_with("directory.packages") {
            return Err(format!("{f} isn't allowed: builds use only the .csproj."));
        }
    }
    let text = std::fs::read_to_string(dir.join(proj)).map_err(|e| e.to_string())?;
    let low = text.to_lowercase();
    for bad in ["<exec", "<usingtask", "<target ", "<target>", "<import", "taskfactory", "<project sdk=\"microsoft.net.sdk.", "beforebuild", "afterbuild", "prebuildevent", "postbuildevent", "$(registry", "[system.", "[msbuild]::"] {
        if low.contains(bad) {
            return Err(format!("The project contains “{bad}…”, which builds refuse: only plain references and settings are allowed."));
        }
    }
    if !low.contains("<project sdk=\"microsoft.net.sdk\"") {
        return Err("The project must start with <Project Sdk=\"Microsoft.NET.Sdk\">.".into());
    }
    const PACKAGES: &[&str] = &["bepinex.", "bepinex", "unityengine.modules", "harmonyx", "lavagang.melonloader", "microsoft.netframework.referenceassemblies", "lib.harmony"];
    for part in low.split("<packagereference").skip(1) {
        let inc = part.split("include=\"").nth(1).and_then(|s| s.split('"').next()).unwrap_or_default();
        if !PACKAGES.iter().any(|p| inc == *p || inc.starts_with(p)) {
            return Err(format!("The package “{inc}” isn't one of the usual modding packages, so the build won't fetch it."));
        }
    }
    for part in low.split("restoreadditionalprojectsources>").skip(1).step_by(2) {
        let src = part.split('<').next().unwrap_or_default().trim();
        if !src.is_empty() && src != "https://nuget.bepinex.dev/v3/index.json" {
            return Err(format!("The package source “{src}” isn't allowed (only BepInEx's own)."));
        }
    }
    Ok(dir.join(proj))
}

/// Build with the .NET SDK: `dotnet build` into `out`, five minutes at most.
fn dotnet_build(project: &Path, out: &Path) -> Result<String, String> {
    let mut c = std::process::Command::new("dotnet");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000);
    }
    c.arg("build").arg(project).args(["-c", "Release", "--nologo", "-v", "q", "-o"]).arg(out).env("DOTNET_CLI_TELEMETRY_OPTOUT", "1").env("DOTNET_NOLOGO", "1").env("MSBUILDDISABLENODEREUSE", "1");
    c.stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped());
    let mut child = c.spawn().map_err(|e| if e.kind() == std::io::ErrorKind::NotFound { "The .NET SDK isn't installed. Install it (free, from dotnet.microsoft.com/download), then build again.".to_string() } else { e.to_string() })?;
    let start = std::time::Instant::now();
    loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            let mut o = String::new();
            let mut e = String::new();
            if let Some(mut s) = child.stdout.take() {
                let _ = s.read_to_string(&mut o);
            }
            if let Some(mut s) = child.stderr.take() {
                let _ = s.read_to_string(&mut e);
            }
            let text = format!("{o}{e}");
            let tail: String = text.chars().rev().take(6000).collect::<Vec<_>>().into_iter().rev().collect();
            return if status.success() { Ok(tail) } else { Err(format!("The build failed:\n{tail}")) };
        }
        if start.elapsed() > Duration::from_secs(300) {
            let _ = child.kill();
            return Err("The build took over five minutes and was stopped.".into());
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

impl Maker {
    pub fn new(dir: &Path, events: EventBus, claude: Arc<Claude>) -> Maker {
        Maker { dir: dir.to_path_buf(), events, claude, runs: Mutex::new(HashMap::new()), io: Mutex::new(()), edit: Mutex::new(()), builder: Mutex::new(Arc::new(|p: &Path, o: &Path| dotnet_build(p, o))) }
    }

    fn pdir(&self, id: &str) -> Result<PathBuf, String> {
        if id.is_empty() || !id.bytes().all(|b| b.is_ascii_alphanumeric()) {
            return Err("No such mod project.".into());
        }
        Ok(self.dir.join(id))
    }

    fn load(&self, id: &str) -> Result<Project, String> {
        let p = self.pdir(id)?;
        std::fs::read(p.join("project.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).ok_or_else(|| "No such mod project.".into())
    }

    fn store(&self, p: &Project) -> Result<(), String> {
        let _g = self.io.lock();
        let d = self.pdir(&p.id)?;
        std::fs::create_dir_all(&d).map_err(|e| e.to_string())?;
        let tmp = d.join("project.json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(p).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, d.join("project.json")).map_err(|e| e.to_string())
    }

    fn messages(&self, id: &str) -> Vec<Value> {
        self.pdir(id).ok().and_then(|d| std::fs::read(d.join("messages.json")).ok()).and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
    }

    fn save_messages(&self, id: &str, m: &[Value]) -> Result<(), String> {
        let _g = self.io.lock();
        let d = self.pdir(id)?;
        let tmp = d.join("messages.json.tmp");
        std::fs::write(&tmp, serde_json::to_vec(m).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, d.join("messages.json")).map_err(|e| e.to_string())
    }

    fn items(&self, id: &str) -> Vec<Item> {
        self.pdir(id).ok().and_then(|d| std::fs::read(d.join("log.json")).ok()).and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
    }

    fn log(&self, id: &str, it: Item) {
        {
            let _g = self.io.lock();
            let mut v = self.items(id);
            v.push(it);
            if let Ok(d) = self.pdir(id) {
                let _ = std::fs::write(d.join("log.json"), serde_json::to_vec(&v).unwrap_or_default());
            }
        }
        self.changed(id);
    }

    fn changed(&self, id: &str) {
        self.events.emit("maker:changed", json!({ "project": id }));
    }

    fn delta(&self, id: &str, kind: &str, text: &str) {
        self.events.emit("maker:delta", json!({ "project": id, "kind": kind, "text": text }));
    }

    fn set(&self, id: &str, f: impl FnOnce(&mut Project)) -> Result<Project, String> {
        let _e = self.edit.lock();
        let mut p = self.load(id)?;
        f(&mut p);
        p.updated = now();
        self.store(&p)?;
        self.changed(id);
        Ok(p)
    }

    pub fn projects(&self, game: Option<&str>) -> Vec<Project> {
        let mut v: Vec<Project> = std::fs::read_dir(&self.dir)
            .map(|rd| rd.flatten().filter_map(|e| self.load(&e.file_name().to_string_lossy()).ok()).filter(|p| game.is_none_or(|g| p.game == g)).collect())
            .unwrap_or_default();
        v.sort_by_key(|p| std::cmp::Reverse(p.updated));
        v
    }

    pub fn view(&self, id: &str) -> Result<View, String> {
        let project = self.load(id)?;
        let md = self.pdir(id)?.join("mod");
        let files = super::install::files_in(&md).into_iter().map(|f| FileInfo { size: std::fs::metadata(md.join(&f)).map(|m| m.len()).unwrap_or(0), path: f }).collect();
        Ok(View { project, items: self.items(id), files })
    }

    /// One of the project's files, for the window to show.
    pub fn file(&self, id: &str, path: &str) -> Result<String, String> {
        let rel = super::install::safe_rel(path).ok_or("Bad path.")?;
        let b = std::fs::read(self.pdir(id)?.join("mod").join(rel)).map_err(|e| e.to_string())?;
        if !is_text(&b) {
            return Err("It isn't a text file.".into());
        }
        Ok(String::from_utf8_lossy(&b).into_owned())
    }

    pub fn delete(&self, id: &str) -> Result<(), String> {
        self.stop(id);
        let d = self.pdir(id)?;
        if d.is_dir() {
            std::fs::remove_dir_all(d).map_err(|e| e.to_string())?;
        }
        self.changed(id);
        Ok(())
    }

    pub fn rename(&self, id: &str, name: &str) -> Result<Project, String> {
        let name: String = name.trim().chars().take(60).collect();
        if name.is_empty() {
            return Err("Give it a name.".into());
        }
        self.set(id, |p| p.name = name)
    }

    /// Start a mod for a game with the user's first message.
    pub fn create(&self, hub: &Arc<ModHub>, game: &str, text: &str) -> Result<Project, String> {
        let (g, _) = hub.maker_context(game)?;
        let kind = g.maker.kind.filter(|_| g.maker.available).ok_or_else(|| g.maker.why.clone().unwrap_or_else(|| "Claude can't make mods for this game.".into()))?;
        if !self.claude.has_key() {
            return Err("Add your Anthropic API key first (Claude settings).".into());
        }
        let s = self.claude.settings();
        let id = uuid::Uuid::new_v4().simple().to_string();
        let name: String = text.split(['.', '\n', '!', '?']).next().unwrap_or(text).trim().chars().take(48).collect();
        let p = Project { id: id.clone(), game: g.key.clone(), game_name: g.name.clone(), name: if name.is_empty() { "New mod".into() } else { name }, kind, created: now(), updated: now(), model: s.model.clone(), web: s.web, ..Default::default() };
        std::fs::create_dir_all(self.pdir(&id)?.join("mod")).map_err(|e| e.to_string())?;
        self.store(&p)?;
        self.send(hub, &id, text)?;
        self.load(&id)
    }

    /// Send the user's message; Claude works on it in the background.
    pub fn send(&self, hub: &Arc<ModHub>, id: &str, text: &str) -> Result<(), String> {
        let text = text.trim();
        if text.is_empty() {
            return Err("Say what the mod should do.".into());
        }
        if text.len() > 20_000 {
            return Err("That message is too long.".into());
        }
        let p = self.load(id)?;
        let mut runs = self.runs.lock();
        if runs.contains_key(id) {
            return Err("Claude is still working on the last message. Wait, or press Stop.".into());
        }
        let mut msgs = self.messages(id);
        // Append-only: a message after an unanswered one joins it.
        match msgs.last_mut() {
            Some(last) if last["role"] == "user" => {
                if let Some(c) = last["content"].as_array_mut() {
                    c.push(json!({ "type": "text", "text": text }));
                }
            }
            _ => msgs.push(json!({ "role": "user", "content": [{ "type": "text", "text": text }] })),
        }
        self.save_messages(id, &msgs)?;
        self.log(id, item("user", text));
        let cancel = Arc::new(AtomicBool::new(false));
        runs.insert(id.to_string(), Run { cancel: cancel.clone(), approval: None });
        drop(runs);
        self.set(id, |p| {
            p.status = Status::Working;
            p.error = None;
        })?;
        let hub = hub.clone();
        let pid = p.id.clone();
        std::thread::Builder::new()
            .name("mod-maker".into())
            .spawn(move || {
                let m = &hub.maker;
                let res = m.run(&hub, &pid, &cancel);
                m.runs.lock().remove(&pid);
                let _ = m.set(&pid, |p| {
                    p.approval = None;
                    match &res {
                        Ok(()) => p.status = Status::Idle,
                        Err(e) => {
                            p.status = if e == "Stopped." { Status::Idle } else { Status::Error };
                            p.error = Some(e.clone()).filter(|e| e != "Stopped.");
                        }
                    }
                });
                match res {
                    Err(e) if e == "Stopped." => m.log(&pid, Item { tone: Some("info".into()), ..item("notice", "Stopped.") }),
                    Err(e) => m.log(&pid, Item { tone: Some("bad".into()), ..item("notice", e) }),
                    Ok(()) => {}
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn stop(&self, id: &str) {
        if let Some(r) = self.runs.lock().get_mut(id) {
            r.cancel.store(true, Ordering::SeqCst);
            if let Some(tx) = r.approval.take() {
                let _ = tx.send(false);
            }
        }
    }

    /// The user's answer to an approval (a build).
    pub fn approve(&self, id: &str, yes: bool) -> Result<(), String> {
        let tx = self.runs.lock().get_mut(id).and_then(|r| r.approval.take()).ok_or("Nothing is waiting for an answer.")?;
        let _ = tx.send(yes);
        Ok(())
    }

    fn ask(&self, id: &str, what: &str, cancel: &AtomicBool) -> bool {
        let (tx, rx) = std::sync::mpsc::channel();
        if let Some(r) = self.runs.lock().get_mut(id) {
            r.approval = Some(tx);
        }
        let aid = uuid::Uuid::new_v4().simple().to_string();
        let _ = self.set(id, |p| {
            p.status = Status::Waiting;
            p.approval = Some(Approval { id: aid, what: what.into() });
        });
        let mut yes = false;
        for _ in 0..(30 * 60 * 5) {
            if cancel.load(Ordering::SeqCst) {
                break;
            }
            match rx.recv_timeout(Duration::from_millis(200)) {
                Ok(v) => {
                    yes = v;
                    break;
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                Err(_) => break,
            }
        }
        let _ = self.set(id, |p| {
            p.status = Status::Working;
            p.approval = None;
        });
        yes
    }

    fn area(&self, hub: &Arc<ModHub>, p: &Project) -> Result<(ModGame, Area), String> {
        let (g, _) = hub.maker_context(&p.game)?;
        Ok((g.clone(), Area { game: PathBuf::from(&g.dir), data: g.data_dir.as_ref().map(PathBuf::from), modd: self.pdir(&p.id)?.join("mod") }))
    }

    fn run(&self, hub: &Arc<ModHub>, id: &str, cancel: &AtomicBool) -> Result<(), String> {
        let p = self.load(id)?;
        let (g, area) = self.area(hub, &p)?;
        let client = self.claude.client()?;
        let settings = self.claude.settings();
        let model = settings.model.clone();
        let t = crate::claude::traits(&model);
        let unity = matches!(g.engine, Engine::UnityMono | Engine::UnityIl2cpp).then(|| unity_version(&area.game)).flatten();
        let system = system_prompt(&g, p.kind, &area.game, unity.as_deref());
        let tool_defs = tools(p.kind, p.web, &model);
        let mut betas: Vec<&str> = Vec::new();
        if t.fallbacks {
            betas.push("server-side-fallback-2026-07-01");
        }
        if t.updates {
            betas.push("thinking-display-updates-2026-08-18");
        }
        let mut rounds = 0;
        loop {
            let mut msgs = self.messages(id);
            let mut body = json!({
                "model": model,
                "max_tokens": 64000,
                "stream": true,
                "thinking": { "type": "adaptive", "display": if t.updates { "updates" } else { "summarized" } },
                "output_config": { "effort": settings.effort },
                "cache_control": { "type": "ephemeral" },
                "system": [{ "type": "text", "text": system }],
                "tools": tool_defs,
                "messages": msgs,
            });
            if t.fallbacks {
                body["fallbacks"] = json!("default");
            }
            let mut text = String::new();
            let turn = client
                .stream(&body, &betas, cancel, &mut |d| match d {
                    Delta::Text(s) => {
                        text.push_str(&s);
                        self.delta(id, "text", &s);
                    }
                    Delta::Note(s) => self.delta(id, "note", &s),
                    Delta::Tool { name, .. } => self.delta(id, "tool", &name),
                    Delta::ServerTool { name } => self.delta(id, "web", &name),
                    Delta::Fallback { to } => self.delta(id, "fallback", &to),
                })
                .map_err(|e| e.to_string())?;
            let mut usage = turn.usage;
            if usage.web_searches == 0 {
                usage.web_searches = turn.content.iter().filter(|b| b["type"] == "server_tool_use" && b["name"] == "web_search").count() as u64;
            }
            let served = if turn.model.is_empty() { model.clone() } else { turn.model.clone() };
            self.set(id, |p| {
                p.usage.add(&usage);
                p.cost += crate::claude::cost(&served, &usage);
                p.model = served.clone();
            })?;
            if turn.stop_reason == "refusal" {
                let why = turn.stop_details["explanation"].as_str().map(|s| format!(" ({s})")).unwrap_or_default();
                self.log(id, Item { tone: Some("warn".into()), ..item("notice", format!("Claude declined this request{why}. Try describing what the mod should do differently."))});
                return Ok(());
            }
            let content = api::after_fallback(turn.content);
            self.log_turn(id, &content);
            msgs.push(json!({ "role": "assistant", "content": content }));
            self.save_messages(id, &msgs)?;
            match turn.stop_reason.as_str() {
                "pause_turn" => continue,
                "tool_use" | "max_tokens" => {}
                "model_context_window_exceeded" => {
                    self.log(id, Item { tone: Some("warn".into()), ..item("notice", "This conversation is too long to continue. Start a new mod and describe where this one got to.") });
                    return Ok(());
                }
                _ => return Ok(()),
            }
            let calls: Vec<Value> = content.iter().filter(|b| b["type"] == "tool_use").cloned().collect();
            if calls.is_empty() {
                if turn.stop_reason == "max_tokens" {
                    self.log(id, Item { tone: Some("info".into()), ..item("notice", "Claude's answer reached its length limit. Say “continue” to let it carry on.") });
                }
                return Ok(());
            }
            let mut results = Vec::new();
            for c in &calls {
                let tid = c["id"].as_str().unwrap_or_default();
                let (out, ok) = if turn.stop_reason == "max_tokens" {
                    ("Not run: your answer was cut off at its length limit before this call was complete. Try again with a smaller step (for a big file, write it in parts with edit_file).".to_string(), false)
                } else if cancel.load(Ordering::SeqCst) {
                    ("Not run: the user stopped this.".to_string(), false)
                } else {
                    match self.tool(id, &area, c["name"].as_str().unwrap_or_default(), &c["input"], cancel) {
                        Ok(s) => (s, true),
                        Err(e) => (e, false),
                    }
                };
                results.push(json!({ "type": "tool_result", "tool_use_id": tid, "content": out, "is_error": !ok }));
            }
            let mut msgs = self.messages(id);
            msgs.push(json!({ "role": "user", "content": results }));
            self.save_messages(id, &msgs)?;
            if cancel.load(Ordering::SeqCst) {
                return Err("Stopped.".into());
            }
            rounds += 1;
            if rounds >= MAX_ROUNDS {
                self.log(id, Item { tone: Some("info".into()), ..item("notice", "Claude paused after many steps. Say “continue” to let it carry on.") });
                return Ok(());
            }
        }
    }

    /// The window's lines for one response.
    fn log_turn(&self, id: &str, content: &[Value]) {
        for b in content {
            match b["type"].as_str().unwrap_or_default() {
                "text" => {
                    let t = b["text"].as_str().unwrap_or_default().trim();
                    if !t.is_empty() {
                        self.log(id, item("text", t));
                    }
                }
                "thinking" => {
                    let t = b["thinking"].as_str().unwrap_or_default().trim();
                    if !t.is_empty() && t != "This part of the response was interrupted before it finished." {
                        self.log(id, item("note", t));
                    }
                }
                "server_tool_use" => {
                    let i = &b["input"];
                    let what = match b["name"].as_str().unwrap_or_default() {
                        "web_search" => format!("Searched the web: {}", i["query"].as_str().unwrap_or_default()),
                        "web_fetch" => format!("Read {}", i["url"].as_str().unwrap_or_default()),
                        n => n.to_string(),
                    };
                    self.log(id, item("web", what));
                }
                "tool_use" => {
                    let i = &b["input"];
                    let path = i["path"].as_str().unwrap_or_default();
                    let what = match b["name"].as_str().unwrap_or_default() {
                        "list_files" => format!("Looked in {path}"),
                        "read_file" => format!("Read {path}"),
                        "search_files" => format!("Searched {path} for “{}”", i["text"].as_str().unwrap_or_default()),
                        "write_file" => format!("Wrote {path} ({} lines)", i["content"].as_str().map(|c| c.lines().count()).unwrap_or(0)),
                        "edit_file" => format!("Changed {path}"),
                        "delete_file" => format!("Deleted {path}"),
                        "build" => "Asked to build the mod".to_string(),
                        "finish" => "Finished the mod".to_string(),
                        n => n.to_string(),
                    };
                    self.log(id, item("tool", what));
                }
                _ => {}
            }
        }
    }

    fn mod_size(&self, area: &Area) -> (usize, u64) {
        let files = super::install::files_in(&area.modd);
        let total = files.iter().map(|f| std::fs::metadata(area.modd.join(f)).map(|m| m.len()).unwrap_or(0)).sum();
        (files.len(), total)
    }

    fn tool(&self, id: &str, area: &Area, name: &str, input: &Value, cancel: &AtomicBool) -> Result<String, String> {
        if let Some(raw) = input.get("INVALID_JSON") {
            return Err(json!({ "INVALID_JSON": raw }).to_string());
        }
        match name {
            "list_files" => {
                let (t, shown) = area.resolve(arg(input, "path")?, false)?;
                match t {
                    Target::Dir(d) => Ok(list_dir(&d, &shown)),
                    Target::Inside(f, inner) => list_inside(&f, &inner, &shown),
                    Target::File(f) if f.is_file() => list_inside(&f, "", &format!("{shown}!/")).or_else(|_| Ok(format!("{shown} is a file ({} bytes).", std::fs::metadata(&f).map(|m| m.len()).unwrap_or(0)))),
                    Target::File(_) => Err(format!("{shown} doesn't exist.")),
                }
            }
            "read_file" => {
                let (t, shown) = area.resolve(arg(input, "path")?, false)?;
                let start = input["start_line"].as_u64().unwrap_or(1).max(1) as usize;
                let count = input["max_lines"].as_u64().unwrap_or(800).clamp(1, 4000) as usize;
                let bytes = match t {
                    Target::Dir(_) => return Err(format!("{shown} is a folder: use list_files.")),
                    Target::Inside(f, inner) => read_inside(&f, &inner)?,
                    Target::File(f) => {
                        let len = std::fs::metadata(&f).map_err(|_| format!("{shown} doesn't exist."))?.len();
                        if len > 8 << 20 {
                            return Err(format!("{shown} is {len} bytes, too big to read."));
                        }
                        std::fs::read(&f).map_err(|e| e.to_string())?
                    }
                };
                if !is_text(&bytes) {
                    return Err(format!("{shown} is a binary file ({} bytes), not text.", bytes.len()));
                }
                Ok(window(&String::from_utf8_lossy(&bytes), start, count))
            }
            "search_files" => {
                let (t, shown) = area.resolve(arg(input, "path")?, false)?;
                let words = arg(input, "text")?.to_lowercase();
                if words.trim().is_empty() {
                    return Err("Say what to search for.".into());
                }
                let mut hits = Vec::new();
                let zipped = match &t {
                    Target::Inside(f, prefix) => Some((f.clone(), prefix.clone())),
                    Target::File(f) if f.is_file() => Some((f.clone(), String::new())),
                    _ => None,
                };
                if let Some((f, prefix)) = zipped {
                    let mut z = open_zip(&f)?;
                    for i in 0..z.len() {
                        let Ok(mut e) = z.by_index(i) else { continue };
                        if e.is_dir() || e.size() > 4 << 20 || !e.name().starts_with(prefix.trim_start_matches('/')) {
                            continue;
                        }
                        let name = e.name().to_string();
                        let mut b = Vec::new();
                        if (&mut e).take(4 << 20).read_to_end(&mut b).is_ok() && is_text(&b) {
                            search_text(&format!("{}!/{name}", shown.split("!/").next().unwrap_or(&shown)), &String::from_utf8_lossy(&b), &words, &mut hits);
                        }
                    }
                } else if let Target::Dir(d) = t {
                    let mut seen = 0;
                    for e in walkdir::WalkDir::new(&d).follow_links(false).into_iter().flatten() {
                        if cancel.load(Ordering::SeqCst) || hits.len() >= 120 || seen > 6000 {
                            break;
                        }
                        if !e.file_type().is_file() || e.metadata().map(|m| m.len()).unwrap_or(0) > 2 << 20 {
                            continue;
                        }
                        seen += 1;
                        if let Ok(b) = std::fs::read(e.path()) {
                            if is_text(&b) {
                                let rel = e.path().strip_prefix(&d).unwrap_or(e.path()).to_string_lossy().replace('\\', "/");
                                search_text(&format!("{}/{rel}", shown.trim_end_matches('/')), &String::from_utf8_lossy(&b), &words, &mut hits);
                            }
                        }
                    }
                } else {
                    return Err(format!("{shown} doesn't exist."));
                }
                Ok(if hits.is_empty() { "No matches.".into() } else { hits.join("\n") })
            }
            "write_file" => {
                let (t, shown) = area.resolve(arg(input, "path")?, true)?;
                let content = arg(input, "content")?;
                let Target::File(f) = t else { return Err(format!("{shown} is a folder.")) };
                if blocked_ext(&shown) {
                    return Err("Programs and scripts that Windows runs can't be part of a mod made here.".into());
                }
                if content.len() > MAX_FILE {
                    return Err("That file is over 1 MB: split it.".into());
                }
                let (n, total) = self.mod_size(area);
                if (!f.exists() && n >= MAX_FILES) || total + content.len() as u64 > MAX_TOTAL {
                    return Err("The mod is too big (300 files or 20 MB at most).".into());
                }
                std::fs::create_dir_all(f.parent().unwrap_or(&area.modd)).map_err(|e| e.to_string())?;
                std::fs::write(&f, content).map_err(|e| e.to_string())?;
                self.changed(id);
                Ok(format!("Wrote {shown} ({} lines).", content.lines().count()))
            }
            "edit_file" => {
                let (t, shown) = area.resolve(arg(input, "path")?, true)?;
                let (old, new) = (arg(input, "old_text")?, arg(input, "new_text")?);
                let Target::File(f) = t else { return Err(format!("{shown} is a folder.")) };
                let text = std::fs::read_to_string(&f).map_err(|_| format!("{shown} doesn't exist or isn't text."))?;
                let n = if old.is_empty() { 0 } else { text.matches(old).count() };
                if n != 1 {
                    return Err(format!("old_text appears {n} times in {shown}; it must appear exactly once (copy it exactly, with more lines around it)."));
                }
                let next = text.replacen(old, new, 1);
                if next.len() > MAX_FILE {
                    return Err("That would make the file over 1 MB.".into());
                }
                std::fs::write(&f, next).map_err(|e| e.to_string())?;
                self.changed(id);
                Ok(format!("Changed {shown}."))
            }
            "delete_file" => {
                let (t, shown) = area.resolve(arg(input, "path")?, true)?;
                match t {
                    Target::File(f) if f.is_file() => {
                        std::fs::remove_file(&f).map_err(|e| e.to_string())?;
                        self.changed(id);
                        Ok(format!("Deleted {shown}."))
                    }
                    _ => Err(format!("{shown} isn't one of the mod's files.")),
                }
            }
            "build" => {
                let p = self.load(id)?;
                if !p.kind.compiled() {
                    return Err("This mod isn't built: it's loaded as it is.".into());
                }
                let proj = check_project(&area.modd)?;
                if !self.ask(id, "Claude wants to build the mod with the .NET SDK (dotnet build). It compiles only the mod's own files, and the project has passed OmniHub's safety checks.", cancel) {
                    return Err("The user didn't approve the build. Ask what they'd like, or explain why the build is needed.".into());
                }
                let out = self.pdir(id)?.join("build");
                let _ = std::fs::remove_dir_all(&out);
                self.log(id, item("tool", "Building with dotnet…"));
                let b = self.builder.lock().clone();
                let r = b(&proj, &out);
                self.log(id, Item { tone: Some(if r.is_ok() { "good" } else { "bad" }.into()), ..item("notice", if r.is_ok() { "The build worked." } else { "The build failed; Claude is looking at the errors." }) });
                r.map(|log| format!("Build succeeded.\n{log}"))
            }
            "finish" => {
                let summary = arg(input, "summary")?.trim().to_string();
                let how = input["how_to_use"].as_str().unwrap_or_default().trim().to_string();
                let name = input["name"].as_str().map(|s| s.trim().chars().take(60).collect::<String>()).filter(|s| !s.is_empty());
                let (n, _) = self.mod_size(area);
                if n == 0 {
                    return Err("The mod has no files yet.".into());
                }
                let p = self.load(id)?;
                if p.kind.compiled() && !self.pdir(id)?.join("build").is_dir() {
                    return Err("Build it first (the build tool), and fix any errors.".into());
                }
                self.set(id, |p| {
                    p.ready = true;
                    p.version += 1;
                    p.summary = summary.chars().take(1200).collect();
                    p.how_to_use = how.chars().take(1200).collect();
                    if let Some(n) = name {
                        p.name = n;
                    }
                })?;
                Ok("Marked ready. The user can now press Install (or Update, if an older version is installed).".into())
            }
            other => Err(format!("There's no tool called {other}.")),
        }
    }

    /// Install (or update) the project's mod through the mod manager.
    pub fn install(&self, hub: &Arc<ModHub>, id: &str) -> Result<Project, String> {
        let p = self.load(id)?;
        if self.runs.lock().contains_key(id) {
            return Err("Wait until Claude has finished.".into());
        }
        if !p.ready {
            return Err("Claude hasn't finished this mod yet.".into());
        }
        let d = self.pdir(id)?;
        let stage = d.join("install");
        let _ = std::fs::remove_dir_all(&stage);
        std::fs::create_dir_all(&stage).map_err(|e| e.to_string())?;
        if p.kind.compiled() {
            // Only the mod's own assembly from the build.
            let proj = check_project(&d.join("mod"))?;
            let stem = proj.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            let text = std::fs::read_to_string(&proj).unwrap_or_default();
            let asm = text.split("<AssemblyName>").nth(1).and_then(|s| s.split('<').next()).map(str::trim).filter(|s| !s.is_empty()).unwrap_or(&stem).to_string();
            let dll = d.join("build").join(format!("{asm}.dll"));
            if !dll.is_file() {
                return Err("The build has no mod file yet. Ask Claude to build it again.".into());
            }
            std::fs::copy(&dll, stage.join(format!("{asm}.dll"))).map_err(|e| e.to_string())?;
        } else {
            super::install::unpack(&d.join("mod"), &stage)?;
        }
        let layout = (p.kind == MakerKind::Files).then_some(super::install::Layout::Exact);
        let r = hub.install_made(&p.game, &stage, &p.name, &p.id, &format!("{}", p.version), p.installed.as_deref(), layout);
        let _ = std::fs::remove_dir_all(&stage);
        let m = r?;
        let v = p.version;
        self.log(id, Item { tone: Some("good".into()), ..item("notice", format!("Installed version {v}. Start the game to try it; tell Claude here what happened."))});
        self.set(id, |p| {
            p.installed = Some(m.id.clone());
            p.installed_version = v;
        })
    }

    /// Forget the installed copy (after the user removed it in the mod list).
    pub fn forget_install(&self, mod_id: &str) {
        for p in self.projects(None) {
            if p.installed.as_deref() == Some(mod_id) {
                let _ = self.set(&p.id, |p| {
                    p.installed = None;
                    p.installed_version = 0;
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_reading_and_project_checks() {
        let t = tempfile::tempdir().unwrap();
        let game = t.path().join("Balatro");
        std::fs::create_dir_all(&game).unwrap();
        // A fused LÖVE game: a program with its Lua in a zip at the end.
        let mut exe = b"MZ fake program bytes ".to_vec();
        {
            let mut z = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
            z.start_file("card.lua", zip::write::SimpleFileOptions::default()).unwrap();
            std::io::Write::write_all(&mut z, b"function Card:set_ability()\n  self.ability = {}\nend\n").unwrap();
            exe.extend(z.finish().unwrap().into_inner());
        }
        std::fs::write(game.join("Balatro.exe"), &exe).unwrap();
        let area = Area { game: game.clone(), data: None, modd: t.path().join("mod") };
        std::fs::create_dir_all(&area.modd).unwrap();
        assert!(area.resolve("game/../secret", false).is_err());
        assert!(area.resolve("game/x.lua", true).unwrap_err().contains("Only files under mod/"));
        assert!(area.resolve("data/x", false).is_err());
        assert!(area.resolve("C:/Windows", false).is_err());
        let (Target::Inside(f, inner), _) = area.resolve("game/Balatro.exe!/card.lua", false).unwrap() else { panic!() };
        assert_eq!(inner, "card.lua");
        assert!(String::from_utf8(read_inside(&f, &inner).unwrap()).unwrap().contains("set_ability"));
        assert!(list_inside(&game.join("Balatro.exe"), "", "game/Balatro.exe!/").unwrap().contains("card.lua"));

        let long: String = (1..=2000).map(|i| format!("line {i}\n")).collect();
        let w = window(&long, 1, 10);
        assert!(w.starts_with("[lines 1–10 of 2000") && w.contains("line 10\n") && !w.contains("line 11\n"));
        assert_eq!(window("a\nb\n", 1, 10), "a\nb\n");
        assert!(blocked_ext("mod/x.DLL") && blocked_ext("mod/run.ps1") && !blocked_ext("mod/main.lua"));

        // C# projects: plain ones pass, build steps and odd packages don't.
        let p = t.path().join("proj");
        std::fs::create_dir_all(&p).unwrap();
        let ok = r#"<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup><TargetFramework>netstandard2.1</TargetFramework><RestoreAdditionalProjectSources>https://nuget.bepinex.dev/v3/index.json</RestoreAdditionalProjectSources></PropertyGroup><ItemGroup><PackageReference Include="BepInEx.Core" Version="5.*" /><PackageReference Include="UnityEngine.Modules" Version="2022.3.21" /></ItemGroup></Project>"#;
        std::fs::write(p.join("Mod.csproj"), ok).unwrap();
        assert!(check_project(&p).is_ok());
        for bad in [
            ok.replace("</Project>", "<Target Name=\"X\"><Exec Command=\"calc\" /></Target></Project>"),
            ok.replace("BepInEx.Core", "Evil.Package"),
            ok.replace("https://nuget.bepinex.dev/v3/index.json", "https://evil.example/v3/index.json"),
            ok.replace("</Project>", "<Import Project=\"x.targets\" /></Project>"),
        ] {
            std::fs::write(p.join("Mod.csproj"), &bad).unwrap();
            assert!(check_project(&p).is_err(), "{bad}");
        }
        std::fs::write(p.join("Mod.csproj"), ok).unwrap();
        std::fs::write(p.join("Directory.Build.props"), "<Project/>").unwrap();
        assert!(check_project(&p).is_err());
    }
}
