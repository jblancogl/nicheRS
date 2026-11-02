// Study draft (setup wizard state). Zustand is justified here: the draft is read by the wizard
// steps, the top-bar context chip and the map (study area), and must survive reloads
// (AC-SET-9). Persisted to localStorage; every storage access is guarded.

import { create } from "zustand";
import { createJSONStorage, persist, type StateStorage } from "zustand/middleware";

import type {
  DatasetRef,
  JobSpec,
  ModelSettings,
  OwnSpeciesData,
  OccurrenceFilters,
  ProjectionSpec,
  SpeciesHit,
  StudyArea,
  ValidationSettings,
} from "@/api/types";
import { DEMO_AREA, DEMO_SECOND_SPECIES, DEMO_SPECIES } from "@/lib/geo";
import { DEFAULT_FILTERS, DEFAULT_MODEL, DEFAULT_SEED, DEFAULT_VALIDATION, normaliseFilters, normaliseModel } from "@/lib/schema";

/** Wizard steps; their labels are translated (useStepLabels, features/setup/messages.ts). */
export const STEPS = [
  { id: "species" },
  { id: "area" },
  { id: "data" },
  { id: "variables" },
  { id: "model" },
  { id: "review" },
] as const;
export type StepIndex = 0 | 1 | 2 | 3 | 4 | 5;

/**
 * How a study is set up. "quick": species, area, launch — data, variables and model keep the
 * recommended settings (ADR-0009) without being shown; "expert": every step. Both build the same
 * JobSpec, so a quick study is as reproducible as an expert one.
 */
export type StudyPath = "quick" | "expert";

/** Steps shown by each path, in order. */
export function pathSteps(path: StudyPath | null): StepIndex[] {
  return path === "quick" ? [0, 1, 5] : [0, 1, 2, 3, 4, 5];
}

/** The step after (or before) `step` on a path; the step itself at either end. */
export function stepAfter(path: StudyPath | null, step: StepIndex, delta: 1 | -1 = 1): StepIndex {
  const steps = pathSteps(path);
  const i = steps.findIndex((x) => x >= step);
  if (i < 0) return step;
  // A step the path skips counts as its next shown step when moving forward.
  const at = steps[i] === step || delta < 0 ? i : i - 1;
  return steps[Math.min(Math.max(at + delta, 0), steps.length - 1)] ?? step;
}

export interface DraftSpecies {
  key: number;
  scientific_name: string;
  canonical_name?: string;
  rank?: string;
  status?: string;
  family?: string | null;
  vernacular_name?: string | null;
  /** The organisation's data used for this species (ADR-0108); absent = GBIF only. */
  own?: OwnSpeciesData | null;
}

/** The organisation's dataset chosen for the study (ADR-0108), with its name for display. */
export interface DraftDataset extends DatasetRef {
  name: string;
}

export type AreaSource = "draw" | "upload" | "place" | `preset:${string}` | "spec";

export interface DraftState {
  step: StepIndex;
  name: string;
  demo: boolean;
  species: DraftSpecies[];
  area: StudyArea | null;
  areaSource: AreaSource | null;
  resolution_factor: number;
  filters: OccurrenceFilters;
  /** null until the user touches the checklist: then the server defaults apply. */
  layers: string[] | null;
  model: ModelSettings;
  validation: ValidationSettings;
  seed: number;
  expert: boolean;
  /** Setup path; remembered across studies in this browser. null until the user chooses. */
  path: StudyPath | null;
  /** Project the study will be created in (organisational only; not part of the spec). */
  projectId?: string | null;
  /** Future-climate projections (opt-in, ADR-0016); absent in older drafts = none. */
  projections?: ProjectionSpec[];
  /** The organisation's dataset used by some species (ADR-0108). */
  ownDataset?: DraftDataset | null;
}

interface DraftActions {
  setStep: (s: StepIndex) => void;
  setName: (n: string) => void;
  addSpecies: (h: SpeciesHit) => void;
  removeSpecies: (key: number) => void;
  setArea: (a: StudyArea | null, source: AreaSource | null) => void;
  renameArea: (name: string) => void;
  setResolution: (f: number) => void;
  setFilters: (f: OccurrenceFilters) => void;
  setLayers: (ids: string[] | null) => void;
  setModel: (m: ModelSettings, v: ValidationSettings, seed: number) => void;
  setProjections: (p: ProjectionSpec[]) => void;
  setExpert: (e: boolean) => void;
  setPath: (p: StudyPath) => void;
  startDemo: () => void;
  /** Demo only: add the second synthetic species (multi-species demonstrations, ADR-0018). */
  addSecondDemoSpecies: () => void;
  /** Duplicate a study: its settings, in the same project. */
  loadSpec: (spec: JobSpec, projectId?: string | null) => void;
  /** Start from scratch, optionally inside a project. */
  reset: (projectId?: string | null) => void;
  setProject: (projectId: string | null) => void;
  /** Choose the organisation's dataset (null: none; every species back to GBIF only). */
  setOwnDataset: (d: DraftDataset | null) => void;
  /** How a species uses the dataset (null: GBIF only). */
  setSpeciesOwn: (key: number, own: OwnSpeciesData | null) => void;
}

const initial: DraftState = {
  step: 0,
  name: "",
  demo: false,
  species: [],
  area: null,
  areaSource: null,
  resolution_factor: 1,
  filters: DEFAULT_FILTERS,
  layers: null,
  model: DEFAULT_MODEL,
  validation: DEFAULT_VALIDATION,
  seed: DEFAULT_SEED,
  expert: false,
  path: null,
};

const safeStorage: StateStorage = {
  getItem: (k) => {
    try {
      return window.localStorage.getItem(k);
    } catch {
      return null;
    }
  },
  setItem: (k, v) => {
    try {
      window.localStorage.setItem(k, v);
    } catch {
      /* quota exceeded / private mode: draft just is not persisted */
    }
  },
  removeItem: (k) => {
    try {
      window.localStorage.removeItem(k);
    } catch {
      /* ignore */
    }
  },
};

export const useDraft = create<DraftState & DraftActions>()(
  persist(
    (set) => ({
      ...initial,
      setStep: (step) => set({ step }),
      setName: (name) => set({ name }),
      // The quick path models one species: choosing another replaces it.
      addSpecies: (h) =>
        set((s) =>
          s.species.some((x) => x.key === h.key)
            ? s
            : {
                demo: false,
                // Leaving the demo: its synthetic predictors and its region (labelled SYNTHETIC)
                // do not belong to a real study, so variables and area are chosen again.
                ...(s.demo ? { layers: null, area: null, areaSource: null } : {}),
                species: [
                  ...(s.path === "quick" ? [] : s.species.filter((x) => x.key !== DEMO_SPECIES.key || !s.demo)),
                  {
                    key: h.key,
                    scientific_name: h.scientific_name,
                    canonical_name: h.canonical_name,
                    rank: h.rank,
                    status: h.status,
                    family: h.family ?? null,
                    vernacular_name: h.vernacular_name ?? null,
                  },
                ],
              },
        ),
      removeSpecies: (key) =>
        set((s) => {
          const species = s.species.filter((x) => x.key !== key);
          const demo = s.demo && species.length > 0;
          return { species, demo, ...(s.demo && !demo ? { layers: null, area: null, areaSource: null } : {}) };
        }),
      // The demo's synthetic predictors exist only in its region: its area is fixed.
      setArea: (area, areaSource) => set((s) => (s.demo ? s : { area, areaSource })),
      renameArea: (name) => set((s) => (s.area ? { area: { ...s.area, name } } : s)),
      setResolution: (resolution_factor) => set({ resolution_factor }),
      setFilters: (filters) => set({ filters }),
      setLayers: (layers) => set({ layers }),
      setModel: (model, validation, seed) => set({ model, validation, seed }),
      setProjections: (projections) => set({ projections }),
      setExpert: (expert) => set({ expert }),
      // Steps the quick path does not show resume on its last step.
      setPath: (path) => set((s) => ({ path, step: path === "quick" && s.step > 1 ? 5 : s.step })),
      startDemo: () => {
        set((s) => ({
          ...initial,
          path: s.path,
          demo: true,
          name: "Synthetic demo",
          species: [{ ...DEMO_SPECIES, rank: "SPECIES", status: "SYNTHETIC" }],
          area: DEMO_AREA,
          areaSource: "preset:demo",
          step: s.path === "quick" ? 5 : 2,
        }));
      },
      addSecondDemoSpecies: () =>
        set((s) =>
          !s.demo || s.path === "quick" || s.species.some((x) => x.key === DEMO_SECOND_SPECIES.key)
            ? s
            : { species: [...s.species, { ...DEMO_SECOND_SPECIES, rank: "SPECIES", status: "SYNTHETIC" }] },
        ),
      // Duplicating a run opens the wizard on its review step, not the welcome screen.
      loadSpec: (spec, projectId = null) => {
        set((s) => ({
          ...initial,
          path: s.path,
          name: `${spec.name} (copy)`,
          demo: Boolean(spec.demo),
          species: spec.species.map((s) => ({ key: s.key, scientific_name: s.scientific_name, own: s.own ?? null })),
          // The dataset's name is not in the spec: its id stands in until it is chosen again.
          ownDataset: spec.own_dataset ? { ...spec.own_dataset, name: spec.own_dataset.id.slice(0, 8) } : null,
          area: spec.area,
          areaSource: "spec",
          resolution_factor: spec.resolution_factor,
          filters: normaliseFilters(spec.filters),
          layers: spec.layers,
          model: normaliseModel(spec.model),
          validation: spec.validation,
          seed: spec.seed,
          projections: spec.projections ?? [],
          step: 5,
          projectId,
        }));
      },
      reset: (projectId = null) => set((s) => ({ ...initial, path: s.path, projectId })),
      setProject: (projectId) => set({ projectId }),
      setOwnDataset: (ownDataset) =>
        set((s) => ({
          ownDataset,
          species: ownDataset === null || ownDataset.id !== s.ownDataset?.id ? s.species.map((x) => ({ ...x, own: null })) : s.species,
        })),
      setSpeciesOwn: (key, own) => set((s) => ({ species: s.species.map((x) => (x.key === key ? { ...x, own } : x)) })),
    }),
    {
      name: "nichers.draft.v1",
      version: 3,
      storage: createJSONStorage(() => safeStorage),
      // v1 drafts could keep the demo region (labelled SYNTHETIC) after switching to a real
      // species: drop it so it is chosen again. Drafts from before v3 were set up step by step:
      // one in progress continues on the expert path; otherwise the path is asked for.
      migrate: (persisted, version) => {
        let d = persisted as DraftState;
        if (version < 2 && d && !d.demo && d.area?.name === DEMO_AREA.name) {
          d = { ...d, area: null, areaSource: null };
        }
        if (version < 3 && d) {
          d = { ...d, path: d.species.length > 0 || d.area !== null ? "expert" : null };
        }
        return d;
      },
    },
  ),
);

/** Build the API JobSpec from the draft (null when incomplete). */
export function draftToSpec(d: DraftState, layers: string[]): JobSpec | null {
  if (d.species.length === 0 || !d.area || layers.length === 0) return null;
  const first = d.species[0];
  const name =
    d.name.trim() ||
    `${first ? first.scientific_name : "Study"}${d.species.length > 1 ? ` + ${d.species.length - 1}` : ""} · ${d.area.name}`;
  // The organisation's data only when a species uses them: GBIF-only specs are unchanged.
  const usesOwn = d.ownDataset != null && d.species.some((s) => s.own);
  return {
    name,
    species: d.species.map((s) => ({
      key: s.key,
      scientific_name: s.scientific_name,
      ...(usesOwn && s.own ? { own: s.own } : {}),
    })),
    area: d.area,
    layers,
    resolution_factor: d.resolution_factor,
    filters: d.filters,
    model: d.model,
    validation: d.validation,
    seed: d.seed,
    ...(d.demo ? { demo: true } : {}),
    ...(d.projections && d.projections.length > 0 ? { projections: d.projections } : {}),
    ...(usesOwn && d.ownDataset ? { own_dataset: { id: d.ownDataset.id, records_sha256: d.ownDataset.records_sha256 } } : {}),
  };
}
