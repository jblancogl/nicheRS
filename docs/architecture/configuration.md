# Configuration

Status: proposal (research phase, no code under `crates/` yet). Date: 2026-10-15.
Related: [twelve-factor-compliance.md](twelve-factor-compliance.md), [deployment-modes.md](deployment-modes.md),
[ADR-0101](architecture-decisions/ADR-0101-config-precedence.md).

## Scope

All *deployment* settings of the `niche` executable: names, sources, defaults, validation, precedence,
platform directories, secrets, and the `niche config show` command. *Experiment definitions*
(what is modelled) are a separate input class and are covered in section 6.

## 1. Two kinds of input, never mixed

| | Deployment config | Experiment definition |
|---|---|---|
| Question it answers | *Where/how* does this instance run? | *What* science is computed? |
| Examples | host, port, data dir, workers, log format, auth | species keys, study area, predictors, algorithm + hyper-parameters, CV scheme, seeds |
| Sources | CLI > env > config file > defaults | A versioned file (`*.niche.toml`) or API payload only |
| Read from env vars? | Yes | **Never** |
| Hashed into reproducibility manifest | No (only result-relevant execution facts such as worker count are recorded, not hashed) | Yes (canonical form → `spec_hash`) |
| May change results? | Must not (see tension 4 in [twelve-factor-compliance.md](twelve-factor-compliance.md)) | Yes, by definition |

[REC] Rationale: 12factor defines config as "everything that is likely to vary between deploys" [2];
an experiment definition does not vary between deploys — it must give the same result on every
deploy — so it is *data/input*, not config.

## 2. Precedence

```
CLI flag  >  NICHE_* environment variable  >  config file  >  built-in default
```

- [REC] Exactly one config file is read: `--config <path>` / `NICHE_CONFIG`, otherwise
  `<config_dir>/config.toml` if it exists. No cascading `/etc` + user + project files (avoids the
  "which file won?" problem; 12factor warns about config grouping explosion [2]).
- [REC] Merging is done field-by-field with provenance tracking (each resolved value remembers
  `cli | env | file:<path> | default`). See ADR-0101 for why this is hand-written over clap + serde
  rather than `figment`/`config`.
- [FACT] clap's `Arg::env` (crate feature `env`) already resolves CLI over env over `default_value`
  [10]. [REC] We therefore do **not** give clap `default_value`s; clap yields `Option<T>` for
  CLI/env, then the file layer and defaults fill the gaps. This preserves CLI > env > file > default.
- [REC] Empty environment variable (`NICHE_PORT=`) is treated as **unset** (documented; clap's own
  behaviour for empty values was not verified, so we normalise before parsing).
- [REC] Unknown keys in the config file are an error (`#[serde(deny_unknown_fields)]`). Unknown
  `NICHE_*` env vars produce a warning with a "did you mean" hint (typo detection).
- [REC] Validation runs once on the merged result, before any port is bound or file opened; failures
  exit with code 78 (`EX_CONFIG`) and list *every* invalid field with its source.

## 3. Settings reference

Config file is TOML; keys are shown dotted. Booleans accept `true/false/1/0`. Durations are integer
seconds; sizes integer MB/GB as named. "Mode default" = differs between Local/Server/Batch (see
[deployment-modes.md](deployment-modes.md)).

### 3.1 Core / process

| Env var | CLI flag | File key | Default | Validation |
|---|---|---|---|---|
| `NICHE_CONFIG` | `--config` | — | `<config_dir>/config.toml` if present | file must exist if explicitly given |
| `NICHE_MODE` | `--mode` (on `serve`) | `mode` | `local` for `serve`; `batch` implied by `niche run` | `local` \| `server` |
| `NICHE_HOST` | `--host` | `server.host` | `127.0.0.1` | IP literal or `localhost`; non-loopback in `local` mode → error unless `--mode server` |
| `NICHE_PORT` | `--port` | `server.port` | `8740` [REC; arbitrary, check IANA before release]; local mode falls back to a free port if busy | 0..=65535; 0 = ephemeral (local mode prints/opens the URL) |
| `NICHE_BASE_PATH` | `--base-path` | `server.base_path` | `/` | starts and ends with `/`; for reverse-proxy sub-path |
| `NICHE_PUBLIC_URL` | `--public-url` | `server.public_url` | unset | absolute http(s) URL; used for links/CORS defaults behind a proxy |
| `NICHE_DATA_DIR` | `--data-dir` | `paths.data_dir` | platform default (§4) | must be creatable, writable, on a local filesystem (SQLite WAL [24]) |
| `NICHE_CACHE_DIR` | `--cache-dir` | `paths.cache_dir` | platform default (§4) | writable; may be on a different volume than data dir |
| `NICHE_OPEN_BROWSER` | `--open` / `--no-open` | `ui.open_browser` | `true` in local, `false` otherwise | bool |
| `NICHE_OFFLINE` | `--offline` | `network.offline` | `false` | bool; when true any remote fetch fails fast with `E_OFFLINE` |

### 3.2 Resources and concurrency

| Env var | CLI flag | File key | Default | Validation |
|---|---|---|---|---|
| `NICHE_MAX_WORKERS` | `--max-workers` | `jobs.max_workers` | local: `max(1, cpus/4)`; server: `max(1, cpus/2)`; batch: 1 | 1..=cpus; number of *concurrently running jobs* |
| `NICHE_CPU_THREADS` | `--cpu-threads` | `jobs.cpu_threads` | `available_parallelism()` | 1..=cpus; size of the single shared Rayon pool (no per-job pools → no oversubscription) |
| `NICHE_MAX_MEMORY_MB` | `--max-memory-mb` | `jobs.max_memory_mb` | 50 % of detected physical/cgroup memory | ≥ 512; soft budget for admission control, not an OS limit |
| `NICHE_MAX_QUEUED_JOBS` | — | `jobs.max_queued` | 1000 | ≥ 1; beyond → HTTP 429 / CLI error |
| `NICHE_JOB_ISOLATION` | `--job-isolation` | `jobs.isolation` | `in-process` | `in-process` \| `subprocess` (ADR-0103) |
| `NICHE_JOB_MAX_ATTEMPTS` | — | `jobs.max_attempts` | 3 | 1..=10; applies to interruption requeue + transient fetch errors |
| `NICHE_SHUTDOWN_GRACE_SECS` | `--shutdown-grace` | `jobs.shutdown_grace_secs` | 30 | 1..=3600; must be < service manager stop timeout (doctor warns) |
| `NICHE_CACHE_MAX_GB` | — | `cache.max_gb` | 20 | ≥ 1; soft quota used by `cache clean --auto` and a startup warning |
| `NICHE_MAX_UPLOAD_MB` | — | `server.max_upload_mb` | local 1024, server 200 | ≥ 1; request body limit for uploads (occurrence CSV, study-area files) |

### 3.3 Logging and observability

| Env var | CLI flag | File key | Default | Validation |
|---|---|---|---|---|
| `NICHE_LOG_LEVEL` | `-v/-q`, `--log-level` | `log.level` | `info` | `tracing_subscriber::EnvFilter` directive syntax, e.g. `info,niche_engine=debug` |
| `NICHE_LOG_FORMAT` | `--log-format` | `log.format` | `auto` (pretty if stderr is a TTY, else JSON) | `auto` \| `pretty` \| `json` |
| `NICHE_LOG_FILE` | `--log-file` | `log.file` | unset (stdout/stderr only) | path; *deviation* from factor XI, local desktop only (see compliance doc) |
| `NICHE_LOG_REDACT_COORDS` | — | `log.redact_coordinates` | `true` | bool; `false` only allowed with `log.level` containing `debug`/`trace` and prints a warning |
| `NICHE_METRICS` | `--metrics` | `observability.metrics` | `false` | bool; exposes `/metrics` (observability.md) |

### 3.4 Security (server-relevant, also valid locally)

| Env var | CLI flag | File key | Default | Validation |
|---|---|---|---|---|
| `NICHE_AUTH` (**implemented**, ADR-0105) | `--auth` (on `serve`) | — | `none` | `none` (single user, local) \| `accounts` (hosted: organisations, invitation links, sessions, quotas) |
| `NICHE_VOLUME_BACKUP` (**implemented**) | `--volume-backup` | — | unset | text shown to administrators: how the deployment backs up the data volume (the app cannot see platform backups; it also keeps 7 daily database copies in `<data_dir>/backups/`, ADR-0109) |
| `NICHE_RETENTION_DAYS` (**implemented**) | `--retention-days` | — | `0` with `none`, `365` with `accounts` | days; finished jobs older than this are deleted; 0 = never |
| `NICHE_PUBLIC_URL` for `niche admin` (**implemented**) | `--public-url` | — | unset | base URL used to print invitation / reset links |
| `NICHE_AUTH_MODE` | `--auth` | `auth.mode` | local: `launch-token`; server: **no default — must be set** | `launch-token` (per-launch random token in the opened URL → `SameSite=Strict` cookie; 12-security-and-licensing.md) \| `token` \| `proxy` \| `none` (loopback only, for scripting; warning); `none` with non-loopback host → error |
| `NICHE_AUTH_TOKEN_FILE` | — | `auth.token_file` | unset | readable file, ≥ 32 bytes after trim, permissions not world-readable (Unix) |
| `NICHE_AUTH_TOKEN` | — | *not allowed in file* | unset | discouraged; warning logged; mutually exclusive with `_FILE` |
| `NICHE_AUTH_PROXY_HEADER` | — | `auth.proxy_header` | `X-Forwarded-User` | header name; honoured only from `trusted_proxies` |
| `NICHE_TRUSTED_PROXIES` | — | `server.trusted_proxies` | empty | comma list of IPs/CIDRs |
| `NICHE_ALLOWED_HOSTS` | — | `server.allowed_hosts` | local: `localhost,127.0.0.1,[::1]` (+port); server: host of `public_url` | Host-header allow-list (DNS-rebinding defence) |
| `NICHE_ALLOWED_ORIGINS` | — | `server.allowed_origins` | same-origin only | list of origins for CORS; `*` forbidden when auth ≠ none |

### 3.5 External data providers

| Env var | File key | Default | Notes |
|---|---|---|---|
| `NICHE_GBIF_USERNAME` | — | unset | shared GBIF account for batch downloads (DOI); the last resort after the user's and the organisation's own accounts (ADR-0110) |
| `NICHE_GBIF_PASSWORD` | — | unset | secret (environment / Kubernetes secret); never logged |
| `NICHE_GBIF_MIRROR` | — | unset | regional GBIF mirror to keep (`fr-es`): studies inside the region read their records from it instead of waiting for a GBIF download (ADR-0114). Needs the shared GBIF account and ~40 GB free at refresh time |
| `NICHE_GBIF_MIRROR_REFRESH_DAYS` | — | 7 | days between refreshes |
| `NICHE_GBIF_MIRROR_MAX_AGE_DAYS` | — | 30 | an older mirror is not used (studies download from GBIF) |
| `NICHE_SECRET_KEY` | — | unset | 64 hex characters (`openssl rand -hex 32`): seals the GBIF accounts users and organisations store (AES-256-GCM, ADR-0110). Without it they can be neither saved nor used. Changing it makes stored accounts unreadable (they are skipped; users enter them again) |
| `NICHE_HTTP_TIMEOUT_SECS` | `network.timeout_secs` | 60 | 1..=3600 |

Standard `HTTPS_PROXY`/`NO_PROXY` are honoured if the HTTP client supports them [OPEN: depends on the
HTTP client chosen in 11-backend-architecture.md].

Anything not listed here is **not configurable** (deliberately): SQLite pragmas (WAL, `synchronous=FULL`,
`busy_timeout`) are fixed by ADR-0102 because they are correctness properties, not preferences.

## 4. Platform default directories

[FACT] `directories::ProjectDirs` 6.0.0 [11] resolves (example project `com/Foo Corp/Bar App`):

| | Linux | macOS | Windows |
|---|---|---|---|
| config | `$XDG_CONFIG_HOME/<app>` or `~/.config/<app>` | `~/Library/Application Support/<id>` | `%APPDATA%\<Org>\<App>\config` (Roaming) |
| data | `$XDG_DATA_HOME/<app>` or `~/.local/share/<app>` | `~/Library/Application Support/<id>` | `%APPDATA%\...\data` (Roaming) |
| data_local | = data | = data | `%LOCALAPPDATA%\...\data` |
| cache | `$XDG_CACHE_HOME/<app>` or `~/.cache/<app>` | `~/Library/Caches/<id>` | `%LOCALAPPDATA%\...\cache` |
| state | `$XDG_STATE_HOME/<app>` or `~/.local/state/<app>` | not provided | not provided |

Decisions [REC]:

- Use `ProjectDirs::from("eu", "Natural Solutions", "NicheRS")` → Linux dir name `nichers`.
- **Data dir = `data_local_dir()`**, not `data_dir()`: on Windows the latter is the *Roaming* profile,
  which would sync multi-GB rasters and a live SQLite file across domain logins. On Linux/macOS they are
  identical [11].
- `state_dir()` is not used (absent on macOS/Windows [11]); all durable state lives in the data dir.
- Server mode (systemd) overrides with `NICHE_DATA_DIR=/var/lib/nichers`, `NICHE_CACHE_DIR=/var/cache/nichers`
  (systemd `StateDirectory=`/`CacheDirectory=`; see deployment-modes.md).
- Alternative `etcetera` 0.11.0 [12] defaults to XDG on macOS (`choose_base_strategy`) and needs
  `choose_native_strategy` for `~/Library`; the brief asks for Library on macOS, so `directories` fits
  with less code. Both are MIT OR Apache-2.0.

Directory layout — aligned with 11-backend-architecture.md §2.4 (which is authoritative for the CAS):

```
<data_dir>/
  niche.db (+ -wal, -shm)       # job/step/artifact metadata (ADR-0102)
  niche.lock                    # exclusive process lock (std File::try_lock)
  cas/<algo>/ab/cd/<hex>        # immutable content-addressed artifacts
  cas/tmp/                      # staging, same filesystem as cas/; swept at startup
  jobs/<job_id>/manifest.json   # reproducibility manifest
  backups/                      # pre-migration DB copies
<cache_dir>/                    # NICHE_CACHE_DIR; regenerable, deletable any time
  datasets/<provider>/<key>/    # downloaded layers, occurrence pulls
  tiles/<artifact_hash>/...     # derived map tiles
```

[REC] The backend doc places `cache/` inside the data dir; this doc keeps it as a **separate**
`NICHE_CACHE_DIR` (brief, factor III) defaulting to the platform cache dir, so OS cleaners and backup
tools treat it as disposable. Same relative layout either way.

## 5. Secrets

- [REC] Secrets come from `*_FILE` variables (Docker/systemd-credentials style) or, discouraged, from
  plain env vars. They are **never** accepted in the TOML config file (prevents committing them) and
  never as CLI flags (visible in `ps`/shell history). This satisfies the 12factor litmus test that the
  codebase "could be made open source at any moment" [2].
- [REC] Held in `secrecy::SecretString` (0.10.3, zeroized on drop, explicit `ExposeSecret`) [31]; our
  `Config` type implements `Debug`/`Serialize` with redaction regardless.
- [REC] systemd: `LoadCredential=auth-token:/etc/nichers/token` then
  `NICHE_AUTH_TOKEN_FILE=%d/auth-token` [UNVERIFIED – prior knowledge of systemd credentials].
- [REC] Startup check: secret file mode must not be world-readable on Unix (error in server mode,
  warning locally).

## 6. Experiment definition files

- Format: `*.niche.toml` (human-edited) or JSON via API; schema-versioned (`schema = "niche.experiment/1"`).
- Contents: species (taxon keys + names as given), study area (geometry file reference by **content hash**,
  CRS), occurrence sources and filters, predictor set (dataset ids + versions/hashes), algorithm and all
  hyper-parameters with defaults *materialised*, partitioning scheme, seeds, thresholds, outputs.
- Canonicalisation: parse → fill defaults → serialize to canonical JSON (sorted keys, no insignificant
  whitespace, normalised numbers; RFC 8785 JCS proposed elsewhere) → hash together with engine version
  = `spec_hash` (11-backend-architecture.md §2.9; hash algorithm BLAKE3 vs SHA-256 still to be unified with
  execution-manifest.md). [REC] Defaults are filled *before* hashing so that a change of a built-in
  default between versions changes the hash.
- The reproducibility manifest (`execution-manifest.md`) records the effective result-affecting
  parameters, input hashes, software build, and worker count ("because reduction order may depend on
  it"). Deployment settings such as ports, log level, paths and auth are **not** recorded. Our goal
  (tension 4) is that thread/worker count does *not* change results; recording it lets tests prove that.
- Env vars cannot override experiment fields. A CLI convenience such as `niche run exp.niche.toml
  --set algorithm.maxent.beta_multiplier=2` produces a *new* materialised definition (and hash), written
  next to the outputs.

## 7. `niche config show`

```
niche config show [--format table|toml|json] [--show-secrets=never] [--defaults]
```

- Prints every resolved setting with its value and **source** (`cli`, `env:NICHE_PORT`,
  `file:/home/u/.config/nichers/config.toml:12`, `default`).
- Secrets: always rendered as `<redacted: env:NICHE_AUTH_TOKEN_FILE, 44 bytes>`; there is no flag to
  reveal them.
- Also redacted: query strings/userinfo in URLs (`https://user:***@host`).
- `--defaults` prints the default config as commented TOML (seed for a new config file).
- `niche config validate [--config path]` validates without starting anything; exit 0 / 78.
- Exit code 78 on invalid config, with all errors listed.

## Alternatives

| Option | Verdict |
|---|---|
| `figment` 0.10.19 [8] (merge/join, provenance metadata) | Good semantics; last release 2024-05-17 at access date; adds a layer clap already partly covers. Kept as fallback (ADR-0101). |
| `config` 0.15.27 [9] (active, 2026-09-30) | Layered sources, but stringly-typed key lookup and weaker provenance; rejected. |
| Env-only config (pure 12factor) | Unfriendly for desktop users who double-click the exe; rejected — file kept as lowest non-default layer. |

## Trade-offs, risks

- Hand-written merge = ~200 lines we own; mitigated by table-driven tests over all 4 layers per setting.
- Risk: a deployment setting silently influencing numeric results (e.g. thread count changing float
  reduction order). Mitigation: determinism tests run with `NICHE_CPU_THREADS=1` and `=N` and compare
  outputs bit-for-bit or within documented tolerance [OPEN: tolerance policy belongs to the
  reproducibility research doc].

## Open questions

- [OPEN] Default port number: confirm against IANA registry before 1.0.
- [OPEN] Should `NICHE_MAX_MEMORY_MB` be enforced (allocator accounting) or only admission-control?

## References

See [twelve-factor-compliance.md § References](twelve-factor-compliance.md#references) — ids are shared across the architecture documents.
