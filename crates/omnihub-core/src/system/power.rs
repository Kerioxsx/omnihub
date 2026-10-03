//! Shutdown, restart, sleep, hibernate, lock, sign out and display off,
//! with a cancellable countdown.
//!
//! None of these need administrator rights: an interactive user may shut
//! down their own session (the shutdown privilege is enabled on demand).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::audit::Audit;
use crate::events::EventBus;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum PowerAction {
    Shutdown,
    Restart,
    Sleep,
    Hibernate,
    Lock,
    SignOut,
    DisplayOff,
}

impl PowerAction {
    pub fn label(self) -> &'static str {
        match self {
            PowerAction::Shutdown => "Shut down",
            PowerAction::Restart => "Restart",
            PowerAction::Sleep => "Sleep",
            PowerAction::Hibernate => "Hibernate",
            PowerAction::Lock => "Lock",
            PowerAction::SignOut => "Sign out",
            PowerAction::DisplayOff => "Turn off display",
        }
    }

    /// Actions that end the session or turn the PC off get a countdown.
    pub fn destructive(self) -> bool {
        matches!(self, PowerAction::Shutdown | PowerAction::Restart | PowerAction::SignOut | PowerAction::Hibernate | PowerAction::Sleep)
    }

    pub fn all() -> [PowerAction; 7] {
        [PowerAction::Lock, PowerAction::DisplayOff, PowerAction::Sleep, PowerAction::Hibernate, PowerAction::SignOut, PowerAction::Restart, PowerAction::Shutdown]
    }
}

#[cfg(windows)]
fn enable_shutdown_privilege() -> std::io::Result<()> {
    use windows::core::w;
    use windows::Win32::Foundation::{CloseHandle, HANDLE, LUID};
    use windows::Win32::Security::{AdjustTokenPrivileges, LookupPrivilegeValueW, LUID_AND_ATTRIBUTES, SE_PRIVILEGE_ENABLED, TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES, TOKEN_QUERY};
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
    unsafe {
        let mut token = HANDLE::default();
        OpenProcessToken(GetCurrentProcess(), TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY, &mut token).map_err(|e| std::io::Error::other(e.message()))?;
        let mut luid = LUID::default();
        let r = LookupPrivilegeValueW(None, w!("SeShutdownPrivilege"), &mut luid);
        if r.is_ok() {
            let tp = TOKEN_PRIVILEGES { PrivilegeCount: 1, Privileges: [LUID_AND_ATTRIBUTES { Luid: luid, Attributes: SE_PRIVILEGE_ENABLED }] };
            let _ = AdjustTokenPrivileges(token, false, Some(&tp), 0, None, None);
        }
        let _ = CloseHandle(token);
        r.map_err(|e| std::io::Error::other(e.message()))
    }
}

#[cfg(windows)]
pub fn execute(action: PowerAction) -> std::io::Result<()> {
    use windows::Win32::Foundation::{LPARAM, WPARAM};
    use windows::Win32::System::Shutdown::{ExitWindowsEx, LockWorkStation, EWX_FORCEIFHUNG, EWX_LOGOFF, EWX_POWEROFF, EWX_REBOOT, EWX_SHUTDOWN, SHTDN_REASON_FLAG_PLANNED, SHTDN_REASON_MAJOR_OTHER};
    use windows::Win32::System::Power::SetSuspendState;
    use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, HWND_BROADCAST, SC_MONITORPOWER, WM_SYSCOMMAND};
    let err = |e: windows::core::Error| std::io::Error::other(e.message());
    let reason = SHTDN_REASON_MAJOR_OTHER | SHTDN_REASON_FLAG_PLANNED;
    unsafe {
        match action {
            PowerAction::Lock => LockWorkStation().map_err(err),
            PowerAction::Sleep | PowerAction::Hibernate => {
                // SetSuspendState needs the shutdown privilege too.
                enable_shutdown_privilege()?;
                if SetSuspendState(action == PowerAction::Hibernate, false, false) {
                    Ok(())
                } else {
                    Err(std::io::Error::last_os_error())
                }
            }
            PowerAction::Shutdown => {
                enable_shutdown_privilege()?;
                ExitWindowsEx(EWX_SHUTDOWN | EWX_POWEROFF | EWX_FORCEIFHUNG, reason).map_err(err)
            }
            PowerAction::Restart => {
                enable_shutdown_privilege()?;
                ExitWindowsEx(EWX_REBOOT | EWX_FORCEIFHUNG, reason).map_err(err)
            }
            PowerAction::SignOut => ExitWindowsEx(EWX_LOGOFF | EWX_FORCEIFHUNG, reason).map_err(err),
            PowerAction::DisplayOff => PostMessageW(Some(HWND_BROADCAST), WM_SYSCOMMAND, WPARAM(SC_MONITORPOWER as usize), LPARAM(2)).map_err(err),
        }
    }
}

#[cfg(not(windows))]
pub fn execute(action: PowerAction) -> std::io::Result<()> {
    Err(std::io::Error::new(std::io::ErrorKind::Unsupported, format!("{} is only implemented on Windows", action.label())))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PendingPower {
    pub id: String,
    pub action: PowerAction,
    pub label: String,
    /// Unix seconds when it fires.
    pub at: i64,
    pub requested_by: String,
}

struct Pending {
    info: PendingPower,
    cancel: Arc<AtomicBool>,
}

/// Runs power actions after a visible, cancellable countdown.
pub struct PowerScheduler {
    pending: Arc<Mutex<Option<Pending>>>,
    events: EventBus,
    audit: Audit,
    /// Log instead of acting (headless test servers and CI).
    dry_run: bool,
}

impl PowerScheduler {
    pub fn new(events: EventBus, audit: Audit, dry_run: bool) -> Self {
        PowerScheduler { pending: Arc::new(Mutex::new(None)), events, audit, dry_run }
    }

    pub fn pending(&self) -> Option<PendingPower> {
        self.pending.lock().as_ref().map(|p| p.info.clone())
    }

    /// Schedule `action` in `delay` seconds (0 runs it now). Replaces any
    /// pending action.
    pub fn schedule(&self, action: PowerAction, delay: u32, requested_by: &str) -> PendingPower {
        self.cancel_with("replaced");
        let info = PendingPower {
            id: uuid::Uuid::new_v4().to_string(),
            action,
            label: action.label().into(),
            at: crate::db::now() + delay as i64,
            requested_by: requested_by.to_string(),
        };
        let cancel = Arc::new(AtomicBool::new(false));
        *self.pending.lock() = Some(Pending { info: info.clone(), cancel: cancel.clone() });
        self.audit.record(requested_by, &format!("power.{}", serde_json::to_value(action).unwrap().as_str().unwrap_or("?")), &format!("scheduled in {delay} s"), true);
        self.events.emit("power:pending", &info);

        let (pending, events, audit, dry_run, id) = (self.pending.clone(), self.events.clone(), self.audit.clone(), self.dry_run, info.id.clone());
        let who = requested_by.to_string();
        std::thread::spawn(move || {
            let deadline = std::time::Instant::now() + Duration::from_secs(delay as u64);
            while std::time::Instant::now() < deadline {
                if cancel.load(Ordering::Relaxed) {
                    return;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            {
                let mut p = pending.lock();
                if cancel.load(Ordering::Relaxed) || p.as_ref().is_none_or(|p| p.info.id != id) {
                    return;
                }
                *p = None;
            }
            let res = if dry_run { Ok(()) } else { execute(action) };
            let ok = res.is_ok();
            audit.record(&who, "power.executed", &match &res {
                Ok(()) => format!("{}{}", action.label(), if dry_run { " (dry run)" } else { "" }),
                Err(e) => format!("{} failed: {e}", action.label()),
            }, ok);
            events.emit("power:executed", serde_json::json!({ "action": action, "ok": ok, "error": res.err().map(|e| e.to_string()), "dryRun": dry_run }));
        });
        info
    }

    pub fn cancel(&self, by: &str) -> bool {
        let cancelled = self.cancel_with("cancelled");
        if cancelled {
            self.audit.record(by, "power.cancel", "", true);
        }
        cancelled
    }

    fn cancel_with(&self, reason: &str) -> bool {
        if let Some(p) = self.pending.lock().take() {
            p.cancel.store(true, Ordering::Relaxed);
            self.events.emit("power:cancelled", serde_json::json!({ "id": p.info.id, "reason": reason }));
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;

    fn scheduler() -> (PowerScheduler, EventBus, Audit) {
        let events = EventBus::new();
        let audit = Audit::new(Arc::new(Db::in_memory().unwrap()), events.clone());
        (PowerScheduler::new(events.clone(), audit.clone(), true), events, audit)
    }

    #[test]
    fn countdown_can_be_cancelled() {
        let (s, events, audit) = scheduler();
        let mut rx = events.subscribe();
        let p = s.schedule(PowerAction::Shutdown, 1, "phone:test");
        assert_eq!(s.pending().unwrap().id, p.id);
        assert!(s.cancel("desktop"));
        assert!(s.pending().is_none());
        std::thread::sleep(Duration::from_millis(1300));
        let mut topics = vec![];
        while let Ok(e) = rx.try_recv() {
            topics.push(e.topic);
        }
        assert!(!topics.contains(&"power:executed".to_string()), "{topics:?}");
        assert!(audit.list(10, 0).iter().any(|e| e.action == "power.cancel"));
    }

    #[test]
    fn executes_after_delay() {
        let (s, events, audit) = scheduler();
        let mut rx = events.subscribe();
        s.schedule(PowerAction::Lock, 0, "desktop");
        std::thread::sleep(Duration::from_millis(400));
        let mut executed = false;
        while let Ok(e) = rx.try_recv() {
            executed |= e.topic == "power:executed";
        }
        assert!(executed);
        assert!(audit.list(10, 0).iter().any(|e| e.action == "power.executed"));
        assert!(s.pending().is_none());
    }
}
