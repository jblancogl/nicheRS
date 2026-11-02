// English help content — SOURCE OF TRUTH for structure (article and section ids, UI strings).
// Every number and rule here is checked against ADR-0009 and crates/niche-app/src (fit.rs,
// env.rs, engine.rs, artifacts.rs). Change the science only through an ADR, then update all
// three languages. Pending review by Natural Solutions ecologists (docs/design/help-content.md).

import type { ArticleContent } from "./types";

const ui = {
  help: "Help",
  helpCentre: "Help centre",
  openHelp: "Open help",
  close: "Close help",
  language: "Help language",
  search: "Search help",
  searchPlaceholder: "Search the help…",
  noResults: "No article matches your search.",
  articles: "Articles",
  onThisPage: "In this article",
  helpOn: "Help:",
  openPage: "Open as a page",
  backToApp: "Back to NicheRS",
  back: "All articles",
  previous: "Previous",
  next: "Next",
  reviewNote:
    "Help content pending review by Natural Solutions ecologists. It describes the rules implemented in this version of NicheRS (ADR-0009); for a given study, its report and manifest are authoritative.",
  languageNote:
    "Help and the interface are available in English, French and Spanish. The language is chosen in the user menu, at the right of the top bar, and applies to the whole interface, help included (in the hosted service it is saved with your account).",
  hintTitle: "New to NicheRS?",
  hintBody:
    "“Getting started” explains the workflow from species to results and how to read the map, in about five minutes.",
  hintOpen: "Read Getting started",
  hintDismiss: "Dismiss",
  demoHelp: "What is the synthetic demo?",
  readingHelp: "How to read these results",
  pathHelp: "What does the quick mode decide for me?",
  decisionHelp: "What these outputs answer, and what they do not",
} satisfies Record<string, string>;

const articles = {
  "getting-started": {
    title: "Getting started",
    summary:
      "What NicheRS does and the seven steps from a species to reviewed results.",
    sections: {
      overview: {
        title: "What NicheRS does",
        blocks: [
          {
            p: "NicheRS fits a MaxEnt species distribution model from GBIF occurrence records and CHELSA climate data, validates it with spatial cross-validation and maps **relative habitat suitability** over your study area.",
          },
          {
            p: "It tells you where climatic conditions resemble those where the species has been recorded. It does not tell you where the species is: see [What the map shows](#map).",
          },
        ],
      },
      paths: {
        title: "Quick or Expert",
        blocks: [
          {
            p: "“New study” asks how you want to set it up; the choice is remembered in this browser and can be changed at any time with the **Quick / Expert** switch above the steps, without losing what you chose.",
          },
          {
            ul: [
              "**Quick** — three steps: one species, the study area, then **Check & launch**. Data cleaning, the CHELSA climate variables, the grid resolution (the finest the area allows) and the model calibration keep the recommended settings ([Data cleaning](#data), [Variables & collinearity](#variables), [Validation](#validation)).",
              "**Expert** — the six steps described below: several species at once, data filters and resolution, choice of variables, model and validation settings.",
            ],
          },
          {
            note: "Both paths run the same validated model, show the same data warnings and record every setting in the study's report and manifest. A quick study is not a lighter analysis: it is the recommended analysis, with fewer questions.",
            tone: "info",
          },
        ],
      },
      workflow: {
        title: "The workflow, step by step",
        blocks: [
          {
            ol: [
              "**Species** — in the workspace (the home page, with your projects and studies), “New study” opens this step, inside the selected project (“Continue this study” resumes a study being prepared in this browser). Search the GBIF taxonomic backbone and pick one or more species (up to 25 per study in the hosted service). Choose a species, not a genus: a higher rank pools several niches and caps the evidence level at *Exploratory*.",
              "**Study area** — search a **place** — one or several municipalities, intercommunalities (FR), departments or provinces, regions, countries or protected areas (parks, reserves, Natura 2000), whose union becomes the area; or click the map to list the places at that point (France and Spain, official IGN, INPN and MITECO boundaries hosted by the server) — or choose a preset, draw a polygon on the map or import a GeoJSON file in WGS84. The model is fitted and predicted only inside this area, and background points are drawn from it.",
              "**Data review** — NicheRS fetches GBIF records, cleans them and shows how many were kept and why the others were excluded. Look at the map for implausible records before continuing. The year, coordinate-uncertainty and licence filters and the grid resolution can be changed here. See [Data cleaning](#data).",
              "**Variables** — choose CHELSA climate predictors (a recommended set is pre-selected) and, optionally, terrain, land-cover and distance-to-water predictors ([Terrain, land cover & water](#predictors)). Strongly correlated variables are dropped automatically when the study is computed. See [Variables & collinearity](#variables).",
              "**Model** — recommended settings are pre-filled (automatic tuning, spatial block cross-validation). Change them only for a documented reason; the Review step lists every non-default value.",
              "**Launch** — “Launch study” on the Review step starts the computation on the server. You can leave the page and come back through **Studies** in the top bar; progress and warnings are shown live. A study first waits for GBIF to prepare the complete occurrence file (with a DOI), usually 2–15 minutes in GBIF's queue: the page shows the wait, GBIF's status and the last check, and links to the download on gbif.org. If GBIF is unusually slow (30 minutes), you may continue without the download: records then come from the GBIF search API, without a DOI and with at most a fixed number per species.",
              "**Results** — for each species: **Summary** (warnings, evidence level, area of suitable conditions, key metrics), **Validation**, **Variables**, and **Data & exports**.",
            ],
          },
        ],
      },
      reading: {
        title: "Reading results in the right order",
        blocks: [
          {
            ol: [
              "Read the **warnings** first (data quality, extrapolation, taxon rank, truncated downloads).",
              "Read the **evidence level** and every reason listed under it. See [Evidence levels](#evidence).",
              "Look at the map with the **extrapolation hatch** on, then switch to the **uncertainty** layer.",
              "Only then use the **area of suitable conditions**, always with its threshold rule and its band. See [Suitable-area band](#area).",
            ],
          },
        ],
      },
      demo: {
        title: "The synthetic demo",
        blocks: [
          {
            note: "The demo uses **synthetic data**. Its outputs describe a virtual species in an artificial environment and must never appear in a deliverable or be presented as a prediction for a real species.",
            tone: "warn",
          },
          {
            p: "“Try the synthetic demo”, in the workspace while it has no study, or on the Species step, runs the whole workflow on artificial data: a virtual species, *Virtualis demonstrandum*, with a known response to three synthetic predictors (temperature, precipitation, ruggedness) over a fixed region of south-eastern France (3–6° E, 43.8–45.8° N). No GBIF records and no CHELSA climate are used.",
          },
          {
            p: "Demo results are labelled synthetic: a **SYNTHETIC** badge next to the species name and a notice on the results page, “(SYNTHETIC)” in the species and study-area names, and `synthetic_data: true` in `report.md` and the manifest. Use the demo to learn the interface and how to read the outputs.",
          },
          {
            p: "In the hosted service a demo study is a normal study: when its model is fitted it counts as one modelled species in your monthly quota. In a local installation the same demo is available from the command line (`niche demo`).",
          },
        ],
      },
      keyboard: {
        title: "Keyboard shortcuts",
        blocks: [
          {
            table: {
              caption: "Keyboard shortcuts",
              head: ["Keys", "Action"],
              rows: [
                ["Arrows / + −", "Pan / zoom the focused map"],
                ["I", "Inspect values at the map centre (results)"],
                ["0", "Zoom to the study area"],
                [
                  "Enter / Esc / Backspace",
                  "Finish / cancel / undo while drawing",
                ],
                ["Esc", "Close this help panel"],
              ],
            },
          },
        ],
      },
    },
  },

  map: {
    title: "What the map shows",
    summary:
      "Relative habitat suitability, the cloglog transform, and what the map is not.",
    sections: {
      suitability: {
        title: "Relative habitat suitability",
        blocks: [
          {
            p: "The main layer is **relative habitat suitability**: a score from 0 to 1 computed by the MaxEnt model for every grid cell (about 1 km) of the study area. Higher values mean environmental conditions more similar to those of the cells where the species was recorded, relative to the conditions available in the study area (the background).",
          },
          {
            ul: [
              "Scores are relative to this study area, these records and these variables. Do not compare scores between species or between study areas.",
              "The colour ramp is fixed from 0 to 1 and never stretched, so a pale map really means low scores.",
            ],
          },
        ],
      },
      cloglog: {
        title: "The cloglog transform",
        blocks: [
          {
            p: "MaxEnt's raw output is rescaled to 0–1 with the **cloglog** transform (Phillips et al. 2017). The transform keeps the ranking of cells; it does not make the score a calibrated probability. With presence-only records and unknown prevalence (how common the species is in the area), no transform can.",
          },
        ],
      },
      not: {
        title: "What the map is not",
        blocks: [
          {
            note: "The map is **not a probability of presence**, **not the occupied area** (or range) and **not proof of presence or absence**.",
            tone: "warn",
          },
          {
            p: "A high score says that conditions in a cell resemble those at known records. The species may still be absent (dispersal barriers, destroyed habitat, competition, land use or habitat structure that the climate variables do not describe). A low score does not prove absence either, especially in poorly surveyed areas.",
          },
        ],
      },
      layers: {
        title: "Map layers",
        blocks: [
          {
            ul: [
              "**Suitability (relative, 0–1)** — relative habitat suitability of the final model (main layer).",
              "**Uncertainty (SD across folds)** — spread of suitability between the cross-validation models. See [Uncertainty & extrapolation](#uncertainty).",
              "**Extrapolation (MESS < 0)** — cells outside the conditions seen in training. The **extrapolation hatching overlay** shows them over the other layers and is on by default.",
              "**Suitable / not suitable at threshold** — cells at or above the threshold of the rule selected in Summary. See [Suitable-area band](#area).",
              "**Sampling effort (target-group records)** — only for studies with a target-group background: GBIF records of the target group per cell, orange on a log scale. It is observed effort, not a prediction. See [Target-group background](#data/targetgroup).",
              "**Occurrences** — occurrence records drawn as markers, either retained for modelling or excluded by a filter: they are observed, not predicted.",
            ],
          },
          { p: "Click a cell (or press **I**) to read its local values." },
        ],
      },
      replaces: {
        title: "What it does not replace",
        blocks: [
          {
            ul: [
              "Field inventories and survey protocols.",
              "An impact assessment and its regulatory analyses.",
              "Expert knowledge of the site: habitats, land use, barriers, microhabitats.",
              "Presence data from naturalist databases and protection status.",
            ],
          },
          {
            p: "Use it to prioritise survey effort, frame questions and document a reasoned hypothesis about where conditions are suitable.",
          },
        ],
      },
    },
  },

  evidence: {
    title: "Evidence levels",
    summary:
      "Insufficient, Exploratory, Indicative and Supported: exact criteria and what to do.",
    sections: {
      overview: {
        title: "Rule-based, no percentages",
        blocks: [
          {
            p: "Each species receives one of four **evidence levels**, computed by fixed rules from the data and the validation results (ADR-0009). There is no confidence percentage: the level and the reasons listed under it (✓ met, ✗ not met) are the complete statement.",
          },
        ],
      },
      levels: {
        title: "Levels and criteria",
        blocks: [
          {
            table: {
              caption: "Evidence levels and their criteria",
              head: ["Level", "Criteria (all must hold)", "Meaning"],
              rows: [
                [
                  "*Insufficient*",
                  "Fewer than 10 usable presence cells after cleaning",
                  "No model is fitted; nothing is mapped.",
                ],
                [
                  "*Exploratory*",
                  "A model was fitted, but at least one *Indicative* criterion fails",
                  "Useful to explore patterns; not robust enough on its own for decisions.",
                ],
                [
                  "*Indicative*",
                  "≥ 25 presence cells; mean continuous Boyce index ≥ 0.40; mean OR10 ≤ 0.25; extrapolated share of the study area ≤ 25 %",
                  "Reasonable support; use together with expert knowledge and field data.",
                ],
                [
                  "*Supported*",
                  "≥ 50 presence cells; mean Boyce ≥ 0.70; mean OR10 ≤ 0.15; extrapolated share ≤ 10 %; spatial block cross-validation",
                  "Data adequacy and validation meet the documented criteria.",
                ],
              ],
            },
          },
          {
            p: "Means are taken across the cross-validation folds of the selected model. If the Boyce index cannot be computed, the Boyce criterion counts as not met. “Presence cells” are grid cells with at least one retained record (one per cell). The extrapolated share is the part of the study area with MESS < 0.",
          },
          {
            p: "Above *Supported*, the *Validated* level needs independent presence–absence field data held out from training and strict criteria (ADR-0108, proposed): see [Your own field data](#field-data/validated). Without such data no level is higher than *Supported*.",
          },
        ],
      },
      independent: {
        title: "No independent validation",
        blocks: [
          {
            note: "Every result states: “No independent validation data: evidence comes from cross-validation of the same, possibly biased, records.” Even *Supported* does not mean the map was checked against field surveys.",
            tone: "warn",
          },
        ],
      },
      taxon: {
        title: "Genus or family selected",
        blocks: [
          {
            p: "If the selected GBIF taxon is above species rank (genus, family…), the records of several species with different niches are pooled. The data review and the results show a warning, and the level is capped at *Exploratory* whatever the metrics. Select the species (or subspecies) instead.",
          },
        ],
      },
      insufficient: {
        title: "What to do with an Insufficient species",
        blocks: [
          {
            ul: [
              "Check the exclusion reasons (Data review, or Data & exports): many records may have been removed by the licence, date or uncertainty filters. Relaxing a filter is possible but must be justified; it is recorded in the report and manifest, and including CC BY-NC records restricts the results to non-commercial use.",
              "Check that the study area does not cut off a large part of the known distribution.",
              "A coarser grid does not help: records are thinned to one per cell, so larger cells give fewer presence cells.",
              "If the species is simply too rarely recorded, report it as not modellable with the available data and rely on expert knowledge and field survey.",
              "An *Insufficient* species does not count in the monthly quota.",
            ],
          },
        ],
      },
      exploratory: {
        title: "What to do with an Exploratory species",
        blocks: [
          {
            ul: [
              "Read the ✗ reasons: they name the failed criterion (sample size, Boyce index, omission, extrapolation, taxon rank).",
              "Few presence cells: present the map as a hypothesis to test in the field.",
              "High OR10: the model may be overfit or transfer poorly between regions; look at the per-fold values on the Validation tab.",
              "High extrapolation: the study area contains conditions absent from the training data; keep the hatch visible or restrict the study area to the region of interest, for ecological reasons.",
              "Low Boyce index: the predictions do not rank held-out records well; look for data errors on the data review map and question the variable set.",
              "In a report: give the level and its reasons verbatim, do not derive decision-making areas from the map alone, and combine it with field data.",
            ],
          },
          {
            note: "Do not change settings only to reach a higher level. Any change must have an ecological justification; it is recorded in the manifest.",
            tone: "info",
          },
        ],
      },
    },
  },

  validation: {
    title: "Validation metrics",
    summary:
      "Spatial block cross-validation, continuous Boyce index, OR10, ORmtp and presence-background AUC.",
    sections: {
      cv: {
        title: "Spatial block cross-validation",
        blocks: [
          {
            p: "By default, presences are split at their median latitude, then each half at its median longitude, giving **4 spatial blocks** of similar size; background points are assigned with the same lines (Muscarella et al. 2014). Each block in turn is held out and predicted by a model trained on the other three. A model is thus tested on a region it has not seen, which reduces the optimism caused by spatial autocorrelation.",
          },
          {
            p: "Random k-fold is available in Model settings but is optimistic when records are spatially clustered (Roberts et al. 2017). A study validated with random folds cannot reach *Supported*.",
          },
          { p: "Metrics are shown as mean ± standard deviation across folds." },
        ],
      },
      grid: {
        title: "Spatial grid blocks sized by autocorrelation (Expert)",
        blocks: [
          {
            p: "An alternative to the 4 median blocks: the study area is covered by a grid of **square blocks**, and whole blocks are assigned to k folds (5 by default) so that each fold has a similar number of presences (blockCV, Valavi et al. 2019). Test records are then separated from training records by at least part of a block, which is what reduces the optimism of validation under spatial autocorrelation (Roberts et al. 2017).",
          },
          {
            p: "**Block size.** Leave it empty for the automatic choice: NicheRS fits a variogram to each predictor on the background points and uses the **median range** — the distance beyond which values are no longer correlated — clamped to a band that keeps enough blocks for cross-validation. You can also enter a size in km. The Validation tab shows the size used, how it was chosen, the range of each predictor and the number of folds actually obtained.",
          },
          {
            p: "If too few blocks contain presences, fewer folds than requested are possible; a warning says so. Both spatial methods count as spatially blocked validation for the evidence level.",
          },
        ],
      },
      nulls: {
        title: "Null models: better than random-location models? (Expert)",
        blocks: [
          {
            p: "Optional test (Raes & ter Steege 2007; Bohl et al. 2019; ENMeval 2.0, Kass et al. 2021). NicheRS draws N sets of **random locations** in the study area, as many as the species' presences, and fits each with exactly the selected setting and the same cross-validation. The held-out Boyce index and OR10 of the real model are then compared with those of the random-location models.",
          },
          {
            p: "**p-value**: the share of random-location models that do at least as well as the real model, counted as (1 + number as good) / (N + 1). p ≤ 0.05 for the Boyce index means the model is **better than random-location models**. With N = 20, the smallest possible p is 1/21 ≈ 0.048 (at least 19 are needed for p to reach 0.05, so fewer are not accepted); use 50–100 for a firmer test.",
          },
          {
            p: "With a [target-group background](#data/targetgroup), the random locations are drawn in proportion to the target-group records per cell (**bias-matched** null models), so the comparison is not inflated by observer bias.",
          },
          {
            note: "A significant result says the model beats chance, not that it is ecologically right. If it is not better than random-location models, the evidence level is capped at *Exploratory*. It is not a confidence level. Cost: N × folds extra model fits (e.g. 100 × 4 = 400), which can multiply the run time several-fold.",
            tone: "warn",
          },
        ],
      },
      boyce: {
        title: "Continuous Boyce index (CBI)",
        blocks: [
          {
            p: "From −1 to 1 (Hirzel et al. 2006). It asks whether held-out presences fall in high-suitability cells more often than expected from the background. Above 0: better than random; close to 1: predictions consistent with where the species was recorded; 0 or below: no better than random.",
          },
          {
            p: "Computed with the ecospat defaults (101 moving windows). It is not computed for a fold with too few test presences (shown as —).",
          },
        ],
      },
      or10: {
        title: "OR10 — omission at the 10th percentile",
        blocks: [
          {
            p: "The share of held-out presences that fall below the threshold excluding 10 % of the training presences. Expected ≈ 10 % for a model that is not overfit (Radosavljevic & Anderson 2014); values well above indicate overfitting or poor transfer to new areas. It is the first model-selection criterion.",
          },
        ],
      },
      ormtp: {
        title: "ORmtp — omission at the minimum training presence",
        blocks: [
          {
            p: "The share of held-out presences below the lowest suitability of any training presence. Expected 0 %; higher values mean held-out records are in conditions the model rates lower than all training records.",
          },
        ],
      },
      auc: {
        title: "Presence-background AUC and its limits",
        blocks: [
          {
            p: "Reported as a secondary metric only. It compares presences with background points, not with known absences: its maximum is below 1 and it largely reflects how large and environmentally varied the study area is (Lobo et al. 2008; Jiménez-Valverde 2012). A larger study area inflates it.",
          },
          {
            note: "Compare AUC only between models of the same species in the same study area. Never compare it across species or areas, never call it “accuracy”. It is not used to select the model.",
            tone: "warn",
          },
        ],
      },
      selection: {
        title: "Model tuning and selection",
        blocks: [
          {
            p: "With automatic tuning, NicheRS evaluates **25 settings**: feature classes L, LQ, H, LQH, LQHP (L linear, Q quadratic, H hinge, P product) × regularization multiplier β 0.5, 1, 2, 3, 4 (higher β gives smoother responses). It selects, in order: the lowest mean OR10, then the highest mean Boyce index, then the fewest coefficients (Radosavljevic & Anderson 2014; Kass et al. 2021). The final model is refitted on all presences and background points with the selected setting.",
          },
          {
            p: "This rule favours smoother, more transferable models over apparent discrimination. The full table is under Validation › Tuning.",
          },
        ],
      },
    },
  },

  area: {
    title: "Suitable-area band",
    summary:
      "MTP, P10 and maximum SSS thresholds, the area band and exact WGS84 cell areas.",
    sections: {
      band: {
        title: "Area of suitable conditions",
        blocks: [
          {
            p: "The headline **area of suitable conditions** is the summed area of the cells whose suitability is at or above a threshold. Because the threshold is a choice, NicheRS always shows the **band** (minimum – maximum) across three rules next to the headline value, with the share of the study area. Changing the rule in Summary also updates the “Suitable / not suitable at threshold” map layer.",
          },
        ],
      },
      rules: {
        title: "Threshold rules",
        blocks: [
          {
            table: {
              caption: "Threshold rules",
              head: ["Rule", "Definition", "Behaviour"],
              rows: [
                [
                  "**P10** — 10th-percentile training presence (default)",
                  "Suitability below which 10 % of the training presences fall",
                  "Tolerates the ~10 % of records that are erroneous or in marginal habitat in opportunistic data while remaining conservative.",
                ],
                [
                  "**MTP** — minimum training presence",
                  "Lowest suitability of any training presence",
                  "Maximally inclusive; a single outlier record can enlarge the area a lot.",
                ],
                [
                  "**Max SSS** — maximum sensitivity + specificity",
                  "Maximises sensitivity + specificity, presences against background (Liu et al. 2013)",
                  "Uses background points as pseudo-absences.",
                ],
              ],
            },
          },
          {
            p: "Thresholds are computed from the final model's predictions at the training presence cells (and at the background points for max SSS).",
          },
        ],
      },
      cells: {
        title: "Exact WGS84 cell areas",
        blocks: [
          {
            p: "The model works on the native CHELSA geographic grid (30 arc-seconds, about 0.93 × 0.6 km at 50° N), without projection or resampling. Each cell's area is computed exactly as a quadrangle on the WGS84 ellipsoid (Snyder 1987), and areas are summed. Cells shrink towards the poles, so a count of cells is not an area.",
          },
          {
            p: "The study-area value is the land area with predictor data: sea and large lakes are excluded.",
          },
        ],
      },
      wording: {
        title: "How to word it",
        blocks: [
          {
            note: "Write: “area of suitable conditions (P10 rule): X km², band Y–Z km² across threshold rules”. Never “range”, “habitat area” or “occupied area”.",
            tone: "info",
          },
        ],
      },
    },
  },

  uncertainty: {
    title: "Uncertainty & extrapolation",
    summary:
      "Spread between fold models (SD) and environmental novelty (MESS).",
    sections: {
      foldsd: {
        title: "Uncertainty: spread between fold models",
        blocks: [
          {
            p: "For each cell, the **Uncertainty** layer shows the standard deviation of suitability across the cross-validation models of the selected setting (4 models with spatial blocks). It measures how much the prediction depends on which records were used for training. The legend runs from 0 to ≥ 0.25; Summary shows the mean over the study area.",
          },
          {
            p: "It is not a confidence interval and does not include every source of error (record errors, missing variables, sampling bias).",
          },
        ],
      },
      mess: {
        title: "Extrapolation (MESS)",
        blocks: [
          {
            p: "The **Extrapolation** layer is the multivariate environmental similarity surface (MESS, Elith et al. 2010), computed against the environments of the background and presence cells. MESS < 0 means that at least one variable is outside the range seen in training: the prediction there is an extrapolation, driven by the model's assumptions rather than by data.",
          },
          {
            p: "These cells are **hatched on the map by default**; keep the hatch in published maps. The share of the study area with MESS < 0 enters the evidence level (≤ 25 % for *Indicative*, ≤ 10 % for *Supported*), and a warning is shown above 10 %.",
          },
        ],
      },
      aoa: {
        title: "Area of applicability (AOA, Expert)",
        blocks: [
          {
            p: "Optional layer (Meyer & Pebesma 2021, R package CAST). For each cell, the **dissimilarity index (DI)** is the distance, in predictor space, to the nearest training point (presence or background), divided by the average distance between training points. Predictors are standardised and weighted by their permutation importance in the final model, so differences in important variables count more.",
          },
          {
            p: "The **threshold** is derived from the training data under the same cross-validation folds (the largest DI of training points that is not an outlier). Cells above it are **outside the area of applicability** and are hatched in purple when the layer is switched on. A warning is shown when more than 10 % of the study area is outside.",
          },
          {
            p: "**Difference from MESS.** MESS flags values outside the range of each variable taken one at a time; the AOA also flags new *combinations* of values inside those ranges, and weighs variables by importance. The two can disagree; read both.",
          },
          {
            note: "The AOA does not change the evidence level: MESS remains the extrapolation criterion (ADR-0009). Being inside the AOA does not mean the prediction is correct.",
            tone: "info",
          },
        ],
      },
      together: {
        title: "Reading them together",
        blocks: [
          {
            ul: [
              "High suitability, low spread, no hatch: the most robust part of the map.",
              "High spread: the prediction changes with the training subset; treat with caution.",
              "Hatched: outside training conditions; do not interpret the score there, whatever its value.",
              "Neither layer shows sampling bias: with opportunistic records, well-surveyed areas tend to look more suitable (no bias correction by default; see [Sampling bias](#data/bias)).",
            ],
          },
        ],
      },
    },
  },

  data: {
    title: "Data cleaning & exclusion reasons",
    summary:
      "GBIF records, cleaning rules in order, licences, and how to cite the GBIF download.",
    sections: {
      source: {
        title: "Where the records come from",
        blocks: [
          {
            p: "Occurrences come from GBIF (gbif.org), resolved through the GBIF taxonomic backbone. When the server has a GBIF account (the hosted service uses Natural Solutions' account), records come from a **GBIF occurrence download with a DOI**.",
          },
          {
            p: "Otherwise the GBIF search API is used, capped at 20 000 records per species by default; above the cap a spatially stratified subsample is taken and the data review says so. For a final report, use a study with a DOI download.",
          },
          {
            p: "Background: 10 000 random land cells within the study area, drawn with the study's seed. No sampling-bias correction is applied by default; Expert mode offers a [target-group background](#data/targetgroup).",
          },
        ],
      },
      rules: {
        title: "Cleaning rules, in order",
        blocks: [
          {
            table: {
              caption: "Occurrence cleaning rules",
              head: ["#", "Exclusion reason shown", "Default", "Why"],
              rows: [
                [
                  "1",
                  "Fossil or captive/cultivated specimen",
                  "Excluded",
                  "Not wild populations in their environmental niche.",
                ],
                [
                  "2",
                  "Licence not allowed for this project",
                  "CC0 and CC BY 4.0 only",
                  "CC BY-NC is non-commercial (see below).",
                ],
                [
                  "3",
                  "Undated record",
                  "Excluded",
                  "Cannot be matched to the climate baseline.",
                ],
                [
                  "4",
                  "Recorded before 1981 (climate baseline)",
                  "Year ≥ 1981",
                  "CHELSA climatology covers 1981–2010.",
                ],
                [
                  "5",
                  "Coordinates generalised by the publisher; precision not stated or > 1000 m",
                  "Excluded; generalised records with stated precision ≤ 1000 m are kept",
                  "Blurred position of unknown precision. See [Generalised coordinates](#data/generalised).",
                ],
                [
                  "6",
                  "Coordinate uncertainty > 1000 m",
                  "≤ 1000 m; records without the field are kept",
                  "Larger than the ~1 km cell.",
                ],
                ["7", "Outside study area", "Excluded", "—"],
                [
                  "8",
                  "In sea or large lake (land mask)",
                  "Excluded",
                  "Terrestrial niche; climate data exist over the sea.",
                ],
                [
                  "9",
                  "Missing environmental data",
                  "Excluded",
                  "No predictor value in the cell.",
                ],
                [
                  "10",
                  "Duplicate within one grid cell",
                  "One record per cell",
                  "Reduces pseudo-replication.",
                ],
                [
                  "11",
                  "Thinned (closer than X km to a retained record)",
                  "Off (Expert option)",
                  "Reduces clusters. See [Distance thinning](#data/thinning).",
                ],
              ],
            },
          },
          {
            p: "Each excluded record is counted under the **first** rule it fails, so the counts add up. The cleaning steps cannot be switched off; the year, uncertainty and licence filters can be changed in Data review and are recorded in the report and manifest.",
          },
          {
            p: "The land mask (Natural Earth 1:10m) has a coastline accuracy of about 0.5–1 km: some coastal records may be excluded wrongly. They are listed with their reason; review them for coastal species. Marine species are out of scope.",
          },
        ],
      },
      generalised: {
        title: "Generalised (blurred) coordinates",
        blocks: [
          {
            p: "**Publishers blur the coordinates of sensitive species. NicheRS does not use blurred points whose precision is unknown.**",
          },
          {
            p: "Some data publishers *generalise* coordinates on purpose — rounded to a 10 km square or moved to a municipality centroid, for example — to protect sensitive species (nesting raptors, orchids, bat roosts) or at the observer's request. GBIF reports it in the Darwin Core fields `dataGeneralizations` and `informationWithheld`.",
          },
          {
            p: "A generalised record is kept only when its stated coordinate uncertainty is within the uncertainty limit (1000 m by default). When the precision is not stated, or is larger, the record is excluded as **Coordinates generalised by the publisher** (rule 5, just before the uncertainty rule) and drawn on the map as a dashed magenta ring. The Data & exports tab gives the counts: generalised, excluded, retained.",
          },
          {
            p: "In France, the SINP (*Système d'information de l'inventaire du patrimoine naturel*) defines, region by region, which species have sensitive locations (*référentiels régionaux de sensibilité*); their records are often published at a coarse resolution. The retained presences can then under-represent the most sensitive sites. The warning `generalised_records` gives the numbers; it becomes a Warning when at least 10 % of the records in the area are generalised.",
          },
          {
            note: "The GBIF search API reports generalisation; GBIF downloads (SIMPLE_CSV format) and the regional GBIF mirror do not. Studies using them show the note *generalisation flags unavailable*: generalised records with a small or missing uncertainty cannot be detected, so check sensitive species against the regional référentiel yourself.",
            tone: "warn",
          },
          {
            p: "`occurrences.geojson` keeps the coordinates as published by GBIF. Some species are sensitive under the SINP rules: check the regional référentiel before reproducing points in a public report.",
          },
        ],
      },
      thinning: {
        title: "Distance thinning (Expert)",
        blocks: [
          {
            p: "Optional, off by default, set in the Model step (Expert mode). After cleaning, retained presence cells are thinned so that no two remaining cells are closer than the chosen distance (more than 0 and at most 100 km). It reduces clusters of records from intensely surveyed places (towns, reserves, well-known sites), which would otherwise weigh heavily in the model, and spatial autocorrelation between presences.",
          },
          {
            p: "The method follows spThin (Aiello-Lammens et al. 2015): cells are removed in random order, the draw is repeated several times and the largest set of cells is kept. The random draws come from the study seed, so the data preview, the run and any rerun of the same study give the same result. Removed cells are counted as **Thinned (closer than X km to a retained record)**, after the per-cell duplicate rule, and drawn on the map as small grey dots.",
          },
          {
            note: "Thinning costs data: every removed cell is a real presence. With few presences it can push a species towards *Exploratory* or below the minimum. Choose a distance that matches the clustering you want to remove (for example a few km around recording hotspots), no larger than needed.",
            tone: "warn",
          },
          {
            p: "The distance is recorded in the specification (`filters.thin_km`), the report and the manifest.",
          },
        ],
      },
      bias: {
        title: "Sampling bias: records show where people look",
        blocks: [
          {
            p: "GBIF records are mostly opportunistic: they come from where people go — near towns, roads, paths, reserves and famous sites — as much as from where the species lives. A model fitted to them learns the habitat **and** the observers' habits: well-surveyed conditions look more suitable than they are, and remote areas less.",
          },
          {
            p: "By default NicheRS does not correct this bias and says so in the warning `sampling_bias`. Read the map with it in mind: a low score in a little-surveyed area may only mean nobody looked there.",
          },
          {
            ul: [
              "**Target-group background** (Expert): draws the background where similar organisms were recorded, so presences and background share the bias. See [Target-group background](#data/targetgroup).",
              "**Distance thinning** (Expert): removes clustered presences. See [Distance thinning](#data/thinning).",
              "Spatial block cross-validation does not remove bias: test folds come from the same biased records.",
            ],
          },
        ],
      },
      targetgroup: {
        title: "Target-group background (Expert)",
        blocks: [
          {
            p: "Instead of random cells, the background is drawn among the cells of the study area that have at least one GBIF record of a **target group** — organisms surveyed in the same way as the species, by default its own GBIF class (Aves for a bird, Insecta for a beetle, Magnoliopsida for a flowering plant). Presences and background then share the observers' bias, which largely cancels out in the model (Phillips et al. 2009). The cells are drawn uniformly, one per cell, with the study seed.",
          },
          {
            p: "It needs the **sampling-effort grids of the regional GBIF mirror** (records of each class per cell). When no mirror covers the area, the mirror has no grids yet, the class has no records, or too few cells have records, the background falls back to random and the warning `target_group_unavailable` gives the reason: no bias correction was applied. When fewer cells than requested have records, the background is smaller (note `target_group_background_smaller`).",
          },
          {
            p: "**Effort map.** Such studies get a *Sampling effort* map layer: target-group records per cell, orange on a log scale (1 record → the maximum in one cell; cells without records are transparent). Counts are records, not visits or observers. In the demo the effort surface is synthetic and labelled so.",
          },
          {
            p: "**Diagnostic.** The Data & exports tab reports the share of presence cells that lie in the most-surveyed cells covering ≈ 10 % of the study area. Without bias about 10 % would be expected; a much larger share means the presences follow observer effort. It is descriptive only and does not change the evidence level.",
          },
          {
            p: "**Bias-matched null models.** When null models are requested, their random locations are drawn in proportion to the target-group records per cell instead of uniformly, so the model is compared with random-location models that carry the same bias.",
          },
          {
            note: "It corrects only the part of the bias the target group shares with the species. Species surveyed differently from their class (a rare bird searched for by specialists, a species recorded mostly by one scheme) keep their own bias, and a target group with very few records gives a poor background.",
            tone: "warn",
          },
        ],
      },
      licences: {
        title: "Licences: why CC BY-NC is excluded",
        blocks: [
          {
            p: "By default only **CC0** and **CC BY 4.0** records are used. **CC BY-NC** records are excluded because they cannot be used in commercial consultancy deliverables without a legal reading of the licence (ADR-0006).",
          },
          {
            note: "You may include CC BY-NC records, but the results are then restricted to non-commercial use: do not put them in a paid deliverable.",
            tone: "warn",
          },
          {
            p: "CC BY records require attribution: cite the GBIF download (below).",
          },
        ],
      },
      citation: {
        title: "Citing the GBIF download (DOI)",
        blocks: [
          {
            p: "The DOI is given on the Data & exports tab (source), in `report.md` and in the manifest (`occurrence_source.citation`). Cite it in the form recommended by GBIF: *GBIF.org (date) GBIF Occurrence Download https://doi.org/10.15468/dl.xxxxxx*.",
          },
          {
            p: "A DOI download is the permanent, citable record of exactly which records were used. A study that used the search API has no DOI.",
          },
        ],
      },
      review: {
        title: "What to check in Data review",
        blocks: [
          {
            ul: [
              "Records in implausible places: gardens, museums, swapped coordinates, country or region centroids.",
              "A large share excluded by one rule deserves a look.",
              "Clusters around towns, reserves or roads: sampling bias, not corrected by default (see [Sampling bias](#data/bias)).",
              "Generalised or thinned records (dashed magenta rings, grey dots) and their counts in the exclusions table.",
              "Warnings: taxon above species rank, truncated or subsampled download.",
              "Grid resolution: ×1 is about 1 km; coarser grids give fewer presence cells because of the one-per-cell rule.",
            ],
          },
        ],
      },
    },
  },

  variables: {
    title: "Variables & collinearity",
    summary:
      "CHELSA v2.1 climate 1981–2010, the default set and the |r| > 0.7 filter.",
    sections: {
      chelsa: {
        title: "CHELSA v2.1 climate",
        blocks: [
          {
            p: "Predictors are the 19 **CHELSA v2.1** bioclimatic variables, 1981–2010 climatology, at 30 arc-seconds (about 1 km), licence CC0 (Karger et al. 2017).",
          },
          {
            note: "They describe **climate only**. Terrain, land cover and distance to water can be added but are not selected by default ([Terrain, land cover & water](#predictors)); soils are not offered. Cells with suitable climate may have no suitable habitat. Future climate projections are an Expert option ([Future climate projections](#future)).",
            tone: "warn",
          },
          {
            p: "WorldClim is not used because its licence does not allow commercial use.",
          },
        ],
      },
      defaults: {
        title: "Default variable set",
        blocks: [
          {
            p: "Pre-selected: BIO1 (annual mean temperature), BIO4 (temperature seasonality), BIO5 (maximum temperature of the warmest month), BIO6 (minimum temperature of the coldest month), BIO12 (annual precipitation), BIO14 (precipitation of the driest month), BIO15 (precipitation seasonality): energy, water, their seasonality and extremes.",
          },
          {
            p: "BIO8, BIO9, BIO18 and BIO19 are flagged and not selected by default: they combine temperature and precipitation of specific quarters and show spatial artefacts (Booth 2022).",
          },
        ],
      },
      collinearity: {
        title: "Collinearity filter",
        blocks: [
          {
            p: "Correlated predictors make response shapes and contributions unstable. Before fitting, NicheRS computes Pearson correlations on the background sample and keeps variables **in your priority order**, dropping any variable with |r| > 0.7 with a variable already kept (Dormann et al. 2013). Dropped variables and their correlation partner are listed in the results and in the report. The threshold can be changed in Model settings.",
          },
        ],
      },
      contributions: {
        title: "Variable contributions",
        blocks: [
          {
            p: "Percent contribution is MaxEnt's heuristic estimate of how much each variable contributed to the fit. It depends on the optimisation path and on correlated variables; it is not a measure of causal importance.",
          },
          {
            p: "Permutation importance (second bar) is MaxEnt's other estimate: each variable's values are randomly permuted among presences and background, and the drop in training AUC is normalised to 100. It depends only on the final model, not on the optimisation path. When the two estimates disagree, the variable's role is uncertain; both share credit between correlated variables.",
          },
          {
            p: "Response curves show relative suitability (cloglog) as one variable varies, with every other variable held at its mean over the presences. Shaded parts lie outside the training data: there the model is clamped and the curve is an extrapolation. They are marginal curves; with correlated variables they can hide effects of variables changing together. Full curves (501 points, as `maxent.jar`) are in `response_curves.csv`.",
          },
        ],
      },
    },
  },

  predictors: {
    title: "Terrain, land cover & water",
    summary:
      "Optional predictors from Copernicus DEM and ESA WorldCover 2021: what they measure, when to use them, their limits and sources.",
    sections: {
      overview: {
        title: "What they add",
        blocks: [
          {
            p: "Climate explains the broad distribution of a species; at a 1 km grid many species also respond to relief and to the composition of the landscape. Besides the 19 CHELSA climate variables, NicheRS offers 12 optional predictors: three terrain variables, eight land-cover fractions and the distance to permanent water. They lie on the same 30″ grid as CHELSA.",
          },
          {
            note: "None is selected by default: the recommended set stays climate only (ADR-0009). These predictors are proposed in ADR-0015, pending review by Natural Solutions ecologists.",
            tone: "info",
          },
          {
            p: "They go through the same pipeline as climate: the collinearity filter keeps variables **in the order you selected them** (select climate first so that a land-cover variable is kept only if it adds information), then MESS, response curves and contributions.",
          },
          {
            p: "Choose a few predictors that matter for the species' ecology. Every added variable can only increase the share of the study area outside the training conditions (MESS takes the minimum over variables), which can lower the evidence level. That is honest, not a fault.",
          },
        ],
      },
      terrain: {
        title: "Terrain (Copernicus DEM)",
        blocks: [
          {
            table: {
              caption: "Terrain predictors",
              head: ["Variable", "Unit", "Value per 1 km cell"],
              rows: [
                [
                  "Elevation (ELEV)",
                  "m",
                  "Mean of the 90 m heights in the cell.",
                ],
                [
                  "Slope (SLOPE)",
                  "degrees",
                  "Mean of the slopes computed at 90 m (Horn 1981) **before** aggregation, so relief is not flattened.",
                ],
                [
                  "Roughness (ROUGH)",
                  "m",
                  "Standard deviation of the 90 m heights in the cell.",
                ],
              ],
            },
          },
          {
            ul: [
              "**When to use**: slope and roughness for species tied to steep or rocky ground, or avoiding it (cliff-nesting birds, rock-dwelling reptiles, mountain ungulates).",
              "Elevation largely repeats temperature (BIO1): it is often dropped by the collinearity filter and rarely adds information.",
            ],
          },
          {
            note: "Copernicus DEM is a **surface** model (TanDEM-X, 2011–2015): heights include forest canopy and buildings, so wooded and urban cells look higher and rougher than the ground.",
            tone: "warn",
          },
        ],
      },
      landcover: {
        title: "Land cover (ESA WorldCover 2021)",
        blocks: [
          {
            p: "Each land-cover variable is the **fraction** (0 to 1) of the 10 m pixels of the cell that belong to a group of WorldCover classes; a 1 km cell holds several thousand pixels, so mosaics and edges are kept instead of a single dominant class.",
          },
          {
            table: {
              caption: "Land-cover groups",
              head: ["Variable", "WorldCover classes"],
              rows: [
                ["Tree cover (TREE)", "10 tree cover"],
                ["Shrubland (SHRUB)", "20 shrubland"],
                ["Grassland (GRASS)", "30 grassland"],
                ["Cropland (CROP)", "40 cropland"],
                ["Built-up (BUILT)", "50 built-up"],
                [
                  "Bare and sparse (BARE)",
                  "60 bare/sparse vegetation, 70 snow and ice, 100 moss and lichen",
                ],
                ["Permanent water (WATER)", "80 permanent water bodies"],
                ["Wetland (WETL)", "90 herbaceous wetland, 95 mangroves"],
              ],
            },
          },
          {
            note: "The eight fractions add up to 1 in every cell. Choose the groups relevant to the species (for example tree cover for a forest species, grassland and cropland for a farmland bird), not all eight: with all eight one is redundant, and the pairwise correlation filter does not detect it. Results then carry a warning.",
            tone: "warn",
          },
          {
            ul: [
              "**Temporal mismatch**: land cover is that of 2021, the climate baseline is 1981–2010 and records may date from 1981. Where land use changed, older records are described by today's land cover.",
              "**Built-up and sampling bias**: built-up area is strongly correlated with where people observe. A positive response to BUILT may reflect observers, not the species; the results warn when it is used.",
              "WorldCover does not separate broadleaved from coniferous forest, nor annual from permanent crops (vineyards, orchards); its overall accuracy is about 77 %.",
              "At 0 % a class absent from all training cells is flagged by MESS as outside the training range even when this is ecologically unremarkable; read the MESS map with that in mind.",
            ],
          },
        ],
      },
      water: {
        title: "Distance to water",
        blocks: [
          {
            p: "**Distance to water (DWAT, km)**: distance from the cell centre to the nearest cell that is at least half permanent water in WorldCover 2021, **truncated at 25 km**. Values up to 25 km are exact and do not depend on the extent of the study area; farther cells get 25 km.",
          },
          {
            ul: [
              "**When to use**: species tied to lakes, reservoirs, large rivers or the coast (waterbirds, some amphibians and dragonflies).",
              "Only water covering at least half of a 1 km cell is seen: lakes, reservoirs, wide rivers, lagoons and the sea. Streams and ponds are not resolved, so it is not suitable for species that depend on small watercourses.",
              "Same temporal mismatch as land cover (2021).",
            ],
          },
        ],
      },
      aggregation: {
        title: "How values are computed",
        blocks: [
          {
            ul: [
              "Nothing is resampled: each source pixel is assigned to the CHELSA cell that contains its centre, so all layers stack exactly.",
              "A cell is NoData when fewer than half of its source pixels are valid. On coarser grids (×2, ×3…) the 1 km values are averaged, like climate.",
              "A cell is modelled only where **every** selected predictor has a value. Along coasts the DEM can be missing where whole sea tiles are absent; records there are excluded as “Missing environmental data”.",
              "A first study in a new region downloads and aggregates the source tiles (measured: about 4–5 s and 8 MB per degree for WorldCover, about 1 s per degree for the DEM); later studies read the instance cache in milliseconds.",
            ],
          },
        ],
      },
      sources: {
        title: "Sources, licences and attribution",
        blocks: [
          {
            table: {
              caption: "Sources of the additional predictors",
              head: [
                "Source",
                "Licence",
                "Attribution (reproduced in reports)",
              ],
              rows: [
                [
                  "Copernicus DEM GLO-90 (ESA, doi:10.5270/ESA-c5d3d65)",
                  "Copernicus WorldDEM-90 licence: free, worldwide; reproduction, distribution and adaptation allowed",
                  "produced using Copernicus WorldDEM-90 © DLR e.V. 2010-2014 and © Airbus Defence and Space GmbH 2014-2018 provided under COPERNICUS by the European Union and ESA; all rights reserved",
                ],
                [
                  "ESA WorldCover 10 m 2021 v200 (Zanaga et al. 2022, doi:10.5281/zenodo.7254221)",
                  "CC BY 4.0",
                  "© ESA WorldCover project 2021 / Contains modified Copernicus Sentinel data (2021) processed by ESA WorldCover consortium",
                ],
              ],
            },
          },
          {
            p: "Both allow commercial use with attribution (checked 2026-10-20; the DEM licence grants the rights without naming commercial use, and a legal confirmation is advisable). Reports and the manifest list every source used with its licence and attribution; keep the attribution when you reuse a map.",
          },
        ],
      },
    },
  },

  algorithms: {
    title: "Modelling algorithms",
    summary:
      "MaxEnt (default), the down-sampled random forest and the MaxEnt + random forest ensemble: what they are, when to use them and how to read their results.",
    sections: {
      overview: {
        title: "MaxEnt remains the default",
        blocks: [
          {
            p: "Every study uses **MaxEnt** unless you choose otherwise in Expert mode (model step, *Algorithm*). MaxEnt is the default because its NicheRS port is tested against `maxent.jar` 3.4.4 on reference cases, its behaviour with presence–background data is well documented, and its smooth response curves are easy to discuss with a client (ADR-0002, ADR-0009).",
          },
          {
            p: "Two opt-in alternatives exist: a **down-sampled random forest** and an **ensemble** of MaxEnt and that forest. Both are proposed in ADR-0017, pending review by Natural Solutions ecologists.",
          },
          {
            note: "Every algorithm produces **relative habitat suitability**, never a probability of presence: presence–background data cannot estimate how common the species is.",
            tone: "warn",
          },
        ],
      },
      rf: {
        title: "Down-sampled random forest",
        blocks: [
          {
            p: "A random forest (Breiman 2001) is a set of classification trees, each grown on a random sample of the data, that vote “presence” or “background”. With presence–background data a default forest fails: there are far more background points than presences, so most trees vote background almost everywhere.",
          },
          {
            p: "The **down-sampled** forest fixes this (Valavi et al. 2021): each tree is grown on a bootstrap of the presences and **as many** background points, drawn with replacement. Valavi et al. 2022 found it in the top group of presence-only methods on 225 species with independent test data, while the default forest was among the worst.",
          },
          {
            ul: [
              "Settings: 500 trees by default (100–2 000), fully grown, `mtry` = √p variables tried at each split (optionally also p/2, chosen by the same validation rule as MaxEnt).",
              "Output: the **share of trees voting presence**, from 0 to 1. It is a relative suitability index: the 1:1 balance of each tree fixes its apparent prevalence.",
              "Thresholds use the forest's **out-of-bag** scores at the training presences (each presence scored only by the trees that did not see it), because fully grown trees score their own training presences at almost 1.",
              "Same folds, metrics, selection rule, null models, MESS, area of applicability and evidence rules as MaxEnt, so both engines are judged identically.",
            ],
          },
        ],
      },
      ensemble: {
        title: "Ensemble of MaxEnt and the random forest",
        blocks: [
          {
            p: "The ensemble tunes each member as it would be tuned alone, then averages them. Because a cloglog score and a vote share are on different scales, each member's value is first converted to its **percentile rank** among its own values at the background points; the ensemble is the mean of the two ranks.",
          },
          {
            p: "An ensemble value of 0.8 therefore means that, on average, the two models rank the cell above 80 % of the study area's background. Values are ranks within this study area: do not compare them between studies.",
          },
          {
            p: "The ensemble is evaluated as a model of its own: in each fold, the two members' fold models are combined and tested on the held-out data. Its metrics drive the evidence level; the Validation tab also lists each member's metrics on the same folds. Thresholds are computed on the ensemble values with the same rules (P10, maximum sensitivity + specificity, minimum training presence).",
          },
        ],
      },
      choose: {
        title: "Which one to use",
        blocks: [
          {
            ul: [
              "**MaxEnt** for a standard study: documented, reproducible against the reference software, smooth curves.",
              "**Ensemble** to check that conclusions do not depend on one algorithm. Valavi et al. 2022 found the ensemble of tuned models the best method on average; when the two members disagree strongly (see their metrics in the Validation tab), say so in the report.",
              "**Random forest alone** mostly as a comparison. Trees cannot extrapolate: outside the training conditions the forest predicts flat values, so under spatial blocks it can score lower than MaxEnt (on the synthetic demo, Boyce 0.22 vs 0.57 with 2 000 background points, but on par with random folds).",
            ],
          },
          {
            note: "Cost: each forest predicts about one million cells in 5 s (500 trees, 8 threads), and the fold models are predicted too for the uncertainty map: about 25 s per species for metropolitan France at 1 km with 4 blocks. The ensemble costs MaxEnt plus the forest; null models refit the selected setting, forests included.",
            tone: "info",
          },
          {
            p: "Results are averages over many species, not guarantees for yours: if MaxEnt and the forest give different maps, neither is “right” by default.",
          },
        ],
      },
      reading: {
        title: "Reading random-forest results",
        blocks: [
          {
            ul: [
              "**Variable importance** (Variables tab): permutation importance, the drop in training AUC when a variable's values are shuffled (3 repeats), normalised to 100. It shows how much the forest relies on a variable, not its ecological effect; correlated variables share credit.",
              "**Partial dependence** (instead of response curves): the mean suitability as one variable varies, averaged over the observed values of the others (Friedman 2001). MaxEnt response curves hold the other variables at their mean, so the two kinds of curve are not directly comparable. Forest curves are step-shaped and flat beyond the training range.",
              "**Files**: `model.rf.json` holds the forest (format, settings, seed, trees); `partial_dependence.csv` the curves. With the ensemble, `model.lambdas` is the MaxEnt member.",
            ],
          },
        ],
      },
    },
  },
  "field-data": {
    title: "Your own field data",
    summary:
      "Import your organisation's observations (CSV), use them with or instead of GBIF, or hold them out to validate the model on independent presence–absence data.",
    sections: {
      overview: {
        title: "What it is for",
        blocks: [
          {
            p: "Many consultancies hold their own surveys: inventories, transects, protocol plots, often with places where the species was looked for and **not found**. NicheRS can use them in two ways (ADR-0108):",
          },
          {
            ul: [
              "**As presences**, together with GBIF or instead of it: your presence records go through exactly the same cleaning rules as GBIF records.",
              "**Held out for validation**: the model is fitted on GBIF records only, then evaluated on your surveyed sites (presences and absences). This is the only independent check of a map, and the only way to the *Validated* level.",
            ],
          },
          {
            note: "These options are proposed in ADR-0108, pending review by Natural Solutions ecologists. A study without your data is computed exactly as before.",
            tone: "info",
          },
        ],
      },
      import: {
        title: "Importing a file",
        blocks: [
          {
            p: "In the **Data** step (Expert path), open *The organisation's field data* and import a CSV file (UTF-8 or Windows-1252, separator `;`, `,` or tab). Excel: “Save as CSV”.",
          },
          {
            ol: [
              "Choose the file: it is read **in your browser**; an extract is shown and nothing is sent yet.",
              "Tell which column holds the species name, X and Y, and optionally the date or year, the coordinate precision in metres, the presence/absence status and the protocol or survey.",
              "Choose the coordinate system (next section) and, for a status column, tick the values meaning “surveyed, not found”.",
              "Import: only the chosen columns are sent. The import report gives rows kept and rejected, with the reason and the file lines of the first rejected rows.",
            ],
          },
          {
            p: "Rejected rows are problems of the file (no species name, coordinates not readable, outside the chosen system, unreadable date or precision); they are separate from the scientific cleaning rules applied later. An empty date is not a rejection: the record is “undated” for the cleaning rules.",
          },
          {
            note: "A dataset cannot be edited: to change the mapping, import the file again. Deleting a dataset does not affect studies already launched: each keeps its own copy, deleted with the study.",
            tone: "info",
          },
        ],
      },
      crs: {
        title: "Coordinate systems",
        blocks: [
          {
            p: "Coordinates are converted to WGS 84 longitude/latitude on the server, with formulas checked against PROJ. The list is closed: WGS 84 degrees (EPSG:4326), Lambert-93 (EPSG:2154, most French data), LAEA Europe (EPSG:3035) and UTM zones 28N–32N (ETRS89 and WGS 84).",
          },
          {
            p: "RGF93 and ETRS89 are treated as WGS 84 (less than 1 m apart, far below the ~1 km grid). Coordinates outside the plausible range of the chosen system are rejected with “outside the chosen system”: usually the wrong system was chosen, or longitude and latitude are swapped.",
          },
          {
            note: "Lambert II étendu (EPSG:27572) and overseas systems are not offered yet: convert such files to Lambert-93 or WGS 84 first.",
            tone: "warn",
          },
        ],
      },
      matching: {
        title: "Matching species names",
        blocks: [
          {
            p: "Every model is fitted for a GBIF backbone taxon. The distinct names of your species column are matched with the GBIF name matcher: **only the names are sent to GBIF**, never coordinates or counts.",
          },
          {
            p: "An exact match at species rank or below is proposed automatically; approximate matches must be checked; a match at a higher rank (genus…) is not modelled. You decide: assign each name to a species of the study (or add the matched species), and leave unwanted names unused. Several names can go to one species (synonyms, subspecies).",
          },
        ],
      },
      sources: {
        title: "Sources per species",
        blocks: [
          {
            table: {
              caption: "How a species can use the organisation's data",
              head: ["Choice", "Training data", "Your absences"],
              rows: [
                ["GBIF only", "GBIF records", "not used"],
                [
                  "GBIF + your presences",
                  "your presences first, then GBIF; a cell given by both counts once, as yours",
                  "never presences; evaluated if possible (see validation)",
                ],
                [
                  "Your presences only",
                  "your presences; no GBIF download",
                  "never presences",
                ],
                [
                  "GBIF for training, your data held out",
                  "GBIF records only",
                  "with your presences, independent validation sites",
                ],
              ],
            },
          },
          {
            p: "The Data tab of the results shows, for each source, the records, the presence cells kept and why the others were not kept, and how many GBIF records fall in cells already given by your data.",
          },
        ],
      },
      cleaning: {
        title: "Same rules as GBIF",
        blocks: [
          {
            p: "Your presences pass the same rules, in the same order: dated, recorded since the climate baseline, precision within the limit, inside the study area, on land, with predictor data, one record per grid cell, optional thinning. Two differences only: the licence rule always accepts the organisation's own data, and an **absence record is never a presence** (rule *absence*, first).",
          },
          {
            note: "Field data are often grouped (a transect, a study site). When your presence cells cover less than 10 % of the study area's extent, a note reminds that areas not surveyed are not absences.",
            tone: "info",
          },
        ],
      },
      validation: {
        title: "Independent validation",
        blocks: [
          {
            p: "With data held out for validation, the final map is evaluated on your surveyed sites. Records pass the same cleaning rules and are reduced to **one site per grid cell** (a cell with any detection is a presence site).",
          },
          {
            note: "**Leakage rule.** A site in a cell used as a training presence (GBIF or yours) is excluded: the model already knows its outcome. This also removes your observations that GBIF republishes (SINP → INPN → GBIF) in the same cell.",
            tone: "warn",
          },
          {
            ul: [
              "**AUC (presence–absence)**: probability that a presence site ranks above an absence site.",
              "**Sensitivity, specificity, TSS and kappa** at two thresholds taken from the training data (10th percentile training presence, maximum sensitivity + specificity), never tuned on your data (Allouche et al. 2006). Read TSS with sensitivity and specificity: it depends on prevalence when errors are unbalanced (Leroy et al. 2018).",
              "**Brier score and calibration table**: diagnostics only. Suitability is relative, not a probability of presence; these show how often surveyed sites were occupied at each suitability, they do not turn it into a probability.",
              "**Spread**: extent of the sites and mean distance between neighbouring sites. Validation on a small cluster says little about the rest of the area.",
            ],
          },
          {
            p: "At least 10 presence and 10 absence sites are needed for metrics. With absences of a dataset whose presences trained the model, no independent presence site remains: hold the dataset out to evaluate on it.",
          },
        ],
      },
      validated: {
        title: "The Validated level",
        blocks: [
          {
            p: "*Validated* is above *Supported*. It is given only when **every** criterion holds: the data were held out from training; at least 30 presence and 30 absence sites after the leakage rule; presence–absence AUC ≥ 0.75; TSS ≥ 0.40 at the training max-SSS threshold; and the model meets every criterion of *Supported* (after any cap).",
          },
          {
            note: "These criteria are proposed in ADR-0108 and wait for the review of Natural Solutions ecologists. When independent data were used, the reasons of the level say so and list each criterion (✓/✗); otherwise they keep “No independent validation data…”.",
            tone: "info",
          },
        ],
      },
      confidentiality: {
        title: "Confidentiality",
        blocks: [
          {
            ul: [
              "Only the chosen columns leave your browser; observers, comments and other columns never do.",
              "Datasets are visible to your organisation only; another organisation gets “not found”. Natural Solutions support does not list them.",
              "Coordinates never appear in the logs, the manifest or the reports: they record the file's SHA-256, the mapping, the system and the counts. Your points appear only in your study's map and its occurrence file (visible to your organisation).",
              "Datasets are deleted with the same retention as studies, or by you at any time.",
            ],
          },
        ],
      },
    },
  },
  future: {
    title: "Future climate projections",
    summary:
      "Projecting a model to CHELSA CMIP6 future climates: periods, emissions scenarios (SSPs) and climate models (GCMs), how to read change maps, and the limits.",
    sections: {
      overview: {
        title: "What a projection does",
        blocks: [
          {
            p: "In Expert mode (model step, *Future climate projections*) a study can project its final model to future climates. The model is fitted once on the 1981–2010 climate; for each scenario, every **climate** predictor is replaced by its future value under each chosen climate model, and the model predicts again. Terrain, land-cover and water predictors stay at their present values.",
          },
          {
            p: "Future layers are the CHELSA v2.1 CMIP6 climatologies (Karger et al. 2017), downscaled at 30″ from five ISIMIP3b climate models; they are read on exactly the same grid window and aggregated like the present layers. Proposed in ADR-0016, pending review by Natural Solutions ecologists.",
          },
          {
            note: "A projection shows where **suitable climatic conditions** may be in the future. It is not a forecast of where the species will live: no dispersal, population dynamics, adaptation, biotic interaction or land-use change is modelled.",
            tone: "warn",
          },
        ],
      },
      scenarios: {
        title: "Periods, SSPs and climate models",
        blocks: [
          {
            ul: [
              "**Periods**: 30-year climatologies 2011–2040, 2041–2070 and 2071–2100.",
              "**SSPs** (shared socio-economic pathways; O'Neill et al. 2016): SSP1-2.6 strong mitigation (about 2 °C by 2100), SSP3-7.0 regional rivalry with little mitigation (about 3.5–4 °C), SSP5-8.5 fossil-fuelled development (the warmest). They are scenarios, not forecasts: none is more likely than another, so compare at least two.",
              "**Climate models (GCMs)**: GFDL-ESM4, IPSL-CM6A-LR, MPI-ESM1-2-HR, MRI-ESM2-0 and UKESM1-0-LL. They give different futures for the same scenario; **use all five** so that this spread is visible (Thuiller et al. 2019).",
            ],
          },
          {
            note: "Cost: each scenario × model reads the model's climate layers once (downloaded on first use, then cached) and predicts the whole study area once, so a study with 5 models costs about 5 present predictions per scenario. At most 15 scenario × model combinations per study.",
            tone: "info",
          },
        ],
      },
      reading: {
        title: "Reading the maps and areas",
        blocks: [
          {
            ul: [
              "**Future suitability**: the mean over the climate models of the model's relative suitability with future climate. Same scale as the present map, never a probability.",
              "**Spread across models**: the standard deviation of that suitability across the models. High values mean the models disagree about the cell.",
              "**Change in suitable conditions**: the present P10 threshold applied to both periods. Each cell is *suitable now and in the future*, *no longer suitable*, *newly suitable* or *unsuitable in both*, judged on the model mean. Pale cells: fewer than a majority of the models agree with that class.",
              "**Hatching**: future conditions outside the range of the training data under at least one model (future MESS < 0). Projections there are extrapolations; do not interpret them without saying so.",
            ],
          },
          {
            p: "The area table gives each class in km² (exact ellipsoidal cell areas) and as a share of the present suitable area, the part where every model agrees, and the range of the future suitable area across the models. Report the range, not the mean alone.",
          },
        ],
      },
      limits: {
        title: "Limits",
        blocks: [
          {
            ul: [
              "**No dispersal**: *newly suitable* means conditions the species could tolerate, not places it will reach; *no longer suitable* does not mean local extinction at a given date.",
              "**Only climate changes**: land cover, land use, terrain and water stay as today, although they will change too.",
              "**Model transfer**: relationships fitted on 1981–2010 conditions may not hold in another climate (Yates et al. 2018), especially where future conditions are new (hatched; Elith et al. 2010).",
              "**Climate-model uncertainty** is shown, not removed; the spread between SSPs is usually larger later in the century.",
              "Choose the wording “change in the area of suitable conditions”, never “range shift” or “future distribution”.",
            ],
          },
        ],
      },
      files: {
        title: "Files and citation",
        blocks: [
          {
            p: "Per scenario: `future_<period>_<ssp>_mean.tif` (mean suitability), `_sd.tif` (spread), `_change.tif` (classes 1 unsuitable in both, 2 no longer suitable, 3 newly suitable, 4 suitable in both), `_agreement.tif` (models agreeing with the class) and `_mess.tif` (lowest future MESS). `future_areas.csv` lists the areas per scenario and per model; the reports have a *Future climate projections* section and `manifest.json` lists every future file read.",
          },
          {
            p: "Cite CHELSA (Karger et al. 2017), CMIP6 (Eyring et al. 2016) and the ISIMIP3b bias adjustment (Lange 2019), and acknowledge the World Climate Research Programme and the five modelling groups: the reports carry this text. The CMIP6 model output is under CC BY 4.0; the licence of the CHELSA future files is still being confirmed with CHELSA.",
          },
        ],
      },
    },
  },
  decision: {
    title: "Decision support",
    summary:
      "Survey priority, project footprints, species with suitable conditions and comparing two studies: outputs computed from a finished study to support a decision.",
    sections: {
      overview: {
        title: "What these outputs are for",
        blocks: [
          {
            p: "A map of relative suitability does not answer a client's question by itself. The **Decision support** tab of a finished study computes, on demand, four outputs that do: where to survey, what a project footprint overlaps, which areas matter for several species, and how much a map changes between two studies.",
          },
          {
            ul: [
              "They are computed from the study's saved results: **no model is refitted**, and the study's own files and manifest are not changed.",
              "They inherit every limit of the models they come from — read the evidence level of each species first.",
            ],
          },
          {
            note: "These outputs do not replace a field inventory or an impact assessment. Low suitability is not evidence of absence.",
            tone: "warn",
          },
        ],
      },
      priority: {
        title: "Survey priority: areas worth surveying",
        blocks: [
          {
            p: '**What it answers:** where field surveys for one species are most worth doing — places with higher relative suitability that have been little sampled. **What it does not answer:** where the species is. A high priority means "worth looking", not "present".',
          },
          {
            p: "Each cell of the study area gets a score from 0 to 1: its suitability rank multiplied by how little it has been sampled. Four classes cross suitability (at or above the species' P10 threshold, or below) with sampling (less or more than the median cell):",
          },
          {
            ul: [
              "**Prospect first** — higher suitability, little sampled.",
              "**Known** — higher suitability, already well sampled.",
              "**Lower suitability, little sampled** and **lower suitability, well sampled**.",
            ],
          },
          {
            p: "**How sampling is measured.** When the study used a target-group background, the sampling effort is the number of records of the target group per cell ([sampling bias](#data)). Otherwise NicheRS uses a proxy, the **distance to the nearest known record of the species**, and says so. That proxy is weaker: a place far from records may have been surveyed without finding the species.",
          },
          {
            p: 'Candidate areas are drawn from the 10 % of the study area with the highest priority scores: groups of neighbouring "prospect first" cells there, ranked by their mean score, with centroid, bounding box, area and the share where the model extrapolates. They download as GeoJSON and CSV for field planning. Model-guided sampling has helped find rare species (Guisan et al. 2006; Le Lay et al. 2010).',
          },
        ],
      },
      footprint: {
        title: "Project footprint: a pre-diagnostic",
        blocks: [
          {
            p: "Upload the footprint of a project (GeoJSON polygon in longitude/latitude) on a finished study. For every species, NicheRS reports what the models say inside it:",
          },
          {
            ul: [
              "the area with suitable conditions inside the footprint at the P10 threshold, with the band from the most inclusive (MTP) to the maximum sensitivity + specificity threshold;",
              "the share of the footprint it represents, and the share of the species' suitable area in the whole study area that the footprint covers;",
              "mean and maximum relative suitability, uncertainty, and the shares where the model extrapolates or is outside its area of applicability;",
              "the evidence level of each model.",
            ],
          },
          {
            p: "Areas are exact: the footprint is intersected with each model cell on the WGS84 ellipsoid. But the model's values belong to whole cells: a footprint smaller than a cell is described by the cells it touches, and NicheRS warns about it.",
          },
          {
            note: "A pre-diagnostic only: it does not replace a field inventory or an impact assessment, and low suitability inside a footprint is not evidence that the species is absent.",
            tone: "warn",
          },
          {
            p: "Footprints are stored with the study, visible only to your organisation, and deleted with the study.",
          },
        ],
      },
      richness: {
        title: "Species with suitable conditions and hotspots",
        blocks: [
          {
            p: "For a study with several species, NicheRS stacks their maps. Each cell gets the **number of species with suitable conditions** (each species at its own P10 threshold) and the **sum of relative suitability**, an index for ranking cells.",
          },
          {
            note: "This is not the number of species present. Stacked models overpredict: each map shows where conditions suit a species, not where it lives, and they ignore competition, dispersal and how many species a place can hold (Calabrese et al. 2014; Guisan & Rahbek 2011).",
            tone: "warn",
          },
          {
            ul: [
              "Only species at or above a minimum evidence level are stacked — Indicative by default, so **Exploratory models are left out** unless you lower the minimum. Left-out species are listed with the reason.",
              "**Hotspots** are the 10 % of cells with the most species with suitable conditions (ties broken by the summed suitability). The 10 % cut is a convention, not an ecological threshold.",
            ],
          },
        ],
      },
      compare: {
        title: "Comparing two studies",
        blocks: [
          {
            p: "Compare two finished studies of the same species — for example random vs target-group background, MaxEnt vs random forest, or climate only vs climate + land cover. NicheRS shows the settings that differ, both studies' metrics, evidence levels and areas, and a map of the difference (B − A).",
          },
          {
            ul: [
              "The two studies must be on the **same grid** (same study area and resolution); otherwise the comparison is refused rather than resampled.",
              "**Schoener's D** (0 = no overlap, 1 = identical) measures how similar the two suitability surfaces are once each is scaled to sum to 1 (Warren et al. 2008); **Spearman's correlation** compares the cells' ranks.",
              "The P10 agreement gives the area suitable in both studies, in only one, or in neither.",
            ],
          },
          {
            p: "This is a sensitivity check: it tells you how much the answer depends on a choice, not which study is right. Judge fit with each study's validation metrics, keeping in mind that different validation designs are not directly comparable.",
          },
        ],
      },
      files: {
        title: "Files and traceability",
        blocks: [
          {
            p: "Each output is saved in an `analyses/` folder of the study, with its rasters (GeoTIFF), tables (CSV, GeoJSON), a printable page (`report.html`) and its own small manifest. That manifest records the checksum of the study's execution manifest and of every study file read, the method and the settings used, so the output can be traced back to the exact study.",
          },
          {
            p: "The study's own outputs and manifest are never changed. Deleting a study deletes its decision-support outputs.",
          },
        ],
      },
    },
  },
  exports: {
    title: "Exports & reproducibility",
    summary:
      "GeoTIFF rasters, GeoJSON, .lambdas, response curves, a printable report and the execution manifest.",
    sections: {
      files: {
        title: "Files per species",
        blocks: [
          {
            table: {
              caption: "Exported files",
              head: ["File", "Content"],
              rows: [
                [
                  "`suitability.tif`",
                  "Relative habitat suitability (cloglog 0–1) of the final model; not a probability.",
                ],
                [
                  "`uncertainty.tif`",
                  "Standard deviation of suitability across the cross-validation models.",
                ],
                ["`mess.tif`", "MESS; negative values are extrapolation."],
                [
                  "`effort.tif`",
                  "Only with a target-group background: GBIF records of the target group per cell (records, not visits or observers; synthetic in the demo).",
                ],
                [
                  "`model.lambdas`",
                  "Final model in the MaxEnt 3.4.4 `.lambdas` format.",
                ],
                [
                  "`occurrences.geojson`",
                  "Retained presences and excluded records with their reason (capped; exact counts in the report). Coordinates as published by GBIF: check the SINP regional référentiel before reproducing points in a public report.",
                ],
                [
                  "`report.md`",
                  "Methods and results summary (ODMAP-style), a starting point for the methods section of a report.",
                ],
                [
                  "`report.html`",
                  "Printable report: maps with the presences, uncertainty, extrapolation, response curves, tables, limits and citations. “Open printable report” shows it in the browser; print it to PDF to attach it to a study.",
                ],
                [
                  "`response_curves.csv`",
                  "Marginal response curves of the final model (`variable,x,y`, 501 points per variable).",
                ],
                [
                  "`model.rf.json`",
                  "Random forest or ensemble only: the final down-sampled random forest (settings, seed, trees; JSON). With the ensemble, `model.lambdas` is its MaxEnt member.",
                ],
                [
                  "`partial_dependence.csv`",
                  "Random forest or ensemble only: partial dependence of the final forest (`variable,x,y`, 51 points per variable).",
                ],
                [
                  "`manifest.json`",
                  "Execution manifest of the study (button “Download execution manifest”).",
                ],
              ],
            },
          },
          {
            p: "For an *Insufficient* species, only `occurrences.geojson`, `report.md` and `report.html` are produced.",
          },
        ],
      },
      geotiff: {
        title: "GeoTIFF format",
        blocks: [
          {
            p: "Rasters are single-band Float32 GeoTIFF in WGS84 geographic coordinates (EPSG:4326) on the native CHELSA grid, tiled 256 × 256 with Deflate compression, laid out like a Cloud-Optimized GeoTIFF but without overviews. NoData is NaN. They open directly in QGIS and GDAL.",
          },
          {
            p: "To compute areas in a GIS, use ellipsoidal areas or an equal-area projection, never a cell count.",
          },
        ],
      },
      manifest: {
        title: "Execution manifest",
        blocks: [
          {
            p: "The manifest records what is needed to trace a study: the full specification (species, study area, filters, model and validation settings) and its SHA-256, the random seed, software version, environmental data provenance, the GBIF source with its download, DOI citation and access date, the methods (selection, uncertainty, extrapolation, area, evidence rules), and per-species results with SHA-256 checksums of every file.",
          },
          {
            p: "Studies are immutable. Use “Duplicate & edit” to make a variant; the new study has its own manifest. Keep the manifest with the deliverable: it is the reference if a result is questioned.",
          },
        ],
      },
    },
  },

  account: {
    title: "Quota & account",
    summary:
      "Hosted service: modelled species per month, invitations, password resets, deletion and retention.",
    sections: {
      quota: {
        title: "Monthly quota: modelled species",
        blocks: [
          {
            p: "The hosted service counts **modelled species**: a species counts when a model is fitted for it, whatever its evidence level (*Exploratory* included), in the calendar month (UTC) in which it finishes.",
          },
          {
            ul: [
              "**Do not count:** *Insufficient* species (no model fitted), failed or cancelled species, viewing and exporting existing results.",
              "A resumed study is never counted twice for the same species.",
              "Deleting a study does not give the quota back.",
              "At launch, species already used plus those reserved by queued or running studies plus the new ones must fit in the quota; otherwise the study is refused (“Monthly quota reached”). Existing results stay available.",
              "The quota is monthly and does not carry over. The account menu in the top bar shows the species left this month; My organisation → Overview shows used, reserved and remaining species.",
            ],
          },
        ],
      },
      limits: {
        title: "Limits per study",
        blocks: [
          {
            ul: [
              "At most 25 species per study.",
              "Study-area extent of at most 2.5 million modelling-grid cells (metropolitan France at 1 km is about 2.0 million).",
              "Studies are queued fairly between organisations; the first GBIF download of a study can take a while (GBIF's own queue).",
            ],
          },
        ],
      },
      access: {
        title: "Invitations and passwords",
        blocks: [
          {
            ul: [
              "Access is by invitation only, issued by Natural Solutions or by your organisation's **managers** (account menu → My organisation): they invite colleagues, send password-reset links and disable accounts of the organisation, within its quota. Ask Natural Solutions to name a manager.",
              "An invitation link is single-use and expires after 7 days. NicheRS sends no e-mail; whoever invites you sends the link, with a ready-made message.",
              "Passwords have at least 12 characters; change yours in My account (your other sessions are closed). Forgotten password: ask a manager of your organisation or Natural Solutions for a reset link (same mechanism); it closes your open sessions.",
              "After 10 failed sign-ins for one e-mail within 15 minutes, sign-in is blocked for that e-mail until the window passes.",
            ],
          },
        ],
      },
      gbif: {
        title: "Your GBIF account",
        blocks: [
          {
            p: "Studies download GBIF occurrence records with a GBIF account; each download gets a DOI to cite in reports. GBIF limits how many downloads an account may run at once, so a busy shared account makes studies wait.",
          },
          {
            ol: [
              "**Your own account** (account menu → My account → GBIF account), if you set one.",
              "Otherwise **your organisation's account**, set by a manager (My organisation).",
              "Otherwise the **shared Natural Solutions account**.",
            ],
          },
          {
            ul: [
              "Enter your gbif.org **user name** (not your e-mail) and password. NicheRS asks GBIF at once whether it accepts them; an account GBIF refuses is skipped until it is corrected.",
              "The password is encrypted on the server, never shown again and never written to logs. Each study's manifest records which kind of account made its download (yours, the organisation's or the shared one), not the user name.",
              "Each account keeps its own downloads: a study never cites a download made with another organisation's account.",
              "The account is fixed when a study's data preparation starts; changing it later does not affect studies already prepared.",
            ],
          },
        ],
      },
      retention: {
        title: "Deletion and retention",
        blocks: [
          {
            ul: [
              "You can delete a finished study at any time; its files are removed.",
              "Finished studies are deleted automatically after 365 days. Download what you need to keep (files and manifest).",
              "Your organisation's studies are visible only to its members. Natural Solutions administrators can open them for support; each such access is logged.",
              "To have your account erased (name, e-mail and password), ask Natural Solutions: the deletion is immediate and final; your organisation keeps its studies. GBIF observer names are never stored.",
            ],
          },
        ],
      },
    },
  },

  glossary: {
    title: "Glossary",
    summary: "Short definitions of the terms used in NicheRS.",
    sections: {
      terms: {
        title: "Terms",
        blocks: [
          {
            dl: [
              [
                "Relative habitat suitability",
                "MaxEnt score from 0 to 1 per cell, relative to the study area; not a probability of presence.",
              ],
              [
                "Cloglog",
                "Monotonic transform rescaling MaxEnt's raw output to 0–1 (Phillips et al. 2017); not a calibration.",
              ],
              [
                "MaxEnt",
                "Presence-background modelling method (Phillips et al. 2006); NicheRS uses a port tested against maxent.jar 3.4.4.",
              ],
              [
                "Occurrence / observation",
                "A GBIF record of the species; observed, not predicted.",
              ],
              [
                "Presence cell",
                "A grid cell with at least one retained record; the unit used by the model and the evidence rules.",
              ],
              [
                "Background",
                "10 000 random land cells of the study area describing the available conditions.",
              ],
              [
                "Study area",
                "The area where the model is fitted and predicted.",
              ],
              [
                "Feature classes",
                "Shapes of response allowed to MaxEnt: L linear, Q quadratic, H hinge, P product.",
              ],
              [
                "Regularization multiplier (β)",
                "Penalty on model complexity; higher values give smoother responses.",
              ],
              [
                "Spatial block cross-validation",
                "Validation in which each of 4 geographic blocks is predicted by a model trained on the other three.",
              ],
              ["Fold", "One held-out part of the data in cross-validation."],
              [
                "Continuous Boyce index (CBI)",
                "−1 to 1: how well predictions rank held-out presences against the background.",
              ],
              [
                "OR10",
                "Share of held-out presences below the 10th-percentile training threshold; expected ≈ 10 %.",
              ],
              [
                "ORmtp",
                "Share of held-out presences below the minimum training presence; expected 0 %.",
              ],
              [
                "Presence-background AUC",
                "Discrimination of presences against background; depends on the study area; secondary metric.",
              ],
              [
                "Threshold rule",
                "MTP, P10 or maximum SSS: how a suitability value is chosen to delimit suitable conditions.",
              ],
              [
                "Area of suitable conditions",
                "Summed exact area of the cells at or above a threshold; not the occupied area.",
              ],
              [
                "MESS / extrapolation",
                "Environmental similarity to training conditions; negative values mean extrapolation.",
              ],
              [
                "Uncertainty (fold SD)",
                "Per-cell standard deviation of suitability across fold models.",
              ],
              [
                "Evidence level",
                "*Insufficient*, *Exploratory*, *Indicative* or *Supported*, from fixed rules (ADR-0009).",
              ],
              [
                "Collinearity",
                "Correlation between predictors; |r| > 0.7 leads to a variable being dropped.",
              ],
              [
                "CHELSA v2.1",
                "Global climatologies 1981–2010 at 30″, licence CC0 (Karger et al. 2017).",
              ],
              [
                "Land-cover fraction",
                "Share (0–1) of a cell covered by a group of ESA WorldCover 2021 classes.",
              ],
              [
                "GBIF download DOI",
                "Permanent identifier of the exact set of GBIF records used; cite it.",
              ],
              [
                "Execution manifest",
                "JSON record of a study's inputs, versions, seed, settings and file checksums.",
              ],
              [
                "Modelled species",
                "Quota unit: a species for which a model is fitted.",
              ],
              [
                "Synthetic demo",
                "Built-in artificial dataset for learning the tool; never real data.",
              ],
            ],
          },
        ],
      },
    },
  },

  responsible: {
    title: "Limits & responsible use",
    summary:
      "Who is responsible for interpretation, how to cite results in a client report, attribution and known limits.",
    sections: {
      responsibility: {
        title: "Responsibility",
        blocks: [
          {
            note: "NicheRS is a decision-support instrument. **Interpreting the results, and using them in a deliverable, is the responsibility of the professional who signs the report.**",
            tone: "warn",
          },
          {
            p: "The software applies documented rules. It does not know the site, the species' ecology beyond the records, or the regulatory context.",
          },
        ],
      },
      citing: {
        title: "Citing a result in a client report",
        blocks: [
          {
            ol: [
              "Name the software and version (in `report.md`) and the method: MaxEnt (Phillips et al. 2006, 2017), NicheRS port of MaxEnt 3.4.4.",
              "Adapt the methods text of `report.md`: data, filters, exclusions, predictors, tuning, validation.",
              "Give the evidence level and its reasons verbatim, including the sentence on independent validation.",
              "Describe the output as “relative habitat suitability (cloglog, 0–1), not a probability of presence”.",
              "Give each area with its threshold rule and the band across rules.",
              "Show the extrapolation hatch and the uncertainty map, or state their values.",
              "Cite the GBIF download DOI and CHELSA; archive `manifest.json` (and the study id) with the deliverable.",
            ],
          },
        ],
      },
      attribution: {
        title: "Attribution: GBIF and CHELSA",
        blocks: [
          {
            ul: [
              "GBIF: *GBIF.org (date) GBIF Occurrence Download* followed by the DOI of the study. CC BY records are attributed through this citation.",
              "CHELSA: Karger D.N. et al. (2017) Climatologies at high resolution for the earth's land surface areas. *Scientific Data* 4:170122, doi:10.1038/sdata.2017.122; data CHELSA v2.1, doi:10.16904/envidat.228.",
              "Land mask: Natural Earth (public domain).",
            ],
          },
        ],
      },
      avoid: {
        title: "Never",
        blocks: [
          {
            ul: [
              "Never write “probability of presence”, a percentage of confidence, “range” or “occupied area” about these outputs.",
              "Never remove the extrapolation hatch from a published map.",
              "Never compare suitability scores or AUC between species or study areas.",
              "Never use low suitability as proof of absence in an impact assessment.",
              "Never base a regulatory decision on an *Exploratory* map alone.",
              "Never use synthetic demo outputs, or results including CC BY-NC records, in a paid deliverable.",
            ],
          },
        ],
      },
      limits: {
        title: "Known limits of this version",
        blocks: [
          {
            ul: [
              "Terrestrial species only; climate 1981–2010 by default (terrain, land cover 2021 and distance to water optional; no soils; future climate projections optional).",
              "No sampling-bias correction by default (the target-group background is an Expert option and needs the regional GBIF mirror); independent validation only with your organisation's own presence–absence data (Expert option, [Your own field data](#field-data)): without them the evidence comes from cross-validation of the same records.",
              "MaxEnt by default; a down-sampled random forest and a MaxEnt + random forest ensemble are Expert options pending review ([Modelling algorithms](#algorithms)).",
              "Coastal cells depend on a land mask with about 0.5–1 km accuracy.",
            ],
          },
        ],
      },
    },
  },
} satisfies Record<string, ArticleContent>;

export const en = { ui, articles };
export type EnContent = typeof en;
