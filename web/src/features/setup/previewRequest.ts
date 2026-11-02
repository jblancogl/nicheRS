import { useMemo } from "react";

import { useAreaCheck } from "@/api/hooks";
import type { DataPreviewRequest } from "@/api/types";
import { useDraft } from "@/stores/draft";

/**
 * Which resolutions the study area fits at (server check of the grid-cell limit). Unknown while
 * the check runs or when it fails, and always "fits" for the offline demo: the server still
 * validates the study when it is created.
 */
export function useResolutionFit() {
  const area = useDraft((s) => s.area);
  const demo = useDraft((s) => s.demo);
  const resolution = useDraft((s) => s.resolution_factor);
  const check = useAreaCheck(area, !demo);
  const data = demo ? undefined : check.data;
  const fits = (factor: number) => data?.resolutions.find((r) => r.factor === factor)?.fits ?? true;
  const finest = data?.resolutions.find((r) => r.fits)?.factor ?? null;
  return {
    /** The check has an answer (or is not needed). */
    known: demo || area === null || check.isSuccess || check.isError,
    fits,
    finest,
    tooLarge: data !== undefined && finest === null,
    currentFits: fits(resolution),
    /** Date of the regional GBIF mirror serving this area (records without waiting for GBIF). */
    mirror: data?.gbif_mirror ?? null,
  };
}

/** The preview request for the first species (shared with the review step via the query cache). */
export function usePreviewRequest(): DataPreviewRequest | null {
  const species = useDraft((s) => s.species);
  const area = useDraft((s) => s.area);
  const resolution = useDraft((s) => s.resolution_factor);
  const filters = useDraft((s) => s.filters);
  const demo = useDraft((s) => s.demo);
  // Distance thinning in the preview is drawn from the study seed, as in the run.
  const seed = useDraft((s) => s.seed);
  const ownDataset = useDraft((s) => s.ownDataset);
  const { known, currentFits } = useResolutionFit();
  const first = species[0];
  // No preview over a grid the server would refuse (it would read the whole oversized window).
  const ready = known && currentFits;
  return useMemo(
    () =>
      first && area && ready
        ? {
            species_key: first.key,
            area,
            resolution_factor: resolution,
            filters,
            seed,
            ...(demo ? { demo: true } : {}),
            // The organisation's data of this species (ADR-0108), as in the run.
            ...(ownDataset && first.own
              ? { own_dataset: { id: ownDataset.id, records_sha256: ownDataset.records_sha256 }, own: first.own }
              : {}),
          }
        : null,
    [first, area, resolution, filters, seed, demo, ready, ownDataset],
  );
}
