//! The local pipe between `omnihub-browser-host` (started by the browser)
//! and the running app: frames of a little-endian `u32` length followed by
//! JSON, the same framing Chrome's native messaging uses, so the host can
//! relay bytes without parsing them.
//!
//! On Windows it is a named pipe whose security descriptor admits only the
//! current user (and SYSTEM), refuses remote clients, and whose client must
//! be our own host executable. On Linux it is a Unix socket with mode 0600.

use std::sync::Arc;

use parking_lot::Mutex;
use serde_json::{json, Value};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use super::{handle, Session, MAX_MESSAGE};
use crate::core::AppCore;

/// Where the app listens (overridable with `OMNIHUB_BROWSER_ENDPOINT`, for tests).
pub fn endpoint() -> String {
    if let Ok(p) = std::env::var("OMNIHUB_BROWSER_ENDPOINT") {
        if !p.is_empty() {
            return p;
        }
    }
    #[cfg(windows)]
    {
        let user: String = std::env::var("USERNAME").unwrap_or_default().chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_').take(40).collect();
        format!(r"\\.\pipe\omnihub-browser-{user}")
    }
    #[cfg(unix)]
    {
        let dir = std::env::var("XDG_RUNTIME_DIR").ok().filter(|d| std::path::Path::new(d).is_dir()).unwrap_or_else(|| "/tmp".into());
        // SAFETY: getuid has no preconditions.
        unsafe extern "C" {
            fn getuid() -> u32;
        }
        format!("{dir}/omnihub-browser-{}.sock", unsafe { getuid() })
    }
}

pub async fn read_frame<R: AsyncRead + Unpin>(r: &mut R) -> std::io::Result<Option<Vec<u8>>> {
    let mut len = [0u8; 4];
    match r.read_exact(&mut len).await {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e),
    }
    let n = u32::from_le_bytes(len) as usize;
    if n > MAX_MESSAGE {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "message too large"));
    }
    let mut buf = vec![0u8; n];
    r.read_exact(&mut buf).await?;
    Ok(Some(buf))
}

pub async fn write_frame<W: AsyncWrite + Unpin>(w: &mut W, bytes: &[u8]) -> std::io::Result<()> {
    if bytes.len() > MAX_MESSAGE {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "message too large"));
    }
    w.write_all(&(bytes.len() as u32).to_le_bytes()).await?;
    w.write_all(bytes).await?;
    w.flush().await
}

/// Serve one connected host until it disconnects.
pub async fn serve<S: AsyncRead + AsyncWrite + Send + 'static>(core: Arc<AppCore>, stream: S) {
    let (mut rd, mut wr) = tokio::io::split(stream);
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Value>();
    let writer = tokio::spawn(async move {
        while let Some(v) = rx.recv().await {
            let Ok(bytes) = serde_json::to_vec(&v) else { continue };
            if write_frame(&mut wr, &bytes).await.is_err() {
                break;
            }
        }
    });
    let session = Arc::new(Mutex::new(Session::default()));
    let mut events = core.events.subscribe();
    loop {
        tokio::select! {
            frame = read_frame(&mut rd) => {
                let Ok(Some(bytes)) = frame else { break };
                let Ok(req) = serde_json::from_slice::<Value>(&bytes) else {
                    let _ = tx.send(json!({ "ok": false, "code": "bad-json", "error": "not JSON" }));
                    continue;
                };
                // Requests are handled in order (host-hello must precede the
                // rest). Only waiting for the user — pairing approval, Windows
                // Hello — runs alongside, so fills keep working meanwhile.
                let slow = matches!(req.get("type").and_then(|t| t.as_str()), Some("pair-wait" | "unlock"));
                let (core, session, tx) = (core.clone(), session.clone(), tx.clone());
                let task = tokio::task::spawn_blocking(move || {
                    if let Some(resp) = handle(&core, &session, &req) {
                        let _ = tx.send(resp);
                    }
                });
                if !slow {
                    let _ = task.await;
                }
            }
            ev = events.recv() => {
                let Ok(ev) = ev else { continue };
                if matches!(ev.topic.as_str(), "vault:locked" | "vault:unlocked") && session.lock().client.is_some() {
                    let _ = tx.send(json!({ "type": "event", "topic": ev.topic }));
                }
            }
        }
    }
    drop(tx);
    let _ = writer.await;
}

pub struct Server {
    stop: Arc<tokio::sync::Notify>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Server {
    /// Start listening on [`endpoint`]. Errors (pipe already taken, socket
    /// path unusable) are reported here, not later.
    pub fn start(core: Arc<AppCore>) -> std::io::Result<Server> {
        Self::start_at(core, endpoint())
    }

    /// Start listening on a given pipe name / socket path.
    pub fn start_at(core: Arc<AppCore>, name: String) -> std::io::Result<Server> {
        let stop = Arc::new(tokio::sync::Notify::new());
        let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(2).thread_name("omnihub-browser").enable_all().build()?;
        #[cfg(unix)]
        let listener = {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::remove_file(&name);
            let l = std::os::unix::net::UnixListener::bind(&name)?;
            std::fs::set_permissions(&name, std::fs::Permissions::from_mode(0o600))?;
            l.set_nonblocking(true)?;
            l
        };
        #[cfg(windows)]
        let first = {
            let _guard = rt.enter();
            win::create(&name, true)?
        };
        let s = stop.clone();
        let thread = std::thread::Builder::new().name("browser-ipc".into()).spawn(move || {
            rt.block_on(async move {
                #[cfg(unix)]
                {
                    let Ok(listener) = tokio::net::UnixListener::from_std(listener) else { return };
                    loop {
                        tokio::select! {
                            _ = s.notified() => break,
                            r = listener.accept() => match r {
                                Ok((stream, _)) => { tokio::spawn(serve(core.clone(), stream)); }
                                Err(e) => { tracing::warn!("browser pipe accept failed: {e}"); tokio::time::sleep(std::time::Duration::from_millis(200)).await; }
                            }
                        }
                    }
                    let _ = std::fs::remove_file(&name);
                }
                #[cfg(windows)]
                {
                    let mut server = first;
                    loop {
                        tokio::select! {
                            _ = s.notified() => break,
                            r = server.connect() => {
                                let next = match win::create(&name, false) {
                                    Ok(n) => n,
                                    Err(e) => { tracing::warn!("browser pipe failed: {e}"); break; }
                                };
                                let connected = std::mem::replace(&mut server, next);
                                if r.is_ok() && win::client_is_our_host(&connected) {
                                    tokio::spawn(serve(core.clone(), connected));
                                } else {
                                    tracing::warn!("refused a browser pipe client that is not omnihub-browser-host");
                                }
                            }
                        }
                    }
                }
            });
        })?;
        Ok(Server { stop, thread: Some(thread) })
    }

    pub fn stop(mut self) {
        self.stop.notify_one();
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.notify_one();
    }
}

/// Executable name of the host the browser starts.
pub fn host_file_name() -> &'static str {
    if cfg!(windows) {
        "omnihub-browser-host.exe"
    } else {
        "omnihub-browser-host"
    }
}

#[cfg(windows)]
mod win {
    use std::os::windows::io::AsRawHandle;

    use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
    use windows::core::{PCWSTR, PWSTR};
    use windows::Win32::Foundation::{CloseHandle, HANDLE, HLOCAL, LocalFree};
    use windows::Win32::Security::Authorization::{ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1};
    use windows::Win32::Security::{GetTokenInformation, TokenUser, PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES, TOKEN_QUERY, TOKEN_USER};
    use windows::Win32::System::Pipes::GetNamedPipeClientProcessId;
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcess, OpenProcessToken, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION};

    /// SDDL granting full access to the current user and SYSTEM only.
    fn sddl() -> Option<String> {
        unsafe {
            let mut token = HANDLE::default();
            OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).ok()?;
            let mut len = 0u32;
            let _ = GetTokenInformation(token, TokenUser, None, 0, &mut len);
            let mut buf = vec![0u8; len as usize];
            let ok = GetTokenInformation(token, TokenUser, Some(buf.as_mut_ptr() as *mut _), len, &mut len).is_ok();
            let _ = CloseHandle(token);
            if !ok {
                return None;
            }
            let user = &*(buf.as_ptr() as *const TOKEN_USER);
            let mut s = PWSTR::null();
            ConvertSidToStringSidW(user.User.Sid, &mut s).ok()?;
            let sid = s.to_string().ok();
            let _ = LocalFree(Some(HLOCAL(s.0 as *mut _)));
            sid.map(|sid| format!("D:P(A;;GA;;;{sid})(A;;GA;;;SY)"))
        }
    }

    pub fn create(name: &str, first: bool) -> std::io::Result<NamedPipeServer> {
        let mut opts = ServerOptions::new();
        opts.first_pipe_instance(first).reject_remote_clients(true);
        if let Some(sddl) = sddl() {
            let wide: Vec<u16> = sddl.encode_utf16().chain(Some(0)).collect();
            let mut sd = PSECURITY_DESCRIPTOR::default();
            unsafe {
                if ConvertStringSecurityDescriptorToSecurityDescriptorW(PCWSTR(wide.as_ptr()), SDDL_REVISION_1, &mut sd, None).is_ok() {
                    let mut sa = SECURITY_ATTRIBUTES { nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32, lpSecurityDescriptor: sd.0, bInheritHandle: false.into() };
                    let r = opts.create_with_security_attributes_raw(name, &mut sa as *mut _ as *mut _);
                    let _ = LocalFree(Some(HLOCAL(sd.0)));
                    return r;
                }
            }
        }
        opts.create(name)
    }

    /// The client must be our own executable (the app as host) or the
    /// stand-alone `omnihub-browser-host.exe` next to it.
    pub fn client_is_our_host(pipe: &NamedPipeServer) -> bool {
        let Ok(me) = std::env::current_exe() else { return false };
        let Some(dir) = me.parent() else { return false };
        let standalone = dir.join(super::host_file_name());
        unsafe {
            let mut pid = 0u32;
            if GetNamedPipeClientProcessId(HANDLE(pipe.as_raw_handle()), &mut pid).is_err() {
                return false;
            }
            let Ok(proc) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else { return false };
            let mut buf = [0u16; 1024];
            let mut len = buf.len() as u32;
            let ok = QueryFullProcessImageNameW(proc, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len).is_ok();
            let _ = CloseHandle(proc);
            if !ok {
                return false;
            }
            let path = String::from_utf16_lossy(&buf[..len as usize]);
            [me.as_path(), standalone.as_path()].iter().any(|p| path.eq_ignore_ascii_case(&p.to_string_lossy()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn frames_round_trip_and_are_bounded() {
        let (mut a, mut b) = tokio::io::duplex(1 << 21);
        write_frame(&mut a, br#"{"type":"ping"}"#).await.unwrap();
        assert_eq!(read_frame(&mut b).await.unwrap().unwrap(), br#"{"type":"ping"}"#);
        a.write_all(&((MAX_MESSAGE as u32) + 1).to_le_bytes()).await.unwrap();
        assert!(read_frame(&mut b).await.is_err());
        drop(a);
        let (c, mut d) = tokio::io::duplex(64);
        drop(c);
        assert!(read_frame(&mut d).await.unwrap().is_none());
    }
}
