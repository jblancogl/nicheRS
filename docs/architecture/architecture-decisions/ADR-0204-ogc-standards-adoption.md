# ADR-0204: OGC and community standards adoption

- Status: Proposed
- Date: 2026-10-15
- Related: `../engineering-principles.md` §9, ADR-0202, ADR-0203

## Context

Ecology consultants consume NicheRS outputs in QGIS/ArcGIS, in reports, and sometimes in institutional catalogues. The product owner asked which OGC standards to adopt: OGC API – Processes, STAC, COG, OGC API – Tiles/Maps, GeoPackage, GeoTIFF 1.1 and OGC API – Features. Each standard carries an implementation and test cost, so adoption must be tied to user value.

## Decision

**Adopt in MVP (file formats and metadata):**

| Standard | Use | Value |
|---|---|---|
| OGC GeoTIFF 1.1 (19-008r4) laid out as **OGC COG 1.0** (21-026) | All raster outputs (suitability, binary, uncertainty, MESS): Float32 or UInt8, explicit nodata, CRS, internal tiling, overviews | Opens in every GIS; efficient partial reads for the map viewer and large areas |
| **GeoJSON** (RFC 7946) | API/UI exchange of study areas and occurrence summaries; exports | Native to MapLibre and the web; WGS84 by definition |
| **GeoPackage 1.4** (12-128r19) | Import of occurrences and study areas; export of vector results | Standard QGIS exchange format for French consultancies [ASSUMPTION] |
| **STAC 1.1.0 Item** (static JSON sidecar) | One Item per output raster, linking the manifest (ADR-0203), with bbox, datetime, CRS (projection extension), asset roles and checksum | Catalogue-ready, machine-discoverable outputs (FAIR) at low cost; no STAC API server |

**Later, conditional:** an **OGC API – Processes – Part 1: Core 1.0.0** (18-062r2) facade in server mode (`/processes`, `/processes/{id}/execution`, `/jobs`, `/jobs/{id}`, `/jobs/{id}/results`), mapped onto the native job model. Trigger: a concrete client integration (institutional platform, QGIS processing client, workflow engine). The native job model must already support async execution, status polling, results retrieval and dismiss/cancel, so that the facade is thin.

**Not adopted:**

- OGC API – Tiles 1.0 (20-057) and Maps 1.0.0 (20-058). The UI's tile/preview endpoint is internal.
- OGC API – Features (Part 1 1.0.1). Per-project vector volumes are small and the native JSON API suffices.
- STAC API.

Revisit if server mode must feed external map clients.

## Alternatives

- **OGC API – Processes as the primary API.** Its generic process/job vocabulary is poorly suited to the UI's richer project, species and comparison model, and Part 1 Core 2.0 status is unconfirmed [OPEN].
- **Plain GeoTIFF (non-COG).** It is simpler to write, but loses partial-read efficiency. The incremental cost of COG depends on the raster stack.
- **NetCDF/Zarr outputs.** Suited to multi-dimensional climate scenarios; consider them later for projections across many scenarios and time steps.
- **Shapefile.** Legacy; supported only as import if users require it [OPEN].

## Evidence

[FACT, accessed 2026-10-15]

- GeoTIFF 1.1: OGC 19-008r4, approved 2019-09-10 [E10].
- COG 1.0: OGC 21-026, approved 2023-05-08, published 2023-07-14 [E9].
- STAC spec v1.1.0 release [E8].
- OGC API – Processes Part 1 Core 1.0.0: 18-062r2, approved 2021-08-23, published 2021-12-20. A 2024 tracker targeted v2.0 for June 2025, not confirmed [E6, E7].
- OGC API – Features Part 1 Core 1.0.1: 17-069r4, ISO 19168-1:2020 [E13].
- OGC API – Tiles Part 1: 20-057, approved [E11].
- OGC API – Maps Part 1 1.0.0: 20-058 [E12].
- GeoPackage 1.4.0: 12-128r19 [E14].

## Trade-offs

- COG writing requires tiling and overview generation. That is trivial with GDAL's COG driver [UNVERIFIED – prior knowledge] and non-trivial in a pure-Rust stack. This couples to the raster-stack decision made elsewhere.
- STAC sidecars duplicate some manifest information. This is accepted because the audiences differ (catalogue vs reproducibility).
- GeoJSON is WGS84 only, so modelling-CRS geometries must be reprojected for exchange. The manifest records the modelling CRS.

## Risks

- A COG validator in CI may require GDAL/Python tooling (dev-only, acceptable; not a runtime dependency).
- If the OGC API – Processes 2.0 release changes the job model, the facade design must target the version current at implementation time.

## Compliance

- CI validates every COG output with a COG validator (dev tool).
- STAC Items are validated against the vendored official STAC 1.1.0 JSON Schemas (`jsonschema` crate).
- GeoPackage export is round-tripped through the importer.
- GeoJSON is checked with a strict parser plus a WGS84 bounds check.
- Later: the OGC API – Processes facade is tested with the OGC Executable Test Suite (TEAM Engine) if available for the targeted version [UNVERIFIED].

## Reversibility

High for file formats (additive). The Processes facade is an optional module and removable.

## Rationale

The file-level standards deliver immediate, measurable interoperability: outputs open in GIS tools and catalogues without conversion, at low cost. The API-level OGC standards cost more and have no current consumer, so they wait for a demonstrated integration need.
