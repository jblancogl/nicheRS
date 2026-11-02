// Strings of the shared domain components (status badges, error and empty states, legends).
// Evidence level names (Insufficient, Exploratory, Indicative, Supported) stay in English in
// every language, as in the help. Server-provided names such as "… (SYNTHETIC)" are untouched.

import type { EvidenceLevel, JobStatus, Severity, SpeciesRunStatus } from "@/api/types";
import { defineMessages } from "@/i18n/messages";

export const useDomainMessages = defineMessages({
  en: {
    synthetic: "Synthetic",
    syntheticTitle: "Synthetic demo data — not real biodiversity data",
    serverDown: "Server unreachable",
    somethingWrong: "Something went wrong",
    technicalDetails: "Technical details",
    retry: "Retry",
    warnings: "Warnings",
    moreInfo: "More information",
    job: (s: JobStatus) =>
      ({ queued: "Queued", running: "Running", succeeded: "Completed", failed: "Failed", cancelled: "Cancelled", interrupted: "Interrupted" })[s],
    species: (s: SpeciesRunStatus) =>
      ({ pending: "Pending", running: "Running", succeeded: "Completed", failed: "Failed", skipped: "Skipped", cancelled: "Cancelled" })[s],
    completedWithWarnings: "Completed with warnings",
    notRun: "Not run",
    severity: (s: Severity) => ({ blocking: "Blocking", warning: "Warning", info: "Note" })[s],
    evidenceMeaning: (l: EvidenceLevel) =>
      ({
        insufficient: "The data or validation do not support using this map for decisions.",
        exploratory: "Useful to explore patterns; not robust enough on its own for decisions.",
        indicative: "Reasonable support; use together with expert knowledge and field data.",
        supported: "Data adequacy and validation meet the documented criteria.",
        validated: "Supported, and confirmed on independent presence–absence field data (criteria proposed, pending review by ecologists).",
      })[l],
    mapLegend: "Map legend",
    modelled: "Modelled",
    observed: "Observed",
    suitabilityTitle: (algorithm?: string | null): string =>
      algorithm === "rf"
        ? "Relative habitat suitability index (share of random-forest trees voting presence, 0–1) — not a probability of presence"
        : algorithm === "ensemble"
          ? "Relative habitat suitability index (mean rank of MaxEnt and random forest, 0–1) — not a probability of presence"
          : "Relative habitat suitability (cloglog, 0–1) — not a probability of presence",
    suitabilityRamp: "Viridis colour ramp from dark purple (0, low relative suitability) to yellow (1, high)",
    whatMeans: "What does this mean?",
    suitabilityExplain:
      "The score ranks places by how similar their conditions are to where the species was observed. It is relative: 0.8 is “more suitable than 0.4”, not an 80 % chance of presence. The colour domain is fixed to 0–1 so different species stay comparable.",
    uncertaintyTitle: "Spread between cross-validation models (SD)",
    uncertaintyRamp: "Grey ramp, light = low spread (0), dark = high spread (0.25 or more)",
    uncertaintyExplain: "Darker = models trained on different folds disagree more.",
    extrapolation: "Environmental conditions outside the training range (MESS < 0)",
    suitableAt: "Suitable at threshold",
    hatched: "Outside the training range (MESS < 0), hatched",
    aoaHatched: "Outside the area of applicability (dissimilarity above the threshold), hatched",
    retained: "Occurrence retained for modelling",
    excluded: "Occurrence excluded by filters",
    generalised: "Excluded: coordinates generalised (blurred) by the publisher",
    thinned: "Removed by distance thinning",
    ownRetained: "Organisation's record retained for modelling",
    validationPresence: "Independent validation site: present (held out)",
    validationAbsence: "Absence (surveyed, not found): validation only",
    syntheticLower: "synthetic",
    effortTitle: "Sampling effort: GBIF records of the target group per cell",
    effortRamp: (max: string) => `Orange colour ramp on a log scale, light = 1 record, dark brown = ${max} records per cell`,
    effortScale: (max: string) => `Log scale: 1 → ${max} records per cell; cells without records are transparent.`,
    effortNote: "Records, not visits or observers. Describes where people recorded, not where the species lives.",
  },
  fr: {
    synthetic: "Synthétique",
    syntheticTitle: "Données de démonstration synthétiques — pas des données de biodiversité réelles",
    serverDown: "Serveur injoignable",
    somethingWrong: "Une erreur s'est produite",
    technicalDetails: "Détails techniques",
    retry: "Réessayer",
    warnings: "Avertissements",
    moreInfo: "Plus d'informations",
    job: (s: JobStatus) =>
      ({ queued: "En file", running: "En cours", succeeded: "Terminée", failed: "En échec", cancelled: "Annulée", interrupted: "Interrompue" })[s],
    species: (s: SpeciesRunStatus) =>
      ({ pending: "En attente", running: "En cours", succeeded: "Terminée", failed: "En échec", skipped: "Ignorée", cancelled: "Annulée" })[s],
    completedWithWarnings: "Terminée avec avertissements",
    notRun: "Non exécutée",
    severity: (s: Severity) => ({ blocking: "Bloquant", warning: "Avertissement", info: "Remarque" })[s],
    evidenceMeaning: (l: EvidenceLevel) =>
      ({
        insufficient: "Les données ou la validation ne permettent pas d'utiliser cette carte pour décider.",
        exploratory: "Utile pour explorer des tendances ; pas assez robuste à lui seul pour décider.",
        indicative: "Appui raisonnable ; à utiliser avec l'expertise naturaliste et les données de terrain.",
        supported: "L'adéquation des données et la validation satisfont les critères documentés.",
        validated: "Supported, et confirmé sur des données de terrain présence–absence indépendantes (critères proposés, en attente de revue par les écologues).",
      })[l],
    mapLegend: "Légende de la carte",
    modelled: "Modélisé",
    observed: "Observé",
    suitabilityTitle: (algorithm?: string | null): string =>
      algorithm === "rf"
        ? "Indice d'idonéité relative de l'habitat (part des arbres de la forêt aléatoire votant « présence », 0–1) — pas une probabilité de présence"
        : algorithm === "ensemble"
          ? "Indice d'idonéité relative de l'habitat (rang moyen de MaxEnt et de la forêt aléatoire, 0–1) — pas une probabilité de présence"
          : "Idonéité relative de l'habitat (cloglog, 0–1) — pas une probabilité de présence",
    suitabilityRamp: "Rampe de couleurs Viridis du violet foncé (0, idonéité relative faible) au jaune (1, élevée)",
    whatMeans: "Qu'est-ce que cela signifie ?",
    suitabilityExplain:
      "Le score classe les lieux selon la ressemblance de leurs conditions avec celles des lieux où l'espèce a été observée. Il est relatif : 0,8 signifie « plus favorable que 0,4 », pas 80 % de chances de présence. Le domaine de couleurs est fixé à 0–1 pour que les espèces restent comparables.",
    uncertaintyTitle: "Dispersion entre les modèles de validation croisée (écart-type)",
    uncertaintyRamp: "Rampe de gris, clair = faible dispersion (0), foncé = forte dispersion (0,25 ou plus)",
    uncertaintyExplain: "Plus c'est foncé, plus les modèles entraînés sur des plis différents divergent.",
    extrapolation: "Conditions environnementales hors de la plage d'entraînement (MESS < 0)",
    suitableAt: "Favorable au seuil",
    hatched: "Hors de la plage d'entraînement (MESS < 0), hachuré",
    aoaHatched: "Hors du domaine d'applicabilité (dissimilarité au-dessus du seuil), hachuré",
    retained: "Occurrence retenue pour la modélisation",
    excluded: "Occurrence exclue par les filtres",
    generalised: "Exclue : coordonnées généralisées (floutées) par le fournisseur",
    thinned: "Retirée par le filtrage spatial par distance",
    ownRetained: "Donnée de l'organisation retenue pour la modélisation",
    validationPresence: "Site de validation indépendante : présence (mise de côté)",
    validationAbsence: "Absence (prospecté, non trouvé) : validation uniquement",
    syntheticLower: "synthétique",
    effortTitle: "Effort d'échantillonnage : données GBIF du groupe cible par cellule",
    effortRamp: (max: string) => `Rampe orange en échelle logarithmique, clair = 1 donnée, brun foncé = ${max} données par cellule`,
    effortScale: (max: string) => `Échelle logarithmique : 1 → ${max} données par cellule ; les cellules sans donnée sont transparentes.`,
    effortNote: "Des données, pas des visites ni des observateurs. Montre où l'on a noté des observations, pas où vit l'espèce.",
  },
  es: {
    synthetic: "Sintético",
    syntheticTitle: "Datos de demostración sintéticos — no son datos reales de biodiversidad",
    serverDown: "Servidor inaccesible",
    somethingWrong: "Se ha producido un error",
    technicalDetails: "Detalles técnicos",
    retry: "Reintentar",
    warnings: "Avisos",
    moreInfo: "Más información",
    job: (s: JobStatus) =>
      ({ queued: "En cola", running: "En curso", succeeded: "Completado", failed: "Fallido", cancelled: "Cancelado", interrupted: "Interrumpido" })[s],
    species: (s: SpeciesRunStatus) =>
      ({ pending: "Pendiente", running: "En curso", succeeded: "Completada", failed: "Fallida", skipped: "Omitida", cancelled: "Cancelada" })[s],
    completedWithWarnings: "Completado con avisos",
    notRun: "No ejecutada",
    severity: (s: Severity) => ({ blocking: "Bloqueante", warning: "Aviso", info: "Nota" })[s],
    evidenceMeaning: (l: EvidenceLevel) =>
      ({
        insufficient: "Los datos o la validación no permiten usar este mapa para tomar decisiones.",
        exploratory: "Útil para explorar patrones; no lo bastante robusto por sí solo para decidir.",
        indicative: "Respaldo razonable; úselo junto con el conocimiento experto y los datos de campo.",
        supported: "La adecuación de los datos y la validación cumplen los criterios documentados.",
        validated: "Supported, y confirmado con datos de campo de presencia–ausencia independientes (criterios propuestos, pendientes de revisión por ecólogos).",
      })[l],
    mapLegend: "Leyenda del mapa",
    modelled: "Modelizado",
    observed: "Observado",
    suitabilityTitle: (algorithm?: string | null): string =>
      algorithm === "rf"
        ? "Índice de idoneidad relativa del hábitat (proporción de árboles del bosque aleatorio que votan «presencia», 0–1) — no es una probabilidad de presencia"
        : algorithm === "ensemble"
          ? "Índice de idoneidad relativa del hábitat (rango medio de MaxEnt y del bosque aleatorio, 0–1) — no es una probabilidad de presencia"
          : "Idoneidad relativa del hábitat (cloglog, 0–1) — no es una probabilidad de presencia",
    suitabilityRamp: "Rampa de color Viridis del morado oscuro (0, idoneidad relativa baja) al amarillo (1, alta)",
    whatMeans: "¿Qué significa?",
    suitabilityExplain:
      "La puntuación ordena los lugares según cuánto se parecen sus condiciones a las de los lugares donde se registró la especie. Es relativa: 0,8 significa «más idóneo que 0,4», no un 80 % de probabilidad de presencia. El dominio de color está fijado en 0–1 para que las especies sigan siendo comparables.",
    uncertaintyTitle: "Dispersión entre los modelos de validación cruzada (desviación típica)",
    uncertaintyRamp: "Rampa de grises, claro = poca dispersión (0), oscuro = mucha dispersión (0,25 o más)",
    uncertaintyExplain: "Cuanto más oscuro, más discrepan los modelos entrenados con pliegues distintos.",
    extrapolation: "Condiciones ambientales fuera del rango de entrenamiento (MESS < 0)",
    suitableAt: "Idóneo en el umbral",
    hatched: "Fuera del rango de entrenamiento (MESS < 0), rayado",
    aoaHatched: "Fuera del área de aplicabilidad (disimilitud por encima del umbral), rayado",
    retained: "Registro conservado para la modelización",
    excluded: "Registro excluido por los filtros",
    generalised: "Excluido: coordenadas generalizadas (difuminadas) por el publicador",
    thinned: "Eliminado por el filtrado espacial por distancia",
    ownRetained: "Registro de la organización conservado para la modelización",
    validationPresence: "Sitio de validación independiente: presencia (reservado)",
    validationAbsence: "Ausencia (prospectado, no encontrado): solo validación",
    syntheticLower: "sintético",
    effortTitle: "Esfuerzo de muestreo: registros de GBIF del grupo objetivo por celda",
    effortRamp: (max: string) => `Rampa naranja en escala logarítmica, claro = 1 registro, marrón oscuro = ${max} registros por celda`,
    effortScale: (max: string) => `Escala logarítmica: 1 → ${max} registros por celda; las celdas sin registros son transparentes.`,
    effortNote: "Registros, no visitas ni observadores. Muestra dónde se registraron observaciones, no dónde vive la especie.",
  },
});
