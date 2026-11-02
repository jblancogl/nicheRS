# 12 — Security and licensing

Status: research draft · Date: 2026-10-15 · Author: research agent (NicheRS)

Tags: [FACT] verified from a source read on 2026-10-15 · [ASSUMPTION] · [OPEN] · [REC] · [UNVERIFIED – prior knowledge].
Not legal advice: licence readings below must be confirmed by NS management/legal before release.

## 1. Scope

(a) Software licences of the reference MaxEnt implementation and the likely Rust/JS stack;
(b) dataset licences, attribution, redistribution and commercial use; (c) personal and
sensitive data (GDPR, protected-species locations); (d) application security of a Rust/Axum
single binary that ingests user files and rasters; (e) supply chain. Two deployment profiles
are distinguished: **P1 local desktop** (single user, loopback) and **P2 internet-exposed
server** (multi-user).

## 2. Findings

### 2.1 Software licences

| Component | Licence | Evidence | Notes |
|---|---|---|---|
| MaxEnt (mrmaxent/Maxent) | MIT, © 2016 Phillips, Dudík, Schapire | LICENSE.md [FACT [1]] | Last commit 2024-01-11; latest tag v3.4.4 [FACT [1]]. Bundled third-party: PtPlot (MIT variant), DocJava layouts (public domain), **GNU getopt Java port (LGPL-2)**, macfaq LE streams (public domain) [FACT, license-summary.txt [1]] |
| GDAL | MIT-style (with per-driver third-party notices in LICENSE.TXT) | [FACT [2]] | Optional proprietary SDK drivers (ECW, MrSID) carry other terms [UNVERIFIED – prior knowledge] — do not enable |
| PROJ | MIT-style (incl. data files) | COPYING [FACT [3]] | proj-data grids have their own licences [UNVERIFIED – prior knowledge] |
| MapLibre GL JS | BSD-3-Clause | LICENSE.txt [FACT [4]] | GitHub reports NOASSERTION because file also contains upstream notices |
| shadcn/ui, Vite, Tailwind CSS | MIT | GitHub API [FACT [5]] | shadcn code is copied into the repo; keep notice |
| Rust crates (crates.io, 2026-10-15) | axum 0.8.9 MIT; tokio 1.53.3 MIT; tower-http 0.7.1 MIT; rayon 1.12.0 MIT/Apache-2.0; gdal 0.19.0 MIT; proj 0.31.0 MIT/Apache-2.0; zip 9.0.3 MIT; flate2 1.1.10 MIT/Apache-2.0; tiff 0.11.3 MIT; geojson 1.0.0 MIT/Apache-2.0; serde_json 1.0.154 MIT/Apache-2.0; sha2 0.11.1 MIT/Apache-2.0; blake3 1.8.7 CC0/Apache-2.0; reqwest 0.13.5 MIT/Apache-2.0; rusqlite 0.40.2 MIT; duckdb 1.10506.0 MIT; arrow/parquet 60.0.0 Apache-2.0; zstd 0.14.2 BSD-3-Clause (as reported) | crates.io API [FACT [6]] | All permissive; Apache-2.0 requires NOTICE propagation |
| cargo-deny 0.20.2, cargo-audit 0.22.2, cargo-cyclonedx 0.5.9 | MIT/Apache-2.0; Apache/MIT; Apache-2.0 | crates.io [FACT [6]] | Build-time tools, not shipped |
| GeoNature (for connectors) | GPL-3.0 | GitHub API [FACT [7]] | Talking to its HTTP API does not make NicheRS a derivative [ASSUMPTION]; do not copy GeoNature code |

Conclusion: no copyleft obligation in the core stack. LGPL-2 getopt only matters if
`maxent.jar` is redistributed (keep its licence files, allow replacement) [ASSUMPTION]. A
clean-room Rust MaxEnt reimplementation has no licence constraint from MIT besides notice [FACT [1]].

### 2.2 Dataset licences and redistribution (from doc 06)

| Dataset | Licence | Commercial use | Redistribute in cache/installer | Status |
|---|---|---|---|---|
| WorldClim 2.1 | Custom: academic/non-commercial free; "Redistribution or commercial use is not allowed without prior permission" [FACT [8]] | **No** (without permission) | **No** | **BLOCKER** as default |
| ESDAC ESDB v2.0 | Registration; non-commercial; no passing to third parties [FACT [9]] | **No** | **No** | **BLOCKER** |
| CHELSA v2.1 climatologies | CC0 1.0 [FACT [10]] | Yes | Yes | OK |
| GBIF records | CC0 / CC BY 4.0 / CC BY-NC 4.0 per record; download licence = most restrictive [FACT [11]] | NC records: no (assumed for paid work) | Yes with attribution (DOI), NC subset excluded | Conditional |
| ESA WorldCover | CC BY 4.0 [FACT [12]] | Yes | Yes + "© ESA WorldCover project [year]" | OK |
| CLMS (CLC, HRL) | EU Copernicus policy: free, full, open; credit, state modifications, no endorsement [FACT [13]] | Yes [ASSUMPTION] | Yes with notice | OK |
| Copernicus DEM GLO-30/90 | Free licence with attribution notices [FACT [14]] | Yes [ASSUMPTION] | Yes with notice (archive licence text) | OK |
| OSO, IGN RGE ALTI, OpenObs | Etalab 2.0 / Licence Ouverte [FACT [15]] | Yes | Yes with attribution | OK |
| SoilGrids 2.0 | CC BY 4.0 [FACT, secondary [16]] | Yes | Yes | OK |
| Bio-ORACLE v3, EUNIS terrestrial, TAXREF | Unclear/conflicting [OPEN] | ? | ? | Clarify |

Derived products: SDM outputs trained on CC BY inputs must carry attribution (CC BY 4.0 §3)
[UNVERIFIED – prior knowledge of CC BY 4.0 legal code]. [REC] Every exported map/report embeds an
auto-generated attribution block built from the run manifest (doc 06 §8.1), including GBIF DOI(s).

### 2.3 Sensitive records and personal data

- Protected-species locations: French law (Code env. art. D411-21-3) underpins SINP blurring
  levels 0–4 (commune/5 km, 10 km, département, none) [FACT [17]]. GBIF records carry
  `dataGeneralizations`/`informationWithheld` [FACT [11]]; GBIF best-practice guide [18].
- Risk: a high-resolution suitability map + training points can re-identify precise sites of
  blurred species (training points directly; suitability peaks indirectly) [ASSUMPTION].
- GDPR: observer names in `recordedBy` (e.g. pseudonym "hugo_chevalier19" in a FR record)
  are personal data (name/identifier relating to a natural person) [FACT [11][19]]. CNIL: research
  processing must comply with GDPR/Loi Informatique et Libertés; derogations exist;
  pseudonymisation/anonymisation recommended [FACT, CNIL research pages via search [19]].
- Uploaded client data (consultancy surveys) are confidential and may contain personal data
  and sensitive locations [ASSUMPTION].

### 2.4 Application security threats (file ingestion and server)

| Threat | Mechanism | Mitigation [REC] unless marked |
|---|---|---|
| Oversized upload | Unbounded body | `tower_http::limit::RequestBodyLimitLayer` → 413, rejects early on Content-Length, otherwise stops at limit [FACT [20]]; per-route limits (CSV 200 MB, GeoJSON 50 MB, raster via path/URL not upload in P1) |
| Zip / decompression bomb | Small archive, huge or overlapping entries | Check declared `size()` vs cap *and* enforce streaming byte counter (declared sizes can lie [ASSUMPTION]); cap entry count, total uncompressed bytes, ratio (e.g. > 100×) ; no nested archives |
| Zip path traversal (zip-slip) | `../` or absolute names | Use `ZipFile::enclosed_name()` (rejects NUL, absolute, escaping paths) — docs say `name()` is dangerous for extraction [FACT [21]]; extract into fresh temp dir only |
| Malformed / deep GeoJSON | Stack exhaustion | Keep serde_json default recursion limit (the `unbounded_depth` feature + `disable_recursion_limit` removes it — never enable) [FACT [22]]; cap vertices/features; validate CRS, ring closure, coordinate ranges |
| TIFF / raster bombs | Tiny file declaring huge dimensions/bands | GDAL guidance: validate dimensions and band counts right after open (`GDALCheckDatasetDimensions`, `GDALCheckBandCount` or own checks); identify driver by content, not extension [FACT [23]]; cap width×height×bands×dtype bytes and require tiling for large rasters |
| GDAL as attack surface | VRT reading arbitrary files, `/vsicurl/` network fetch (SSRF), SRS from URL, PDF driver file embedding, VRT Python | GDAL security page: restrict `/vsicurl/` for untrusted input, `GDAL_SKIP` / build only needed drivers, `GDAL_USE_CURL=OFF` if not needed, keep VRT Python disabled, `SetFromUserInput()` accepts URLs, run under low-privilege account with CPU/memory limits [FACT [23]]. [REC] Allowlist drivers (GTiff/COG, GPKG, GeoJSON, CSV, Shapefile); reject VRT from users. |
| Path traversal on server paths | User-supplied file paths in P1 "open local file" | Canonicalise; restrict to user-chosen workspace root; never pass raw paths into GDAL from HTTP |
| Resource exhaustion | Hundreds of species × CV folds × big rasters | Job queue with global concurrency, per-job memory estimate before start, cancellation, timeouts, disk quota for cache |
| DNS rebinding (P1) | Attacker page resolves its domain to 127.0.0.1 → browser reads loopback API (SOP checks hostname, not IP) | Validate `Host` against allowlist (`localhost`, `127.0.0.1`, `[::1]` + configured names) and `Origin` on every request; this is what MCP SDKs implemented after advisories (mcp-go < 0.56.0 lacked Host validation) [FACT [24]] |
| CSRF / cross-origin | Any web page can POST to localhost | No permissive CORS; require `Origin` match + per-launch random token (cookie `SameSite=Strict` or header) even on loopback [REC]; reject `text/plain` form posts on JSON routes |
| Network exposure (P1) | Binding `0.0.0.0` exposes to LAN | Bind `127.0.0.1` by default; explicit flag + auth required for other interfaces |
| AuthN/AuthZ (P2) | Multi-user, shared caches | OIDC (e.g. company IdP) behind reverse proxy or built-in; per-project ACL; cache entries tagged by licence/sensitivity/owner |
| Secrets | GBIF/CDSE/GeoNature credentials | OS keyring in P1; env/secret store in P2; never in manifests (manifest stores account *id* only) |
| Supply chain | Malicious/vulnerable deps | `Cargo.lock` + `package-lock`/`pnpm-lock` committed; `cargo-deny` (licences, bans, advisories, sources), `cargo-audit` (RustSec), `npm audit`/`pnpm audit` in CI; SBOM via `cargo-cyclonedx` (Rust) + CycloneDX for npm; `cargo build --locked` [REC; tool versions FACT [6]] |

## 3. Security profiles

| Control | P1 local desktop | P2 internet-exposed server |
|---|---|---|
| Bind | 127.0.0.1 only (default) | Behind TLS reverse proxy; app on private interface |
| Host/Origin checks | Mandatory (DNS rebinding) | Mandatory (configured public hostname) |
| Auth | Per-launch token opened via browser URL | OIDC + sessions, CSRF tokens, rate limiting per user |
| Uploads | Limits + validation; local paths allowed within workspace | Limits + validation; no local paths; raster ingest via approved sources only |
| GDAL | Driver allowlist | Driver allowlist + worker process sandbox (seccomp/containers), no `/vsicurl/` on user input |
| Data isolation | Single user | Per-tenant storage; NC/sensitive data never shared across tenants |
| Licensing | User fetches restricted data themselves | No restricted datasets cached server-side for multiple users |
| Logging | Local log, no PII | Structured audit log; PII minimised; retention policy (GDPR) |
| Updates | Signed releases, SBOM | Same + vulnerability scanning of image |

> **Update 2026-10-16 (ADR-0105):** authentication (invitation links, Argon2id passwords,
> `HttpOnly; Secure; SameSite=Strict` session cookie, sign-in throttling) and per-organisation
> isolation of jobs are implemented in hosted mode. Still open for P2: sandboxed raster workers
> (user raster upload is not offered in v0), structured audit log, multi-replica rate limiting.

[REC] Deployment restriction: P2 is **not supported** until auth, tenant isolation and sandboxed
raster workers exist. Document "do not run with `--bind 0.0.0.0` on untrusted networks".

## 4. Evidence and references

See section 10.

## 5. Alternatives

- Licensing: wrap `maxent.jar` (needs JRE; LGPL getopt inside) vs native Rust reimplementation
  (no JVM, full control, must be validated against reference) — see modelling docs.
- Climate default: CHELSA (CC0) vs negotiating a WorldClim commercial permission.
- Soil: SoilGrids (CC BY) vs negotiating ESDAC terms.
- Raster I/O: GDAL (broad, large attack surface) vs pure-Rust `tiff` for GeoTIFF/COG only
  (smaller surface, fewer formats).
- Auth in P2: built-in OIDC vs delegated to reverse proxy (oauth2-proxy, Authentik).

## 6. Trade-offs

- Strict GDAL driver allowlist reduces formats users can import.
- Excluding CC BY-NC records reduces sample size (~4.9% of FR GBIF records [FACT, 10.5 M / 215 M, doc 06]).
- Coarsening outputs for sensitive species reduces scientific utility; must be explicit.
- Per-launch tokens complicate bookmarking/scripting against the local API.

## 7. Risks

- **BLOCKER**: WorldClim 2.1 and ESDAC used in commercial deliverables or shipped in cache/installer.
- CC BY-NC records leaking into commercial outputs if licence filter is not enforced by default.
- Exposure of sensitive-species precise locations through exported training points or maps.
- DNS rebinding against a loopback server lacking Host/Origin validation.
- GDAL/libtiff CVEs via user rasters; GDAL bundled versions lagging.
- GDPR non-compliance if observer names are exported in reports without legal basis.

## 8. Recommendations

1. [REC] Default dataset set limited to CC0/CC BY/Etalab/Copernicus sources; restricted sources
   only via explicit opt-in with licence acknowledgement; manifest field `commercial_ok`
   propagated to every output; export refuses or watermarks when `false`.
2. [REC] Licence filter on GBIF downloads: predicate excludes `CC_BY_NC_4_0` when project is commercial.
3. [REC] Sensitive data: never export raw training points for records with sensitivity ≥ 1 or
   GBIF generalisation; optionally coarsen outputs; log decision.
4. [REC] Drop `recordedBy`/`identifiedBy` at ingest unless the user opts in; keep record IDs for traceability.
5. [REC] Implement Host + Origin validation, loopback bind, per-launch token from day one.
6. [REC] Upload hardening as table 2.4; fuzz parsers (cargo-fuzz) for CSV/GeoJSON/zip/TIFF header paths.
7. [REC] CI: `cargo deny check` (licences allowlist: MIT, Apache-2.0, BSD-2/3, ISC, Zlib, CC0, Unicode, MPL-2.0 reviewed), `cargo audit`, `npm audit --omit=dev`, `cargo cyclonedx`, reproducible `--locked` builds; ship THIRD-PARTY-NOTICES generated from SBOM.

## 9. Open questions

- [OPEN] Legal reading of CC BY-NC for consultancy deliverables (NS legal).
- [OPEN] Whether NS can obtain WorldClim commercial permission (email info@worldclim.org).
- [OPEN] Exact licence text of Copernicus DEM GLO-30 (archive the PDF) and Bio-ORACLE v3.
- [OPEN] GDPR legal basis for processing observer names from SINP/GBIF in client projects.
- [OPEN] Is P2 (hosted multi-user) in scope for v1?

## 10. References

- [1] mrmaxent/Maxent GitHub: LICENSE.md, license-summary.txt, commits/tags API, https://github.com/mrmaxent/Maxent. Accessed 2026-10-15.
- [2] OSGeo/gdal LICENSE.TXT, https://github.com/OSGeo/gdal/blob/master/LICENSE.TXT. Accessed 2026-10-15.
- [3] OSGeo/PROJ COPYING, https://github.com/OSGeo/PROJ/blob/master/COPYING. Accessed 2026-10-15.
- [4] maplibre/maplibre-gl-js LICENSE.txt, https://github.com/maplibre/maplibre-gl-js/blob/main/LICENSE.txt. Accessed 2026-10-15.
- [5] GitHub API licence fields: shadcn-ui/ui, vitejs/vite, tailwindlabs/tailwindcss. Accessed 2026-10-15.
- [6] crates.io API `/api/v1/crates/<name>` for listed crates (version, licence, downloads). Accessed 2026-10-15.
- [7] GitHub PnX-SI/GeoNature (GPL-3.0). Accessed 2026-10-15.
- [8] WorldClim About / terms, https://www.worldclim.org/about.html. Accessed 2026-10-15.
- [9] ESDAC ESDB v2.0 access page, https://esdac.jrc.ec.europa.eu/content/european-soil-database-v20-vector-and-attribute-data. Accessed 2026-10-15.
- [10] CHELSA climatologies v2.1, https://www.chelsa-climate.org/datasets/chelsa_climatologies. Accessed 2026-10-15.
- [11] GBIF API live probes (licence facet, download licence/DOI, record fields), https://api.gbif.org/v1/. Accessed 2026-10-15.
- [12] ESA WorldCover data access, https://esa-worldcover.org/en/data-access. Accessed 2026-10-15.
- [13] CLMS data policy, https://land.copernicus.eu/en/data-policy. Accessed 2026-10-15.
- [14] CDSE Copernicus DEM collection description, https://dataspace.copernicus.eu/explore-data/data-collections/copernicus-contributing-missions/collections-description/COP-DEM. Accessed 2026-10-15.
- [15] GEODES OSO pages (Etalab 2.0) https://geodes.cnes.fr/?p=12386 ; IGN 2024 open-data document https://www.ign.fr/files/default/2024-05/DP-IGN-operateur-data.pdf ; OpenObs home https://openobs.mnhn.fr/. Accessed 2026-10-15.
- [16] Poggio et al. 2021, SOIL 7:217–240, doi:10.5194/soil-7-217-2021 ; LifeWatch metadata record. Accessed 2026-10-15.
- [17] DREAL Nouvelle-Aquitaine SINP sensitivity decision 2019-44 (see doc 06 [10]). Accessed 2026-10-15.
- [18] Chapman, Current Best Practices for Generalizing Sensitive Species Occurrence Data, GBIF 2023. Accessed 2026-10-15.
- [19] CNIL, Recherche scientifique (hors santé) and Q&A, https://www.cnil.fr/fr/recherche-scientifique-hors-sante. Accessed 2026-10-15.
- [20] tower-http `limit` module docs, https://docs.rs/tower-http/latest/tower_http/limit/index.html. Accessed 2026-10-15.
- [21] zip crate `ZipFile` docs (`enclosed_name`, `mangled_name`), https://docs.rs/zip/latest/zip/read/struct.ZipFile.html. Accessed 2026-10-15.
- [22] serde_json `Deserializer::disable_recursion_limit` (feature `unbounded_depth`), https://docs.rs/serde_json/latest/serde_json/struct.Deserializer.html. Accessed 2026-10-15.
- [23] GDAL Security considerations, https://gdal.org/en/stable/user/security.html. Accessed 2026-10-15.
- [24] MCP TypeScript SDK hostHeaderValidation middleware https://ts.sdk.modelcontextprotocol.io/v2/api/@modelcontextprotocol/express/middleware/hostHeaderValidation.html ; VulnCheck advisory mcp-go < 0.56.0 https://www.vulncheck.com/advisories/mcp-go-before-0.56.0-missing-host-header-validation-enables-dns-rebinding. Accessed 2026-10-15.
