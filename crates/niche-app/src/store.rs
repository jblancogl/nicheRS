//! Durable job state in SQLite (WAL, synchronous=FULL; ADR-0008) and job events.

use std::path::Path;
use std::sync::Mutex;

use rusqlite::{Connection, OptionalExtension, params};

use crate::AppError;
use crate::accounts::{Principal, Scope, quota_status};
use crate::dto::{Job, JobEvent, JobSpec, JobStatus, SpeciesRunStatus, SpeciesRunSummary};

pub const SCHEMA_VERSION: i64 = 11;

/// v2: organisations, accounts, one-time links, sessions, usage ledger (hosted mode, ADR-0105),
/// job ownership and the data-preparation state that keeps GBIF waits off the compute worker.
const MIGRATE_V1_TO_V2: &str = "
    ALTER TABLE jobs ADD COLUMN org_id TEXT;
    ALTER TABLE jobs ADD COLUMN created_by TEXT;
    ALTER TABLE jobs ADD COLUMN data_state TEXT NOT NULL DEFAULT 'ready';
    CREATE INDEX jobs_org ON jobs(org_id, created_at);
    CREATE TABLE orgs(
        id TEXT PRIMARY KEY, name TEXT NOT NULL UNIQUE COLLATE NOCASE,
        monthly_quota INTEGER NOT NULL, created_at TEXT NOT NULL, disabled_at TEXT);
    CREATE TABLE users(
        id TEXT PRIMARY KEY, org_id TEXT NOT NULL REFERENCES orgs(id),
        email TEXT NOT NULL UNIQUE COLLATE NOCASE, name TEXT NOT NULL,
        password_hash TEXT NOT NULL, created_at TEXT NOT NULL, disabled_at TEXT);
    CREATE TABLE links(
        token_hash TEXT PRIMARY KEY, kind TEXT NOT NULL,
        org_id TEXT REFERENCES orgs(id), user_id TEXT REFERENCES users(id),
        created_at TEXT NOT NULL, expires_at TEXT NOT NULL, used_at TEXT, revoked_at TEXT);
    CREATE TABLE sessions(
        token_hash TEXT PRIMARY KEY,
        user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
        created_at TEXT NOT NULL, expires_at TEXT NOT NULL);
    CREATE TABLE usage(
        org_id TEXT NOT NULL, job_id TEXT NOT NULL, species_key INTEGER NOT NULL,
        month TEXT NOT NULL, at TEXT NOT NULL, PRIMARY KEY(job_id, species_key));
    CREATE INDEX usage_org_month ON usage(org_id, month);
";

/// v3: Natural Solutions administrators (flag set from the CLI only) and last sign-in (ADR-0106).
const MIGRATE_V2_TO_V3: &str = "
    ALTER TABLE users ADD COLUMN is_admin INTEGER NOT NULL DEFAULT 0;
    ALTER TABLE users ADD COLUMN last_login_at TEXT;
";

/// v4: interface language chosen by each account (`en`, `fr`, `es`; NULL = browser default).
const MIGRATE_V3_TO_V4: &str = "
    ALTER TABLE users ADD COLUMN lang TEXT;
";

/// v5: projects grouping an organisation's studies (local mode: projects without organisation).
/// Deleting a project leaves its studies without project.
const MIGRATE_V4_TO_V5: &str = "
    CREATE TABLE projects(
        id TEXT PRIMARY KEY, org_id TEXT REFERENCES orgs(id), name TEXT NOT NULL,
        created_at TEXT NOT NULL, created_by TEXT);
    CREATE UNIQUE INDEX projects_org_name ON projects(ifnull(org_id, ''), name COLLATE NOCASE);
    ALTER TABLE jobs ADD COLUMN project_id TEXT REFERENCES projects(id) ON DELETE SET NULL;
    CREATE INDEX jobs_project ON jobs(project_id);
";

/// v6: organisation managers (invite and manage the members of their own organisation,
/// ADR-0109) and when each job's data preparation (GBIF download) started and ended.
const MIGRATE_V5_TO_V6: &str = "
    ALTER TABLE users ADD COLUMN is_manager INTEGER NOT NULL DEFAULT 0;
    ALTER TABLE jobs ADD COLUMN prep_started_at TEXT;
    ALTER TABLE jobs ADD COLUMN prep_finished_at TEXT;
";

/// v7: GBIF accounts of users and organisations (username, password sealed with the server key,
/// last check; ADR-0110) and the account each job's download uses (`user:<id>`, `org:<id>`,
/// `shared`, `none`), fixed when its data preparation starts.
const MIGRATE_V6_TO_V7: &str = "
    ALTER TABLE users ADD COLUMN gbif_username TEXT;
    ALTER TABLE users ADD COLUMN gbif_secret TEXT;
    ALTER TABLE users ADD COLUMN gbif_checked_at TEXT;
    ALTER TABLE users ADD COLUMN gbif_ok INTEGER;
    ALTER TABLE orgs ADD COLUMN gbif_username TEXT;
    ALTER TABLE orgs ADD COLUMN gbif_secret TEXT;
    ALTER TABLE orgs ADD COLUMN gbif_checked_at TEXT;
    ALTER TABLE orgs ADD COLUMN gbif_ok INTEGER;
    ALTER TABLE jobs ADD COLUMN gbif_account TEXT;
";

/// v8: feedback from users with its screenshots and voice recordings (ADR-0111). `forwarded_at`
/// and `forward_ref` record the copy sent to the operator's issue tracker; `forwarding_at`
/// claims an item while it is being sent.
const MIGRATE_V7_TO_V8: &str = "
    CREATE TABLE feedback(
        id TEXT PRIMARY KEY, org_id TEXT REFERENCES orgs(id), user_id TEXT,
        kind TEXT NOT NULL, message TEXT NOT NULL, page TEXT, context TEXT NOT NULL,
        created_at TEXT NOT NULL, status TEXT NOT NULL DEFAULT 'open',
        forwarding_at TEXT, forwarded_at TEXT, forward_ref TEXT, forward_error TEXT);
    CREATE INDEX feedback_org ON feedback(org_id, created_at);
    CREATE TABLE feedback_files(
        id TEXT PRIMARY KEY,
        feedback_id TEXT NOT NULL REFERENCES feedback(id) ON DELETE CASCADE,
        seq INTEGER NOT NULL, name TEXT NOT NULL, mime TEXT NOT NULL, size INTEGER NOT NULL,
        data BLOB NOT NULL);
    CREATE INDEX feedback_files_of ON feedback_files(feedback_id, seq);
";

/// v9: the operator organisation (ADR-0112), the only one whose members can be administrators;
/// at most one. Existing databases: the organisation named Natural Solutions, otherwise the one
/// every current administrator belongs to, otherwise none (designate it with the CLI).
const MIGRATE_V8_TO_V9: &str = "
    ALTER TABLE orgs ADD COLUMN is_operator INTEGER NOT NULL DEFAULT 0;
    CREATE UNIQUE INDEX orgs_operator ON orgs(is_operator) WHERE is_operator=1;
    UPDATE orgs SET is_operator=1 WHERE name='Natural Solutions';
    UPDATE orgs SET is_operator=1
     WHERE NOT EXISTS (SELECT 1 FROM orgs WHERE is_operator=1)
       AND (SELECT COUNT(DISTINCT org_id) FROM users WHERE is_admin=1)=1
       AND id=(SELECT org_id FROM users WHERE is_admin=1 LIMIT 1);
";

/// v10: audit trail of the administration (ADR-0113). No foreign keys: rows outlive the
/// accounts, organisations and studies they mention.
const MIGRATE_V9_TO_V10: &str = "
    CREATE TABLE audit(
        id INTEGER PRIMARY KEY AUTOINCREMENT, at TEXT NOT NULL, actor_id TEXT,
        via TEXT NOT NULL, action TEXT NOT NULL,
        org_id TEXT, user_id TEXT, job_id TEXT, detail TEXT);
    CREATE INDEX audit_org ON audit(org_id, id);
    CREATE INDEX audit_user ON audit(user_id, id);
    CREATE INDEX audit_actor ON audit(actor_id, id);
";

/// v11: datasets of occurrence records imported by organisations (ADR-0108). `info` is the
/// dataset description and import report as JSON (counts, never records); the records are in
/// `<data_dir>/datasets/<id>/`. No foreign keys: the creator may be erased later.
const MIGRATE_V10_TO_V11: &str = "
    CREATE TABLE IF NOT EXISTS datasets(
        id TEXT PRIMARY KEY, org_id TEXT, created_by TEXT, created_at TEXT NOT NULL,
        info TEXT NOT NULL);
    CREATE INDEX IF NOT EXISTS datasets_org ON datasets(org_id, created_at);
";

pub struct Store {
    conn: Mutex<Connection>,
}

pub(crate) fn now() -> String {
    jiff::Timestamp::now().to_string()
}

pub(crate) fn db(e: rusqlite::Error) -> AppError {
    AppError::Storage(e.to_string())
}

/// `VACUUM INTO <dir of the database>/backups/niche-premigrate-v<from>-<time>.db`, listed and
/// pruned with the daily copies (`system`). A failed copy stops the migration.
fn pre_migration_copy(conn: &Connection, path: &Path, from: i64) -> Result<(), AppError> {
    let dir = crate::system::backups_dir(path.parent().unwrap_or(Path::new(".")));
    std::fs::create_dir_all(&dir).map_err(|e| AppError::Storage(e.to_string()))?;
    let stamp = jiff::Timestamp::now().strftime("%Y%m%d-%H%M%S").to_string();
    let dest = dir.join(format!("niche-premigrate-v{from}-{stamp}.db"));
    let tmp = dir.join(format!(".niche-premigrate-{stamp}.tmp"));
    let _ = std::fs::remove_file(&tmp);
    let target = tmp
        .to_str()
        .ok_or_else(|| AppError::Storage("backup path is not UTF-8".into()))?;
    conn.execute("VACUUM INTO ?1", [target]).map_err(db)?;
    std::fs::rename(&tmp, &dest).map_err(|e| AppError::Storage(e.to_string()))?;
    tracing::info!(file = %dest.display(), from, to = SCHEMA_VERSION, "database copied before migration");
    Ok(())
}

impl Store {
    pub fn open(path: &Path) -> Result<Self, AppError> {
        let mut conn = Connection::open(path).map_err(db)?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000;
             CREATE TABLE IF NOT EXISTS meta(key TEXT PRIMARY KEY, value TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS jobs(
                id TEXT PRIMARY KEY, name TEXT NOT NULL, status TEXT NOT NULL,
                created_at TEXT NOT NULL, started_at TEXT, finished_at TEXT,
                progress REAL NOT NULL DEFAULT 0, stage TEXT NOT NULL DEFAULT '',
                spec TEXT NOT NULL, species TEXT NOT NULL, error TEXT, spec_hash TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS events(
                job_id TEXT NOT NULL REFERENCES jobs(id) ON DELETE CASCADE, seq INTEGER NOT NULL,
                body TEXT NOT NULL, PRIMARY KEY(job_id, seq));",
        )
        .map_err(db)?;
        let v: Option<String> = conn
            .query_row(
                "SELECT value FROM meta WHERE key='schema_version'",
                [],
                |r| r.get(0),
            )
            .optional()
            .map_err(db)?;
        // A fresh database starts from the v1 tables above and is migrated like an old one.
        let version = v
            .as_deref()
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(1);
        if version > SCHEMA_VERSION {
            return Err(AppError::Storage(format!(
                "database schema v{version} is newer than this NicheRS (v{SCHEMA_VERSION})"
            )));
        }
        if version < SCHEMA_VERSION {
            // An existing database is copied before it is migrated, so a release can go out
            // without a manual volume backup: rollback = the old image with this copy.
            if v.is_some() {
                pre_migration_copy(&conn, path, version)?;
            }
            let tx = conn.transaction().map_err(db)?;
            if version < 2 {
                tx.execute_batch(MIGRATE_V1_TO_V2).map_err(db)?;
            }
            if version < 3 {
                tx.execute_batch(MIGRATE_V2_TO_V3).map_err(db)?;
            }
            if version < 4 {
                tx.execute_batch(MIGRATE_V3_TO_V4).map_err(db)?;
            }
            if version < 5 {
                tx.execute_batch(MIGRATE_V4_TO_V5).map_err(db)?;
            }
            if version < 6 {
                tx.execute_batch(MIGRATE_V5_TO_V6).map_err(db)?;
            }
            if version < 7 {
                tx.execute_batch(MIGRATE_V6_TO_V7).map_err(db)?;
            }
            if version < 8 {
                tx.execute_batch(MIGRATE_V7_TO_V8).map_err(db)?;
            }
            if version < 9 {
                tx.execute_batch(MIGRATE_V8_TO_V9).map_err(db)?;
            }
            if version < 10 {
                tx.execute_batch(MIGRATE_V9_TO_V10).map_err(db)?;
            }
            tx.execute_batch(MIGRATE_V10_TO_V11).map_err(db)?;
            tx.execute(
                "INSERT INTO meta(key,value) VALUES('schema_version',?1)
                 ON CONFLICT(key) DO UPDATE SET value=excluded.value",
                [SCHEMA_VERSION.to_string()],
            )
            .map_err(db)?;
            tx.commit().map_err(db)?;
        }
        let store = Self {
            conn: Mutex::new(conn),
        };
        store.warn_admins_outside_operator()?;
        Ok(store)
    }

    pub(crate) fn with<T>(
        &self,
        f: impl FnOnce(&Connection) -> Result<T, AppError>,
    ) -> Result<T, AppError> {
        let c = self
            .conn
            .lock()
            .map_err(|_| AppError::Storage("database lock poisoned".into()))?;
        f(&c)
    }

    /// Insert a queued job. With an owner, the monthly quota is checked under the same database
    /// lock as the insert, so concurrent submissions cannot overshoot it (ADR-0105).
    pub fn create(
        &self,
        id: &str,
        spec: &JobSpec,
        spec_hash: &str,
        owner: Option<&Principal>,
    ) -> Result<Job, AppError> {
        let species: Vec<SpeciesRunSummary> = spec
            .species
            .iter()
            .map(|s| SpeciesRunSummary {
                key: s.key,
                scientific_name: s.scientific_name.clone(),
                status: SpeciesRunStatus::Pending,
                evidence_level: None,
                error: None,
            })
            .collect();
        let name = if spec.name.trim().is_empty() {
            format!(
                "{} — {}",
                spec.species
                    .first()
                    .map_or("?", |s| s.scientific_name.as_str()),
                spec.area.name
            )
        } else {
            spec.name.clone()
        };
        self.with(|c| {
            if let Some(p) = owner {
                let q = quota_status(c, &p.org_id, p.monthly_quota)?;
                let asked = spec.species.len() as u64;
                if q.used + q.reserved + asked > q.total {
                    return Err(AppError::QuotaExceeded(format!(
                        "this job needs {asked} species but only {} of the {} monthly species remain for {} ({} used, {} reserved by queued or running jobs)",
                        q.remaining,
                        q.total,
                        q.month,
                        q.used,
                        q.reserved
                    )));
                }
            }
            c.execute(
                "INSERT INTO jobs(id,name,status,created_at,stage,spec,species,spec_hash,org_id,created_by,data_state)
                 VALUES(?1,?2,'queued',?3,'Queued',?4,?5,?6,?7,?8,'pending')",
                params![
                    id,
                    name,
                    now(),
                    json(spec)?,
                    json(&species)?,
                    spec_hash,
                    owner.map(|p| p.org_id.as_str()),
                    owner.map(|p| p.user_id.as_str())
                ],
            )
            .map_err(db)?;
            Ok(())
        })?;
        self.event(id, "Job queued", "info", None)?;
        self.get(id)?
            .ok_or_else(|| AppError::Internal("job vanished".into()))
    }

    pub fn get(&self, id: &str) -> Result<Option<Job>, AppError> {
        self.with(|c| {
            c.query_row(&format!("{SELECT} WHERE id=?1"), [id], row)
                .optional()
                .map_err(db)?
                .transpose()
        })
    }

    /// The job if it is visible in `scope`; a job of another organisation is reported as absent.
    pub fn get_scoped(&self, id: &str, scope: &Scope) -> Result<Option<Job>, AppError> {
        let visible = match scope {
            Scope::All => true,
            Scope::Org(org) => self
                .job_org(id)?
                .is_some_and(|o| o.as_deref() == Some(org.as_str())),
        };
        if visible { self.get(id) } else { Ok(None) }
    }

    /// `None`: no such job; `Some(None)`: a job without organisation (local mode).
    pub fn job_org(&self, id: &str) -> Result<Option<Option<String>>, AppError> {
        self.with(|c| {
            c.query_row("SELECT org_id FROM jobs WHERE id=?1", [id], |r| r.get(0))
                .optional()
                .map_err(db)
        })
    }

    pub fn list(&self, scope: &Scope) -> Result<Vec<Job>, AppError> {
        self.with(|c| {
            let (sql, org) = match scope {
                Scope::All => (format!("{SELECT} ORDER BY created_at DESC LIMIT 500"), None),
                Scope::Org(o) => (
                    format!("{SELECT} WHERE org_id=?1 ORDER BY created_at DESC LIMIT 500"),
                    Some(o.as_str()),
                ),
            };
            let mut st = c.prepare(&sql).map_err(db)?;
            let rows = match org {
                Some(o) => st.query_map([o], row),
                None => st.query_map([], row),
            }
            .map_err(db)?;
            let mut out = Vec::new();
            for r in rows {
                out.push(r.map_err(db)??);
            }
            Ok(out)
        })
    }

    /// Next job for the compute worker: data prepared, organisations served in turn (the one
    /// whose last job started longest ago first), then oldest job first.
    pub fn next_queued(&self) -> Result<Option<String>, AppError> {
        self.with(|c| {
            c.query_row(
                "SELECT j.id FROM jobs j
                 WHERE j.status='queued' AND j.data_state IN ('ready','nobatch')
                 ORDER BY (SELECT MAX(k.started_at) FROM jobs k WHERE k.org_id IS j.org_id) ASC NULLS FIRST,
                          j.created_at
                 LIMIT 1",
                [],
                |r| r.get(0),
            )
            .optional()
            .map_err(db)
        })
    }

    /// Claim the oldest queued job whose input data has not been prepared yet.
    pub fn claim_unprepared(&self) -> Result<Option<String>, AppError> {
        self.with(|c| {
            c.query_row(
                "UPDATE jobs SET data_state='preparing', prep_started_at=?1
                 WHERE id=(SELECT id FROM jobs WHERE status='queued' AND data_state='pending' ORDER BY created_at LIMIT 1)
                 RETURNING id",
                [now()],
                |r| r.get(0),
            )
            .optional()
            .map_err(db)
        })
    }

    pub fn data_state(&self, id: &str) -> Result<Option<DataState>, AppError> {
        self.with(|c| {
            c.query_row("SELECT data_state FROM jobs WHERE id=?1", [id], |r| {
                r.get::<_, String>(0)
            })
            .optional()
            .map_err(db)
        })
        .map(|s| s.map(|s| DataState::parse(&s)))
    }

    pub fn set_data_state(&self, id: &str, state: DataState) -> Result<(), AppError> {
        self.with(|c| {
            c.execute(
                // Leaving `preparing` closes the preparation interval (GBIF wait, for metrics).
                "UPDATE jobs SET data_state=?2,
                    prep_finished_at=CASE WHEN data_state='preparing' AND ?2 IN ('ready','nobatch') THEN ?3 ELSE prep_finished_at END
                 WHERE id=?1",
                params![id, state.as_str(), now()],
            )
            .map(|_| ())
            .map_err(db)
        })
    }

    /// Remove a job's rows (events cascade). The usage ledger is kept on purpose.
    pub fn delete(&self, id: &str) -> Result<(), AppError> {
        self.with(|c| {
            c.execute("DELETE FROM jobs WHERE id=?1", [id])
                .map(|_| ())
                .map_err(db)
        })
    }

    /// Finished jobs (succeeded, failed, cancelled) whose end is older than `cutoff` (RFC 3339).
    pub fn finished_before(&self, cutoff: &str) -> Result<Vec<String>, AppError> {
        self.with(|c| {
            let mut st = c
                .prepare(
                    "SELECT id FROM jobs WHERE status IN ('succeeded','failed','cancelled')
                     AND finished_at IS NOT NULL AND finished_at < ?1",
                )
                .map_err(db)?;
            st.query_map([cutoff], |r| r.get(0))
                .map_err(db)?
                .collect::<Result<Vec<String>, _>>()
                .map_err(db)
        })
    }

    /// On startup: jobs left `running` by a crash or kill are marked interrupted and requeued;
    /// species already completed are skipped when the job resumes (checkpointed on disk).
    /// Consistent copy of the database into `path` (SQLite `VACUUM INTO`; readers and the WAL
    /// are unaffected).
    pub fn vacuum_into(&self, path: &Path) -> Result<(), AppError> {
        let target = path
            .to_str()
            .ok_or_else(|| AppError::Storage("backup path is not UTF-8".into()))?;
        self.with(|c| {
            c.execute("VACUUM INTO ?1", [target])
                .map(|_| ())
                .map_err(db)
        })
    }

    /// Studies by state (administration: system status).
    pub fn job_counts(&self) -> Result<crate::dto::JobCounts, AppError> {
        let week_ago =
            (jiff::Timestamp::now() - jiff::SignedDuration::from_hours(7 * 24)).to_string();
        self.with(|c| {
            c.query_row(
                "SELECT COUNT(*),
                        SUM(status='queued'),
                        SUM(status='queued' AND data_state='preparing'),
                        SUM(status='running'),
                        SUM(status='failed' AND finished_at >= ?1)
                 FROM jobs",
                [week_ago],
                |r| {
                    let n = |i: usize| -> rusqlite::Result<u64> {
                        Ok(r.get::<_, Option<i64>>(i)?.unwrap_or(0).max(0) as u64)
                    };
                    Ok(crate::dto::JobCounts {
                        total: n(0)?,
                        queued: n(1)?,
                        preparing: n(2)?,
                        running: n(3)?,
                        failed_7d: n(4)?,
                    })
                },
            )
            .map_err(db)
        })
    }

    /// Studies being modelled right now (a restart would interrupt them; they resume from the
    /// last completed species, but the species in progress starts again).
    pub fn running_count(&self) -> Result<u64, AppError> {
        self.with(|c| {
            c.query_row(
                "SELECT COUNT(*) FROM jobs WHERE status='running'",
                [],
                |r| r.get::<_, i64>(0),
            )
            .map(|n| n.max(0) as u64)
            .map_err(db)
        })
    }

    pub fn recover_interrupted(&self) -> Result<Vec<String>, AppError> {
        self.with(|c| {
            c.execute(
                "UPDATE jobs SET data_state='pending' WHERE data_state='preparing'",
                [],
            )
            .map(|_| ())
            .map_err(db)
        })?;
        let ids: Vec<String> = self.with(|c| {
            let mut st = c
                .prepare("SELECT id FROM jobs WHERE status='running'")
                .map_err(db)?;

            st.query_map([], |r| r.get(0))
                .map_err(db)?
                .collect::<Result<Vec<String>, _>>()
                .map_err(db)
        })?;
        for id in &ids {
            self.event(
                id,
                "Interrupted (process stopped); requeued to resume from the last completed species",
                "warning",
                None,
            )?;
            self.with(|c| {
                c.execute(
                    "UPDATE jobs SET status='queued', stage='Queued (resuming)' WHERE id=?1",
                    [id],
                )
                .map(|_| ())
                .map_err(db)
            })?;
        }
        Ok(ids)
    }

    pub fn set_status(
        &self,
        id: &str,
        status: JobStatus,
        stage: &str,
        error: Option<&str>,
    ) -> Result<(), AppError> {
        self.with(|c| {
            let ts = now();
            match status {
                JobStatus::Running => c.execute("UPDATE jobs SET status=?2, stage=?3, started_at=COALESCE(started_at, ?4) WHERE id=?1", params![id, status.as_str(), stage, ts]),
                s if s.is_terminal() => c.execute(
                    "UPDATE jobs SET status=?2, stage=?3, finished_at=?4, error=?5, progress=CASE WHEN ?2='succeeded' THEN 1 ELSE progress END WHERE id=?1",
                    params![id, status.as_str(), stage, ts, error],
                ),
                _ => c.execute("UPDATE jobs SET status=?2, stage=?3 WHERE id=?1", params![id, status.as_str(), stage]),
            }
            .map(|_| ())
            .map_err(db)
        })
    }

    pub fn set_progress(&self, id: &str, progress: f64, stage: &str) -> Result<(), AppError> {
        self.with(|c| {
            c.execute(
                "UPDATE jobs SET progress=?2, stage=?3 WHERE id=?1",
                params![id, progress, stage],
            )
            .map(|_| ())
            .map_err(db)
        })
    }

    pub fn set_species(&self, id: &str, species: &[SpeciesRunSummary]) -> Result<(), AppError> {
        self.with(|c| {
            c.execute(
                "UPDATE jobs SET species=?2 WHERE id=?1",
                params![id, json(species)?],
            )
            .map(|_| ())
            .map_err(db)
        })
    }

    pub fn event(
        &self,
        id: &str,
        message: &str,
        level: &str,
        species_key: Option<u64>,
    ) -> Result<JobEvent, AppError> {
        let job = self
            .get(id)?
            .ok_or_else(|| AppError::NotFound(format!("job {id}")))?;
        self.with(|c| {
            let seq: i64 = c
                .query_row(
                    "SELECT COALESCE(MAX(seq),0)+1 FROM events WHERE job_id=?1",
                    [id],
                    |r| r.get(0),
                )
                .map_err(db)?;
            let ev = JobEvent {
                seq,
                job_id: id.into(),
                status: job.status,
                stage: job.stage.clone(),
                progress: job.progress,
                species_key,
                message: message.into(),
                level: level.into(),
                at: now(),
            };
            c.execute(
                "INSERT INTO events(job_id,seq,body) VALUES(?1,?2,?3)",
                params![id, seq, json(&ev)?],
            )
            .map_err(db)?;
            Ok(ev)
        })
    }

    pub fn events_after(&self, id: &str, after: i64) -> Result<Vec<JobEvent>, AppError> {
        self.with(|c| {
            let mut st = c
                .prepare(
                    "SELECT body FROM events WHERE job_id=?1 AND seq>?2 ORDER BY seq LIMIT 1000",
                )
                .map_err(db)?;
            let rows = st
                .query_map(params![id, after], |r| r.get::<_, String>(0))
                .map_err(db)?;
            let mut out = Vec::new();
            for r in rows {
                out.push(
                    serde_json::from_str(&r.map_err(db)?)
                        .map_err(|e| AppError::Storage(e.to_string()))?,
                );
            }
            Ok(out)
        })
    }
}

/// Input-data preparation of a queued job (GBIF batch download), done off the compute worker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataState {
    /// Not prepared yet (also: interrupted while preparing).
    Pending,
    Preparing,
    /// Ready; a batch download, if any, is in the cache.
    Ready,
    /// Preparation failed: run with the per-species search API instead of waiting again.
    NoBatch,
}

impl DataState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Preparing => "preparing",
            Self::Ready => "ready",
            Self::NoBatch => "nobatch",
        }
    }
    pub fn parse(s: &str) -> Self {
        match s {
            "preparing" => Self::Preparing,
            "ready" => Self::Ready,
            "nobatch" => Self::NoBatch,
            _ => Self::Pending,
        }
    }
}

const SELECT: &str =
    "SELECT id,name,status,created_at,started_at,finished_at,progress,stage,spec,species,error,
    project_id, (SELECT u.name FROM users u WHERE u.id=jobs.created_by) FROM jobs";

fn row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Result<Job, AppError>> {
    let spec: String = r.get(8)?;
    let species: String = r.get(9)?;
    let parse = || -> Result<Job, AppError> {
        Ok(Job {
            id: r.get(0).map_err(db)?,
            name: r.get(1).map_err(db)?,
            status: JobStatus::parse(&r.get::<_, String>(2).map_err(db)?),
            created_at: r.get(3).map_err(db)?,
            started_at: r.get(4).map_err(db)?,
            finished_at: r.get(5).map_err(db)?,
            progress: r.get(6).map_err(db)?,
            stage: r.get(7).map_err(db)?,
            spec: serde_json::from_str(&spec).map_err(|e| AppError::Storage(e.to_string()))?,
            species: serde_json::from_str(&species)
                .map_err(|e| AppError::Storage(e.to_string()))?,
            error: r.get(10).map_err(db)?,
            project_id: r.get(11).map_err(db)?,
            created_by_name: r.get(12).map_err(db)?,
        })
    };
    Ok(parse())
}

fn json<T: serde::Serialize + ?Sized>(v: &T) -> Result<String, AppError> {
    serde_json::to_string(v).map_err(|e| AppError::Internal(e.to_string()))
}
