import { ChevronLeft, ChevronRight, ChevronsUp, ChevronsDown } from "lucide-react";
import { useEffect, useRef, useState, type CSSProperties, type FocusEvent, type PointerEvent } from "react";
import { Outlet, useLocation } from "react-router";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { MapCanvas } from "@/map/MapCanvas";
import { useMapStore } from "@/stores/map";

import { useShellMessages } from "./messages";

const NARROW = "(max-width: 767px)";

/** Bottom-sheet positions on narrow screens, as a share of the area below the top bar. */
type Snap = "peek" | "half" | "full";
const SNAPS: Record<Snap, number> = { peek: 0.22, half: 0.45, full: 0.88 };
/** Below this movement a press on the handle is a tap, not a drag. */
const DRAG_SLOP = 6;

const isNarrow = () => typeof window.matchMedia === "function" && window.matchMedia(NARROW).matches;

/** Text fields bring up the on-screen keyboard: the sheet opens fully so the field stays visible. */
const isTextField = (el: EventTarget) =>
  el instanceof HTMLTextAreaElement ||
  el instanceof HTMLSelectElement ||
  (el instanceof HTMLInputElement && !["checkbox", "radio", "range", "button", "submit"].includes(el.type));

/**
 * Map layout (setup wizard, study): persistent map + contextual side panel on the left
 * (~400 px, collapsible). Below 768 px the panel becomes a non-modal bottom sheet so the map
 * stays visible: three positions (peek, half, full), dragged by its handle or switched with
 * its button. The sheet height is published to the map store so fit-to-bounds and the
 * floating map controls keep clear of it.
 */
export function MapLayout() {
  const [collapsed, setCollapsed] = useState(false);
  const [snap, setSnap] = useState<Snap>("half");
  // Live height (px) while the handle is dragged; null otherwise.
  const [dragPx, setDragPx] = useState<number | null>(null);
  const frame = useRef<HTMLDivElement>(null);
  const aside = useRef<HTMLElement>(null);
  const raster = useMapStore((s) => s.raster);
  const setSheetInset = useMapStore((s) => s.setSheetInset);
  const loc = useLocation();
  const m = useShellMessages();
  const mapLabel = raster ? m.mapResults(raster.layer) : loc.pathname === "/new" ? m.mapSetup : m.map;

  // Publish how much of the map the sheet covers (0 on wide screens, where the panel is a column).
  useEffect(() => {
    const el = aside.current;
    if (!el) return;
    const publish = () => setSheetInset(isNarrow() ? el.offsetHeight : 0);
    const ro = new ResizeObserver(publish);
    ro.observe(el);
    const mq = typeof window.matchMedia === "function" ? window.matchMedia(NARROW) : null;
    mq?.addEventListener("change", publish);
    return () => {
      ro.disconnect();
      mq?.removeEventListener("change", publish);
      setSheetInset(0);
    };
  }, [setSheetInset]);

  // Drawing a polygon needs the map: the sheet steps aside, and comes back when the drawing ends.
  useEffect(
    () =>
      useMapStore.subscribe((s, prev) => {
        if (s.draw.active === prev.draw.active || !isNarrow()) return;
        setSnap(s.draw.active ? "peek" : "half");
      }),
    [],
  );

  const onHandleDown = (e: PointerEvent<HTMLDivElement>) => {
    const box = frame.current;
    const sheet = aside.current;
    if (!box || !sheet || e.button !== 0) return;
    const startY = e.clientY;
    const startH = sheet.offsetHeight;
    const total = box.clientHeight;
    let dragged = false;
    let last = startH;
    const move = (ev: globalThis.PointerEvent) => {
      const dy = startY - ev.clientY;
      if (!dragged && Math.abs(dy) < DRAG_SLOP) return;
      dragged = true;
      last = Math.min(total * SNAPS.full, Math.max(total * SNAPS.peek * 0.6, startH + dy));
      setDragPx(last);
    };
    const up = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      window.removeEventListener("pointercancel", up);
      if (!dragged) return;
      // A drag that ends on the handle's button is not a press of that button.
      const swallow = (ev: MouseEvent) => ev.stopPropagation();
      window.addEventListener("click", swallow, { capture: true, once: true });
      window.setTimeout(() => window.removeEventListener("click", swallow, true), 0);
      const share = last / total;
      const nearest = (Object.keys(SNAPS) as Snap[]).reduce((a, b) => (Math.abs(SNAPS[b] - share) < Math.abs(SNAPS[a] - share) ? b : a));
      setSnap(nearest);
      setDragPx(null);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    window.addEventListener("pointercancel", up);
  };

  const onPanelFocus = (e: FocusEvent<HTMLElement>) => {
    if (snap !== "full" && isNarrow() && isTextField(e.target)) setSnap("full");
  };

  const full = snap === "full";
  const sheetHeight = dragPx !== null ? `${dragPx}px` : `${SNAPS[snap] * 100}%`;

  return (
    <>
      <a
        href="#map"
        className="sr-only z-50 rounded bg-primary px-3 py-1.5 text-primary-foreground focus:not-sr-only focus:absolute focus:top-2 focus:left-32"
      >
        {m.skipMap}
      </a>
      <div ref={frame} className="relative flex min-h-0 flex-1">
        <aside
          ref={aside}
          id="side-panel"
          aria-label={m.sidePanel}
          onFocus={onPanelFocus}
          style={{ "--sheet-h": sheetHeight } as CSSProperties}
          className={cn(
            "z-20 flex flex-col bg-card",
            // narrow: bottom sheet
            "absolute inset-x-0 bottom-0 h-(--sheet-h) rounded-t-xl border-t shadow-md",
            dragPx === null && "transition-[height] duration-200 ease-out",
            // wide: left column
            "md:static md:h-auto md:w-[400px] md:shrink-0 md:rounded-none md:border-t-0 md:border-r md:shadow-none md:transition-none lg:w-[420px]",
            collapsed && "md:hidden",
          )}
        >
          <div className="flex shrink-0 touch-none flex-col items-center border-b pt-1.5 pb-1 select-none md:hidden" onPointerDown={onHandleDown}>
            <span className="h-1 w-10 cursor-grab rounded-full bg-muted-foreground/40 active:cursor-grabbing" aria-hidden />
            <Button
              variant="ghost"
              size="sm"
              onClick={() => setSnap(full ? "half" : "full")}
              aria-expanded={full}
              aria-controls="side-panel"
            >
              {full ? <ChevronsDown aria-hidden /> : <ChevronsUp aria-hidden />}
              {full ? m.showMoreMap : m.expandPanel}
            </Button>
          </div>
          <div className="min-h-0 flex-1">
            <Outlet />
          </div>
        </aside>
        <main id="map" className="relative min-w-0 flex-1" tabIndex={-1}>
          <MapCanvas label={mapLabel} />
          <Button
            variant="outline"
            size="icon-sm"
            className="absolute top-3 left-3 z-10 hidden bg-card shadow-sm md:inline-flex"
            onClick={() => setCollapsed((v) => !v)}
            aria-label={collapsed ? m.showPanel : m.hidePanel}
            aria-expanded={!collapsed}
            aria-controls="side-panel"
          >
            {collapsed ? <ChevronRight aria-hidden /> : <ChevronLeft aria-hidden />}
          </Button>
        </main>
      </div>
    </>
  );
}
