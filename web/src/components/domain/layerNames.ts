// Predictor names, units and sources of the server catalogue (crates/niche-app/src/env.rs) worded
// in the interface language by layer id. The server keeps English (reports, manifest); unknown ids
// keep the server's text. Short names (BIO1, ELEV…) and licences are not translated.

import type { LayerInfo } from "@/api/types";
import { useLang, type Lang } from "@/i18n/lang";

type Names = Record<string, string>;

const fr: Names = {
  chelsa_bio1: "Température moyenne annuelle",
  chelsa_bio2: "Amplitude thermique diurne moyenne",
  chelsa_bio3: "Isothermalité",
  chelsa_bio4: "Saisonnalité de la température (écart type des moyennes mensuelles)",
  chelsa_bio5: "Température maximale du mois le plus chaud",
  chelsa_bio6: "Température minimale du mois le plus froid",
  chelsa_bio7: "Amplitude thermique annuelle",
  chelsa_bio8: "Température moyenne du trimestre le plus humide",
  chelsa_bio9: "Température moyenne du trimestre le plus sec",
  chelsa_bio10: "Température moyenne du trimestre le plus chaud",
  chelsa_bio11: "Température moyenne du trimestre le plus froid",
  chelsa_bio12: "Précipitations annuelles",
  chelsa_bio13: "Précipitations du mois le plus humide",
  chelsa_bio14: "Précipitations du mois le plus sec",
  chelsa_bio15: "Saisonnalité des précipitations (CV)",
  chelsa_bio16: "Précipitations du trimestre le plus humide",
  chelsa_bio17: "Précipitations du trimestre le plus sec",
  chelsa_bio18: "Précipitations du trimestre le plus chaud",
  chelsa_bio19: "Précipitations du trimestre le plus froid",
  dem_elevation: "Altitude (moyenne du MNE à 90 m)",
  dem_slope: "Pente (moyenne des pentes à 90 m, Horn)",
  dem_roughness: "Rugosité du relief (écart type des altitudes à 90 m)",
  lc_tree: "Proportion de couvert arboré",
  lc_shrub: "Proportion de fourrés",
  lc_grass: "Proportion de prairies",
  lc_crop: "Proportion de cultures",
  lc_built: "Proportion de bâti",
  lc_bare: "Proportion de sol nu, végétation clairsemée, neige/glace et mousses/lichens",
  lc_water: "Proportion d'eau permanente",
  lc_wetland: "Proportion de zones humides herbacées et mangroves",
  dist_water: "Distance à l'eau permanente (tronquée à 25 km)",
  demo_temp: "Température synthétique",
  demo_prec: "Précipitations synthétiques",
  demo_rugged: "Rugosité synthétique",
};

const es: Names = {
  chelsa_bio1: "Temperatura media anual",
  chelsa_bio2: "Oscilación térmica diurna media",
  chelsa_bio3: "Isotermalidad",
  chelsa_bio4: "Estacionalidad de la temperatura (desviación típica de las medias mensuales)",
  chelsa_bio5: "Temperatura máxima del mes más cálido",
  chelsa_bio6: "Temperatura mínima del mes más frío",
  chelsa_bio7: "Oscilación térmica anual",
  chelsa_bio8: "Temperatura media del trimestre más húmedo",
  chelsa_bio9: "Temperatura media del trimestre más seco",
  chelsa_bio10: "Temperatura media del trimestre más cálido",
  chelsa_bio11: "Temperatura media del trimestre más frío",
  chelsa_bio12: "Precipitación anual",
  chelsa_bio13: "Precipitación del mes más húmedo",
  chelsa_bio14: "Precipitación del mes más seco",
  chelsa_bio15: "Estacionalidad de la precipitación (CV)",
  chelsa_bio16: "Precipitación del trimestre más húmedo",
  chelsa_bio17: "Precipitación del trimestre más seco",
  chelsa_bio18: "Precipitación del trimestre más cálido",
  chelsa_bio19: "Precipitación del trimestre más frío",
  dem_elevation: "Altitud (media del MDS de 90 m)",
  dem_slope: "Pendiente (media de las pendientes a 90 m, Horn)",
  dem_roughness: "Rugosidad del terreno (desviación típica de las altitudes a 90 m)",
  lc_tree: "Fracción de cubierta arbórea",
  lc_shrub: "Fracción de matorral",
  lc_grass: "Fracción de pastizal",
  lc_crop: "Fracción de cultivos",
  lc_built: "Fracción edificada",
  lc_bare: "Fracción de suelo desnudo, vegetación escasa, nieve/hielo y musgos/líquenes",
  lc_water: "Fracción de agua permanente",
  lc_wetland: "Fracción de humedales herbáceos y manglares",
  dist_water: "Distancia al agua permanente (truncada a 25 km)",
  demo_temp: "Temperatura sintética",
  demo_prec: "Precipitación sintética",
  demo_rugged: "Rugosidad sintética",
};

const UNITS: Record<"fr" | "es", Record<string, string>> = {
  fr: { "fraction 0–1": "fraction 0–1", index: "indice" },
  es: { "fraction 0–1": "fracción 0–1", index: "índice" },
};

/** English fragments of the catalogue's source strings and their wording. */
const SOURCE_PARTS: Record<"fr" | "es", [string, string][]> = {
  fr: [
    ["Synthetic demonstration layer (not real data)", "Couche de démonstration synthétique (pas des données réelles)"],
    ["1981–2010 climatology", "climatologie 1981–2010"],
    ["3 arc-second surface model, AWS Open Data copy", "modèle numérique de surface à 3 secondes d'arc, copie AWS Open Data"],
    ["TanDEM-X acquisitions", "acquisitions TanDEM-X"],
  ],
  es: [
    ["Synthetic demonstration layer (not real data)", "Capa de demostración sintética (no son datos reales)"],
    ["1981–2010 climatology", "climatología 1981–2010"],
    ["3 arc-second surface model, AWS Open Data copy", "modelo digital de superficie de 3 segundos de arco, copia de AWS Open Data"],
    ["TanDEM-X acquisitions", "adquisiciones TanDEM-X"],
  ],
};

const NAMES: Record<"fr" | "es", Names> = { fr, es };

export interface LayerText {
  name: (l: LayerInfo) => string;
  unit: (l: LayerInfo) => string;
  source: (l: LayerInfo) => string;
}

export function layerText(lang: Lang): LayerText {
  if (lang === "en") return { name: (l) => l.name, unit: (l) => l.unit, source: (l) => l.source };
  return {
    name: (l) => NAMES[lang][l.id] ?? l.name,
    unit: (l) => UNITS[lang][l.unit] ?? l.unit,
    source: (l) => SOURCE_PARTS[lang].reduce((s, [en, t]) => s.split(en).join(t), l.source),
  };
}

/** Predictor names, units and sources in the interface language. */
export function useLayerText(): LayerText {
  return layerText(useLang((s) => s.lang));
}

/** The catalogue entry a result label refers to: layer id, short name, "SHORT — name" (variables
 *  used, contributions) or "<name> (SYNTHETIC)" (demo layers). */
export function findLayer(layers: LayerInfo[] | undefined, label: string): LayerInfo | undefined {
  return layers?.find((x) => x.id === label || x.short_name === label || label.startsWith(`${x.short_name} — `) || label === `${x.name} (SYNTHETIC)`);
}

/** A variable of the results as "name (SHORT) · unit" in the interface language; unknown labels as given. */
export function useVariableName(layers: LayerInfo[] | undefined): (label: string) => string {
  const lt = useLayerText();
  return (label) => {
    const l = findLayer(layers, label);
    return l ? `${lt.name(l)} (${l.short_name}) · ${lt.unit(l)}` : label;
  };
}
