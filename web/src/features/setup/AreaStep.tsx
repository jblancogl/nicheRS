import { Crosshair, FileUp, MapPin, MapPinned, PenLine } from "lucide-react";
import { useId, useRef, useState } from "react";

import { InfoTip, KeyValue } from "@/components/domain/common";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { formatArea, formatCount } from "@/lib/format";
import {
  AREA_PRESETS,
  MAX_UPLOAD_BYTES,
  areaKm2,
  bboxOf,
  parseStudyAreaGeoJson,
  vertexCount,
} from "@/lib/geo";
import { useHealth } from "@/api/hooks";
import { stepAfter, useDraft } from "@/stores/draft";
import { useMapStore } from "@/stores/map";

import { useExtentMessages } from "./extentMessages";
import { useAreaMessages, usePresetMessages } from "./messages";
import { PlacePicker } from "./PlacePicker";
import { usePlaceMessages } from "./placeMessages";
import { useResolutionFit } from "./previewRequest";
import { WizardFrame } from "./WizardFrame";

type Method = "place" | "draw" | "upload" | "preset";

export function AreaStep() {
  const area = useDraft((s) => s.area);
  const source = useDraft((s) => s.areaSource);
  const setArea = useDraft((s) => s.setArea);
  const renameArea = useDraft((s) => s.renameArea);
  const setStep = useDraft((s) => s.setStep);
  const setResolution = useDraft((s) => s.setResolution);
  const demo = useDraft((s) => s.demo);
  const path = useDraft((s) => s.path);
  const fit = useResolutionFit();
  const xm = useExtentMessages();
  const drawing = useMapStore((s) => s.draw.active);
  const startDraw = useMapStore((s) => s.startDraw);
  const cancelDraw = useMapStore((s) => s.cancelDraw);
  const fitTo = useMapStore((s) => s.fitTo);
  const m = useAreaMessages();
  const pm: Record<string, string> = usePresetMessages();

  const placesAvailable = (useHealth().data?.boundaries?.length ?? 0) > 0;
  const plm = usePlaceMessages();
  const [chosenMethod, setMethod] = useState<Method | null>(
    source === "draw" ? "draw" : source === "upload" ? "upload" : source === "place" ? "place" : source ? "preset" : null,
  );
  // By default, places when the server has boundaries, presets otherwise.
  const method: Method = chosenMethod ?? (placesAvailable ? "place" : "preset");
  const [uploadError, setUploadError] = useState<string | null>(null);
  const fileRef = useRef<HTMLInputElement>(null);
  const nameId = useId();

  const presetId = source?.startsWith("preset:") ? source.slice(7) : "";

  const onDraw = () => {
    startDraw((ring) => {
      const name = area && source === "draw" ? area.name : m.drawnName;
      setArea({ name, geometry: { type: "Polygon", coordinates: [ring] } }, "draw");
    });
  };

  const onFile = async (file: File | undefined) => {
    setUploadError(null);
    if (!file) return;
    if (file.size > MAX_UPLOAD_BYTES) {
      setUploadError(m.fileTooBig((file.size / 1024 / 1024).toFixed(1)));
      return;
    }
    const text = await file.text();
    const r = parseStudyAreaGeoJson(text);
    if (!r.ok) {
      setUploadError(r.error);
      return;
    }
    const name = r.name ?? file.name.replace(/\.(geo)?json$/i, "");
    setArea({ name, geometry: r.geometry }, "upload");
    fitTo(bboxOf(r.geometry));
  };

  const nameMissing = area !== null && area.name.trim().length === 0;

  return (
    <WizardFrame
      title={m.title}
      subtitle={m.subtitle}
      onNext={() => {
        // Start the next step at a resolution the area fits at.
        if (!fit.currentFits && fit.finest !== null) setResolution(fit.finest);
        setStep(stepAfter(path, 1));
      }}
      nextDisabled={!area || nameMissing || drawing || fit.tooLarge}
      nextReason={drawing ? m.finishDrawing : !area ? m.defineArea : fit.tooLarge ? xm.tooLargeShort : m.nameArea}
      nextLabel={m.useArea}
    >
      <div className="space-y-6">
        {demo ? (
          <Alert>
            <AlertTitle>{m.demoTitle}</AlertTitle>
            <AlertDescription>
              {m.demoText}
            </AlertDescription>
          </Alert>
        ) : (
          <>
          <div className="space-y-2">
            <p className="text-sm font-medium" id="method-label">
              {m.method}
            </p>
            <ToggleGroup
              aria-labelledby="method-label"
              value={[method]}
              onValueChange={(v) => {
                const next = v[0] as Method | undefined;
                if (next) {
                  setMethod(next);
                  if (next !== "draw" && drawing) cancelDraw();
                }
              }}
              variant="outline"
              className="w-full"
            >
              {placesAvailable && (
                <ToggleGroupItem value="place" className="flex-1">
                  <MapPin aria-hidden /> {plm.method}
                </ToggleGroupItem>
              )}
              {/* Hand-drawn approximate presets only where no official boundaries are installed
                  (or for a draft that already uses one). */}
              {(!placesAvailable || method === "preset") && (
                <ToggleGroupItem value="preset" className="flex-1">
                  <MapPinned aria-hidden /> {m.preset}
                </ToggleGroupItem>
              )}
              <ToggleGroupItem value="draw" className="flex-1">
                <PenLine aria-hidden /> {m.draw}
              </ToggleGroupItem>
              <ToggleGroupItem value="upload" className="flex-1">
                <FileUp aria-hidden /> {m.upload}
              </ToggleGroupItem>
            </ToggleGroup>
          </div>

          {method === "place" && <PlacePicker />}

          {method === "preset" && (
            <RadioGroup
              aria-label={m.presets}
              value={presetId}
              onValueChange={(id) => {
                const p = AREA_PRESETS.find((x) => x.id === id);
                if (p) {
                  setArea(p.area, `preset:${p.id}`);
                  fitTo(bboxOf(p.area.geometry));
                }
              }}
            >
              {AREA_PRESETS.map((p) => (
                <Label key={p.id} className="flex cursor-pointer items-start gap-3 rounded-lg border p-3 font-normal has-data-checked:border-primary">
                  <RadioGroupItem value={p.id} className="mt-0.5" />
                  <span className="space-y-0.5">
                    <span className="flex items-center gap-2 font-medium">
                      {pm[p.id] ?? p.label} <Badge variant="outline">{m.approximate}</Badge>
                    </span>
                    <span className="block text-xs text-muted-foreground">{pm[`${p.id}_note`] ?? p.note}</span>
                  </span>
                </Label>
              ))}
            </RadioGroup>
          )}

          {method === "draw" && (
            <div className="space-y-2 text-sm">
              <p className="text-muted-foreground">
                {m.drawHelp}
              </p>
              {drawing ? (
                <Button variant="outline" onClick={cancelDraw}>
                  {m.cancelDrawing}
                </Button>
              ) : (
                <Button variant="outline" onClick={onDraw}>
                  <PenLine aria-hidden /> {area && source === "draw" ? m.redraw : m.startDrawing}
                </Button>
              )}
              <p className="text-xs text-muted-foreground">
                {m.keyboardAlt}
              </p>
            </div>
          )}

          {method === "upload" && (
            <div className="space-y-2 text-sm">
              <Label htmlFor="area-file">{m.fileLabel}</Label>
              <Input
                id="area-file"
                ref={fileRef}
                type="file"
                accept=".geojson,.json,application/geo+json,application/json"
                onChange={(e) => void onFile(e.target.files?.[0])}
              />
              <p className="text-xs text-muted-foreground">
                {m.fileHint}
              </p>
              {uploadError && (
                <Alert variant="destructive">
                  <AlertTitle>{m.fileRejected}</AlertTitle>
                  <AlertDescription>{uploadError}</AlertDescription>
                </Alert>
              )}
            </div>
          )}
          </>
        )}

        {area && (
          <div className="space-y-3 rounded-lg border p-3">
            <div className="space-y-1.5">
              <Label htmlFor={nameId}>{m.nameLabel}</Label>
              <Input
                id={nameId}
                value={area.name}
                onChange={(e) => renameArea(e.target.value)}
                aria-invalid={nameMissing}
                aria-describedby={nameMissing ? `${nameId}-err` : undefined}
              />
              {nameMissing && (
                <p id={`${nameId}-err`} className="text-sm text-destructive">
                  {m.nameRequired}
                </p>
              )}
            </div>
            <KeyValue
              items={[
                [
                  <span key="a" className="inline-flex items-center gap-1">
                    {m.areaApprox}
                    <InfoTip>{m.areaTip}</InfoTip>
                  </span>,
                  formatArea(areaKm2(area.geometry)),
                ],
                // Geometry details matter to GIS users, not to a quick study.
                ...(path === "quick"
                  ? []
                  : ([
                      [m.geometry, area.geometry.type],
                      [m.vertices, formatCount(vertexCount(area.geometry))],
                    ] as [string, string][])),
                ...(fit.finest !== null ? [[xm.finest, xm.km(fit.finest)] as [string, string]] : []),
              ]}
            />
            {fit.tooLarge && (
              <p className="text-sm text-destructive" role="alert">
                {xm.tooLarge}
              </p>
            )}
            {fit.finest !== null && fit.finest > 1 && <p className="text-xs text-muted-foreground">{xm.coarser(fit.finest)}</p>}
            {(fit.tooLarge || (fit.finest ?? 1) > 1) && area.places?.some((p) => p.id === "FR" || p.id === "ES") && (
              <p className="text-xs text-muted-foreground">{xm.mainland}</p>
            )}
            <Button variant="ghost" size="sm" onClick={() => fitTo(bboxOf(area.geometry))}>
              <Crosshair aria-hidden /> {m.zoom}
            </Button>
          </div>
        )}
      </div>
    </WizardFrame>
  );
}
