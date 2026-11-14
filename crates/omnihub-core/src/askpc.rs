//! "Ask your PC": a question in plain words ("why is my PC slow?", "what's
//! using my internet?") answered from what OmniHub measures right now: the
//! busiest programs, the network, temperatures, free space and the health
//! checks. The answer lists the facts it used and what to do; with a local
//! model (Ollama) it can also be put into a friendlier sentence, still from
//! those facts only.

use serde::Serialize;

/// What a question is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Topic {
    Slow,
    Internet,
    Heat,
    Space,
    Memory,
    Health,
}

pub fn topics(question: &str) -> Vec<Topic> {
    let q = question.to_lowercase();
    let has = |ws: &[&str]| ws.iter().any(|w| q.contains(w));
    let mut t = Vec::new();
    if has(&["slow", "lag", "laggy", "stutter", "freez", "lento", "cpu", "fps", "busy", "hang"]) {
        t.push(Topic::Slow);
    }
    if has(&["internet", "network", "wifi", "wi-fi", "download", "upload", "ping", "bandwidth", "data", "red ", "connection"]) {
        t.push(Topic::Internet);
    }
    if has(&["hot", "heat", "temp", "fan", "loud", "noisy", "caliente", "overheat"]) {
        t.push(Topic::Heat);
    }
    if has(&["space", "storage", "disk", "drive", "full", "espacio", "gb"]) {
        t.push(Topic::Space);
    }
    if has(&["ram", "memory", "memoria"]) {
        t.push(Topic::Memory);
    }
    if t.is_empty() || has(&["health", "wrong", "problem", "check", "ok?", "okay"]) {
        t.push(Topic::Health);
    }
    t.dedup();
    t
}

/// What OmniHub measured, for the answer.
#[derive(Debug, Clone, Default)]
pub struct Facts {
    /// (program, CPU %, memory bytes)
    pub top_cpu: Vec<(String, f32, u64)>,
    pub top_memory: Vec<(String, f32, u64)>,
    pub cpu_total: Option<f32>,
    pub memory_used: Option<f64>,
    /// Bytes per second.
    pub down: u64,
    pub up: u64,
    /// (program, open connections)
    pub net_apps: Vec<(String, usize)>,
    pub cpu_temp: Option<f32>,
    pub gpu_temp: Option<f32>,
    /// (drive, free bytes, total bytes)
    pub disks: Vec<(String, u64, u64)>,
    /// Health checks that aren't fine: (title, detail, fix).
    pub warnings: Vec<(String, String, Option<String>)>,
    pub health_score: Option<u8>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Answer {
    pub title: String,
    pub points: Vec<String>,
    pub tips: Vec<String>,
    /// A page that helps ("tasks", "storage?tab=cleanup"…).
    pub route: Option<String>,
}

fn gb(b: u64) -> String {
    format!("{:.1} GB", b as f64 / 1e9)
}

fn speed(b: u64) -> String {
    let bits = b as f64 * 8.0;
    if bits >= 1e6 {
        format!("{:.1} Mbit/s", bits / 1e6)
    } else {
        format!("{:.0} kbit/s", bits / 1e3)
    }
}

pub fn answer(topic: Topic, f: &Facts) -> Answer {
    match topic {
        Topic::Slow => {
            let mut points = Vec::new();
            if let Some(c) = f.cpu_total {
                points.push(format!("The processor is {c:.0}% busy right now."));
            }
            for (n, c, _) in f.top_cpu.iter().filter(|p| p.1 >= 5.0).take(3) {
                points.push(format!("{n} is using {c:.0}% of the processor."));
            }
            if let Some(m) = f.memory_used.filter(|m| *m >= 0.85) {
                points.push(format!("Memory is {:.0}% full, so Windows is swapping to disk.", m * 100.0));
            }
            if let Some(t) = f.cpu_temp.filter(|t| *t >= 90.0) {
                points.push(format!("The processor is at {t:.0} °C and may be slowing itself down to cool."));
            }
            let busy = f.cpu_total.unwrap_or(0.0) >= 70.0 || f.top_cpu.iter().any(|p| p.1 >= 30.0);
            let mut tips = Vec::new();
            if let Some((n, _, _)) = f.top_cpu.first().filter(|p| p.1 >= 20.0) {
                tips.push(format!("Close {n} if you don't need it (Tasks page)."));
            }
            if f.memory_used.is_some_and(|m| m >= 0.85) {
                tips.push("Close some browser tabs or apps to free memory.".into());
            }
            if tips.is_empty() {
                tips.push(if busy { "Use a game's boost before playing: it pauses background work." } else { "Nothing is hogging the PC right now. If it's slow in one game, check that game's FPS history." }.into());
            }
            Answer { title: if busy { "Something is keeping the PC busy".into() } else { "The PC isn't busy right now".into() }, points, tips, route: Some("tasks".into()) }
        }
        Topic::Internet => {
            let mut points = vec![format!("Right now: {} down, {} up.", speed(f.down), speed(f.up))];
            for (n, c) in f.net_apps.iter().take(4) {
                points.push(format!("{n} has {c} connection{} open.", if *c == 1 { "" } else { "s" }));
            }
            let heavy = f.down >= 2_000_000 || f.up >= 1_000_000;
            let tips = vec![if heavy { "Pause downloads and updates (Steam, Windows Update, cloud sync) while you play.".into() } else { "Your connection is quiet. For lag in games, check the ping on the Games page.".into() }];
            Answer { title: if heavy { "Your internet is busy".into() } else { "Your internet is quiet".into() }, points, tips, route: Some("tools?tab=network".into()) }
        }
        Topic::Heat => {
            let mut points = Vec::new();
            if let Some(t) = f.cpu_temp {
                points.push(format!("Processor: {t:.0} °C."));
            }
            if let Some(t) = f.gpu_temp {
                points.push(format!("Graphics card: {t:.0} °C."));
            }
            let hot = f.cpu_temp.is_some_and(|t| t >= 85.0) || f.gpu_temp.is_some_and(|t| t >= 83.0);
            if points.is_empty() {
                points.push("OmniHub can't read the temperatures on this PC (the sensors need LibreHardwareMonitor support).".into());
            }
            let tips = vec![if hot { "Clean dust from the fans and filters, and make sure air can get in and out. A Fast fan profile helps meanwhile.".into() } else { "Temperatures are fine.".into() }];
            Answer { title: if hot { "It's running hot".into() } else { "Temperatures look normal".into() }, points, tips, route: Some("rig?tab=fans".into()) }
        }
        Topic::Space => {
            let mut points = Vec::new();
            for (d, free, total) in &f.disks {
                points.push(format!("{d} has {} free of {}.", gb(*free), gb(*total)));
            }
            let low = f.disks.iter().any(|(_, free, total)| *total > 0 && (*free as f64 / *total as f64) < 0.1);
            let tips = vec![if low { "Run a clean-up (Storage → Clean up) and look for big files.".into() } else { "There's room on your drives.".into() }];
            Answer { title: if low { "A drive is almost full".into() } else { "There's enough free space".into() }, points, tips, route: Some("storage".into()) }
        }
        Topic::Memory => {
            let mut points = Vec::new();
            if let Some(m) = f.memory_used {
                points.push(format!("Memory is {:.0}% full.", m * 100.0));
            }
            for (n, _, mem) in f.top_memory.iter().take(4) {
                points.push(format!("{n} uses {}.", gb(*mem)));
            }
            let full = f.memory_used.is_some_and(|m| m >= 0.85);
            Answer { title: if full { "Memory is nearly full".into() } else { "Memory is fine".into() }, points, tips: vec![if full { "Close the biggest programs you don't need.".into() } else { "No need to close anything.".into() }], route: Some("tasks?sort=memory".into()) }
        }
        Topic::Health => {
            let mut points: Vec<String> = f.warnings.iter().take(5).map(|(t, d, _)| format!("{t}: {d}")).collect();
            if points.is_empty() {
                points.push("Every health check is fine.".into());
            }
            let tips: Vec<String> = f.warnings.iter().filter_map(|(_, _, fix)| fix.clone()).take(3).collect();
            Answer { title: f.health_score.map(|s| format!("Health score: {s}/100")).unwrap_or_else(|| "PC health".into()), points, tips, route: Some("home".into()) }
        }
    }
}

/// The facts as text, for a local model to put into words (and nothing else).
pub fn as_prompt(question: &str, answers: &[Answer]) -> String {
    let mut s = format!("The user asked about their Windows PC: \"{question}\"\n\nWhat OmniHub measured right now:\n");
    for a in answers {
        s.push_str(&format!("- {}\n", a.title));
        for p in &a.points {
            s.push_str(&format!("  - {p}\n"));
        }
        for t in &a.tips {
            s.push_str(&format!("  - Suggested: {t}\n"));
        }
    }
    s.push_str("\nAnswer in 2–4 short, friendly sentences using only these facts. Don't invent numbers or causes.");
    s
}

impl crate::core::AppCore {
    /// What OmniHub measures now, for "Ask your PC".
    pub fn ask_facts(&self) -> Facts {
        use crate::system::procs::ProcessSort;
        let u = self.procs.usage(ProcessSort::Cpu, 6, false);
        let mem = self.procs.top(ProcessSort::Memory, 6);
        let rig = self.rig.status();
        let net = self.netmon.status(crate::system::netmon::group_connections(&crate::system::ports::list()));
        let inputs = crate::health::gather();
        let health = crate::health::score(&inputs, crate::db::now() * 1000);
        Facts {
            top_cpu: u.processes.iter().map(|p| (p.name.clone(), p.cpu, p.memory)).collect(),
            top_memory: mem.iter().map(|p| (p.name.clone(), p.cpu, p.memory)).collect(),
            cpu_total: Some(u.cpu),
            memory_used: (u.memory_total > 0).then(|| u.memory_used as f64 / u.memory_total as f64),
            down: net.down,
            up: net.up,
            net_apps: {
                let mut a: Vec<(String, usize)> = net.apps.iter().map(|a| (a.process.clone(), a.connections)).collect();
                a.sort_by_key(|x| std::cmp::Reverse(x.1));
                a
            },
            cpu_temp: rig.cpu_temp,
            gpu_temp: rig.gpu_temp,
            disks: inputs.disks.clone(),
            warnings: health.checks.iter().filter(|c| c.status != "ok").map(|c| (c.title.clone(), c.detail.clone(), c.fix.clone())).collect(),
            health_score: Some(health.score),
        }
    }

    /// Answer a question about the PC from what's measured now.
    pub fn ask_pc(&self, question: &str) -> Vec<Answer> {
        let f = self.ask_facts();
        topics(question).into_iter().map(|t| answer(t, &f)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn answers_from_facts() {
        assert_eq!(topics("why is my pc so slow?"), vec![Topic::Slow]);
        assert_eq!(topics("what's using my internet"), vec![Topic::Internet]);
        assert_eq!(topics("hello"), vec![Topic::Health]);
        let f = Facts { top_cpu: vec![("chrome.exe".into(), 45.0, 2_000_000_000)], cpu_total: Some(82.0), memory_used: Some(0.91), down: 5_000_000, net_apps: vec![("steam.exe".into(), 12)], disks: vec![("C:".into(), 20_000_000_000, 500_000_000_000)], ..Default::default() };
        let a = answer(Topic::Slow, &f);
        assert_eq!(a.title, "Something is keeping the PC busy");
        assert!(a.points.iter().any(|p| p.contains("chrome.exe is using 45%")));
        assert!(a.tips.iter().any(|t| t.contains("Close chrome.exe")));
        assert!(answer(Topic::Internet, &f).points[0].contains("40.0 Mbit/s"));
        assert_eq!(answer(Topic::Space, &f).title, "A drive is almost full");
        let heat = answer(Topic::Heat, &f);
        assert!(heat.points[0].contains("can't read"), "no invented temperatures");
        assert!(as_prompt("slow?", &[a]).contains("only these facts"));
    }
}
