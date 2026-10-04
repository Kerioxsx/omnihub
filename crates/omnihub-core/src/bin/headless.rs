//! OmniHub without the desktop window: runs the phone companion server
//! (and the elevated scan helper when started in helper mode).
//!
//!     omnihub-headless [--home DIR] [--port N] [--http] [--pair] [--dry-run-power]
//!
//! While it runs, type `send PATH|PATH`, `text TEXT` or `pair` to offer
//! files, folders or text to the phones, or open a new pairing window;
//! `vault-create PW`, `vault-unlock PW`, `vault-add {json}`, `vault-list`, `vault-lock`,
//! `browser on|off` and `browser-allow` / `browser-deny` drive the vault and
//! browser autofill (for testing the extension).
//!
//! Useful on a PC you only control from your phone, and for testing the
//! phone app during development.

use std::path::PathBuf;
use std::sync::Arc;

use omnihub_core::core::{AppCore, CoreOptions};
use omnihub_core::paths::AppPaths;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if let Some(code) = omnihub_core::helper::run_if_helper(&args) {
        std::process::exit(code);
    }
    if let Some(origin) = omnihub_core::browser::host::invoked_as_host(&args) {
        omnihub_core::browser::host::run(origin);
        return Ok(());
    }
    tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into())).init();

    let mut home: Option<PathBuf> = None;
    let mut port: Option<u16> = None;
    let mut http = false;
    let mut pair = false;
    let mut dry_run = false;
    let mut it = args.iter().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--home" => home = it.next().map(PathBuf::from),
            "--port" => port = it.next().and_then(|p| p.parse().ok()),
            "--http" => http = true,
            "--pair" => pair = true,
            "--dry-run-power" => dry_run = true,
            "-h" | "--help" => {
                println!("omnihub-headless [--home DIR] [--port N] [--http] [--pair] [--dry-run-power]");
                return Ok(());
            }
            other => anyhow::bail!("unknown argument {other}"),
        }
    }
    let paths = match home {
        Some(h) => AppPaths::at(&h)?,
        None => AppPaths::default_for_user()?,
    };
    let core: Arc<AppCore> = AppCore::new(paths, CoreOptions { dry_run_power: dry_run, ..Default::default() })?;
    let mut patch = serde_json::json!({ "remote": { "enabled": true } });
    if let Some(p) = port {
        patch["remote"]["port"] = p.into();
    }
    if http {
        patch["remote"]["tls"] = false.into();
    }
    // Enabling (or changing) the server settings starts it.
    let before = core.settings.get().remote;
    core.update_settings(&patch)?;
    if before.enabled && !core.remote.is_running() {
        core.remote.start(core.clone())?;
    }
    core.start_background();
    let status = core.remote.status();
    println!("OmniHub companion server running:");
    for u in &status.urls {
        println!("  {u}");
    }
    if let Some(fp) = &status.fingerprint {
        println!("certificate SHA-256: {fp}");
    }
    if pair {
        let info = core.remote.begin_pairing()?;
        println!("pairing open for 5 minutes, PIN {}", info.pin);
        for u in &info.urls {
            println!("  {u}");
        }
    }
    // A small console for development and tests:
    //   send PATH [PATH…]   offer files or folders to every phone
    //   text TEXT           offer text
    //   pair                open a new pairing window
    //   vault-create PW, vault-unlock PW, vault-lock, vault-add {EntryInput json}
    //   browser on|off      browser autofill (registers the host with the browsers)
    //   browser-allow, browser-deny   answer a pending browser pairing
    let console = core.clone();
    std::thread::spawn(move || {
        use std::io::BufRead;
        for line in std::io::stdin().lock().lines().map_while(Result::ok) {
            let line = line.trim();
            let (cmd, rest) = line.split_once(' ').unwrap_or((line, ""));
            let res: anyhow::Result<String> = match cmd {
                "send" => {
                    let paths: Vec<String> = rest.split('|').map(|p| p.trim().to_string()).filter(|p| !p.is_empty()).collect();
                    console.remote.send_to_phone(&paths, None, &console.outbox_dir()).map(|v| format!("offered {}", v.iter().map(|i| i.name.as_str()).collect::<Vec<_>>().join(", ")))
                }
                "text" => console.remote.send_text(rest, None).map(|i| format!("offered text {}", i.id)),
                "pair" => console.remote.begin_pairing().map(|i| format!("PIN {} {}", i.pin, i.urls.join(" "))),
                "vault-create" => console.vault.create(rest).map(|_| "vault created".into()).map_err(Into::into),
                "vault-unlock" => console.vault.unlock(rest).map(|_| "vault unlocked".into()).map_err(Into::into),
                "vault-lock" => {
                    console.vault.lock();
                    Ok("vault locked".into())
                }
                "vault-list" => console.vault.list().map(|l| l.iter().map(|e| format!("{} <{}> {}", e.title, if e.username.is_empty() { &e.email } else { &e.username }, e.url)).collect::<Vec<_>>().join(" | ")).map_err(Into::into),
                "vault-add" => serde_json::from_str::<omnihub_core::vault::EntryInput>(rest).map_err(anyhow::Error::from).and_then(|input| console.vault.save(input).map(|s| format!("saved {}", s.id)).map_err(Into::into)),
                "browser" => console.update_settings(&serde_json::json!({ "vault": { "browserAutofill": rest == "on" } })).map(|_| format!("browser autofill {} ({})", if rest == "on" { "on" } else { "off" }, omnihub_core::browser::ipc::endpoint())),
                "browser-allow" | "browser-deny" => match console.browser.pending() {
                    Some(p) => {
                        console.browser.respond(&p.id, cmd == "browser-allow");
                        Ok(format!("{} {} (code {})", if cmd == "browser-allow" { "allowed" } else { "denied" }, p.name, p.code))
                    }
                    None => Err(anyhow::anyhow!("no browser is waiting")),
                },
                "" => continue,
                other => Err(anyhow::anyhow!("unknown command {other} (send, text, pair, vault-*, browser*)")),
            };
            match res {
                Ok(msg) => println!("ok: {msg}"),
                Err(e) => println!("error: {e}"),
            }
        }
    });
    core.remote.runtime().block_on(async {
        let _ = tokio::signal::ctrl_c().await;
    });
    core.remote.stop();
    Ok(())
}
