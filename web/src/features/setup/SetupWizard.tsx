import { useEffect } from "react";

import { useDraft } from "@/stores/draft";
import { useMapStore } from "@/stores/map";

import { AreaStep } from "./AreaStep";
import { DataStep } from "./DataStep";
import { ModelStep } from "./ModelStep";
import { PathChooser } from "./PathChooser";
import { ReviewStep } from "./ReviewStep";
import { SpeciesStep } from "./SpeciesStep";
import { VariablesStep } from "./VariablesStep";
import { useReachable } from "./reachable";

export function SetupWizard() {
  const step = useDraft((s) => s.step);
  const setStep = useDraft((s) => s.setStep);
  const area = useDraft((s) => s.area);
  const path = useDraft((s) => s.path);
  const reachable = useReachable();

  // A restored draft may point past its prerequisites (e.g. species removed): clamp. The quick
  // path does not show the data, variables and model steps: they resolve to its last step.
  const clamped = Math.min(step, reachable);
  const shown = path === "quick" && clamped > 1 ? 5 : clamped;
  useEffect(() => {
    if (step !== shown) setStep(shown as typeof step);
  }, [step, shown, setStep]);

  // The map shows the draft study area during setup; model rasters are cleared.
  useEffect(() => {
    const m = useMapStore.getState();
    m.setRaster(null);
    m.setInspect(null);
    m.setStudyArea(area?.geometry ?? null);
  }, [area]);

  // Occurrences and the modelling grid belong to the data review: other steps do not show them
  // (stale points from another species or from the demo would be misleading).
  useEffect(() => {
    if (shown !== 2) {
      const m = useMapStore.getState();
      m.setPoints([]);
      m.setGrid(null);
    }
  }, [shown]);

  // Leaving setup: drop setup-only overlays.
  useEffect(
    () => () => {
      const m = useMapStore.getState();
      m.cancelDraw();
      m.setPoints([]);
      m.setGrid(null);
    },
    [],
  );

  if (path === null) return <PathChooser />;

  switch (shown) {
    case 0:
      return <SpeciesStep />;
    case 1:
      return <AreaStep />;
    case 2:
      return <DataStep />;
    case 3:
      return <VariablesStep />;
    case 4:
      return <ModelStep />;
    default:
      return <ReviewStep />;
  }
}
