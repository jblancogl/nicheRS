//! GBIF accounts of users and organisations (ADR-0110). A study's batch download uses, in order:
//! the account of the user who created it, that of their organisation, then the server's shared
//! account (`NICHE_GBIF_USERNAME`). An account GBIF refused at its last check is skipped.
//!
//! Passwords are sealed with the server key ([`crate::secrets`]); they never leave the server
//! and are never logged. The account chosen for a job is fixed when its preparation starts, so
//! the preparation and the compute worker use the same one (and find the same cached archive).

use rusqlite::{OptionalExtension, params};

use crate::AppError;
use crate::store::{Store, db, now};

/// Whose account.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GbifOwner {
    User(String),
    Org(String),
}

impl GbifOwner {
    /// Stored in `jobs.gbif_account` and bound to the sealed password (AEAD associated data).
    pub fn tag(&self) -> String {
        match self {
            Self::User(id) => format!("user:{id}"),
            Self::Org(id) => format!("org:{id}"),
        }
    }

    pub fn parse(tag: &str) -> Option<Self> {
        match tag.split_once(':') {
            Some(("user", id)) if !id.is_empty() => Some(Self::User(id.to_string())),
            Some(("org", id)) if !id.is_empty() => Some(Self::Org(id.to_string())),
            _ => None,
        }
    }

    fn table(&self) -> &'static str {
        match self {
            Self::User(_) => "users",
            Self::Org(_) => "orgs",
        }
    }

    fn id(&self) -> &str {
        match self {
            Self::User(id) | Self::Org(id) => id,
        }
    }
}

/// A stored account: the password stays sealed until the engine needs it.
#[derive(Clone, PartialEq, Eq)]
pub struct StoredGbif {
    pub username: String,
    pub sealed: String,
    pub checked_at: Option<String>,
    /// GBIF's answer at the last check (`None`: never checked, or GBIF unreachable).
    pub ok: Option<bool>,
}

impl std::fmt::Debug for StoredGbif {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StoredGbif")
            .field("username", &self.username)
            .field("sealed", &"<redacted>")
            .field("checked_at", &self.checked_at)
            .field("ok", &self.ok)
            .finish()
    }
}

/// Owner of a job, for choosing its GBIF account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobOwner {
    pub org_id: Option<String>,
    pub created_by: Option<String>,
    /// Account fixed at preparation (`user:<id>`, `org:<id>`, `shared`, `none`).
    pub gbif_account: Option<String>,
}

impl Store {
    pub fn gbif_get(&self, owner: &GbifOwner) -> Result<Option<StoredGbif>, AppError> {
        let sql = format!(
            "SELECT gbif_username, gbif_secret, gbif_checked_at, gbif_ok FROM {} WHERE id=?1",
            owner.table()
        );
        let row = self.with(|c| {
            c.query_row(&sql, [owner.id()], |r| {
                Ok((
                    r.get::<_, Option<String>>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, Option<i64>>(3)?,
                ))
            })
            .optional()
            .map_err(db)
        })?;
        Ok(match row {
            Some((Some(username), Some(sealed), checked_at, ok)) => Some(StoredGbif {
                username,
                sealed,
                checked_at,
                ok: ok.map(|v| v != 0),
            }),
            _ => None,
        })
    }

    /// Store (or replace) an account; its check starts again from "not checked".
    pub fn gbif_set(
        &self,
        owner: &GbifOwner,
        username: &str,
        sealed: &str,
    ) -> Result<(), AppError> {
        let sql = format!(
            "UPDATE {} SET gbif_username=?2, gbif_secret=?3, gbif_checked_at=NULL, gbif_ok=NULL WHERE id=?1",
            owner.table()
        );
        let n = self.with(|c| {
            c.execute(&sql, params![owner.id(), username, sealed])
                .map_err(db)
        })?;
        if n == 0 {
            return Err(AppError::NotFound("account".into()));
        }
        Ok(())
    }

    pub fn gbif_clear(&self, owner: &GbifOwner) -> Result<(), AppError> {
        let sql = format!(
            "UPDATE {} SET gbif_username=NULL, gbif_secret=NULL, gbif_checked_at=NULL, gbif_ok=NULL WHERE id=?1",
            owner.table()
        );
        self.with(|c| c.execute(&sql, [owner.id()]).map(|_| ()).map_err(db))
    }

    /// Record GBIF's answer (`None` when GBIF could not be reached: the account stays usable).
    pub fn gbif_mark_checked(&self, owner: &GbifOwner, ok: Option<bool>) -> Result<(), AppError> {
        let sql = format!(
            "UPDATE {} SET gbif_checked_at=?2, gbif_ok=?3 WHERE id=?1 AND gbif_username IS NOT NULL",
            owner.table()
        );
        self.with(|c| {
            c.execute(&sql, params![owner.id(), now(), ok.map(i64::from)])
                .map(|_| ())
                .map_err(db)
        })
    }

    pub fn job_owner(&self, job_id: &str) -> Result<Option<JobOwner>, AppError> {
        self.with(|c| {
            c.query_row(
                "SELECT org_id, created_by, gbif_account FROM jobs WHERE id=?1",
                [job_id],
                |r| {
                    Ok(JobOwner {
                        org_id: r.get(0)?,
                        created_by: r.get(1)?,
                        gbif_account: r.get(2)?,
                    })
                },
            )
            .optional()
            .map_err(db)
        })
    }

    pub fn set_job_gbif_account(&self, job_id: &str, tag: &str) -> Result<(), AppError> {
        self.with(|c| {
            c.execute(
                "UPDATE jobs SET gbif_account=?2 WHERE id=?1",
                params![job_id, tag],
            )
            .map(|_| ())
            .map_err(db)
        })
    }

    /// Queued jobs waiting for data preparation, oldest first.
    pub fn pending_preparation(&self, limit: usize) -> Result<Vec<String>, AppError> {
        self.with(|c| {
            let mut st = c
                .prepare("SELECT id FROM jobs WHERE status='queued' AND data_state='pending' ORDER BY created_at LIMIT ?1")
                .map_err(db)?;
            st.query_map([limit as i64], |r| r.get(0))
                .map_err(db)?
                .collect::<Result<Vec<String>, _>>()
                .map_err(db)
        })
    }

    /// Take one job for preparation; `false` if another thread took it or it left the queue.
    pub fn claim_preparation(&self, id: &str) -> Result<bool, AppError> {
        self.with(|c| {
            c.execute(
                "UPDATE jobs SET data_state='preparing', prep_started_at=?2
                 WHERE id=?1 AND status='queued' AND data_state='pending'",
                params![id, now()],
            )
            .map(|n| n == 1)
            .map_err(db)
        })
    }
}
