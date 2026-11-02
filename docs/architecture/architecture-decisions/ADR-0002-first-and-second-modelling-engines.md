# ADR-0002 — MaxEnt first; down-sampled Random Forest as second engine

- Status: Accepted
- Evidence: `docs/research/01-sdm-state-of-the-art.md` (Valavi et al. 2022; Elith et al. 2006, 2011; Renner & Warton 2013)
- Refined by: ADR-0017 (*Proposed*) — the down-sampled random forest (`crates/niche-rf`) and a
  MaxEnt + RF ensemble are implemented as opt-in algorithms (`model.algorithm`, Science v1
  phase 6); MaxEnt remains the default.

## Decision
1. Engine 1 (MVP): MaxEnt-compatible (ADR-0001), presented as a presence-background model of
   **relative suitability**; tuned per species (feature classes × β) with spatial block CV.
2. Engine 2 (post-MVP): down-sampled (balanced) Random Forest, chosen for strong benchmark
   performance with few hyperparameters and a different inductive bias (useful to expose
   algorithm disagreement).
3. Ensemble (MaxEnt + RF-ds + BRT) is a later target; JSDMs and deep learning are out of scope until
   presence-absence community data / large training sets justify them.

## Alternatives
GLM/GAM (good interpretability, weaker on complex responses), BRT/XGBoost (heavy tuning), JSDM
(needs site × species presence-absence), DL (data-hungry, hard to audit).

## Risks
Presence-background outputs cannot estimate prevalence; UI and exports must never call them
probabilities (ADR-0007).
