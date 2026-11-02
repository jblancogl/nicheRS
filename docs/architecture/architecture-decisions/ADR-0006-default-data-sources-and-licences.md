# ADR-0006 — Default environmental and occurrence data must be licensed for commercial use

- Status: Accepted
- Evidence: `docs/research/06-biodiversity-data.md`, `docs/research/12-security-and-licensing.md`

## Decision
- Climate default: **CHELSA v2.1 (CC0)**. WorldClim 2.1 is *not* offered by default ("Redistribution
  or commercial use is not allowed without prior permission"); users may import it locally
  under their own licence.
- Soil: SoilGrids 2.0 (CC BY 4.0), not ESDAC ESDB (non-commercial, no redistribution).
- Elevation: Copernicus DEM GLO-30/90 (EU-DEM is obsolete). Land cover: CORINE 2018 / ESA WorldCover.
  Implemented (2026-10-20, ADR-0015, optional predictors, none selected by default): Copernicus
  DEM **GLO-90** (Copernicus WorldDEM-90 free licence; DLR/Airbus notice required) and **ESA
  WorldCover 2021 v200** (CC BY 4.0; "© ESA WorldCover project 2021 / Contains modified
  Copernicus Sentinel data (2021) processed by ESA WorldCover consortium"). Both allow commercial
  use with attribution; licence URLs and notices in `docs/architecture/predictors-v1.md`; reports
  and manifests carry the attribution of every source used.
- GBIF: always pin `checklistKey`; use the download API (DOI) for production runs; exclude
  CC BY-NC records by default in commercial projects; carry per-record licences into the manifest.
- The demo dataset is a synthetic virtual species, clearly labelled.

## Risks
Licence terms change; the data catalogue stores licence + access date and `niche doctor` surfaces it.
