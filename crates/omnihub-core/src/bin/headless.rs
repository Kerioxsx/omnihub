//! OmniHub without the desktop window: runs the phone companion server
//! (and the elevated scan helper when started in helper mode).
//!
//!     omnihub-headless [--home DIR] [--port N] [--http] [--pair] [--dry-run-power]
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
    core.remote.runtime().block_on(async {
        let _ = tokio::signal::ctrl_c().await;
    });
    core.remote.stop();
    Ok(())
}
