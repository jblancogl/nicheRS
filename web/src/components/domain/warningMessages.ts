// Data-quality and validation warnings worded in the interface language from their code and
// parameters (`Warning.params`). Unknown codes, or warnings stored before parameters existed,
// keep the server's English text.

import type { Warning } from "@/api/types";
import { useLang, type Lang } from "@/i18n/lang";
import { formatCount, formatDecimal, formatPercent } from "@/lib/format";

import { fallbackText } from "./serverText";

type P = Record<string, string | number | boolean>;
/** Words of a warning from its parameters; `server` is the server's English message. */
type Words = Record<string, (p: P, server: string) => string | null>;

const num = (p: P, k: string) => (typeof p[k] === "number" ? formatCount(p[k]) : null);
/** Decimal parameter (p-values): never rounded to a whole number. */
const dec = (p: P, k: string, digits = 3) => (typeof p[k] === "number" ? formatDecimal(p[k], digits) : null);
/** Share parameter (fraction 0–1) as a percentage. */
const pct = (p: P, k: string) => (typeof p[k] === "number" ? formatPercent(p[k]) : null);
const str = (p: P, k: string) => (typeof p[k] === "string" && p[k] !== "" ? p[k] : null);
/** The effort surface of the demo is synthetic (param `synthetic`; older results: in the message only). */
const syntheticEffort = (server: string, p: P) => p.synthetic === true || server.includes("(synthetic effort)");

const RANKS: Record<"fr" | "es", Record<string, string>> = {
  fr: { genus: "genre", family: "famille", order: "ordre", class: "classe", phylum: "embranchement", kingdom: "règne", tribe: "tribu", section: "section" },
  es: { genus: "género", family: "familia", order: "orden", class: "clase", phylum: "filo", kingdom: "reino", tribe: "tribu", section: "sección" },
};

const en: Words = {
  pooled_taxa: (p) =>
    p.rank
      ? `The selected taxon is a ${p.rank}: records of several species with different ecological niches are pooled into one model. Select a single species (ADR-0009).`
      : null,
  synthetic: () => "Synthetic demonstration data: a virtual species on artificial predictors. Results illustrate the workflow only.",
  truncated: (p) =>
    num(p, "total") && num(p, "fetched")
      ? `${num(p, "total")} records match; ${num(p, "fetched")} were retrieved as a spatially stratified subsample. For final reports use a GBIF download (DOI).`
      : null,
  too_few_presences: (p) =>
    num(p, "n") && num(p, "min") ? `${num(p, "n")} usable presence cells; at least ${num(p, "min")} are needed to fit a model.` : null,
  few_presences: (p) => (num(p, "n") ? `${num(p, "n")} usable presence cells: results will be exploratory at best (≥ 25 recommended).` : null),
  non_commercial: (p) => (num(p, "n") ? `${num(p, "n")} records are CC BY-NC (excluded by default for commercial projects).` : null),
  sampling_bias: (p) =>
    p.method === "target_group"
      ? `Occurrence records come from opportunistic surveys; they reflect observer effort as well as habitat. Bias correction: background drawn from cells with records of the target group ${str(p, "class") ?? "—"} (Phillips et al. 2009), so presences and background share the observers' bias; this corrects only the part of the bias the target group shares with the species.`
      : "Occurrence records come from opportunistic surveys; they reflect observer effort as well as habitat. No bias correction is applied.",
  generalised_records: (p) =>
    num(p, "n") && num(p, "excluded") && num(p, "retained") && num(p, "max_m")
      ? `${num(p, "n")} records were generalised by their publishers (sensitive species or observer request): ${num(p, "excluded")} excluded, ${num(p, "retained")} retained with stated precision ≤ ${num(p, "max_m")} m. Retained presences may under-represent the most sensitive sites.`
      : null,
  generalisation_flags_unavailable: () =>
    "This GBIF download format does not report coordinate generalisation; generalised records with small or missing uncertainty cannot be detected.",
  target_group_unavailable: (p) =>
    str(p, "reason")
      ? `Target-group background not available: ${str(p, "reason")}. The background was drawn at random over the study area, as by default, so no bias correction was applied.`
      : null,
  target_group_background_smaller: (p) =>
    num(p, "eligible") && num(p, "requested") && str(p, "class")
      ? `Only ${num(p, "eligible")} cells of the study area have records of the target group ${str(p, "class")}: the background has fewer cells than the ${num(p, "requested")} requested.`
      : null,
  effort_concentration: (p, server) =>
    pct(p, "share") && pct(p, "expected") && str(p, "class")
      ? `${pct(p, "share")} of the presence cells lie in the ${pct(p, "expected")} of the study area with most records of the target group ${str(p, "class")}${syntheticEffort(server, p) ? " (synthetic effort)" : ""} (${pct(p, "expected")} expected if presences ignored observer effort). Sampling diagnostic only; it does not change the evidence level.`
      : null,
  own_data_concentrated: (p) =>
    pct(p, "share")
      ? `The organisation's presence cells span ${pct(p, "share")} of the study area's extent: field data are often grouped (transects, study sites). Areas that were not surveyed are not absences.`
      : null,
  many_rejected: (p) =>
    num(p, "rejected") && num(p, "rows") ? `${num(p, "rejected")} of ${num(p, "rows")} rows were rejected: check the columns and the coordinate system chosen.` : null,
  axes_swapped: () => "Longitudes are all between 27 and 72 and latitudes between −32 and 45: the longitude and latitude columns may be swapped.",
  no_absences: () => "A status column was chosen but no row has one of the values marked as absent: every record is a presence.",
  random_cv: () =>
    "Random k-fold cross-validation overestimates performance under spatial autocorrelation (Roberts et al. 2017); prefer spatial blocks.",
  small_fold: (p) => (num(p, "fold") && num(p, "n") ? `Fold ${num(p, "fold")} has only ${num(p, "n")} test presences; its metrics are unstable.` : null),
  extrapolation: (p) =>
    num(p, "percent")
      ? `${num(p, "percent")} % of the study area has environmental conditions outside the training range (MESS < 0); predictions there are extrapolations.`
      : null,
  aoa_outside: (p) =>
    num(p, "percent")
      ? `${num(p, "percent")} % of the study area is outside the area of applicability (dissimilarity to the training data above the threshold); predictions there are less reliable.`
      : null,
  fewer_folds: (p) =>
    num(p, "requested") && num(p, "achieved")
      ? `Spatial grid blocks gave ${num(p, "achieved")} folds instead of the ${num(p, "requested")} requested: too few blocks contain presences. Use smaller blocks or fewer folds.`
      : null,
  block_size_fallback: (p) =>
    num(p, "block_km")
      ? `The spatial autocorrelation range of the predictors could not be estimated; blocks of ${num(p, "block_km")} km (one fifth of the study-area extent) were used.`
      : null,
  block_size_clamped: (p) =>
    num(p, "median_km") && num(p, "block_km")
      ? `The median autocorrelation range of the predictors (${num(p, "median_km")} km) was clamped to ${num(p, "block_km")} km to keep enough blocks for cross-validation.`
      : null,
  null_models_incomplete: (p) =>
    num(p, "completed") && num(p, "requested")
      ? `Only ${num(p, "completed")} of ${num(p, "requested")} null models could be fitted; the significance test uses those.`
      : null,
  not_better_than_null: (p) =>
    dec(p, "p_value")
      ? `The model is not better than models fitted to random locations (Boyce index, p = ${dec(p, "p_value")}); the evidence level is capped at Exploratory.`
      : null,
  land_cover_temporal: () =>
    "Land-cover predictors describe 2021 (ESA WorldCover); the climate baseline is 1981–2010 and records may be older. Where land use changed, older records are described by today's land cover.",
  built_up_bias: () =>
    "Built-up fraction is used: it is strongly correlated with observer effort in opportunistic records, so a positive response may reflect where people look rather than the species.",
  land_cover_composition: () =>
    "All eight land-cover fractions are used: they sum to 1 in every cell, an exact linear dependency that the pairwise correlation filter does not detect. Leave out at least one group (usually the least relevant to the species).",
  surface_model: () =>
    "Terrain predictors come from Copernicus DEM, a surface model: heights include forest canopy and buildings.",
};

const fr: Words = {
  pooled_taxa: (p) =>
    p.rank
      ? `Le taxon choisi est de rang ${RANKS.fr[String(p.rank)] ?? p.rank} : les données de plusieurs espèces aux niches écologiques différentes sont réunies dans un seul modèle. Choisissez une seule espèce (ADR-0009).`
      : null,
  synthetic: () => "Données de démonstration synthétiques : une espèce virtuelle sur des prédicteurs artificiels. Les résultats illustrent seulement la démarche.",
  truncated: (p) =>
    num(p, "total") && num(p, "fetched")
      ? `${num(p, "total")} occurrences correspondent ; ${num(p, "fetched")} ont été récupérées en sous-échantillon stratifié dans l'espace. Pour un rapport final, utilisez un téléchargement GBIF (DOI).`
      : null,
  too_few_presences: (p) =>
    num(p, "n") && num(p, "min")
      ? `${num(p, "n")} cellules de présence utilisables ; il en faut au moins ${num(p, "min")} pour ajuster un modèle.`
      : null,
  few_presences: (p) =>
    num(p, "n") ? `${num(p, "n")} cellules de présence utilisables : les résultats seront au mieux exploratoires (≥ 25 recommandées).` : null,
  non_commercial: (p) => (num(p, "n") ? `${num(p, "n")} occurrences sont sous CC BY-NC (exclues par défaut pour les projets commerciaux).` : null),
  sampling_bias: (p) =>
    p.method === "target_group"
      ? `Les occurrences proviennent d'observations opportunistes : elles reflètent l'effort d'observation autant que l'habitat. Correction du biais : points de fond tirés dans les cellules ayant des données du groupe cible ${str(p, "class") ?? "—"} (Phillips et al. 2009), pour que présences et fond partagent le biais des observateurs ; seule la part du biais commune au groupe cible et à l'espèce est corrigée.`
      : "Les occurrences proviennent d'observations opportunistes : elles reflètent l'effort d'observation autant que l'habitat. Aucune correction de biais n'est appliquée.",
  generalised_records: (p) =>
    num(p, "n") && num(p, "excluded") && num(p, "retained") && num(p, "max_m")
      ? `${num(p, "n")} occurrences ont des coordonnées généralisées par leur fournisseur (espèce sensible ou demande de l'observateur) : ${num(p, "excluded")} exclues, ${num(p, "retained")} retenues avec une précision indiquée ≤ ${num(p, "max_m")} m. Les présences retenues peuvent sous-représenter les sites les plus sensibles.`
      : null,
  generalisation_flags_unavailable: () =>
    "Ce format de téléchargement GBIF n'indique pas la généralisation des coordonnées : les occurrences généralisées dont l'incertitude est faible ou absente ne peuvent pas être détectées.",
  target_group_unavailable: (p) =>
    str(p, "reason")
      ? `Fond par groupe cible indisponible : ${fallbackText(String(p.reason), "fr")}. Les points de fond ont été tirés au hasard dans la zone d'étude, comme par défaut : aucune correction de biais n'a été appliquée.`
      : null,
  target_group_background_smaller: (p) =>
    num(p, "eligible") && num(p, "requested") && str(p, "class")
      ? `Seules ${num(p, "eligible")} cellules de la zone d'étude ont des données du groupe cible ${str(p, "class")} : le fond compte moins de cellules que les ${num(p, "requested")} demandées.`
      : null,
  effort_concentration: (p, server) =>
    pct(p, "share") && pct(p, "expected") && str(p, "class")
      ? `${pct(p, "share")} des cellules de présence se trouvent dans les ${pct(p, "expected")} de la zone d'étude qui ont le plus de données du groupe cible ${str(p, "class")}${syntheticEffort(server, p) ? " (effort synthétique)" : ""} (${pct(p, "expected")} attendus si les présences ne dépendaient pas de l'effort d'observation). Diagnostic d'échantillonnage seulement ; il ne modifie pas le niveau de preuve.`
      : null,
  own_data_concentrated: (p) =>
    pct(p, "share")
      ? `Les cellules de présence de l'organisation couvrent ${pct(p, "share")} de l'emprise de la zone d'étude : les données de terrain sont souvent groupées (transects, sites d'étude). Les zones non prospectées ne sont pas des absences.`
      : null,
  many_rejected: (p) =>
    num(p, "rejected") && num(p, "rows") ? `${num(p, "rejected")} lignes sur ${num(p, "rows")} ont été rejetées : vérifiez les colonnes et le système de coordonnées choisis.` : null,
  axes_swapped: () => "Les longitudes sont toutes entre 27 et 72 et les latitudes entre −32 et 45 : les colonnes longitude et latitude sont peut-être inversées.",
  no_absences: () => "Une colonne de statut a été choisie mais aucune ligne n'a une valeur marquée comme absence : toutes les données sont des présences.",
  random_cv: () =>
    "La validation croisée aléatoire en k blocs surestime les performances en présence d'autocorrélation spatiale (Roberts et al. 2017) ; préférez les blocs spatiaux.",
  small_fold: (p) =>
    num(p, "fold") && num(p, "n") ? `Le bloc ${num(p, "fold")} n'a que ${num(p, "n")} présences de test ; ses métriques sont instables.` : null,
  extrapolation: (p) =>
    num(p, "percent")
      ? `${num(p, "percent")} % de la zone d'étude présente des conditions environnementales hors de la plage d'apprentissage (MESS < 0) ; les prédictions y sont des extrapolations.`
      : null,
  aoa_outside: (p) =>
    num(p, "percent")
      ? `${num(p, "percent")} % de la zone d'étude est hors du domaine d'applicabilité (dissimilarité aux données d'apprentissage au-dessus du seuil) ; les prédictions y sont moins fiables.`
      : null,
  fewer_folds: (p) =>
    num(p, "requested") && num(p, "achieved")
      ? `Les blocs spatiaux en grille ont donné ${num(p, "achieved")} plis au lieu des ${num(p, "requested")} demandés : trop peu de blocs contiennent des présences. Utilisez des blocs plus petits ou moins de plis.`
      : null,
  block_size_fallback: (p) =>
    num(p, "block_km")
      ? `La portée d'autocorrélation spatiale des prédicteurs n'a pas pu être estimée ; des blocs de ${num(p, "block_km")} km (un cinquième de l'étendue de la zone d'étude) ont été utilisés.`
      : null,
  block_size_clamped: (p) =>
    num(p, "median_km") && num(p, "block_km")
      ? `La portée d'autocorrélation médiane des prédicteurs (${num(p, "median_km")} km) a été ramenée à ${num(p, "block_km")} km pour garder assez de blocs pour la validation croisée.`
      : null,
  null_models_incomplete: (p) =>
    num(p, "completed") && num(p, "requested")
      ? `Seuls ${num(p, "completed")} modèles nuls sur ${num(p, "requested")} ont pu être ajustés ; le test de significativité utilise ceux-là.`
      : null,
  not_better_than_null: (p) =>
    dec(p, "p_value")
      ? `Le modèle ne fait pas mieux que des modèles ajustés sur des localisations aléatoires (indice de Boyce, p = ${dec(p, "p_value")}) ; le niveau de preuve est plafonné à Exploratory.`
      : null,
  land_cover_temporal: () =>
    "Les prédicteurs d'occupation du sol décrivent 2021 (ESA WorldCover) ; la normale climatique est 1981–2010 et des données peuvent être plus anciennes. Là où l'usage du sol a changé, les données anciennes sont décrites par l'occupation actuelle.",
  built_up_bias: () =>
    "La proportion de bâti est utilisée : elle est fortement corrélée à l'effort d'observation des données opportunistes, si bien qu'une réponse positive peut refléter les lieux d'observation plutôt que l'espèce.",
  land_cover_composition: () =>
    "Les huit proportions d'occupation du sol sont utilisées : elles font 1 dans chaque cellule, une dépendance linéaire exacte que le filtre de corrélation par paires ne détecte pas. Retirez au moins un groupe (en général le moins pertinent pour l'espèce).",
  surface_model: () =>
    "Les prédicteurs de relief viennent du Copernicus DEM, un modèle de surface : les altitudes incluent la canopée et les bâtiments.",
};

const es: Words = {
  pooled_taxa: (p) =>
    p.rank
      ? `El taxón elegido tiene rango de ${RANKS.es[String(p.rank)] ?? p.rank}: se reúnen en un solo modelo registros de varias especies con nichos ecológicos distintos. Elija una sola especie (ADR-0009).`
      : null,
  synthetic: () => "Datos de demostración sintéticos: una especie virtual sobre predictores artificiales. Los resultados solo ilustran el procedimiento.",
  truncated: (p) =>
    num(p, "total") && num(p, "fetched")
      ? `${num(p, "total")} registros coinciden; se obtuvieron ${num(p, "fetched")} como submuestra estratificada en el espacio. Para informes finales use una descarga de GBIF (DOI).`
      : null,
  too_few_presences: (p) =>
    num(p, "n") && num(p, "min") ? `${num(p, "n")} celdas de presencia utilizables; se necesitan al menos ${num(p, "min")} para ajustar un modelo.` : null,
  few_presences: (p) =>
    num(p, "n") ? `${num(p, "n")} celdas de presencia utilizables: los resultados serán como mucho exploratorios (se recomiendan ≥ 25).` : null,
  non_commercial: (p) => (num(p, "n") ? `${num(p, "n")} registros son CC BY-NC (excluidos por defecto en proyectos comerciales).` : null),
  sampling_bias: (p) =>
    p.method === "target_group"
      ? `Los registros proceden de observaciones oportunistas: reflejan el esfuerzo de observación tanto como el hábitat. Corrección del sesgo: puntos de fondo tomados en celdas con registros del grupo objetivo ${str(p, "class") ?? "—"} (Phillips et al. 2009), para que presencias y fondo compartan el sesgo de los observadores; solo se corrige la parte del sesgo que el grupo objetivo comparte con la especie.`
      : "Los registros proceden de observaciones oportunistas: reflejan el esfuerzo de observación tanto como el hábitat. No se aplica corrección del sesgo.",
  generalised_records: (p) =>
    num(p, "n") && num(p, "excluded") && num(p, "retained") && num(p, "max_m")
      ? `${num(p, "n")} registros tienen coordenadas generalizadas por su publicador (especie sensible o petición del observador): ${num(p, "excluded")} excluidos, ${num(p, "retained")} conservados con precisión indicada ≤ ${num(p, "max_m")} m. Las presencias conservadas pueden infrarrepresentar los sitios más sensibles.`
      : null,
  generalisation_flags_unavailable: () =>
    "Este formato de descarga de GBIF no indica la generalización de coordenadas: no se pueden detectar los registros generalizados con incertidumbre pequeña o ausente.",
  target_group_unavailable: (p) =>
    str(p, "reason")
      ? `Fondo por grupo objetivo no disponible: ${fallbackText(String(p.reason), "es")}. Los puntos de fondo se tomaron al azar en el área de estudio, como por defecto: no se aplicó ninguna corrección del sesgo.`
      : null,
  target_group_background_smaller: (p) =>
    num(p, "eligible") && num(p, "requested") && str(p, "class")
      ? `Solo ${num(p, "eligible")} celdas del área de estudio tienen registros del grupo objetivo ${str(p, "class")}: el fondo tiene menos celdas que las ${num(p, "requested")} pedidas.`
      : null,
  effort_concentration: (p, server) =>
    pct(p, "share") && pct(p, "expected") && str(p, "class")
      ? `El ${pct(p, "share")} de las celdas de presencia está en el ${pct(p, "expected")} del área de estudio con más registros del grupo objetivo ${str(p, "class")}${syntheticEffort(server, p) ? " (esfuerzo sintético)" : ""} (${pct(p, "expected")} esperado si las presencias no dependieran del esfuerzo de observación). Solo es un diagnóstico del muestreo; no cambia el nivel de evidencia.`
      : null,
  own_data_concentrated: (p) =>
    pct(p, "share")
      ? `Las celdas de presencia de la organización abarcan el ${pct(p, "share")} de la extensión del área de estudio: los datos de campo suelen estar agrupados (transectos, sitios de estudio). Las zonas no prospectadas no son ausencias.`
      : null,
  many_rejected: (p) =>
    num(p, "rejected") && num(p, "rows") ? `Se rechazaron ${num(p, "rejected")} de ${num(p, "rows")} filas: revise las columnas y el sistema de coordenadas elegidos.` : null,
  axes_swapped: () => "Todas las longitudes están entre 27 y 72 y las latitudes entre −32 y 45: puede que las columnas de longitud y latitud estén intercambiadas.",
  no_absences: () => "Se eligió una columna de estado pero ninguna fila tiene un valor marcado como ausencia: todos los registros son presencias.",
  random_cv: () =>
    "La validación cruzada aleatoria en k grupos sobrestima el rendimiento cuando hay autocorrelación espacial (Roberts et al. 2017); son preferibles los bloques espaciales.",
  small_fold: (p) =>
    num(p, "fold") && num(p, "n") ? `El bloque ${num(p, "fold")} solo tiene ${num(p, "n")} presencias de prueba; sus métricas son inestables.` : null,
  extrapolation: (p) =>
    num(p, "percent")
      ? `El ${num(p, "percent")} % del área de estudio tiene condiciones ambientales fuera del rango de entrenamiento (MESS < 0); allí las predicciones son extrapolaciones.`
      : null,
  aoa_outside: (p) =>
    num(p, "percent")
      ? `El ${num(p, "percent")} % del área de estudio está fuera del área de aplicabilidad (disimilitud con los datos de entrenamiento por encima del umbral); allí las predicciones son menos fiables.`
      : null,
  fewer_folds: (p) =>
    num(p, "requested") && num(p, "achieved")
      ? `Los bloques espaciales en cuadrícula dieron ${num(p, "achieved")} pliegues en lugar de los ${num(p, "requested")} pedidos: muy pocos bloques contienen presencias. Use bloques más pequeños o menos pliegues.`
      : null,
  block_size_fallback: (p) =>
    num(p, "block_km")
      ? `No se pudo estimar el alcance de la autocorrelación espacial de los predictores; se usaron bloques de ${num(p, "block_km")} km (una quinta parte de la extensión del área de estudio).`
      : null,
  block_size_clamped: (p) =>
    num(p, "median_km") && num(p, "block_km")
      ? `El alcance mediano de autocorrelación de los predictores (${num(p, "median_km")} km) se limitó a ${num(p, "block_km")} km para conservar bloques suficientes para la validación cruzada.`
      : null,
  null_models_incomplete: (p) =>
    num(p, "completed") && num(p, "requested")
      ? `Solo se pudieron ajustar ${num(p, "completed")} de ${num(p, "requested")} modelos nulos; la prueba de significación usa esos.`
      : null,
  not_better_than_null: (p) =>
    dec(p, "p_value")
      ? `El modelo no es mejor que los modelos ajustados con localizaciones aleatorias (índice de Boyce, p = ${dec(p, "p_value")}); el nivel de evidencia se limita a Exploratory.`
      : null,
  land_cover_temporal: () =>
    "Los predictores de cobertura del suelo describen 2021 (ESA WorldCover); la climatología es 1981–2010 y los registros pueden ser más antiguos. Donde el uso del suelo cambió, los registros antiguos quedan descritos por la cobertura actual.",
  built_up_bias: () =>
    "Se usa la proporción edificada: está muy correlacionada con el esfuerzo de observación de los registros oportunistas, así que una respuesta positiva puede reflejar dónde se observa más que a la especie.",
  land_cover_composition: () =>
    "Se usan las ocho proporciones de cobertura del suelo: suman 1 en cada celda, una dependencia lineal exacta que el filtro de correlación por pares no detecta. Quite al menos un grupo (normalmente el menos pertinente para la especie).",
  surface_model: () =>
    "Los predictores de relieve proceden del Copernicus DEM, un modelo de superficie: las altitudes incluyen el dosel forestal y los edificios.",
};

// Future-climate projections (ADR-0016).
const SSP: Record<string, [string, string, string]> = {
  ssp126: ["SSP1-2.6 (low emissions)", "SSP1-2.6 (émissions faibles)", "SSP1-2.6 (emisiones bajas)"],
  ssp370: ["SSP3-7.0 (high emissions)", "SSP3-7.0 (émissions élevées)", "SSP3-7.0 (emisiones altas)"],
  ssp585: ["SSP5-8.5 (very high emissions)", "SSP5-8.5 (émissions très élevées)", "SSP5-8.5 (emisiones muy altas)"],
};
const scen = (p: P, i: 0 | 1 | 2) => {
  const period = str(p, "period");
  const ssp = str(p, "ssp");
  const label = ssp ? SSP[ssp]?.[i] : undefined;
  return period && label ? `${period} ${label}` : null;
};
Object.assign(en, {
  static_predictors_in_projection: (p: P) =>
    str(p, "variables")
      ? `In the future projections only climate changes: ${str(p, "variables")} stay at their present values (no land-use or land-cover change is modelled).`
      : null,
  projection_without_climate: () =>
    "No climate predictor was kept in the model (collinearity filter), so the future projections equal the present map.",
  projection_extrapolation: (p: P) =>
    scen(p, 0) && num(p, "percent")
      ? `${scen(p, 0)}: ${num(p, "percent")} % of the projected area has future conditions outside the training range under at least one climate model (MESS < 0); the projection there is an extrapolation.`
      : null,
  projection_missing_cells: (p: P) =>
    scen(p, 0) && num(p, "cells")
      ? `${scen(p, 0)}: ${num(p, "cells")} cells with present data have no future climate value under at least one model; they are left out of the projection maps and areas.`
      : null,
  projection_not_computed: () => "No model was fitted for this species, so no future projection was made.",
} satisfies Words);
Object.assign(fr, {
  static_predictors_in_projection: (p: P) =>
    str(p, "variables")
      ? `Dans les projections futures, seul le climat change : ${str(p, "variables")} gardent leurs valeurs actuelles (aucun changement d'usage ou d'occupation du sol n'est modélisé).`
      : null,
  projection_without_climate: () =>
    "Aucun prédicteur climatique n'a été conservé dans le modèle (filtre de colinéarité) : les projections futures sont identiques à la carte actuelle.",
  projection_extrapolation: (p: P) =>
    scen(p, 1) && num(p, "percent")
      ? `${scen(p, 1)} : ${num(p, "percent")} % de la surface projetée a des conditions futures hors de l'étendue d'apprentissage sous au moins un modèle climatique (MESS < 0) ; la projection y est une extrapolation.`
      : null,
  projection_missing_cells: (p: P) =>
    scen(p, 1) && num(p, "cells")
      ? `${scen(p, 1)} : ${num(p, "cells")} cellules avec des données actuelles n'ont pas de valeur climatique future sous au moins un modèle ; elles sont exclues des cartes et des surfaces projetées.`
      : null,
  projection_not_computed: () => "Aucun modèle n'a été ajusté pour cette espèce : aucune projection future n'a été faite.",
} satisfies Words);
Object.assign(es, {
  static_predictors_in_projection: (p: P) =>
    str(p, "variables")
      ? `En las proyecciones futuras solo cambia el clima: ${str(p, "variables")} conservan sus valores actuales (no se modela ningún cambio de uso o cobertura del suelo).`
      : null,
  projection_without_climate: () =>
    "No se conservó ningún predictor climático en el modelo (filtro de colinealidad): las proyecciones futuras son iguales al mapa actual.",
  projection_extrapolation: (p: P) =>
    scen(p, 2) && num(p, "percent")
      ? `${scen(p, 2)}: el ${num(p, "percent")} % de la superficie proyectada tiene condiciones futuras fuera del rango de entrenamiento en al menos un modelo climático (MESS < 0); ahí la proyección es una extrapolación.`
      : null,
  projection_missing_cells: (p: P) =>
    scen(p, 2) && num(p, "cells")
      ? `${scen(p, 2)}: ${num(p, "cells")} celdas con datos actuales no tienen valor climático futuro en al menos un modelo; se excluyen de los mapas y superficies proyectados.`
      : null,
  projection_not_computed: () => "No se ajustó ningún modelo para esta especie, así que no se hizo ninguna proyección futura.",
} satisfies Words);

// Decision-support analyses (ADR-0018).
Object.assign(en, {
  priority_distance_proxy: () =>
    "No sampling-effort surface for this study (target-group background not used): survey priority uses the distance to the nearest known record of the species as a proxy. Distance to the species' own records is not sampling effort: areas far from records may have been surveyed without finding it.",
  priority_no_candidate: () =>
    "No cell combines suitability at or above the P10 threshold with little sampling among the highest priority scores: no candidate area to list.",
  footprint_smaller_than_cell: (p: P) =>
    dec(p, "area_km2", 2) && dec(p, "cell_km2", 2) && num(p, "cells")
      ? `The footprint (${dec(p, "area_km2", 2)} km²) is smaller than one model cell (${dec(p, "cell_km2", 2)} km²): the values describe the ${num(p, "cells")} cell(s) it touches, not conditions within the footprint itself.`
      : null,
  footprint_outside_study_area: (p: P) =>
    dec(p, "percent", 1)
      ? `${dec(p, "percent", 1)} % of the footprint lies outside the study area or over cells without predictions (sea, missing predictor data): shares are of the part with predictions.`
      : null,
  footprint_extrapolation: () =>
    "For at least one species the model extrapolates (MESS < 0) over part of the footprint: values there are less reliable.",
  stacking_overprediction: () =>
    "Stacked species maps overpredict the number of species present in a place: each map shows where conditions are suitable, not where the species occurs, and the species are not assumed to co-occur. Read the count as \"species with suitable conditions\", not as species richness on the ground (Calabrese et al. 2014; Guisan & Rahbek 2011).",
  species_below_evidence_level: (p: P) =>
    num(p, "count") && str(p, "level")
      ? `${num(p, "count")} species left out because their evidence level is below ${str(p, "level")} (Exploratory models are excluded by default).`
      : null,
} satisfies Words);
Object.assign(fr, {
  priority_distance_proxy: () =>
    "Pas de surface d'effort d'échantillonnage pour cette étude (fond par groupe cible non utilisé) : la priorité de prospection utilise comme indicateur la distance à l'occurrence connue la plus proche de l'espèce. La distance aux occurrences de l'espèce n'est pas l'effort d'échantillonnage : des zones éloignées des occurrences peuvent avoir été prospectées sans la trouver.",
  priority_no_candidate: () =>
    "Aucune cellule ne combine une idonéité au moins égale au seuil P10 et un faible échantillonnage parmi les scores de priorité les plus élevés : aucune zone candidate à lister.",
  footprint_smaller_than_cell: (p: P) =>
    dec(p, "area_km2", 2) && dec(p, "cell_km2", 2) && num(p, "cells")
      ? `L'emprise (${dec(p, "area_km2", 2)} km²) est plus petite qu'une cellule du modèle (${dec(p, "cell_km2", 2)} km²) : les valeurs décrivent la ou les ${num(p, "cells")} cellule(s) qu'elle touche, pas les conditions à l'intérieur de l'emprise elle-même.`
      : null,
  footprint_outside_study_area: (p: P) =>
    dec(p, "percent", 1)
      ? `${dec(p, "percent", 1)} % de l'emprise est hors de la zone d'étude ou sur des cellules sans prédiction (mer, prédicteurs manquants) : les parts portent sur la partie avec prédictions.`
      : null,
  footprint_extrapolation: () =>
    "Pour au moins une espèce, le modèle extrapole (MESS < 0) sur une partie de l'emprise : les valeurs y sont moins fiables.",
  stacking_overprediction: () =>
    "L'empilement de cartes d'espèces surestime le nombre d'espèces présentes en un lieu : chaque carte montre où les conditions sont favorables, pas où l'espèce est présente, et la co-occurrence des espèces n'est pas supposée. Lisez le nombre comme « espèces aux conditions favorables », pas comme la richesse spécifique sur le terrain (Calabrese et al. 2014 ; Guisan & Rahbek 2011).",
  species_below_evidence_level: (p: P) =>
    num(p, "count") && str(p, "level")
      ? `${num(p, "count")} espèce(s) écartée(s) car leur niveau de preuve est inférieur à ${str(p, "level")} (les modèles Exploratory sont exclus par défaut).`
      : null,
} satisfies Words);
Object.assign(es, {
  priority_distance_proxy: () =>
    "No hay superficie de esfuerzo de muestreo para este estudio (no se usó el fondo por grupo objetivo): la prioridad de prospección usa como aproximación la distancia al registro conocido más cercano de la especie. La distancia a los registros de la especie no es esfuerzo de muestreo: zonas alejadas de los registros pueden haberse prospectado sin encontrarla.",
  priority_no_candidate: () =>
    "Ninguna celda combina una idoneidad igual o superior al umbral P10 con poco muestreo entre las puntuaciones de prioridad más altas: no hay zonas candidatas que listar.",
  footprint_smaller_than_cell: (p: P) =>
    dec(p, "area_km2", 2) && dec(p, "cell_km2", 2) && num(p, "cells")
      ? `La huella (${dec(p, "area_km2", 2)} km²) es menor que una celda del modelo (${dec(p, "cell_km2", 2)} km²): los valores describen la(s) ${num(p, "cells")} celda(s) que toca, no las condiciones dentro de la propia huella.`
      : null,
  footprint_outside_study_area: (p: P) =>
    dec(p, "percent", 1)
      ? `El ${dec(p, "percent", 1)} % de la huella queda fuera del área de estudio o sobre celdas sin predicción (mar, predictores ausentes): las proporciones se refieren a la parte con predicciones.`
      : null,
  footprint_extrapolation: () =>
    "Para al menos una especie el modelo extrapola (MESS < 0) en parte de la huella: allí los valores son menos fiables.",
  stacking_overprediction: () =>
    "Apilar mapas de especies sobrestima el número de especies presentes en un lugar: cada mapa muestra dónde las condiciones son idóneas, no dónde está la especie, y no se supone que las especies coexistan. Lea el recuento como «especies con condiciones idóneas», no como riqueza de especies sobre el terreno (Calabrese et al. 2014; Guisan & Rahbek 2011).",
  species_below_evidence_level: (p: P) =>
    num(p, "count") && str(p, "level")
      ? `${num(p, "count")} especie(s) excluida(s) porque su nivel de evidencia es inferior a ${str(p, "level")} (los modelos Exploratory se excluyen por defecto).`
      : null,
} satisfies Words);

const WORDS: Record<Lang, Words> = { en, fr, es };

/** The text of a warning in the interface language. */
export function useWarningText(): (w: Warning) => string {
  const words = WORDS[useLang((s) => s.lang)];
  return (w) => words[w.code]?.(w.params ?? {}, w.message) ?? w.message;
}
