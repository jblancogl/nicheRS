//! Application commands. Configuration precedence: CLI flag > `NICHE_*` environment variable >
//! default (docs/architecture/configuration.md).

use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand, ValueEnum};
use niche_app::audit::{AuditAction as A, AuditEvent, AuditFilter};
use niche_app::dto::{
    JobSpec, JobStatus, OccurrenceFilters, SpeciesRef, StudyArea, ValidationSettings,
};
use niche_app::{Config, Engine, Limits, demo as demo_data};

#[derive(Args, Clone)]
pub struct Common {
    /// Data directory (SQLite database, job outputs).
    #[arg(long, env = "NICHE_DATA_DIR")]
    pub data_dir: Option<PathBuf>,
    /// Cache directory (downloaded predictor windows, land mask).
    #[arg(long, env = "NICHE_CACHE_DIR")]
    pub cache_dir: Option<PathBuf>,
    /// Worker threads for numerical work (default: all cores).
    #[arg(long, env = "NICHE_MAX_WORKERS")]
    pub max_workers: Option<usize>,
    /// Maximum GBIF records retrieved per species.
    #[arg(long, env = "NICHE_MAX_RECORDS", default_value_t = 20_000)]
    pub max_records: usize,
    /// GBIF user name for batch occurrence downloads (DOI, no paging limits). Optional.
    #[arg(long, env = "NICHE_GBIF_USERNAME")]
    pub gbif_username: Option<String>,
    /// GBIF password (prefer the environment variable or a secret; never logged).
    #[arg(long, env = "NICHE_GBIF_PASSWORD", hide_env_values = true)]
    pub gbif_password: Option<String>,
    /// Key sealing the GBIF accounts users and organisations store (64 hex characters, e.g.
    /// `openssl rand -hex 32`). Without it those accounts can be neither saved nor used.
    #[arg(long, env = "NICHE_SECRET_KEY", hide_env_values = true)]
    pub secret_key: Option<String>,
    /// Regional GBIF mirror to keep (`fr-es`): studies inside the region read their records from
    /// it in seconds instead of waiting for a GBIF download (ADR-0114). Needs the shared GBIF
    /// account and tens of GB of disk.
    #[arg(long, env = "NICHE_GBIF_MIRROR")]
    pub gbif_mirror: Option<String>,
    /// Days between refreshes of the mirror.
    #[arg(long, env = "NICHE_GBIF_MIRROR_REFRESH_DAYS", default_value_t = 7)]
    pub gbif_mirror_refresh_days: u64,
    /// A mirror older than this many days is not used (studies download from GBIF instead).
    #[arg(long, env = "NICHE_GBIF_MIRROR_MAX_AGE_DAYS", default_value_t = 30)]
    pub gbif_mirror_max_age_days: u64,
    /// Log filter (e.g. info, debug, niche_app=debug).
    #[arg(long, env = "NICHE_LOG_LEVEL", default_value = "info")]
    pub log_level: String,
    /// Log format: pretty or json.
    #[arg(long, env = "NICHE_LOG_FORMAT", default_value = "pretty")]
    pub log_format: String,
}

#[derive(Args)]
pub struct ServeArgs {
    #[command(flatten)]
    pub common: Common,
    /// Bind address. Loopback by default; binding elsewhere exposes the server (see deployment-modes.md).
    #[arg(long, env = "NICHE_HOST", default_value = "127.0.0.1")]
    pub host: IpAddr,
    #[arg(long, env = "NICHE_PORT", default_value_t = 8080)]
    pub port: u16,
    /// Additional accepted Host names (comma-separated), e.g. behind a reverse proxy.
    /// How the deployment backs up the data volume, shown to administrators (e.g. "Longhorn,
    /// daily at 03:30, 14 kept"). The application cannot see platform backups itself.
    #[arg(long, env = "NICHE_VOLUME_BACKUP")]
    pub volume_backup: Option<String>,
    #[arg(long, env = "NICHE_ALLOWED_HOSTS", value_delimiter = ',')]
    pub allowed_hosts: Vec<String>,
    /// Serve the UI from a directory instead of the embedded build (development).
    #[arg(long, env = "NICHE_UI_DIR")]
    pub ui_dir: Option<PathBuf>,
    /// `none`: single user, no sign-in (local mode). `accounts`: hosted mode with organisations,
    /// invitation links, sessions and monthly quotas (ADR-0105).
    #[arg(long, env = "NICHE_AUTH", value_enum, default_value_t = AuthArg::None)]
    pub auth: AuthArg,
    /// Delete finished jobs after this many days (0 = never). Default: 0 without accounts,
    /// 365 with accounts.
    #[arg(long, env = "NICHE_RETENTION_DAYS")]
    pub retention_days: Option<u64>,
    /// Operator only: GitLab instance that receives every user feedback as an issue
    /// (ADR-0110), e.g. `https://gitlab.example.org`. Needs the project and token too.
    #[arg(long, env = "NICHE_FEEDBACK_GITLAB_URL")]
    pub feedback_gitlab_url: Option<String>,
    /// GitLab project (`group/project` or numeric id) for feedback issues.
    #[arg(long, env = "NICHE_FEEDBACK_GITLAB_PROJECT")]
    pub feedback_gitlab_project: Option<String>,
    /// Project access token (scope `api`, role Reporter); prefer the environment or a secret.
    #[arg(long, env = "NICHE_FEEDBACK_GITLAB_TOKEN", hide_env_values = true)]
    pub feedback_gitlab_token: Option<String>,
    /// Labels of every feedback issue (comma-separated); `feedback::<kind>` is added.
    #[arg(
        long,
        env = "NICHE_FEEDBACK_GITLAB_LABELS",
        value_delimiter = ',',
        default_value = "feedback"
    )]
    pub feedback_gitlab_labels: Vec<String>,
}

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum AuthArg {
    None,
    Accounts,
}

#[derive(Args)]
pub struct RunArgs {
    #[command(flatten)]
    pub common: Common,
    /// Job specification JSON file.
    pub spec: PathBuf,
}

#[derive(Args)]
pub struct CacheArgs {
    #[command(flatten)]
    pub common: Common,
    #[command(subcommand)]
    pub action: CacheAction,
}

#[derive(Subcommand)]
pub enum CacheAction {
    /// Show cache location and size.
    Inspect,
    /// Delete cached downloads (they are re-downloaded on demand).
    Clean,
}

#[derive(Args)]
pub struct MirrorArgs {
    #[command(flatten)]
    pub common: Common,
    #[command(subcommand)]
    pub action: MirrorAction,
}

#[derive(Subcommand)]
pub enum MirrorAction {
    /// Show the snapshot in use (the server refreshes it by itself).
    Status,
    /// Build the mirror from a GBIF download of the region made by hand (same predicate as
    /// `region.predicate`), then make it the one in use. The server picks it up on restart.
    Import {
        /// The SIMPLE_CSV archive (.zip) of the download.
        #[arg(long)]
        zip: PathBuf,
        /// Its GBIF download key (its DOI and date are cited by every study).
        #[arg(long)]
        key: String,
    },
}

fn dirs() -> Option<directories::ProjectDirs> {
    directories::ProjectDirs::from("eu", "Natural Solutions", "NicheRS")
}

/// The data directory `init` would use (without opening the engine).
pub fn data_dir(c: &Common) -> Result<PathBuf> {
    c.data_dir
        .clone()
        .or_else(|| dirs().map(|d| d.data_local_dir().to_path_buf()))
        .context("no data directory; set NICHE_DATA_DIR")
}

fn init(c: &Common) -> Result<Engine> {
    init_with(c, Limits::default(), 0)
}

fn init_with(c: &Common, limits: Limits, retention_days: u64) -> Result<Engine> {
    let filter = tracing_subscriber::EnvFilter::try_new(&c.log_level)
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    let builder = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr);
    let _ = if c.log_format == "json" {
        builder.json().try_init()
    } else {
        builder.try_init()
    };
    if let Some(n) = c.max_workers {
        rayon::ThreadPoolBuilder::new()
            .num_threads(n.max(1))
            .build_global()
            .context("configuring worker threads")?;
    }
    let pd = dirs();
    let data_dir = c
        .data_dir
        .clone()
        .or_else(|| pd.as_ref().map(|d| d.data_local_dir().to_path_buf()))
        .context("no data directory; set NICHE_DATA_DIR")?;
    let cache_dir = c
        .cache_dir
        .clone()
        .or_else(|| pd.as_ref().map(|d| d.cache_dir().to_path_buf()))
        .unwrap_or_else(|| data_dir.join("cache"));
    tracing::info!(data_dir = %data_dir.display(), cache_dir = %cache_dir.display(), "configuration");
    let gbif = match (&c.gbif_username, &c.gbif_password) {
        (Some(u), Some(p)) if !u.is_empty() && !p.is_empty() => {
            tracing::info!(user = %u, "GBIF batch downloads enabled");
            Some(niche_data::gbif_download::Credentials {
                username: u.clone(),
                password: p.clone(),
            })
        }
        _ => None,
    };
    let secret_key = match c.secret_key.as_deref().filter(|k| !k.is_empty()) {
        Some(k) => Some(niche_app::secrets::SecretKey::from_hex(k).context("NICHE_SECRET_KEY")?),
        None => None,
    };
    if secret_key.is_none() {
        tracing::info!(
            "NICHE_SECRET_KEY not set: users and organisations cannot store GBIF accounts"
        );
    }
    let mirror = match c
        .gbif_mirror
        .as_deref()
        .map(str::trim)
        .filter(|m| !m.is_empty())
    {
        Some(id) => {
            let region = niche_data::gbif_mirror::region(id).with_context(|| {
                let known: Vec<_> = niche_data::gbif_mirror::REGIONS
                    .iter()
                    .map(|r| r.id)
                    .collect();
                format!(
                    "unknown NICHE_GBIF_MIRROR {id:?} (known: {})",
                    known.join(", ")
                )
            })?;
            tracing::info!(region = id, "regional GBIF mirror enabled");
            Some(niche_app::MirrorConfig {
                region,
                refresh_days: c.gbif_mirror_refresh_days.max(1),
                max_age_days: c
                    .gbif_mirror_max_age_days
                    .max(c.gbif_mirror_refresh_days.max(1)),
            })
        }
        None => None,
    };
    Ok(Engine::open(Config {
        data_dir,
        cache_dir,
        max_records: c.max_records,
        gbif,
        secret_key,
        mirror,
        limits,
        retention_days,
    })?)
}

pub fn serve(a: ServeArgs) -> Result<()> {
    let accounts = a.auth == AuthArg::Accounts;
    let (limits, retention) = if accounts {
        (Limits::hosted(), a.retention_days.unwrap_or(365))
    } else {
        (Limits::default(), a.retention_days.unwrap_or(0))
    };
    let engine = Arc::new(init_with(&a.common, limits, retention)?);
    if accounts {
        tracing::info!(
            retention_days = retention,
            max_species = limits.max_species_per_job,
            "hosted mode: accounts, organisations and quotas enabled"
        );
        if a.allowed_hosts.is_empty() {
            tracing::warn!(
                "hosted mode without NICHE_ALLOWED_HOSTS: only localhost will be accepted"
            );
        }
    } else if !a.host.is_loopback() {
        tracing::warn!(host = %a.host, "binding to a non-loopback address: NicheRS has no authentication; expose it only behind an authenticating reverse proxy");
    }
    let feedback_gitlab = match (
        &a.feedback_gitlab_url,
        &a.feedback_gitlab_project,
        &a.feedback_gitlab_token,
    ) {
        (Some(url), Some(project), Some(token))
            if !url.is_empty() && !project.is_empty() && !token.is_empty() =>
        {
            tracing::info!(%url, %project, "feedback forwarded to GitLab issues");
            Some(niche_app::feedback::GitLabProject {
                url: url.clone(),
                project: project.clone(),
                token: token.clone(),
                labels: a
                    .feedback_gitlab_labels
                    .iter()
                    .map(|l| l.trim().to_string())
                    .filter(|l| !l.is_empty())
                    .collect(),
            })
        }
        (None, None, None) => None,
        _ => {
            tracing::warn!(
                "NICHE_FEEDBACK_GITLAB_URL, _PROJECT and _TOKEN must all be set: feedback is kept in NicheRS only"
            );
            None
        }
    };
    let opts = niche_server::ServerOptions {
        allowed_hosts: a.allowed_hosts,
        ui_dir: a.ui_dir,
        build: build_id(),
        accounts,
        volume_backup: a.volume_backup.clone(),
        feedback_gitlab,
    };
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    rt.block_on(niche_server::serve(
        engine,
        SocketAddr::new(a.host, a.port),
        opts,
    ))?;
    Ok(())
}

fn build_id() -> String {
    option_env!("NICHE_BUILD_ID").unwrap_or("dev").to_string()
}

fn run_spec(engine: &Engine, spec: JobSpec) -> Result<()> {
    let job = engine.submit(spec)?;
    eprintln!("job {} queued", job.id);
    let never = AtomicBool::new(false);
    engine.run_job(&job.id, &never, &never);
    let res = engine.results(&job.id)?;
    println!("job {} — {:?}", res.job.id, res.job.status);
    println!(
        "{:<40} {:>10} {:>12} {:>7} {:>6} {:>14}",
        "species", "status", "evidence", "Boyce", "OR10", "area P10 km²"
    );
    for r in &res.species {
        let p10 = r
            .thresholds
            .iter()
            .find(|t| t.rule == "p10")
            .map_or(f64::NAN, |t| t.area_km2);
        println!(
            "{:<40} {:>10} {:>12} {:>7} {:>6.2} {:>14.0}",
            r.scientific_name,
            format!("{:?}", r.status).to_lowercase(),
            format!("{:?}", r.evidence.level).to_lowercase(),
            r.validation
                .cbi_mean
                .map_or("–".into(), |c| format!("{c:.2}")),
            r.validation.or10_mean,
            p10
        );
    }
    println!(
        "outputs: {}",
        engine.cfg.data_dir.join("jobs").join(&res.job.id).display()
    );
    if res.job.status != JobStatus::Succeeded {
        bail!("job did not succeed: {}", res.job.error.unwrap_or_default());
    }
    Ok(())
}

pub fn run(a: RunArgs) -> Result<()> {
    let engine = init(&a.common)?;
    let text = std::fs::read_to_string(&a.spec)
        .with_context(|| format!("reading {}", a.spec.display()))?;
    let spec: JobSpec = serde_json::from_str(&text).context("invalid job spec")?;
    run_spec(&engine, spec)
}

pub fn demo(c: Common) -> Result<()> {
    let engine = init(&c)?;
    let spec = JobSpec {
        name: "Synthetic demonstration".into(),
        species: vec![SpeciesRef {
            key: demo_data::SPECIES_KEY,
            scientific_name: demo_data::SPECIES_NAME.into(),
            own: None,
        }],
        area: StudyArea {
            name: "Demo region (synthetic)".into(),
            geometry: demo_data::area_geojson(),
            places: vec![],
        },
        layers: demo_data::LAYERS.iter().map(|l| l.0.to_string()).collect(),
        resolution_factor: 1,
        filters: OccurrenceFilters::default(),
        model: Default::default(),
        validation: ValidationSettings::default(),
        seed: 1,
        demo: true,
        own_dataset: None,
        projections: vec![],
    };
    run_spec(&engine, spec)
}

pub fn doctor(c: Common) -> Result<()> {
    let engine = init(&c)?;
    let mut ok = true;
    let mut check = |name: &str, r: Result<String>| match r {
        Ok(m) => println!("✓ {name}: {m}"),
        Err(e) => {
            ok = false;
            println!("✗ {name}: {e:#}");
        }
    };
    check(
        "data directory",
        Ok(engine.cfg.data_dir.display().to_string()),
    );
    check(
        "database",
        engine
            .store()
            .list(&niche_app::accounts::Scope::All)
            .map(|j| format!("{} jobs", j.len()))
            .map_err(Into::into),
    );
    check(
        "cache directory",
        Ok(engine.cfg.cache_dir.display().to_string()),
    );
    check(
        "GBIF API",
        niche_data::gbif::search_species(engine.http(), "Lacerta")
            .map(|h| format!("reachable ({} hits)", h.len()))
            .map_err(Into::into),
    );
    check(
        "CHELSA v2.1",
        niche_data::chelsa::native_grid(engine.http(), &engine.cfg.cache_dir)
            .map(|g| format!("grid {}×{} at {:.6}°", g.width, g.height, g.cell))
            .map_err(Into::into),
    );
    check(
        "worker threads",
        Ok(rayon::current_num_threads().to_string()),
    );
    if !ok {
        bail!("some checks failed (offline use is possible with cached data and the demo)");
    }
    Ok(())
}

fn dir_size(p: &std::path::Path) -> u64 {
    std::fs::read_dir(p)
        .map(|rd| {
            rd.filter_map(Result::ok)
                .map(|e| match e.metadata() {
                    Ok(m) if m.is_dir() => dir_size(&e.path()),
                    Ok(m) => m.len(),
                    Err(_) => 0,
                })
                .sum()
        })
        .unwrap_or(0)
}

pub fn mirror(a: MirrorArgs) -> Result<()> {
    let engine = init(&a.common)?;
    let status = match a.action {
        MirrorAction::Status => engine.system_status()?.gbif_mirror,
        MirrorAction::Import { zip, key } => Some(engine.mirror_import(&zip, &key)?),
    };
    match status {
        Some(s) => println!("{}", serde_json::to_string_pretty(&s)?),
        None => println!("no regional mirror configured (set NICHE_GBIF_MIRROR, e.g. fr-es)"),
    }
    Ok(())
}

pub fn cache(a: CacheArgs) -> Result<()> {
    let engine = init(&a.common)?;
    let dir = engine.cfg.cache_dir.clone();
    match a.action {
        CacheAction::Inspect => {
            println!("{} — {:.1} MB", dir.display(), dir_size(&dir) as f64 / 1e6)
        }
        CacheAction::Clean => {
            // gbif-downloads: per-taxon splits of GBIF batch downloads (archives are already
            // deleted once split, since they carry observer names).
            for sub in ["chelsa", "naturalearth", "gbif", "gbif-downloads"] {
                let p = dir.join(sub);
                if p.exists() {
                    std::fs::remove_dir_all(&p)
                        .with_context(|| format!("removing {}", p.display()))?;
                }
            }
            let _ = std::fs::remove_file(dir.join("chelsa_grid.json"));
            println!("cache cleaned: {}", dir.display());
        }
    }
    Ok(())
}

#[derive(Args)]
pub struct ExportArgs {
    #[command(flatten)]
    pub common: Common,
    /// Job specification JSON file.
    pub spec: PathBuf,
    /// Output directory (one sub-directory per species key).
    #[arg(long)]
    pub out: PathBuf,
}

pub fn export_swd(a: ExportArgs) -> Result<()> {
    let engine = init(&a.common)?;
    let spec: JobSpec =
        serde_json::from_str(&std::fs::read_to_string(&a.spec)?).context("invalid job spec")?;
    for d in engine.export_swd(&spec, &a.out)? {
        println!("{}", d.display());
    }
    Ok(())
}

#[derive(Args)]
pub struct AdminArgs {
    #[command(flatten)]
    pub common: Common,
    /// Public base URL of the hosted instance, used to print invitation and reset links
    /// (e.g. https://niche.example.org).
    #[arg(long, env = "NICHE_PUBLIC_URL")]
    pub public_url: Option<String>,
    #[command(subcommand)]
    pub action: AdminAction,
}

#[derive(Subcommand)]
pub enum AdminAction {
    /// Organisations (clients) and their monthly quota of modelled species.
    #[command(subcommand)]
    Org(OrgAction),
    /// Print a one-time invitation link into an organisation (valid 7 days). NicheRS sends no
    /// e-mail: hand the link over yourself.
    Invite {
        #[arg(long)]
        org: String,
    },
    /// Accounts.
    #[command(subcommand)]
    User(UserAction),
    /// Audit trail of the administration, newest first (ADR-0113).
    Audit {
        /// Only this organisation (name or id).
        #[arg(long)]
        org: Option<String>,
        /// Only what was done to or by this account (e-mail).
        #[arg(long)]
        user: Option<String>,
        #[arg(long, default_value_t = 50)]
        limit: usize,
    },
    /// Modelled species per organisation and month, as CSV (billing).
    Usage {
        #[arg(long)]
        org: Option<String>,
        /// Month `YYYY-MM` (UTC). Default: every month.
        #[arg(long)]
        month: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum OrgAction {
    Create {
        name: String,
        /// Modelled species per calendar month.
        #[arg(long)]
        quota: u64,
        /// Make it the operator organisation (ADR-0112). "Natural Solutions" becomes it by
        /// default when none is designated yet.
        #[arg(long)]
        operator: bool,
    },
    List,
    /// Designate the operator organisation (ADR-0112): only its members can be administrators.
    /// Replaces the previous one; existing administrators keep the flag.
    Operator {
        name: String,
    },
    /// Change the monthly quota.
    Quota {
        name: String,
        quota: u64,
    },
    /// Disable the organisation and sign out its users (data is kept).
    Disable {
        name: String,
    },
    Enable {
        name: String,
    },
    /// Give the jobs created before hosted mode (no organisation) to this organisation.
    Adopt {
        name: String,
    },
    /// Revoke every unused invitation link of the organisation.
    RevokeInvites {
        name: String,
    },
}

#[derive(Subcommand)]
pub enum UserAction {
    List {
        #[arg(long)]
        org: Option<String>,
    },
    /// Print a one-time password reset link (valid 7 days; earlier reset links are revoked).
    Reset {
        email: String,
    },
    /// Disable the account and close its sessions.
    Disable {
        email: String,
    },
    Enable {
        email: String,
    },
    /// Make the account a Natural Solutions administrator (web administration pages, read
    /// access to every organisation's runs). Only possible from this command line.
    Promote {
        email: String,
    },
    /// Remove the administrator flag.
    Demote {
        email: String,
    },
    /// Make the account a manager of its organisation (ADR-0109): invites colleagues, creates
    /// reset links and disables accounts of that organisation only.
    Manager {
        email: String,
        /// Remove the role instead.
        #[arg(long)]
        off: bool,
    },
    /// Erase the account (GDPR right to erasure): e-mail, name, password, sessions and reset
    /// links. The organisation's jobs and usage are kept. Irreversible.
    Delete {
        email: String,
        /// Confirm the irreversible deletion.
        #[arg(long)]
        yes: bool,
    },
}

fn link(public_url: Option<&str>, page: &str, token: &str) -> Result<String> {
    let base = public_url
        .map(|u| u.trim_end_matches('/'))
        .filter(|u| !u.is_empty())
        .context("set --public-url or NICHE_PUBLIC_URL to print links")?;
    // The token travels in the fragment: browsers never send it to the server or in Referer.
    Ok(format!("{base}/{page}#{token}"))
}

fn csv(field: &str) -> String {
    if field.contains([',', '"', '\n']) {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

pub fn admin(a: AdminArgs) -> Result<()> {
    let engine = init(&a.common)?;
    let st = engine.store();
    let url = a.public_url.as_deref();
    // Every change made here is recorded as made on the server (ADR-0113).
    let audit = |ev: AuditEvent| st.audit(&ev);
    let account = |email: &str| st.account_ref(email);
    match a.action {
        AdminAction::Org(o) => match o {
            OrgAction::Create {
                name,
                quota,
                operator,
            } => {
                let org = st.org_create(&name, quota)?;
                audit(
                    AuditEvent::cli(A::OrgCreated)
                        .org(&org.id)
                        .detail(serde_json::json!({"quota": org.monthly_quota})),
                )?;
                println!(
                    "organisation {} created (quota {} species/month)",
                    org.name, org.monthly_quota
                );
                let default_operator = org
                    .name
                    .eq_ignore_ascii_case(niche_app::accounts::DEFAULT_OPERATOR_ORG)
                    && st.operator_org()?.is_none();
                if operator || default_operator {
                    st.org_set_operator(&org.id)?;
                    audit(AuditEvent::cli(A::OperatorDesignated).org(&org.id))?;
                    println!("{} is the operator organisation", org.name);
                }
            }
            OrgAction::Operator { name } => {
                let org = st.org_set_operator(&name)?;
                audit(AuditEvent::cli(A::OperatorDesignated).org(&org.id))?;
                println!(
                    "{} is now the operator organisation: only its members can be administrators",
                    org.name
                );
                let outside = st.admins_outside_operator()?.len();
                if outside > 0 {
                    eprintln!(
                        "{outside} administrator(s) belong to another organisation and keep the flag; \
                         see `niche admin user list` and remove it with `niche admin user demote`"
                    );
                }
            }
            OrgAction::List => {
                println!(
                    "{:<32} {:>8} {:>8} {:>9}  status",
                    "organisation", "quota", "used", "reserved"
                );
                for org in st.org_list()? {
                    let q = st.quota(&org.id, org.monthly_quota)?;
                    println!(
                        "{:<32} {:>8} {:>8} {:>9}  {}",
                        org.name,
                        org.monthly_quota,
                        q.used,
                        q.reserved,
                        match (org.disabled_at.is_some(), org.is_operator) {
                            (true, true) => "disabled, operator",
                            (true, false) => "disabled",
                            (false, true) => "active, operator",
                            (false, false) => "active",
                        }
                    );
                }
            }
            OrgAction::Quota { name, quota } => {
                let from = st.org(&name)?.monthly_quota;
                let org = st.org_set_quota(&name, quota)?;
                audit(
                    AuditEvent::cli(A::OrgQuotaChanged)
                        .org(&org.id)
                        .detail(serde_json::json!({"from": from, "to": org.monthly_quota})),
                )?;
                println!("{}: quota {} species/month", org.name, org.monthly_quota);
            }
            OrgAction::Disable { name } => {
                let org = st.org_set_disabled(&name, true)?;
                audit(AuditEvent::cli(A::OrgDisabled).org(&org.id))?;
                println!("{} disabled; its users are signed out", org.name);
            }
            OrgAction::Enable { name } => {
                let org = st.org_set_disabled(&name, false)?;
                audit(AuditEvent::cli(A::OrgEnabled).org(&org.id))?;
                println!("{} enabled", org.name);
            }
            OrgAction::Adopt { name } => {
                let n = st.org_adopt_unowned(&name)?;
                println!("{n} job(s) without organisation now belong to {name}");
            }
            OrgAction::RevokeInvites { name } => {
                let n = st.revoke_invites(&name)?;
                if n > 0 {
                    audit(
                        AuditEvent::cli(A::InviteRevoked)
                            .org(st.org(&name)?.id)
                            .detail(serde_json::json!({"count": n})),
                    )?;
                }
                println!("{n} unused link(s) revoked");
            }
        },
        AdminAction::Invite { org } => {
            let link = link(url, "invite", &st.invite(&org)?)?;
            audit(AuditEvent::cli(A::InviteCreated).org(st.org(&org)?.id))?;
            println!("{link}");
            eprintln!(
                "one-time invitation into {org}, valid {} days; send it through a trusted channel",
                niche_app::accounts::INVITE_DAYS
            );
        }
        AdminAction::User(u) => match u {
            UserAction::List { org } => {
                println!(
                    "{:<36} {:<28} {:<24} {:<8} {:<16} last sign-in",
                    "e-mail", "name", "organisation", "status", "role"
                );
                for user in st.user_list(org.as_deref())? {
                    println!(
                        "{:<36} {:<28} {:<24} {:<8} {:<16} {}",
                        user.email,
                        user.name,
                        user.org_name,
                        if user.disabled_at.is_some() {
                            "disabled"
                        } else {
                            "active"
                        },
                        match (user.is_admin, user.in_operator_org) {
                            (true, true) => "admin",
                            (true, false) => "admin (outside)",
                            (false, _) => "member",
                        },
                        user.last_login_at
                            .as_deref()
                            .and_then(|t| t.get(..16))
                            .unwrap_or("never")
                    );
                }
            }
            UserAction::Reset { email } => {
                let link = link(url, "reset", &st.reset_link(&email)?)?;
                audit(AuditEvent::cli(A::ResetLinkCreated).account(&account(&email)?))?;
                println!("{link}");
                eprintln!(
                    "one-time password reset link for {email}, valid {} days",
                    niche_app::accounts::RESET_DAYS
                );
            }
            UserAction::Disable { email } => {
                st.user_set_disabled(&email, true)?;
                audit(AuditEvent::cli(A::AccountDisabled).account(&account(&email)?))?;
                println!("{email} disabled; its sessions are closed");
            }
            UserAction::Enable { email } => {
                st.user_set_disabled(&email, false)?;
                audit(AuditEvent::cli(A::AccountEnabled).account(&account(&email)?))?;
                println!("{email} enabled");
            }
            UserAction::Promote { email } => {
                st.user_set_admin(&email, true)?;
                audit(AuditEvent::cli(A::AdminGranted).account(&account(&email)?))?;
                println!("{email} is now an administrator (effective on the next request)");
            }
            UserAction::Demote { email } => {
                st.user_set_admin(&email, false)?;
                audit(AuditEvent::cli(A::AdminRemoved).account(&account(&email)?))?;
                println!("{email} is no longer an administrator");
            }
            UserAction::Manager { email, off } => {
                st.user_set_manager(&email, !off)?;
                let action = if off {
                    A::ManagerRemoved
                } else {
                    A::ManagerGranted
                };
                audit(AuditEvent::cli(action).account(&account(&email)?))?;
                if off {
                    println!("{email} no longer manages its organisation");
                } else {
                    println!(
                        "{email} now manages its organisation (effective on the next request)"
                    );
                }
            }
            UserAction::Delete { email, yes } => {
                if !yes {
                    anyhow::bail!(
                        "deleting {email} is irreversible; run again with --yes to confirm"
                    );
                }
                let erased = account(&email)?;
                let id = st.user_delete(&email)?;
                audit(AuditEvent::cli(A::AccountDeleted).account(&erased))?;
                println!("account {id} ({email}) erased; its organisation's jobs are kept");
            }
        },
        AdminAction::Audit { org, user, limit } => {
            let org = org.map(|o| st.org(&o)).transpose()?.map(|o| o.id);
            let user = user.map(|e| account(&e)).transpose()?.map(|a| a.user_id);
            let page = st.audit_list(&AuditFilter {
                org,
                user,
                before: None,
                limit: Some(limit),
            })?;
            println!(
                "{:<20} {:<4} {:<22} {:<24} {:<24} {:<28} detail",
                "at", "via", "action", "by", "organisation", "account"
            );
            let or_dash = |v: Option<&str>| v.unwrap_or("—").to_string();
            for e in &page.entries {
                println!(
                    "{:<20} {:<4} {:<22} {:<24} {:<24} {:<28} {}",
                    e.at.get(..19).unwrap_or(&e.at),
                    e.via,
                    e.action,
                    if e.via == "cli" {
                        "server".to_string()
                    } else {
                        or_dash(e.actor_name.as_deref().or(e.actor_id.as_deref()))
                    },
                    or_dash(e.org_name.as_deref()),
                    or_dash(e.user_email.as_deref().or(e.user_id.as_deref())),
                    e.detail.as_ref().map(|d| d.to_string()).unwrap_or_default()
                );
            }
            if page.next.is_some() {
                eprintln!("older entries exist; raise --limit to see them");
            }
        }
        AdminAction::Usage { org, month } => {
            let rows = st.usage(org.as_deref(), month.as_deref())?;
            println!("organisation,month,job_id,species_key,counted_at");
            for r in &rows {
                println!(
                    "{},{},{},{},{}",
                    csv(&r.org_name),
                    r.month,
                    r.job_id,
                    r.species_key,
                    r.at
                );
            }
            eprintln!("{} modelled species", rows.len());
        }
    }
    Ok(())
}
