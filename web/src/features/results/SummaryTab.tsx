import type { JobSpec, SpeciesResult } from "@/api/types";
import { InfoTip, KeyValue, WarningsList } from "@/components/domain/common";
import { EvidenceBadge } from "@/components/domain/status";
import { evidenceMeaning } from "@/components/domain/status-meta";
import { Label } from "@/components/ui/label";
import type { HelpTarget } from "@/features/help/content";
import { HelpTrigger } from "@/features/help/HelpTrigger";
import { FutureChips } from "@/features/projections/FutureChips";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import { formatArea, formatDecimal, formatPercent, formatScore } from "@/lib/format";
import { useEvidenceReasonText } from "@/components/domain/serverText";

import { MethodLines } from "./BiasSections";
import { areaBandText, meanSd, percentMeanSd, schemeText } from "./derive";
import { useResultsMessages } from "./messages";
import type { ResultsParams, Rule } from "./params";

export function Card({ title, children, labelledBy, help }: { title: React.ReactNode; children: React.ReactNode; labelledBy: string; help?: HelpTarget }) {
  return (
    <section className="space-y-2 rounded-lg border p-3" aria-labelledby={labelledBy}>
      <div className="flex items-center justify-between gap-2">
        <h3 id={labelledBy} className="text-xs font-semibold tracking-wide text-muted-foreground uppercase">
          {title}
        </h3>
        {help && <HelpTrigger target={help} />}
      </div>
      {children}
    </section>
  );
}

export function EvidenceCard({ r }: { r: SpeciesResult }) {
  const msg = useResultsMessages();
  const reasonText = useEvidenceReasonText();
  return (
    <Card title={msg.evidenceLevel} labelledBy="ev-title" help={{ article: "evidence", section: "levels" }}>
      <div className="flex items-center gap-2">
        <EvidenceBadge level={r.evidence.level} />
        <span className="text-xs text-muted-foreground">{msg.ruleBased}</span>
      </div>
      <p className="text-sm">{evidenceMeaning(r.evidence.level)}</p>
      {r.evidence.reasons.length > 0 && (
        <ul className="list-disc space-y-0.5 pl-5 text-sm text-muted-foreground">
          {r.evidence.reasons.map((x) => (
            <li key={x}>{reasonText(x)}</li>
          ))}
        </ul>
      )}
    </Card>
  );
}

function ThresholdCard({ r, rule, onRule }: { r: SpeciesResult; rule: Rule; onRule: (r: Rule) => void }) {
  const msg = useResultsMessages();
  const t = r.thresholds.find((x) => x.rule === rule) ?? r.thresholds[0];
  if (!t) {
    return (
      <Card title={msg.areaTitle} labelledBy="area-title" help={{ article: "area", section: "rules" }}>
        <p className="text-sm text-muted-foreground">{msg.noThreshold}</p>
      </Card>
    );
  }
  return (
    <Card title={msg.areaTitle} labelledBy="area-title" help={{ article: "area", section: "rules" }}>
      <p className="text-sm" role="status" aria-live="polite">
        {msg.areaLead}<span className="num text-2xl font-semibold">{formatArea(t.area_km2)}</span>{" "}
        <span className="num text-muted-foreground">{msg.areaRange(areaBandText(r))}</span>
      </p>
      <p className="num text-sm text-muted-foreground">
        {msg.areaFraction(formatPercent(t.fraction_of_study_area), formatArea(r.study_area_km2))}
        {formatScore(t.threshold)} ({msg.ruleLabel(t.rule, t.label)})
      </p>
      <p className="text-xs text-muted-foreground">
        {msg.areaNote}
      </p>
      <fieldset className="pt-1">
        <legend className="mb-1 text-sm font-medium">{msg.thresholdRule}</legend>
        <RadioGroup value={t.rule} onValueChange={(v) => onRule(v as Rule)} aria-label={msg.thresholdRule}>
          {r.thresholds.map((x) => (
            <Label key={x.rule} className="flex items-center justify-between gap-2 font-normal">
              <span className="flex items-center gap-2">
                <RadioGroupItem value={x.rule} />
                {msg.ruleLabel(x.rule, x.label)}
                {x.rule === "p10" && <span className="text-xs text-muted-foreground">{msg.defaultTag}</span>}
              </span>
              <span className="num text-xs text-muted-foreground">
                t = {formatScore(x.threshold)} · {formatArea(x.area_km2)}
              </span>
            </Label>
          ))}
        </RadioGroup>
      </fieldset>
    </Card>
  );
}

export function SummaryTab({
  r,
  spec,
  params,
  update,
}: {
  r: SpeciesResult;
  spec: JobSpec;
  params: ResultsParams;
  update: (p: Partial<ResultsParams>) => void;
}) {
  const msg = useResultsMessages();
  const v = r.validation;
  return (
    <div className="space-y-4">
      {r.warnings.length > 0 && (
        <section aria-labelledby="warn-title" className="space-y-2">
          <h3 id="warn-title" className="text-sm font-semibold">
            {msg.warnings(r.warnings.length)}
          </h3>
          <WarningsList warnings={r.warnings} />
        </section>
      )}
      <EvidenceCard r={r} />
      <ThresholdCard r={r} rule={params.rule} onRule={(rule) => update({ rule })} />
      <FutureChips r={r} onOpen={() => update({ tab: "future" })} />
      <Card title={msg.validation} labelledBy="val-title" help={{ article: "validation", section: "cv" }}>
        <KeyValue
          className="grid-cols-[minmax(0,1fr)_auto]"
          items={[
            [
              <span key="cbi" className="inline-flex items-center gap-1">
                {msg.cbi}
                <InfoTip>{msg.cbiTip}</InfoTip>
              </span>,
              meanSd(v.cbi_mean, v.folds.map((f) => f.cbi)),
            ],
            [
              <span key="or" className="inline-flex items-center gap-1">
                {msg.or10}
                <InfoTip>{msg.or10Tip}</InfoTip>
              </span>,
              <span key="orv">
                {percentMeanSd(v.or10_mean, v.folds.map((f) => f.or10))} <span className="text-muted-foreground">{msg.expectedParen("10 %")}</span>
              </span>,
            ],
          ]}
        />
        <p className="text-xs text-muted-foreground">{msg.meanSdFolds(schemeText(r))}</p>
      </Card>
      <Card title={msg.methodTitle} labelledBy="method-title" help={{ article: "data", section: "bias" }}>
        <MethodLines r={r} spec={spec} />
      </Card>
      <Card title={msg.mapSummary} labelledBy="map-sum" help={{ article: "map", section: "suitability" }}>
        <KeyValue
          className="grid-cols-[minmax(0,1fr)_auto]"
          items={[
            [msg.studyArea, formatArea(r.study_area_km2)],
            [
              <span key="mess" className="inline-flex items-center gap-1">
                {msg.outsideRange} <HelpTrigger target={{ article: "uncertainty", section: "mess" }} />
              </span>,
              formatPercent(r.extrapolation.novel_fraction),
            ],
            ...(r.aoa
              ? [
                  [
                    <span key="aoa" className="inline-flex items-center gap-1">
                      {msg.aoaOutside} <HelpTrigger target={{ article: "uncertainty", section: "aoa" }} />
                    </span>,
                    formatPercent(r.aoa.fraction_outside),
                  ] as [React.ReactNode, React.ReactNode],
                ]
              : []),
            [
              <span key="sd" className="inline-flex items-center gap-1">
                {msg.foldSpread} <HelpTrigger target={{ article: "uncertainty", section: "foldsd" }} />
              </span>,
              `${formatDecimal(r.uncertainty.mean_fold_sd, 3)} ${msg.sdUnit}`,
            ],
            [msg.selectedModel, msg.selectedModelValue(r.selected)],
          ]}
        />
        <p className="text-xs text-muted-foreground">{msg.mapTextEquivalent}</p>
      </Card>
    </div>
  );
}
