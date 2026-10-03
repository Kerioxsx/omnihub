//! Cleanup suggestions computed from a finished scan.
//!
//! Nothing here deletes anything on its own: rules only *suggest* items and
//! say how risky removing them is. Deleting is a separate, explicit call
//! ([`delete_paths`]) that refuses protected system locations, and by
//! default sends things to the Recycle Bin.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::tree::{extension_of, ScanTree, NODE_CLOUD, NODE_SYSTEM};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum Category {
    Temp,
    Cache,
    CrashDumps,
    Downloads,
    LargeOld,
    RecycleBin,
    Developer,
    System,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum Risk {
    /// Regenerated automatically; safe to remove.
    Safe,
    /// Probably unneeded, but look before deleting.
    Review,
    /// Shown for information; handled by Windows tools, not by OmniHub.
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Suggestion {
    pub id: String,
    pub category: Category,
    pub title: String,
    pub description: String,
    pub risk: Risk,
    pub size: u64,
    pub files: u64,
    /// Largest items, for display (at most 50).
    pub items: Vec<SuggestionItem>,
    /// Every path the suggestion covers (what "clean" would delete).
    pub paths: Vec<String>,
    /// Requires administrator rights to remove.
    pub needs_admin: bool,
    /// A Windows tool that handles this instead (e.g. `cleanmgr`).
    pub action_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestionItem {
    pub id: u32,
    pub path: String,
    pub size: u64,
    pub modified: u32,
    pub is_dir: bool,
}

/// Folder rules: a set of path patterns whose *contents* are suggested.
struct FolderRule {
    id: &'static str,
    category: Category,
    risk: Risk,
    title: &'static str,
    description: &'static str,
    patterns: &'static [&'static str],
    /// Only items not modified for this many days (avoids files in use).
    older_than_days: Option<u32>,
    needs_admin: bool,
    action_hint: Option<&'static str>,
    /// Suggest the matched path itself instead of its contents.
    whole: bool,
}

#[cfg(windows)]
const FOLDER_RULES: &[FolderRule] = &[
    FolderRule {
        id: "user-temp",
        category: Category::Temp,
        risk: Risk::Safe,
        title: "Temporary files",
        description: "Files programs left in your Temp folder. Anything untouched for a day is safe to remove.",
        patterns: &["{TEMP}"],
        older_than_days: Some(1),
        needs_admin: false,
        action_hint: None,
        whole: false,
    },
    FolderRule {
        id: "windows-temp",
        category: Category::Temp,
        risk: Risk::Safe,
        title: "Windows temporary files",
        description: "The system-wide Temp folder.",
        patterns: &["{WINDIR}\\Temp"],
        older_than_days: Some(1),
        needs_admin: true,
        action_hint: None,
        whole: false,
    },
    FolderRule {
        id: "browser-cache",
        category: Category::Cache,
        risk: Risk::Safe,
        title: "Browser caches",
        description: "Cached web pages, scripts and images. Browsers download them again when needed. Close the browser first.",
        patterns: &[
            "{LOCALAPPDATA}\\Google\\Chrome\\User Data\\*\\Cache",
            "{LOCALAPPDATA}\\Google\\Chrome\\User Data\\*\\Code Cache",
            "{LOCALAPPDATA}\\Google\\Chrome\\User Data\\*\\GPUCache",
            "{LOCALAPPDATA}\\Microsoft\\Edge\\User Data\\*\\Cache",
            "{LOCALAPPDATA}\\Microsoft\\Edge\\User Data\\*\\Code Cache",
            "{LOCALAPPDATA}\\Microsoft\\Edge\\User Data\\*\\GPUCache",
            "{LOCALAPPDATA}\\BraveSoftware\\Brave-Browser\\User Data\\*\\Cache",
            "{LOCALAPPDATA}\\BraveSoftware\\Brave-Browser\\User Data\\*\\Code Cache",
            "{LOCALAPPDATA}\\Vivaldi\\User Data\\*\\Cache",
            "{APPDATA}\\Opera Software\\Opera Stable\\Cache",
            "{LOCALAPPDATA}\\Opera Software\\Opera Stable\\Cache",
            "{LOCALAPPDATA}\\Mozilla\\Firefox\\Profiles\\*\\cache2",
        ],
        older_than_days: None,
        needs_admin: false,
        action_hint: None,
        whole: false,
    },
    FolderRule {
        id: "shader-cache",
        category: Category::Cache,
        risk: Risk::Safe,
        title: "Graphics shader caches",
        description: "Compiled shaders from your GPU driver and games. They are rebuilt the next time a game runs (the first launch may stutter briefly).",
        patterns: &[
            "{LOCALAPPDATA}\\D3DSCache",
            "{LOCALAPPDATA}\\NVIDIA\\DXCache",
            "{LOCALAPPDATA}\\NVIDIA\\GLCache",
            "{LOCALAPPDATA}\\AMD\\DxCache",
            "{LOCALAPPDATA}\\AMD\\DxcCache",
            "{LOCALAPPDATA}\\AMD\\VkCache",
            "{LOCALAPPDATA}\\Intel\\ShaderCache",
        ],
        older_than_days: None,
        needs_admin: false,
        action_hint: None,
        whole: false,
    },
    FolderRule {
        id: "thumbnails",
        category: Category::Cache,
        risk: Risk::Review,
        title: "Thumbnail cache",
        description: "Explorer's picture previews. Some files stay locked while Explorer is running.",
        patterns: &["{LOCALAPPDATA}\\Microsoft\\Windows\\Explorer\\thumbcache_*.db", "{LOCALAPPDATA}\\Microsoft\\Windows\\Explorer\\iconcache_*.db"],
        older_than_days: None,
        needs_admin: false,
        action_hint: None,
        whole: true,
    },
    FolderRule {
        id: "crash-dumps",
        category: Category::CrashDumps,
        risk: Risk::Safe,
        title: "Crash dumps and error reports",
        description: "Memory dumps and reports written when programs or Windows crashed. Only useful to send to a developer.",
        patterns: &[
            "{LOCALAPPDATA}\\CrashDumps",
            "{LOCALAPPDATA}\\Microsoft\\Windows\\WER\\ReportArchive",
            "{LOCALAPPDATA}\\Microsoft\\Windows\\WER\\ReportQueue",
            "{PROGRAMDATA}\\Microsoft\\Windows\\WER\\ReportArchive",
            "{PROGRAMDATA}\\Microsoft\\Windows\\WER\\ReportQueue",
            "{WINDIR}\\Minidump",
            "{WINDIR}\\LiveKernelReports",
        ],
        older_than_days: None,
        needs_admin: false,
        action_hint: None,
        whole: false,
    },
    FolderRule {
        id: "memory-dump",
        category: Category::CrashDumps,
        risk: Risk::Safe,
        title: "Full memory dump",
        description: "The last complete memory dump Windows wrote after a blue screen.",
        patterns: &["{WINDIR}\\MEMORY.DMP"],
        older_than_days: None,
        needs_admin: true,
        action_hint: None,
        whole: true,
    },
    FolderRule {
        id: "windows-update",
        category: Category::System,
        risk: Risk::Review,
        title: "Windows Update downloads",
        description: "Update packages that were already installed. Windows downloads them again if it still needs them.",
        patterns: &["{WINDIR}\\SoftwareDistribution\\Download"],
        older_than_days: Some(7),
        needs_admin: true,
        action_hint: Some("cleanmgr /sageset"),
        whole: false,
    },
    FolderRule {
        id: "delivery-optimization",
        category: Category::System,
        risk: Risk::Safe,
        title: "Delivery Optimization cache",
        description: "Update pieces Windows keeps to share with other PCs.",
        patterns: &["{WINDIR}\\ServiceProfiles\\NetworkService\\AppData\\Local\\Microsoft\\Windows\\DeliveryOptimization\\Cache"],
        older_than_days: None,
        needs_admin: true,
        action_hint: None,
        whole: false,
    },
    FolderRule {
        id: "dev-caches",
        category: Category::Developer,
        risk: Risk::Review,
        title: "Developer package caches",
        description: "Downloaded packages kept by npm, pip, Yarn, Cargo, Gradle and Go. Builds download them again when needed.",
        patterns: &[
            "{LOCALAPPDATA}\\npm-cache",
            "{APPDATA}\\npm-cache",
            "{LOCALAPPDATA}\\pip\\cache",
            "{LOCALAPPDATA}\\Yarn\\Cache",
            "{USERPROFILE}\\.cargo\\registry\\cache",
            "{USERPROFILE}\\.gradle\\caches",
            "{LOCALAPPDATA}\\go-build",
            "{LOCALAPPDATA}\\NuGet\\v3-cache",
        ],
        older_than_days: None,
        needs_admin: false,
        action_hint: None,
        whole: false,
    },
    FolderRule {
        id: "windows-old",
        category: Category::System,
        risk: Risk::Info,
        title: "Previous Windows installation",
        description: "Windows.old lets you roll back a feature update. Remove it with Disk Cleanup → Clean up system files → Previous Windows installation(s).",
        patterns: &["{SYSTEMDRIVE}\\Windows.old"],
        older_than_days: None,
        needs_admin: true,
        action_hint: Some("cleanmgr"),
        whole: true,
    },
    FolderRule {
        id: "hibernation",
        category: Category::System,
        risk: Risk::Info,
        title: "Hibernation file",
        description: "Reserved for hibernate and Fast Startup. Running `powercfg /h off` as administrator removes it (and disables both).",
        patterns: &["{SYSTEMDRIVE}\\hiberfil.sys"],
        older_than_days: None,
        needs_admin: true,
        action_hint: Some("powercfg /h off"),
        whole: true,
    },
    FolderRule {
        id: "pagefile",
        category: Category::System,
        risk: Risk::Info,
        title: "Page file",
        description: "Virtual memory managed by Windows. Adjust it in System → Advanced system settings → Performance → Virtual memory.",
        patterns: &["{SYSTEMDRIVE}\\pagefile.sys", "{SYSTEMDRIVE}\\swapfile.sys"],
        older_than_days: None,
        needs_admin: true,
        action_hint: Some("SystemPropertiesPerformance"),
        whole: true,
    },
];

#[cfg(not(windows))]
const FOLDER_RULES: &[FolderRule] = &[
    FolderRule {
        id: "user-cache",
        category: Category::Cache,
        risk: Risk::Review,
        title: "Application caches",
        description: "Files applications cache under ~/.cache.",
        patterns: &["{HOME}/.cache/*"],
        older_than_days: None,
        needs_admin: false,
        action_hint: None,
        whole: true,
    },
    FolderRule {
        id: "user-temp",
        category: Category::Temp,
        risk: Risk::Safe,
        title: "Temporary files",
        description: "Files in the temporary folder untouched for a day.",
        patterns: &["{TEMP}"],
        older_than_days: Some(1),
        needs_admin: false,
        action_hint: None,
        whole: false,
    },
];

/// Values for the `{TOKEN}`s used in rule patterns.
pub type Tokens = HashMap<&'static str, String>;

pub fn default_tokens() -> Tokens {
    let mut t = Tokens::new();
    let env = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
    #[cfg(windows)]
    {
        for (token, var) in [
            ("TEMP", "TEMP"),
            ("LOCALAPPDATA", "LOCALAPPDATA"),
            ("APPDATA", "APPDATA"),
            ("USERPROFILE", "USERPROFILE"),
            ("WINDIR", "WINDIR"),
            ("PROGRAMDATA", "PROGRAMDATA"),
            ("SYSTEMDRIVE", "SYSTEMDRIVE"),
            ("PROGRAMFILES", "ProgramFiles"),
            ("PROGRAMFILESX86", "ProgramFiles(x86)"),
        ] {
            if let Some(v) = env(var) {
                // %TEMP% is often the 8.3 form (C:\Users\JOHNSM~1\...).
                let v = std::fs::canonicalize(&v)
                    .map(|p| p.to_string_lossy().trim_start_matches(r"\\?\").to_string())
                    .unwrap_or(v);
                t.insert(token, v);
            }
        }
    }
    #[cfg(not(windows))]
    {
        if let Some(h) = env("HOME") {
            t.insert("HOME", h);
        }
        t.insert("TEMP", std::env::temp_dir().to_string_lossy().trim_end_matches('/').to_string());
    }
    t
}

fn expand(pattern: &str, tokens: &Tokens) -> Option<String> {
    let mut out = pattern.to_string();
    while let Some(start) = out.find('{') {
        let end = out[start..].find('}')? + start;
        let key = &out[start + 1..end];
        let value = tokens.get(key)?;
        out.replace_range(start..=end, value);
    }
    Some(out)
}

/// Resolve a path pattern (with `*` and `?` in components) to tree nodes.
pub fn resolve_pattern(tree: &ScanTree, pattern: &str) -> Vec<u32> {
    let sep = tree.info.separator;
    let root = tree.name(0).trim_end_matches(['\\', '/']).to_lowercase();
    let norm = pattern.replace(['\\', '/'], &sep.to_string()).to_lowercase();
    let Some(rest) = norm.strip_prefix(&root) else { return Vec::new() };
    let comps: Vec<&str> = rest.split(sep).filter(|c| !c.is_empty()).collect();
    let mut frontier = vec![0u32];
    for comp in comps {
        let matcher = super::tree::NameMatcher::new(comp);
        let glob = comp.contains(['*', '?']);
        let mut next = Vec::new();
        for &node in &frontier {
            for child in tree.child_ids(node) {
                let name = tree.name(child);
                let hit = if glob { matcher.matches(name) } else { name.to_lowercase() == comp };
                if hit {
                    next.push(child);
                }
            }
        }
        if next.is_empty() {
            return next;
        }
        frontier = next;
    }
    if frontier == [0] {
        Vec::new()
    } else {
        frontier
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct CleanupOptions {
    pub large_file_min: u64,
    pub large_file_age_days: u32,
    pub installer_age_days: u32,
}

impl Default for CleanupOptions {
    fn default() -> Self {
        CleanupOptions { large_file_min: 1 << 30, large_file_age_days: 180, installer_age_days: 30 }
    }
}

pub fn suggest(tree: &ScanTree, tokens: &Tokens, opts: &CleanupOptions, now: u32) -> Vec<Suggestion> {
    let mut out = Vec::new();
    for rule in FOLDER_RULES {
        let mut targets: Vec<u32> = Vec::new();
        for pat in rule.patterns {
            let Some(p) = expand(pat, tokens) else { continue };
            for id in resolve_pattern(tree, &p) {
                if rule.whole {
                    targets.push(id);
                } else {
                    targets.extend(tree.child_ids(id));
                }
            }
        }
        if let Some(days) = rule.older_than_days {
            let cutoff = now.saturating_sub(days * 86_400);
            targets.retain(|&id| tree.node(id).is_some_and(|n| n.modified <= cutoff));
        }
        targets.sort_unstable();
        targets.dedup();
        if let Some(s) = build(tree, rule.id, rule.category, rule.risk, rule.title, rule.description, targets, rule.needs_admin, rule.action_hint) {
            out.push(s);
        }
    }

    if let Some(s) = recycle_bin(tree, tokens) {
        out.push(s);
    }
    if let Some(s) = old_installers(tree, tokens, opts, now) {
        out.push(s);
    }
    if let Some(s) = large_old_files(tree, tokens, opts, now) {
        out.push(s);
    }
    out.sort_by(|a, b| a.risk.cmp(&b.risk).then(b.size.cmp(&a.size)));
    out
}

#[allow(clippy::too_many_arguments)]
fn build(
    tree: &ScanTree,
    id: &str,
    category: Category,
    risk: Risk,
    title: &str,
    description: &str,
    mut targets: Vec<u32>,
    needs_admin: bool,
    action_hint: Option<&str>,
) -> Option<Suggestion> {
    // Drop targets nested inside other targets so sizes are not counted twice.
    targets.sort_unstable();
    let mut kept: Vec<u32> = Vec::with_capacity(targets.len());
    for t in targets {
        if let Some(&last) = kept.last() {
            if tree.subtree(last).contains(&(t as usize)) {
                continue;
            }
        }
        kept.push(t);
    }
    let (mut size, mut files) = (0u64, 0u64);
    for &t in &kept {
        let n = tree.node(t)?;
        size += n.size;
        files += n.files as u64;
    }
    if size == 0 {
        return None;
    }
    kept.sort_by_key(|&i| std::cmp::Reverse(tree.node(i).unwrap().size));
    let items = kept
        .iter()
        .take(50)
        .map(|&i| {
            let n = tree.node(i).unwrap();
            SuggestionItem { id: i, path: tree.path(i), size: n.size, modified: n.modified, is_dir: n.is_dir() }
        })
        .collect();
    Some(Suggestion {
        id: id.to_string(),
        category,
        title: title.to_string(),
        description: description.to_string(),
        risk,
        size,
        files,
        items,
        paths: kept.iter().filter(|&&i| tree.path_is_exact(i)).map(|&i| tree.path(i)).collect(),
        needs_admin,
        action_hint: action_hint.map(str::to_string),
    })
}

fn recycle_bin(tree: &ScanTree, tokens: &Tokens) -> Option<Suggestion> {
    #[cfg(windows)]
    let pattern = format!("{}\\$Recycle.Bin", tokens.get("SYSTEMDRIVE")?);
    #[cfg(not(windows))]
    let pattern = format!("{}/.local/share/Trash", tokens.get("HOME")?);
    let ids = resolve_pattern(tree, &pattern);
    let mut s = build(
        tree,
        "recycle-bin",
        Category::RecycleBin,
        Risk::Safe,
        "Recycle Bin",
        "Files you already deleted. Emptying the Recycle Bin removes them for good.",
        ids,
        false,
        Some("empty-recycle-bin"),
    )?;
    // The bin is emptied through the shell, not by deleting its folders.
    s.paths.clear();
    Some(s)
}

const INSTALLER_EXTS: &[&str] = &["exe", "msi", "msix", "appx", "iso", "img", "zip", "7z", "rar", "dmg", "apk"];

fn old_installers(tree: &ScanTree, tokens: &Tokens, opts: &CleanupOptions, now: u32) -> Option<Suggestion> {
    #[cfg(windows)]
    let pattern = format!("{}\\Downloads", tokens.get("USERPROFILE")?);
    #[cfg(not(windows))]
    let pattern = format!("{}/Downloads", tokens.get("HOME")?);
    let dl = *resolve_pattern(tree, &pattern).first()?;
    let cutoff = now.saturating_sub(opts.installer_age_days * 86_400);
    let ids: Vec<u32> = tree
        .files_under(dl)
        .filter(|&i| {
            let n = tree.node(i).unwrap();
            n.size >= 10 << 20
                && n.modified <= cutoff
                && n.flags & NODE_CLOUD == 0
                && extension_of(tree.name(i)).is_some_and(|e| INSTALLER_EXTS.contains(&e.as_str()))
        })
        .collect();
    build(
        tree,
        "old-downloads",
        Category::Downloads,
        Risk::Review,
        "Old installers and archives in Downloads",
        "Setup programs, disk images and archives you downloaded more than a month ago. Usually already installed or extracted.",
        ids,
        false,
        None,
    )
}

fn large_old_files(tree: &ScanTree, tokens: &Tokens, opts: &CleanupOptions, now: u32) -> Option<Suggestion> {
    let cutoff = now.saturating_sub(opts.large_file_age_days * 86_400);
    let protected: Vec<u32> = protected_roots(tokens).iter().flat_map(|p| resolve_pattern(tree, p)).collect();
    let mut ids: Vec<u32> = tree
        .files_under(0)
        .filter(|&i| {
            let n = tree.node(i).unwrap();
            n.size >= opts.large_file_min
                && n.modified > 0
                && n.modified <= cutoff
                && n.flags & (NODE_SYSTEM | NODE_CLOUD) == 0
                && !tree.name(i).starts_with('$')
                && tree.node(n.parent).is_some_and(|p| p.parent != super::tree::NO_PARENT || !is_root_system_file(tree.name(i)))
        })
        .filter(|&i| !protected.iter().any(|&p| tree.subtree(p).contains(&(i as usize))))
        .collect();
    ids.sort_by_key(|&i| std::cmp::Reverse(tree.node(i).unwrap().size));
    ids.truncate(200);
    let days = opts.large_file_age_days;
    let mut s = build(
        tree,
        "large-old",
        Category::LargeOld,
        Risk::Review,
        "Large files you have not touched in a while",
        "Big files not modified for a long time. Move them to an external drive or delete what you no longer need.",
        ids,
        false,
        None,
    )?;
    s.description = format!("Files over {} not modified for {days} days. Move them to an external drive or delete what you no longer need.", human_size(opts.large_file_min));
    Some(s)
}

fn is_root_system_file(name: &str) -> bool {
    matches!(name.to_lowercase().as_str(), "pagefile.sys" | "hiberfil.sys" | "swapfile.sys" | "dumpstack.log.tmp")
}

fn protected_roots(tokens: &Tokens) -> Vec<String> {
    let mut v = Vec::new();
    #[cfg(windows)]
    {
        for t in ["WINDIR", "PROGRAMFILES", "PROGRAMFILESX86"] {
            if let Some(p) = tokens.get(t) {
                v.push(p.clone());
            }
        }
        if let Some(p) = tokens.get("PROGRAMDATA") {
            v.push(format!("{p}\\Microsoft"));
        }
        if let Some(d) = tokens.get("SYSTEMDRIVE") {
            v.push(format!("{d}\\System Volume Information"));
            v.push(format!("{d}\\Recovery"));
            v.push(format!("{d}\\$WinREAgent"));
        }
    }
    #[cfg(not(windows))]
    {
        let _ = tokens;
        for p in ["/usr", "/bin", "/sbin", "/lib", "/lib64", "/etc", "/boot", "/var/lib", "/proc", "/sys"] {
            v.push(p.to_string());
        }
    }
    v
}

pub fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];
    let mut v = bytes as f64;
    let mut u = 0;
    while v >= 1024.0 && u < UNITS.len() - 1 {
        v /= 1024.0;
        u += 1;
    }
    if u == 0 {
        format!("{bytes} B")
    } else {
        format!("{v:.1} {}", UNITS[u])
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeleteResult {
    pub path: String,
    pub ok: bool,
    pub error: Option<String>,
}

/// Why a path may never be deleted from OmniHub, if it may not.
pub fn refusal_reason(path: &Path, tokens: &Tokens) -> Option<&'static str> {
    let s = path.to_string_lossy().replace('/', std::path::MAIN_SEPARATOR_STR);
    let lower = s.trim_end_matches(['\\', '/']).to_lowercase();
    if !path.is_absolute() {
        return Some("path must be absolute");
    }
    if path.parent().is_none() || lower.len() <= 3 {
        return Some("refusing to delete a drive root");
    }
    if s.contains("..") {
        return Some("relative components are not allowed");
    }
    let under = |base: &str| {
        let b = base.trim_end_matches(['\\', '/']).to_lowercase();
        lower == b || lower.starts_with(&format!("{b}{}", std::path::MAIN_SEPARATOR))
    };
    let equals = |base: &str| lower == base.trim_end_matches(['\\', '/']).to_lowercase();
    #[cfg(windows)]
    {
        if let Some(home) = tokens.get("USERPROFILE") {
            if equals(home) {
                return Some("refusing to delete your user folder");
            }
        }
        // Explicitly allowed system locations (cleanup rules point here).
        let allowed: Vec<String> = [
            ("WINDIR", "\\Temp"),
            ("WINDIR", "\\SoftwareDistribution\\Download"),
            ("WINDIR", "\\Minidump"),
            ("WINDIR", "\\LiveKernelReports"),
            ("WINDIR", "\\MEMORY.DMP"),
            ("WINDIR", "\\ServiceProfiles\\NetworkService\\AppData\\Local\\Microsoft\\Windows\\DeliveryOptimization\\Cache"),
            ("PROGRAMDATA", "\\Microsoft\\Windows\\WER\\ReportArchive"),
            ("PROGRAMDATA", "\\Microsoft\\Windows\\WER\\ReportQueue"),
        ]
        .iter()
        .filter_map(|(t, suffix)| tokens.get(*t).map(|b| format!("{b}{suffix}")))
        .collect();
        if allowed.iter().any(|a| under(a) && !equals(a)) {
            return None;
        }
        for (t, why) in [
            ("WINDIR", "Windows system files are protected"),
            ("PROGRAMFILES", "installed programs are protected; uninstall them instead"),
            ("PROGRAMFILESX86", "installed programs are protected; uninstall them instead"),
        ] {
            if tokens.get(t).is_some_and(|b| under(b)) {
                return Some(why);
            }
        }
        if let Some(pd) = tokens.get("PROGRAMDATA") {
            if under(&format!("{pd}\\Microsoft")) {
                return Some("Windows data is protected");
            }
        }
        let name = path.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
        let at_root = path.parent().is_some_and(|p| p.parent().is_none());
        if at_root && (name.starts_with('$') || is_root_system_file(&name) || name == "system volume information" || name == "recovery" || name == "boot" || name == "bootmgr") {
            return Some("system file at the drive root");
        }
    }
    #[cfg(not(windows))]
    {
        if let Some(home) = tokens.get("HOME") {
            if equals(home) {
                return Some("refusing to delete your home folder");
            }
        }
        for p in protected_roots(tokens) {
            if under(&p) {
                return Some("system location is protected");
            }
        }
    }
    None
}

/// Delete paths, to the Recycle Bin unless `permanent`.
pub fn delete_paths(paths: &[String], permanent: bool, tokens: &Tokens) -> Vec<DeleteResult> {
    let mut results = Vec::with_capacity(paths.len());
    let mut to_trash: Vec<PathBuf> = Vec::new();
    for p in paths {
        let path = PathBuf::from(p);
        if let Some(why) = refusal_reason(&path, tokens) {
            results.push(DeleteResult { path: p.clone(), ok: false, error: Some(why.to_string()) });
            continue;
        }
        let meta = match std::fs::symlink_metadata(&path) {
            Ok(m) => m,
            Err(e) => {
                let gone = e.kind() == std::io::ErrorKind::NotFound;
                results.push(DeleteResult { path: p.clone(), ok: gone, error: (!gone).then(|| e.to_string()) });
                continue;
            }
        };
        if permanent {
            let r = if meta.is_dir() && !meta.file_type().is_symlink() {
                std::fs::remove_dir_all(&path)
            } else {
                std::fs::remove_file(&path).or_else(|e| {
                    // Directory symlinks / junctions on Windows.
                    if meta.is_dir() { std::fs::remove_dir(&path) } else { Err(e) }
                })
            };
            results.push(DeleteResult { path: p.clone(), ok: r.is_ok(), error: r.err().map(|e| e.to_string()) });
        } else {
            to_trash.push(path);
        }
    }
    // Trash one by one so a locked file does not fail the whole batch.
    for path in to_trash {
        let r = trash::delete(&path);
        results.push(DeleteResult {
            path: path.to_string_lossy().to_string(),
            ok: r.is_ok(),
            error: r.err().map(|e| e.to_string()),
        });
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::tree::{ScanInfo, ScanMethod, TreeBuilder, NO_PARENT};

    fn tree(sep: char, root: &str) -> ScanTree {
        let mut b = TreeBuilder::with_capacity(16);
        let r = b.add(NO_PARENT, root, true, 0, 0, 0, 0);
        let home = b.add(r, "home", true, 0, 0, 0, 0);
        let me = b.add(home, "me", true, 0, 0, 0, 0);
        let cache = b.add(me, ".cache", true, 0, 0, 0, 0);
        let c1 = b.add(cache, "browser", true, 0, 0, 0, 0);
        b.add(c1, "blob1", false, 1000, 4096, 100, 0);
        b.add(cache, "pip", false, 500, 4096, 100, 0);
        let dl = b.add(me, "Downloads", true, 0, 0, 0, 0);
        b.add(dl, "setup.EXE", false, 50 << 20, 50 << 20, 1_000, 0);
        b.add(dl, "fresh.msi", false, 50 << 20, 50 << 20, 99_999_000, 0);
        b.add(dl, "photo.jpg", false, 50 << 20, 50 << 20, 1_000, 0);
        b.add(me, "old-backup.vhdx", false, 3 << 30, 3 << 30, 1_000, 0);
        let usr = b.add(r, "usr", true, 0, 0, 0, 0);
        b.add(usr, "huge-system.img", false, 4 << 30, 4 << 30, 1_000, 0);
        b.finish(r, ScanInfo {
            root_path: root.into(),
            method: ScanMethod::Walk,
            started_at: 0,
            duration_ms: 0,
            volume_serial: None,
            volume_total: None,
            volume_free: None,
            cluster_size: None,
            errors: 0,
            from_cache: false,
            separator: sep,
        })
    }

    #[test]
    fn patterns_resolve_with_globs() {
        let t = tree('/', "/");
        assert_eq!(resolve_pattern(&t, "/home/me/.cache/*").len(), 2);
        assert_eq!(resolve_pattern(&t, "/HOME/ME/downloads").len(), 1);
        assert!(resolve_pattern(&t, "/home/nobody").is_empty());
        let mut tokens = Tokens::new();
        tokens.insert("HOME", "/home/me".into());
        assert_eq!(expand("{HOME}/x", &tokens).as_deref(), Some("/home/me/x"));
        assert_eq!(expand("{NOPE}/x", &tokens), None);
    }

    #[cfg(not(windows))]
    #[test]
    fn suggestions_on_unix_layout() {
        let t = tree('/', "/");
        let mut tokens = Tokens::new();
        tokens.insert("HOME", "/home/me".into());
        tokens.insert("TEMP", "/tmp".into());
        let s = suggest(&t, &tokens, &CleanupOptions::default(), 100_000_000);
        let cache = s.iter().find(|s| s.id == "user-cache").expect("cache suggestion");
        assert_eq!(cache.size, 1500);
        assert_eq!(cache.paths.len(), 2);
        let dl = s.iter().find(|s| s.id == "old-downloads").expect("downloads suggestion");
        assert_eq!(dl.files, 1, "only the old installer, not the fresh one or the photo");
        let large = s.iter().find(|s| s.id == "large-old").expect("large files");
        assert_eq!(large.items.len(), 1, "system locations are excluded");
        assert!(large.items[0].path.ends_with("old-backup.vhdx"));
    }

    #[cfg(not(windows))]
    #[test]
    fn refuses_protected_paths() {
        let mut tokens = Tokens::new();
        tokens.insert("HOME", "/home/me".into());
        assert!(refusal_reason(Path::new("/"), &tokens).is_some());
        assert!(refusal_reason(Path::new("/home/me"), &tokens).is_some());
        assert!(refusal_reason(Path::new("/usr/lib/x"), &tokens).is_some());
        assert!(refusal_reason(Path::new("relative/x"), &tokens).is_some());
        assert!(refusal_reason(Path::new("/home/me/../../etc"), &tokens).is_some());
        assert!(refusal_reason(Path::new("/home/me/.cache/pip"), &tokens).is_none());
    }

    #[test]
    fn permanent_delete_removes_files_and_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("f.txt");
        let d = dir.path().join("sub");
        std::fs::write(&f, b"x").unwrap();
        std::fs::create_dir_all(d.join("deep")).unwrap();
        std::fs::write(d.join("deep/g"), b"y").unwrap();
        let tokens = Tokens::new();
        let res = delete_paths(&[f.to_string_lossy().into(), d.to_string_lossy().into()], true, &tokens);
        assert!(res.iter().all(|r| r.ok), "{res:?}");
        assert!(!f.exists() && !d.exists());
    }

    #[test]
    fn sizes() {
        assert_eq!(human_size(512), "512 B");
        assert_eq!(human_size(1536), "1.5 KB");
        assert_eq!(human_size(1 << 30), "1.0 GB");
    }
}
