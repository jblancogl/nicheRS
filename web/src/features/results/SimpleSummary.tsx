import type { JobSpec, LayerInfo, SpeciesResult } from "@/api/types";
import { WarningsList } from "@/components/domain/common";
import { EvidenceBadge } from "@/components/domain/status";
import { evidenceMeaning } from "@/components/domain/status-meta";
import { Button } from "@/components/ui/button";
import { HelpTrigger } from "@/features/help/HelpTrigger";
import { formatArea, formatCount, formatPercent } from "@/lib/format";
import { useLayerText } from "@/components/domain/layerNames";
import { useEvidenceReasonText } from "@/components/domain/serverText";

import { areaBandText } from "./derive";
import { useSimpleMessages } from "./simpleMessages";
import { Card } from "./SummaryTab";

/**
 * Plain-language reading of one species' results (the "Simple" reading, for visitors without
 * modelling knowledge). It hides metrics, never caveats: warnings come first, the evidence level
 * keeps its reasons, and the area keeps its band across threshold rules (AGENTS.md).
 */
export function SimpleSummary({ r, spec, layers, onFull }: { r: SpeciesResult; spec: JobSpec; layers: LayerInfo[] | undefined; onFull: () => void }) {
  const s = useSimpleMessages();
  const lt = useLayerText();
  const reasonText = useEvidenceReasonText();
  const p10 = r.thresholds.find((t) => t.rule === "p10") ?? r.thresholds[0];
  // Sources of the layers the study used (variables_used holds display labels, spec.layers the ids).
  const sources = [...new Set((layers ?? []).filter((l) => spec.layers.includes(l.id)).map((l) => lt.source(l)))];
  // Below 0.1 % the share would print as "0.0 %": say "practically all" rather than "none".
  const novel = r.extrapolation.novel_fraction;
  const blocks = r.validation.method === "spatial_block" || r.validation.method === "spatial_grid";
  return (
    <div className="space-y-4">
      <p className="text-base font-medium">{s.headline}</p>

      {r.warnings.length > 0 && (
        <section aria-labelledby="simple-warn" className="space-y-2">
          <h3 id="simple-warn" className="text-sm font-semibold">
            {s.warningsTitle(r.warnings.length)}
          </h3>
          <WarningsList warnings={r.warnings} />
        </section>
      )}

      <Card title={s.mapTitle} labelledBy="simple-map" help={{ article: "map", section: "suitability" }}>
        <ul className="list-disc space-y-1 pl-5 text-sm">
          <li>{s.mapColours}</li>
          <li>{novel >= 0.001 ? s.mapHatch(formatPercent(novel)) : novel > 0 ? s.mapAlmostNoHatch : s.mapNoHatch}</li>
          <li>{s.mapPoints}</li>
        </ul>
      </Card>

      <Card title={s.areaTitle} labelledBy="simple-area" help={{ article: "area", section: "band" }}>
        {p10 ? (
          <>
            <p className="text-sm">
              {s.areaLead}
              <span className="num text-xl font-semibold">{formatArea(p10.area_km2)}</span>
              <span className="num">{s.areaOf(formatPercent(p10.fraction_of_study_area), formatArea(r.study_area_km2))}</span>
            </p>
            <p className="num text-sm text-muted-foreground">{s.areaBand(areaBandText(r))}</p>
          </>
        ) : (
          <p className="text-sm text-muted-foreground">{s.noArea}</p>
        )}
      </Card>

      <Card title={s.trustTitle} labelledBy="simple-trust" help={{ article: "evidence", section: "levels" }}>
        <EvidenceBadge level={r.evidence.level} />
        <p className="text-sm">{evidenceMeaning(r.evidence.level)}</p>
        {r.evidence.reasons.length > 0 && (
          <ul className="list-disc space-y-0.5 pl-5 text-sm text-muted-foreground">
            {r.evidence.reasons.map((x) => (
              <li key={x}>{reasonText(x)}</li>
            ))}
          </ul>
        )}
      </Card>

      <Card title={s.dataTitle} labelledBy="simple-data" help={{ article: "data", section: "source" }}>
        <ul className="list-disc space-y-1 pl-5 text-sm">
          <li className="num">{s.dataRecords(formatCount(r.data.presences), formatCount(r.data.records_fetched))}</li>
          {sources.length > 0 && <li>{s.dataClimate(sources.join("; "))}</li>}
          <li>{s.dataModel(spec.model.tuning === "auto", blocks, spec.model.algorithm ?? "maxent")}</li>
        </ul>
      </Card>

      <Card title={s.notTitle} labelledBy="simple-not" help={{ article: "map", section: "not" }}>
        <ul className="list-disc space-y-1 pl-5 text-sm">
          <li>{s.not1}</li>
          <li>{s.not2}</li>
          <li>{s.not3}</li>
          <li>{s.not4}</li>
        </ul>
      </Card>

      <div className="space-y-1">
        <Button variant="outline" onClick={onFull}>
          {s.seeFull}
        </Button>
        <p className="text-xs text-muted-foreground">{s.fullHint}</p>
        <HelpTrigger target={{ article: "getting-started", section: "reading" }} text="readingHelp" />
      </div>
    </div>
  );
}
