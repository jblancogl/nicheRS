# NicheRS

Created by Javier Blanco.

Species distribution modelling (SDM) for ecological consultancies: **one self-contained
executable** that serves a map-centred web application and an API, fits MaxEnt models validated
against the original `maxent.jar`, and reports suitability, uncertainty, extrapolation and
suitable-conditions area with an explicit evidence level.

Start with [`docs/executive-summary.md`](docs/executive-summary.md); MVP status against the
definition of done is in [`docs/mvp-status.md`](docs/mvp-status.md).

## Quick start

```bash
./scripts/build-release.sh          # builds web UI (npm) + tests + stripped binary
./target/dist/niche serve           # http://127.0.0.1:8080
```

No Java, R, Python or Node is needed to run the binary. On first use it downloads, on demand and
cached: CHELSA v2.1 climate windows (CC0), Natural Earth land mask (public domain) and GBIF
occurrences. The **synthetic demo** works fully offline (`niche demo`, or "Try the synthetic demo"
in the UI) and is labelled synthetic everywhere.

## What it does

1. Search a species (GBIF backbone), draw/import a study area (GeoJSON, WGS84).
2. Review data: GBIF records cleaned with explicit exclusion reasons (licence, date, coordinate
   uncertainty, sea/lakes, duplicates per cell), spatially stratified retrieval when capped.
3. Predictors: CHELSA v2.1 bioclim (1981–2010), collinearity filter |r| ≤ 0.7.
4. Model: MaxEnt (Rust port of 3.4.4), tuned over feature classes × regularization with spatial
   block cross-validation; selection by omission rate then Boyce index (ADR-0009).
5. Results: relative suitability map (never called probability), uncertainty (fold SD), MESS
   extrapolation hatch, threshold band (P10 / max SSS / MTP) with exact ellipsoidal areas, evidence
   level with reasons, per-variable contributions, methods report, COG/GeoJSON/.lambdas exports,
   execution manifest with checksums.
6. Batch: many species per job (UI or `niche run spec.json`); jobs persist in SQLite and resume
   after a crash or restart.
7. Expert options (Science v1, **off by default, pending review by Natural Solutions
   ecologists**; [`docs/science-v1-plan.md`](docs/science-v1-plan.md)): target-group background
   and distance thinning (ADR-0013); null models, autocorrelation-sized grid blocks, area of
   applicability (ADR-0014); terrain, land-cover and distance-to-water predictors (ADR-0015);
   CHELSA CMIP6 future projections (ADR-0016); random forest and MaxEnt + RF ensemble
   (ADR-0017); the organisation's own field data with independent presence–absence validation
   (ADR-0108); survey priority, project footprint, multi-species richness and study comparison
   (ADR-0018). The one rule applied by default is ADR-0011 (publisher-generalised search-API
   records without adequate precision are excluded).

## Commands

```text
niche serve        web app + API (NICHE_HOST, NICHE_PORT, NICHE_DATA_DIR, NICHE_CACHE_DIR,
                   NICHE_MAX_WORKERS, NICHE_MAX_RECORDS, NICHE_LOG_LEVEL, NICHE_LOG_FORMAT,
                   NICHE_ALLOWED_HOSTS, NICHE_UI_DIR)
niche run SPEC     headless batch from a job spec (same JSON as POST /api/v1/jobs)
niche demo         synthetic offline demonstration job
niche doctor       configuration, storage and data-source checks
niche cache inspect|clean
niche export-swd SPEC --out DIR   cleaned presences + background as MaxEnt SWD CSV
niche train-swd / predict-swd     MaxEnt on SWD files (comparison with maxent.jar)
```

Example specs: `tests/real-specs/*.json`.

## Development

```bash
export PATH=$HOME/.cargo/bin:$PATH
cargo test --workspace                       # unit, golden (vs maxent.jar refs), e2e demo, recovery
cargo test -p niche-data --test live -- --ignored   # network smoke tests (CHELSA, GBIF)
cargo run -p niche-cli -- serve              # API on :8080 (UI from embedded build)
(cd web && npm install && npm run dev)       # Vite dev server, proxies /api to :8080
```

Reference comparison with `maxent.jar` 3.4.4 (Java ≥ 11):

```bash
mkdir -p reference/vendor/jar && curl -L -o reference/vendor/jar/maxent.jar \
  https://github.com/mrmaxent/Maxent/raw/master/ArchivedReleases/3.4.4/maxent.jar
python3 -I scripts/gen_synthetic.py
for n in bretagne_lacerta occitanie_quercus_ilex auvergne_salamandra normandie_erinaceus nordest_rana_dalmatina; do
  ./target/release/niche export-swd tests/real-specs/$n.json --out bench-out/real-$n && \
  mkdir -p bench-out/real/$n && cp bench-out/real-$n/*/*.csv bench-out/real/$n/
done
cargo build --release && python3 -I scripts/compare_maxent.py --reps 3   # → docs/benchmarks/phase3-results.md
```

Rules for contributors and coding agents: [`AGENTS.md`](AGENTS.md).

## Data credits

CHELSA v2.1 (Karger et al. 2017, 2021; CC0) · GBIF.org occurrence data (record-level licences
CC0 / CC BY, see each job manifest) · Natural Earth (public domain) · OpenFreeMap / OpenStreetMap
contributors (basemap, ODbL). Only when the corresponding option is used (attribution in each
report and manifest): Copernicus WorldDEM-90 (Copernicus DEM GLO-90) · ESA WorldCover 2021 v200
(CC BY 4.0) · CHELSA CMIP6 projections (CMIP6 model output, ISIMIP3b bias adjustment).

## Licence

MIT. Includes a port of MaxEnt 3.4.4 (© 2016 Phillips, Dudík & Schapire, MIT) — see `NOTICE`.

## Web UI (frontend)

React + TypeScript (strict) app in `web/`, built with Vite into `web/dist` (embedded by the
server, ADR-0004). Requires Node ≥ 22.

```bash
cd web
npm ci
npm run dev        # http://localhost:5173, proxies /api to http://127.0.0.1:8080
NICHE_API_URL=http://127.0.0.1:8091 npm run dev   # if the API runs on another port
npm run typecheck  # tsc -b --noEmit
npm run lint       # eslint, zero warnings allowed
npm run build      # -> web/dist (content-hashed assets)
npm run preview    # serve the production build (same /api proxy)
```

Stack: Tailwind CSS v4 with the design tokens of `docs/design/design-tokens.md`, shadcn/ui on
Base UI (components owned in `web/src/components/ui`), TanStack Query, Zustand (study draft, map
view, theme), React Hook Form + Zod, MapLibre GL JS with the OpenFreeMap basemap (falls back to
a plain background offline). The API contract is `web/src/api/types.ts`.
