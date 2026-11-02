# Deployment Modes: Local, Server, Batch

Status: proposal. Date: 2026-10-15. Implementation: not started — research phase.

> **Update 2026-10-16 — hosted mode implemented (ADR-0105).** `niche serve --auth accounts`
> (`NICHE_AUTH=accounts`) is the implemented server mode: organisations, invitation links printed
> by `niche admin`, password sessions, per-organisation isolation and monthly quotas. Run it behind
> TLS with `NICHE_ALLOWED_HOSTS` set. The `--mode`, `launch-token` and `proxy` options below are
> still design proposals, not implemented.

## Scope

What differs and what is shared between the three ways to run the single `niche` executable.

## 1. Entry points

| Mode | Command | Typical user |
|---|---|---|
| Local | `niche` (no args, double-click) ≡ `niche serve --mode local` | ecologist on a laptop/workstation |
| Server | `niche serve --mode server` under systemd / container | team instance on an internal server |
| Batch | `niche run <experiment.niche.toml> [--out dir]`, `niche run --batch list.txt` | scripted runs, CI, HPC-like nodes |

## 2. What is shared (must not differ)

Engine crates and algorithms; experiment schema and hashing; model bundle format and manifests;
validation rules (`datasets validate` runs the same validators as job `prepare`); SQLite schema and FS
layout; job state machine and recovery ([job-lifecycle.md](job-lifecycle.md)); config conventions and
precedence ([configuration.md](configuration.md)); log field names. Batch mode uses the same job
scheduler and store — `niche run` creates a normal job in the data dir and waits for it, so results are
inspectable later from the UI.

## 3. What differs

| Aspect | Local | Server | Batch |
|---|---|---|---|
| HTTP listener | yes, `127.0.0.1:8740` (port 0 allowed) | yes, configured host (e.g. `127.0.0.1` behind proxy, or `0.0.0.0`) | no |
| Opens browser | yes (`NICHE_OPEN_BROWSER`) | no | no |
| Auth | `launch-token` (per-launch random token in the opened URL → cookie); `none` allowed on loopback for scripting | **required**: `token` or `proxy`; `none` + non-loopback = startup error | n/a (OS user) |
| Host header check | allow `localhost`, `127.0.0.1`, `[::1]` (+port) — DNS-rebinding defence | allow host(s) of `NICHE_PUBLIC_URL` / `NICHE_ALLOWED_HOSTS` | n/a |
| Origin / CORS | same-origin only; state-changing requests with foreign `Origin` → 403 (CSRF defence for a loopback app) | same-origin by default; explicit `NICHE_ALLOWED_ORIGINS`; `CorsLayer` default sends no headers [30] | n/a |
| Limits | upload 1 GB; workers `cpus/4` | upload 200 MB; workers `cpus/2`; per-user queue quota [OPEN]; request timeout | workers 1 by default (one job at a time, full CPU pool) |
| Logging | pretty to TTY; optional `NICHE_LOG_FILE` | JSON to stderr → journald/container runtime | pretty on TTY else JSON; machine result on stdout |
| Data/cache dirs | platform defaults (configuration.md §4) | `/var/lib/nichers`, `/var/cache/nichers` | platform defaults or `--data-dir` (e.g. per-run scratch) |
| Migrations | auto with backup | explicit `niche migrate` (or `NICHE_AUTO_MIGRATE=true`) | auto with backup |
| Shutdown | Ctrl+C / window close / UI quit | SIGTERM from systemd/container; `Type=notify` | Ctrl+C: interrupt and requeue; exit code 130; re-run of same command resumes via idempotency key = experiment hash |
| Metrics | off | optional `/metrics` | off; `--report json` instead |
| Exit codes | n/a | n/a | 0 succeeded, 1 job failed, 3 cancelled/interrupted, 75 locked, 78 config |

Concurrency with a running server: Batch on the **same** data dir is refused (exit 75) because the
server holds the lock; use `niche run --server http://127.0.0.1:8740` (submits via API) or a different
`--data-dir` [REC].

Administrative boundaries (ADR-0012), same in every mode: build a pack once with
`scripts/build_boundaries.py` + `niche boundaries build`, then `niche boundaries --data-dir <dir>
install <pack>.nrsb` (copies it to `<data_dir>/boundaries/`) and restart the server. Without a
pack the "Place" method is simply not offered. On Kubernetes, copy the pack into the PVC (or
bake it into the image) — packs are never committed.

A hosted instance requires an environment-specific deployment and operations runbook covering accounts, backups, releases and rollback.

## 4. Reverse proxy notes

- TLS terminates at the proxy; niche binds to loopback; set `NICHE_PUBLIC_URL`, `NICHE_TRUSTED_PROXIES`.
- Sub-path hosting: `NICHE_BASE_PATH=/niche/`; the SPA reads base path from an injected `<base>`/config
  endpoint, not from build-time Vite config (one build for all deployments).
- SSE progress streams: disable proxy response buffering and raise read timeouts. nginx:
  `proxy_buffering off; proxy_read_timeout 1h;` (server can also send `X-Accel-Buffering: no`)
  [UNVERIFIED – prior knowledge of nginx directives]. Server sends SSE keep-alive comments (≤ 30 s).
- Upload size: proxy limit (`client_max_body_size` in nginx [UNVERIFIED – prior knowledge]) must be ≥
  `NICHE_MAX_UPLOAD_MB`.
- `proxy` auth mode: proxy (e.g. oauth2-proxy / Authentik forward-auth) sets `X-Forwarded-User`; niche
  trusts it only from `NICHE_TRUSTED_PROXIES` addresses and strips it otherwise.

Example (Caddy) [UNVERIFIED – prior knowledge of Caddyfile syntax]:

```
niche.example.internal {
    reverse_proxy 127.0.0.1:8740 {
        flush_interval -1   # stream SSE immediately
    }
}
```

## 5. systemd unit (example)

```ini
# /etc/systemd/system/nichers.service
[Unit]
Description=NicheRS species distribution modelling server
After=network-online.target
Wants=network-online.target

[Service]
Type=notify                     # niche sends READY=1 after recovery (sd-notify) [28][29]
NotifyAccess=main
ExecStart=/usr/local/bin/niche serve --mode server
Environment=NICHE_HOST=127.0.0.1
Environment=NICHE_PORT=8740
Environment=NICHE_PUBLIC_URL=https://niche.example.internal/
Environment=NICHE_AUTH_MODE=proxy
Environment=NICHE_TRUSTED_PROXIES=127.0.0.1
Environment=NICHE_DATA_DIR=/var/lib/nichers
Environment=NICHE_CACHE_DIR=/var/cache/nichers
Environment=NICHE_SHUTDOWN_GRACE_SECS=60
Environment=NICHE_LOG_FORMAT=json
StateDirectory=nichers
CacheDirectory=nichers
DynamicUser=yes
TimeoutStopSec=90               # > NICHE_SHUTDOWN_GRACE_SECS; then SIGKILL [28]
Restart=on-failure
NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
PrivateTmp=yes

[Install]
WantedBy=multi-user.target
```

[FACT] With no `ExecStop=`, systemd sends SIGTERM immediately and SIGKILL after the stop timeout [28].
Hardening directives and `StateDirectory`/`DynamicUser` semantics: [UNVERIFIED – prior knowledge of
systemd.exec(5)]; validate with `systemd-analyze security nichers` on the target host.

## 6. Container (example, optional)

No Kubernetes requirement. A single static-ish binary allows a minimal image [ASSUMPTION — depends on
the native-lib decision, tension 8].

```dockerfile
# Containerfile
FROM docker.io/library/debian:stable-slim
RUN useradd --system --uid 10001 niche && mkdir -p /data /cache && chown niche /data /cache
COPY niche /usr/local/bin/niche
USER niche
ENV NICHE_MODE=server NICHE_HOST=0.0.0.0 NICHE_PORT=8740 \
    NICHE_DATA_DIR=/data NICHE_CACHE_DIR=/cache NICHE_LOG_FORMAT=json \
    NICHE_AUTH_MODE=token NICHE_AUTH_TOKEN_FILE=/run/secrets/niche_token
VOLUME ["/data", "/cache"]
EXPOSE 8740
STOPSIGNAL SIGTERM
HEALTHCHECK CMD ["/usr/local/bin/niche", "probe", "--ready"]
ENTRYPOINT ["/usr/local/bin/niche", "serve"]
```

- `/data` must be a local volume (not NFS) — SQLite WAL requirement [24].
- Run with `--stop-timeout` > `NICHE_SHUTDOWN_GRACE_SECS` (Docker default stop timeout is 10 s
  [UNVERIFIED – prior knowledge]).
- `niche probe --ready` = tiny HTTP GET to `/readyz` so no `curl` is needed in the image [REC].
- One replica per `/data` volume; the data-dir lock prevents accidental double start.

## Alternatives / trade-offs

- Separate binaries per mode — rejected (factor I; triples release surface).
- Built-in TLS (rustls) — deferred; proxies are standard in server deployments; adds cert lifecycle.

## Risks

- Users exposing Local mode on `0.0.0.0` without auth — blocked by validation (non-loopback requires
  `--mode server`, which requires auth).
- Network shares for data dir in consultancies — doctor/startup checks.

## Open questions

- [OPEN] Multi-user authorisation model (projects, roles) for Server mode — out of scope here.
- [OPEN] Windows service packaging (e.g. WinSW/NSSM vs native service API).

## References

Ids as in [twelve-factor-compliance.md § References](twelve-factor-compliance.md#references).
