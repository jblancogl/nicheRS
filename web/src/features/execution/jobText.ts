// The engine writes a study's stage and log in English (they also go to logs and the manifest).
// These tables word the known messages in the interface language; anything else is shown as is.
// Evidence level names stay in English everywhere (components/domain/messages.ts).

import { useLang, type Lang } from "@/i18n/lang";
import { formatCount } from "@/lib/format";

/** A pattern and its wording; `g(i)` is capture group i ("" when absent). */
type Rule = [RegExp, (g: (i: number) => string) => string];

const n = (s: string) => formatCount(Number(s));
/** French puts a space before the colon. */
const sep = (l: Lang) => (l === "fr" ? " : " : ": ");

const GBIF_STATUS: Record<Lang, Record<string, string>> = {
  en: { preparing: "preparing", running: "running", suspended: "suspended" },
  fr: { preparing: "en préparation", running: "en cours", suspended: "suspendu" },
  es: { preparing: "en preparación", running: "en curso", suspended: "suspendido" },
};

const SPECIES_STATUS: Record<Lang, Record<string, string>> = {
  en: {},
  fr: { succeeded: "terminée", failed: "échec", insufficientdata: "données insuffisantes", cancelled: "annulée" },
  es: { succeeded: "terminada", failed: "fallida", insufficientdata: "datos insuficientes", cancelled: "cancelada" },
};

/** Per-step texts, with or without a "Species name: " prefix. */
function steps(l: Lang): Rule[] {
  const t = {
    en: null,
    fr: {
      queued: "En file d'attente",
      resuming: "En file d'attente (reprise)",
      env: "Préparation des données environnementales",
      grid: "Lecture de la grille CHELSA",
      reading: (v: string, i: string, k: string) => `Lecture de ${v} (${i}/${k})`,
      raster: "Rastérisation de la zone d'étude",
      land: "Application du masque terre/mer",
      cached: "Téléchargement GBIF déjà en cache",
      requesting: "Demande du téléchargement à GBIF",
      waiting: (st: string, min: string) => `GBIF prépare le téléchargement (${st} ; ${min} min)`,
      downloading: (r: string) => `Téléchargement de l'archive GBIF (${r} occurrences)`,
      splitting: "Répartition des occurrences par espèce",
      retrieving: "Récupération des occurrences",
      collinearity: "Contrôle de la colinéarité des variables",
      cv: (d: string, k: string) => `Validation croisée : ${d}/${k} modèles`,
      final: "Ajustement du modèle final sur toutes les données",
      importance: "Importance des variables et courbes de réponse",
      predicting: "Prédiction de l'idonéité relative",
      uncertainty: "Estimation de l'incertitude entre modèles de validation croisée",
      mess: "Calcul de la nouveauté environnementale (MESS)",
      aoa: "Calcul du domaine d'applicabilité",
      nulls: (d: string, k: string) => `Modèles nuls : ${d}/${k} modèles`,
      complete: "Modèle terminé",
      completed: "Terminée",
      cancelled: "Annulée",
      cancelledBefore: "Annulée avant le démarrage",
      shutdown: "Interrompue par l'arrêt du serveur",
    },
    es: {
      queued: "En cola",
      resuming: "En cola (reanudación)",
      env: "Preparando los datos ambientales",
      grid: "Leyendo la cuadrícula de CHELSA",
      reading: (v: string, i: string, k: string) => `Leyendo ${v} (${i}/${k})`,
      raster: "Rasterizando el área de estudio",
      land: "Aplicando la máscara de tierra",
      cached: "Descarga de GBIF ya en caché",
      requesting: "Solicitando la descarga a GBIF",
      waiting: (st: string, min: string) => `GBIF está preparando la descarga (${st}; ${min} min)`,
      downloading: (r: string) => `Descargando el archivo de GBIF (${r} registros)`,
      splitting: "Repartiendo los registros por especie",
      retrieving: "Obteniendo los registros de presencia",
      collinearity: "Comprobando la colinealidad de las variables",
      cv: (d: string, k: string) => `Validación cruzada: ${d}/${k} modelos`,
      final: "Ajustando el modelo final con todos los registros",
      importance: "Importancia de las variables y curvas de respuesta",
      predicting: "Prediciendo la idoneidad relativa",
      uncertainty: "Estimando la incertidumbre entre modelos de validación cruzada",
      mess: "Calculando la novedad ambiental (MESS)",
      aoa: "Calculando el área de aplicabilidad",
      nulls: (d: string, k: string) => `Modelos nulos: ${d}/${k} modelos`,
      complete: "Modelo terminado",
      completed: "Terminado",
      cancelled: "Cancelado",
      cancelledBefore: "Cancelado antes de empezar",
      shutdown: "Interrumpido por el apagado del servidor",
    },
  }[l];
  if (!t) return [];
  const gs = GBIF_STATUS[l];
  return [
    [/^Queued$/, () => t.queued],
    [/^Queued \(resuming\)$/, () => t.resuming],
    [/^Preparing environmental data$/, () => t.env],
    [/^Reading CHELSA grid description$/, () => t.grid],
    [/^Reading (.+) \((\d+)\/(\d+)\)$/, (g) => t.reading(g(1), g(2), g(3))],
    [/^Rasterizing study area$/, () => t.raster],
    [/^Applying land mask$/, () => t.land],
    [/^Using cached GBIF download \S+$/, () => t.cached],
    [/^Requesting GBIF download for the whole batch$/, () => t.requesting],
    [/^GBIF is preparing the download \((\w+); (\d+) min elapsed\)$/, (g) => t.waiting(gs[g(1)] ?? g(1), g(2))],
    [/^Downloading GBIF archive \((\d+) records\)$/, (g) => t.downloading(n(g(1)))],
    [/^Splitting GBIF archive per species$/, () => t.splitting],
    [/^Retrieving occurrence records$/, () => t.retrieving],
    [/^Checking collinearity between variables$/, () => t.collinearity],
    [/^Cross-validation: (\d+)\/(\d+) models$/, (g) => t.cv(g(1), g(2))],
    [/^Fitting final model on all records$/, () => t.final],
    [/^Computing variable importance and response curves$/, () => t.importance],
    [/^Predicting habitat suitability$/, () => t.predicting],
    [/^Estimating uncertainty across cross-validation models$/, () => t.uncertainty],
    [/^Computing environmental novelty \(MESS\)$/, () => t.mess],
    [/^Computing the area of applicability$/, () => t.aoa],
    [/^Null models: (\d+)\/(\d+) models$/, (g) => t.nulls(g(1), g(2))],
    [/^Model complete$/, () => t.complete],
    [/^Completed$/, () => t.completed],
    [/^Cancelled$/, () => t.cancelled],
    [/^Cancelled before start$/, () => t.cancelledBefore],
    [/^Interrupted by shutdown$/, () => t.shutdown],
  ];
}

function events(l: Lang): Rule[] {
  const t = {
    en: null,
    fr: {
      queued: "Étude mise en file d'attente",
      started: "Démarrage",
      requesting: (k: string) => `Demande d'un téléchargement GBIF pour ${k} espèce(s)`,
      ready: (key: string, r: string, doi?: string) => `Téléchargement GBIF ${key} prêt : ${r} occurrences${doi ? `, DOI ${doi}` : ""}`,
      grid: (w: string, h: string, c: string, a: string, s: string) =>
        `Grille environnementale prête : ${w} × ${h} cellules, ${c} cellules terrestres dans la zone (${a} km²) en ${s} s`,
      retrieving: "récupération des occurrences",
      outcome: (st: string, lvl: string) => `${st} — niveau de preuve ${lvl}`,
      skipped: "déjà terminée, ignorée (reprise)",
      failedSp: (e: string) => `échec — ${e}`,
      completed: "Étude terminée",
      cancelled: "Étude annulée",
      failed: (e: string) => `Échec de l'étude : ${e}`,
      byUser: "Annulée par l'utilisateur",
      cancelReq: "Annulation demandée ; arrêt à la prochaine étape",
      shutdown: "Arrêtée par l'arrêt du serveur ; reprendra au prochain démarrage",
      batchFailed: (e: string) => `Échec du téléchargement GBIF (${e}) ; utilisation de l'API de recherche par espèce`,
      prepFailed: (e: string) => `Les occurrences n'ont pas pu être préparées à l'avance (${e}) ; l'API de recherche GBIF sera utilisée par espèce`,
      skip: (k: string) =>
        `Poursuite sans le téléchargement GBIF à la demande de l'utilisateur : occurrences issues de l'API de recherche GBIF (sans DOI, au plus ${k} par espèce)`,
    },
    es: {
      queued: "Estudio en cola",
      started: "Inicio",
      requesting: (k: string) => `Solicitando una descarga de GBIF para ${k} especie(s)`,
      ready: (key: string, r: string, doi?: string) => `Descarga de GBIF ${key} lista: ${r} registros${doi ? `, DOI ${doi}` : ""}`,
      grid: (w: string, h: string, c: string, a: string, s: string) =>
        `Cuadrícula ambiental lista: ${w} × ${h} celdas, ${c} celdas de tierra en el área (${a} km²) en ${s} s`,
      retrieving: "obteniendo los registros",
      outcome: (st: string, lvl: string) => `${st} — nivel de evidencia ${lvl}`,
      skipped: "ya terminada, omitida (reanudación)",
      failedSp: (e: string) => `fallida — ${e}`,
      completed: "Estudio terminado",
      cancelled: "Estudio cancelado",
      failed: (e: string) => `El estudio falló: ${e}`,
      byUser: "Cancelado por el usuario",
      cancelReq: "Cancelación solicitada; se detendrá en el siguiente paso",
      shutdown: "Detenido por el apagado del servidor; se reanudará al arrancar",
      batchFailed: (e: string) => `La descarga de GBIF falló (${e}); se usa la API de búsqueda por especie`,
      prepFailed: (e: string) => `No se pudieron preparar los registros por adelantado (${e}); se usará la API de búsqueda de GBIF por especie`,
      skip: (k: string) =>
        `Se continúa sin la descarga de GBIF a petición del usuario: registros de la API de búsqueda de GBIF (sin DOI, como máximo ${k} por especie)`,
    },
  }[l];
  if (!t) return [];
  const st = SPECIES_STATUS[l];
  return [
    [/^Job queued$/, () => t.queued],
    [/^Started$/, () => t.started],
    [/^Requesting one GBIF download for (\d+) species$/, (g) => t.requesting(g(1))],
    [/^GBIF download (\S+) ready: (\d+) records(?:, DOI (\S+))?$/, (g) => t.ready(g(1), n(g(2)), g(3))],
    [
      /^Environmental grid ready: (\d+) × (\d+) cells, (\d+) land cells in study area \(([\d.]+) km²\) in ([\d.]+) s$/,
      (g) => t.grid(g(1), g(2), n(g(3)), n(g(4)), g(5)),
    ],
    [/^(.+?): retrieving occurrences$/, (g) => `${g(1)}${sep(l)}${t.retrieving}`],
    [/^(.+?): (\w+) — evidence level (\w+)$/, (g) => `${g(1)}${sep(l)}${t.outcome(st[g(2)] ?? g(2), g(3))}`],
    [/^(.+?): already completed, skipped \(resume\)$/, (g) => `${g(1)}${sep(l)}${t.skipped}`],
    [/^(.+?): failed — (.+)$/, (g) => `${g(1)}${sep(l)}${t.failedSp(g(2))}`],
    [/^Job completed$/, () => t.completed],
    [/^Job cancelled$/, () => t.cancelled],
    [/^Job failed: (.+)$/, (g) => t.failed(g(1))],
    [/^Cancelled by user$/, () => t.byUser],
    [/^Cancellation requested; stopping at the next checkpoint$/, () => t.cancelReq],
    [/^Stopped by server shutdown; will resume on next start$/, () => t.shutdown],
    [/^GBIF batch download failed \((.+)\); using the search API per species$/, (g) => t.batchFailed(g(1))],
    [/^Occurrence data could not be prepared in advance \((.+)\); the GBIF search API will be used per species$/, (g) => t.prepFailed(g(1))],
    [/^Continuing without the GBIF download at the user's request: .* at most (\d+) records per species\)$/, (g) => t.skip(n(g(1)))],
  ];
}

function apply(rules: Rule[], s: string): string | null {
  for (const [re, f] of rules) {
    const m = s.match(re);
    if (m) return f((i) => m[i] ?? "");
  }
  return null;
}

/** Translators for a study's stage and log messages in the interface language. */
export function useJobText() {
  const lang = useLang((s) => s.lang);
  const st = steps(lang);
  const ev = events(lang);
  return {
    stage: (s: string): string => {
      if (!s) return s;
      const whole = apply(st, s);
      if (whole) return whole;
      // "Species name: step" (per-species progress); the step may itself contain ": ".
      for (let i = s.indexOf(": "); i > 0; i = s.indexOf(": ", i + 2)) {
        const step = apply(st, s.slice(i + 2));
        if (step) return `${s.slice(0, i)}${sep(lang)}${step}`;
      }
      return s;
    },
    event: (s: string): string => apply(ev, s) ?? s,
  };
}

/** What a queued study is doing: waiting for GBIF (its stage says so) or just for a slot. */
export function queuedText(job: { stage: string }, waiting: string, stage: (s: string) => string): string {
  return job.stage.startsWith("GBIF") || job.stage.startsWith("Requesting GBIF") ? stage(job.stage) : waiting;
}
