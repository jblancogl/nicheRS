//! Audit trail of the administration (ADR-0113): who changed which organisation or account, and
//! which administrator opened another organisation's study. One row per change, written by the
//! server (web) and by `niche admin` (CLI). Rows carry ids, never e-mail addresses: erasing an
//! account removes the id ↔ person mapping, so its past rows become anonymous but stay.

use rusqlite::{OptionalExtension, params};
use serde::Serialize;

use crate::AppError;
use crate::accounts::Principal;
use crate::store::{Store, db, now};

/// Entries returned by one page when the caller asks for none or too many.
pub const DEFAULT_PAGE: usize = 50;
pub const MAX_PAGE: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditAction {
    OrgCreated,
    OrgQuotaChanged,
    OrgDisabled,
    OrgEnabled,
    OperatorDesignated,
    InviteCreated,
    InviteRevoked,
    ResetLinkCreated,
    AccountDisabled,
    AccountEnabled,
    AccountDeleted,
    ManagerGranted,
    ManagerRemoved,
    AdminGranted,
    AdminRemoved,
    OrgGbifSet,
    OrgGbifRemoved,
    /// An administrator opened a study of another organisation (at most once per hour per pair).
    SupportAccess,
}

impl AuditAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OrgCreated => "org_created",
            Self::OrgQuotaChanged => "org_quota_changed",
            Self::OrgDisabled => "org_disabled",
            Self::OrgEnabled => "org_enabled",
            Self::OperatorDesignated => "operator_designated",
            Self::InviteCreated => "invite_created",
            Self::InviteRevoked => "invite_revoked",
            Self::ResetLinkCreated => "reset_link_created",
            Self::AccountDisabled => "account_disabled",
            Self::AccountEnabled => "account_enabled",
            Self::AccountDeleted => "account_deleted",
            Self::ManagerGranted => "manager_granted",
            Self::ManagerRemoved => "manager_removed",
            Self::AdminGranted => "admin_granted",
            Self::AdminRemoved => "admin_removed",
            Self::OrgGbifSet => "org_gbif_set",
            Self::OrgGbifRemoved => "org_gbif_removed",
            Self::SupportAccess => "support_access",
        }
    }
}

/// Where a change was made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Via {
    Web,
    /// `niche admin` on the server: no account, the operator.
    Cli,
}

/// One change to record. `org` is the organisation concerned (also for an account's change),
/// so an organisation's history includes what happened to its members.
#[derive(Debug, Clone, PartialEq)]
pub struct AuditEvent {
    pub actor_id: Option<String>,
    pub via: Via,
    pub action: AuditAction,
    pub org_id: Option<String>,
    pub user_id: Option<String>,
    pub job_id: Option<String>,
    /// Small facts of the change (e.g. `{"from": 20, "to": 50}`), never personal data.
    pub detail: Option<serde_json::Value>,
}

impl AuditEvent {
    pub fn web(actor: &Principal, action: AuditAction) -> Self {
        Self::new(Some(actor.user_id.clone()), Via::Web, action)
    }
    pub fn cli(action: AuditAction) -> Self {
        Self::new(None, Via::Cli, action)
    }
    fn new(actor_id: Option<String>, via: Via, action: AuditAction) -> Self {
        Self {
            actor_id,
            via,
            action,
            org_id: None,
            user_id: None,
            job_id: None,
            detail: None,
        }
    }
    pub fn org(mut self, id: impl Into<String>) -> Self {
        self.org_id = Some(id.into());
        self
    }
    /// The account changed, and its organisation.
    pub fn account(mut self, a: &AccountRef) -> Self {
        self.user_id = Some(a.user_id.clone());
        self.org_id = Some(a.org_id.clone());
        self
    }
    pub fn job(mut self, id: impl Into<String>) -> Self {
        self.job_id = Some(id.into());
        self
    }
    pub fn detail(mut self, v: serde_json::Value) -> Self {
        self.detail = Some(v);
        self
    }
}

/// An account as the audit trail refers to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountRef {
    pub user_id: String,
    pub org_id: String,
}

/// `GET /api/v1/admin/audit`: one recorded change, with the current names of what it refers to
/// (`None` once erased or deleted).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AuditEntry {
    pub id: i64,
    pub at: String,
    /// `web` or `cli`.
    pub via: String,
    /// Snake-case action (`AuditAction::as_str`); newer servers may add values.
    pub action: String,
    pub actor_id: Option<String>,
    pub actor_name: Option<String>,
    pub org_id: Option<String>,
    pub org_name: Option<String>,
    pub user_id: Option<String>,
    pub user_name: Option<String>,
    pub user_email: Option<String>,
    pub job_id: Option<String>,
    pub job_name: Option<String>,
    pub detail: Option<serde_json::Value>,
}

/// Newest first. `user` matches what was done to the account and what it did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AuditFilter {
    pub org: Option<String>,
    pub user: Option<String>,
    /// Entries older than this id (the `next` of the previous page).
    pub before: Option<i64>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AuditPage {
    pub entries: Vec<AuditEntry>,
    /// Pass as `before` for the next (older) page; `None` at the end.
    pub next: Option<i64>,
}

impl Store {
    pub fn audit(&self, e: &AuditEvent) -> Result<(), AppError> {
        let detail = e.detail.as_ref().map(serde_json::Value::to_string);
        let via = match e.via {
            Via::Web => "web",
            Via::Cli => "cli",
        };
        self.with(|c| {
            c.execute(
                "INSERT INTO audit(at,actor_id,via,action,org_id,user_id,job_id,detail)
                 VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
                params![
                    now(),
                    e.actor_id,
                    via,
                    e.action.as_str(),
                    e.org_id,
                    e.user_id,
                    e.job_id,
                    detail
                ],
            )
            .map(|_| ())
            .map_err(db)
        })
    }

    pub fn audit_list(&self, f: &AuditFilter) -> Result<AuditPage, AppError> {
        let limit = f.limit.unwrap_or(DEFAULT_PAGE).clamp(1, MAX_PAGE);
        let mut entries = self.with(|c| {
            let mut st = c
                .prepare(
                    "SELECT a.id, a.at, a.via, a.action, a.actor_id, ua.name, a.org_id, o.name,
                            a.user_id, u.name, u.email, a.job_id, j.name, a.detail
                     FROM audit a
                     LEFT JOIN users ua ON ua.id=a.actor_id
                     LEFT JOIN orgs o ON o.id=a.org_id
                     LEFT JOIN users u ON u.id=a.user_id
                     LEFT JOIN jobs j ON j.id=a.job_id
                     WHERE (?1 IS NULL OR a.org_id=?1)
                       AND (?2 IS NULL OR a.user_id=?2 OR a.actor_id=?2)
                       AND (?3 IS NULL OR a.id<?3)
                     ORDER BY a.id DESC LIMIT ?4",
                )
                .map_err(db)?;
            st.query_map(params![f.org, f.user, f.before, (limit + 1) as i64], |r| {
                let detail: Option<String> = r.get(13)?;
                Ok(AuditEntry {
                    id: r.get(0)?,
                    at: r.get(1)?,
                    via: r.get(2)?,
                    action: r.get(3)?,
                    actor_id: r.get(4)?,
                    actor_name: r.get(5)?,
                    org_id: r.get(6)?,
                    org_name: r.get(7)?,
                    user_id: r.get(8)?,
                    user_name: r.get(9)?,
                    user_email: r.get(10)?,
                    job_id: r.get(11)?,
                    job_name: r.get(12)?,
                    detail: detail.and_then(|d| serde_json::from_str(&d).ok()),
                })
            })
            .map_err(db)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(db)
        })?;
        let next = (entries.len() > limit).then(|| {
            entries.truncate(limit);
            entries.last().map(|e| e.id)
        });
        Ok(AuditPage {
            entries,
            next: next.flatten(),
        })
    }

    /// The account with this e-mail and its organisation (resolve before erasing it).
    pub fn account_ref(&self, email: &str) -> Result<AccountRef, AppError> {
        let email = email.trim().to_lowercase();
        self.with(|c| {
            c.query_row(
                "SELECT id, org_id FROM users WHERE email=?1",
                [&email],
                |r| {
                    Ok(AccountRef {
                        user_id: r.get(0)?,
                        org_id: r.get(1)?,
                    })
                },
            )
            .optional()
            .map_err(db)?
            .ok_or_else(|| AppError::NotFound(format!("account {email}")))
        })
    }

    /// Organisation of an invitation, by the id the console lists (its hash).
    pub fn invite_org(&self, id: &str) -> Result<Option<String>, AppError> {
        self.with(|c| {
            c.query_row(
                "SELECT org_id FROM links WHERE token_hash=?1 AND kind='invite'",
                [id],
                |r| r.get(0),
            )
            .optional()
            .map_err(db)
            .map(Option::flatten)
        })
    }
}
