//! The native-messaging host: Brave, Chrome and Edge start it for the
//! OmniHub extension, and it relays the browser's messages (on stdin/stdout)
//! to the running OmniHub app over its local pipe, and back.
//!
//! The OmniHub executable is its own host — the browser starts it with the
//! extension's origin as the first argument (see [`invoked_as_host`]); the
//! stand-alone `omnihub-browser-host` binary does the same for the headless
//! build. The origin is forwarded so the app can refuse anything but the
//! OmniHub extension. While OmniHub is not running the host answers
//! `app-not-running`, and starts OmniHub when the extension asks
//! (`launch-app`).

use std::time::Duration;

use super::ipc::{endpoint, read_frame, write_frame};
use serde_json::{json, Value};
use tokio::io::{AsyncRead, AsyncWrite};

#[cfg(windows)]
async fn connect(name: &str) -> std::io::Result<tokio::net::windows::named_pipe::NamedPipeClient> {
    tokio::net::windows::named_pipe::ClientOptions::new().open(name)
}

#[cfg(unix)]
async fn connect(name: &str) -> std::io::Result<tokio::net::UnixStream> {
    tokio::net::UnixStream::connect(name).await
}

/// Start OmniHub minimised: this executable when the app itself is the
/// host, otherwise the app next to the stand-alone host.
fn launch_app() {
    let Ok(me) = std::env::current_exe() else { return };
    let standalone = me.file_name().is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case(super::ipc::host_file_name()));
    let app = if standalone {
        let Some(dir) = me.parent() else { return };
        dir.join(if cfg!(windows) { "OmniHub.exe" } else { "omnihub" })
    } else {
        me
    };
    if app.is_file() {
        let _ = std::process::Command::new(app).arg("--minimized").stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn();
    }
}

async fn relay<S: AsyncRead + AsyncWrite + Send + 'static>(pipe: S, origin: String) {
    let (mut prd, mut pwr) = tokio::io::split(pipe);
    let hello = json!({ "type": "host-hello", "origin": origin });
    if write_frame(&mut pwr, hello.to_string().as_bytes()).await.is_err() {
        return;
    }
    let up = async move {
        let mut stdin = tokio::io::stdin();
        while let Ok(Some(frame)) = read_frame(&mut stdin).await {
            if write_frame(&mut pwr, &frame).await.is_err() {
                break;
            }
        }
    };
    let down = async move {
        let mut stdout = tokio::io::stdout();
        while let Ok(Some(frame)) = read_frame(&mut prd).await {
            if write_frame(&mut stdout, &frame).await.is_err() {
                break;
            }
        }
    };
    // Either side closing ends the session (the browser restarts the host as needed).
    tokio::select! {
        _ = up => {}
        _ = down => {}
    }
}

/// Answer `app-not-running` until the extension asks to start OmniHub and
/// it comes up (true), or the browser disconnects (false).
async fn not_running(name: &str) -> bool {
    let mut stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    while let Ok(Some(frame)) = read_frame(&mut stdin).await {
        let req = serde_json::from_slice::<Value>(&frame).unwrap_or(Value::Null);
        let id = req.get("id").cloned().unwrap_or(Value::Null);
        let reply = if req.get("type").and_then(|t| t.as_str()) == Some("launch-app") {
            launch_app();
            let mut up = false;
            for _ in 0..60 {
                if connect(name).await.is_ok() {
                    up = true;
                    break;
                }
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
            let reply = json!({ "id": id, "ok": up, "code": if up { "launched" } else { "app-not-running" }, "error": if up { "" } else { "OmniHub did not start." } });
            if write_frame(&mut stdout, reply.to_string().as_bytes()).await.is_err() {
                return false;
            }
            if up {
                return true;
            }
            continue;
        } else {
            json!({ "id": id, "ok": false, "code": "app-not-running", "error": "OmniHub is not running on this PC." })
        };
        if write_frame(&mut stdout, reply.to_string().as_bytes()).await.is_err() {
            break;
        }
    }
    false
}

/// The calling extension's origin when the browser started this process as
/// the native-messaging host (`<exe> chrome-extension://<id>/ …`).
pub fn invoked_as_host(args: &[String]) -> Option<&str> {
    args.get(1).map(String::as_str).filter(|a| a.starts_with("chrome-extension://"))
}

/// Relay between the browser (stdin/stdout) and the running app until
/// either side closes.
pub fn run(origin: &str) {
    let origin = origin.to_string();
    let Ok(rt) = tokio::runtime::Builder::new_current_thread().enable_all().build() else { return };
    rt.block_on(async move {
        let name = endpoint();
        loop {
            if let Ok(pipe) = connect(&name).await {
                return relay(pipe, origin).await;
            }
            if !not_running(&name).await {
                return;
            }
        }
    });
}
