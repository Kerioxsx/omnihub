//! Audit log of sensitive actions: power commands, file transfers, vault
//! reveals, app launches, pairing and remote control sessions.

use std::sync::Arc;

use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::db::{now, Db};
use crate::events::EventBus;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AuditEntry {
    pub id: i64,
    pub at: i64,
    pub actor: String,
    pub action: String,
    pub detail: String,
    pub ok: bool,
}

#[derive(Clone)]
pub struct Audit {
    db: Arc<Db>,
    events: EventBus,
}

/// Entries older than this are dropped.
const RETENTION_DAYS: i64 = 180;

impl Audit {
    pub fn new(db: Arc<Db>, events: EventBus) -> Self {
        let a = Audit { db, events };
        let _ = a.db.with(|c| c.execute("DELETE FROM audit WHERE at < ?1", params![now() - RETENTION_DAYS * 86_400]));
        a
    }

    pub fn record(&self, actor: &str, action: &str, detail: &str, ok: bool) {
        let at = now();
        let res = self.db.with(|c| {
            c.execute("INSERT INTO audit (at, actor, action, detail, ok) VALUES (?1, ?2, ?3, ?4, ?5)", params![at, actor, action, detail, ok])?;
            Ok(c.last_insert_rowid())
        });
        match res {
            Ok(id) => {
                tracing::info!(actor, action, detail, ok, "audit");
                self.events.emit("audit:new", AuditEntry { id, at, actor: actor.into(), action: action.into(), detail: detail.into(), ok });
            }
            Err(e) => tracing::error!("audit write failed: {e}"),
        }
    }

    pub fn list(&self, limit: usize, offset: usize) -> Vec<AuditEntry> {
        self.db
            .with(|c| {
                let mut st = c.prepare("SELECT id, at, actor, action, detail, ok FROM audit ORDER BY id DESC LIMIT ?1 OFFSET ?2")?;
                let rows = st.query_map(params![limit as i64, offset as i64], |r| {
                    Ok(AuditEntry { id: r.get(0)?, at: r.get(1)?, actor: r.get(2)?, action: r.get(3)?, detail: r.get(4)?, ok: r.get(5)? })
                })?;
                rows.collect()
            })
            .unwrap_or_default()
    }

    pub fn clear(&self) {
        let _ = self.db.with(|c| c.execute("DELETE FROM audit", []));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_and_lists_newest_first() {
        let a = Audit::new(Arc::new(Db::in_memory().unwrap()), EventBus::new());
        a.record("phone:Pixel", "power.shutdown", "in 10 s", true);
        a.record("desktop", "vault.reveal", "GitHub", true);
        let l = a.list(10, 0);
        assert_eq!(l.len(), 2);
        assert_eq!(l[0].action, "vault.reveal");
        assert_eq!(a.list(1, 1)[0].actor, "phone:Pixel");
    }
}
