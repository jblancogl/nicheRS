# ADR-0011 — Records with coordinates generalised by the publisher

- Status: **Proposed — pending review by Natural Solutions ecologists.** Implemented on
  2026-10-20 (Science v1, phase 2) although proposed, because it only removes points whose
  precision is unknown; see "Why it is implemented before the review".
- Date: 2026-10-20
- Amends: ADR-0009 §1 (occurrence cleaning) — one new rule, placed before the uncertainty rule.
- Design: the design notes §4 (A1 option 2, A3 option ii, A4, A5 option 1)
  and §6.1 items 3–8. Data layer: `docs/architecture/data-sources-v1.md` §1.
- Implementation: `crates/niche-app/src/env.rs::clean`, `engine.rs` (`generalised_warnings`,
  manifest), `artifacts.rs`, `report_html.rs`, `dto.rs` + `web/src/api/types.ts`, UI and help.
- Tests: `env.rs` unit tests (donnees-sensibles-rgpd.md §7 tests 1–4), `engine.rs`
  `generalised_record_warnings` (tests 3, 4, 6), `crates/niche-app/tests/bias_options.rs`
  (defaults bit-identical).

## Context

Publishers of sensitive species blur their coordinates (French SINP rules, art. D411-21-3 of the
code de l'environnement; Chapman 2020 for GBIF). GBIF reports it in the Darwin Core fields
`dataGeneralizations` and `informationWithheld`, free text. Observed on 2026-10-16
(donnees-sensibles-rgpd.md §2): iNaturalist raises the uncertainty to ≈ 27.5 km, PatriNat to
5 km, Observation.org rounds to 0.01° with 1 000 m, a camera-trap project generalises to a 400 m
grid with 283 m. ADR-0009 excludes records with uncertainty > 1 000 m but **keeps records with
no uncertainty**, so a blurred point without a stated uncertainty was used as if it were
precise: a point moved to the centre of a 10 km square trains the model on the wrong cell.

Only the GBIF search API returns the two fields. SIMPLE_CSV downloads (production) and the
regional mirror built from them do not: `gbif::Occurrence::generalised` is `None` there
(unknown, never "not generalised"; `gbif::generalisation_flags_available`).

## Decision

1. **Rule `generalised`** (exclusion code `generalised`, reason "Coordinates generalised by the
   publisher; precision not stated or > {max} m"), applied after the date rules and **before**
   the uncertainty rule: a record flagged as generalised (`generalised == Some(true)`, i.e. one
   of the two fields is non-blank) is excluded when its uncertainty is **missing or greater than
   `max_uncertainty_m`**. A generalised record with a stated uncertainty within the limit is kept
   and counted. Records not flagged (`Some(false)`) or of unknown status (`None`) follow ADR-0009
   unchanged.
2. Consequences for the observed cases: blurred to 27 km or 5 km → moves from the `uncertain`
   reason to `generalised` (total exclusions unchanged); 0.01° with 1 000 m or 400 m grid with
   283 m → kept and counted; blurred without uncertainty → newly excluded (the only change of the
   training set).
3. **Counts**: records flagged, excluded by the rule, and kept as presences
   (`SpeciesResult.data.generalised = {flags_available, records, excluded, retained}`, absent for
   synthetic data).
4. **Warnings** (never hideable in the UI):
   - `generalised_records`: *Info* as soon as one record is flagged, with both counts;
     *Warning* when flagged records are **≥ 10 %** of the records whose coordinates fall in the
     study area (threshold proposed in donnees-sensibles-rgpd.md §A4, to be confirmed by the
     ecologists). Text: "{n} records were generalised by their publishers (sensitive species or
     observer request): {x} excluded, {r} retained with stated precision ≤ {max} m. Retained
     presences may under-represent the most sensitive sites."
   - `generalisation_flags_unavailable` (*Info*) for every species whose records come from a
     download or the mirror: "This GBIF download format does not report coordinate
     generalisation; generalised records with small or missing uncertainty cannot be detected."
   - No effect on the evidence level (a cap is the ecologists' decision, §A4).
5. **Exports**: excluded points keep their published coordinates with status `generalised` in
   `occurrences.geojson`. For real data, the description of `occurrences.geojson`, `report.md`
   and `report.html` carry: "Coordinates as published by GBIF. Some species are sensitive under
   French SINP rules: check the regional référentiel before reproducing points in a public
   report." (not for the synthetic demo, which has no sensitive species).
6. **Manifest**: `occurrence_source.download_format` (`SIMPLE_CSV` | `search API (JSON)`),
   `occurrence_source.generalisation_flags_available`, per species
   `generalisation_flags_available`, `generalised_records`, `generalised_excluded`,
   `generalised_retained`; `methods.cleaning` names this ADR.

## Why it is implemented before the review

ADR-0009 says cleaning rules cannot be switched off, so this cannot be an opt-in; it changes a
default. It is implemented now, while *Proposed*, because:

- it only removes points whose precision is **unknown** to NicheRS and declared doubtful by
  their publisher; keeping them is the error the rule fixes (Chapman 2020 warns that blurred
  points used without care give misleading climatic models);
- it changes nothing where the flag is unknown: production downloads and the mirror give `None`,
  so their results are identical; the demo is identical (checked byte for byte, see Evidence);
- for the search-API path, results change only for species with generalised records — this is
  the intended, documented change; a species without such records is cleaned exactly as before;
- reverting is trivial if the ecologists decide otherwise (one rule in `clean`).

## Evidence

- Unit tests of donnees-sensibles-rgpd.md §7: test 1 (generalised, no uncertainty → excluded,
  counted), test 2 (27 505 m → `generalised` instead of `uncertain`, same total), test 3 (283 m →
  retained, counted, Info), test 4 (no flagged record → same statuses and counts as with the
  flag false or unknown; no warning), test 6 (download source → `generalisation_flags_unavailable`).
- Default demo (`niche demo`) before (`f07f9b2`) and after: byte-identical rasters, lambdas,
  curves, `occurrences.geojson`, `result.json`; reports identical except their generation time;
  same `spec_sha256`.
- Live search-API study (*Testudo hermanni*, 2026-10-20; `docs/science-v1-plan.md`, phase 2):
  339 of 1 509 records flagged generalised, 108 of them moving from the uncertainty reason to
  the new one; total exclusions, presences, metrics and evidence level unchanged.

## Not decided here (open for the ecologists)

1. Confirm option A3-ii and the rule's position.
2. The 10 % warning threshold and whether to cap the evidence level.
3. Whether non-generalised records **without** uncertainty should be excluded too (a broader
   ADR-0009 question, deliberately not mixed in).
4. Downloads in DWCA or SQL format to obtain the flags in production (donnees-sensibles-rgpd.md
   §A2); until then the `generalisation_flags_unavailable` warning says the rule cannot apply.

## References

- Chapman, A.D. (2020) *Current Best Practices for Generalizing Sensitive Species Occurrence
  Data*. GBIF Secretariat. https://doi.org/10.15468/doc-5jp4-5g10
- Code de l'environnement, art. L124-4, L411-1 A, D411-21-3 (décret n° 2016-1619).
