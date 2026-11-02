# ADR-0007 — Output semantics: relative suitability, evidence levels, no confidence percentage

- Status: Accepted
- Evidence: `docs/research/07-scientific-validation.md`, `docs/research/09-ux-research.md`

## Decision
- Maps are "relative habitat suitability (0–1, cloglog transform)"; never "probability of presence".
- Headline validation: continuous Boyce index and omission at the training-10th-percentile
  threshold under spatial block CV, plus a null-model comparison. AUC only in detail views,
  labelled "presence-background AUC".
- Confidence is communicated as separate indicators (data adequacy, CV evidence, per-cell spread
  across folds, extrapolation MESS/AOA) combined by explicit rules into an evidence level
  (Exploratory / Indicative / Supported) with listed reasons. Cut-offs to be agreed with
  Natural Solutions ecologists before release.
- Suitable area is computed on an equal-area grid (EPSG:3035) or with ellipsoidal cell areas,
  reported as a band across thresholds (MTP, P10, max SSS) and folds: "area of suitable
  conditions", not occupied range.
