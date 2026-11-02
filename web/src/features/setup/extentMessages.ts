// Strings of the study-area size check: the modelling grid covers the box around the whole area,
// and its cell count is limited, so far-apart parts may force a coarser resolution.

import { defineMessages } from "@/i18n/messages";

export const useExtentMessages = defineMessages({
  en: {
    finest: "Finest resolution",
    km: (f: number) => `≈ ${f} km`,
    coarser: (f: number) =>
      `Too large for the 1 km grid: this study will use ≈ ${f} km. The grid covers the box around every part of the area, sea and gaps included.`,
    tooLarge:
      "This area is too large even at ≈ 10 km: the grid covers the box around every part of it, islands and overseas territories included. Choose fewer or closer places — for instance “France métropolitaine” or “España peninsular y Baleares”.",
    tooLargeShort: "Choose a smaller study area",
    mainland: "Whole countries include their islands and overseas territories: “France métropolitaine” and “España peninsular y Baleares” fit at 1 km.",
    disabledHint: (f: number) => `Finer resolutions exceed the grid limit for this study area; the finest possible is ≈ ${f} km.`,
    unavailable: (f: number) => `≈ ${f} km: too many grid cells for this study area`,
    reviewBlocked: "The study area is too large for the selected resolution: go back to Data and choose a coarser one.",
  },
  fr: {
    finest: "Résolution la plus fine",
    km: (f: number) => `≈ ${f} km`,
    coarser: (f: number) =>
      `Trop grande pour la grille à 1 km : cette étude utilisera ≈ ${f} km. La grille couvre le rectangle englobant toutes les parties de la zone, mer et espaces vides compris.`,
    tooLarge:
      "Cette zone est trop grande même à ≈ 10 km : la grille couvre le rectangle englobant toutes ses parties, îles et outre-mer compris. Choisissez moins de lieux ou des lieux plus proches — par exemple « France métropolitaine » ou « España peninsular y Baleares ».",
    tooLargeShort: "Choisissez une zone d'étude plus petite",
    mainland: "Les pays entiers comprennent leurs îles et l'outre-mer : « France métropolitaine » et « España peninsular y Baleares » tiennent à 1 km.",
    disabledHint: (f: number) => `Les résolutions plus fines dépassent la limite de la grille pour cette zone ; la plus fine possible est ≈ ${f} km.`,
    unavailable: (f: number) => `≈ ${f} km : trop de cellules pour cette zone d'étude`,
    reviewBlocked: "La zone d'étude est trop grande pour la résolution choisie : revenez à l'étape Données et choisissez-en une plus grossière.",
  },
  es: {
    finest: "Resolución más fina",
    km: (f: number) => `≈ ${f} km`,
    coarser: (f: number) =>
      `Demasiado grande para la cuadrícula de 1 km: este estudio usará ≈ ${f} km. La cuadrícula cubre el rectángulo que envuelve todas las partes del área, mar y huecos incluidos.`,
    tooLarge:
      "Esta área es demasiado grande incluso a ≈ 10 km: la cuadrícula cubre el rectángulo que envuelve todas sus partes, islas y territorios de ultramar incluidos. Elija menos lugares o lugares más próximos; por ejemplo, «France métropolitaine» o «España peninsular y Baleares».",
    tooLargeShort: "Elija un área de estudio más pequeña",
    mainland: "Los países completos incluyen sus islas y territorios de ultramar: «France métropolitaine» y «España peninsular y Baleares» caben a 1 km.",
    disabledHint: (f: number) => `Las resoluciones más finas superan el límite de la cuadrícula para esta área; la más fina posible es ≈ ${f} km.`,
    unavailable: (f: number) => `≈ ${f} km: demasiadas celdas para esta área de estudio`,
    reviewBlocked: "El área de estudio es demasiado grande para la resolución elegida: vuelva al paso Datos y elija una más gruesa.",
  },
});
