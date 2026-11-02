import { Check, Undo2, X } from "lucide-react";

import { Button } from "@/components/ui/button";
import { useMapStore } from "@/stores/map";

import { useMapMessages } from "./messages";

/** Floating toolbar while drawing a study area (keyboard equivalents listed, WCAG 2.5.7). */
export function DrawToolbar() {
  const msg = useMapMessages();
  const n = useMapStore((s) => s.draw.vertices.length);
  const undo = useMapStore((s) => s.undoVertex);
  const finish = useMapStore((s) => s.finishDraw);
  const cancel = useMapStore((s) => s.cancelDraw);
  return (
    <div
      className="absolute top-3 left-1/2 z-10 flex -translate-x-1/2 flex-wrap items-center gap-2 rounded-lg border bg-card px-3 py-2 text-sm shadow-sm"
      role="toolbar"
      aria-label={msg.drawToolbar}
    >
      <span className="text-muted-foreground" aria-live="polite">
        <span className="num">{n}</span> {msg.vertices(n)} · {msg.drawHint}
      </span>
      <Button size="sm" variant="ghost" onClick={undo} disabled={n === 0} aria-label={msg.undoLabel}>
        <Undo2 aria-hidden /> {msg.undo}
      </Button>
      <Button size="sm" variant="ghost" onClick={cancel} aria-label={msg.cancelLabel}>
        <X aria-hidden /> {msg.cancel}
      </Button>
      <Button size="sm" onClick={finish} disabled={n < 3} aria-label={msg.finishLabel}>
        <Check aria-hidden /> {msg.finish}
      </Button>
    </div>
  );
}
