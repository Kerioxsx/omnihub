//! Approve on your phone: before something sensitive happens on the PC
//! (showing or copying a vault password, restarting as administrator,
//! shredding, changing PATH, turning the check off), OmniHub asks the paired
//! phones and waits for a tap. Phones see the request while the OmniHub app
//! is open on them; with no answer within the time, the action doesn't happen.

use std::collections::HashMap;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

/// How long a request waits for a phone.
pub const WAIT: Duration = Duration::from_secs(60);
/// After a yes, the same thing isn't asked again for this long (open, then copy).
pub const GRACE: Duration = Duration::from_secs(120);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum Kind {
    VaultReveal,
    /// Setting a new master password with Windows Hello (forgot it).
    VaultReset,
    Admin,
    Shred,
    Environment,
    /// Turning approve-on-phone off (so it can't be switched off quietly).
    Settings,
}

impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Kind::VaultReveal => "Show a vault password on the PC",
            Kind::VaultReset => "Set a new vault master password",
            Kind::Admin => "Restart OmniHub as administrator",
            Kind::Shred => "Shred files for good",
            Kind::Environment => "Change environment variables",
            Kind::Settings => "Stop asking the phone for approval",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub id: String,
    pub kind: Kind,
    pub title: String,
    /// What exactly ("Gmail", "C:\\old\\taxes.zip").
    pub detail: String,
    /// Unix seconds when it stops waiting.
    pub expires_at: i64,
}

/// A waiting request and where its answer (yes?, which phone) goes.
type Pending = HashMap<String, (Request, mpsc::Sender<(bool, String)>)>;

#[derive(Default)]
pub struct Approvals {
    pending: Mutex<Pending>,
    granted: Mutex<Vec<(Kind, String, Instant)>>,
}

impl Approvals {
    /// Ask and wait. `notify` tells the phones (an event); `Ok(device)` when approved.
    pub fn ask(&self, kind: Kind, detail: &str, wait: Duration, notify: impl FnOnce(&Request)) -> Result<String, String> {
        let id = crate::remote::auth::random_token(12);
        let req = Request { id: id.clone(), kind, title: kind.label().into(), detail: detail.chars().take(200).collect(), expires_at: chrono::Utc::now().timestamp() + wait.as_secs() as i64 };
        let (tx, rx) = mpsc::channel();
        self.pending.lock().insert(id.clone(), (req.clone(), tx));
        notify(&req);
        let r = rx.recv_timeout(wait);
        self.pending.lock().remove(&id);
        match r {
            Ok((true, device)) => {
                let mut g = self.granted.lock();
                g.retain(|(_, _, at)| at.elapsed() < GRACE);
                g.push((kind, detail.to_string(), Instant::now()));
                Ok(device)
            }
            Ok((false, device)) => Err(format!("Declined on {device}.")),
            Err(_) => Err("No phone approved it in time. Open OmniHub on your phone and try again.".into()),
        }
    }

    /// Approved for this a moment ago.
    pub fn recently(&self, kind: Kind, detail: &str) -> bool {
        self.granted.lock().iter().any(|(k, d, at)| *k == kind && d == detail && at.elapsed() < GRACE)
    }

    /// Requests still waiting (for a phone that opens the app late).
    pub fn pending(&self) -> Vec<Request> {
        let now = chrono::Utc::now().timestamp();
        self.pending.lock().values().map(|(r, _)| r.clone()).filter(|r| r.expires_at > now).collect()
    }

    /// A phone's answer; false when the request is gone.
    pub fn answer(&self, id: &str, approve: bool, device: &str) -> bool {
        match self.pending.lock().remove(id) {
            Some((_, tx)) => tx.send((approve, device.to_string())).is_ok(),
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn ask_and_answer() {
        let a = Arc::new(Approvals::default());
        // Approved from another thread (the phone).
        let a2 = a.clone();
        let r = a.ask(Kind::VaultReveal, "Gmail", Duration::from_secs(5), move |req| {
            assert_eq!(req.title, "Show a vault password on the PC");
            let (a3, id) = (a2.clone(), req.id.clone());
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(50));
                assert_eq!(a3.pending().len(), 1);
                assert!(a3.answer(&id, true, "Pixel 9"));
            });
        });
        assert_eq!(r.unwrap(), "Pixel 9");
        assert!(a.pending().is_empty());
        assert!(a.recently(Kind::VaultReveal, "Gmail") && !a.recently(Kind::VaultReveal, "Bank"));
        let a2 = a.clone();
        let r = a.ask(Kind::Shred, "x", Duration::from_secs(5), move |req| {
            let (a3, id) = (a2.clone(), req.id.clone());
            std::thread::spawn(move || {
                a3.answer(&id, false, "iPhone");
            });
        });
        assert_eq!(r.unwrap_err(), "Declined on iPhone.");
        assert!(a.ask(Kind::Admin, "", Duration::from_millis(50), |_| {}).unwrap_err().contains("in time"));
        assert!(!a.answer("gone", true, "x"));
    }
}
