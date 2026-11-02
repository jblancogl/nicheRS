// Strings of the results page (tabs, cards, tables, map layer control).
// Scientific wording (AGENTS.md): relative suitability, never a probability. Server free texts
// (warnings, evidence reasons, exclusion reasons) are shown as they come.

import type { SelectedModel, ThresholdArea } from "@/api/types";
import { defineMessages } from "@/i18n/messages";

type Rule = ThresholdArea["rule"];

const EN_RULE: Record<Rule, string> = {
  p10: "10th percentile training presence",
  mtp: "minimum training presence",
  max_sss: "maximum sensitivity + specificity",
};
const FR_RULE: Record<Rule, string> = {
  p10: "10e percentile des présences d'apprentissage",
  mtp: "minimum des présences d'apprentissage",
  max_sss: "maximum sensibilité + spécificité",
};
const ES_RULE: Record<Rule, string> = {
  p10: "percentil 10 de las presencias de entrenamiento",
  mtp: "presencia mínima de entrenamiento",
  max_sss: "máxima sensibilidad + especificidad",
};
const cap = (s: string) => s.charAt(0).toUpperCase() + s.slice(1);
/** Server labels are English; other languages translate by rule id and fall back to the server text. */
const ruleBy = (table: Record<string, string>) => (rule: string, server?: string) => {
  const t = table[rule];
  return t ? cap(t) : server || rule;
};

// Descriptions of the exported files (server FileInfo.description is English), by file name.
const FR_FILES: Record<string, string> = {
  "suitability.tif": "Idonéité relative de l'habitat (cloglog 0–1), modèle final ; ce n'est PAS une probabilité de présence",
  "uncertainty.tif": "Écart-type de l'idonéité entre les modèles de validation croisée",
  "mess.tif": "Similarité environnementale multivariée (MESS) ; négatif = extrapolation",
  "aoa.tif": "Indice de dissimilarité du domaine d'applicabilité (AOA) ; au-dessus du seuil = hors du domaine",
  "effort.tif":
    "Effort d'échantillonnage : données GBIF du groupe cible par cellule (miroir régional ; synthétique dans la démonstration), NaN hors de la zone d'étude. Des données, pas des visites ni des observateurs",
  "model.lambdas": "Modèle MaxEnt final (format .lambdas de MaxEnt 3.4.4)",
  "model.lambdas:ensemble": "Membre MaxEnt final de l'ensemble (format .lambdas de MaxEnt 3.4.4)",
  "suitability.tif:rf":
    "Indice d'idonéité relative de l'habitat (part des arbres de la forêt aléatoire votant « présence », 0–1), modèle final ; ce n'est PAS une probabilité de présence",
  "suitability.tif:ensemble":
    "Indice d'idonéité relative de l'habitat de l'ensemble MaxEnt + forêt aléatoire (rang centile moyen des deux modèles parmi les points de fond, 0–1), modèle final ; ce n'est PAS une probabilité de présence",
  "model.rf.json":
    "Forêt aléatoire sous-échantillonnée finale (format niche-rf version 1, JSON : réglages avec la graine, plages d'apprentissage, arbres en tableaux de nœuds ; lire les nombres avec un aller-retour flottant exact pour reproduire les prédictions)",
  "partial_dependence.csv":
    "Dépendance partielle de la forêt aléatoire finale (part moyenne des arbres votant « présence » quand une variable varie, autres variables à leurs valeurs observées ; Friedman 2001)",
  "response_curves.csv":
    "Courbes de réponse marginales du modèle final (idonéité relative cloglog ; 501 points par variable, autres variables à leur moyenne sur les présences)",
  "occurrences.geojson":
    "Occurrences : toutes les présences conservées, plus les occurrences exclues avec leur motif d'exclusion (nombre limité ; décomptes exacts dans le rapport)",
  "report.md": "Synthèse des méthodes et des résultats (inspirée d'ODMAP)",
  "report.html": "Rapport imprimable avec cartes, courbes de réponse et tableaux (à ouvrir dans un navigateur ; imprimer en PDF)",
};
const ES_FILES: Record<string, string> = {
  "suitability.tif": "Idoneidad relativa del hábitat (cloglog 0–1), modelo final; NO es una probabilidad de presencia",
  "uncertainty.tif": "Desviación típica de la idoneidad entre los modelos de validación cruzada",
  "mess.tif": "Similitud ambiental multivariante (MESS); negativo = extrapolación",
  "aoa.tif": "Índice de disimilitud del área de aplicabilidad (AOA); por encima del umbral = fuera del área",
  "effort.tif":
    "Esfuerzo de muestreo: registros de GBIF del grupo objetivo por celda (réplica regional; sintético en la demostración), NaN fuera del área de estudio. Registros, no visitas ni observadores",
  "model.lambdas": "Modelo MaxEnt final (formato .lambdas de MaxEnt 3.4.4)",
  "model.lambdas:ensemble": "Miembro MaxEnt final del ensamble (formato .lambdas de MaxEnt 3.4.4)",
  "suitability.tif:rf":
    "Índice de idoneidad relativa del hábitat (proporción de árboles del bosque aleatorio que votan «presencia», 0–1), modelo final; NO es una probabilidad de presencia",
  "suitability.tif:ensemble":
    "Índice de idoneidad relativa del hábitat del ensamble MaxEnt + bosque aleatorio (rango percentil medio de ambos modelos entre los puntos de fondo, 0–1), modelo final; NO es una probabilidad de presencia",
  "model.rf.json":
    "Bosque aleatorio submuestreado final (formato niche-rf versión 1, JSON: ajustes con la semilla, rangos de entrenamiento, árboles como tablas de nodos; leer los números con ida y vuelta exacta de coma flotante para reproducir las predicciones)",
  "partial_dependence.csv":
    "Dependencia parcial del bosque aleatorio final (proporción media de árboles que votan «presencia» cuando una variable varía, demás variables en sus valores observados; Friedman 2001)",
  "response_curves.csv":
    "Curvas de respuesta marginales del modelo final (idoneidad relativa cloglog; 501 puntos por variable, demás variables en su media sobre las presencias)",
  "occurrences.geojson":
    "Registros de presencia: todas las presencias conservadas, más los registros excluidos con su motivo de exclusión (número limitado; recuentos exactos en el informe)",
  "report.md": "Resumen de métodos y resultados (al estilo ODMAP)",
  "report.html": "Informe imprimible con mapas, curvas de respuesta y tablas (abrir en un navegador; imprimir en PDF)",
};

// Appended by the server to the occurrences.geojson description for real (non-demo) data.
const SINP_NOTE = {
  fr: "Coordonnées telles que publiées par GBIF. Certaines espèces sont sensibles au sens des règles du SINP : vérifiez le référentiel régional avant de reproduire des points dans un rapport public.",
  es: "Coordenadas tal como las publica GBIF. Algunas especies son sensibles según las normas del SINP francés: consulte el référentiel regional antes de reproducir puntos en un informe público.",
};
// Future-climate projection files (ADR-0016): `future_<period>_<ssp>_<kind>.tif`.
interface FutureFiles {
  files: Record<string, (scenario: string) => string>;
  areas: string;
  synthetic: string;
}
const FR_FUTURE: FutureFiles = {
  files: {
    mean: (x) => `Idonéité relative future de l'habitat (${x}) : moyenne sur les modèles climatiques de la prédiction du modèle final avec le climat futur (autres prédicteurs aux valeurs actuelles) ; ce n'est PAS une probabilité de présence`,
    sd: (x) => `Écart-type de l'idonéité relative future entre les modèles climatiques (${x})`,
    change: (x) => `Évolution des conditions favorables au seuil P10 actuel (${x}) : 1 défavorable aujourd'hui et dans le futur, 2 favorable aujourd'hui mais plus dans le futur, 3 nouvellement favorable, 4 favorable aujourd'hui et dans le futur. Aucune dispersion n'est modélisée : 3 signifie des conditions favorables, pas une colonisation`,
    agreement: (x) => `Nombre de modèles climatiques concordant avec la classe d'évolution (${x})`,
    mess: (x) => `MESS futur par rapport aux données d'apprentissage, le plus bas sur les modèles climatiques (${x}) ; négatif = conditions hors de l'étendue d'apprentissage sous au moins un modèle (extrapolation)`,
  },
  areas: "Surfaces de conditions favorables par scénario futur (moyenne des modèles) et par modèle climatique, km² (surfaces exactes des cellules sur l'ellipsoïde) ; seuil P10 actuel ; aucune dispersion modélisée",
  synthetic: "scénario de démonstration SYNTHÉTIQUE, pas des données CMIP6",
};
const ES_FUTURE: FutureFiles = {
  files: {
    mean: (x) => `Idoneidad relativa futura del hábitat (${x}): media entre modelos climáticos de la predicción del modelo final con el clima futuro (demás predictores con sus valores actuales); NO es una probabilidad de presencia`,
    sd: (x) => `Desviación típica de la idoneidad relativa futura entre los modelos climáticos (${x})`,
    change: (x) => `Cambio en las condiciones favorables con el umbral P10 actual (${x}): 1 desfavorable hoy y en el futuro, 2 favorable hoy pero no en el futuro, 3 nuevamente favorable, 4 favorable hoy y en el futuro. No se modela la dispersión: 3 significa condiciones favorables, no colonización`,
    agreement: (x) => `Número de modelos climáticos que coinciden con la clase de cambio (${x})`,
    mess: (x) => `MESS futuro respecto a los datos de entrenamiento, el más bajo entre los modelos climáticos (${x}); negativo = condiciones fuera del rango de entrenamiento en al menos un modelo (extrapolación)`,
  },
  areas: "Superficies de condiciones favorables por escenario futuro (media de modelos) y por modelo climático, km² (superficies exactas de las celdas sobre el elipsoide); umbral P10 actual; sin dispersión modelada",
  synthetic: "escenario de demostración SINTÉTICO, no datos CMIP6",
};
const futureText = (f: FutureFiles, name: string, server: string): string | null => {
  if (name === "future_areas.csv") return f.areas;
  const m = /^future_(\d{4}-\d{4})_ssp(\d)(\d)(\d)_(mean|sd|change|agreement|mess)\.tif$/.exec(name);
  if (!m) return null;
  const scenario = `${m[1]}, SSP${m[2]}-${m[3]}.${m[4]}${/SYNTHETIC/.test(server) ? ` — ${f.synthetic}` : ""}`;
  return f.files[m[5] ?? ""]?.(scenario) ?? null;
};

/** Translated file description, plus the SINP note when the server's description carries it. */
const fileText = (table: Record<string, string>, note: string, future: FutureFiles) => (name: string, server: string) => {
  const ft = futureText(future, name, server);
  if (ft) return ft;
  // Algorithm-specific descriptions (ADR-0017), recognised from the server's English text.
  const variant = /random forest's trees/.test(server) ? ":rf" : /ensemble/i.test(server) ? ":ensemble" : "";
  const t = table[name + variant] ?? table[name];
  if (!t) return server;
  return /SINP/.test(server) && !/SINP/.test(t) ? `${t}. ${note}` : t;
};

export const useResultsMessages = defineMessages({
  en: {
    // Second engine (ADR-0017)
    algorithm: "Algorithm",
    algorithmName: (a: string): string =>
      a === "rf" ? "Down-sampled random forest" : a === "ensemble" ? "Ensemble (MaxEnt + random forest)" : "MaxEnt",
    algorithmShort: (a: string): string => (a === "rf" ? "Random forest" : a === "ensemble" ? "Ensemble" : "MaxEnt"),
    selectedModelValue: (s: SelectedModel) =>
      s.algorithm === "rf"
        ? `Random forest, mtry ${s.rf_mtry ?? "–"}, ${s.rf_trees ?? "–"} trees`
        : s.algorithm === "ensemble"
          ? `Ensemble: MaxEnt ${s.feature_classes}, β = ${s.beta_multiplier} + random forest (mtry ${s.rf_mtry ?? "–"}, ${s.rf_trees ?? "–"} trees)`
          : `${s.feature_classes}, β = ${s.beta_multiplier}`,
    importanceTitle: "Variable importance",
    rfImportance: "RF permutation importance",
    contributionsNoteRf:
      "Random-forest permutation importance: drop in training presence–background AUC when the variable's values are shuffled (3 repeats), normalised to 100. It shows how much the forest relies on a variable, not its ecological effect; correlated variables share credit.",
    contributionsNoteEnsemble:
      "MaxEnt member: percent contribution and permutation importance (MaxEnt 3.4.4 definitions). Random-forest member: permutation importance (drop in training AUC when the variable is shuffled, normalised to 100). Correlated variables share credit.",
    responseCurvesMaxent: "Response curves (MaxEnt member)",
    partialDependence: "Partial dependence (random forest)",
    pdNote:
      "Mean relative suitability (share of trees voting presence) as each variable varies, averaged over the observed values of the other variables (Friedman 2001). Not a MaxEnt response curve: the others are not held at their mean. Curves cover the training range only: trees do not extrapolate, and beyond it predictions stay at the edge value.",
    membersTitle: "Ensemble and its members",
    membersNote:
      "Means across the same folds. The first row is the ensemble evaluated as one model (its fold models combined); it drives the evidence level.",
    tuningSelectedRf: (mtry: number, trees: number) => `Selected: random forest, mtry = ${mtry}, ${trees} trees. Means across folds.`,
    tuningSelectedEnsemble: (fc: string, beta: number, mtry: number, trees: number) =>
      `Selected members: MaxEnt features ${fc}, β = ${beta}; random forest mtry = ${mtry}, ${trees} trees. Means across folds.`,
    rfSetting: (mtry: number) => `RF mtry ${mtry}`,
    setting: "Setting",
    rfSettingsLabel: "Random forest",
    rfSettingsValue: (trees: number, tune: boolean) => `${trees} trees; mtry ${tune ? "√p or p/2 (tuned)" : "√p"}`,
    // ResultsView
    results: "Results",
    noSpeciesResults: "This study has no species results.",
    speciesSwitch: (n: number) => `Species (${n})`,
    duplicate: "Duplicate & edit",
    exports: "Exports",
    demoNotice: "Synthetic demo: these maps describe a virtual species and must not be used as ecological evidence.",
    noModel: (status: string) => `No model for this species (${status})`,
    notModelled: "The species was not modelled. See the study's event log for details.",
    tabSummary: "Summary",
    tabValidation: "Validation",
    tabVariables: "Variables",
    tabData: "Data & exports",
    zoomToResults: "Zoom to results",
    compareSpecies: "Compare species",
    compareNote: "Mean ± SD across folds. AUC is omitted: it is not comparable across species.",
    cannotLoadRun: "Cannot load this study",
    notCompleted: "Results are available once the study has completed.",
    goToRun: "Go to study status",
    resultsUnavailable: "Results unavailable",
    // Threshold rules and validation scheme
    ruleLabel: (rule: string, server?: string) => server || EN_RULE[rule as Rule] || rule,
    ruleInline: (rule: string) => EN_RULE[rule as Rule] ?? rule,
    schemeSpatial: (k: number) => `spatial block cross-validation, k = ${k}`,
    schemeOther: (method: string, k: number) => `${method.replace(/_/g, " ")}, k = ${k}`,
    schemeGrid: (k: number, km: number) => `spatial grid-block cross-validation (${km} km blocks), k = ${k}`,
    blockDesign: "Block design",
    blockSize: "Block size",
    chosenBy: "Chosen by",
    chosenUser: "set in the study settings",
    chosenAuto: "median spatial autocorrelation range of the predictors",
    foldsObtained: "Folds obtained",
    foldsOf: (a: number, r: number) => `${a} of ${r} requested`,
    medianRange: "Median autocorrelation range",
    clampBand: "Allowed block sizes",
    rangesTitle: (n: number) => `Autocorrelation range per predictor (${n})`,
    rangesCaption: "Spatial autocorrelation range per predictor",
    variable: "Variable",
    rangeKm: "Range (km)",
    variogramModel: "Variogram model",
    variogram: (m: string): string => ({ exponential: "exponential", spherical: "spherical", gaussian: "Gaussian" } as Record<string, string>)[m] ?? m,
    atBound: "lower bound: search limit reached",
    notFitted: "not fitted (no spatial structure)",
    variogramNote: (n: string, cutoff: string) => `Variograms fitted on ${n} background points, distances up to ${cutoff} km.`,
    nullTitle: "Null models (random locations)",
    nullLead: (c: number, r: number, n: number) =>
      `${c} of ${r} models fitted, each to ${n} random locations, with the selected setting and the same cross-validation.`,
    nullMetric: "Metric (held-out folds)",
    nullObserved: "This model",
    nullMean: "Random-location models (mean ± SD)",
    pValue: "p",
    nullBetter: "better than random-location models",
    nullNotBetter: "not better than random-location models",
    nullCaption: "Comparison with models fitted to random locations",
    nullNote:
      "p is the share of random-location models that do at least as well as this model (p ≤ 0.05: better than random-location models). It says whether the model beats chance, not whether it is ecologically right; it is not a confidence level.",
    aoaOutside: "Outside the area of applicability",
    // SummaryTab
    evidenceLevel: "Evidence level",
    ruleBased: "rule-based, see reasons",
    areaTitle: "Suitable-conditions area",
    noThreshold: "No threshold was computed for this species.",
    areaLead: "Suitable-conditions area: ",
    areaRange: (band: string) => `(range ${band} across threshold rules)`,
    areaFraction: (pct: string, area: string) => `${pct} of the study area (${area}) · threshold `,
    areaNote:
      "Area where conditions are at least as suitable as the threshold — not the occupied range. It changes with the threshold rule.",
    thresholdRule: "Threshold rule",
    defaultTag: "(default)",
    warnings: (n: number) => `Warnings (${n}) — review before use`,
    validation: "Validation",
    cbi: "Continuous Boyce index",
    cbiTip: "From −1 to 1. Positive values mean predictions rank observed presences higher than chance.",
    or10: "Omission at 10th percentile",
    or10Tip: "Share of held-out presences predicted unsuitable at the training 10th-percentile threshold. Expected ≈ 10 %.",
    expectedParen: (pct: string) => `(expected ${pct})`,
    meanSdFolds: (scheme: string) => `Mean ± SD across folds; ${scheme}.`,
    mapSummary: "Map summary",
    studyArea: "Study area",
    outsideRange: "Outside training range (MESS < 0)",
    foldSpread: "Mean spread between fold models",
    sdUnit: "SD",
    selectedModel: "Selected model",
    mapTextEquivalent: "Text equivalent of the map layers. Click the map to read local values.",
    // ValidationTab
    evaluatedWith: (scheme: string) => `Evaluated with ${scheme}: each fold is predicted by a model trained on the other folds.`,
    primaryMetrics: "Primary metrics (mean ± SD across folds)",
    cbiFull: "Continuous Boyce index (CBI)",
    or10Full: "Omission at 10th percentile (OR10)",
    ormtpFull: "Omission at minimum training presence",
    expected: (pct: string) => `expected ${pct}`,
    secondary: "Secondary",
    aucPb: "presence-background AUC",
    aucNote:
      "Presence-background AUC compares presences with background points, not with known absences. It is only meaningful for comparing models of the same species and study area — never across species.",
    perFold: "Per fold",
    perFoldCaption: "Validation metrics per fold",
    fold: "Fold",
    testN: "Test n",
    aucShort: "AUC (p-b)",
    cbiMissing: "— CBI not computed for folds with too few test presences.",
    tuning: (n: number) => `Tuning (${n} combinations)`,
    tuningSelected: (fc: string, beta: number) => `Selected: features ${fc}, β = ${beta}. Means across folds.`,
    tuningCaption: "Tuning grid",
    features: "Features",
    coef: "Coef.",
    selectedSr: " (selected)",
    // VariablesTab
    contributions: "Variable contributions",
    contributionsNote:
      "Percent contribution: share of the training gain attributed to each variable. Permutation importance: drop in training AUC when the variable's values are randomly permuted, normalised to 100 (MaxEnt 3.4.4 definitions). Correlated variables share credit, so a low value does not mean a variable is ecologically irrelevant.",
    noContributions: "No contributions reported.",
    percentContribution: "Percent contribution",
    permutationImportance: "Permutation importance",
    responseCurves: "Response curves",
    curvesNote:
      "Relative suitability (cloglog) as each variable varies, all other variables held at their mean over the presence records. Shaded: values outside the training data, where the model is clamped and the curve is an extrapolation. With correlated variables, marginal curves can hide effects of variables changing together.",
    curveSummary: (label: string, lo: string, hi: string, peak: string, tmin: string, tmax: string) =>
      `${label}: relative suitability from ${lo} to ${hi}, highest at ${peak}; training data from ${tmin} to ${tmax}.`,
    variablesUsed: (n: number) => `Variables used (${n})`,
    variablesDropped: (n: number) => `Variables dropped (${n})`,
    droppedReason: (r: string, other: string, t: string) => `|r| = ${r} with ${other} (threshold ${t})`,
    none: "None.",
    // DataTab
    data: "Data",
    presences: "Presences (unique cells)",
    background: "Background points",
    recordsFetched: "Records fetched",
    source: "Source",
    sourceQuery: "Source query",
    excludedCaption: "Excluded records",
    exclusionReason: "Exclusion reason",
    records: "Records",
    provenance: "Provenance",
    runId: "Study id",
    created: "Created",
    finished: "Finished",
    resolutionFactor: "Resolution factor",
    filters: "Filters",
    filtersValue: (year: number, unc: string, licences: string) => `since ${year}, ≤ ${unc} m, ${licences}`,
    tuningLabel: "Tuning",
    tuningValue: (tuning: string, fc: string, betas: string) => `${tuning}; features ${fc}; β ${betas}`,
    collinearity: "Collinearity |r|",
    seed: "Seed",
    exportsCount: (n: number) => `Exports (${n})`,
    noFiles: "No files were produced for this species.",
    fileDescription: (_name: string, server: string) => server,
    download: (name: string) => `Download ${name}`,
    openReport: "Open printable report",
    downloadManifest: "Download execution manifest",
    hideManifest: "Hide manifest",
    viewManifest: "View manifest",
    manifestUnavailable: "Manifest unavailable",
    manifestJson: "Execution manifest JSON",
    // ComparisonTable
    comparisonCaption: "Comparison across species",
    species: "Species",
    evidence: "Evidence",
    areaP10: "Area (P10)",
    // MapOverlay
    layerSuitability: "Suitability (relative, 0–1)",
    layerUncertainty: "Uncertainty (SD across folds)",
    layerExtrapolation: "Extrapolation (MESS < 0)",
    layerBinary: "Suitable / not suitable at threshold",
    layersLegend: "Layers & legend",
    mapLayer: "Map layer",
    opacity: "Opacity",
    hatching: "Extrapolation hatching overlay",
    aoaHatching: "Outside the area of applicability (hatching)",
    occurrences: "Occurrences",
    noOccurrences: "Occurrence points are not exported for this study.",
    clickToRead: "Click the map (or focus it and press I) to read values.",
    // Sampling bias, background, thinning, generalised coordinates (ADR-0011, ADR-0013)
    layerEffort: "Sampling effort (target-group records)",
    methodTitle: "Method",
    bgLabel: "Background",
    bgRandom: "random over the study area (no sampling-bias correction)",
    bgTargetGroup: (cls: string) => `target group ${cls}: drawn in cells with GBIF records of this class (sampling-bias correction)`,
    bgFallback: (cls: string | null) =>
      `random over the study area: target group ${cls ?? "(the species' own class)"} requested but not available, so no bias correction`,
    bgFallbackReason: (reason: string) => `Why: ${reason}.`,
    bgEligible: (n: string) => `${n} cells of the study area have target-group records.`,
    thinLabel: "Distance thinning",
    thinValue: (km: string) => `presence cells at least ${km} km apart`,
    nullSampling: "Random locations drawn",
    nullUniform: "uniformly over cells with predictor data",
    nullEffort: "bias-matched (target-group effort)",
    biasTitle: "Sampling bias (target-group effort)",
    biasGroup: "Target group (GBIF class)",
    biasRecords: "Target-group records",
    biasCells: "Cells with records",
    biasCellsValue: (n: string, pct: string) => `${n} (${pct} of the study area)`,
    biasMaxCell: "Most records in one cell",
    biasConcentration: (share: string, top: string) =>
      `${share} of the presence cells lie in the most-surveyed ≈ ${top} of the study area (≈ ${top} expected if presences ignored observer effort).`,
    biasDescriptive: "Descriptive only: it does not change the evidence level. Counts are records, not visits or observers.",
    biasSynthetic: "Synthetic effort surface (demo data), not real GBIF records.",
    biasShowMap: "Show the effort map",
    generalisedTitle: "Generalised coordinates",
    genRecords: "Records generalised by their publishers",
    genExcluded: "Excluded (precision not stated or above the limit)",
    genRetained: "Retained (stated precision within the limit)",
    genFlagsUnavailable:
      "This GBIF source does not report coordinate generalisation: generalised records with small or missing uncertainty cannot be detected.",
    thinFilter: (km: string) => `, thinning ${km} km`,
  },
  fr: {
    // Second moteur (ADR-0017)
    algorithm: "Algorithme",
    algorithmName: (a: string): string =>
      a === "rf" ? "Forêt aléatoire sous-échantillonnée" : a === "ensemble" ? "Ensemble (MaxEnt + forêt aléatoire)" : "MaxEnt",
    algorithmShort: (a: string): string => (a === "rf" ? "Forêt aléatoire" : a === "ensemble" ? "Ensemble" : "MaxEnt"),
    selectedModelValue: (s: SelectedModel) =>
      s.algorithm === "rf"
        ? `Forêt aléatoire, mtry ${s.rf_mtry ?? "–"}, ${s.rf_trees ?? "–"} arbres`
        : s.algorithm === "ensemble"
          ? `Ensemble : MaxEnt ${s.feature_classes}, β = ${s.beta_multiplier} + forêt aléatoire (mtry ${s.rf_mtry ?? "–"}, ${s.rf_trees ?? "–"} arbres)`
          : `${s.feature_classes}, β = ${s.beta_multiplier}`,
    importanceTitle: "Importance des variables",
    rfImportance: "Importance par permutation (forêt)",
    contributionsNoteRf:
      "Importance par permutation de la forêt aléatoire : baisse de l'AUC présence–fond d'apprentissage quand les valeurs de la variable sont mélangées (3 répétitions), normalisée à 100. Elle indique combien la forêt s'appuie sur une variable, pas son effet écologique ; des variables corrélées se partagent le crédit.",
    contributionsNoteEnsemble:
      "Membre MaxEnt : contribution en pourcentage et importance par permutation (définitions de MaxEnt 3.4.4). Membre forêt aléatoire : importance par permutation (baisse de l'AUC d'apprentissage quand la variable est mélangée, normalisée à 100). Des variables corrélées se partagent le crédit.",
    responseCurvesMaxent: "Courbes de réponse (membre MaxEnt)",
    partialDependence: "Dépendance partielle (forêt aléatoire)",
    pdNote:
      "Idonéité relative moyenne (part des arbres votant « présence ») quand chaque variable varie, moyennée sur les valeurs observées des autres variables (Friedman 2001). Ce n'est pas une courbe de réponse MaxEnt : les autres variables ne sont pas fixées à leur moyenne. Les courbes couvrent seulement la plage d'apprentissage : les arbres n'extrapolent pas et, au-delà, les prédictions restent à la valeur du bord.",
    membersTitle: "L'ensemble et ses membres",
    membersNote:
      "Moyennes sur les mêmes plis. La première ligne est l'ensemble évalué comme un seul modèle (ses modèles de pli combinés) ; c'est elle qui détermine le niveau de preuve.",
    tuningSelectedRf: (mtry: number, trees: number) => `Retenu : forêt aléatoire, mtry = ${mtry}, ${trees} arbres. Moyennes sur les plis.`,
    tuningSelectedEnsemble: (fc: string, beta: number, mtry: number, trees: number) =>
      `Membres retenus : MaxEnt classes ${fc}, β = ${beta} ; forêt aléatoire mtry = ${mtry}, ${trees} arbres. Moyennes sur les plis.`,
    rfSetting: (mtry: number) => `Forêt mtry ${mtry}`,
    setting: "Réglage",
    rfSettingsLabel: "Forêt aléatoire",
    rfSettingsValue: (trees: number, tune: boolean) => `${trees} arbres ; mtry ${tune ? "√p ou p/2 (calibré)" : "√p"}`,
    results: "Résultats",
    noSpeciesResults: "Cette étude ne contient aucun résultat par espèce.",
    speciesSwitch: (n: number) => `Espèces (${n})`,
    duplicate: "Dupliquer et modifier",
    exports: "Exports",
    demoNotice:
      "Démonstration synthétique : ces cartes décrivent une espèce virtuelle et ne doivent pas servir de preuve écologique.",
    noModel: (status: string) => `Aucun modèle pour cette espèce (${status})`,
    notModelled: "L'espèce n'a pas été modélisée. Consultez le journal de l'étude pour plus de détails.",
    tabSummary: "Synthèse",
    tabValidation: "Validation",
    tabVariables: "Variables",
    tabData: "Données et exports",
    zoomToResults: "Zoomer sur les résultats",
    compareSpecies: "Comparer les espèces",
    compareNote: "Moyenne ± écart-type entre plis. L'AUC est omise : elle n'est pas comparable d'une espèce à l'autre.",
    cannotLoadRun: "Impossible de charger cette étude",
    notCompleted: "Les résultats sont disponibles une fois l'étude terminée.",
    goToRun: "Voir l'état de l'étude",
    resultsUnavailable: "Résultats indisponibles",
    ruleLabel: ruleBy(FR_RULE),
    ruleInline: (rule: string) => FR_RULE[rule as Rule] ?? rule,
    schemeSpatial: (k: number) => `validation croisée par blocs spatiaux, k = ${k}`,
    schemeOther: (method: string, k: number) =>
      method === "random_kfold" ? `validation croisée aléatoire en k plis, k = ${k}` : `${method.replace(/_/g, " ")}, k = ${k}`,
    schemeGrid: (k: number, km: number) => `validation croisée par blocs spatiaux en grille (blocs de ${km} km), k = ${k}`,
    blockDesign: "Plan des blocs",
    blockSize: "Taille des blocs",
    chosenBy: "Choisie par",
    chosenUser: "réglage de l'étude",
    chosenAuto: "portée médiane d'autocorrélation spatiale des prédicteurs",
    foldsObtained: "Plis obtenus",
    foldsOf: (a: number, r: number) => `${a} sur ${r} demandés`,
    medianRange: "Portée médiane d'autocorrélation",
    clampBand: "Tailles de blocs admises",
    rangesTitle: (n: number) => `Portée d'autocorrélation par prédicteur (${n})`,
    rangesCaption: "Portée d'autocorrélation spatiale par prédicteur",
    variable: "Variable",
    rangeKm: "Portée (km)",
    variogramModel: "Modèle de variogramme",
    variogram: (m: string): string => ({ exponential: "exponentiel", spherical: "sphérique", gaussian: "gaussien" } as Record<string, string>)[m] ?? m,
    atBound: "borne inférieure : limite de recherche atteinte",
    notFitted: "non ajusté (pas de structure spatiale)",
    variogramNote: (n: string, cutoff: string) => `Variogrammes ajustés sur ${n} points de fond, distances jusqu'à ${cutoff} km.`,
    nullTitle: "Modèles nuls (localisations aléatoires)",
    nullLead: (c: number, r: number, n: number) =>
      `${c} modèles ajustés sur ${r}, chacun sur ${n} localisations aléatoires, avec le réglage retenu et la même validation croisée.`,
    nullMetric: "Indicateur (plis mis de côté)",
    nullObserved: "Ce modèle",
    nullMean: "Modèles sur localisations aléatoires (moyenne ± é.-t.)",
    pValue: "p",
    nullBetter: "meilleur que des modèles sur localisations aléatoires",
    nullNotBetter: "pas meilleur que des modèles sur localisations aléatoires",
    nullCaption: "Comparaison avec des modèles ajustés sur des localisations aléatoires",
    nullNote:
      "p est la part des modèles sur localisations aléatoires qui font au moins aussi bien que ce modèle (p ≤ 0,05 : meilleur que des modèles sur localisations aléatoires). Il indique si le modèle fait mieux que le hasard, pas s'il est écologiquement juste ; ce n'est pas un niveau de confiance.",
    aoaOutside: "Hors du domaine d'applicabilité",
    evidenceLevel: "Niveau de preuve",
    ruleBased: "fondé sur des règles, voir les motifs",
    areaTitle: "Surface de conditions favorables",
    noThreshold: "Aucun seuil n'a été calculé pour cette espèce.",
    areaLead: "Surface de conditions favorables : ",
    areaRange: (band: string) => `(fourchette ${band} selon les règles de seuil)`,
    areaFraction: (pct: string, area: string) => `${pct} de la zone d'étude (${area}) · seuil `,
    areaNote:
      "Surface où les conditions sont au moins aussi favorables que le seuil — ce n'est pas l'aire occupée. Elle varie avec la règle de seuil.",
    thresholdRule: "Règle de seuil",
    defaultTag: "(défaut)",
    warnings: (n: number) => `Avertissements (${n}) — à examiner avant utilisation`,
    validation: "Validation",
    cbi: "Indice de Boyce continu",
    cbiTip: "De −1 à 1. Une valeur positive signifie que les prédictions classent les présences observées mieux que le hasard.",
    or10: "Omission au 10e percentile",
    or10Tip:
      "Part des présences mises de côté prédites défavorables au seuil du 10e percentile des présences d'apprentissage. Attendu ≈ 10 %.",
    expectedParen: (pct: string) => `(attendu ${pct})`,
    meanSdFolds: (scheme: string) => `Moyenne ± écart-type entre plis ; ${scheme}.`,
    mapSummary: "Synthèse de la carte",
    studyArea: "Zone d'étude",
    outsideRange: "Hors de la gamme d'apprentissage (MESS < 0)",
    foldSpread: "Dispersion moyenne entre modèles des plis",
    sdUnit: "é.-t.",
    selectedModel: "Modèle retenu",
    mapTextEquivalent: "Équivalent textuel des couches de la carte. Cliquez sur la carte pour lire les valeurs locales.",
    evaluatedWith: (scheme: string) =>
      `Évalué par ${scheme} : chaque pli est prédit par un modèle entraîné sur les autres plis.`,
    primaryMetrics: "Indicateurs principaux (moyenne ± écart-type entre plis)",
    cbiFull: "Indice de Boyce continu (CBI)",
    or10Full: "Omission au 10e percentile (OR10)",
    ormtpFull: "Omission au minimum des présences d'apprentissage",
    expected: (pct: string) => `attendu ${pct}`,
    secondary: "Secondaire",
    aucPb: "AUC présence-fond",
    aucNote:
      "L'AUC présence-fond compare les présences aux points de fond, et non à des absences connues. Elle n'a de sens que pour comparer des modèles de la même espèce sur la même zone d'étude — jamais entre espèces.",
    perFold: "Par pli",
    perFoldCaption: "Indicateurs de validation par pli",
    fold: "Pli",
    testN: "n test",
    aucShort: "AUC (p-f)",
    cbiMissing: "— CBI non calculé pour les plis ayant trop peu de présences de test.",
    tuning: (n: number) => `Calibration (${n} combinaisons)`,
    tuningSelected: (fc: string, beta: number) =>
      `Retenu : classes de variables ${fc}, β = ${beta}. Moyennes entre plis.`,
    tuningCaption: "Grille de calibration",
    features: "Classes",
    coef: "Coef.",
    selectedSr: " (retenu)",
    contributions: "Contributions des variables",
    contributionsNote:
      "Contribution en pourcentage : part du gain d'apprentissage attribuée à chaque variable. Importance par permutation : baisse de l'AUC d'entraînement quand les valeurs de la variable sont permutées au hasard, ramenée à 100 (définitions de MaxEnt 3.4.4). Les variables corrélées se partagent le crédit : une valeur faible ne signifie pas qu'une variable est écologiquement sans importance.",
    noContributions: "Aucune contribution renseignée.",
    percentContribution: "Contribution en pourcentage",
    permutationImportance: "Importance par permutation",
    responseCurves: "Courbes de réponse",
    curvesNote:
      "Idonéité relative (cloglog) quand chaque variable varie, toutes les autres étant fixées à leur moyenne sur les présences. Grisé : valeurs hors des données d'entraînement, où le modèle est borné (clamping) et la courbe est une extrapolation. Avec des variables corrélées, les courbes marginales peuvent masquer l'effet de variables qui varient ensemble.",
    curveSummary: (label: string, lo: string, hi: string, peak: string, tmin: string, tmax: string) =>
      `${label} : idonéité relative de ${lo} à ${hi}, maximale à ${peak} ; données d'entraînement de ${tmin} à ${tmax}.`,
    variablesUsed: (n: number) => `Variables utilisées (${n})`,
    variablesDropped: (n: number) => `Variables écartées (${n})`,
    droppedReason: (r: string, other: string, t: string) => `|r| = ${r} avec ${other} (seuil ${t})`,
    none: "Aucune.",
    data: "Données",
    presences: "Présences (cellules uniques)",
    background: "Points de fond",
    recordsFetched: "Occurrences récupérées",
    source: "Source",
    sourceQuery: "Requête source",
    excludedCaption: "Occurrences exclues",
    exclusionReason: "Motif d'exclusion",
    records: "Occurrences",
    provenance: "Provenance",
    runId: "Identifiant de l'étude",
    created: "Créé",
    finished: "Terminé",
    resolutionFactor: "Facteur de résolution",
    filters: "Filtres",
    filtersValue: (year: number, unc: string, licences: string) => `depuis ${year}, ≤ ${unc} m, ${licences}`,
    tuningLabel: "Calibration",
    tuningValue: (tuning: string, fc: string, betas: string) => `${tuning} ; classes ${fc} ; β ${betas}`,
    collinearity: "Colinéarité |r|",
    seed: "Graine",
    exportsCount: (n: number) => `Exports (${n})`,
    noFiles: "Aucun fichier n'a été produit pour cette espèce.",
    fileDescription: fileText(FR_FILES, SINP_NOTE.fr, FR_FUTURE),
    download: (name: string) => `Télécharger ${name}`,
    openReport: "Ouvrir le rapport imprimable",
    downloadManifest: "Télécharger le manifeste d'exécution",
    hideManifest: "Masquer le manifeste",
    viewManifest: "Afficher le manifeste",
    manifestUnavailable: "Manifeste indisponible",
    manifestJson: "Manifeste d'exécution (JSON)",
    comparisonCaption: "Comparaison entre espèces",
    species: "Espèce",
    evidence: "Preuve",
    areaP10: "Surface (P10)",
    layerSuitability: "Idonéité (relative, 0–1)",
    layerUncertainty: "Incertitude (écart-type entre plis)",
    layerExtrapolation: "Extrapolation (MESS < 0)",
    layerBinary: "Favorable / non favorable au seuil",
    layersLegend: "Couches et légende",
    mapLayer: "Couche",
    opacity: "Opacité",
    hatching: "Hachures d'extrapolation",
    aoaHatching: "Hors du domaine d'applicabilité (hachures)",
    occurrences: "Occurrences",
    noOccurrences: "Les points d'occurrence ne sont pas exportés pour cette étude.",
    clickToRead: "Cliquez sur la carte (ou donnez-lui le focus et appuyez sur I) pour lire les valeurs.",
    layerEffort: "Effort d'échantillonnage (données du groupe cible)",
    methodTitle: "Méthode",
    bgLabel: "Points de fond",
    bgRandom: "aléatoires dans la zone d'étude (pas de correction du biais d'échantillonnage)",
    bgTargetGroup: (cls: string) =>
      `groupe cible ${cls} : tirés dans les cellules ayant des données GBIF de cette classe (correction du biais d'échantillonnage)`,
    bgFallback: (cls: string | null) =>
      `aléatoires dans la zone d'étude : groupe cible ${cls ?? "(classe de l'espèce)"} demandé mais indisponible, donc pas de correction du biais`,
    bgFallbackReason: (reason: string) => `Motif : ${reason}.`,
    bgEligible: (n: string) => `${n} cellules de la zone d'étude ont des données du groupe cible.`,
    thinLabel: "Filtrage spatial par distance",
    thinValue: (km: string) => `cellules de présence distantes d'au moins ${km} km`,
    nullSampling: "Localisations aléatoires tirées",
    nullUniform: "uniformément parmi les cellules ayant des données des prédicteurs",
    nullEffort: "appariées au biais (effort du groupe cible)",
    biasTitle: "Biais d'échantillonnage (effort du groupe cible)",
    biasGroup: "Groupe cible (classe GBIF)",
    biasRecords: "Données du groupe cible",
    biasCells: "Cellules avec données",
    biasCellsValue: (n: string, pct: string) => `${n} (${pct} de la zone d'étude)`,
    biasMaxCell: "Maximum de données dans une cellule",
    biasConcentration: (share: string, top: string) =>
      `${share} des cellules de présence se trouvent dans les ≈ ${top} les plus prospectés de la zone d'étude (≈ ${top} attendus si les présences ne dépendaient pas de l'effort d'observation).`,
    biasDescriptive:
      "Descriptif seulement : ne modifie pas le niveau de preuve. Les décomptes sont des données, pas des visites ni des observateurs.",
    biasSynthetic: "Surface d'effort synthétique (données de démonstration), pas de vraies données GBIF.",
    biasShowMap: "Afficher la carte d'effort",
    generalisedTitle: "Coordonnées généralisées",
    genRecords: "Occurrences généralisées par leur fournisseur",
    genExcluded: "Exclues (précision non indiquée ou au-delà de la limite)",
    genRetained: "Retenues (précision indiquée dans la limite)",
    genFlagsUnavailable:
      "Cette source GBIF n'indique pas la généralisation des coordonnées : les occurrences généralisées dont l'incertitude est faible ou absente ne peuvent pas être détectées.",
    thinFilter: (km: string) => `, filtrage spatial ${km} km`,
  },
  es: {
    // Segundo motor (ADR-0017)
    algorithm: "Algoritmo",
    algorithmName: (a: string): string =>
      a === "rf" ? "Bosque aleatorio submuestreado" : a === "ensemble" ? "Ensamble (MaxEnt + bosque aleatorio)" : "MaxEnt",
    algorithmShort: (a: string): string => (a === "rf" ? "Bosque aleatorio" : a === "ensemble" ? "Ensamble" : "MaxEnt"),
    selectedModelValue: (s: SelectedModel) =>
      s.algorithm === "rf"
        ? `Bosque aleatorio, mtry ${s.rf_mtry ?? "–"}, ${s.rf_trees ?? "–"} árboles`
        : s.algorithm === "ensemble"
          ? `Ensamble: MaxEnt ${s.feature_classes}, β = ${s.beta_multiplier} + bosque aleatorio (mtry ${s.rf_mtry ?? "–"}, ${s.rf_trees ?? "–"} árboles)`
          : `${s.feature_classes}, β = ${s.beta_multiplier}`,
    importanceTitle: "Importancia de las variables",
    rfImportance: "Importancia por permutación (bosque)",
    contributionsNoteRf:
      "Importancia por permutación del bosque aleatorio: caída del AUC presencia–fondo de entrenamiento al mezclar los valores de la variable (3 repeticiones), normalizada a 100. Indica cuánto se apoya el bosque en una variable, no su efecto ecológico; las variables correlacionadas se reparten el mérito.",
    contributionsNoteEnsemble:
      "Miembro MaxEnt: contribución porcentual e importancia por permutación (definiciones de MaxEnt 3.4.4). Miembro bosque aleatorio: importancia por permutación (caída del AUC de entrenamiento al mezclar la variable, normalizada a 100). Las variables correlacionadas se reparten el mérito.",
    responseCurvesMaxent: "Curvas de respuesta (miembro MaxEnt)",
    partialDependence: "Dependencia parcial (bosque aleatorio)",
    pdNote:
      "Idoneidad relativa media (proporción de árboles que votan «presencia») cuando cada variable varía, promediada sobre los valores observados de las demás variables (Friedman 2001). No es una curva de respuesta de MaxEnt: las demás variables no se fijan en su media. Las curvas cubren solo el rango de entrenamiento: los árboles no extrapolan y, fuera de él, las predicciones se quedan en el valor del borde.",
    membersTitle: "El ensamble y sus miembros",
    membersNote:
      "Medias sobre los mismos pliegues. La primera fila es el ensamble evaluado como un único modelo (sus modelos de pliegue combinados); es la que determina el nivel de evidencia.",
    tuningSelectedRf: (mtry: number, trees: number) => `Seleccionado: bosque aleatorio, mtry = ${mtry}, ${trees} árboles. Medias sobre los pliegues.`,
    tuningSelectedEnsemble: (fc: string, beta: number, mtry: number, trees: number) =>
      `Miembros seleccionados: MaxEnt clases ${fc}, β = ${beta}; bosque aleatorio mtry = ${mtry}, ${trees} árboles. Medias sobre los pliegues.`,
    rfSetting: (mtry: number) => `Bosque mtry ${mtry}`,
    setting: "Ajuste",
    rfSettingsLabel: "Bosque aleatorio",
    rfSettingsValue: (trees: number, tune: boolean) => `${trees} árboles; mtry ${tune ? "√p o p/2 (calibrado)" : "√p"}`,
    results: "Resultados",
    noSpeciesResults: "Este estudio no tiene resultados por especie.",
    speciesSwitch: (n: number) => `Especies (${n})`,
    duplicate: "Duplicar y editar",
    exports: "Exportaciones",
    demoNotice:
      "Demostración sintética: estos mapas describen una especie virtual y no deben usarse como evidencia ecológica.",
    noModel: (status: string) => `No hay modelo para esta especie (${status})`,
    notModelled: "La especie no se ha modelizado. Consulte el registro del estudio para más detalles.",
    tabSummary: "Resumen",
    tabValidation: "Validación",
    tabVariables: "Variables",
    tabData: "Datos y exportación",
    zoomToResults: "Acercar a los resultados",
    compareSpecies: "Comparar especies",
    compareNote: "Media ± desviación típica entre pliegues. Se omite el AUC: no es comparable entre especies.",
    cannotLoadRun: "No se puede cargar este estudio",
    notCompleted: "Los resultados estarán disponibles cuando termine el estudio.",
    goToRun: "Ver el estado del estudio",
    resultsUnavailable: "Resultados no disponibles",
    ruleLabel: ruleBy(ES_RULE),
    ruleInline: (rule: string) => ES_RULE[rule as Rule] ?? rule,
    schemeSpatial: (k: number) => `validación cruzada por bloques espaciales, k = ${k}`,
    schemeOther: (method: string, k: number) =>
      method === "random_kfold" ? `validación cruzada aleatoria en k pliegues, k = ${k}` : `${method.replace(/_/g, " ")}, k = ${k}`,
    schemeGrid: (k: number, km: number) => `validación cruzada por bloques espaciales en cuadrícula (bloques de ${km} km), k = ${k}`,
    blockDesign: "Diseño de bloques",
    blockSize: "Tamaño de bloque",
    chosenBy: "Elegido por",
    chosenUser: "ajuste del estudio",
    chosenAuto: "alcance mediano de autocorrelación espacial de los predictores",
    foldsObtained: "Pliegues obtenidos",
    foldsOf: (a: number, r: number) => `${a} de ${r} pedidos`,
    medianRange: "Alcance mediano de autocorrelación",
    clampBand: "Tamaños de bloque admitidos",
    rangesTitle: (n: number) => `Alcance de autocorrelación por predictor (${n})`,
    rangesCaption: "Alcance de autocorrelación espacial por predictor",
    variable: "Variable",
    rangeKm: "Alcance (km)",
    variogramModel: "Modelo de variograma",
    variogram: (m: string): string => ({ exponential: "exponencial", spherical: "esférico", gaussian: "gaussiano" } as Record<string, string>)[m] ?? m,
    atBound: "cota inferior: se alcanzó el límite de búsqueda",
    notFitted: "no ajustado (sin estructura espacial)",
    variogramNote: (n: string, cutoff: string) => `Variogramas ajustados con ${n} puntos de fondo, distancias de hasta ${cutoff} km.`,
    nullTitle: "Modelos nulos (localizaciones aleatorias)",
    nullLead: (c: number, r: number, n: number) =>
      `${c} de ${r} modelos ajustados, cada uno con ${n} localizaciones aleatorias, con el ajuste seleccionado y la misma validación cruzada.`,
    nullMetric: "Indicador (pliegues reservados)",
    nullObserved: "Este modelo",
    nullMean: "Modelos con localizaciones aleatorias (media ± DE)",
    pValue: "p",
    nullBetter: "mejor que los modelos con localizaciones aleatorias",
    nullNotBetter: "no mejor que los modelos con localizaciones aleatorias",
    nullCaption: "Comparación con modelos ajustados con localizaciones aleatorias",
    nullNote:
      "p es la fracción de modelos con localizaciones aleatorias que lo hacen al menos tan bien como este modelo (p ≤ 0,05: mejor que los modelos con localizaciones aleatorias). Indica si el modelo supera al azar, no si es ecológicamente correcto; no es un nivel de confianza.",
    aoaOutside: "Fuera del área de aplicabilidad",
    evidenceLevel: "Nivel de evidencia",
    ruleBased: "basado en reglas, véanse los motivos",
    areaTitle: "Superficie de condiciones favorables",
    noThreshold: "No se ha calculado ningún umbral para esta especie.",
    areaLead: "Superficie de condiciones favorables: ",
    areaRange: (band: string) => `(horquilla ${band} según las reglas de umbral)`,
    areaFraction: (pct: string, area: string) => `${pct} del área de estudio (${area}) · umbral `,
    areaNote:
      "Superficie donde las condiciones son al menos tan favorables como el umbral; no es el área ocupada. Cambia con la regla de umbral.",
    thresholdRule: "Regla de umbral",
    defaultTag: "(por defecto)",
    warnings: (n: number) => `Avisos (${n}): revíselos antes de usar los resultados`,
    validation: "Validación",
    cbi: "Índice de Boyce continuo",
    cbiTip: "De −1 a 1. Los valores positivos indican que las predicciones sitúan las presencias observadas por encima del azar.",
    or10: "Omisión en el percentil 10",
    or10Tip:
      "Fracción de presencias reservadas que se predicen no favorables con el umbral del percentil 10 de las presencias de entrenamiento. Se espera ≈ 10 %.",
    expectedParen: (pct: string) => `(se espera ${pct})`,
    meanSdFolds: (scheme: string) => `Media ± desviación típica entre pliegues; ${scheme}.`,
    mapSummary: "Resumen del mapa",
    studyArea: "Área de estudio",
    outsideRange: "Fuera del rango de entrenamiento (MESS < 0)",
    foldSpread: "Dispersión media entre los modelos de los pliegues",
    sdUnit: "DE",
    selectedModel: "Modelo seleccionado",
    mapTextEquivalent: "Equivalente textual de las capas del mapa. Haga clic en el mapa para leer los valores locales.",
    evaluatedWith: (scheme: string) =>
      `Evaluado con ${scheme}: cada pliegue se predice con un modelo entrenado con los demás pliegues.`,
    primaryMetrics: "Indicadores principales (media ± desviación típica entre pliegues)",
    cbiFull: "Índice de Boyce continuo (CBI)",
    or10Full: "Omisión en el percentil 10 (OR10)",
    ormtpFull: "Omisión en la presencia mínima de entrenamiento",
    expected: (pct: string) => `se espera ${pct}`,
    secondary: "Secundarios",
    aucPb: "AUC presencia-fondo",
    aucNote:
      "El AUC presencia-fondo compara las presencias con puntos de fondo, no con ausencias conocidas. Solo tiene sentido para comparar modelos de la misma especie y la misma área de estudio, nunca entre especies.",
    perFold: "Por pliegue",
    perFoldCaption: "Indicadores de validación por pliegue",
    fold: "Pliegue",
    testN: "n prueba",
    aucShort: "AUC (p-f)",
    cbiMissing: "— CBI no calculado en los pliegues con muy pocas presencias de prueba.",
    tuning: (n: number) => `Calibración (${n} combinaciones)`,
    tuningSelected: (fc: string, beta: number) =>
      `Seleccionado: clases de variables ${fc}, β = ${beta}. Medias entre pliegues.`,
    tuningCaption: "Malla de calibración",
    features: "Clases",
    coef: "Coef.",
    selectedSr: " (seleccionado)",
    contributions: "Contribuciones de las variables",
    contributionsNote:
      "Contribución porcentual: parte de la ganancia de entrenamiento atribuida a cada variable. Importancia por permutación: descenso del AUC de entrenamiento cuando los valores de la variable se permutan al azar, normalizado a 100 (definiciones de MaxEnt 3.4.4). Las variables correlacionadas se reparten el mérito, así que un valor bajo no significa que una variable sea irrelevante desde el punto de vista ecológico.",
    noContributions: "No se han indicado contribuciones.",
    percentContribution: "Contribución porcentual",
    permutationImportance: "Importancia por permutación",
    responseCurves: "Curvas de respuesta",
    curvesNote:
      "Idoneidad relativa (cloglog) al variar cada variable, con las demás fijadas en su media sobre los registros de presencia. Sombreado: valores fuera de los datos de entrenamiento, donde el modelo se acota (clamping) y la curva es una extrapolación. Con variables correlacionadas, las curvas marginales pueden ocultar efectos de variables que cambian a la vez.",
    curveSummary: (label: string, lo: string, hi: string, peak: string, tmin: string, tmax: string) =>
      `${label}: idoneidad relativa de ${lo} a ${hi}, máxima en ${peak}; datos de entrenamiento de ${tmin} a ${tmax}.`,
    variablesUsed: (n: number) => `Variables usadas (${n})`,
    variablesDropped: (n: number) => `Variables descartadas (${n})`,
    droppedReason: (r: string, other: string, t: string) => `|r| = ${r} con ${other} (umbral ${t})`,
    none: "Ninguna.",
    data: "Datos",
    presences: "Presencias (celdas únicas)",
    background: "Puntos de fondo",
    recordsFetched: "Registros obtenidos",
    source: "Fuente",
    sourceQuery: "Consulta de origen",
    excludedCaption: "Registros excluidos",
    exclusionReason: "Motivo de exclusión",
    records: "Registros",
    provenance: "Procedencia",
    runId: "Id. del estudio",
    created: "Creada",
    finished: "Terminada",
    resolutionFactor: "Factor de resolución",
    filters: "Filtros",
    filtersValue: (year: number, unc: string, licences: string) => `desde ${year}, ≤ ${unc} m, ${licences}`,
    tuningLabel: "Calibración",
    tuningValue: (tuning: string, fc: string, betas: string) => `${tuning}; clases ${fc}; β ${betas}`,
    collinearity: "Colinealidad |r|",
    seed: "Semilla",
    exportsCount: (n: number) => `Exportaciones (${n})`,
    noFiles: "No se ha generado ningún archivo para esta especie.",
    fileDescription: fileText(ES_FILES, SINP_NOTE.es, ES_FUTURE),
    download: (name: string) => `Descargar ${name}`,
    openReport: "Abrir el informe imprimible",
    downloadManifest: "Descargar el manifiesto de ejecución",
    hideManifest: "Ocultar el manifiesto",
    viewManifest: "Ver el manifiesto",
    manifestUnavailable: "Manifiesto no disponible",
    manifestJson: "Manifiesto de ejecución (JSON)",
    comparisonCaption: "Comparación entre especies",
    species: "Especie",
    evidence: "Evidencia",
    areaP10: "Superficie (P10)",
    layerSuitability: "Idoneidad (relativa, 0–1)",
    layerUncertainty: "Incertidumbre (DE entre pliegues)",
    layerExtrapolation: "Extrapolación (MESS < 0)",
    layerBinary: "Favorable o no según el umbral",
    layersLegend: "Capas y leyenda",
    mapLayer: "Capa",
    opacity: "Opacidad",
    hatching: "Rayado de extrapolación",
    aoaHatching: "Fuera del área de aplicabilidad (rayado)",
    occurrences: "Registros de presencia",
    noOccurrences: "Los puntos de presencia no se exportan en este estudio.",
    clickToRead: "Haga clic en el mapa (o dele el foco y pulse I) para leer los valores.",
    layerEffort: "Esfuerzo de muestreo (registros del grupo objetivo)",
    methodTitle: "Método",
    bgLabel: "Puntos de fondo",
    bgRandom: "aleatorios en el área de estudio (sin corrección del sesgo de muestreo)",
    bgTargetGroup: (cls: string) =>
      `grupo objetivo ${cls}: tomados en celdas con registros de GBIF de esta clase (corrección del sesgo de muestreo)`,
    bgFallback: (cls: string | null) =>
      `aleatorios en el área de estudio: se pidió el grupo objetivo ${cls ?? "(clase de la especie)"} pero no estaba disponible, así que no hay corrección del sesgo`,
    bgFallbackReason: (reason: string) => `Motivo: ${reason}.`,
    bgEligible: (n: string) => `${n} celdas del área de estudio tienen registros del grupo objetivo.`,
    thinLabel: "Filtrado espacial por distancia",
    thinValue: (km: string) => `celdas de presencia separadas al menos ${km} km`,
    nullSampling: "Localizaciones aleatorias tomadas",
    nullUniform: "uniformemente entre las celdas con datos de los predictores",
    nullEffort: "emparejadas con el sesgo (esfuerzo del grupo objetivo)",
    biasTitle: "Sesgo de muestreo (esfuerzo del grupo objetivo)",
    biasGroup: "Grupo objetivo (clase de GBIF)",
    biasRecords: "Registros del grupo objetivo",
    biasCells: "Celdas con registros",
    biasCellsValue: (n: string, pct: string) => `${n} (${pct} del área de estudio)`,
    biasMaxCell: "Máximo de registros en una celda",
    biasConcentration: (share: string, top: string) =>
      `El ${share} de las celdas de presencia está en el ≈ ${top} más muestreado del área de estudio (≈ ${top} esperado si las presencias no dependieran del esfuerzo de observación).`,
    biasDescriptive:
      "Solo descriptivo: no cambia el nivel de evidencia. Los recuentos son registros, no visitas ni observadores.",
    biasSynthetic: "Superficie de esfuerzo sintética (datos de demostración), no son registros reales de GBIF.",
    biasShowMap: "Mostrar el mapa de esfuerzo",
    generalisedTitle: "Coordenadas generalizadas",
    genRecords: "Registros generalizados por su publicador",
    genExcluded: "Excluidos (precisión no indicada o por encima del límite)",
    genRetained: "Conservados (precisión indicada dentro del límite)",
    genFlagsUnavailable:
      "Esta fuente de GBIF no indica la generalización de coordenadas: no se pueden detectar los registros generalizados con incertidumbre pequeña o ausente.",
    thinFilter: (km: string) => `, filtrado espacial ${km} km`,
  },
});
