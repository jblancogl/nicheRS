# ADR-0012 — Study areas from self-hosted administrative boundaries

- Status: Accepted (2026-10-16). Product request (session of 2026-10-16): pick
  any administrative unit of a country "very simply", with boundaries hosted by the service
  itself, as small as possible; start with France and Spain.
- Extends ADR-0006 (licences) and ADR-0010 (native grid). Implementation:
  `scripts/build_boundaries.py`, `crates/niche-geo/src/boundaries.rs`,
  `crates/niche-cli/src/boundaries_cmd.rs`, `crates/niche-app/src/engine.rs`
  (`search_places`, `places_at`, `place_geometry`), `crates/niche-server/src/lib.rs`
  (`/api/v1/places`), `web/src/features/setup/PlacePicker.tsx`.

## Context

Drawing or importing a polygon is slow and error-prone when the study area is an administrative
unit (a department, a province, a group of communes), which is the usual case in consultancy
work. GADM is not licensed for commercial use; geocoding services (Nominatim/OSM) add an external
dependency, usage limits and a moving target for reproducibility.

## Decision

1. **Sources** (commercial reuse with attribution, ADR-0006):
   - FR: IGN ADMIN EXPRESS COG CARTO 4.0, edition 2026-01-01 — Licence Ouverte 2.0 (Etalab).
     Régions, départements, EPCI, communes (with population).
   - ES: IGN España, OGC API Features `administrativeunit` (INSPIRE AU) — CC BY 4.0 equivalent.
     Comunidades/ciudades autónomas, provincias, municipios; hierarchy from `nationalcode`;
     the 7 shared "Territorio" units are left out.
   Levels: country, region, department (FR département / ES provincia), intercommunality (FR
   EPCI, attached for display to its first member department), municipality.
2. **Build offline, ship a pack.** `scripts/build_boundaries.py` (pyogrio + shapely, outside
   shipped code) downloads, simplifies per level (Douglas–Peucker, 0.004° country → 0.0005°
   municipality, ~55 m — below the ~1 km modelling cell), and writes NDJSON plus
   `*.sources.json` (edition, licence, SHA-256 of each download). `niche boundaries build`
   encodes it into a `.nrsb` pack: deflated JSON index, then one deflated geometry blob per unit
   (varint, zig-zag deltas, coordinates quantized to 1e-4°). FR + ES: **10.4 MB, 44 550 units**.
3. **Self-hosted.** `niche boundaries install <pack>` copies it to `<data_dir>/boundaries/`; the
   server loads every pack at start-up (no network at run time). Packs and downloads are never
   committed. Search folds accents and case, ranks exact name, then prefix, then larger level,
   population and area; extra words may name an ancestor ("saint gironde"). Countries are also
   found by their names in the UI and co-official languages (`aliases`).
4. **API.** `GET /api/v1/places?q=&country=&limit=`, `/places/at?lng=&lat=`, `/places/{id}`
   (GeoJSON); `/health.boundaries` lists the packs. The UI offers "Place" first only when a pack
   is installed.
5. **Size before launch.** The modelling grid covers the bounding box of the whole area, so a
   country with distant islands or overseas territories exceeds the hosted limit (2.5 M cells) at
   1 km: Spain with the Canary Islands needs 5.2 M. Hence two extra units, "France métropolitaine"
   (`FR-METRO`, régions without 01–06) and "España peninsular y Baleares" (`ES-PENIN`, without
   Canarias, Ceuta, Melilla and the plazas de soberanía), which fit at 1 km; and
   `POST /api/v1/study-areas/check`, which gives the cells at each offered resolution so the
   wizard shows the finest possible one at the Area step, disables the others and never launches
   a study the server would refuse.
6. **Protected areas** (second pack, `protected-fr-es-2026.1`, 3.0 MB, 4 323 areas), built with
   `scripts/build_boundaries.py --set protected --land <admin ndjson>`:
   - FR: MNHN/PatriNat — INPN base des espaces protégés (metropolitan GeoPackage) and Natura 2000
     sites, Licence Ouverte 2.0 (SIB records on data.gouv.fr; the INPN site was down after the
     MNHN cyber-attack, files come from the temporary PatriNat page — hence the SHA-256 kept).
     Kept: parcs naturels régionaux, parcs nationaux (cœur, aire d'adhésion), réserves naturelles
     (nationales, régionales, de Corse), réserves biologiques, APB, Ramsar, réserves de chasse et
     faune sauvage, Natura 2000 (SIC/ZSC, ZPS; sites ≥ 95 % marine left out).
   - ES: MITECO Banco de Datos de la Naturaleza — Espacios Naturales Protegidos 2025 and Red
     Natura 2000 (ZEC, ZEPA), CC BY 4.0 citing MITECO. Single trees, caves and the ENP entries
     duplicating Natura 2000 are left out.
   - Left out everywhere: areas with less than 1 km² of land (below one modelling cell) —
     measured against the departments/provinces of the administrative pack. Boundaries are
     informative (both providers say so); the legal limits are in each designation act.
   - Units have level `protected`, their official `designation` in the source language and their
     own source (attribution per unit). WDPA is not used (no commercial use).
   - Search ranks both packs together (`search_packs`), protected areas after departments, so
     communes cannot crowd out a park of the same name.
7. **Pick on the map.** While the Place method is open, a click on the map lists every unit and
   protected area containing that point (`/places/at`), each one-click addable.
8. **Traceability.** A study area built from places is the union (MultiPolygon) of their
   geometries and keeps `places: [{id, name, pack, attribution}]` in the job specification, hence
   in its hash and the execution manifest; `report.html` / `report.md` cite the boundary
   attribution. Areas without places serialise exactly as before (unchanged hashes).

## Consequences

- One pack per release of the sources (e.g. `fr-es-2026.2`, which added the mainland units to `fr-es-2026.1`); a new edition is a new pack name,
  so earlier studies remain traceable to the boundaries they used.
- More countries need a commercially licensed source each (national mapping agencies;
  geoBoundaries where its per-country licence allows it). Protected areas (Natura 2000 from the
  EEA) are a possible second pack type; WDPA is not licensed for commercial use.
- Simplified boundaries are for study areas on a ~1 km grid, not for legal or cadastral use.
