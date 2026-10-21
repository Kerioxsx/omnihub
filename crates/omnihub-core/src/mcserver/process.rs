//! A running server: the Java process, its console (kept, and read for
//! what's happening: started, players joining and leaving), commands typed
//! into it, and questions OmniHub asks it (where players are, what they
//! carry) with the answers picked out of the console.

use std::collections::VecDeque;
use std::io::{BufRead, Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use serde::Serialize;

/// Console lines kept per server.
const KEEP: usize = 3000;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Line {
    pub n: u64,
    pub at: i64,
    pub text: String,
    /// info, warn, error, chat, cmd (typed), omni (OmniHub's own note)
    pub level: String,
}

/// What a console line says, for the hub.
#[derive(Debug, Clone, PartialEq)]
pub enum Said {
    Started,
    Joined(String),
    /// Who logged in and from where (the address only).
    LoggedIn(String, String),
    Left(String),
    /// From `list`: who's online (name, UUID if listed) and the most allowed.
    List(Vec<(String, Option<String>)>, Option<u32>),
    Chat(String, String),
    /// A problem worth telling the user about in plain words.
    Problem(String),
    Nothing,
}

/// Remove terminal colour codes (Paper prints them even into a pipe).
pub fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        if c == '\x1b' {
            if it.peek() == Some(&'[') {
                it.next();
                for d in it.by_ref() {
                    if d.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            continue;
        }
        if c == '\r' {
            continue;
        }
        out.push(c);
    }
    out
}

/// A log line's level and message: `[12:00:00] [Server thread/INFO]: msg`,
/// `[12:00:00 INFO]: msg` (Paper) or `[20Oct2026 …] [main/INFO] [logger/]: msg` (NeoForge).
pub fn split(line: &str) -> (&'static str, &str) {
    if line.starts_with('[') {
        if let Some(i) = line.find("]: ") {
            let head = &line[..i];
            let level = if head.contains("WARN") {
                "warn"
            } else if head.contains("ERROR") || head.contains("FATAL") {
                "error"
            } else {
                "info"
            };
            let msg = &line[i + 3..];
            return (level, msg.strip_prefix("System chat: ").unwrap_or(msg));
        }
    }
    let l = line.trim_start();
    let level = if l.starts_with("at ") || l.starts_with("Caused by") || l.starts_with("Exception") || l.contains("Error:") { "error" } else { "info" };
    (level, line)
}

fn name_ok(n: &str) -> bool {
    !n.is_empty() && n.len() <= 32 && !n.contains(char::is_whitespace) && !n.contains(['<', '>', '[', ']'])
}

/// Read one console message.
pub fn read(msg: &str) -> Said {
    let m = msg.trim();
    if m.starts_with("Done (") && m.contains(")! For help") {
        return Said::Started;
    }
    if let Some(n) = m.strip_suffix(" joined the game").filter(|n| name_ok(n)) {
        return Said::Joined(n.to_string());
    }
    // "Steve[/203.0.113.5:51234] logged in with entity id 7 at (…)"
    if let Some((who, _)) = m.split_once("] logged in with entity id ") {
        if let Some((n, addr)) = who.split_once("[/").filter(|(n, _)| name_ok(n)) {
            return Said::LoggedIn(n.to_string(), super::guard::place(addr));
        }
    }
    if let Some(n) = m.strip_suffix(" left the game").filter(|n| name_ok(n)) {
        return Said::Left(n.to_string());
    }
    if let Some(rest) = m.strip_prefix("There are ") {
        // "There are 1 of a max of 20 players online: Steve (uuid)" or older "There are 1/20 players online:".
        let (counts, names) = rest.split_once(" players online:").unwrap_or((rest, ""));
        let max = counts.rsplit([' ', '/']).next().and_then(|n| n.parse().ok());
        let players = names
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| match s.split_once(" (") {
                Some((n, u)) => (n.to_string(), Some(u.trim_end_matches(')').to_string())),
                None => (s.to_string(), None),
            })
            .filter(|(n, _)| name_ok(n))
            .collect();
        return Said::List(players, max);
    }
    let chat = m.strip_prefix("[Not Secure] ").unwrap_or(m);
    if let Some(rest) = chat.strip_prefix('<') {
        if let Some((n, text)) = rest.split_once("> ") {
            if name_ok(n) {
                return Said::Chat(n.to_string(), text.to_string());
            }
        }
    }
    if m.contains("FAILED TO BIND TO PORT") {
        return Said::Problem("Another program already uses this server's port (often another server still running). Stop it, or pick another port in Settings.".into());
    }
    if m.contains("You need to agree to the EULA") {
        return Said::Problem("The server needs the Minecraft EULA accepted before it starts.".into());
    }
    if m.contains("Unsupported class file major version") {
        return Said::Problem("The mod loader can't read this Minecraft version yet (the game is newer than the loader). Pick an older Minecraft version for this server, or wait for a loader update.".into());
    }
    if m.contains("UnsupportedClassVersionError") || m.contains("has been compiled by a more recent version of the Java Runtime") {
        return Said::Problem("This server needs a newer Java than the one it was started with. Pick “OmniHub's Java” in Settings.".into());
    }
    if m.contains("Exception in server tick loop") || m.starts_with("---- Minecraft Crash Report ----") {
        return Said::Problem("The server crashed. Its crash report is in the crash-reports folder; the console shows what went wrong.".into());
    }
    if m.contains("Incompatible mods found") || m.contains("Mod resolution failed") || (m.contains("requires version") && m.contains("of mod")) {
        return Said::Problem("Some mods don't fit together or need other mods; the console says which.".into());
    }
    Said::Nothing
}

/// Someone waiting for an answer in the console.
struct Waiter {
    id: u64,
    want: Box<dyn Fn(&str) -> bool + Send>,
    end: Box<dyn Fn(&str) -> bool + Send>,
    tx: Sender<Option<String>>,
}

pub struct Proc {
    child: Mutex<Child>,
    stdin: Mutex<Option<ChildStdin>>,
    pub pid: u32,
    pub started: Instant,
    pub stopping: AtomicBool,
    exited: AtomicBool,
    console: Arc<Mutex<VecDeque<Line>>>,
    next: Arc<AtomicU64>,
    waiters: Arc<Mutex<Vec<Waiter>>>,
    /// One question at a time, so answers don't mix.
    asking: Mutex<()>,
}

/// What the process says, line by line, as it happens.
pub type OnLine = Arc<dyn Fn(&Line, &Said) + Send + Sync>;

impl Proc {
    /// Start the server; `on_line` hears every line; `on_exit` its exit code.
    pub fn spawn(mut cmd: Command, console: Arc<Mutex<VecDeque<Line>>>, next: Arc<AtomicU64>, on_line: OnLine, on_exit: Box<dyn FnOnce(Option<i32>) + Send>) -> Result<Arc<Proc>, String> {
        super::install::no_window(&mut cmd);
        cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
        let mut child = cmd.spawn().map_err(|e| if e.kind() == std::io::ErrorKind::NotFound { "Java wasn't found. Pick OmniHub's Java in the server's settings.".to_string() } else { format!("The server couldn't start: {e}") })?;
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let stdin = child.stdin.take();
        let p = Arc::new(Proc {
            pid: child.id(),
            child: Mutex::new(child),
            stdin: Mutex::new(stdin),
            started: Instant::now(),
            stopping: AtomicBool::new(false),
            exited: AtomicBool::new(false),
            console,
            next,
            waiters: Arc::default(),
            asking: Mutex::new(()),
        });
        let (tx, rx) = channel::<(bool, String)>();
        for (err, s) in [(false, stdout.map(|s| Box::new(s) as Box<dyn Read + Send>)), (true, stderr.map(|s| Box::new(s) as Box<dyn Read + Send>))] {
            let Some(s) = s else { continue };
            let tx = tx.clone();
            std::thread::Builder::new()
                .name("mc-console".into())
                .spawn(move || {
                    let mut r = std::io::BufReader::new(s);
                    let mut buf = Vec::new();
                    loop {
                        buf.clear();
                        match r.read_until(b'\n', &mut buf) {
                            Ok(0) | Err(_) => break,
                            Ok(_) => {
                                let text = strip_ansi(String::from_utf8_lossy(&buf).trim_end_matches(['\n', '\r']));
                                if tx.send((err, text)).is_err() {
                                    break;
                                }
                            }
                        }
                    }
                })
                .map_err(|e| e.to_string())?;
        }
        drop(tx);
        let me = p.clone();
        std::thread::Builder::new()
            .name("mc-server".into())
            .spawn(move || {
                me.pump(rx, &on_line);
                let code = me.child.lock().wait().ok().and_then(|s| s.code());
                me.exited.store(true, Ordering::SeqCst);
                for w in me.waiters.lock().drain(..) {
                    let _ = w.tx.send(None);
                }
                on_exit(code);
            })
            .map_err(|e| e.to_string())?;
        Ok(p)
    }

    fn pump(&self, rx: Receiver<(bool, String)>, on_line: &OnLine) {
        while let Ok((err, text)) = rx.recv() {
            if text.trim().is_empty() {
                continue;
            }
            let (level, msg) = split(&text);
            let said = read(msg);
            // OmniHub's own questions and their answers stay out of the console.
            let mut quiet = false;
            {
                let mut ws = self.waiters.lock();
                ws.retain(|w| {
                    if (w.want)(msg) {
                        quiet = true;
                        let _ = w.tx.send(Some(msg.to_string()));
                    }
                    if (w.end)(msg) {
                        quiet = true;
                        let _ = w.tx.send(None);
                        return false;
                    }
                    true
                });
            }
            let level = match &said {
                Said::Chat(..) => "chat",
                _ if err && level == "info" => "warn",
                _ => level,
            };
            let line = Line { n: self.next.fetch_add(1, Ordering::SeqCst), at: crate::db::now(), text, level: level.into() };
            if !quiet {
                let mut c = self.console.lock();
                c.push_back(line.clone());
                while c.len() > KEEP {
                    c.pop_front();
                }
            }
            on_line(&line, &said);
        }
    }

    pub fn alive(&self) -> bool {
        !self.exited.load(Ordering::SeqCst)
    }

    /// Type a command into the server's console.
    pub fn send(&self, command: &str) -> Result<(), String> {
        let c = command.trim().trim_start_matches('/');
        if c.is_empty() {
            return Ok(());
        }
        if c.contains(['\n', '\r']) {
            return Err("One command at a time.".into());
        }
        let mut g = self.stdin.lock();
        let w = g.as_mut().ok_or("The server isn't running.")?;
        w.write_all(format!("{c}\n").as_bytes()).and_then(|_| w.flush()).map_err(|_| "The server isn't running.".to_string())
    }

    /// Ask the server something: send `commands`, collect the lines `want`
    /// picks until `end` matches one (or `timeout`). The lines stay out of the console.
    pub fn ask(&self, commands: &[String], want: impl Fn(&str) -> bool + Send + 'static, end: impl Fn(&str) -> bool + Send + 'static, timeout: Duration) -> Result<Vec<String>, String> {
        let _one = self.asking.lock();
        if !self.alive() {
            return Err("The server isn't running.".into());
        }
        let (tx, rx) = channel();
        let id = self.next.fetch_add(1, Ordering::SeqCst);
        self.waiters.lock().push(Waiter { id, want: Box::new(want), end: Box::new(end), tx });
        for c in commands {
            self.send(c)?;
        }
        let deadline = Instant::now() + timeout;
        let mut out = Vec::new();
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            match rx.recv_timeout(left) {
                Ok(Some(l)) => out.push(l),
                Ok(None) => return Ok(out),
                Err(_) => {
                    // Too slow: forget this question.
                    self.waiters.lock().retain(|w| w.id != id);
                    return if out.is_empty() { Err("The server didn't answer in time.".into()) } else { Ok(out) };
                }
            }
        }
    }

    /// Ask the server to save and stop; true if it stopped within `wait`.
    pub fn stop(&self, wait: Duration) -> bool {
        self.stopping.store(true, Ordering::SeqCst);
        let _ = self.send("stop");
        let until = Instant::now() + wait;
        while Instant::now() < until {
            if !self.alive() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        false
    }

    /// End it at once (unsaved changes since its last save are lost).
    pub fn kill(&self) {
        self.stopping.store(true, Ordering::SeqCst);
        *self.stdin.lock() = None;
        let _ = self.child.lock().kill();
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    #[test]
    fn console_lines_are_understood() {
        // Real lines from vanilla 26.3, 26.1.2, Paper 26.3, Fabric and NeoForge.
        let (l, m) = split("[19:48:58] [Server thread/INFO]: Done (3.365s)! For help, type \"help\"");
        assert_eq!((l, read(m)), ("info", Said::Started));
        let (_, m) = split("[19:55:15 INFO]: Done (16.332s)! For help, type \"help\"");
        assert_eq!(read(m), Said::Started);
        let (_, m) = split("[20Oct2026 19:56:08.123] [Server thread/INFO] [net.minecraft.server.dedicated.DedicatedServer/]: Done (2.981s)! For help, type \"help\"");
        assert_eq!(read(m), Said::Started);
        let (_, m) = split("[19:51:13] [Server thread/INFO]: Steve joined the game");
        assert_eq!(read(m), Said::Joined("Steve".into()));
        assert_eq!(read(split("[19:51:46] [Server thread/INFO]: Steve left the game").1), Said::Left("Steve".into()));
        let (_, m) = split("[22:06:35] [Server thread/INFO]: Steve[/127.0.0.1:32784] logged in with entity id 113 at (10.5, 51.0, 10.5)");
        assert_eq!(read(m), Said::LoggedIn("Steve".into(), "127.0.0.1".into()));
        assert_eq!(read("Alex[/[0:0:0:0:0:0:0:1]:5000] logged in with entity id 2 at (0.0, 64.0, 0.0)"), Said::LoggedIn("Alex".into(), "::1".into()));
        assert_eq!(read("<Steve> Bob[/1.2.3.4:5] logged in with entity id 1"), Said::Chat("Steve".into(), "Bob[/1.2.3.4:5] logged in with entity id 1".into()), "chat can't pretend to be a login");
        assert_eq!(read(split("[19:51:15] [Server thread/INFO]: [Not Secure] <Steve> hello from the bot").1), Said::Chat("Steve".into(), "hello from the bot".into()));
        assert_eq!(read("<Steve> Alex joined the game"), Said::Chat("Steve".into(), "Alex joined the game".into()), "chat that looks like a join isn't one");
        let (_, m) = split("[19:48:58] [Server thread/INFO]: System chat: There are 0 of a max of 20 players online: ");
        assert_eq!(read(m), Said::List(vec![], Some(20)));
        assert_eq!(read("There are 2 of a max of 8 players online: Steve (5627dd98-e6be-3c21-b8a8-e92344183641), Alex (11111111-2222-3333-4444-555555555555)"), Said::List(vec![("Steve".into(), Some("5627dd98-e6be-3c21-b8a8-e92344183641".into())), ("Alex".into(), Some("11111111-2222-3333-4444-555555555555".into()))], Some(8)));
        assert_eq!(read("There are 1/20 players online:"), Said::List(vec![], Some(20)));
        assert_eq!(split("[19:48:55] [Server thread/WARN]: **** SERVER IS RUNNING IN OFFLINE/INSECURE MODE!").0, "warn");
        assert!(matches!(read(split("[12:00:00] [Server thread/WARN]: **** FAILED TO BIND TO PORT!").1), Said::Problem(_)));
        assert_eq!(strip_ansi("\x1b[97mThis server is running Paper\x1b[0m\r"), "This server is running Paper");
    }

    /// Run as a pretend Minecraft server when the test runner starts this
    /// test with OMNIHUB_FAKE_MC set (other tests start it that way).
    #[test]
    fn fake_server() {
        let Ok(dir) = std::env::var("OMNIHUB_FAKE_MC") else { return };
        let dir = std::path::PathBuf::from(dir);
        let say = |s: &str| {
            println!("[12:00:00] [Server thread/INFO]: {s}");
            let _ = std::io::stdout().flush();
        };
        if std::fs::read_to_string(dir.join("eula.txt")).map(|t| !t.contains("eula=true")).unwrap_or(true) {
            say("You need to agree to the EULA in order to run the server. Go to eula.txt for more info.");
            return;
        }
        if dir.join("crash-now").exists() {
            say("Exception in server tick loop");
            std::process::exit(1);
        }
        say("Starting minecraft server version 26.3");
        let props = super::super::props::Props::load(&dir.join("server.properties"));
        let max = props.get_or("max-players", "20").to_string();
        std::thread::sleep(Duration::from_millis(150));
        say("Done (0.150s)! For help, type \"help\"");
        let mut online: Vec<&str> = Vec::new();
        for l in std::io::stdin().lock().lines().map_while(Result::ok) {
            let l = l.trim().to_string();
            let words: Vec<&str> = l.split_whitespace().collect();
            match words.as_slice() {
                ["stop"] => {
                    say("Stopping the server");
                    say("Saving worlds");
                    std::fs::write(dir.join("stopped-cleanly"), "yes").ok();
                    return;
                }
                ["crash"] => std::process::exit(3),
                ["join", n, from @ ..] => {
                    let n: &'static str = Box::leak(n.to_string().into_boxed_str());
                    online.push(n);
                    say(&format!("{n}[/{}:50123] logged in with entity id 7 at (0.5, 70.0, 0.5)", from.first().unwrap_or(&"127.0.0.1")));
                    say(&format!("{n} joined the game"));
                }
                ["leave", n] => {
                    online.retain(|o| o != n);
                    say(&format!("{n} left the game"));
                }
                ["kick", n, ..] if online.contains(n) => {
                    online.retain(|o| o != n);
                    say(&format!("Kicked {n}: {}", words[2..].join(" ")));
                    say(&format!("{n} left the game"));
                }
                // "player-op" is an operator typing /op in the game.
                ["op" | "deop" | "player-op", n] => {
                    // ops.json, as a real server keeps it.
                    use super::super::players::{self, List, Listed};
                    if words[0] != "deop" {
                        players::add_to(&dir, List::Ops, Listed::plain(&players::offline_uuid(n), n), None).ok();
                        say(&format!("Made {n} a server operator"));
                    } else {
                        players::remove_from(&dir, List::Ops, n).ok();
                        say(&format!("Made {n} no longer a server operator"));
                    }
                }
                ["list", ..] => say(&format!("System chat: There are {} of a max of {max} players online: {}", online.len(), online.iter().map(|n| format!("{n} (5627dd98-e6be-3c21-b8a8-e92344183641)")).collect::<Vec<_>>().join(", "))),
                ["execute", "as", "@a", "run", "data", "get", "entity", "@s", "Pos"] => online.iter().for_each(|n| say(&format!("{n} has the following entity data: [10.5d, 70.0d, -3.25d]"))),
                ["execute", "as", "@a", "run", "data", "get", "entity", "@s", "Dimension"] => online.iter().for_each(|n| say(&format!("{n} has the following entity data: \"minecraft:overworld\""))),
                ["data", "get", "entity", n] if online.contains(n) => say(&format!("{n} has the following entity data: {{Pos: [10.5d, 70.0d, -3.25d], Dimension: \"minecraft:overworld\", Health: 18.0f, foodLevel: 20, XpLevel: 3, playerGameType: 0, SelectedItemSlot: 0, Inventory: [{{count: 1, Slot: 0b, id: \"minecraft:diamond_sword\"}}, {{count: 32, Slot: 1b, id: \"minecraft:oak_log\"}}], equipment: {{head: {{count: 1, id: \"minecraft:diamond_helmet\"}}}}}}")),
                ["data", "get", "entity", ..] => say("No entity was found"),
                ["whitelist", "add", n] => say(&format!("Added {n} to the whitelist")),
                ["save-off"] => say("Automatic saving is now disabled"),
                ["save-on"] => say("Automatic saving is now enabled"),
                ["save-all", ..] => {
                    say("Saving the game (this may take a moment!)");
                    say("Saved the game");
                }
                _ => say(&format!("Unknown or incomplete command: {l}")),
            }
        }
    }

    /// A command that runs `fake_server` in `dir`.
    pub fn fake_command(dir: &std::path::Path) -> Command {
        let mut c = Command::new(std::env::current_exe().unwrap());
        c.args(["--exact", "mcserver::process::tests::fake_server", "--nocapture", "--test-threads=1", "-q"]).env("OMNIHUB_FAKE_MC", dir).current_dir(dir);
        c
    }

    #[test]
    fn runs_asks_and_stops() {
        let t = tempfile::tempdir().unwrap();
        std::fs::write(t.path().join("eula.txt"), "eula=true\n").unwrap();
        let console: Arc<Mutex<VecDeque<Line>>> = Arc::default();
        let heard: Arc<Mutex<Vec<Said>>> = Arc::default();
        let h = heard.clone();
        let (etx, erx) = channel();
        let p = Proc::spawn(fake_command(t.path()), console.clone(), Arc::default(), Arc::new(move |_, s| h.lock().push(s.clone())), Box::new(move |c| etx.send(c).unwrap())).unwrap();
        let until = Instant::now() + Duration::from_secs(20);
        while !heard.lock().contains(&Said::Started) {
            assert!(Instant::now() < until, "never started");
            std::thread::sleep(Duration::from_millis(20));
        }
        p.send("join Steve").unwrap();
        let pos = p.ask(&["execute as @a run data get entity @s Pos".into(), "list uuids".into()], |m| m.contains(" has the following entity data: "), |m| m.starts_with("There are "), Duration::from_secs(5)).unwrap();
        assert_eq!(pos, ["Steve has the following entity data: [10.5d, 70.0d, -3.25d]"]);
        assert!(!console.lock().iter().any(|l| l.text.contains("entity data")), "answers stay out of the console");
        assert!(console.lock().iter().any(|l| l.text.contains("Steve joined the game")));
        assert!(heard.lock().contains(&Said::Joined("Steve".into())));
        assert!(p.send("two\nlines").is_err());
        assert!(p.stop(Duration::from_secs(10)));
        assert_eq!(erx.recv_timeout(Duration::from_secs(5)).unwrap(), Some(0));
        assert!(t.path().join("stopped-cleanly").exists());
        assert!(p.send("list").is_err() || !p.alive());
        assert!(p.ask(&["list".into()], |_| true, |_| true, Duration::from_secs(1)).is_err());
    }
}
