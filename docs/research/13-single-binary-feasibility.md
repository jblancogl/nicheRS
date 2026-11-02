# 13 — Single-binary feasibility for NicheRS

Status: research, 2026-10-15. Tags: [FACT] verified from a source read on 2026-10-15; [ASSUMPTION];
[OPEN]; [REC]; [UNVERIFIED – prior knowledge]. Related: 05 (geospatial deps), 10 (frontend
embedding), 11 (backend), 12 (security/licensing), `_BRIEF-12FACTOR.md`.

## 1. Scope

Can NicheRS ship as one Rust executable per platform that serves API + embedded React UI, runs
offline, and has no runtime dependency on Python/Java/R/Node/GDAL? Covers: definition of "single
binary", native dependencies, cross-platform build matrix and tooling, code signing/notarisation,
reproducible builds, update mechanisms.

## 2. Findings

### 2.1 Definition of "single binary" for NicheRS [REC]

A release artefact is **one executable file per (OS, arch)** such that:

1. It runs on a clean supported OS install with no installer, no admin rights, no runtime
   (no Python/Java/R/Node/JVM/.NET), no GDAL/PROJ/SQLite shared libraries.
   - Allowed OS-provided dependencies: Linux — none (static musl) or glibc ≥ a documented floor if a
     gnu build is shipped; Windows — system DLLs (kernel32, ws2_32, bcrypt…) with the MSVC CRT linked
     statically (`+crt-static`); macOS — libSystem and system frameworks (Apple does not support
     fully static binaries [UNVERIFIED – prior knowledge]).
2. Embedded inside: frontend assets (precompressed), SQL migrations, default config, CRS
   definitions, colour ramps, licence texts/SBOM, model-format schemas. PROJ's proj.db only if the
   optional GDAL/PROJ feature is enabled (embedded by PROJ static builds, see 05 §2.4).
3. Writable state lives **outside** the binary in `NICHE_DATA_DIR` (SQLite job/project DB, model
   artefacts, run manifests) and `NICHE_CACHE_DIR` (downloaded rasters, tile cache) — platform
   defaults via XDG/`%LOCALAPPDATA%`/`~/Library/Application Support` [REC]. Large datasets
   (WorldClim, CHELSA, CORINE) are **never** embedded; they are downloaded or imported and
   checksummed.
4. Same binary supports the three deployment modes (local UI, server, batch CLI via subcommands
   `serve`, `run`, `doctor`, `migrate`, `cache …`) — twelve-factor XII.
5. Not required: a GUI window (browser opened on `http://127.0.0.1:<port>`), auto-update,
   installer. An installer/pkg/dmg wrapper may exist **only** for signing/notarisation reasons
   (§2.5) and must contain the same executable.

Tension 2 (single executable vs process isolation): the binary may re-exec itself as a worker
(`niche worker --job <id>`) for isolation of long training jobs without breaking the definition
[REC]; this keeps one file on disk.

### 2.2 Native dependencies — what breaks "single binary"

| Dependency | Needed? | Static option | Verdict |
|---|---|---|---|
| SQLite | yes (jobs, projects, GPKG) | `rusqlite` 0.40.2 `bundled` (compiles SQLite C, needs C compiler) [FACT, 05-1] | Fine everywhere incl. musl/Windows |
| TLS (data downloads: GBIF, WorldClim) | yes | `rustls` (pure Rust + `ring`/`aws-lc-rs` with C/asm) [UNVERIFIED – prior knowledge] | Avoid OpenSSL; use rustls |
| GDAL + PROJ | optional (05 recommendation D) | `gdal-sys/bundled` + `proj-sys/bundled_proj`: static GDAL 3.12.1, PROJ 9.6.2, C++ runtime via `link-cplusplus` [05-3][05-6] | Possible; heavy; spike required |
| GEOS | no | `geos_static` makes binary LGPL-bound per gdal-src comment [05-3] | Avoid |
| BLAS/LAPACK | maybe (MaxEnt/GLMs — doc 04/08) | pure-Rust (`faer`) preferred [ASSUMPTION; see doc 04] | Avoid system BLAS |
| Memory allocator | performance | musl's malloc is slow under multithreading [UNVERIFIED – prior knowledge]; `mimalloc` 0.1.52 (C, MIT) builds statically | [REC] mimalloc on musl targets; benchmark |

### 2.3 Target matrix

Rust platform support (nightly docs, 2026-10-15) [13-1] [FACT]:

| Target | Rust tier | Role for NicheRS |
|---|---|---|
| `x86_64-unknown-linux-musl` | Tier 2 with host tools | Primary Linux artefact (fully static) |
| `aarch64-unknown-linux-musl` | Tier 2 with host tools | Linux ARM servers, Raspberry-class |
| `x86_64-unknown-linux-gnu` / `aarch64-unknown-linux-gnu` | Tier 1 | Optional; needs glibc floor |
| `x86_64-pc-windows-msvc` | Tier 1 | Primary Windows (consultants' laptops) |
| `aarch64-pc-windows-msvc` | Tier 1 | Windows on ARM laptops |
| `aarch64-apple-darwin` | Tier 1 | Primary macOS |
| `x86_64-apple-darwin` | Tier 2 with host tools | Intel Macs; could ship universal2 |

### 2.4 Cross-compilation tooling

| Tool | Version (date) | Licence | What it does | Limits |
|---|---|---|---|---|
| `cargo-zigbuild` | 0.23.4 (2026-09-02) | MIT | Zig as linker/C compiler; **"Currently only Linux and macOS targets are supported"**; glibc version suffix (e.g. `aarch64-unknown-linux-gnu.2.17`); static via musl (glibc `crt-static` unsupported); `universal2-apple-darwin`; macOS framework linking needs `SDKROOT` outside macOS; `-nostdinc` hides system headers [13-2] [FACT] | No Windows; C++ deps (GDAL) untested by README |
| `cross` | 0.2.5 (2023-02-04) on crates.io; repo active (pushed 2026-10-14) [05-1][13-3] [FACT] | MIT/Apache-2.0 | Docker images with per-target toolchains [UNVERIFIED – prior knowledge] | Needs Docker/Podman; no macOS/MSVC images by default [UNVERIFIED – prior knowledge] |
| `cargo-xwin` | 0.23.1 (2026-08-13) | MIT | Cross-compile to `*-pc-windows-msvc` from Linux with MS CRT/SDK download [UNVERIFIED – prior knowledge of mechanism] | Licence acceptance of MS SDK; C++ (GDAL) via clang-cl uncertain [OPEN] |
| Native CI runners | — | — | Build each target on its own OS (Linux, Windows, macOS ARM) | Simplest for C/C++ deps; needs macOS runner |

If a Windows CI runner is available, native MSVC builds are the low-risk path for Windows
[ASSUMPTION].

Assessment: **pure-Rust + bundled SQLite** cross-compiles with zigbuild (Linux musl + macOS) and
xwin or a native runner (Windows) [ASSUMPTION – standard pattern; to confirm in CI spike]. **Static
GDAL** adds CMake + C++ cross-compilation per target; native runners per OS are the realistic path
[ASSUMPTION].

### 2.5 Code signing

| Platform | Mechanism | Verified facts | Cost/effort |
|---|---|---|---|
| Windows | Authenticode | Azure **Artifact Signing** (formerly Trusted Signing): ~$9.99/month, available to **organisations in the EU** (individuals only US/CA), no hardware token, CI integration; SmartScreen reputation still builds over time. OV certificate $150–300/yr, private key on HSM/token since June 2023. **EV no longer bypasses SmartScreen (since 2024)** [13-4] [FACT] | Natural Solutions (EU organisation) qualifies for Artifact Signing [inference] |
| macOS | Developer ID signing + notarisation | Bare executables **cannot be stapled**; Gatekeeper fetches the ticket online at first run; Apple DTS advises wrapping in a signed+notarised+stapled `.pkg` or `.dmg` [13-5] [FACT, Apple Developer Forums DTS answers]. `rcodesign` (`apple-codesign` 0.29.0, 2024-11-29, MPL-2.0) signs and submits to the Notary API from Linux [13-6] [FACT for capability as described by its author; version from crates.io] | Apple Developer Program membership (annual fee) [UNVERIFIED – prior knowledge] |
| Linux | detached signatures | minisign/cosign/zipsign of archives; checksums [REC] | Free |

Consequence for the definition: on macOS the distributable is a notarised `.pkg`/`.dmg`/zip
containing the single executable; on Windows a signed `.exe` (optionally in a signed MSI) [REC].
Downloads via `curl` in a terminal do not get the quarantine attribute and skip Gatekeeper
[UNVERIFIED – prior knowledge].

### 2.6 Reproducible builds

| Concern | Measure [REC] |
|---|---|
| Dependency pinning | `Cargo.lock` + `--locked`; pnpm lockfile + `--frozen-lockfile`; pinned `rust-toolchain.toml`; pinned Node for build only |
| Embedded paths | `--remap-path-prefix` (or Cargo `trim-paths` [UNVERIFIED – stabilisation status]) |
| Timestamps | `SOURCE_DATE_EPOCH`; `rust-embed` `deterministic-timestamps` feature zeroes embedded file mtimes [13-7] [FACT] |
| Frontend | Vite content-hashed filenames are deterministic given same inputs [ASSUMPTION — verify by double build] |
| C code (SQLite, mimalloc, GDAL) | same compiler version in container; reproducibility harder with C++ GDAL [ASSUMPTION] |
| Provenance | `cargo-auditable` 0.7.7 (2026-10-02) embeds dependency list in the binary; `cargo-cyclonedx` 0.5.9 for SBOM [05-1 data] — supports twelve-factor II "SBOM" |
| Verification | CI builds twice in clean containers and diffs hashes (`diffoscope` on failure) |

Scientific reproducibility (doc 07) is separate: run manifests record binary version + git hash +
build hash; that requires deterministic version stamping, not bit-reproducible binaries, but both
are recommended.

### 2.7 Update mechanisms

| Option | Version (date) | Notes |
|---|---|---|
| `self_update` | 1.3.0 (2026-09-02), MIT, 12.5 M dl | Backends GitHub, **GitLab (self-hosted)**, Gitea, Gitee, S3, Manifest (any static server), custom `ReleaseSource`; tar/zip; `signatures` feature verifies **zipsign** signatures; Windows swap via `self_replace` [13-8] [FACT] |
| `axoupdater` (cargo-dist) | 0.10.2 (2026-08-12) / cargo-dist 0.32.0 (2026-05-22) | Tied to cargo-dist release layout [UNVERIFIED – prior knowledge] |
| Manual | — | "New version available" banner + download link; users replace file |

[REC] No silent auto-update for a scientific instrument: an update changes results. Ship
`niche self-update` (explicit, signature-verified via zipsign, from a Natural Solutions GitLab
release or static manifest), check-for-update opt-in, and keep old binaries runnable side by side.
Notarised macOS updates must replace the binary inside an already-trusted location; quarantine is
not applied to files written by the process itself [UNVERIFIED – prior knowledge].

### 2.8 Binary size budget

No verified figures available [OPEN]. Components: Rust server + engine (Axum, Tokio, Rayon,
rusqlite bundled) + frontend (React + MapLibre GL JS v6 + shadcn) precompressed. MapLibre is the
largest frontend dependency [ASSUMPTION]. Static GDAL would dominate if enabled [ASSUMPTION]. The
spike must report sizes per target with `opt-level=3`, `lto="fat"`, `codegen-units=1`,
`strip=true`.

## 3. Evidence and references

See References. Crate data from crates.io API (2026-10-15).

## 4. Alternatives

1. **Single static binary (pure-Rust geo)** — recommended baseline.
2. Single binary + optional GDAL-enabled variant (`niche-full`), separate artefact.
3. Binary + sidecar files (zip with `proj.db`, GDAL plugins, assets) — simpler builds, weaker UX.
4. Desktop shell (Tauri) — adds WebView dependency and per-OS packaging; not needed since the UI is
   a local web app [ASSUMPTION; desktop shell not required by brief].
5. Containers (Docker/OCI) for server mode only — good complement for server deployments, not for
   consultants' laptops.

## 5. Trade-offs

| | 1. Static pure-Rust | 2. + GDAL variant | 3. Binary + sidecars | 5. Container |
|---|---|---|---|---|
| UX (download & run) | ++ | ++ / + | − | −− (laptops) |
| Build complexity | + | − | + | + |
| Format coverage | − | ++ | ++ | ++ |
| Signing burden | one file per OS | two artefacts | many files | n/a |
| Reproducibility | ++ | + | − | + |

## 6. Risks

- Static GDAL fails to cross-compile for some target → mitigated by keeping GDAL optional (05).
- macOS notarisation of a bare CLI: first launch requires network; offline users see a Gatekeeper
  block unless distributed in a stapled pkg/dmg [13-5].
- SmartScreen warnings on early Windows releases even when signed [13-4] — communicate to users.
- musl performance regressions (allocator) — benchmark (doc 08).
- Antivirus false positives for unsigned self-updating binaries [UNVERIFIED – prior knowledge].
- `cross` crates.io release is 2023; using it from git is a supply-chain consideration.

## 7. Recommendations

- [REC] Adopt definition §2.1; data and cache directories outside the binary are explicitly allowed.
- [REC] Default build is pure Rust + bundled SQLite + rustls + mimalloc (musl), no GDAL/PROJ.
- [REC] Release matrix: `x86_64/aarch64-unknown-linux-musl` (cargo-zigbuild 0.23.4),
  `aarch64/x86_64-apple-darwin` or universal2 (native macOS runner, or zigbuild with SDK),
  `x86_64/aarch64-pc-windows-msvc` (native Windows runner; cargo-xwin 0.23.1 as fallback).
- [REC] Signing: Azure Artifact Signing for Windows (Natural Solutions as EU organisation); Apple
  Developer ID + notarised, stapled `.pkg` (or `.dmg`) containing the binary; minisign/zipsign +
  SHA-256 for Linux; publish SBOM (cargo-cyclonedx) and embed dependency list (cargo-auditable).
- [REC] Updates: explicit `niche self-update` using `self_update` 1.3.0 with `signatures`
  (zipsign) and GitLab or Manifest backend; never automatic.
- [REC] Spike (1–2 days): build hello-world Axum + embedded UI + rusqlite bundled for all targets;
  second spike adds `gdal` bundled; measure size, build time, startup time.

## 8. Open questions

- [OPEN] Static GDAL+PROJ cross-build feasibility and size per target.
- [OPEN] Does Natural Solutions already hold an Apple Developer account / code-signing identity?
- [OPEN] Minimum supported OS versions (Windows 10?, macOS 13?) — affects targets and WebView-free
  browser assumptions (MapLibre v6 requires WebGL2, doc 10).
- [OPEN] Distribution channel for updates (self-hosted GitLab releases vs static site).

## References

[13-1] Rust Platform Support (rustc book, nightly). https://doc.rust-lang.org/nightly/rustc/platform-support.html (accessed 2026-10-15)
[13-2] rust-cross/cargo-zigbuild README, v0.23.4. https://github.com/rust-cross/cargo-zigbuild (accessed 2026-10-15)
[13-3] cross-rs/cross repository (GitHub API: not archived, pushed 2026-10-14); crates.io cross 0.2.5 (2023-02-04). https://github.com/cross-rs/cross (accessed 2026-10-15)
[13-4] Microsoft Learn, "Code signing options for Windows app developers" (updated 2026-08-29). https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/code-signing-options (accessed 2026-10-15)
[13-5] Apple Developer Forums, DTS answers on stapling and command-line tools: https://developer.apple.com/forums/thread/116812 ; https://developer.apple.com/forums/thread/124336 (accessed 2026-10-15, via search summary)
[13-6] G. Szorc, "Achieving a completely open source implementation of Apple code signing and notarization" (2022-08-08); apple-codesign 0.29.0 on crates.io. https://gregoryszorc.com/blog/2022/08/08/achieving-a-completely-open-source-implementation-of-apple-code-signing-and-notarization/ (accessed 2026-10-15, via search summary)
[13-7] rust-embed 8.13.0 README and Cargo.toml (features debug-embed, compression, deterministic-timestamps, include-exclude). https://crates.io/crates/rust-embed/8.13.0 (accessed 2026-10-15)
[13-8] jaemk/self_update README, v1.3.0. https://github.com/jaemk/self_update (accessed 2026-10-15)
[05-1] crates.io API v1 metadata (shared with doc 05). https://crates.io/api/v1/crates/<name> (accessed 2026-10-15)
[05-3] georust/gdal gdal-src build files. https://github.com/georust/gdal/tree/master/gdal-src (accessed 2026-10-15)
[05-6] georust/proj proj-sys README/build.rs. https://github.com/georust/proj/tree/main/proj-sys (accessed 2026-10-15)
