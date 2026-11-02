import { useDraft, type StepIndex } from "@/stores/draft";

/** Highest step the user may jump to, from the draft prerequisites. */
export function useReachable(): StepIndex {
  const hasSpecies = useDraft((s) => s.species.length > 0);
  const hasArea = useDraft((s) => s.area !== null && s.area.name.trim().length > 0);
  if (!hasSpecies) return 0;
  if (!hasArea) return 1;
  return 5;
}
