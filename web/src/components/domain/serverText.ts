// Server free texts (English) worded in the interface language when they match a known pattern:
// occurrence exclusion reasons (crates/niche-app/src/env.rs) and the reasons a target-group
// background fell back to random (engine.rs). Anything unknown is shown as the server wrote it.

import { useLang, type Lang } from "@/i18n/lang";

type Rule = [RegExp, (m: RegExpMatchArray) => string];
const g = (m: RegExpMatchArray, i: number) => m[i] ?? "";

const EXCLUSIONS: Record<"fr" | "es", Rule[]> = {
  fr: [
    [/^Fossil or captive\/cultivated specimen$/, () => "Spécimen fossile ou captif/cultivé"],
    [/^Licence not allowed for this project \((.+)\)$/, (m) => `Licence non autorisée pour ce projet (${g(m, 1)})`],
    [/^Undated record$/, () => "Occurrence non datée"],
    [/^Recorded before (\d+) \(climate baseline\)$/, (m) => `Observée avant ${g(m, 1)} (période de référence climatique)`],
    [
      /^Coordinates generalised by the publisher; precision not stated or > (.+) m$/,
      (m) => `Coordonnées généralisées (floutées) par le fournisseur ; précision non indiquée ou > ${g(m, 1)} m`,
    ],
    [/^Coordinate uncertainty > (.+) m$/, (m) => `Incertitude des coordonnées > ${g(m, 1)} m`],
    [/^Outside study area$/, () => "Hors de la zone d'étude"],
    [/^In sea or large lake \(land mask\)$/, () => "En mer ou dans un grand lac (masque terrestre)"],
    [/^Missing environmental data$/, () => "Données environnementales manquantes"],
    [/^Duplicate within one grid cell$/, () => "Doublon dans une même cellule de la grille"],
    [/^Thinned \(closer than (.+) km to a retained record\)$/, (m) => `Filtrage spatial (à moins de ${g(m, 1)} km d'une occurrence retenue)`],
    [/^Absence record \(surveyed, not found\): not used as a presence$/, () => "Donnée d'absence (prospecté, non trouvé) : jamais utilisée comme présence"],
  ],
  es: [
    [/^Fossil or captive\/cultivated specimen$/, () => "Ejemplar fósil o cautivo/cultivado"],
    [/^Licence not allowed for this project \((.+)\)$/, (m) => `Licencia no permitida en este proyecto (${g(m, 1)})`],
    [/^Undated record$/, () => "Registro sin fecha"],
    [/^Recorded before (\d+) \(climate baseline\)$/, (m) => `Registrado antes de ${g(m, 1)} (periodo de referencia climático)`],
    [
      /^Coordinates generalised by the publisher; precision not stated or > (.+) m$/,
      (m) => `Coordenadas generalizadas (difuminadas) por el publicador; precisión no indicada o > ${g(m, 1)} m`,
    ],
    [/^Coordinate uncertainty > (.+) m$/, (m) => `Incertidumbre de las coordenadas > ${g(m, 1)} m`],
    [/^Outside study area$/, () => "Fuera del área de estudio"],
    [/^In sea or large lake \(land mask\)$/, () => "En el mar o en un gran lago (máscara de tierra)"],
    [/^Missing environmental data$/, () => "Faltan datos ambientales"],
    [/^Duplicate within one grid cell$/, () => "Duplicado en una misma celda de la cuadrícula"],
    [/^Thinned \(closer than (.+) km to a retained record\)$/, (m) => `Filtrado espacial (a menos de ${g(m, 1)} km de un registro conservado)`],
    [/^Absence record \(surveyed, not found\): not used as a presence$/, () => "Registro de ausencia (prospectado, no encontrado): nunca se usa como presencia"],
  ],
};

const FALLBACKS: Record<"fr" | "es", Rule[]> = {
  fr: [
    [
      /^the synthetic demo has an effort grid only for its own synthetic target group, not for (.+)$/,
      (m) => `la démonstration synthétique n'a de grille d'effort que pour son propre groupe cible synthétique, pas pour ${g(m, 1)}`,
    ],
    [
      /^no regional GBIF mirror covers this study area/,
      () => "aucun miroir GBIF régional ne couvre cette zone d'étude (les grilles d'effort d'échantillonnage proviennent du miroir, ADR-0114)",
    ],
    [
      /^the regional GBIF mirror has no sampling-effort grids yet/,
      () => "le miroir GBIF régional n'a pas encore de grilles d'effort d'échantillonnage (elles sont construites à sa prochaine mise à jour)",
    ],
    [/^GBIF gives no class for this taxon$/, () => "GBIF n'indique pas de classe pour ce taxon"],
    [/^the GBIF class of the species could not be looked up \((.+)\)$/, (m) => `la classe GBIF de l'espèce n'a pas pu être obtenue (${g(m, 1)})`],
    [/^the regional GBIF mirror has no records of class (.+)$/, (m) => `le miroir GBIF régional n'a aucune donnée de la classe ${g(m, 1)}`],
    [/^the regional GBIF mirror's effort grid could not be read \((.+)\)$/, (m) => `la grille d'effort du miroir GBIF régional n'a pas pu être lue (${g(m, 1)})`],
    [
      /^only (\d+) cells of the study area have records of (.+) \(at least (\d+) needed\)$/,
      (m) => `seules ${g(m, 1)} cellules de la zone d'étude ont des données de ${g(m, 2)} (il en faut au moins ${g(m, 3)})`,
    ],
  ],
  es: [
    [
      /^the synthetic demo has an effort grid only for its own synthetic target group, not for (.+)$/,
      (m) => `la demostración sintética solo tiene cuadrícula de esfuerzo para su propio grupo objetivo sintético, no para ${g(m, 1)}`,
    ],
    [
      /^no regional GBIF mirror covers this study area/,
      () => "ninguna réplica regional de GBIF cubre esta área de estudio (las cuadrículas de esfuerzo de muestreo proceden de la réplica, ADR-0114)",
    ],
    [
      /^the regional GBIF mirror has no sampling-effort grids yet/,
      () => "la réplica regional de GBIF aún no tiene cuadrículas de esfuerzo de muestreo (se construyen en su próxima actualización)",
    ],
    [/^GBIF gives no class for this taxon$/, () => "GBIF no indica ninguna clase para este taxón"],
    [/^the GBIF class of the species could not be looked up \((.+)\)$/, (m) => `no se pudo obtener la clase de GBIF de la especie (${g(m, 1)})`],
    [/^the regional GBIF mirror has no records of class (.+)$/, (m) => `la réplica regional de GBIF no tiene registros de la clase ${g(m, 1)}`],
    [/^the regional GBIF mirror's effort grid could not be read \((.+)\)$/, (m) => `no se pudo leer la cuadrícula de esfuerzo de la réplica regional de GBIF (${g(m, 1)})`],
    [
      /^only (\d+) cells of the study area have records of (.+) \(at least (\d+) needed\)$/,
      (m) => `solo ${g(m, 1)} celdas del área de estudio tienen registros de ${g(m, 2)} (se necesitan al menos ${g(m, 3)})`,
    ],
  ],
};

function apply(rules: Record<"fr" | "es", Rule[]>, text: string, lang: Lang): string {
  if (lang === "en") return text;
  for (const [re, f] of rules[lang]) {
    const m = text.match(re);
    if (m) return f(m);
  }
  return text;
}

// Why a species was left out of a multi-species stack (crates/niche-app/src/analyses/richness.rs).
const RICHNESS: Record<"fr" | "es", Rule[]> = {
  fr: [
    [/^no model for this species$/, () => "pas de modèle pour cette espèce"],
    [/^evidence level (\w+) is below the minimum (\w+)$/, (m) => `niveau de preuve ${g(m, 1)} inférieur au minimum ${g(m, 2)}`],
    [/^no P10 threshold$/, () => "pas de seuil P10"],
  ],
  es: [
    [/^no model for this species$/, () => "no hay modelo para esta especie"],
    [/^evidence level (\w+) is below the minimum (\w+)$/, (m) => `nivel de evidencia ${g(m, 1)} inferior al mínimo ${g(m, 2)}`],
    [/^no P10 threshold$/, () => "no hay umbral P10"],
  ],
};

// Reasons of the evidence level (crates/niche-app/src/fit.rs `evidence`, independent.rs
// `apply_to_evidence`, engine.rs `cap_evidence_for_pooled_taxa`). A leading ✓/✗ is kept as is;
// level names (Supported, Indicative…) stay in English as everywhere in the interface.
const RANK: Record<"fr" | "es", Record<string, string>> = {
  fr: { genus: "genre", family: "famille", order: "ordre", class: "classe", phylum: "embranchement", kingdom: "règne" },
  es: { genus: "género", family: "familia", order: "orden", class: "clase", phylum: "filo", kingdom: "reino" },
};
const undef = (v: string, lang: "fr" | "es") => (v === "undefined" ? (lang === "fr" ? "non défini" : "no definido") : v);
const synth = (name: string, lang: "fr" | "es") =>
  name.replace(/ \(synthetic data\)$/, lang === "fr" ? " (données synthétiques)" : " (datos sintéticos)");
const EVIDENCE: Record<"fr" | "es", Rule[]> = {
  fr: [
    [/^(\d+) presence cells \(supported needs ≥ 50\)$/, (m) => `${g(m, 1)} cellules de présence (Supported exige ≥ 50)`],
    [/^mean continuous Boyce index (.+) \(supported needs ≥ 0\.70\)$/, (m) => `indice de Boyce continu moyen ${g(m, 1)} (Supported exige ≥ 0.70)`],
    [
      /^mean 10th-percentile omission ([\d.]+) \(supported needs ≤ 0\.15; 0\.10 expected\)$/,
      (m) => `omission moyenne au 10e percentile ${g(m, 1)} (Supported exige ≤ 0.15 ; 0.10 attendu)`,
    ],
    [/^(\d+) % of the area extrapolated \(supported needs ≤ 10 %\)$/, (m) => `${g(m, 1)} % de la zone en extrapolation (Supported exige ≤ 10 %)`],
    [/^spatially blocked cross-validation$/, () => "validation croisée par blocs spatiaux"],
    [
      /^Indicative: ≥ 25 presences, Boyce ≥ 0\.40, omission ≤ 0\.25, extrapolation ≤ 25 %\.$/,
      () => "Indicative : ≥ 25 présences, Boyce ≥ 0.40, omission ≤ 0.25, extrapolation ≤ 25 %.",
    ],
    [
      /^Exploratory: at least one indicative criterion not met/,
      () => "Exploratory : au moins un critère du niveau Indicative n'est pas rempli (≥ 25 présences, Boyce ≥ 0.40, omission ≤ 0.25, extrapolation ≤ 25 %).",
    ],
    [
      /^Null models requested but none could be fitted: significance not assessed\.$/,
      () => "Modèles nuls demandés mais aucun n'a pu être ajusté : significativité non évaluée.",
    ],
    [
      /^Boyce index not better than (\d+) random-location null models \(p = ([\d.]+) > ([\d.]+)\) — capped at Exploratory$/,
      (m) => `indice de Boyce pas meilleur que ${g(m, 1)} modèles nuls à localisations aléatoires (p = ${g(m, 2)} > ${g(m, 3)}) — limité à Exploratory`,
    ],
    [
      /^Boyce index better than (\d+) random-location null models \(p = ([\d.]+) ≤ ([\d.]+)\)$/,
      (m) => `indice de Boyce meilleur que ${g(m, 1)} modèles nuls à localisations aléatoires (p = ${g(m, 2)} ≤ ${g(m, 3)})`,
    ],
    [
      /^No independent validation data: evidence comes from cross-validation of the same, possibly biased, records\.$/,
      () => "Pas de données de validation indépendantes : la preuve vient de la validation croisée des mêmes occurrences, potentiellement biaisées.",
    ],
    [
      /^Independent data provided \((.*)\) but not usable for validation: (.*) Evidence comes from cross-validation of the same, possibly biased, records\.$/,
      (m) => `Données indépendantes fournies (${synth(g(m, 1), "fr")}) mais inutilisables pour la validation : ${g(m, 2)} La preuve vient de la validation croisée des mêmes occurrences, potentiellement biaisées.`,
    ],
    [
      /^Independent presence–absence validation on the organisation's data \((.*); (held out from training|absences of a training dataset)\): (\d+) presence and (\d+) absence sites, presence–absence AUC ([\d.]+), TSS (\S+) at the training max-SSS threshold\.$/,
      (m) =>
        `Validation indépendante présence–absence sur les données de l'organisation (${synth(g(m, 1), "fr")} ; ${g(m, 2) === "held out from training" ? "écartées de l'apprentissage" : "absences d'un jeu d'apprentissage"}) : ${g(m, 3)} sites de présence et ${g(m, 4)} d'absence, AUC présence–absence ${g(m, 5)}, TSS ${undef(g(m, 6), "fr")} au seuil max-SSS de l'apprentissage.`,
    ],
    [
      /^validation data held out from training \(validated needs held-out data\)$/,
      () => "données de validation écartées de l'apprentissage (Validated exige des données écartées)",
    ],
    [
      /^(\d+) presence and (\d+) absence independent sites \(validated needs ≥ (\d+) of each\)$/,
      (m) => `${g(m, 1)} sites indépendants de présence et ${g(m, 2)} d'absence (Validated exige ≥ ${g(m, 3)} de chaque)`,
    ],
    [/^presence–absence AUC ([\d.]+) \(validated needs ≥ ([\d.]+)\)$/, (m) => `AUC présence–absence ${g(m, 1)} (Validated exige ≥ ${g(m, 2)})`],
    [
      /^TSS (\S+) at the training max-SSS threshold \(validated needs ≥ ([\d.]+)\)$/,
      (m) => `TSS ${undef(g(m, 1), "fr")} au seuil max-SSS de l'apprentissage (Validated exige ≥ ${g(m, 2)})`,
    ],
    [
      /^every criterion of Supported met \(validated needs Supported first\)$/,
      () => "tous les critères de Supported remplis (Validated exige d'abord Supported)",
    ],
    [
      /^Validated: the model is Supported and confirmed on independent presence–absence data/,
      () =>
        "Validated : le modèle est Supported et confirmé sur des données présence–absence indépendantes (critères proposés dans l'ADR-0108, en attente de revue par des écologues).",
    ],
    [
      /^taxon rank (\w+): several species pooled — capped at Exploratory$/,
      (m) => `rang taxonomique ${RANK.fr[g(m, 1)] ?? g(m, 1)} : plusieurs espèces regroupées — limité à Exploratory`,
    ],
  ],
  es: [
    [/^(\d+) presence cells \(supported needs ≥ 50\)$/, (m) => `${g(m, 1)} celdas de presencia (Supported exige ≥ 50)`],
    [/^mean continuous Boyce index (.+) \(supported needs ≥ 0\.70\)$/, (m) => `índice de Boyce continuo medio ${g(m, 1)} (Supported exige ≥ 0.70)`],
    [
      /^mean 10th-percentile omission ([\d.]+) \(supported needs ≤ 0\.15; 0\.10 expected\)$/,
      (m) => `omisión media en el percentil 10 ${g(m, 1)} (Supported exige ≤ 0.15; 0.10 esperado)`,
    ],
    [/^(\d+) % of the area extrapolated \(supported needs ≤ 10 %\)$/, (m) => `${g(m, 1)} % del área en extrapolación (Supported exige ≤ 10 %)`],
    [/^spatially blocked cross-validation$/, () => "validación cruzada por bloques espaciales"],
    [
      /^Indicative: ≥ 25 presences, Boyce ≥ 0\.40, omission ≤ 0\.25, extrapolation ≤ 25 %\.$/,
      () => "Indicative: ≥ 25 presencias, Boyce ≥ 0.40, omisión ≤ 0.25, extrapolación ≤ 25 %.",
    ],
    [
      /^Exploratory: at least one indicative criterion not met/,
      () => "Exploratory: no se cumple al menos un criterio del nivel Indicative (≥ 25 presencias, Boyce ≥ 0.40, omisión ≤ 0.25, extrapolación ≤ 25 %).",
    ],
    [
      /^Null models requested but none could be fitted: significance not assessed\.$/,
      () => "Se pidieron modelos nulos pero no se pudo ajustar ninguno: significación no evaluada.",
    ],
    [
      /^Boyce index not better than (\d+) random-location null models \(p = ([\d.]+) > ([\d.]+)\) — capped at Exploratory$/,
      (m) => `índice de Boyce no mejor que ${g(m, 1)} modelos nulos con localizaciones aleatorias (p = ${g(m, 2)} > ${g(m, 3)}) — limitado a Exploratory`,
    ],
    [
      /^Boyce index better than (\d+) random-location null models \(p = ([\d.]+) ≤ ([\d.]+)\)$/,
      (m) => `índice de Boyce mejor que ${g(m, 1)} modelos nulos con localizaciones aleatorias (p = ${g(m, 2)} ≤ ${g(m, 3)})`,
    ],
    [
      /^No independent validation data: evidence comes from cross-validation of the same, possibly biased, records\.$/,
      () => "Sin datos de validación independientes: la evidencia procede de la validación cruzada de los mismos registros, posiblemente sesgados.",
    ],
    [
      /^Independent data provided \((.*)\) but not usable for validation: (.*) Evidence comes from cross-validation of the same, possibly biased, records\.$/,
      (m) => `Se aportaron datos independientes (${synth(g(m, 1), "es")}) pero no sirven para la validación: ${g(m, 2)} La evidencia procede de la validación cruzada de los mismos registros, posiblemente sesgados.`,
    ],
    [
      /^Independent presence–absence validation on the organisation's data \((.*); (held out from training|absences of a training dataset)\): (\d+) presence and (\d+) absence sites, presence–absence AUC ([\d.]+), TSS (\S+) at the training max-SSS threshold\.$/,
      (m) =>
        `Validación independiente de presencia–ausencia con los datos de la organización (${synth(g(m, 1), "es")}; ${g(m, 2) === "held out from training" ? "apartados del entrenamiento" : "ausencias de un conjunto de entrenamiento"}): ${g(m, 3)} sitios de presencia y ${g(m, 4)} de ausencia, AUC de presencia–ausencia ${g(m, 5)}, TSS ${undef(g(m, 6), "es")} en el umbral max-SSS del entrenamiento.`,
    ],
    [
      /^validation data held out from training \(validated needs held-out data\)$/,
      () => "datos de validación apartados del entrenamiento (Validated exige datos apartados)",
    ],
    [
      /^(\d+) presence and (\d+) absence independent sites \(validated needs ≥ (\d+) of each\)$/,
      (m) => `${g(m, 1)} sitios independientes de presencia y ${g(m, 2)} de ausencia (Validated exige ≥ ${g(m, 3)} de cada tipo)`,
    ],
    [/^presence–absence AUC ([\d.]+) \(validated needs ≥ ([\d.]+)\)$/, (m) => `AUC de presencia–ausencia ${g(m, 1)} (Validated exige ≥ ${g(m, 2)})`],
    [
      /^TSS (\S+) at the training max-SSS threshold \(validated needs ≥ ([\d.]+)\)$/,
      (m) => `TSS ${undef(g(m, 1), "es")} en el umbral max-SSS del entrenamiento (Validated exige ≥ ${g(m, 2)})`,
    ],
    [
      /^every criterion of Supported met \(validated needs Supported first\)$/,
      () => "se cumplen todos los criterios de Supported (Validated exige primero Supported)",
    ],
    [
      /^Validated: the model is Supported and confirmed on independent presence–absence data/,
      () =>
        "Validated: el modelo es Supported y se confirma con datos independientes de presencia–ausencia (criterios propuestos en la ADR-0108, pendientes de revisión por ecólogos).",
    ],
    [
      /^taxon rank (\w+): several species pooled — capped at Exploratory$/,
      (m) => `rango taxonómico ${RANK.es[g(m, 1)] ?? g(m, 1)}: se reúnen varias especies — limitado a Exploratory`,
    ],
  ],
};

/** An exclusion reason (ExclusionCount.reason) in the given language. */
export const exclusionText = (reason: string, lang: Lang) => apply(EXCLUSIONS, reason, lang);

/** Why a target-group background fell back to random (BackgroundSampling.fallback_reason). */
export const fallbackText = (reason: string, lang: Lang) => apply(FALLBACKS, reason, lang);

export function useExclusionText(): (reason: string) => string {
  const lang = useLang((s) => s.lang);
  return (reason) => exclusionText(reason, lang);
}

export function useFallbackText(): (reason: string) => string {
  const lang = useLang((s) => s.lang);
  return (reason) => fallbackText(reason, lang);
}

/** Why a species was left out of a multi-species stack (RichnessSpecies.reason). */
export function useRichnessReasonText(): (reason: string) => string {
  const lang = useLang((s) => s.lang);
  return (reason) => apply(RICHNESS, reason, lang);
}

/** A reason of the evidence level (Evidence.reasons), keeping its leading ✓ or ✗. */
export function useEvidenceReasonText(): (reason: string) => string {
  const lang = useLang((s) => s.lang);
  return (reason) => {
    const mark = /^([✓✗]) /.exec(reason);
    return mark ? `${mark[1]} ${apply(EVIDENCE, reason.slice(2), lang)}` : apply(EVIDENCE, reason, lang);
  };
}
