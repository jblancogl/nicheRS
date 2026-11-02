# 06 — Biodiversity and environmental data sources (France / Europe)

Status: research draft · Date: 2026-10-15 · Author: research agent (NicheRS)

Tags: [FACT] verified from a source read on 2026-10-15 · [ASSUMPTION] · [OPEN] · [REC] · [UNVERIFIED – prior knowledge].

## 1. Scope

Which occurrence and environmental-predictor providers NicheRS should ingest for SDM work in
France/Europe, what each one permits (licence, redistribution, commercial use), how to access
and cache it reproducibly, and which occurrence-quality issues the pipeline must handle. The
ingestion design summarised in section 8 is detailed in `docs/architecture/data-pipeline.md`.
Licensing and security implications are in `12-security-and-licensing.md`.

Out of scope: modelling algorithms (MaxEnt etc.), future-climate scenario choice (only noted
where a provider bundles CMIP6 projections).

## 2. Findings — occurrence providers

### 2.1 GBIF (Global Biodiversity Information Facility)

| Aspect | Finding |
|---|---|
| Coverage FR | 215,410,171 occurrence records with `country=FR` on 2026-10-15 [FACT, live API query [1]] |
| Licences per record | Only three licences exist at record level. FR split: CC BY 4.0 = 201,858,892; CC BY-NC 4.0 = 10,521,491; CC0 = 3,029,788 (facet on `license`) [FACT [1]] |
| Search API paging | `limit` silently capped at 300 (requesting 1000 returned `"limit":300`). `offset+limit` > 100,000 rejected with HTTP 400 "Max offset of 100001 exceeded" [FACT, live probe [1]] |
| Download API | Asynchronous, requires a GBIF account; JSON predicate (`and`, `or`, `within` …); formats `SIMPLE_CSV`, `DWCA`, `SPECIES_LIST`, `FASTA_ARCHIVE` (+ `SIMPLE_PARQUET` per formats page) [FACT [2][3]]. Each finished download gets a DOI and a computed licence (most restrictive of included records), e.g. download `0000005-200127171203522` → DOI `10.15468/dl.a2niwr`, licence CC BY-NC 4.0 [FACT [1]] |
| Download concurrency | Historically 3 concurrent downloads per user, reduced to 1 when the global queue is busy; HTTP 420 on excess [UNVERIFIED – secondary sources from 2015–2017 and rgbif docs [4]]. Treat as "assume 1–3, back off on 420" [REC] |
| Search rate limits | No published numeric rate limit found [OPEN]. [REC] ≤ 3 parallel requests, exponential backoff on 429/5xx, identify via User-Agent. |
| Taxonomy | **Breaking change 2026**: GBIF.org switched its default taxonomy to the Catalogue of Life eXtended Release (COL XR); legacy GBIF Backbone (checklistKey `d7dddbf4-2cf0-4f39-9b2a-bb099caae36c`) "will not be updated again"; COL XR key `7ddf754f-d193-4cc9-b351-99906754a03b`. COL XR identifiers are alphanumeric (e.g. `1427067` → `Q2M4`). The occurrence search API still defaults to the legacy backbone unless `checklistKey` is passed; downloads must set `checklistKey` on the TAXON_KEY predicate and at top level [FACT, GBIF data blog 2026-06-23 [5]] |
| Name matching | `/v1/species/match?name=` (legacy) and `/v2/species/match?scientificName=` (multi-taxonomy, accepts `checklistKey`) both live [FACT, live probe [1]] |
| Record quality fields | `coordinateUncertaintyInMeters`, `issues` (e.g. `COORDINATE_ROUNDED`, `TAXON_ID_NOT_FOUND`), `basisOfRecord`, `dataGeneralizations`, `informationWithheld`, `recordedBy` (observer name/pseudonym → personal data) [FACT, sample record [1]] |
| Citation | Data from the search API gets no DOI; GBIF "derived datasets" let a user register a DOI for a filtered/API-sourced subset by listing source datasetKeys + counts and a persistent URL [FACT, secondary: rgbif docs + gbif.org/derived-dataset [6]] |
| Commercial suitability | CC0/CC BY fine with attribution. CC BY-NC: a paid consultancy deliverable is plausibly "primarily intended for commercial advantage" → exclude by default in commercial projects [ASSUMPTION — legal reading, see doc 12] |

Search vs download: [REC] Use the search API only for interactive previews/counts (≤ 100k
records, no DOI). Any model run used in a deliverable must come from a Download API request
(DOI, licence, reproducible predicate), or, if search-sourced, be registered as a derived dataset.

### 2.2 INPN / OpenObs / TAXREF (MNHN – PatriNat)

| Aspect | Finding |
|---|---|
| TAXREF | National taxonomic reference. Current version 18.0 (09/01/2025) per `GET https://taxref.mnhn.fr/api/taxrefVersions` (HAL+JSON, requires `Accept: application/hal+json`) [FACT [7]]. Annual release cadence observed (v15 2021-12, v16 2022-12, v17 2024-01, v18 2025-01) [FACT [7]]; v19 not yet flagged current on 2026-10-15 [FACT [7]]. Licence of TAXREF/API not verified [OPEN]. |
| OpenObs | National query portal on ALA (Living Atlases) stack [FACT [8]]. Default shows only records that passed INPN integration checks; invalid/doubtful visible via advanced search. Exports limited to 2 M records per download (form for more). Sensitive records shown after geographic blurring defined by the national sensitivity reference. Licence stated on home page: Licence Ouverte (Etalab), redistribution under same conditions; diffusion rights set per dataset metadata. FAQ said no web-service API yet [FACT, quoted via search snippets of openobs.mnhn.fr and openobs.mnhn.fr/faq [9]; direct fetch blocked by BunkerWeb WAF on 2026-10-15]. An `openobs.mnhn.fr/api/docs` URL exists (HTTP 200) but content not retrievable non-interactively [OPEN]. |
| Programmatic access, tested 2026-10-19 | `openobs.mnhn.fr/biocache-service/…` and `/api/docs` answer the BunkerWeb anti-bot page to non-interactive clients (with or without a browser User-Agent): no usable API for an automated service without an agreement [FACT, ADR-0114]. |
| GBIF.ES (Spain, for comparison) | `registros-ws.gbif.es` (ALA biocache) answers: 15.5 s per 1,000 records; 2,303 records of *Lacerta bilineata* vs 3,595 in GBIF for Spain; 2,018 of them CC BY-NC (excluded by default) [FACT, measured 2026-10-19, ADR-0114]. Not faster nor more complete than GBIF. |
| Overlap with GBIF | INPN publishes to GBIF (GBIF France node) [UNVERIFIED – prior knowledge]; risk of duplicates when merging OpenObs + GBIF (see 2.6). |
| Commercial suitability | Etalab 2.0 permits commercial reuse with attribution [UNVERIFIED – prior knowledge of Etalab 2.0 text]. |

### 2.3 SINP and regional platforms

- SINP sensitivity scale (protocol 2017, as transcribed in a 2019 Nouvelle-Aquitaine
  decision): 0 = not sensitive; 1 = commune / 5×5 km; 2 = 10×10 km grid; 3 = département;
  4 = no diffusion. Regional lists differ in level 1 wording (e.g. Centre-Val de Loire). The DEE
  exchange standard encodes 0/1/2 (2 = 10×10 km). Legal basis: Code de l'environnement art.
  D411-21-3. Private data max transmitted precision 10×10 km, owner decides [FACT, search
  extracts of DREAL/SINP PDFs [10]]. Regional lists are revised (e.g. PACA birds v1, validated
  by MNHN 2025-06-20) [FACT [10]].
- SILENE (PACA, CBN-run): publishes sensitive-species lists (2025 v1) [FACT [10]]; access and
  licence terms not verified [OPEN].
- Faune-France / Biolovision (LPO network): data governed by regional codes of ethics; records
  split into public/private/management parts; contributors can hide sensitive records; third-
  party communication rules are portal-specific [FACT, regional CGU/charter PDFs [11]]. No open
  API for third parties found [OPEN]. [REC] Treat as "bring your own export" (user uploads a
  file obtained under a convention), never automated harvest.
- GeoNature (PnX-SI, GPL-3.0 [FACT, GitHub API [12]]): Synthèse module exports observations to
  CSV/GeoJSON/Shapefile/GeoPackage, filtered by IDs/cd_ref; export content depends on user
  rights [FACT, GeoNature 2.16.4 docs [13]]. Many regional SINP platforms run GeoNature
  instances [ASSUMPTION]. Natural Solutions develops GeoNature-related tools (per task brief)
  [ASSUMPTION]. [REC] Implement a GeoNature Synthèse connector (authenticated, per-instance)
  as the first "institutional" connector after GBIF: it reuses TAXREF `cd_nom`/`cd_ref`,
  carries sensitivity/diffusion fields, and matches NS clients' workflows.

### 2.4 Occurrence quality issues (literature + implications)

| Issue | Evidence | Pipeline rule [REC] |
|---|---|---|
| Coordinate uncertainty | GBIF `coordinateUncertaintyInMeters` often null; `COORDINATE_ROUNDED` flags [FACT [1]] | Filter `uncertainty ≤ k × cell size` (default k=0.5); null → configurable (keep+flag by default) |
| Generalised / blurred coordinates | `dataGeneralizations`, `informationWithheld` DwC fields; SINP levels 1–4 [FACT [1][10]]; GBIF best-practice guide (Chapman, 2023 ed.) [14] | Records generalised to grid > model resolution excluded from training, kept for presence-at-coarse-scale checks |
| Duplicates | Same observation via GBIF + OpenObs + regional platform [ASSUMPTION] | Dedupe key: (taxon, rounded coords to cell, date) + source-ID crosswalk when available |
| Taxonomic synonymy | GBIF Backbone frozen, COL XR default [5]; TAXREF v18 [7] | Pin taxonomy version per run; resolve to one accepted concept; store both TAXREF `cd_ref` and GBIF/COL key |
| Spatial sampling bias | Phillips et al. 2009 (target-group background) [15]; Syfert et al. 2013 [16]; Barber et al. 2021 [17] | Offer target-group background / bias file; report sampling-density map |
| Thinning | spThin (Aiello-Lammens et al. 2015) [18]; environmental filtering (Varela et al. 2014) [19] | Default 1 record per raster cell (deterministic); optional distance thinning with fixed seed |
| Automated cleaning | CoordinateCleaner tests (centroids, capitals, institutions, zeros, sea) [20] | Port tests natively; each exclusion carries a reason code |
| Temporal mismatch | WorldClim 2.1 baseline 1970–2000 [21]; CHELSA v2.1 1981–2010 [22] | Default occurrence window aligned to climate baseline ± margin; report fraction outside |
| Citizen-science bias | Roads/urban proximity, observer effort [ASSUMPTION – general literature] | Expose basisOfRecord/dataset breakdown; allow per-dataset exclusion |
| Insufficient records | van Proosdij et al. 2016: minimum 3 (artefact-prone) to 13 (virtual area), 14–25 in African case study, depending on range size [23] | Hard block < 5 effective records; warning < 15 (narrow) / < 25 (widespread) after thinning; record decision in manifest |

## 3. Findings — environmental predictors

| Dataset | Coverage / res. | Temporal | Licence (verified?) | Redistribution / commercial | Access |
|---|---|---|---|---|---|
| **WorldClim 2.1** | Global, 30″ (~1 km) to 10′ [FACT [21]] | 1970–2000 normals, released Jan 2020 [FACT [21]] | "freely available for academic use and other non-commercial use. Redistribution or commercial use is not allowed without prior permission." [FACT, worldclim.org/about [24]] | **BLOCKER** for consultancy use and bundling without written permission | HTTP zip |
| **CHELSA v2.1 climatologies** | Global, "kilometer-scale" (30″) | 1981–2010 baseline + 2011–2100 CMIP6 periods [FACT [22]] | **CC0 1.0** on official dataset page [FACT [22]]. Data DOI 10.16904/envidat.228; model paper 10.1038/sdata.2017.122 | Commercial and redistribution OK; cite anyway [REC]. CHELSA-bioclim/BIOCLIM+ licence to confirm separately [OPEN]; CHELSAcruts reported CC BY-SA [UNVERIFIED – secondary] | COG on `os.unil.cloud.switch.ch/chelsa02/` [FACT [22]] |
| **CORINE Land Cover** | EEA38+UK, vector + 100 m raster, 44 classes, MMU 25 ha [UNVERIFIED – prior knowledge for MMU] | 1990, 2000, 2006, 2012, 2018. CLMS page on 2026-10-15 still says most recent update 2018 [FACT [25]]. CLC2024 planned "mid-2026" (CENIA) and Iceland national CLC2024 published 2026-09-08 [FACT, secondary [26]] → EU-wide CLC2024 release status [OPEN] | CLMS data policy: full, open, free (Reg. 1159/2013, 2021/696); credit source, state modifications, no implied endorsement [FACT [27]] | Commercial OK with attribution [ASSUMPTION – policy does not name commercial use explicitly] | CLMS portal (account) |
| **Copernicus HRL (VLCC)** | EEA38+UK, 10 m | Tree cover density/leaf type, grassland, crop types: annual 2017–2021 (release May 2025); Imperviousness built-up 2021 10 m [FACT, secondary snippets [28]] | CLMS policy [27] | As CLC | CLMS portal |
| **ESA WorldCover** | Global, 10 m, 11 classes | 2020 (v100, OA 74.4%), 2021 (v200, OA 76.7%); algorithm differs between versions → do not diff [FACT [29]] | **CC BY 4.0**; attribution "© ESA WorldCover project [year]" [FACT [29]] | OK | AWS S3 `esa-worldcover` (no-sign-request), Zenodo, COG EPSG:4326 [FACT [29]] |
| **OSO (CESBIO/Theia, CNES)** | Metropolitan France, raster 10 m (vector from 20 m) | Annual (2016–); 17 classes 2016–17, 23 classes from 2018 [FACT, GEODES pages [30]] | **Etalab 2.0** [FACT [30]] | OK with attribution | GEODES (CNES) |
| **Copernicus DEM GLO-30 / GLO-90** | Global, 1″/3″, EGM2008 heights | TanDEM-X 2011–2015, released 2019, maintained to 2026 [FACT [31]] | "free license" for GLO-30/90 with attribution notices; EEA-10 restricted [FACT [31]] | OK with attribution (exact licence text to archive) [REC] | CDSE (registration) / API |
| **EU-DEM v1.1** | Europe, 25 m | — | Obsolete/superseded; no longer disseminated by CLMS [FACT [32]] | Do not use | — |
| **IGN RGE ALTI** | France, 1 m / 5 m | Periodic | Etalab 2.0 since 2021-01-01 (IGN open data) [FACT, IGN 2024 doc via secondary [33]] | OK | cartes.gouv.fr / Géoplateforme |
| **ESDAC (ESDB v2.0)** | Europe soils | Static | Registration; **no passing to third parties; non-commercial only** [FACT [34]] | **BLOCKER** for commercial use/redistribution | Manual form |
| **SoilGrids 2.0** | Global, 250 m, 6 depths | Static (2020) | CC BY 4.0 [FACT, secondary metadata + paper [35]] | OK | WebDAV VRT, WCS [FACT [35]] |
| **Bio-ORACLE v3.0** (marine) | Global ocean, 0.05° | Present + CMIP6 SSPs [FACT [36]] | Conflicting: catalogue lists GPL; VLIZ says free for any purpose; site footer "all rights reserved" [FACT [36]] | [OPEN] — clarify before commercial use | ERDDAP |
| **EUNIS habitats** | Europe, classification (+ some probability maps) | 2021 terrestrial revision, corrected Jan 2023 [FACT [37]] | Marine EUNIS CC BY 4.0; terrestrial files licence not found [FACT/OPEN [37]] | [OPEN] | EEA downloads (xlsx) |

## 4. Evidence and references

See section 10 (References). Live API probes were run on 2026-10-15 from the research
workstation (`api.gbif.org`, `taxref.mnhn.fr`).

## 5. Alternatives

- **Climate**: WorldClim 2.1 (most cited, but non-commercial) vs CHELSA v2.1 (CC0) vs
  national products (Météo-France DRIAS/SAFRAN — not investigated) [OPEN].
- **Land cover**: CLC (100 m, long series) vs HRL (10 m thematic) vs WorldCover (10 m, 2 years)
  vs OSO (10 m, annual, France only).
- **Elevation**: Copernicus DEM (pan-EU, consistent) vs RGE ALTI (France, very high res; must be
  aggregated anyway to model resolution).
- **Occurrences**: GBIF (API, DOI) vs OpenObs (more FR records incl. SINP, no API) vs GeoNature
  instances (client-specific, authenticated) vs user uploads.

## 6. Trade-offs

- DOIs and reproducibility (GBIF download) vs latency (asynchronous, minutes to hours).
- Merging providers increases coverage but multiplies duplicates and licence heterogeneity.
- 10 m land cover is far below typical SDM grain (100 m–1 km); aggregation choices (fractional
  cover vs majority class) become modelling decisions that must be logged.
- CC0/CC BY sources simplify redistribution of cached data; non-commercial sources must stay
  user-fetched and never be shipped.

## 7. Risks

- WorldClim and ESDAC licence terms make them unusable in commercial deliverables without
  permission (BLOCKER if chosen as defaults).
- GBIF taxonomy migration (COL XR) changes keys; caches keyed on legacy integer taxonKeys
  become ambiguous [5].
- OpenObs/regional platforms lack APIs → manual exports, weaker provenance.
- Sensitive-record handling errors could expose protected species locations (legal, art.
  D411-21-3).
- CLC2024 release may change the "current" land-cover layer mid-project; pinning required.

## 8. Recommendations

1. [REC] Default climate = CHELSA v2.1 (CC0). WorldClim only as a user-fetched, opt-in source with
   a licence acknowledgement and a non-commercial flag propagated to outputs.
2. [REC] Default occurrences = GBIF Download API with `checklistKey` set explicitly (COL XR), store
   DOI + licence + predicate; exclude CC BY-NC records when project is flagged commercial.
3. [REC] Taxon resolution: TAXREF (pinned version) as the French reference + GBIF v2 match; store
   both IDs; persist the crosswalk in the run manifest.
4. [REC] Land cover: CLC 2018 (until CLC2024 confirmed), HRL for 10 m thematic layers, OSO for
   France; DEM: Copernicus GLO-30. Soil: SoilGrids; never ESDAC by default.
5. [REC] Every exclusion has a reason code; every source a manifest entry (see 8.1).
6. [REC] Minimum-records gate per van Proosdij et al. [23], applied after thinning.

### 8.1 Provenance-aware ingestion (summary)

Manifest per fetched artefact (JSON, schema-versioned):
`source_id`, `provider`, `endpoint`, `request` (canonical JSON predicate / URL),
`query_hash` (SHA-256 of canonical request), `retrieved_at` (UTC), `provider_version`
(e.g. TAXREF 18.0, CHELSA 2.1, CLC 2018 V2020_20u1), `checklist_key`, `doi`, `licence`
(SPDX or URI, per record aggregated + most-restrictive), `attribution_text`,
`content_sha256`, `bytes`, `record_count`, `filters_applied[]` (ordered, with parameters),
`exclusions[]` (`reason_code`, `count`, sample IDs), `sensitivity_policy`,
`commercial_ok` (derived), `tool_version`. Full design: `docs/architecture/data-pipeline.md`.

## 9. Open questions

- [OPEN] Current GBIF download concurrency and search rate limits (official figure).
- [OPEN] Licence of TAXREF API, OpenObs per-dataset diffusion terms, SILENE terms.
- [OPEN] CHELSA-bioclim / BIOCLIM+ licence (confirm CC0 like climatologies).
- [OPEN] Bio-ORACLE v3 licence; EUNIS terrestrial licence.
- [OPEN] EU-wide CLC2024 availability date.
- [OPEN] Whether NS clients hold conventions allowing their GeoNature/Faune-France data in
  multi-client caches.

## 10. References

- [1] GBIF API live probes: `api.gbif.org/v1/occurrence/search` (country=FR, facet=license, limit/offset), `/v1/occurrence/download/0000005-200127171203522`, `/v1/species/match`, `/v2/species/match`. Accessed 2026-10-15.
- [2] GBIF Technical Documentation, "API downloads", https://techdocs.gbif.org/en/data-use/api-downloads. Accessed 2026-10-15.
- [3] GBIF Technical Documentation, "Occurrence download formats", https://techdocs.gbif.org/en/data-use/download-formats. Accessed 2026-10-15.
- [4] rgbif `downloads` docs (3.8.5) https://www.rdocumentation.org/packages/rgbif/versions/3.8.5/topics/downloads ; GBIF api-users list 2017 https://lists.gbif.org/pipermail/api-users/2017-February/000430.html. Accessed 2026-10-15.
- [5] GBIF Data Blog, "How to migrate from the legacy GBIF Backbone Taxonomy to Catalogue of Life Extended Release", 2026-06-23, https://data-blog.gbif.org/post/catalogue-of-life-taxonomic-backbone/. Accessed 2026-10-15.
- [6] rgbif "Citing GBIF mediated data" https://docs.ropensci.org/rgbif/articles/gbif_citations.html ; GBIF derived datasets https://www.gbif.org/derived-dataset/about. Accessed 2026-10-15.
- [7] TAXREF API, `https://taxref.mnhn.fr/api/taxrefVersions` (HAL+JSON). Accessed 2026-10-15.
- [8] GBIF Living Atlases, OpenObs participant page, https://living-atlases.gbif.org/participants/openobs. Accessed 2026-10-15.
- [9] OpenObs home and FAQ, https://openobs.mnhn.fr/ , https://openobs.mnhn.fr/faq (content via search extracts; direct fetch WAF-blocked). Accessed 2026-10-15.
- [10] SINP sensitivity documents: DREAL Nouvelle-Aquitaine decision 2019-44 https://www.nouvelle-aquitaine.developpement-durable.gouv.fr/IMG/pdf/2019-44_decisionlisteaddespsens-signlc_200127.pdf ; SINP Centre notice https://sinp.naturefrance.fr/wp-content/uploads/sites/29/2022/08/Notice_donnees_sensibles_Region_centre.pdf ; SILENE PACA birds 2025 v1 https://silene.eu/wp-content/uploads/2026/05/Donnees_sensibles_SINP_PACA_oiseaux_2025_version1.pdf. Accessed 2026-10-15.
- [11] Biolovision regional charters, e.g. Faune-Charente-Maritime code de déontologie https://cdnfiles2.biolovision.net/www.faune-charente-maritime.org/pdffiles/code_de_deontologie_utilisation005.pdf. Accessed 2026-10-15.
- [12] GitHub PnX-SI/GeoNature (licence GPL-3.0 via GitHub API). Accessed 2026-10-15.
- [13] GeoNature 2.16.4 docs, gn_synthese exports, https://geonature.readthedocs.io/fr/2.16.4/autoapi/geonature/core/gn_synthese/blueprints/exports/index.html. Accessed 2026-10-15.
- [14] Chapman, A.D. Current Best Practices for Generalizing Sensitive Species Occurrence Data, GBIF, ed. 2023-05-01, https://docs.gbif.org/sensitive-species-best-practices/master/en/. Accessed 2026-10-15.
- [15] Phillips et al. 2009, Ecol. Appl. 19:181–197, doi:10.1890/07-2153.1.
- [16] Syfert, Smith & Coomes 2013, PLoS ONE 8:e55158, doi:10.1371/journal.pone.0055158.
- [17] Barber et al. 2021, Divers. Distrib. 28:128–141, doi:10.1111/ddi.13442.
- [18] Aiello-Lammens et al. 2015, Ecography 38:541–545, doi:10.1111/ecog.01132.
- [19] Varela et al. 2014, Ecography 37:1084–1091, doi:10.1111/j.1600-0587.2013.00441.x.
- [20] Zizka et al. 2019, Methods Ecol. Evol. 10:744–751, doi:10.1111/2041-210X.13152.
- [21] WorldClim 2.1 data page https://www.worldclim.org/data/worldclim21.html ; Fick & Hijmans 2017, Int. J. Climatol. 37:4302–4315, doi:10.1002/joc.5086. Accessed 2026-10-15.
- [22] CHELSA climatologies v2.1 dataset page https://www.chelsa-climate.org/datasets/chelsa_climatologies ; Karger et al. 2017, Sci. Data 4:170122, doi:10.1038/sdata.2017.122 ; data doi:10.16904/envidat.228. Accessed 2026-10-15.
- [23] van Proosdij, Sosef, Wieringa & Raes 2016, Ecography 39:542–552, doi:10.1111/ecog.01509.
- [24] WorldClim "About" (terms), https://www.worldclim.org/about.html. Accessed 2026-10-15.
- [25] CLMS CORINE Land Cover product page, https://land.copernicus.eu/en/products/corine-land-cover. Accessed 2026-10-15.
- [26] CENIA CLMS-CLC2024 project page https://cenia.gov.cz/?p=13464 ; Iceland CLC2024 metadata https://gatt.natt.is/geonetwork/srv/api/records/c1b44b38-372d-4400-9a0d-bc360ea44fe0. Accessed 2026-10-15.
- [27] CLMS data policy, https://land.copernicus.eu/en/data-policy. Accessed 2026-10-15.
- [28] Copernicus HRL VLCC 2017–2021 release (copernicus.eu news, May 2025) and EEA catalogue records (search extracts). Accessed 2026-10-15.
- [29] ESA WorldCover data access, https://esa-worldcover.org/en/data-access. Accessed 2026-10-15.
- [30] CNES GEODES OSO raster/vector pages, https://geodes.cnes.fr/projects/oso-occupation-des-sols-raster-l3b/. Accessed 2026-10-15.
- [31] CDSE Copernicus DEM collection description, https://dataspace.copernicus.eu/explore-data/data-collections/copernicus-contributing-missions/collections-description/COP-DEM. Accessed 2026-10-15.
- [32] CLMS "Products no longer disseminated: EU-DEM" and EEA catalogue (obsolete), https://www.eea.europa.eu/data-and-maps/data/eu-dem. Accessed 2026-10-15.
- [33] IGN "DP IGN opérateur data" 2024, https://www.ign.fr/files/default/2024-05/DP-IGN-operateur-data.pdf ; cartes.gouv.fr RGE ALTI record. Accessed 2026-10-15.
- [34] ESDAC, European Soil Database v2.0 access conditions, https://esdac.jrc.ec.europa.eu/content/european-soil-database-v20-vector-and-attribute-data. Accessed 2026-10-15.
- [35] Poggio et al. 2021, SOIL 7:217–240, doi:10.5194/soil-7-217-2021 ; ISRIC SoilGrids FAQ https://docs.isric.org/globaldata/soilgrids/SoilGrids_faqs.html. Accessed 2026-10-15.
- [36] Assis et al. 2024, Glob. Ecol. Biogeogr. 33:e13813, doi:10.1111/geb.13813 ; https://www.bio-oracle.org/ ; LifeWatch catalogue record. Accessed 2026-10-15.
- [37] EEA EUNIS terrestrial habitat classification 2021, https://www.eea.europa.eu/data-and-maps/data/eunis-habitat-classification-1. Accessed 2026-10-15.
