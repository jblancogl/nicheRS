//! Hosted mode (ADR-0105, ADR-0106): schema migration, invitation links, sessions, quota, organisation
//! isolation, deletion, retention and the fair job queue. Offline (synthetic demo data).

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use niche_app::accounts::{Principal, Scope, UserFilter};
use niche_app::audit::{AuditAction, AuditEvent, AuditFilter};
use niche_app::dto::*;
use niche_app::gbif_accounts::GbifOwner;
use niche_app::secrets::SecretKey;
use niche_app::store::{DataState, Store};
use niche_app::{AppError, Config, Engine, Limits, demo};

fn tmp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("niche-accounts-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("tmp dir");
    dir
}

fn engine(dir: &Path, retention_days: u64) -> Engine {
    Engine::open(Config {
        data_dir: dir.join("data"),
        cache_dir: dir.join("cache"),
        max_records: 5000,
        gbif: None,
        secret_key: None,
        mirror: None,
        limits: Limits {
            max_species_per_job: 25,
            max_area_cells: None,
        },
        retention_days,
    })
    .expect("engine")
}

fn spec(n_species: u64) -> JobSpec {
    JobSpec {
        name: format!("{n_species} species"),
        species: (0..n_species)
            .map(|i| SpeciesRef {
                key: i,
                scientific_name: format!("{} #{i}", demo::SPECIES_NAME),
                own: None,
            })
            .collect(),
        area: StudyArea {
            name: "Demo".into(),
            geometry: demo::area_geojson(),
            places: vec![],
        },
        layers: demo::LAYERS.iter().map(|l| l.0.to_string()).collect(),
        resolution_factor: 2,
        filters: OccurrenceFilters::default(),
        model: ModelSettings {
            feature_classes: vec!["LQ".into()],
            beta_multipliers: vec![1.0],
            ..ModelSettings::default()
        },
        validation: ValidationSettings::default(),
        seed: 5,
        demo: true,
        projections: vec![],
        own_dataset: None,
    }
}

/// An organisation with one signed-up user.
fn member(st: &Store, org: &str, quota: u64, email: &str) -> Principal {
    st.org_create(org, quota).expect("org");
    let token = st.invite(org).expect("invite");
    st.accept_invite(&token, "Test User", email, "a long enough password")
        .expect("accept")
        .0
}

const V10_DROP: &str = "DROP TABLE audit;";
const V9_DROP: &str = "DROP INDEX orgs_operator; ALTER TABLE orgs DROP COLUMN is_operator;";
const V8_DROP: &str = "DROP TABLE feedback_files; DROP TABLE feedback;";
const V7_DROP: &str = "ALTER TABLE users DROP COLUMN gbif_username; ALTER TABLE users DROP COLUMN gbif_secret; ALTER TABLE users DROP COLUMN gbif_checked_at; ALTER TABLE users DROP COLUMN gbif_ok; ALTER TABLE orgs DROP COLUMN gbif_username; ALTER TABLE orgs DROP COLUMN gbif_secret; ALTER TABLE orgs DROP COLUMN gbif_checked_at; ALTER TABLE orgs DROP COLUMN gbif_ok; ALTER TABLE jobs DROP COLUMN gbif_account;";

fn sql(dir: &Path, stmt: &str) {
    let c = rusqlite::Connection::open(dir.join("data/niche.db")).expect("db");
    c.execute_batch(stmt).expect("sql");
}

#[test]
fn v1_database_is_migrated_and_newer_is_refused() {
    let dir = tmp("migrate");
    let db = dir.join("niche.db");
    {
        let c = rusqlite::Connection::open(&db).expect("db");
        c.execute_batch(
            "CREATE TABLE meta(key TEXT PRIMARY KEY, value TEXT NOT NULL);
             INSERT INTO meta VALUES('schema_version','1');
             CREATE TABLE jobs(id TEXT PRIMARY KEY, name TEXT NOT NULL, status TEXT NOT NULL,
                created_at TEXT NOT NULL, started_at TEXT, finished_at TEXT,
                progress REAL NOT NULL DEFAULT 0, stage TEXT NOT NULL DEFAULT '',
                spec TEXT NOT NULL, species TEXT NOT NULL, error TEXT, spec_hash TEXT NOT NULL);
             CREATE TABLE events(job_id TEXT NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
                seq INTEGER NOT NULL, body TEXT NOT NULL, PRIMARY KEY(job_id, seq));",
        )
        .expect("v1 schema");
        c.execute(
            "INSERT INTO jobs(id,name,status,created_at,spec,species,spec_hash) VALUES('old','old','succeeded','2026-01-01T00:00:00Z',?1,'[]','h')",
            [serde_json::to_string(&spec(1)).expect("json")],
        )
        .expect("v1 job");
    }
    let st = Store::open(&db).expect("migrated");
    let job = st.get("old").expect("get").expect("job kept");
    assert_eq!(job.status, JobStatus::Succeeded);
    assert_eq!(st.job_org("old").expect("org"), Some(None));
    assert_eq!(st.data_state("old").expect("state"), Some(DataState::Ready));
    // Existing jobs can be given to an organisation.
    st.org_create("NS interne", 100).expect("org");
    assert_eq!(st.org_adopt_unowned("NS interne").expect("adopt"), 1);
    drop(st);
    // The v1 database was copied before being migrated.
    let copies = |prefix: &str| {
        std::fs::read_dir(dir.join("backups"))
            .map(|rd| {
                rd.flatten()
                    .filter(|e| e.file_name().to_string_lossy().starts_with(prefix))
                    .count()
            })
            .unwrap_or(0)
    };
    assert_eq!(copies("niche-premigrate-v1-"), 1);
    // Re-opening a current database is a no-op (and makes no copy).
    Store::open(&db).expect("reopen");
    assert_eq!(copies("niche-premigrate-"), 1);
    // A v2 database (first hosted release) gains the v3 columns.
    let c = rusqlite::Connection::open(&db).expect("db");
    c.execute_batch(
        "DROP TABLE audit; DROP INDEX orgs_operator; ALTER TABLE orgs DROP COLUMN is_operator; DROP TABLE feedback_files; DROP TABLE feedback; ALTER TABLE users DROP COLUMN is_admin; ALTER TABLE users DROP COLUMN last_login_at;
         ALTER TABLE users DROP COLUMN lang; DROP INDEX jobs_project; ALTER TABLE jobs DROP COLUMN project_id; DROP TABLE projects;
         ALTER TABLE users DROP COLUMN gbif_username; ALTER TABLE users DROP COLUMN gbif_secret; ALTER TABLE users DROP COLUMN gbif_checked_at; ALTER TABLE users DROP COLUMN gbif_ok; ALTER TABLE orgs DROP COLUMN gbif_username; ALTER TABLE orgs DROP COLUMN gbif_secret; ALTER TABLE orgs DROP COLUMN gbif_checked_at; ALTER TABLE orgs DROP COLUMN gbif_ok; ALTER TABLE jobs DROP COLUMN gbif_account; ALTER TABLE users DROP COLUMN is_manager; ALTER TABLE jobs DROP COLUMN prep_started_at; ALTER TABLE jobs DROP COLUMN prep_finished_at;
         UPDATE meta SET value='2' WHERE key='schema_version';",
    )
    .expect("downgrade to v2");
    drop(c);
    let st = Store::open(&db).expect("v2 → v4");
    // NotFound (not a storage error): the column exists again, the account does not.
    assert!(matches!(
        st.user_set_admin("nobody@x.fr", false),
        Err(AppError::NotFound(_))
    ));
    st.user_set_lang("nobody", "fr").expect("lang column added");
    drop(st);
    // A v3 database (administration release) gains the language column.
    let c = rusqlite::Connection::open(&db).expect("db");
    c.execute_batch(
        "DROP TABLE audit; DROP INDEX orgs_operator; ALTER TABLE orgs DROP COLUMN is_operator; DROP TABLE feedback_files; DROP TABLE feedback; ALTER TABLE users DROP COLUMN lang; DROP INDEX jobs_project; ALTER TABLE jobs DROP COLUMN project_id; DROP TABLE projects;
         ALTER TABLE users DROP COLUMN gbif_username; ALTER TABLE users DROP COLUMN gbif_secret; ALTER TABLE users DROP COLUMN gbif_checked_at; ALTER TABLE users DROP COLUMN gbif_ok; ALTER TABLE orgs DROP COLUMN gbif_username; ALTER TABLE orgs DROP COLUMN gbif_secret; ALTER TABLE orgs DROP COLUMN gbif_checked_at; ALTER TABLE orgs DROP COLUMN gbif_ok; ALTER TABLE jobs DROP COLUMN gbif_account; ALTER TABLE users DROP COLUMN is_manager; ALTER TABLE jobs DROP COLUMN prep_started_at; ALTER TABLE jobs DROP COLUMN prep_finished_at;
         UPDATE meta SET value='3' WHERE key='schema_version';",
    )
    .expect("downgrade to v3");
    drop(c);
    let st = Store::open(&db).expect("v3 → v4");
    st.user_set_lang("nobody", "es").expect("lang column added");
    assert!(matches!(
        st.user_set_lang("nobody", "de"),
        Err(AppError::Invalid(_))
    ));
    drop(st);
    // A v4 database (account language) gains projects; existing studies have none.
    let c = rusqlite::Connection::open(&db).expect("db");
    c.execute_batch("DROP TABLE audit; DROP INDEX orgs_operator; ALTER TABLE orgs DROP COLUMN is_operator; DROP TABLE feedback_files; DROP TABLE feedback; DROP INDEX jobs_project; ALTER TABLE jobs DROP COLUMN project_id; DROP TABLE projects; ALTER TABLE users DROP COLUMN gbif_username; ALTER TABLE users DROP COLUMN gbif_secret; ALTER TABLE users DROP COLUMN gbif_checked_at; ALTER TABLE users DROP COLUMN gbif_ok; ALTER TABLE orgs DROP COLUMN gbif_username; ALTER TABLE orgs DROP COLUMN gbif_secret; ALTER TABLE orgs DROP COLUMN gbif_checked_at; ALTER TABLE orgs DROP COLUMN gbif_ok; ALTER TABLE jobs DROP COLUMN gbif_account; ALTER TABLE users DROP COLUMN is_manager; ALTER TABLE jobs DROP COLUMN prep_started_at; ALTER TABLE jobs DROP COLUMN prep_finished_at; UPDATE meta SET value='4' WHERE key='schema_version';")
        .expect("downgrade to v4");
    drop(c);
    let st = Store::open(&db).expect("v4 → v5");
    assert_eq!(st.job_project("old").expect("project"), Some(None));
    assert!(st.project_list(&Scope::All).expect("projects").is_empty());
    drop(st);
    // A v5 database (projects) gains managers and data-preparation times.
    let c = rusqlite::Connection::open(&db).expect("db");
    c.execute_batch("DROP TABLE audit; DROP INDEX orgs_operator; ALTER TABLE orgs DROP COLUMN is_operator; DROP TABLE feedback_files; DROP TABLE feedback; ALTER TABLE users DROP COLUMN gbif_username; ALTER TABLE users DROP COLUMN gbif_secret; ALTER TABLE users DROP COLUMN gbif_checked_at; ALTER TABLE users DROP COLUMN gbif_ok; ALTER TABLE orgs DROP COLUMN gbif_username; ALTER TABLE orgs DROP COLUMN gbif_secret; ALTER TABLE orgs DROP COLUMN gbif_checked_at; ALTER TABLE orgs DROP COLUMN gbif_ok; ALTER TABLE jobs DROP COLUMN gbif_account; ALTER TABLE users DROP COLUMN is_manager; ALTER TABLE jobs DROP COLUMN prep_started_at; ALTER TABLE jobs DROP COLUMN prep_finished_at; UPDATE meta SET value='5' WHERE key='schema_version';")
        .expect("downgrade to v5");
    drop(c);
    let st = Store::open(&db).expect("v5 → v6");
    assert!(matches!(
        st.user_set_manager("nobody@x.fr", true),
        Err(AppError::NotFound(_))
    ));
    drop(st);
    // A v6 database (managers) gains GBIF accounts of users, organisations and jobs.
    let c = rusqlite::Connection::open(&db).expect("db");
    c.execute_batch(&format!(
        "{V10_DROP} {V9_DROP} {V8_DROP} {V7_DROP} UPDATE meta SET value='6' WHERE key='schema_version';"
    ))
    .expect("downgrade to v6");
    drop(c);
    let st = Store::open(&db).expect("v6 → v7");
    assert_eq!(
        st.gbif_get(&GbifOwner::User("nobody".into()))
            .expect("gbif columns added"),
        None
    );
    drop(st);
    // A v7 database (GBIF accounts) gains feedback.
    let c = rusqlite::Connection::open(&db).expect("db");
    c.execute_batch(&format!(
        "{V10_DROP} {V9_DROP} {V8_DROP} UPDATE meta SET value='7' WHERE key='schema_version';"
    ))
    .expect("downgrade to v7");
    drop(c);
    let st = Store::open(&db).expect("v7 → v8");
    assert!(
        st.feedback_list(None)
            .expect("feedback tables added")
            .is_empty()
    );
    drop(st);
    // A v8 database (feedback) gains the operator organisation: the one named Natural
    // Solutions, otherwise the one of every current administrator, otherwise none.
    let downgrade_to_v8 = |extra: &str| {
        let c = rusqlite::Connection::open(&db).expect("db");
        c.execute_batch(&format!(
            "{V10_DROP} {V9_DROP} {extra} UPDATE meta SET value='8' WHERE key='schema_version';"
        ))
        .expect("downgrade to v8");
    };
    downgrade_to_v8("");
    let st = Store::open(&db).expect("v8 → v9");
    assert_eq!(
        st.operator_org().expect("operator"),
        None,
        "no admins, no NS"
    );
    let a = st.org_create("Staff", 1).expect("org");
    let t = st.invite("Staff").expect("invite");
    st.accept_invite(&t, "Admin", "admin@staff.fr", "a long enough password")
        .expect("accept");
    drop(st);
    downgrade_to_v8("UPDATE users SET is_admin=1 WHERE email='admin@staff.fr';");
    let st = Store::open(&db).expect("v8 → v9");
    assert_eq!(
        st.operator_org().expect("operator").map(|o| o.id),
        Some(a.id)
    );
    assert!(st.admins_outside_operator().expect("outside").is_empty());
    let ns = st.org_create("natural solutions", 1).expect("org");
    drop(st);
    downgrade_to_v8("");
    let st = Store::open(&db).expect("v8 → v9");
    assert_eq!(
        st.operator_org().expect("operator").map(|o| o.id),
        Some(ns.id),
        "Natural Solutions first, whatever the case"
    );
    assert_eq!(
        st.admins_outside_operator().expect("outside").len(),
        1,
        "kept, reported"
    );
    drop(st);
    // A v9 database (operator organisation) gains the audit trail.
    let c = rusqlite::Connection::open(&db).expect("db");
    c.execute_batch(&format!(
        "{V10_DROP} UPDATE meta SET value='9' WHERE key='schema_version';"
    ))
    .expect("downgrade to v9");
    drop(c);
    let st = Store::open(&db).expect("v9 → v10");
    let page = st
        .audit_list(&AuditFilter::default())
        .expect("audit table added");
    assert!(page.entries.is_empty() && page.next.is_none());
    drop(st);
    // A newer schema is refused.
    let c = rusqlite::Connection::open(&db).expect("db");
    c.execute("UPDATE meta SET value='99' WHERE key='schema_version'", [])
        .expect("bump");
    drop(c);
    assert!(matches!(Store::open(&db), Err(AppError::Storage(_))));
}

#[test]
fn invitation_session_reset_and_disable() {
    let dir = tmp("links");
    let e = engine(&dir, 0);
    let st = e.store();
    st.org_create("Bureau X", 10).expect("org");
    let token = st
        .invite("bureau x")
        .expect("invite (case-insensitive name)");
    let info = st.link_info(&token).expect("info");
    assert_eq!(info.kind, LinkKind::Invite);
    assert_eq!(info.org_name, "Bureau X");
    assert!(
        st.accept_invite(&token, "Alice", "alice@bureau-x.fr", "short")
            .is_err(),
        "short password refused"
    );
    let (p, session) = st
        .accept_invite(
            &token,
            "Alice",
            "Alice@Bureau-X.fr",
            "a long enough password",
        )
        .expect("accept");
    assert_eq!(p.email, "alice@bureau-x.fr");
    assert_eq!(st.session(&session).expect("session"), Some(p.clone()));
    // Single use.
    assert!(st.link_info(&token).is_err());
    assert!(
        st.accept_invite(&token, "Eve", "eve@x.fr", "a long enough password")
            .is_err()
    );
    // Same e-mail cannot be registered twice.
    let t2 = st.invite("Bureau X").expect("invite");
    assert!(matches!(
        st.accept_invite(&t2, "A2", "alice@bureau-x.fr", "a long enough password"),
        Err(AppError::Conflict(_))
    ));
    // Expired and revoked links.
    let t3 = st.invite("Bureau X").expect("invite");
    sql(
        &dir,
        "UPDATE links SET expires_at='2000-01-01T00:00:00Z' WHERE used_at IS NULL",
    );
    assert!(st.link_info(&t3).is_err(), "expired");
    let t4 = st.invite("Bureau X").expect("invite");
    // t2 (refused), t3 (expired) and t4 were never used.
    assert_eq!(st.revoke_invites("Bureau X").expect("revoke"), 3);
    assert!(st.link_info(&t4).is_err(), "revoked");
    // Login.
    assert!(
        st.login("alice@bureau-x.fr", "wrong password!!")
            .expect("login")
            .is_none()
    );
    assert!(
        st.login("nobody@bureau-x.fr", "a long enough password")
            .expect("login")
            .is_none()
    );
    let (_, s2) = st
        .login(" ALICE@bureau-x.fr", "a long enough password")
        .expect("login")
        .expect("ok");
    // Reset: new password, other sessions closed, link single use.
    let r = st.reset_link("alice@bureau-x.fr").expect("reset link");
    assert_eq!(st.link_info(&r).expect("info").kind, LinkKind::Reset);
    let (_, s3) = st
        .reset_password(&r, "another long password")
        .expect("reset");
    assert_eq!(st.session(&session).expect("s"), None);
    assert_eq!(st.session(&s2).expect("s"), None);
    assert!(st.session(&s3).expect("s").is_some());
    assert!(st.reset_password(&r, "yet another password").is_err());
    assert!(
        st.login("alice@bureau-x.fr", "a long enough password")
            .expect("l")
            .is_none()
    );
    // Expired session.
    let (_, s4) = st
        .login("alice@bureau-x.fr", "another long password")
        .expect("l")
        .expect("ok");
    sql(
        &dir,
        "UPDATE sessions SET expires_at='2000-01-01T00:00:00Z'",
    );
    assert_eq!(st.session(&s4).expect("s"), None);
    // Disabled account, then disabled organisation.
    let (_, s5) = st
        .login("alice@bureau-x.fr", "another long password")
        .expect("l")
        .expect("ok");
    st.user_set_disabled("alice@bureau-x.fr", true)
        .expect("disable");
    assert_eq!(st.session(&s5).expect("s"), None);
    assert!(
        st.login("alice@bureau-x.fr", "another long password")
            .expect("l")
            .is_none()
    );
    st.user_set_disabled("alice@bureau-x.fr", false)
        .expect("enable");
    let (_, s6) = st
        .login("alice@bureau-x.fr", "another long password")
        .expect("l")
        .expect("ok");
    st.org_set_disabled("Bureau X", true).expect("disable org");
    assert_eq!(st.session(&s6).expect("s"), None);
    assert!(
        st.login("alice@bureau-x.fr", "another long password")
            .expect("l")
            .is_none()
    );
    assert!(
        st.invite("Bureau X").is_err(),
        "no invitations into a disabled org"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn quota_isolation_deletion_and_usage_ledger() {
    let dir = tmp("quota");
    let e = engine(&dir, 0);
    let st = e.store();
    let a = member(st, "A", 3, "a@a.fr");
    let b = member(st, "B", 10, "b@b.fr");
    assert!(
        matches!(e.submit_as(spec(26), Some(&b)), Err(AppError::Invalid(_))),
        "species-per-job limit"
    );
    let j1 = e.submit_as(spec(2), Some(&a)).expect("within quota");
    let q = st.quota(&a.org_id, a.monthly_quota).expect("quota");
    assert_eq!((q.used, q.reserved, q.remaining), (0, 2, 1));
    let err = e.submit_as(spec(2), Some(&a)).expect_err("over quota");
    assert!(matches!(err, AppError::QuotaExceeded(_)), "{err}");
    // Organisation isolation.
    assert!(e.job(&j1.id, &a.scope()).is_ok());
    assert!(matches!(
        e.job(&j1.id, &b.scope()),
        Err(AppError::NotFound(_))
    ));
    assert!(e.job(&j1.id, &Scope::All).is_ok());
    assert_eq!(st.list(&a.scope()).expect("list").len(), 1);
    assert!(st.list(&b.scope()).expect("list").is_empty());
    // Running counts each modelled species once, in the month it finishes.
    assert!(
        matches!(e.delete_job(&j1.id), Err(AppError::Conflict(_))),
        "queued job"
    );
    let never = AtomicBool::new(false);
    e.run_job(&j1.id, &never, &never);
    let job = st.get(&j1.id).expect("get").expect("job");
    assert_eq!(job.status, JobStatus::Succeeded, "{:?}", job.error);
    let fitted = job
        .species
        .iter()
        .filter(|s| s.status == SpeciesRunStatus::Succeeded)
        .count() as u64;
    let q = st.quota(&a.org_id, a.monthly_quota).expect("quota");
    assert_eq!((q.used, q.reserved), (fitted, 0));
    st.record_usage(&j1.id, 0).expect("idempotent");
    assert_eq!(
        st.usage(Some("A"), None).expect("usage").len() as u64,
        fitted
    );
    // Local-mode jobs (no organisation) are never counted.
    let local = e.submit(spec(1)).expect("local job");
    st.record_usage(&local.id, 0).expect("no-op");
    assert_eq!(st.usage(None, None).expect("usage").len() as u64, fitted);
    // Deleting removes files and rows but keeps the ledger (no refund).
    let out = dir.join("data/jobs").join(&j1.id);
    assert!(out.exists());
    e.delete_job(&j1.id).expect("delete");
    assert!(!out.exists());
    assert!(st.get(&j1.id).expect("get").is_none());
    assert_eq!(st.quota(&a.org_id, 3).expect("q").used, fitted);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn retention_deletes_only_old_finished_jobs() {
    let dir = tmp("retention");
    let e = engine(&dir, 365);
    let old = e.submit(spec(1)).expect("job");
    let recent = e.submit(spec(1)).expect("job");
    let queued = e.submit(spec(1)).expect("job");
    sql(
        &dir,
        &format!(
            "UPDATE jobs SET status='succeeded', finished_at='2020-01-01T00:00:00Z' WHERE id='{}';
             UPDATE jobs SET status='succeeded', finished_at='2099-01-01T00:00:00Z' WHERE id='{}';
             UPDATE jobs SET finished_at='2020-01-01T00:00:00Z' WHERE id='{}';",
            old.id, recent.id, queued.id
        ),
    );
    assert_eq!(e.sweep_expired().expect("sweep"), vec![old.id.clone()]);
    assert!(e.store().get(&old.id).expect("get").is_none());
    assert!(e.store().get(&recent.id).expect("get").is_some());
    assert!(e.store().get(&queued.id).expect("get").is_some());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn queue_waits_for_data_and_serves_organisations_in_turn() {
    let dir = tmp("queue");
    let e = engine(&dir, 0);
    let st = e.store();
    let a = member(st, "A", 100, "a@a.fr");
    let b = member(st, "B", 100, "b@b.fr");
    let a1 = e.submit_as(spec(1), Some(&a)).expect("a1");
    let a2 = e.submit_as(spec(1), Some(&a)).expect("a2");
    let b1 = e.submit_as(spec(1), Some(&b)).expect("b1");
    // Nothing runs before its data is prepared.
    assert_eq!(st.next_queued().expect("next"), None);
    let mut claimed = Vec::new();
    while let Some(id) = st.claim_unprepared().expect("claim") {
        claimed.push(id);
    }
    assert_eq!(claimed, vec![a1.id.clone(), a2.id.clone(), b1.id.clone()]);
    assert_eq!(st.next_queued().expect("next"), None, "still preparing");
    for id in &claimed {
        st.set_data_state(id, DataState::Ready).expect("ready");
    }
    // A has just started a job: B goes before A's second one, although it was queued later.
    assert_eq!(st.next_queued().expect("next"), Some(a1.id.clone()));
    st.set_status(&a1.id, JobStatus::Running, "running", None)
        .expect("start");
    assert_eq!(st.next_queued().expect("next"), Some(b1.id.clone()));
    // An interrupted preparation is redone after a restart.
    st.set_data_state(&a2.id, DataState::Preparing)
        .expect("preparing");
    st.recover_interrupted().expect("recover");
    assert_eq!(
        st.data_state(&a2.id).expect("state"),
        Some(DataState::Pending)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn prepare_loop_marks_jobs_ready_without_gbif_account() {
    let dir = tmp("prepare");
    let e = std::sync::Arc::new(engine(&dir, 0));
    let job = e.submit(spec(1)).expect("job");
    let stop = std::sync::Arc::new(AtomicBool::new(false));
    let (e2, s2) = (e.clone(), stop.clone());
    let h = std::thread::spawn(move || e2.prepare_loop(&s2));
    let t0 = std::time::Instant::now();
    while e.store().data_state(&job.id).expect("state") != Some(DataState::Ready) {
        assert!(t0.elapsed().as_secs() < 20, "not prepared");
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    stop.store(true, std::sync::atomic::Ordering::Relaxed);
    h.join().expect("join");
    assert_eq!(e.store().next_queued().expect("next"), Some(job.id));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn administration_queries() {
    let dir = tmp("admin");
    let e = engine(&dir, 0);
    let st = e.store();
    let a = member(st, "A", 5, "a@a.fr");
    assert!(!a.is_admin());
    assert_eq!(a.access(), a.scope());
    assert!(
        matches!(
            st.user_set_admin("A@a.fr", true),
            Err(AppError::Conflict(_))
        ),
        "no operator organisation yet"
    );
    assert!(st.org_set_operator("A").expect("operator").is_operator);
    st.user_set_admin("A@a.fr", true).expect("promote");
    let (p, _) = st
        .login("a@a.fr", "a long enough password")
        .expect("login")
        .expect("ok");
    assert!(p.is_admin());
    assert_eq!(p.access(), Scope::All);
    assert_eq!(
        p.scope(),
        Scope::Org(p.org_id.clone()),
        "own runs only in the workspace"
    );
    let users = st.user_list(None).expect("users");
    assert!(users[0].is_admin && users[0].last_login_at.is_some() && users[0].in_operator_org);
    assert!(st.user_set_admin("nobody@a.fr", true).is_err());
    assert!(
        st.check_password_of(&a.user_id, "a long enough password")
            .expect("check")
    );
    assert!(
        !st.check_password_of(&a.user_id, "wrong password!!")
            .expect("check")
    );

    // Overview counts: users, runs, open invitations; organisation lookup by id or name.
    st.org_create("B", 2).expect("org");
    let t1 = st.invite("B").expect("invite");
    let _t2 = st.invite("B").expect("invite");
    let b = st.org("b").expect("by name");
    assert_eq!(st.org(&b.id).expect("by id").name, "B");
    let job = e.submit_as(spec(1), Some(&a)).expect("job");
    let ov = st.org_overview().expect("overview");
    let oa = ov.iter().find(|o| o.org.name == "A").expect("A");
    let ob = ov.iter().find(|o| o.org.name == "B").expect("B");
    assert_eq!(
        (oa.users, oa.runs, oa.open_invites, oa.quota.reserved),
        (1, 1, 0, 1)
    );
    assert_eq!((ob.users, ob.runs, ob.open_invites), (0, 0, 2));
    // Revoke one invitation by id; the other stays usable.
    let open = st.open_invites(&b.id).expect("invites");
    assert_eq!(open.len(), 2);
    let target = open
        .iter()
        .find(|i| i.id == niche_app::accounts::token_hash(&t1))
        .expect("t1 listed by hash");
    st.revoke_link(&target.id).expect("revoke");
    assert!(st.revoke_link(&target.id).is_err(), "already revoked");
    assert!(st.link_info(&t1).is_err());
    assert_eq!(st.open_invites("B").expect("invites").len(), 1);

    // Usage summary lists every organisation for a month; activity shows every organisation.
    let never = AtomicBool::new(false);
    e.run_job(&job.id, &never, &never);
    let month = niche_app::accounts::current_month();
    let us = st.usage_summary(Some(&month)).expect("usage");
    assert_eq!(
        us.iter()
            .map(|u| (u.org_name.as_str(), u.species))
            .collect::<Vec<_>>(),
        vec![("A", 1), ("B", 0)]
    );
    assert_eq!(st.usage_summary(None).expect("all months").len(), 1);
    let local = e.submit(spec(2)).expect("local job");
    let act = st.activity(10, None).expect("activity");
    assert_eq!(act.len(), 2);
    assert_eq!(act[0].id, local.id, "newest first");
    assert_eq!(act[0].org_name, None);
    assert_eq!(act[0].species, 2);
    assert_eq!(act[1].org_name.as_deref(), Some("A"));
    assert_eq!(act[1].created_by.as_deref(), Some("a@a.fr"));
    assert_eq!(act[1].status, JobStatus::Succeeded);
    // One organisation's studies only.
    let in_a = st.activity(10, Some(&a.org_id)).expect("activity");
    assert_eq!(in_a.len(), 1);
    assert_eq!(in_a[0].org_id.as_deref(), Some(a.org_id.as_str()));
    // One organisation's overview equals its line in the list.
    let one = st.org_overview_of("A").expect("one");
    assert_eq!(
        Some(&one),
        st.org_overview()
            .expect("all")
            .iter()
            .find(|o| o.org.name == "A")
    );
    assert!(matches!(
        st.org_overview_of("nope"),
        Err(AppError::NotFound(_))
    ));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn accounts_are_searched_and_paged_on_the_server() {
    let dir = tmp("users");
    let e = engine(&dir, 0);
    let st = e.store();
    let a = member(st, "Alpha", 5, "zoe@alpha.fr");
    for email in ["yan@alpha.fr", "x_y@alpha.fr"] {
        let t = st.invite("Alpha").expect("invite");
        st.accept_invite(&t, "Alpha member", email, "a long enough password")
            .expect("accept");
    }
    member(st, "Beta", 5, "ana@beta.fr");
    let all = st.user_search(&UserFilter::default()).expect("all");
    let emails: Vec<_> = all.users.iter().map(|u| u.email.as_str()).collect();
    assert_eq!(
        emails,
        [
            "x_y@alpha.fr",
            "yan@alpha.fr",
            "zoe@alpha.fr",
            "ana@beta.fr"
        ]
    );
    assert_eq!((all.total, all.next_offset), (4, None));
    // Pages.
    let p1 = st
        .user_search(&UserFilter {
            limit: Some(3),
            ..UserFilter::default()
        })
        .expect("p1");
    assert_eq!((p1.users.len(), p1.next_offset), (3, Some(3)));
    let p2 = st
        .user_search(&UserFilter {
            limit: Some(3),
            offset: p1.next_offset.map(|o| o as usize),
            ..UserFilter::default()
        })
        .expect("p2");
    assert_eq!((p2.users.len(), p2.total, p2.next_offset), (1, 4, None));
    // Filters: organisation, text (name, e-mail or organisation; `_` and `%` are literal).
    let org = UserFilter {
        org: Some(a.org_id.clone()),
        ..UserFilter::default()
    };
    assert_eq!(st.user_search(&org).expect("org").total, 3);
    let q = |t: &str| {
        st.user_search(&UserFilter {
            q: Some(t.into()),
            ..UserFilter::default()
        })
        .expect("q")
        .total
    };
    assert_eq!(
        (q("BETA"), q("yan"), q("x_"), q("_"), q("%")),
        (1, 1, 1, 1, 0)
    );
    assert_eq!(q("  "), 4, "blank = no filter");
    // One account by id.
    assert_eq!(st.user_info(&a.user_id).expect("one").email, "zoe@alpha.fr");
    assert!(matches!(st.user_info("nope"), Err(AppError::NotFound(_))));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn deleting_an_account_erases_its_personal_data_only() {
    let dir = tmp("erase");
    let e = engine(&dir, 0);
    let st = e.store();
    let a = member(st, "A", 10, "erase.me@a.fr");
    let job = e.submit_as(spec(1), Some(&a)).expect("job");
    let token = st.reset_link("erase.me@a.fr").expect("reset link");
    assert_eq!(st.user_id("ERASE.ME@a.fr").expect("id"), a.user_id);

    assert_eq!(st.user_delete("erase.me@a.fr").expect("delete"), a.user_id);
    // No trace of the person in the database file.
    let c = rusqlite::Connection::open(dir.join("data/niche.db")).expect("db");
    for (table, col) in [
        ("users", "id"),
        ("sessions", "user_id"),
        ("links", "user_id"),
        ("jobs", "created_by"),
    ] {
        let n: i64 = c
            .query_row(
                &format!("SELECT COUNT(*) FROM {table} WHERE {col}=?1"),
                [&a.user_id],
                |r| r.get(0),
            )
            .expect("count");
        assert_eq!(n, 0, "{table}.{col} still references the account");
    }
    // The organisation keeps its job; the reset link no longer works; the e-mail is free again.
    assert!(e.job(&job.id, &Scope::Org(a.org_id.clone())).is_ok());
    assert!(st.link_info(&token).is_err());
    assert!(
        st.login("erase.me@a.fr", "a long enough password")
            .expect("login")
            .is_none()
    );
    assert!(matches!(
        st.user_delete("erase.me@a.fr"),
        Err(AppError::NotFound(_))
    ));
    let again = st.invite("A").expect("invite");
    st.accept_invite(&again, "Back", "erase.me@a.fr", "another long password")
        .expect("same e-mail can be invited again");
    let _ = std::fs::remove_dir_all(dir);
}

fn spain() -> serde_json::Value {
    serde_json::json!({"type": "Polygon", "coordinates": [[[-9.0, 36.5], [3.0, 36.5], [3.0, 43.5], [-9.0, 43.5], [-9.0, 36.5]]]})
}

#[test]
fn demo_outside_its_region_is_refused_and_failures_keep_their_stage() {
    let dir = tmp("demo-area");
    let e = engine(&dir, 0);
    let mut s = spec(1);
    s.area.geometry = spain();
    let err = e.submit_as(s, None).expect_err("demo over Spain");
    assert!(
        matches!(&err, AppError::Invalid(m) if m.contains("demo region")),
        "{err}"
    );

    // A real study cannot keep the demo region's SYNTHETIC label.
    let mut real = spec(1);
    real.demo = false;
    real.layers = vec!["chelsa_bio1".into()];
    real.area.name = "Synthetic demo region, SE France (SYNTHETIC)".into();
    let err = e
        .submit_as(real, None)
        .expect_err("real study on the demo area");
    assert!(
        matches!(&err, AppError::Invalid(m) if m.contains("synthetic demo region")),
        "{err}"
    );

    // A job that fails while preparing data (spec altered after validation) keeps that stage,
    // and its failure event is written before the terminal status.
    let job = e.submit_as(spec(1), None).expect("demo job");
    sql(
        &dir,
        &format!(
            "UPDATE jobs SET spec=json_set(spec,'$.area.geometry',json('{}')) WHERE id='{}'",
            spain(),
            job.id
        ),
    );
    let never = AtomicBool::new(false);
    e.run_job(&job.id, &never, &never);
    let j = e.job(&job.id, &Scope::All).expect("job");
    assert_eq!(j.status, JobStatus::Failed);
    assert!(
        !j.stage.is_empty() && j.stage != "Failed",
        "stage of the failure kept, got {:?}",
        j.stage
    );
    let evs = e.store().events_after(&job.id, 0).expect("events");
    let last = evs.last().expect("event");
    assert!(
        last.message.starts_with("Job failed: no land cells"),
        "{}",
        last.message
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn projects_group_studies_within_their_organisation() {
    let dir = tmp("projects");
    let e = engine(&dir, 0);
    let st = e.store();
    let a = member(st, "A", 10, "a@a.fr");
    let b = member(st, "B", 10, "b@b.fr");
    let (sa, sb) = (a.scope(), b.scope());

    let p1 = st
        .project_create(&sa, Some(&a), "  Client X — ZAC du port ")
        .expect("create");
    assert_eq!(p1.name, "Client X — ZAC du port");
    assert!(matches!(
        st.project_create(&sa, Some(&a), "client x — zac du port"),
        Err(AppError::Conflict(_))
    ));
    assert!(matches!(
        st.project_create(&sa, Some(&a), " "),
        Err(AppError::Invalid(_))
    ));
    // Same name in another organisation is fine; organisations never see each other's projects.
    st.project_create(&sb, Some(&b), "Client X — ZAC du port")
        .expect("other org");
    assert_eq!(st.project_list(&sa).expect("list").len(), 1);
    assert!(matches!(
        st.project(&p1.id, &sb),
        Err(AppError::NotFound(_))
    ));
    assert!(matches!(
        st.project_rename(&p1.id, &sb, "x"),
        Err(AppError::NotFound(_))
    ));
    assert!(matches!(
        st.project_delete(&p1.id, &sb),
        Err(AppError::NotFound(_))
    ));

    // A study submitted into a project, another moved there, a third renamed.
    let j1 = e
        .submit_in(spec(1), Some(&a), Some(&p1.id))
        .expect("in project");
    assert_eq!(j1.project_id.as_deref(), Some(p1.id.as_str()));
    assert_eq!(j1.created_by_name.as_deref(), Some("Test User"));
    assert!(matches!(
        e.submit_in(spec(1), Some(&b), Some(&p1.id)),
        Err(AppError::NotFound(_))
    ));
    let j2 = e.submit_as(spec(1), Some(&a)).expect("no project");
    st.job_update(&j2.id, &sa, Some(" Renamed "), Some(Some(&p1.id)))
        .expect("move");
    let j2 = e.job(&j2.id, &sa).expect("job");
    assert_eq!(
        (j2.name.as_str(), j2.project_id.as_deref()),
        ("Renamed", Some(p1.id.as_str()))
    );
    assert_eq!(st.project(&p1.id, &sa).expect("p").studies, 2);
    let pb = st.project_list(&sb).expect("list")[0].id.clone();
    assert!(matches!(
        st.job_update(&j2.id, &sa, None, Some(Some(&pb))),
        Err(AppError::NotFound(_))
    ));
    st.job_update(&j2.id, &sa, None, Some(None))
        .expect("remove from project");
    assert_eq!(e.job(&j2.id, &sa).expect("job").project_id, None);

    // Deleting a project keeps its studies, without project.
    st.project_rename(&p1.id, &sa, "Client X").expect("rename");
    st.project_delete(&p1.id, &sa).expect("delete");
    assert_eq!(e.job(&j1.id, &sa).expect("kept").project_id, None);
    assert!(st.project_list(&sa).expect("list").is_empty());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn monthly_metrics_measure_computing_gbif_waits_and_failure_causes() {
    let dir = tmp("metrics");
    let e = engine(&dir, 0);
    let p = member(e.store(), "Client", 50, "c@client.fr");
    let ok = e.submit_as(spec(2), Some(&p)).expect("job 1").id;
    let bad = e.submit_as(spec(1), Some(&p)).expect("job 2").id;
    let other_month = e.submit_as(spec(1), Some(&p)).expect("job 3").id;
    sql(
        &dir,
        &format!(
            "UPDATE jobs SET status='succeeded', created_at='2026-10-03T10:00:00Z',
                 prep_started_at='2026-10-03T10:00:00Z', prep_finished_at='2026-10-03T10:12:00Z',
                 started_at='2026-10-03T10:12:00Z', finished_at='2026-10-03T10:14:00.5Z' WHERE id='{ok}';
             UPDATE jobs SET status='failed', created_at='2026-10-05T09:00:00Z',
                 prep_started_at='2026-10-05T09:00:00Z', prep_finished_at='2026-10-05T09:00:00.01Z',
                 started_at='2026-10-05T09:00:01Z', finished_at='2026-10-05T09:00:31Z',
                 error='data source problem: GBIF download 1 ended with status KILLED' WHERE id='{bad}';
             UPDATE jobs SET created_at='2026-09-30T23:00:00Z' WHERE id='{other_month}';"
        ),
    );
    let m = e.store().metrics("2026-10").expect("metrics");
    assert_eq!(m.len(), 1);
    let m = &m[0];
    assert_eq!(
        (m.org_name.as_str(), m.studies, m.succeeded, m.failed),
        ("Client", 2, 1, 1)
    );
    assert_eq!(m.species, 3);
    assert!((m.compute_s - 150.5).abs() < 1e-6, "{}", m.compute_s);
    // 120.5 s for the 2 species of the succeeded study.
    assert!((m.compute_s_per_species.expect("per species") - 60.25).abs() < 1e-6);
    // The 10 ms preparation (no download) is not a GBIF wait.
    assert_eq!(m.gbif_wait_s_mean, Some(720.0));
    assert_eq!(m.failures.get("gbif"), Some(&1));
    assert_eq!(
        failure_cause("model fitting failed: every species failed; see events"),
        "model"
    );
    assert_eq!(
        failure_cause("the study area does not overlap the predictor grid"),
        "area"
    );
    let _ = std::fs::remove_dir_all(dir);
}

const KEY: &str = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";

fn engine_with(dir: &Path, key: Option<&str>, shared: bool) -> Engine {
    Engine::open(Config {
        data_dir: dir.join("data"),
        cache_dir: dir.join("cache"),
        max_records: 5000,
        gbif: shared.then(|| niche_data::gbif_download::Credentials {
            username: "ns-shared".into(),
            password: "shared-password".into(),
        }),
        secret_key: key.map(|k| SecretKey::from_hex(k).expect("key")),
        mirror: None,
        limits: Limits::default(),
        retention_days: 0,
    })
    .expect("engine")
}

#[test]
fn gbif_account_of_the_user_first_then_organisation_then_shared() {
    let dir = tmp("gbif-order");
    let e = engine_with(&dir, Some(KEY), true);
    let st = e.store();
    let alice = member(st, "Org A", 100, "alice@a.fr");
    let (user, org) = (
        GbifOwner::User(alice.user_id.clone()),
        GbifOwner::Org(alice.org_id.clone()),
    );
    let s0 = e.gbif_settings(&alice).expect("settings");
    assert_eq!(s0.effective, GbifSource::Shared);
    assert!(s0.can_store && s0.shared && s0.own.is_none() && s0.organisation.is_none());

    e.gbif_store(&org, "org-account", "org-password")
        .expect("org account");
    let s1 = e.gbif_settings(&alice).expect("settings");
    assert_eq!(s1.effective, GbifSource::Organisation);
    // A member who is not a manager sees that the organisation has an account, not its name.
    assert_eq!(
        s1.organisation.as_ref().map(|o| o.username.clone()),
        Some(None)
    );

    e.gbif_store(&user, "alice-gbif", "alice-password")
        .expect("own account");
    let s2 = e.gbif_settings(&alice).expect("settings");
    assert_eq!(s2.effective, GbifSource::User);
    assert_eq!(
        s2.own.and_then(|o| o.username).as_deref(),
        Some("alice-gbif")
    );

    // Passwords are sealed: never stored in clear.
    let c = rusqlite::Connection::open(dir.join("data/niche.db")).expect("db");
    let secrets: Vec<String> = c
        .prepare("SELECT gbif_secret FROM users UNION ALL SELECT gbif_secret FROM orgs")
        .expect("sql")
        .query_map([], |r| r.get::<_, Option<String>>(0))
        .expect("query")
        .flatten()
        .flatten()
        .collect();
    assert_eq!(secrets.len(), 2);
    assert!(
        secrets
            .iter()
            .all(|s| !s.contains("password") && s.len() > 40)
    );
    drop(c);

    // A study uses the account chosen when its preparation starts, and keeps it.
    let job = e.submit_as(spec(1), Some(&alice)).expect("job");
    assert_eq!(
        e.job_gbif_source(&job.id).expect("source"),
        GbifSource::User
    );
    e.gbif_store(&org, "org-account-2", "org-password-2")
        .expect("replace org account");
    assert_eq!(
        e.job_gbif_source(&job.id).expect("source"),
        GbifSource::User
    );
    // Removed (or refused by GBIF): the next account in line is used.
    st.gbif_clear(&user).expect("clear");
    assert_eq!(
        e.job_gbif_source(&job.id).expect("source"),
        GbifSource::Organisation
    );
    st.gbif_mark_checked(&org, Some(false)).expect("refused");
    assert_eq!(
        e.gbif_settings(&alice).expect("settings").effective,
        GbifSource::Shared
    );
    let job2 = e.submit_as(spec(1), Some(&alice)).expect("job");
    assert_eq!(
        e.job_gbif_source(&job2.id).expect("source"),
        GbifSource::Shared
    );
    // GBIF unreachable (`None`) keeps an account usable.
    st.gbif_mark_checked(&org, None).expect("unknown");
    assert_eq!(
        e.gbif_settings(&alice).expect("settings").effective,
        GbifSource::Organisation
    );

    // Input checks.
    assert!(matches!(
        e.gbif_store(&user, "my name", "x"),
        Err(AppError::Invalid(_))
    ));
    assert!(matches!(
        e.gbif_store(&user, "alice", ""),
        Err(AppError::Invalid(_))
    ));
}

#[test]
fn gbif_accounts_need_the_server_key() {
    let dir = tmp("gbif-key");
    {
        let e = engine_with(&dir, Some(KEY), false);
        let bob = member(e.store(), "Org B", 100, "bob@b.fr");
        e.gbif_store(
            &GbifOwner::User(bob.user_id.clone()),
            "bob-gbif",
            "bob-password",
        )
        .expect("stored");
        assert_eq!(
            e.gbif_settings(&bob).expect("settings").effective,
            GbifSource::User
        );
    }
    // Without the key: nothing can be stored, and stored accounts are not used.
    let e = engine_with(&dir, None, false);
    let bob = e
        .store()
        .login("bob@b.fr", "a long enough password")
        .expect("login")
        .expect("ok")
        .0;
    let s = e.gbif_settings(&bob).expect("settings");
    assert!(!s.can_store);
    assert_eq!(s.effective, GbifSource::None);
    assert!(matches!(
        e.gbif_store(&GbifOwner::User(bob.user_id.clone()), "bob", "x"),
        Err(AppError::Conflict(_))
    ));
    // With another key the sealed password does not open: skipped, not an error.
    let other = KEY.replace("00", "ff");
    let e = engine_with(&dir, Some(&other), false);
    assert_eq!(
        e.gbif_settings(&bob).expect("settings").effective,
        GbifSource::None
    );
}

#[test]
fn account_name_and_password_change() {
    let dir = tmp("account-self");
    let e = engine(&dir, 0);
    let st = e.store();
    let p = member(st, "Org C", 10, "carol@c.fr");
    st.user_set_name(&p.user_id, "  Carol Example ")
        .expect("name");
    assert!(matches!(
        st.user_set_name(&p.user_id, "  "),
        Err(AppError::Invalid(_))
    ));
    let (_, here) = st
        .login("carol@c.fr", "a long enough password")
        .expect("login")
        .expect("ok");
    let (_, elsewhere) = st
        .login("carol@c.fr", "a long enough password")
        .expect("login")
        .expect("ok");
    assert_eq!(
        st.session(&here).expect("session").map(|p| p.name),
        Some("Carol Example".into())
    );
    assert!(matches!(
        st.change_password(
            &p.user_id,
            "wrong password!!",
            "another long password",
            &here
        ),
        Err(AppError::Invalid(_))
    ));
    assert!(matches!(
        st.change_password(&p.user_id, "a long enough password", "short", &here),
        Err(AppError::Invalid(_))
    ));
    st.change_password(
        &p.user_id,
        "a long enough password",
        "another long password",
        &here,
    )
    .expect("changed");
    assert!(
        st.session(&here).expect("session").is_some(),
        "the current session stays open"
    );
    assert!(
        st.session(&elsewhere).expect("session").is_none(),
        "other sessions are closed"
    );
    assert!(
        st.login("carol@c.fr", "a long enough password")
            .expect("login")
            .is_none()
    );
    assert!(
        st.login("carol@c.fr", "another long password")
            .expect("login")
            .is_some()
    );
}

#[test]
fn only_members_of_the_operator_organisation_become_administrators() {
    let dir = tmp("operator");
    let e = engine(&dir, 0);
    let st = e.store();
    member(st, "Staff", 5, "staff@ns.fr");
    member(st, "Client", 5, "c@client.fr");
    st.org_set_operator("staff").expect("operator");
    assert!(matches!(
        st.user_set_admin("c@client.fr", true),
        Err(AppError::Conflict(_))
    ));
    st.user_set_admin("staff@ns.fr", true).expect("promote");
    assert!(st.admins_outside_operator().expect("outside").is_empty());
    // Designating another one moves the role's boundary, not the flag: reported, kept.
    st.org_set_operator("Client").expect("operator");
    let ops: Vec<_> = st
        .org_list()
        .expect("orgs")
        .into_iter()
        .filter(|o| o.is_operator)
        .map(|o| o.name)
        .collect();
    assert_eq!(ops, vec!["Client"], "at most one operator organisation");
    assert_eq!(st.admins_outside_operator().expect("outside").len(), 1);
    let staff = st.user_list(Some("Staff")).expect("users");
    assert!(staff[0].is_admin && !staff[0].in_operator_org);
    // Removing the flag needs no operator membership.
    st.user_set_admin("staff@ns.fr", false).expect("demote");
    assert!(st.admins_outside_operator().expect("outside").is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn audit_trail_pages_filters_and_outlives_erased_accounts() {
    let dir = tmp("audit");
    let e = engine(&dir, 0);
    let st = e.store();
    let admin = member(st, "Staff", 5, "admin@ns.fr");
    let client = member(st, "Client", 5, "c@client.fr");
    let c = st.account_ref("C@client.fr").expect("ref");
    assert_eq!(
        (c.user_id.as_str(), c.org_id.as_str()),
        (client.user_id.as_str(), client.org_id.as_str())
    );
    st.audit(
        &AuditEvent::web(&admin, AuditAction::OrgQuotaChanged)
            .org(&client.org_id)
            .detail(serde_json::json!({"from": 5, "to": 9})),
    )
    .expect("audit");
    st.audit(&AuditEvent::web(&admin, AuditAction::ManagerGranted).account(&c))
        .expect("audit");
    st.audit(&AuditEvent::cli(AuditAction::OrgCreated).org(&admin.org_id))
        .expect("audit");
    // Newest first; names joined; the CLI has no actor.
    let all = st.audit_list(&AuditFilter::default()).expect("list");
    let actions: Vec<_> = all.entries.iter().map(|e| e.action.as_str()).collect();
    assert_eq!(
        actions,
        ["org_created", "manager_granted", "org_quota_changed"]
    );
    assert_eq!(
        (
            all.entries[0].via.as_str(),
            all.entries[0].actor_id.as_deref()
        ),
        ("cli", None)
    );
    let m = &all.entries[1];
    assert_eq!(
        (
            m.actor_name.as_deref(),
            m.org_name.as_deref(),
            m.user_email.as_deref()
        ),
        (Some("Test User"), Some("Client"), Some("c@client.fr"))
    );
    assert_eq!(
        all.entries[2].detail,
        Some(serde_json::json!({"from": 5, "to": 9}))
    );
    // Filters: an organisation's history; what was done to or by an account.
    let org = AuditFilter {
        org: Some(client.org_id.clone()),
        ..AuditFilter::default()
    };
    assert_eq!(st.audit_list(&org).expect("org").entries.len(), 2);
    let by = AuditFilter {
        user: Some(admin.user_id.clone()),
        ..AuditFilter::default()
    };
    assert_eq!(st.audit_list(&by).expect("by").entries.len(), 2);
    // Pages: `next` is the id to continue before.
    let p1 = st
        .audit_list(&AuditFilter {
            limit: Some(2),
            ..AuditFilter::default()
        })
        .expect("page 1");
    assert_eq!(p1.entries.len(), 2);
    let p2 = st
        .audit_list(&AuditFilter {
            limit: Some(2),
            before: p1.next,
            ..AuditFilter::default()
        })
        .expect("page 2");
    assert_eq!((p2.entries.len(), p2.next), (1, None));
    assert_eq!(p2.entries[0].action, "org_quota_changed");
    // Erasing an account keeps its rows, without the person.
    st.user_delete("c@client.fr").expect("erase");
    let after = st.audit_list(&org).expect("org");
    assert_eq!(after.entries.len(), 2);
    let m = after
        .entries
        .iter()
        .find(|e| e.action == "manager_granted")
        .expect("kept");
    assert_eq!(
        (m.user_id.as_deref(), m.user_email.as_deref()),
        (Some(c.user_id.as_str()), None)
    );
    let _ = std::fs::remove_dir_all(&dir);
}
