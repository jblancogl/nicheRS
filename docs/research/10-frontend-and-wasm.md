# 10 — Frontend architecture, map rendering, embedding and WASM

Status: research, 2026-10-15. Tags: [FACT] verified from a source read on 2026-10-15; [ASSUMPTION];
[OPEN]; [REC]; [UNVERIFIED – prior knowledge]. Package versions from the npm registry and
crates.io APIs on 2026-10-15 [10-1][05-1]. Related: 05 (geospatial), 09 (UX), 11 (backend), 13
(single binary).

## 1. Scope

- Mandated stack, verified for current versions and compatibility: React + TypeScript, Vite,
  Tailwind, shadcn/ui, Lucide, TanStack Query, Zustand (where justified), MapLibre, React Hook Form
  + Zod. React Router evaluated.
- Map library choice (MapLibre GL JS vs OpenLayers vs deck.gl) and raster rendering strategy for
  habitat-suitability maps (continuous 0–1, binary, uncertainty, categorical inputs).
- Client vs server responsibilities, web workers, progressive loading.
- Embedding the built frontend in the Rust executable: crates, precompression, SPA fallback, cache
  headers, CSP, dev proxy, dev override, version consistency.
- WASM: tooling status, browser constraints, and comparison with TS and server-side Rust per use
  case.

## 2. Findings

### 2.1 Mandated stack — current versions (npm, 2026-10-15)

| Package | Latest | Published | Licence | Notes |
|---|---|---|---|---|
| react | 19.3.1 | 2026-10-14 | MIT | |
| vite | 8.3.4 | 2026-10-08 | MIT | Vite 8 (2026-03-12) uses **Rolldown** as single Rust-based bundler; Node 20.19+/22.12+ (build only) [10-2] [FACT] |
| @vitejs/plugin-react | 6.1.2 | 2026-10-05 | MIT | v6 uses Oxc for React Refresh, no Babel [10-2] [FACT] |
| typescript | 7.0.2 | 2026-07-08 | Apache-2.0 | [UNVERIFIED – prior knowledge] TS 7 is the Go-native compiler; check tooling compatibility (typescript-eslint) [OPEN] |
| tailwindcss, @tailwindcss/vite | 4.3.3 | 2026-07-16 | MIT | |
| shadcn (CLI) | 4.21.5 | 2026-10-14 | MIT | see §2.2 |
| lucide-react | 1.57.0 | 2026-10-14 | ISC | |
| @tanstack/react-query | 5.104.3 | 2026-10-15 | MIT | |
| zustand | 5.0.15 | 2026-08-13 | MIT | |
| react-router | 8.4.0 | 2026-09-15 | MIT | |
| react-hook-form | 7.90.0 | 2026-10-11 | MIT | |
| zod | 4.6.7 | 2026-10-12 | MIT | |
| @hookform/resolvers | 5.9.1 | 2026-08-17 | MIT | |
| maplibre-gl | 6.14.0 | 2026-10-11 | BSD-3-Clause | v6: ESM only, WebGL2 required, CSP build removed (§2.7) [10-3] [FACT] |
| ol (OpenLayers) | 10.11.0 | 2026-10-04 | BSD-2-Clause | |
| deck.gl | 9.4.0 | 2026-09-05 | MIT | |
| geotiff (geotiff.js) | 3.0.5 | 2026-03-11 | MIT | |
| @geomatico/maplibre-cog-protocol | 0.10.0 | 2026-09-18 | MIT | |
| @developmentseed/deck.gl-geotiff | 0.8.1 | 2026-09-23 | MIT | |
| pmtiles | 4.5.0 | 2026-08-10 | BSD-3-Clause | |

### 2.2 shadcn/ui status (verified on ui.shadcn.com)

- Vite install: `pnpm create vite@latest` (React + TS) → `pnpm add tailwindcss @tailwindcss/vite`,
  `src/index.css` = `@import "tailwindcss";`, `baseUrl`/`paths` `@/*` in `tsconfig.json` **and**
  `tsconfig.app.json`, `@types/node`, `tailwindcss()` plugin + `@` alias in `vite.config.ts`, then
  `pnpm dlx shadcn@latest init` and `pnpm dlx shadcn@latest add button`; or scaffold with
  `pnpm dlx shadcn@latest init -t vite` [10-4] [FACT].
- Tailwind v4 + React 19 are the defaults for new projects; v3/React 18 apps still work;
  `tailwindcss-animate` → `tw-animate-css`; `toast` → `sonner`; `default` style deprecated →
  `new-york`; colours HSL → **OKLCH** with `@theme inline`; `data-slot` on every primitive [10-5]
  [FACT].
- **July 2026: Base UI is the default primitive library for new projects; Radix not deprecated,
  selectable with `pnpm dlx shadcn init -b radix`; new components ship for both** [10-6] [FACT].
  React Aria is also listed as a supported base (Jul/Aug 2026 changelog entries) [10-7] [FACT –
  titles only].
- Forms: shadcn documents React Hook Form + Zod using `<Field/>`, `<FieldLabel>`,
  `<FieldDescription>`, `<FieldError/>` with RHF `<Controller/>` and `zodResolver` from
  `@hookform/resolvers/zod`; the page's example uses "zod v3" and mentions any Standard Schema
  library [10-8] [FACT]. [OPEN] Confirm `@hookform/resolvers` 5.9.1 + `zod` 4.x interplay in a spike
  (prior knowledge says resolvers ≥5 support Zod 4 [UNVERIFIED]).
- Sep 2026: `cn` moved to a standalone package with a `migrate cn` command [10-7] [FACT].

[REC] Start with Base UI (default) unless a needed component exists only for Radix; pin the shadcn
CLI version in `package.json` scripts since components are copied source, not a dependency.

### 2.3 Libraries — role and justification

| Concern | Choice | Justification [REC] |
|---|---|---|
| Server state (projects, runs, jobs, metrics) | TanStack Query 5 | Caching, polling of job progress (or SSE invalidation), retries; avoids hand-rolled stores |
| Client UI state | Zustand 5 **only** for cross-component ephemeral state: map viewport, selected layers, active threshold, comparison selection | Do not mirror server state in Zustand |
| URL state | React Router 8 (library/"data" mode, SPA) | Deep links to project/run/map view; SSR/"framework mode" not needed for an embedded SPA [ASSUMPTION about mode naming in v8] |
| Forms | RHF 7 + Zod 4 | Zod schemas can be generated from Rust types (e.g. via JSON Schema/`ts-rs`/`specta`) to keep API validation consistent [ASSUMPTION; tooling choice in doc 11] |
| Icons | lucide-react | mandated |
| Charts (response curves, ROC) | out of scope here; see doc 09 | |

### 2.4 Map library comparison

| Criterion | MapLibre GL JS 6.14 | OpenLayers 10.11 | deck.gl 9.4 |
|---|---|---|---|
| Rendering | WebGL2 only (v6) [10-3] | Canvas + WebGL tile layers | WebGL2/WebGPU layers |
| Projections | Web Mercator (+ globe) [UNVERIFIED – prior knowledge] | **Arbitrary projections** (proj4js), native GeoTIFF source via geotiff.js, reads projection from GeoTIFF metadata, nodata, `normalize:false` for raw values [10-9] [FACT] | Web Mercator / globe views; raster reprojection in `deck.gl-raster` claimed [10-10] partial |
| Vector basemaps / styles | Best-in-class (style spec, MVT, PMTiles protocol) | Good, less idiomatic for MVT styling | Via MapLibre basemap interleaving [UNVERIFIED] |
| Raster colour ramp in GPU | `color-relief` layer: client-side colouring of DEM-encoded rasters (Terrain-RGB, Terrarium, **custom encodings**) with `color-relief-color` [10-11] [FACT] | WebGL tile layer style expressions [UNVERIFIED – prior knowledge] | Custom shaders / `deck.gl-raster` |
| React integration | `react-map-gl` or thin own wrapper | own wrapper | first-class React |
| Ecosystem fit with mandated stack | mandated | — | complementary |

[REC] MapLibre GL JS (mandated) as the only map engine. OpenLayers is the stronger tool for native
non-Mercator display but conflicts with the mandate; deck.gl adds weight without a demonstrated
need.

### 2.5 Raster rendering strategies

| Strategy | How | Pros | Cons | Fit |
|---|---|---|---|---|
| **S1 Server XYZ PNG/WebP tiles, pre-coloured** | Axum endpoint `/tiles/{run}/{layer}/{z}/{x}/{y}.png?ramp=…&min=…&max=…`: warp tile footprint from model-grid COG (3035/2154) to 3857, apply ramp, encode; disk cache | Works with any model CRS; simple client; exact scientific values never leave the server | Ramp/threshold change → refetch tiles; server CPU | Good baseline |
| **S2 Server value-encoded tiles + GPU ramp** | Same endpoint returns **lossless PNG with values packed into RGB** (custom encoding) consumed by `raster-dem` + `color-relief` layer; client changes ramp/threshold via `color-relief-color` expression without refetch | Instant threshold/ramp changes, server stays authoritative for warping | Abuses DEM layer semantics; precision limited by encoding (24 bits fine for 0–1); hillshade-oriented features may interfere [ASSUMPTION – needs spike] | **Preferred if spike passes** |
| S3 Client-side COG (`maplibre-cog-protocol`) | `addProtocol('cog', …)`, range requests, geotiff.js decoding in browser; `#color:<scheme>,min,max,c` ramps; `locationValues()` for click values [10-12] [FACT] | No tile server; works offline from local server | **COG must be EPSG:3857; other projections throw** [10-12] [FACT] → requires writing an extra 3857 display copy; colour changes only affect newly rendered tiles; `setColorFunction` is global | Possible for display copies only |
| S4 Client-side COG in deck.gl (`deck.gl-raster`) | GPU decode, "fully client-side" [10-10] [FACT] | Fast, flexible | Second map engine; maturity [OPEN] | Not now |
| S5 Static pre-rendered pyramids (PMTiles raster) | Export pyramid per run | Great for sharing/offline reports | Fixed ramp | Export feature |

Click-inspection [REC]: values must come from the **native model grid**, not from rendered/reprojected
tiles: `GET /api/runs/{id}/sample?lon=&lat=` → server transforms 4326→model CRS, reads the cell,
returns suitability, binary class at the active threshold, uncertainty, and env. covariate values
with units. This avoids resampling artefacts (05 §2.6) and keeps a single source of truth.

### 2.6 Client vs server responsibilities [REC]

| Task | Where | Why |
|---|---|---|
| Reprojection, resampling, area, statistics | server (Rust) | correctness, one implementation, testable vs GDAL |
| Tile colouring | server (S1) or GPU (S2) | |
| Threshold slider → area/percentage | **client TS** from a server-provided histogram (e.g. 1 000 bins of suitability × per-bin area in km²) | O(bins) per update, no round trip, exact to bin resolution; exact value recomputed server-side on commit |
| Occurrence display (thousands of points) | client GeoJSON source (MapLibre clustering) | |
| Large vector overlays | PMTiles/MVT from server | |
| Form validation | client Zod + server validation | never trust client |

Web workers: MapLibre already decodes tiles in its own worker [UNVERIFIED – prior knowledge].
App-level workers only for CSV/GeoJSON parsing of user uploads before sending (optional).
Progressive loading: route-level code splitting (`React.lazy`) so the map bundle loads only on
map views; TanStack Query `placeholderData` for job lists; SSE/WebSocket job progress (doc 11).

### 2.7 Embedding the frontend in the Rust binary

| Crate | Version (date) | Licence | Mechanism | Precompression | ETag/cache | SPA fallback | Dev mode |
|---|---|---|---|---|---|---|---|
| `memory-serve` | 2.4.0 (2026-09-17) | Apache-2.0/MIT | `load_directory` in `build.rs` + `load!` macro → axum `Router` | **brotli at compile time in release**; serves plain/gzip/brotli | ETag + conditional requests; cache-control presets Long/Medium/Short/NoCache/Custom (Short default for HTML) | index + configurable fallback with status code | debug builds read & compress from disk per request; `force-embed` feature [10-13] [FACT] |
| `rust-embed` | 8.13.0 (2026-10-07) | MIT | derive macro | `compression` feature = include-flate (deflate-compressed in binary, decompressed at runtime) — **not** content-encoding passthrough [10-14] [FACT] | `Metadata::sha256_hash()`, `last_modified()` [10-14] [FACT] → build own ETag | own handler (axum-spa example exists) | debug reads from FS unless `debug-embed`; `deterministic-timestamps` [10-14] [FACT] |
| `include_dir` | 0.7.4 (2024-06-17) | MIT | macro | none | none | own | none | 
| `static-serve` | 0.6.4 (2026-09-06) | MIT/Apache-2.0 | macro | [UNVERIFIED] | [UNVERIFIED] | [UNVERIFIED] | [UNVERIFIED] |
| `tower-http` `ServeDir` | 0.7.1 (2026-08-31) | MIT | serves from disk | serves precompressed `.br/.gz` siblings [UNVERIFIED – prior knowledge] | | `fallback` | ideal for **dev override** |

[REC] `memory-serve` 2.4.0 for embedded assets (it implements exactly the needed precompression,
ETag, cache-control and SPA fallback); fall back to `rust-embed` + own ~150-line handler if its
build-script model conflicts with the workspace. Cache policy: `/assets/*` (Vite content-hashed
filenames) → `Cache-Control: public, max-age=31536000, immutable`; `index.html` → `no-cache`;
API → `no-store` unless explicitly cacheable; tiles → `max-age` keyed by run id + ramp params.

Dev override [REC]: `NICHE_UI_DIR=/path/to/dist` (or `--ui-dir`) makes the binary serve the UI from
disk via `ServeDir` instead of embedded assets (also useful for hot-fixing UI in the field); logged at
startup as a warning; disabled in `server` mode unless explicitly allowed.

Dev loop [REC]: Vite dev server with `server.proxy` `/api` → `http://127.0.0.1:<axum-port>`
(`changeOrigin: true`, `ws: true` for progress sockets) [10-15] [FACT for config syntax; note Vite
does not check WebSocket origin when proxying]. Production never needs Node (twelve-factor V/X).

Version consistency [REC]: build script computes `NICHE_BUILD_ID` (git describe + dirty flag +
frontend dist hash); injected into Vite via `define: { __BUILD_ID__: … }` and into Rust via
`env!`; API returns `X-NicheRS-Build`; UI compares on each query and prompts reload on mismatch
(matters with the dev override). Build ID also recorded in run manifests (doc 07).

CSP [REC], served by Axum on `index.html`:
```
default-src 'self'; script-src 'self'; worker-src 'self' blob:; img-src 'self' data: blob:;
style-src 'self' 'unsafe-inline'; connect-src 'self' <basemap/tile hosts>; font-src 'self';
object-src 'none'; base-uri 'none'; frame-ancestors 'none'
```
MapLibre v6 notes [10-3] [FACT]: the dedicated CSP bundle is gone; with a **self-hosted** worker the
worker URL is same-origin and `blob:` is not required; bundlers (Vite) need a one-time
`setWorkerUrl()` call. So `worker-src 'self'` should suffice once `setWorkerUrl` is wired; keep
`blob:` only if the spike shows it is still used. `'unsafe-inline'` for styles is a conservative
default for Base UI/Radix positioning [UNVERIFIED – whether needed; test and remove].

### 2.8 WASM feasibility

Tooling status (verified):
- The rustwasm GitHub org was sunset (announcement 2025-07-21); wasm-bindgen moved to a new
  `wasm-bindgen` org with new maintainers [10-16] [FACT].
- `wasm-pack` is **not dead**: repo `wasm-bindgen/wasm-pack` (old `rustwasm/wasm-pack` redirects),
  not archived, last push 2026-08-12; releases 0.14.0 (2026-01-20) and 0.15.0 (2026-05-15) [10-17]
  [FACT via GitHub & crates.io APIs].
- `wasm-bindgen` 0.2.129 (2026-09-25), `js-sys`/`web-sys` 0.3.106 [05-1] [FACT].
- Targets: `wasm32-unknown-unknown` and `wasm32-wasip2` Tier 2 (std ✓); `wasm64-unknown-unknown`
  Tier 3 [13-1] [FACT].
- Memory: wasm32 linear memory is limited to 4 GiB [UNVERIFIED – prior knowledge]. Memory64:
  Chrome 133, Firefox 134, **Safari only behind a flag (Technology Preview 251)** [10-18] [FACT].
- Threads: Chrome 74 / Firefox 79 / Safari 14.1 [10-18] [FACT], but `SharedArrayBuffer` requires a
  **secure context and cross-origin isolation** (COOP `same-origin` + COEP `require-corp`) [10-19]
  [FACT; header values UNVERIFIED – prior knowledge]. Consequence: a NicheRS server reached over
  plain `http://<LAN-IP>` is not a secure context → no threads [inference]; `http://127.0.0.1` is
  treated as potentially trustworthy [UNVERIFIED – prior knowledge]. COEP also forces every
  cross-origin basemap/tile resource to send CORP/CORS headers.
- Rust WASM threads (`wasm-bindgen-rayon` 1.3.0, 2024-12-21) still need **nightly Rust,
  `-Z build-std`, `+atomics,+bulk-memory`**, `--target web` [10-20] [FACT].
- Bundle size and JS↔WASM transfer cost: no verified numbers gathered [OPEN]; typed arrays can be
  viewed over WASM memory without copying, while structured data crosses via serialisation
  [UNVERIFIED – prior knowledge].

Use-case comparison:

| Use case | Optimised TS | Server-side Rust (localhost/LAN) | WASM | Verdict [REC] |
|---|---|---|---|---|
| Point inference on click (MaxEnt: Σ features·λ, cloglog) | trivial (≤ few hundred features) | 1 HTTP round trip; also needs covariate values that live server-side | duplicate engine in browser + ship covariates | **Server** (values are server-side anyway) |
| Small model evaluation over a raster | needs env. rasters in browser (MB–GB) | natural, Rayon, cached | possible but limited memory, no threads w/o isolation | **Server** |
| Interactive thresholding (area vs threshold) | O(bins) on server-provided histogram | exact recompute on commit | unnecessary | **TS + server** |
| Raster statistics (zonal, histograms) | slow for large rasters | natural | duplicated logic | **Server** |
| Offline use | app already runs locally (local-first single binary) | local server is offline | not needed | **Server (local)** |
| Static, serverless demo / viewer of exported model packages | — | not available | **only case with real value**: same Rust engine code in browser, exact parity | Deferred; candidate post-MVP |

Conclusion: no demonstrated use case for WASM in NicheRS's local-first + server architecture. The
engine reuse argument (one implementation of prediction) is real but only pays off for a
serverless viewer.

## 3. Evidence and references

See References. Versions from npm registry (`registry.npmjs.org/<pkg>`, `dist-tags.latest` and
`time`) and crates.io API on 2026-10-15.

## 4. Alternatives

- Map: OpenLayers (best for native projections, conflicts with mandate); deck.gl (+ deck.gl-raster)
  for GPU COG; Leaflet (no WebGL raster pipeline) — not considered further.
- Rendering: S1 server pre-coloured; S2 value-encoded + GPU ramp; S3 client COG (3857 only).
- Embedding: memory-serve; rust-embed + own handler; include_dir; sidecar `ui/` directory (breaks
  single binary).
- Routing: TanStack Router instead of React Router (type-safe search params) [ASSUMPTION; not
  mandated].
- WASM now vs later vs never.

## 5. Trade-offs

| | S1 | S2 | S3 |
|---|---|---|---|
| Scientific integrity of displayed values | + | + | − (needs extra 3857 copy) |
| Interactivity (ramp/threshold) | − refetch | ++ | + |
| Implementation risk | low | medium (spike) | medium |
| Server CPU | higher | medium | low |

Twelve-factor tension 5 (embedded assets vs frontend dev flexibility) is resolved by: Vite only in
dev (proxy), embedded in release, explicit `NICHE_UI_DIR` override with build-ID mismatch warning.

## 6. Risks

- MapLibre v6 is a fresh major (ESM-only, WebGL2 required) [10-3]: older/locked-down corporate
  machines without WebGL2 get `GPUInitializationError` → need a graceful fallback message [REC].
- shadcn switched default primitives (Base UI) in July 2026 [10-6]; community examples are mostly
  Radix → friction. Pin versions.
- TypeScript 7 / Vite 8 / React Router 8 are recent majors; plugin compatibility [OPEN].
- S2 depends on using DEM-encoding semantics for non-elevation data [ASSUMPTION].
- CSP too strict breaks MapLibre workers or UI positioning — test in CI with a headless browser.
- WASM threads unavailable over non-secure LAN HTTP — relevant only if WASM is adopted.

## 7. Recommendations

- [REC] Stack (pin exact versions in lockfile): React 19.3, TS 7.0 (fallback 5.x/6.x if tooling
  breaks), Vite 8.3 + @vitejs/plugin-react 6, Tailwind 4.3 via `@tailwindcss/vite`, shadcn CLI 4.21
  (Base UI default), lucide-react, TanStack Query 5.104, Zustand 5.0 (viewport/selection/threshold
  only), React Router 8.4 (SPA), RHF 7.90 + Zod 4.6 + @hookform/resolvers 5.9, MapLibre GL JS 6.14
  with `setWorkerUrl` via Vite `?worker&url`.
- [REC] Raster display: implement S1 first (server XYZ tiles from model-grid COGs, PNG/WebP, disk
  cache); spike S2 (value-encoded tiles + `color-relief`) for instant ramp/threshold changes; click
  values always via `/sample` on the native grid.
- [REC] Threshold UI: server returns histogram + per-bin area; TS computes live area; server
  recomputes exact numbers on commit.
- [REC] Embedding: `memory-serve` 2.4.0; immutable caching for hashed assets; `no-cache` for
  `index.html`; CSP as in §2.7; `NICHE_UI_DIR` dev override; build ID injected into both sides.
- [REC] WASM: not in MVP. Keep the prediction core crate free of I/O and add CI
  `cargo check -p niche-core --target wasm32-unknown-unknown` so a serverless viewer stays possible
  at low cost; revisit only with a demonstrated use case.

## 8. Open questions

- [OPEN] Does S2 (custom-encoded `raster-dem` + `color-relief`) give pixel-accurate class
  boundaries and acceptable precision? Spike.
- [OPEN] Minimum browser matrix of client machines (WebGL2 availability on consultants' laptops).
- [OPEN] Basemap source for offline use (bundled low-zoom PMTiles vs online IGN/OSM tiles; licences
  → doc 12).
- [OPEN] Zod 4 + `@hookform/resolvers` 5.9 + shadcn `Field` integration — confirm in spike.
- [OPEN] Measured bundle size of the embedded UI (MapLibre dominant) and its effect on binary size.

## References

[10-1] npm registry metadata (dist-tags.latest, publish time, licence) for all npm packages listed. https://registry.npmjs.org/<package> (accessed 2026-10-15)
[10-2] Vite team, "Vite 8.0 is out!" (2026-03-12). https://vite.dev/blog/announcing-vite8 (accessed 2026-10-15)
[10-3] MapLibre GL JS v5→v6 migration guide (ESM only, CSP, setWorkerUrl, WebGL2). https://maplibre.org/maplibre-gl-js/docs/guides/v5-to-v6-migration-guide/ (accessed 2026-10-15)
[10-4] shadcn/ui, Installation — Vite. https://ui.shadcn.com/docs/installation/vite (accessed 2026-10-15)
[10-5] shadcn/ui, Tailwind v4. https://ui.shadcn.com/docs/tailwind-v4 (accessed 2026-10-15)
[10-6] shadcn/ui changelog, "Base UI as the default" (July 2026). https://ui.shadcn.com/docs/changelog/2026-07-base-ui-default (accessed 2026-10-15)
[10-7] shadcn/ui changelog index. https://ui.shadcn.com/docs/changelog (accessed 2026-10-15)
[10-8] shadcn/ui, Forms — React Hook Form. https://ui.shadcn.com/docs/forms/react-hook-form (accessed 2026-10-15)
[10-9] OpenLayers API, `ol/source/GeoTIFF`. https://openlayers.org/en/latest/apidoc/module-ol_source_GeoTIFF-GeoTIFFSource.html (accessed 2026-10-15)
[10-10] developmentseed/deck.gl-raster README. https://github.com/developmentseed/deck.gl-raster (accessed 2026-10-15)
[10-11] MapLibre Style Spec, Layers (raster, hillshade, color-relief). https://maplibre.org/maplibre-style-spec/layers/ (accessed 2026-10-15)
[10-12] geomatico/maplibre-cog-protocol README, v0.10.0. https://github.com/geomatico/maplibre-cog-protocol (accessed 2026-10-15)
[10-13] tweedegolf/memory-serve README, v2.4.0. https://github.com/tweedegolf/memory-serve (accessed 2026-10-15)
[10-14] rust-embed 8.13.0 crate source (README, Cargo.toml features; rust-embed-utils `Metadata::sha256_hash`). https://crates.io/crates/rust-embed/8.13.0 (accessed 2026-10-15)
[10-15] Vite docs, Server options (`server.proxy`, `server.headers`). https://vite.dev/config/server-options (accessed 2026-10-15)
[10-16] Inside Rust blog, "Sunsetting the rustwasm GitHub org" (2025-07-21). https://blog.rust-lang.org/inside-rust/2025/07/21/sunsetting-the-rustwasm-github-org (accessed 2026-10-15, via search summary)
[10-17] GitHub API repos wasm-bindgen/wasm-pack (archived=false, pushed 2026-08-12); crates.io wasm-pack 0.15.0 (2026-05-15). https://github.com/wasm-bindgen/wasm-pack (accessed 2026-10-15)
[10-18] WebAssembly feature status data, `features.json` (memory64, threads, simd, jspi by browser). https://github.com/WebAssembly/website/blob/main/features.json ; https://webassembly.org/features/ (accessed 2026-10-15)
[10-19] MDN, SharedArrayBuffer — security requirements. https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/SharedArrayBuffer (accessed 2026-10-15)
[10-20] RReverser/wasm-bindgen-rayon README (v1.3.0). https://github.com/RReverser/wasm-bindgen-rayon (accessed 2026-10-15)
[13-1] Rust Platform Support. https://doc.rust-lang.org/nightly/rustc/platform-support.html (accessed 2026-10-15)
[05-1] crates.io API v1 metadata. https://crates.io/api/v1/crates/<name> (accessed 2026-10-15)
