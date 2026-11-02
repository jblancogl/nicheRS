// The persistent map. Owns the MapLibre instance; panels only talk to it through useMapStore.

import {
  AttributionControl,
  Map as MapLibreMap,
  NavigationControl,
  Popup,
  ScaleControl,
  setWorkerUrl,
} from "maplibre-gl";
import workerUrl from "maplibre-gl/dist/maplibre-gl-worker.mjs?url";
import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";

import { useLang } from "@/i18n/lang";
import { bboxOf } from "@/lib/geo";
import { useMapStore } from "@/stores/map";
import { useTheme } from "@/stores/theme";

import { loadBasemap, localizeLabels, resetLabelFields } from "./basemap";
import { DrawToolbar } from "./DrawToolbar";
import { InspectContent } from "./InspectContent";
import { NR_RASTER_SOURCES, syncOverlays, type OverlayState } from "./layers";
import { useMapMessages } from "./messages";

// Same-origin worker URL: the bundled main chunk cannot locate the worker by itself, and the
// embedded server's CSP allows same-origin workers (ADR-0004).
setWorkerUrl(workerUrl);

const reducedMotion = () =>
  typeof window.matchMedia === "function" && window.matchMedia("(prefers-reduced-motion: reduce)").matches;

function overlayState(): OverlayState {
  const s = useMapStore.getState();
  return {
    studyArea: s.studyArea,
    points: s.points,
    showPoints: s.showPoints,
    grid: s.grid,
    raster: s.raster,
    analysis: s.analysis,
    footprint: s.footprint,
    draw: s.draw,
    theme: useTheme.getState().resolved,
  };
}

export function MapCanvas({ label }: { label: string }) {
  const msg = useMapMessages();
  const container = useRef<HTMLDivElement>(null);
  const mapRef = useRef<MapLibreMap | null>(null);
  const popupRef = useRef<Popup | null>(null);
  const styleReadyReset = useRef<() => void>(() => undefined);
  const [popupNode, setPopupNode] = useState<HTMLDivElement | null>(null);
  const [webglError, setWebglError] = useState<string | null>(null);
  const theme = useTheme((s) => s.resolved);
  const draw = useMapStore((s) => s.draw);
  const inspect = useMapStore((s) => s.inspect);
  const hasRaster = useMapStore((s) => s.raster !== null);
  const fallback = useMapStore((s) => s.basemapFallback);

  // Create the map once.
  useEffect(() => {
    const el = container.current;
    if (!el) return;
    let map: MapLibreMap;
    try {
      map = new MapLibreMap({
        container: el,
        style: { version: 8, sources: {}, layers: [] },
        center: [2.5, 46.5],
        zoom: 4.6,
        attributionControl: false,
        dragRotate: false,
        pitchWithRotate: false,
        touchPitch: false,
        maxPitch: 0,
        fadeDuration: reducedMotion() ? 0 : 300,
      });
    } catch (err) {
      queueMicrotask(() =>
        setWebglError(err instanceof Error ? err.message : useMapMessages.get().noWebgl),
      );
      return;
    }
    mapRef.current = map;
    if (import.meta.env.DEV) (window as unknown as { __nrMap?: MapLibreMap }).__nrMap = map;
    map.keyboard.disableRotation();
    map.touchZoomRotate.disableRotation();
    map.addControl(new NavigationControl({ showCompass: false }), "top-right");
    map.addControl(new ScaleControl({ unit: "metric" }), "bottom-right");
    map.addControl(new AttributionControl({ compact: true }), "bottom-right");

    let ready = false;
    const resync = () => {
      if (!ready) return;
      try {
        syncOverlays(map, overlayState());
      } catch (err) {
        console.error("Map overlay sync failed", err);
      }
    };
    map.on("style.load", () => {
      ready = true;
      resetLabelFields(map);
      try {
        localizeLabels(map, useLang.getState().lang);
      } catch (err) {
        console.error("Basemap label localisation failed", err);
      }
      resync();
    });
    const unsubLang = useLang.subscribe((s, prev) => {
      if (!ready || s.lang === prev.lang) return;
      try {
        localizeLabels(map, s.lang);
      } catch (err) {
        console.error("Basemap label localisation failed", err);
      }
    });
    styleReadyReset.current = () => {
      ready = false;
    };
    // Tiles of a model layer that is not ready yet (404) should not spam the console.
    map.on("error", (e) => {
      const src = (e as { sourceId?: string }).sourceId;
      if (src && NR_RASTER_SOURCES.includes(src)) return;
      console.error(e.error);
    });

    map.on("click", (e) => {
      const st = useMapStore.getState();
      if (st.draw.active) {
        st.addVertex([Number(e.lngLat.lng.toFixed(6)), Number(e.lngLat.lat.toFixed(6))]);
        return;
      }
      if (st.pick) {
        st.pick(e.lngLat.lng, e.lngLat.lat);
        return;
      }
      if (st.raster) st.setInspect({ lng: e.lngLat.lng, lat: e.lngLat.lat });
    });
    map.on("dblclick", (e) => {
      const st = useMapStore.getState();
      if (st.draw.active) {
        e.preventDefault();
        st.finishDraw();
      }
    });

    const ro = new ResizeObserver(() => map.resize());
    ro.observe(el);

    const unsub = useMapStore.subscribe((s, prev) => {
      if (
        s.studyArea !== prev.studyArea ||
        s.points !== prev.points ||
        s.showPoints !== prev.showPoints ||
        s.grid !== prev.grid ||
        s.raster !== prev.raster ||
        s.analysis !== prev.analysis ||
        s.footprint !== prev.footprint ||
        s.draw !== prev.draw
      )
        resync();
      if (s.fitRequest && s.fitRequest !== prev.fitRequest) {
        const [w, so, e, n] = s.fitRequest.bounds;
        map.fitBounds(
          [
            [w, so],
            [e, n],
          ],
          {
            // Below 768 px the side panel is a bottom sheet covering the lower part of the map.
            padding: s.sheetInset > 0 ? { top: 24, left: 24, right: 24, bottom: s.sheetInset + 24 } : 48,
            animate: !reducedMotion(),
            maxZoom: 11,
          },
        );
      }
      if (s.draw.active !== prev.draw.active) {
        if (s.draw.active) map.doubleClickZoom.disable();
        else map.doubleClickZoom.enable();
      }
      if (s.draw.active !== prev.draw.active || (s.pick === null) !== (prev.pick === null)) {
        map.getCanvas().style.cursor = s.draw.active || s.pick ? "crosshair" : "";
      }
    });

    return () => {
      unsub();
      unsubLang();
      ro.disconnect();
      popupRef.current?.remove();
      map.remove();
      mapRef.current = null;
    };
  }, []);

  // Basemap follows the theme; data ramps do not.
  useEffect(() => {
    let cancelled = false;
    void loadBasemap(theme).then(({ style, fallback: fb }) => {
      const map = mapRef.current;
      if (cancelled || !map) return;
      useMapStore.getState().setBasemapFallback(fb);
      styleReadyReset.current();
      map.setStyle(style, { diff: false });
    });
    return () => {
      cancelled = true;
    };
  }, [theme]);

  // Inspect popup: MapLibre positions it, React renders its content through a portal.
  useEffect(() => {
    const map = mapRef.current;
    if (!map) return;
    if (!inspect || !hasRaster) {
      popupRef.current?.remove();
      popupRef.current = null;
      queueMicrotask(() => setPopupNode(null));
      return;
    }
    const node = document.createElement("div");
    popupRef.current?.remove();
    const popup = new Popup({ closeButton: false, maxWidth: "280px", focusAfterOpen: false })
      .setLngLat([inspect.lng, inspect.lat])
      .setDOMContent(node)
      .addTo(map);
    popup.on("close", () => {
      if (popupRef.current === popup) useMapStore.getState().setInspect(null);
    });
    popupRef.current = popup;
    queueMicrotask(() => setPopupNode(node));
  }, [inspect, hasRaster]);

  // Keyboard: draw-mode keys globally while drawing; inspect keys when the map has focus.
  useEffect(() => {
    if (!draw.active) return;
    const onKey = (e: KeyboardEvent) => {
      const t = e.target as HTMLElement | null;
      if (t && (t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.isContentEditable)) return;
      const st = useMapStore.getState();
      if (e.key === "Escape") {
        e.preventDefault();
        st.cancelDraw();
      } else if (e.key === "Enter") {
        e.preventDefault();
        st.finishDraw();
      } else if (e.key === "Backspace" || (e.key === "z" && (e.ctrlKey || e.metaKey))) {
        e.preventDefault();
        st.undoVertex();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [draw.active]);

  const onMapKeyDown = (e: React.KeyboardEvent) => {
    const map = mapRef.current;
    if (!map) return;
    const st = useMapStore.getState();
    if ((e.key === "i" || e.key === "I") && st.raster && !st.draw.active) {
      const c = map.getCenter();
      st.setInspect({ lng: c.lng, lat: c.lat });
    } else if (e.key === "Escape" && st.inspect) {
      st.setInspect(null);
    } else if (e.key === "0" && st.studyArea) {
      st.fitTo(bboxOf(st.studyArea));
    }
  };

  return (
    <div className="relative h-full w-full bg-muted" onKeyDown={onMapKeyDown}>
      <div
        ref={container}
        // not "absolute inset-0": maplibre-gl.css (unlayered) forces position: relative
        style={{ position: "absolute", inset: 0 }}
        role="region"
        aria-label={label}
        aria-describedby="map-help"
      />
      <p id="map-help" className="sr-only">
        {msg.keyboardHelp}
      </p>
      {draw.active && <DrawToolbar />}
      {fallback && (
        <div className="absolute bottom-8 left-2 z-10 rounded-md border bg-card px-2 py-1 text-xs text-muted-foreground shadow-sm">
          {msg.basemapFallback}
        </div>
      )}
      {webglError && (
        <div className="absolute inset-0 grid place-items-center p-6 text-center text-sm text-muted-foreground">
          <p>{msg.mapFailed(webglError)}</p>
        </div>
      )}
      {popupNode && createPortal(<InspectContent />, popupNode)}
    </div>
  );
}
