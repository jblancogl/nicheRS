//! Hosted mode (ADR-0105): organisations, accounts entered through one-time invitation links,
//! sessions and the monthly quota of modelled species. NicheRS never sends e-mail: links are
//! printed by `niche admin` and handed over by Natural Solutions.
//!
//! Secrets (link and session tokens) are random 256-bit values; only their SHA-256 is stored.
//! Passwords are hashed with Argon2id.

use std::sync::{Condvar, Mutex, OnceLock};

use argon2::Argon2;
use argon2::password_hash::{PasswordHasher, PasswordVerifier};
use jiff::{SignedDuration, Timestamp};
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::AppError;
use crate::dto::{
    GbifWait, JobStatus, OrgMetrics, SpeciesRunStatus, SpeciesRunSummary, failure_cause,
};
use crate::store::{Store, db, now};

pub const INVITE_DAYS: i64 = 7;
pub const RESET_DAYS: i64 = 7;
pub const SESSION_DAYS: i64 = 30;
pub const MIN_PASSWORD_CHARS: usize = 12;
/// Organisation that becomes the operator by default (ADR-0112): migration and `niche admin org
/// create` when none is designated yet.
pub const DEFAULT_OPERATOR_ORG: &str = "Natural Solutions";

/// Which jobs a caller may see.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scope {
    /// Local mode and administration: every job.
    All,
    /// Hosted mode: the jobs of one organisation.
    Org(String),
}

/// What an account may do (ADR-0113). Stored as two flags (`users.is_admin`, `users.is_manager`,
/// also in the API); an administrator's manager flag does not matter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Member,
    /// Manager of their own organisation (ADR-0109): invites and manages its members.
    Manager,
    /// Natural Solutions administrator (ADR-0106): admin pages, read access to every job.
    Admin,
}

impl Role {
    pub fn from_flags(is_admin: bool, is_manager: bool) -> Self {
        match (is_admin, is_manager) {
            (true, _) => Self::Admin,
            (false, true) => Self::Manager,
            (false, false) => Self::Member,
        }
    }
}

/// The signed-in user of a request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Principal {
    pub user_id: String,
    pub email: String,
    pub name: String,
    pub org_id: String,
    pub org_name: String,
    pub monthly_quota: u64,
    pub role: Role,
    /// Interface language chosen by the account (`en`, `fr`, `es`), if any.
    pub lang: Option<String>,
}

impl Principal {
    pub fn is_admin(&self) -> bool {
        self.role == Role::Admin
    }
    /// May manage the members of their own organisation (managers; NS administrators too).
    pub fn manages_org(&self) -> bool {
        self.role >= Role::Manager
    }
    /// Jobs listed in the workspace: always the user's own organisation.
    pub fn scope(&self) -> Scope {
        Scope::Org(self.org_id.clone())
    }
    /// Jobs that may be opened: administrators may open any organisation's job.
    pub fn access(&self) -> Scope {
        if self.is_admin() {
            Scope::All
        } else {
            self.scope()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Org {
    pub id: String,
    pub name: String,
    pub monthly_quota: u64,
    pub created_at: String,
    pub disabled_at: Option<String>,
    /// The operator organisation (ADR-0112): only its members can be administrators.
    pub is_operator: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UserInfo {
    pub id: String,
    pub email: String,
    pub name: String,
    pub org_name: String,
    pub created_at: String,
    pub disabled_at: Option<String>,
    pub is_admin: bool,
    pub is_manager: bool,
    pub last_login_at: Option<String>,
    /// Member of the operator organisation (ADR-0112): may be made an administrator.
    pub in_operator_org: bool,
}

/// Accounts shown per page when the caller asks for none.
pub const USER_PAGE: usize = 100;

/// `org`: an organisation's id; `q`: part of a name, an e-mail or an organisation name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UserFilter {
    pub org: Option<String>,
    pub q: Option<String>,
    pub offset: Option<usize>,
    pub limit: Option<usize>,
}

/// `GET /api/v1/admin/users`: one page of accounts and how many match.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UserPage {
    pub users: Vec<UserInfo>,
    pub total: u64,
    /// Pass as `offset` for the next page; `None` at the end.
    pub next_offset: Option<u64>,
}

/// One organisation as the administration page lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OrgOverview {
    #[serde(flatten)]
    pub org: Org,
    pub quota: QuotaStatus,
    pub users: u64,
    pub runs: u64,
    pub open_invites: u64,
}

/// An unused, unexpired invitation. `id` is the stored hash: it identifies the link for
/// revocation but cannot be turned back into a usable link.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InviteInfo {
    pub id: String,
    pub created_at: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UsageSummary {
    pub org_id: String,
    pub org_name: String,
    pub month: String,
    pub species: u64,
    pub monthly_quota: u64,
}

/// A job across organisations (administration: server activity).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ActivityRow {
    pub id: String,
    pub name: String,
    pub org_id: Option<String>,
    pub org_name: Option<String>,
    pub created_by: Option<String>,
    pub status: JobStatus,
    /// `pending`, `preparing`, `ready` or `nobatch` (input data preparation).
    pub data_state: String,
    pub stage: String,
    pub progress: f64,
    pub species: u64,
    pub created_at: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub error: Option<String>,
    /// Failed studies: the kind of problem (see [`failure_cause`]).
    pub cause: Option<String>,
    /// Queued studies waiting for GBIF (filled from the engine's in-memory state).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gbif_wait: Option<GbifWait>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LinkKind {
    Invite,
    Reset,
}

/// What a valid link opens, shown before the person sets a password.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LinkInfo {
    pub kind: LinkKind,
    pub org_name: String,
    /// Reset links only: the account being reset.
    pub email: Option<String>,
    pub expires_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct QuotaStatus {
    /// Calendar month, UTC (`YYYY-MM`).
    pub month: String,
    pub total: u64,
    /// Species modelled this month (a model was fitted).
    pub used: u64,
    /// Species of queued or running jobs not finished yet.
    pub reserved: u64,
    /// `total − used − reserved`, never negative.
    pub remaining: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UsageRow {
    pub org_name: String,
    pub month: String,
    pub job_id: String,
    pub species_key: u64,
    pub at: String,
}

/// A new secret token (64 hex characters, 256 bits).
pub fn new_token() -> Result<String, AppError> {
    let mut b = [0u8; 32];
    getrandom::fill(&mut b).map_err(|e| AppError::Internal(format!("random source: {e}")))?;
    Ok(hex(&b))
}

/// What is stored for a token.
pub fn token_hash(token: &str) -> String {
    hex(&Sha256::digest(token.as_bytes()))
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// At most this many Argon2 computations at once (each holds ~19 MiB and one core), so a flood
/// of unauthenticated sign-in or invitation requests queues instead of exhausting memory.
const ARGON2_PERMITS: usize = 2;

static ARGON2_IN_USE: Mutex<usize> = Mutex::new(0);
static ARGON2_FREED: Condvar = Condvar::new();

/// One of the [`ARGON2_PERMITS`], given back on drop (also if hashing panicked).
struct Argon2Permit;

impl Drop for Argon2Permit {
    fn drop(&mut self) {
        if let Ok(mut n) = ARGON2_IN_USE.lock() {
            *n = n.saturating_sub(1);
        }
        ARGON2_FREED.notify_one();
    }
}

/// Runs `f` while holding an Argon2 permit, waiting for one if all are in use.
fn with_argon2_permit<T>(f: impl FnOnce() -> T) -> T {
    let permit = ARGON2_IN_USE.lock().ok().and_then(|mut n| {
        while *n >= ARGON2_PERMITS {
            n = ARGON2_FREED.wait(n).ok()?;
        }
        *n += 1;
        Some(Argon2Permit)
    });
    let out = f();
    drop(permit);
    out
}

pub fn hash_password(password: &str) -> Result<String, AppError> {
    check_password(password)?;
    with_argon2_permit(|| {
        Argon2::default()
            .hash_password(password.as_bytes())
            .map(|h| h.to_string())
            .map_err(|e| AppError::Internal(format!("password hashing: {e}")))
    })
}

fn verify_password(password: &str, hash: &str) -> bool {
    with_argon2_permit(|| {
        Argon2::default()
            .verify_password(password.as_bytes(), hash)
            .is_ok()
    })
}

/// Hash used when the account does not exist, so a login takes the same time either way.
fn dummy_hash() -> &'static str {
    static H: OnceLock<String> = OnceLock::new();
    H.get_or_init(|| {
        Argon2::default()
            .hash_password(b"nichers-timing-equaliser")
            .map(|h| h.to_string())
            .unwrap_or_default()
    })
}

fn check_password(password: &str) -> Result<(), AppError> {
    if password.chars().count() < MIN_PASSWORD_CHARS {
        return Err(AppError::Invalid(format!(
            "the password must have at least {MIN_PASSWORD_CHARS} characters"
        )));
    }
    Ok(())
}

fn normalise_email(email: &str) -> Result<String, AppError> {
    let e = email.trim().to_lowercase();
    let ok = e.len() <= 254
        && e.split_once('@')
            .is_some_and(|(l, d)| !l.is_empty() && d.contains('.') && !d.starts_with('.'))
        && !e.chars().any(char::is_whitespace);
    if ok {
        Ok(e)
    } else {
        Err(AppError::Invalid("enter a valid e-mail address".into()))
    }
}

fn later(days: i64) -> Result<String, AppError> {
    Timestamp::now()
        .checked_add(SignedDuration::from_hours(24 * days))
        .map(|t| t.to_string())
        .map_err(|e| AppError::Internal(e.to_string()))
}

fn is_past(ts: &str) -> bool {
    ts.parse::<Timestamp>()
        .map_or(true, |t| t <= Timestamp::now())
}

pub fn current_month() -> String {
    Timestamp::now().strftime("%Y-%m").to_string()
}

pub(crate) fn new_id() -> String {
    uuid::Uuid::now_v7().to_string()
}

/// Quota of an organisation, read on an open connection (used inside the job insert).
pub(crate) fn quota_status(
    c: &Connection,
    org_id: &str,
    total: u64,
) -> Result<QuotaStatus, AppError> {
    let month = current_month();
    let used: i64 = c
        .query_row(
            "SELECT COUNT(*) FROM usage WHERE org_id=?1 AND month=?2",
            params![org_id, month],
            |r| r.get(0),
        )
        .map_err(db)?;
    let mut st = c
        .prepare("SELECT status, species FROM jobs WHERE org_id=?1 AND status IN ('queued','running','interrupted')")
        .map_err(db)?;
    let rows = st
        .query_map([org_id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })
        .map_err(db)?;
    let mut reserved = 0u64;
    for r in rows {
        let (status, species) = r.map_err(db)?;
        reserved += reserved_by(&status, &species)?;
    }
    Ok(quota_of(month, total, used.max(0) as u64, reserved))
}

/// Species an unfinished job still reserves: those pending or running.
fn reserved_by(status: &str, species: &str) -> Result<u64, AppError> {
    if JobStatus::parse(status).is_terminal() {
        return Ok(0);
    }
    let sp: Vec<SpeciesRunSummary> =
        serde_json::from_str(species).map_err(|e| AppError::Storage(e.to_string()))?;
    Ok(sp
        .iter()
        .filter(|s| {
            matches!(
                s.status,
                SpeciesRunStatus::Pending | SpeciesRunStatus::Running
            )
        })
        .count() as u64)
}

fn quota_of(month: String, total: u64, used: u64, reserved: u64) -> QuotaStatus {
    QuotaStatus {
        month,
        total,
        used,
        reserved,
        remaining: total.saturating_sub(used + reserved),
    }
}

const USER_INFO: &str =
    "SELECT u.id, u.email, u.name, o.name, u.created_at, u.disabled_at, u.is_admin,
    u.last_login_at, u.is_manager, o.is_operator FROM users u JOIN orgs o ON o.id=u.org_id";

fn user_info(r: &rusqlite::Row<'_>) -> rusqlite::Result<UserInfo> {
    Ok(UserInfo {
        id: r.get(0)?,
        email: r.get(1)?,
        name: r.get(2)?,
        org_name: r.get(3)?,
        created_at: r.get(4)?,
        disabled_at: r.get(5)?,
        is_admin: r.get::<_, i64>(6)? != 0,
        is_manager: r.get::<_, i64>(8)? != 0,
        last_login_at: r.get(7)?,
        in_operator_org: r.get::<_, i64>(9)? != 0,
    })
}

const PRINCIPAL: &str =
    "SELECT u.id, u.email, u.name, o.id, o.name, o.monthly_quota, u.is_admin, u.lang, u.is_manager
    FROM users u JOIN orgs o ON o.id=u.org_id";

fn principal(r: &rusqlite::Row<'_>) -> rusqlite::Result<Principal> {
    Ok(Principal {
        user_id: r.get(0)?,
        email: r.get(1)?,
        name: r.get(2)?,
        org_id: r.get(3)?,
        org_name: r.get(4)?,
        monthly_quota: r.get::<_, i64>(5)?.max(0) as u64,
        role: Role::from_flags(r.get::<_, i64>(6)? != 0, r.get::<_, i64>(8)? != 0),
        lang: r.get(7)?,
    })
}

fn org_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Org> {
    Ok(Org {
        id: r.get(0)?,
        name: r.get(1)?,
        monthly_quota: r.get::<_, i64>(2)?.max(0) as u64,
        created_at: r.get(3)?,
        disabled_at: r.get(4)?,
        is_operator: r.get::<_, i64>(5)? != 0,
    })
}

fn insert_session(c: &Connection, user_id: &str) -> Result<String, AppError> {
    let token = new_token()?;
    c.execute(
        "INSERT INTO sessions(token_hash,user_id,created_at,expires_at) VALUES(?1,?2,?3,?4)",
        params![token_hash(&token), user_id, now(), later(SESSION_DAYS)?],
    )
    .map_err(db)?;
    c.execute(
        "UPDATE users SET last_login_at=?2 WHERE id=?1",
        params![user_id, now()],
    )
    .map_err(db)?;
    Ok(token)
}

/// A link row that can still be used (not used, not revoked, not expired).
struct OpenLink {
    kind: LinkKind,
    org_id: Option<String>,
    user_id: Option<String>,
    expires_at: String,
}

fn open_link(c: &Connection, token: &str) -> Result<OpenLink, AppError> {
    let invalid = || {
        AppError::Invalid(
            "this link is not valid any more (already used, revoked or expired); ask Natural Solutions for a new one"
                .into(),
        )
    };
    let row: Option<(String, Option<String>, Option<String>, String, Option<String>, Option<String>)> = c
        .query_row(
            "SELECT kind, org_id, user_id, expires_at, used_at, revoked_at FROM links WHERE token_hash=?1",
            [token_hash(token)],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
        )
        .optional()
        .map_err(db)?;
    let Some((kind, org_id, user_id, expires_at, used, revoked)) = row else {
        return Err(invalid());
    };
    if used.is_some() || revoked.is_some() || is_past(&expires_at) {
        return Err(invalid());
    }
    let kind = match kind.as_str() {
        "invite" => LinkKind::Invite,
        "reset" => LinkKind::Reset,
        _ => return Err(invalid()),
    };
    Ok(OpenLink {
        kind,
        org_id,
        user_id,
        expires_at,
    })
}

fn mark_used(c: &Connection, token: &str) -> Result<(), AppError> {
    let n = c
        .execute(
            "UPDATE links SET used_at=?2 WHERE token_hash=?1 AND used_at IS NULL",
            params![token_hash(token), now()],
        )
        .map_err(db)?;
    if n == 1 {
        Ok(())
    } else {
        Err(AppError::Invalid("this link has already been used".into()))
    }
}

fn org_active(c: &Connection, org_id: &str) -> Result<bool, AppError> {
    c.query_row(
        "SELECT disabled_at IS NULL FROM orgs WHERE id=?1",
        [org_id],
        |r| r.get(0),
    )
    .optional()
    .map_err(db)
    .map(|v| v.unwrap_or(false))
}

impl Store {
    // ---- organisations -------------------------------------------------------------------

    pub fn org_create(&self, name: &str, monthly_quota: u64) -> Result<Org, AppError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(AppError::Invalid("organisation name is empty".into()));
        }
        let id = new_id();
        self.with(|c| {
            c.execute(
                "INSERT INTO orgs(id,name,monthly_quota,created_at) VALUES(?1,?2,?3,?4)",
                params![id, name, monthly_quota as i64, now()],
            )
            .map_err(|e| match e {
                rusqlite::Error::SqliteFailure(f, _)
                    if f.code == rusqlite::ErrorCode::ConstraintViolation =>
                {
                    AppError::Conflict(format!("an organisation named {name} already exists"))
                }
                other => db(other),
            })?;
            Ok(())
        })?;
        self.org(name)
    }

    pub fn org_list(&self) -> Result<Vec<Org>, AppError> {
        self.with(|c| {
            let mut st = c
                .prepare(
                    "SELECT id,name,monthly_quota,created_at,disabled_at,is_operator FROM orgs ORDER BY name",
                )
                .map_err(db)?;
            st.query_map([], org_row)
                .map_err(db)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(db)
        })
    }

    /// Organisation by id or by name (case-insensitive). Every organisation method accepts both.
    pub fn org(&self, name: &str) -> Result<Org, AppError> {
        self.with(|c| {
            c.query_row(
                "SELECT id,name,monthly_quota,created_at,disabled_at,is_operator FROM orgs WHERE id=?1 OR name=?1",
                [name.trim()],
                org_row,
            )
            .optional()
            .map_err(db)?
            .ok_or_else(|| AppError::NotFound(format!("organisation {name}")))
        })
    }

    pub fn org_set_quota(&self, name: &str, monthly_quota: u64) -> Result<Org, AppError> {
        let org = self.org(name)?;
        self.with(|c| {
            c.execute(
                "UPDATE orgs SET monthly_quota=?2 WHERE id=?1",
                params![org.id, monthly_quota as i64],
            )
            .map(|_| ())
            .map_err(db)
        })?;
        self.org(name)
    }

    /// Disable or re-enable an organisation. Disabling signs out all of its users.
    pub fn org_set_disabled(&self, name: &str, disabled: bool) -> Result<Org, AppError> {
        let org = self.org(name)?;
        self.with(|c| {
            c.execute(
                "UPDATE orgs SET disabled_at=?2 WHERE id=?1",
                params![org.id, disabled.then(now)],
            )
            .map_err(db)?;
            if disabled {
                c.execute(
                    "DELETE FROM sessions WHERE user_id IN (SELECT id FROM users WHERE org_id=?1)",
                    [&org.id],
                )
                .map_err(db)?;
            }
            Ok(())
        })?;
        self.org(name)
    }

    /// The operator organisation (ADR-0112), if one has been designated.
    pub fn operator_org(&self) -> Result<Option<Org>, AppError> {
        self.with(|c| {
            c.query_row(
                "SELECT id,name,monthly_quota,created_at,disabled_at,is_operator FROM orgs WHERE is_operator=1",
                [],
                org_row,
            )
            .optional()
            .map_err(db)
        })
    }

    /// Designate the operator organisation (ADR-0112), replacing the previous one. Only the CLI
    /// calls this: whoever chooses it chooses who can become an administrator. Administrators
    /// outside the new one keep the flag and are reported (`admins_outside_operator`).
    pub fn org_set_operator(&self, name: &str) -> Result<Org, AppError> {
        let org = self.org(name)?;
        self.with(|c| {
            let tx = c.unchecked_transaction().map_err(db)?;
            tx.execute("UPDATE orgs SET is_operator=0 WHERE is_operator=1", [])
                .map_err(db)?;
            tx.execute("UPDATE orgs SET is_operator=1 WHERE id=?1", [&org.id])
                .map_err(db)?;
            tx.commit().map_err(db)
        })?;
        self.org(&org.id)
    }

    /// Give jobs created before hosted mode (no organisation) to `name`. Returns how many.
    pub fn org_adopt_unowned(&self, name: &str) -> Result<usize, AppError> {
        let org = self.org(name)?;
        self.with(|c| {
            c.execute("UPDATE jobs SET org_id=?1 WHERE org_id IS NULL", [&org.id])
                .map_err(db)
        })
    }

    pub fn quota(&self, org_id: &str, total: u64) -> Result<QuotaStatus, AppError> {
        self.with(|c| quota_status(c, org_id, total))
    }

    // ---- links ---------------------------------------------------------------------------

    /// One-time invitation into `org_name`. Returns the secret token (shown once).
    pub fn invite(&self, org_name: &str) -> Result<String, AppError> {
        let org = self.org(org_name)?;
        if org.disabled_at.is_some() {
            return Err(AppError::Invalid(format!(
                "organisation {} is disabled",
                org.name
            )));
        }
        let token = new_token()?;
        self.with(|c| {
            c.execute(
                "INSERT INTO links(token_hash,kind,org_id,created_at,expires_at) VALUES(?1,'invite',?2,?3,?4)",
                params![token_hash(&token), org.id, now(), later(INVITE_DAYS)?],
            )
            .map(|_| ())
            .map_err(db)
        })?;
        Ok(token)
    }

    /// One-time password reset link for an account. Earlier unused reset links are revoked.
    pub fn reset_link(&self, email: &str) -> Result<String, AppError> {
        let email = normalise_email(email)?;
        let token = new_token()?;
        self.with(|c| {
            let user_id: String = c
                .query_row("SELECT id FROM users WHERE email=?1", [&email], |r| r.get(0))
                .optional()
                .map_err(db)?
                .ok_or_else(|| AppError::NotFound(format!("account {email}")))?;
            c.execute(
                "UPDATE links SET revoked_at=?2 WHERE user_id=?1 AND kind='reset' AND used_at IS NULL AND revoked_at IS NULL",
                params![user_id, now()],
            )
            .map_err(db)?;
            c.execute(
                "INSERT INTO links(token_hash,kind,user_id,created_at,expires_at) VALUES(?1,'reset',?2,?3,?4)",
                params![token_hash(&token), user_id, now(), later(RESET_DAYS)?],
            )
            .map(|_| ())
            .map_err(db)
        })?;
        Ok(token)
    }

    /// Revoke every unused link of an organisation (invitations) — e.g. a link sent by mistake.
    pub fn revoke_invites(&self, org_name: &str) -> Result<usize, AppError> {
        let org = self.org(org_name)?;
        self.with(|c| {
            c.execute(
                "UPDATE links SET revoked_at=?2 WHERE org_id=?1 AND used_at IS NULL AND revoked_at IS NULL",
                params![org.id, now()],
            )
            .map_err(db)
        })
    }

    pub fn link_info(&self, token: &str) -> Result<LinkInfo, AppError> {
        self.with(|c| {
            let l = open_link(c, token)?;
            let (org_name, email) = match l.kind {
                LinkKind::Invite => {
                    let org_id = l.org_id.clone().unwrap_or_default();
                    if !org_active(c, &org_id)? {
                        return Err(AppError::Invalid("this organisation is disabled".into()));
                    }
                    let n: String = c
                        .query_row("SELECT name FROM orgs WHERE id=?1", [&org_id], |r| r.get(0))
                        .map_err(db)?;
                    (n, None)
                }
                LinkKind::Reset => {
                    let uid = l.user_id.clone().unwrap_or_default();
                    let (n, e): (String, String) = c
                        .query_row(
                            "SELECT o.name, u.email FROM users u JOIN orgs o ON o.id=u.org_id WHERE u.id=?1",
                            [&uid],
                            |r| Ok((r.get(0)?, r.get(1)?)),
                        )
                        .map_err(db)?;
                    (n, Some(e))
                }
            };
            Ok(LinkInfo {
                kind: l.kind,
                org_name,
                email,
                expires_at: l.expires_at,
            })
        })
    }

    /// Create the account of an invitation and open its first session.
    pub fn accept_invite(
        &self,
        token: &str,
        name: &str,
        email: &str,
        password: &str,
    ) -> Result<(Principal, String), AppError> {
        let email = normalise_email(email)?;
        let name = name.trim();
        if name.is_empty() {
            return Err(AppError::Invalid("enter your name".into()));
        }
        // Reject unknown or spent links before paying for a hash (checked again in the transaction).
        self.with(|c| open_link(c, token).map(|_| ()))?;
        let hash = hash_password(password)?;
        let user_id = new_id();
        let session = self.with(|c| {
            let tx = c.unchecked_transaction().map_err(db)?;
            let c = &*tx;
            let l = open_link(c, token)?;
            let org_id = match (l.kind, l.org_id) {
                (LinkKind::Invite, Some(o)) => o,
                _ => return Err(AppError::Invalid("this is not an invitation link".into())),
            };
            if !org_active(c, &org_id)? {
                return Err(AppError::Invalid("this organisation is disabled".into()));
            }
            let taken: bool = c
                .query_row("SELECT COUNT(*)>0 FROM users WHERE email=?1", [&email], |r| r.get(0))
                .map_err(db)?;
            if taken {
                return Err(AppError::Conflict(
                    "an account with this e-mail already exists; sign in instead".into(),
                ));
            }
            c.execute(
                "INSERT INTO users(id,org_id,email,name,password_hash,created_at) VALUES(?1,?2,?3,?4,?5,?6)",
                params![user_id, org_id, email, name, hash, now()],
            )
            .map_err(db)?;
            mark_used(c, token)?;
            let session = insert_session(c, &user_id)?;
            tx.commit().map_err(db)?;
            Ok(session)
        })?;
        let p = self
            .user_principal(&user_id)?
            .ok_or_else(|| AppError::Internal("account vanished".into()))?;
        Ok((p, session))
    }

    /// Set a new password from a reset link; every other session of the account is closed.
    pub fn reset_password(
        &self,
        token: &str,
        password: &str,
    ) -> Result<(Principal, String), AppError> {
        self.with(|c| open_link(c, token).map(|_| ()))?;
        let hash = hash_password(password)?;
        let (user_id, session) = self.with(|c| {
            let tx = c.unchecked_transaction().map_err(db)?;
            let c = &*tx;
            let l = open_link(c, token)?;
            let user_id = match (l.kind, l.user_id) {
                (LinkKind::Reset, Some(u)) => u,
                _ => {
                    return Err(AppError::Invalid(
                        "this is not a password reset link".into(),
                    ));
                }
            };
            c.execute(
                "UPDATE users SET password_hash=?2 WHERE id=?1",
                params![user_id, hash],
            )
            .map_err(db)?;
            c.execute("DELETE FROM sessions WHERE user_id=?1", [&user_id])
                .map_err(db)?;
            mark_used(c, token)?;
            let s = insert_session(c, &user_id)?;
            tx.commit().map_err(db)?;
            Ok((user_id, s))
        })?;
        let p = self
            .user_principal(&user_id)?
            .ok_or_else(|| AppError::Invalid("this account is disabled".into()))?;
        Ok((p, session))
    }

    /// E-mail + password → session token. `None` for any failure (no hint which part failed).
    pub fn login(
        &self,
        email: &str,
        password: &str,
    ) -> Result<Option<(Principal, String)>, AppError> {
        let Ok(email) = normalise_email(email) else {
            return Ok(None);
        };
        let found: Option<(String, String)> = self.with(|c| {
            c.query_row(
                "SELECT u.id, u.password_hash FROM users u JOIN orgs o ON o.id=u.org_id
                 WHERE u.email=?1 AND u.disabled_at IS NULL AND o.disabled_at IS NULL",
                [&email],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(db)
        })?;
        let (user_id, hash) = match found {
            Some(f) => f,
            None => {
                let _ = verify_password(password, dummy_hash());
                return Ok(None);
            }
        };
        if !verify_password(password, &hash) {
            return Ok(None);
        }
        let token = self.with(|c| insert_session(c, &user_id))?;
        Ok(self.user_principal(&user_id)?.map(|p| (p, token)))
    }

    /// The account of a session token, if the session is valid and the account active.
    pub fn session(&self, token: &str) -> Result<Option<Principal>, AppError> {
        let row: Option<(String, String)> = self.with(|c| {
            c.query_row(
                "SELECT user_id, expires_at FROM sessions WHERE token_hash=?1",
                [token_hash(token)],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(db)
        })?;
        match row {
            Some((user_id, expires)) if !is_past(&expires) => self.user_principal(&user_id),
            Some(_) => {
                self.logout(token)?;
                Ok(None)
            }
            None => Ok(None),
        }
    }

    pub fn logout(&self, token: &str) -> Result<(), AppError> {
        self.with(|c| {
            c.execute(
                "DELETE FROM sessions WHERE token_hash=?1",
                [token_hash(token)],
            )
            .map(|_| ())
            .map_err(db)
        })
    }

    /// Active account (user and organisation enabled).
    fn user_principal(&self, user_id: &str) -> Result<Option<Principal>, AppError> {
        self.with(|c| {
            c.query_row(
                &format!(
                    "{PRINCIPAL} WHERE u.id=?1 AND u.disabled_at IS NULL AND o.disabled_at IS NULL"
                ),
                [user_id],
                principal,
            )
            .optional()
            .map_err(db)
        })
    }

    // ---- users ---------------------------------------------------------------------------

    pub fn user_list(&self, org_name: Option<&str>) -> Result<Vec<UserInfo>, AppError> {
        let org = org_name.map(|n| self.org(n)).transpose()?.map(|o| o.id);
        Ok(self
            .user_search(&UserFilter {
                org,
                limit: Some(usize::MAX),
                ..UserFilter::default()
            })?
            .users)
    }

    /// Accounts by organisation and e-mail, filtered and one page at a time (ADR-0113).
    pub fn user_search(&self, f: &UserFilter) -> Result<UserPage, AppError> {
        let limit = f.limit.unwrap_or(USER_PAGE).max(1).min(i64::MAX as usize) as i64;
        let offset = f.offset.unwrap_or(0).min(i64::MAX as usize) as i64;
        let like =
            f.q.as_deref()
                .map(str::trim)
                .filter(|q| !q.is_empty())
                .map(|q| {
                    format!(
                        "%{}%",
                        q.replace('\\', "\\\\")
                            .replace('%', "\\%")
                            .replace('_', "\\_")
                    )
                });
        const WHERE: &str = "WHERE (?1 IS NULL OR u.org_id=?1)
            AND (?2 IS NULL OR u.name LIKE ?2 ESCAPE '\\' OR u.email LIKE ?2 ESCAPE '\\' OR o.name LIKE ?2 ESCAPE '\\')";
        self.with(|c| {
            let total: i64 = c
                .query_row(
                    &format!("SELECT COUNT(*) FROM users u JOIN orgs o ON o.id=u.org_id {WHERE}"),
                    params![f.org, like],
                    |r| r.get(0),
                )
                .map_err(db)?;
            let mut st = c
                .prepare(&format!(
                    "{USER_INFO} {WHERE} ORDER BY o.name, u.email LIMIT ?3 OFFSET ?4"
                ))
                .map_err(db)?;
            let users = st
                .query_map(params![f.org, like, limit, offset], user_info)
                .map_err(db)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(db)?;
            let total = total.max(0) as u64;
            let shown = offset as u64 + users.len() as u64;
            Ok(UserPage {
                users,
                total,
                next_offset: (shown < total).then_some(shown),
            })
        })
    }

    /// One account by id.
    pub fn user_info(&self, user_id: &str) -> Result<UserInfo, AppError> {
        self.with(|c| {
            c.query_row(&format!("{USER_INFO} WHERE u.id=?1"), [user_id], user_info)
                .optional()
                .map_err(db)?
                .ok_or_else(|| AppError::NotFound("account".into()))
        })
    }

    /// Disable or re-enable an account. Disabling closes its sessions immediately.
    pub fn user_set_disabled(&self, email: &str, disabled: bool) -> Result<(), AppError> {
        let email = normalise_email(email)?;
        self.with(|c| {
            let n = c
                .execute(
                    "UPDATE users SET disabled_at=?2 WHERE email=?1",
                    params![email, disabled.then(now)],
                )
                .map_err(db)?;
            if n == 0 {
                return Err(AppError::NotFound(format!("account {email}")));
            }
            if disabled {
                c.execute(
                    "DELETE FROM sessions WHERE user_id=(SELECT id FROM users WHERE email=?1)",
                    [&email],
                )
                .map_err(db)?;
            }
            Ok(())
        })
    }

    /// Interface languages an account can choose.
    pub const LANGS: [&'static str; 3] = ["en", "fr", "es"];

    /// Remember the account's interface language.
    pub fn user_set_lang(&self, user_id: &str, lang: &str) -> Result<(), AppError> {
        if !Self::LANGS.contains(&lang) {
            return Err(AppError::Invalid(format!(
                "unsupported language {lang:?} (en, fr or es)"
            )));
        }
        self.with(|c| {
            c.execute(
                "UPDATE users SET lang=?2 WHERE id=?1",
                params![user_id, lang],
            )
            .map_err(db)
            .map(|_| ())
        })
    }

    /// The account changes its own name (shown as author of studies and signing invitations).
    pub fn user_set_name(&self, user_id: &str, name: &str) -> Result<(), AppError> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 120 {
            return Err(AppError::Invalid(
                "enter your name (at most 120 characters)".into(),
            ));
        }
        self.with(|c| {
            c.execute(
                "UPDATE users SET name=?2 WHERE id=?1",
                params![user_id, name],
            )
            .map(|_| ())
            .map_err(db)
        })
    }

    /// The account changes its own password, proving it knows the current one. Every other
    /// session of the account is closed; the one making the change (`keep_session`) stays open.
    pub fn change_password(
        &self,
        user_id: &str,
        current: &str,
        new: &str,
        keep_session: &str,
    ) -> Result<(), AppError> {
        let hash: Option<String> = self.with(|c| {
            c.query_row(
                "SELECT password_hash FROM users WHERE id=?1",
                [user_id],
                |r| r.get(0),
            )
            .optional()
            .map_err(db)
        })?;
        let Some(hash) = hash else {
            return Err(AppError::NotFound("account".into()));
        };
        if !verify_password(current, &hash) {
            return Err(AppError::Invalid(
                "the current password is not correct".into(),
            ));
        }
        let new_hash = hash_password(new)?;
        self.with(|c| {
            let tx = c.unchecked_transaction().map_err(db)?;
            tx.execute(
                "UPDATE users SET password_hash=?2 WHERE id=?1",
                params![user_id, new_hash],
            )
            .map_err(db)?;
            tx.execute(
                "DELETE FROM sessions WHERE user_id=?1 AND token_hash<>?2",
                params![user_id, token_hash(keep_session)],
            )
            .map_err(db)?;
            tx.commit().map_err(db)
        })
    }

    /// Active members of an organisation.
    pub fn member_count(&self, org_id: &str) -> Result<u64, AppError> {
        self.with(|c| {
            c.query_row(
                "SELECT count(*) FROM users WHERE org_id=?1 AND disabled_at IS NULL",
                [org_id],
                |r| r.get::<_, i64>(0),
            )
            .map(|n| n.max(0) as u64)
            .map_err(db)
        })
    }

    /// Id of the account with this e-mail (for logs, which never carry e-mail addresses).
    pub fn user_id(&self, email: &str) -> Result<String, AppError> {
        let email = normalise_email(email)?;
        self.with(|c| {
            c.query_row("SELECT id FROM users WHERE email=?1", [&email], |r| {
                r.get(0)
            })
            .optional()
            .map_err(db)?
            .ok_or_else(|| AppError::NotFound(format!("account {email}")))
        })
    }

    /// Erase an account (GDPR right to erasure): its row (e-mail, name, password hash, last
    /// sign-in), sessions and reset links. The organisation's jobs, usage and feedback are kept;
    /// jobs and feedback it created lose their author. Returns the erased account's id.
    pub fn user_delete(&self, email: &str) -> Result<String, AppError> {
        let email = normalise_email(email)?;
        self.with(|c| {
            let tx = c.unchecked_transaction().map_err(db)?;
            let c = &*tx;
            let id: String = c
                .query_row("SELECT id FROM users WHERE email=?1", [&email], |r| {
                    r.get(0)
                })
                .optional()
                .map_err(db)?
                .ok_or_else(|| AppError::NotFound(format!("account {email}")))?;
            c.execute("DELETE FROM sessions WHERE user_id=?1", [&id])
                .map_err(db)?;
            c.execute("DELETE FROM links WHERE user_id=?1", [&id])
                .map_err(db)?;
            c.execute("UPDATE jobs SET created_by=NULL WHERE created_by=?1", [&id])
                .map_err(db)?;
            c.execute("UPDATE feedback SET user_id=NULL WHERE user_id=?1", [&id])
                .map_err(db)?;
            c.execute("DELETE FROM users WHERE id=?1", [&id])
                .map_err(db)?;
            tx.commit().map_err(db)?;
            Ok(id)
        })
    }

    // ---- usage ---------------------------------------------------------------------------

    /// Count one modelled species of a job against its organisation (idempotent; no-op for jobs
    /// without organisation).
    pub fn record_usage(&self, job_id: &str, species_key: u64) -> Result<(), AppError> {
        self.with(|c| {
            c.execute(
                "INSERT OR IGNORE INTO usage(org_id,job_id,species_key,month,at)
                 SELECT org_id, id, ?2, ?3, ?4 FROM jobs WHERE id=?1 AND org_id IS NOT NULL",
                params![job_id, species_key as i64, current_month(), now()],
            )
            .map(|_| ())
            .map_err(db)
        })
    }

    pub fn usage(
        &self,
        org_name: Option<&str>,
        month: Option<&str>,
    ) -> Result<Vec<UsageRow>, AppError> {
        let org_id = org_name.map(|n| self.org(n)).transpose()?.map(|o| o.id);
        self.with(|c| {
            let mut st = c
                .prepare(
                    "SELECT o.name, u.month, u.job_id, u.species_key, u.at
                     FROM usage u JOIN orgs o ON o.id=u.org_id
                     WHERE (?1 IS NULL OR u.org_id=?1) AND (?2 IS NULL OR u.month=?2)
                     ORDER BY o.name, u.at",
                )
                .map_err(db)?;
            st.query_map(params![org_id, month], |r| {
                Ok(UsageRow {
                    org_name: r.get(0)?,
                    month: r.get(1)?,
                    job_id: r.get(2)?,
                    species_key: r.get::<_, i64>(3)?.max(0) as u64,
                    at: r.get(4)?,
                })
            })
            .map_err(db)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(db)
        })
    }

    // ---- administration (ADR-0106) -------------------------------------------------------

    /// Grant or remove the administrator flag. Only members of the operator organisation can be
    /// granted it (ADR-0112); removing it is always possible. The web grants it after the
    /// administrator re-enters their password (`check_password_of`); only the CLI removes it.
    pub fn user_set_admin(&self, email: &str, admin: bool) -> Result<(), AppError> {
        let email = normalise_email(email)?;
        if admin {
            let operator = self.operator_org()?.ok_or_else(|| {
                AppError::Conflict(
                    "no operator organisation is designated (niche admin org operator <name>)"
                        .into(),
                )
            })?;
            let in_operator = self.with(|c| {
                c.query_row(
                    "SELECT org_id=?2 FROM users WHERE email=?1",
                    params![email, operator.id],
                    |r| r.get::<_, bool>(0),
                )
                .optional()
                .map_err(db)
            })?;
            match in_operator {
                None => return Err(AppError::NotFound(format!("account {email}"))),
                Some(false) => {
                    return Err(AppError::Conflict(format!(
                        "only members of {} can be administrators",
                        operator.name
                    )));
                }
                Some(true) => {}
            }
        }
        self.with(|c| {
            let n = c
                .execute(
                    "UPDATE users SET is_admin=?2 WHERE email=?1",
                    params![email, i64::from(admin)],
                )
                .map_err(db)?;
            if n == 0 {
                return Err(AppError::NotFound(format!("account {email}")));
            }
            Ok(())
        })
    }

    /// Whether `password` is the account's current password (re-authentication before a
    /// sensitive change from the web, ADR-0112).
    pub fn check_password_of(&self, user_id: &str, password: &str) -> Result<bool, AppError> {
        let hash: Option<String> = self.with(|c| {
            c.query_row(
                "SELECT password_hash FROM users WHERE id=?1",
                [user_id],
                |r| r.get(0),
            )
            .optional()
            .map_err(db)
        })?;
        Ok(hash.is_some_and(|h| verify_password(password, &h)))
    }

    /// Ids of the administrators outside the operator organisation (all of them when none is
    /// designated). They keep the flag; the server warns at start-up (ADR-0112).
    pub fn admins_outside_operator(&self) -> Result<Vec<String>, AppError> {
        self.with(|c| {
            let mut st = c
                .prepare(
                    "SELECT u.id FROM users u JOIN orgs o ON o.id=u.org_id
                     WHERE u.is_admin=1 AND o.is_operator=0 ORDER BY u.id",
                )
                .map_err(db)?;
            st.query_map([], |r| r.get(0))
                .map_err(db)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(db)
        })
    }

    pub(crate) fn warn_admins_outside_operator(&self) -> Result<(), AppError> {
        for user_id in self.admins_outside_operator()? {
            tracing::warn!(%user_id, "administrator outside the operator organisation (ADR-0112)");
        }
        Ok(())
    }

    /// Grant or remove the organisation-manager role (ADR-0109): from the console or the CLI.
    pub fn user_set_manager(&self, email: &str, manager: bool) -> Result<(), AppError> {
        let email = normalise_email(email)?;
        self.with(|c| {
            let n = c
                .execute(
                    "UPDATE users SET is_manager=?2 WHERE email=?1",
                    params![email, i64::from(manager)],
                )
                .map_err(db)?;
            if n == 0 {
                return Err(AppError::NotFound(format!("account {email}")));
            }
            Ok(())
        })
    }

    /// A manager may act on an account only inside their organisation, never on a Natural
    /// Solutions administrator (whose access spans every organisation) nor on their own account.
    /// Accounts of other organisations answer "not found", like their jobs.
    pub fn check_member(&self, manager: &Principal, email: &str) -> Result<(), AppError> {
        let email = normalise_email(email)?;
        let is_admin: i64 = self
            .with(|c| {
                c.query_row(
                    "SELECT org_id, is_admin FROM users WHERE email=?1",
                    [&email],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()
                .map_err(db)
            })?
            .filter(|(org, _): &(String, i64)| *org == manager.org_id)
            .map(|(_, admin)| admin)
            .ok_or_else(|| AppError::NotFound(format!("account {email}")))?;
        if email.eq_ignore_ascii_case(&manager.email) {
            return Err(AppError::Conflict("not on your own account".into()));
        }
        if is_admin != 0 && !manager.is_admin() {
            return Err(AppError::Forbidden(
                "Natural Solutions administrators are managed by Natural Solutions".into(),
            ));
        }
        Ok(())
    }

    /// Revoke one invitation of an organisation (managers: their own organisation only).
    pub fn revoke_link_in(&self, org_id: &str, id: &str) -> Result<(), AppError> {
        self.with(|c| {
            let n = c
                .execute(
                    "UPDATE links SET revoked_at=?3 WHERE token_hash=?1 AND org_id=?2 AND kind='invite'
                       AND used_at IS NULL AND revoked_at IS NULL",
                    params![id, org_id, now()],
                )
                .map_err(db)?;
            if n == 0 {
                return Err(AppError::NotFound("open invitation".into()));
            }
            Ok(())
        })
    }

    /// Every organisation with this month's quota, members, studies and open invitations, in a
    /// fixed number of queries whatever the number of organisations.
    pub fn org_overview(&self) -> Result<Vec<OrgOverview>, AppError> {
        self.org_overviews(None)
    }

    /// One organisation's overview, by id or name.
    pub fn org_overview_of(&self, org: &str) -> Result<OrgOverview, AppError> {
        let id = self.org(org)?.id;
        self.org_overviews(Some(&id))?
            .pop()
            .ok_or_else(|| AppError::NotFound(format!("organisation {org}")))
    }

    fn org_overviews(&self, only: Option<&str>) -> Result<Vec<OrgOverview>, AppError> {
        let month = current_month();
        self.with(|c| {
            let mut reserved: std::collections::HashMap<String, u64> = Default::default();
            let mut st = c
                .prepare(
                    "SELECT org_id, status, species FROM jobs
                     WHERE org_id IS NOT NULL AND status IN ('queued','running','interrupted')
                       AND (?1 IS NULL OR org_id=?1)",
                )
                .map_err(db)?;
            let rows = st
                .query_map([only], |r| {
                    Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?))
                })
                .map_err(db)?;
            for r in rows {
                let (org, status, species) = r.map_err(db)?;
                *reserved.entry(org).or_default() += reserved_by(&status, &species)?;
            }
            let mut st = c
                .prepare(
                    "SELECT o.id, o.name, o.monthly_quota, o.created_at, o.disabled_at, o.is_operator,
                            (SELECT COUNT(*) FROM users WHERE org_id=o.id),
                            (SELECT COUNT(*) FROM jobs WHERE org_id=o.id),
                            (SELECT COUNT(*) FROM links WHERE org_id=o.id AND kind='invite'
                               AND used_at IS NULL AND revoked_at IS NULL AND expires_at > ?2),
                            (SELECT COUNT(*) FROM usage WHERE org_id=o.id AND month=?3)
                     FROM orgs o WHERE ?1 IS NULL OR o.id=?1 ORDER BY o.name",
                )
                .map_err(db)?;
            let count = |r: &rusqlite::Row<'_>, i| r.get::<_, i64>(i).map(|n| n.max(0) as u64);
            st.query_map(params![only, now(), month], |r| {
                let org = org_row(r)?;
                let used = count(r, 9)?;
                let res = reserved.get(&org.id).copied().unwrap_or(0);
                Ok(OrgOverview {
                    quota: quota_of(month.clone(), org.monthly_quota, used, res),
                    users: count(r, 6)?,
                    runs: count(r, 7)?,
                    open_invites: count(r, 8)?,
                    org,
                })
            })
            .map_err(db)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(db)
        })
    }

    /// Unused, unexpired invitations of an organisation, newest first.
    pub fn open_invites(&self, org: &str) -> Result<Vec<InviteInfo>, AppError> {
        let org = self.org(org)?;
        self.with(|c| {
            let mut st = c
                .prepare(
                    "SELECT token_hash, created_at, expires_at FROM links
                     WHERE org_id=?1 AND kind='invite' AND used_at IS NULL AND revoked_at IS NULL
                     ORDER BY created_at DESC",
                )
                .map_err(db)?;
            let rows = st
                .query_map([&org.id], |r| {
                    Ok(InviteInfo {
                        id: r.get(0)?,
                        created_at: r.get(1)?,
                        expires_at: r.get(2)?,
                    })
                })
                .map_err(db)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(db)?;
            Ok(rows
                .into_iter()
                .filter(|i| !is_past(&i.expires_at))
                .collect())
        })
    }

    /// Revoke one invitation by its id (stored hash).
    pub fn revoke_link(&self, id: &str) -> Result<(), AppError> {
        self.with(|c| {
            let n = c
                .execute(
                    "UPDATE links SET revoked_at=?2 WHERE token_hash=?1 AND used_at IS NULL AND revoked_at IS NULL",
                    params![id, now()],
                )
                .map_err(db)?;
            if n == 0 {
                return Err(AppError::NotFound("open invitation".into()));
            }
            Ok(())
        })
    }

    /// Modelled species per organisation and month (all organisations, including those with
    /// nothing used in `month` when a month is given).
    pub fn usage_summary(&self, month: Option<&str>) -> Result<Vec<UsageSummary>, AppError> {
        self.with(|c| {
            let sql = match month {
                Some(_) => {
                    "SELECT o.id, o.name, ?1, COUNT(u.species_key), o.monthly_quota
                     FROM orgs o LEFT JOIN usage u ON u.org_id=o.id AND u.month=?1
                     GROUP BY o.id ORDER BY o.name"
                }
                None => {
                    "SELECT o.id, o.name, u.month, COUNT(*), o.monthly_quota
                     FROM usage u JOIN orgs o ON o.id=u.org_id
                     WHERE ?1 IS NULL GROUP BY o.id, u.month ORDER BY u.month DESC, o.name"
                }
            };
            let mut st = c.prepare(sql).map_err(db)?;
            st.query_map([month], |r| {
                Ok(UsageSummary {
                    org_id: r.get(0)?,
                    org_name: r.get(1)?,
                    month: r.get(2)?,
                    species: r.get::<_, i64>(3)?.max(0) as u64,
                    monthly_quota: r.get::<_, i64>(4)?.max(0) as u64,
                })
            })
            .map_err(db)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(db)
        })
    }

    /// Jobs of every organisation: all unfinished ones, then the `recent` most recently created
    /// finished ones.
    /// Per organisation, the studies created in `month` (`YYYY-MM`): outcomes, computing time,
    /// GBIF waiting time and failure causes. Studies without organisation (local mode, before
    /// accounts) are grouped under "—".
    pub fn metrics(&self, month: &str) -> Result<Vec<OrgMetrics>, AppError> {
        type Row = (
            Option<String>,
            String,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            String,
            Option<String>,
        );
        let rows: Vec<Row> = self.with(|c| {
            let mut st = c
                .prepare(
                    "SELECT o.name, j.status, j.started_at, j.finished_at, j.prep_started_at,
                            j.prep_finished_at, j.species, j.error
                     FROM jobs j LEFT JOIN orgs o ON o.id=j.org_id
                     WHERE substr(j.created_at,1,7)=?1",
                )
                .map_err(db)?;
            st.query_map([month], |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                    r.get(7)?,
                ))
            })
            .map_err(db)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(db)
        })?;
        let secs = |a: &Option<String>, b: &Option<String>| -> Option<f64> {
            let a = a.as_deref()?.parse::<Timestamp>().ok()?;
            let b = b.as_deref()?.parse::<Timestamp>().ok()?;
            Some(b.duration_since(a).as_secs_f64().max(0.0))
        };
        let mut by_org: std::collections::BTreeMap<String, (OrgMetrics, f64, u64, Vec<f64>)> =
            std::collections::BTreeMap::new();
        for (org, status, started, finished, prep0, prep1, species, error) in rows {
            let name = org.unwrap_or_else(|| "—".into());
            let entry = by_org.entry(name.clone()).or_insert_with(|| {
                (
                    OrgMetrics {
                        org_name: name,
                        month: month.to_string(),
                        studies: 0,
                        succeeded: 0,
                        failed: 0,
                        cancelled: 0,
                        species: 0,
                        compute_s: 0.0,
                        compute_s_per_species: None,
                        gbif_wait_s_mean: None,
                        gbif_wait_s_max: None,
                        failures: Default::default(),
                    },
                    0.0,
                    0,
                    Vec::new(),
                )
            });
            let (m, ok_s, ok_species, waits) = entry;
            let n_species = serde_json::from_str::<Vec<serde_json::Value>>(&species)
                .map_or(0, |v| v.len() as u64);
            m.studies += 1;
            m.species += n_species;
            let compute = secs(&started, &finished);
            m.compute_s += compute.unwrap_or(0.0);
            match status.as_str() {
                "succeeded" => {
                    m.succeeded += 1;
                    *ok_s += compute.unwrap_or(0.0);
                    *ok_species += n_species;
                }
                "failed" => {
                    m.failed += 1;
                    *m.failures
                        .entry(failure_cause(error.as_deref().unwrap_or("")).to_string())
                        .or_insert(0) += 1;
                }
                "cancelled" => m.cancelled += 1,
                _ => {}
            }
            // A preparation without download (no GBIF account, cached archive) takes
            // milliseconds: not a GBIF wait.
            if let Some(w) = secs(&prep0, &prep1).filter(|w| *w >= 1.0) {
                waits.push(w);
            }
        }
        Ok(by_org
            .into_values()
            .map(|(mut m, ok_s, ok_species, waits)| {
                m.compute_s_per_species = (ok_species > 0).then(|| ok_s / ok_species as f64);
                if !waits.is_empty() {
                    m.gbif_wait_s_mean = Some(waits.iter().sum::<f64>() / waits.len() as f64);
                    m.gbif_wait_s_max = waits.iter().copied().reduce(f64::max);
                }
                m
            })
            .collect())
    }

    /// Unfinished studies and the `recent` most recent finished ones, of every organisation or of
    /// one (`org`: its id).
    pub fn activity(&self, recent: usize, org: Option<&str>) -> Result<Vec<ActivityRow>, AppError> {
        self.with(|c| {
            let mut st = c
                .prepare(
                    "SELECT * FROM (
                        SELECT j.id, j.name, o.name, u.email, j.status, j.data_state, j.stage, j.progress,
                               j.species, j.created_at, j.started_at, j.finished_at, j.error, j.org_id
                        FROM jobs j LEFT JOIN orgs o ON o.id=j.org_id LEFT JOIN users u ON u.id=j.created_by
                        WHERE j.status IN ('queued','running','interrupted') AND (?2 IS NULL OR j.org_id=?2)
                        UNION ALL
                        SELECT * FROM (
                          SELECT j.id, j.name, o.name, u.email, j.status, j.data_state, j.stage, j.progress,
                                 j.species, j.created_at, j.started_at, j.finished_at, j.error, j.org_id
                          FROM jobs j LEFT JOIN orgs o ON o.id=j.org_id LEFT JOIN users u ON u.id=j.created_by
                          WHERE j.status NOT IN ('queued','running','interrupted') AND (?2 IS NULL OR j.org_id=?2)
                          ORDER BY j.created_at DESC LIMIT ?1)
                     ) ORDER BY created_at DESC",
                )
                .map_err(db)?;
            let rows = st
                .query_map(params![recent as i64, org], |r| {
                    let species: String = r.get(8)?;
                    Ok(ActivityRow {
                        id: r.get(0)?,
                        name: r.get(1)?,
                        org_id: r.get(13)?,
                        org_name: r.get(2)?,
                        created_by: r.get(3)?,
                        status: JobStatus::parse(&r.get::<_, String>(4)?),
                        data_state: r.get(5)?,
                        stage: r.get(6)?,
                        progress: r.get(7)?,
                        species: serde_json::from_str::<Vec<serde_json::Value>>(&species)
                            .map_or(0, |v| v.len() as u64),
                        created_at: r.get(9)?,
                        started_at: r.get(10)?,
                        finished_at: r.get(11)?,
                        cause: (r.get::<_, String>(4)? == "failed").then(|| {
                            failure_cause(&r.get::<_, Option<String>>(12).ok().flatten().unwrap_or_default())
                                .to_string()
                        }),
                        error: r.get(12)?,
                        gbif_wait: None,
                    })
                })
                .map_err(db)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(db)?;
            Ok(rows)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn argon2_runs_at_most_two_at_once() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static NOW: AtomicUsize = AtomicUsize::new(0);
        static PEAK: AtomicUsize = AtomicUsize::new(0);
        let threads: Vec<_> = (0..8)
            .map(|_| {
                std::thread::spawn(|| {
                    with_argon2_permit(|| {
                        let n = NOW.fetch_add(1, Ordering::SeqCst) + 1;
                        PEAK.fetch_max(n, Ordering::SeqCst);
                        std::thread::sleep(std::time::Duration::from_millis(20));
                        NOW.fetch_sub(1, Ordering::SeqCst);
                    })
                })
            })
            .collect();
        for t in threads {
            assert!(t.join().is_ok());
        }
        // Permits are process-wide: other tests hashing in parallel may hold some of them.
        let peak = PEAK.load(Ordering::SeqCst);
        assert!((1..=ARGON2_PERMITS).contains(&peak), "peak {peak}");
    }

    #[test]
    fn tokens_are_random_and_hashed() {
        let a = new_token().expect("token");
        let b = new_token().expect("token");
        assert_eq!(a.len(), 64);
        assert_ne!(a, b);
        assert_eq!(token_hash(&a), token_hash(&a));
        assert_ne!(token_hash(&a), a);
    }

    #[test]
    fn passwords() {
        assert!(hash_password("short").is_err());
        let h = hash_password("correct horse battery").expect("hash");
        assert!(h.starts_with("$argon2id$"));
        assert!(verify_password("correct horse battery", &h));
        assert!(!verify_password("correct horse batterY", &h));
    }

    #[test]
    fn emails() {
        assert_eq!(
            normalise_email(" Alice@Bureau-X.fr ").expect("ok"),
            "alice@bureau-x.fr"
        );
        for bad in ["", "alice", "@x.fr", "a@b", "a b@c.fr", "a@.fr"] {
            assert!(normalise_email(bad).is_err(), "{bad}");
        }
    }
}
