import { ChevronDown } from "lucide-react";
import { Fragment } from "react";

import type { BlockDesign, JobSpec, LayerInfo, NullModelSummary, NullStat, SpeciesResult } from "@/api/types";
import { useVariableName } from "@/components/domain/layerNames";
import { KeyValue, Section } from "@/components/domain/common";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { EM_DASH, formatCount, formatDecimal, formatPercent } from "@/lib/format";
import type { HelpTarget } from "@/features/help/content";
import { HelpTrigger } from "@/features/help/HelpTrigger";
import { IndependentValidationSection } from "@/features/owndata/IndependentValidationSection";
import { cn } from "@/lib/utils";

import { MethodLines } from "./BiasSections";
import { meanSd, percentMeanSd, schemeText } from "./derive";
import { useResultsMessages } from "./messages";

function Term({ children, target }: { children: React.ReactNode; target: HelpTarget }) {
  return (
    <span className="inline-flex items-center gap-1">
      {children}
      <HelpTrigger target={target} />
    </span>
  );
}

const km = (v: number | null | undefined) => (v === null || v === undefined || !Number.isFinite(v) ? EM_DASH : `${formatCount(v)} km`);

/** Spatial grid-block design: size used, how it was chosen, folds obtained, variogram ranges. */
function BlockDesignSection({ b, layers }: { b: BlockDesign; layers: LayerInfo[] | undefined }) {
  const name = useVariableName(layers);
  const msg = useResultsMessages();
  const a = b.autocorrelation;
  const items: [React.ReactNode, React.ReactNode][] = [
    [msg.blockSize, km(b.block_km)],
    [msg.chosenBy, b.chosen_by === "user" ? msg.chosenUser : msg.chosenAuto],
    [
      msg.foldsObtained,
      <span key="k" className={cn(b.k_achieved < b.k_requested && "font-medium text-destructive")}>
        {msg.foldsOf(b.k_achieved, b.k_requested)}
      </span>,
    ],
  ];
  if (a) {
    items.push([msg.medianRange, km(a.median_km)]);
    items.push([msg.clampBand, `${km(a.min_km)} – ${km(a.max_km)}`]);
  }
  return (
    <Section title={msg.blockDesign} id="val-blocks" action={<HelpTrigger target={{ article: "validation", section: "grid" }} />}>
      <KeyValue className="grid-cols-[minmax(0,1fr)_auto]" items={items} />
      {a && a.variables.length > 0 && (
        <Collapsible>
          <CollapsibleTrigger className="group flex w-full items-center justify-between text-sm">
            {msg.rangesTitle(a.variables.length)}
            <ChevronDown className="size-4 transition-transform group-data-[panel-open]:rotate-180" aria-hidden />
          </CollapsibleTrigger>
          <CollapsibleContent className="pt-2">
            <Table>
              <caption className="sr-only">{msg.rangesCaption}</caption>
              <TableHeader>
                <TableRow>
                  <TableHead>{msg.variable}</TableHead>
                  <TableHead className="text-right">{msg.rangeKm}</TableHead>
                  <TableHead>{msg.variogramModel}</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody className="num">
                {a.variables.map((v) => (
                  <TableRow key={v.variable}>
                    <TableCell>{name(v.variable)}</TableCell>
                    <TableCell className="text-right">
                      {v.range_km === null ? EM_DASH : `${v.at_upper_bound ? "≥ " : ""}${formatCount(v.range_km)}`}
                    </TableCell>
                    <TableCell className="text-xs text-muted-foreground">
                      {v.range_km === null ? msg.notFitted : `${v.model ? msg.variogram(v.model) : ""}${v.at_upper_bound ? ` · ${msg.atBound}` : ""}`}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
            <p className="mt-1 text-xs text-muted-foreground">{msg.variogramNote(formatCount(a.n_points), formatCount(a.cutoff_km))}</p>
          </CollapsibleContent>
        </Collapsible>
      )}
    </Section>
  );
}

/** Observed metric vs models fitted to random locations. Wording: never "confidence". */
function NullModelSection({ n }: { n: NullModelSummary }) {
  const msg = useResultsMessages();
  const row = (label: string, s: NullStat, fmt: (v: number | null) => string) => {
    const better = s.n > 0 && s.p_value <= 0.05;
    return (
      // The verdict sits on its own line under the metric's row so that the table fits the side
      // panel, phone width included.
      <Fragment key={label}>
        <TableRow className="border-b-0">
          <TableCell className="whitespace-normal">{label}</TableCell>
          <TableCell className="text-right">{fmt(s.observed)}</TableCell>
          <TableCell className="text-right whitespace-normal">
            {s.mean === null ? EM_DASH : s.sd === null ? fmt(s.mean) : `${fmt(s.mean)} ± ${fmt(s.sd)}`}
          </TableCell>
          <TableCell className="text-right">{formatDecimal(s.p_value, 3)}</TableCell>
        </TableRow>
        <TableRow>
          <TableCell colSpan={4} className={cn("pt-0 text-right text-xs whitespace-normal", better ? "text-foreground" : "font-medium text-destructive")}>
            {better ? msg.nullBetter : msg.nullNotBetter}
          </TableCell>
        </TableRow>
      </Fragment>
    );
  };
  return (
    <Section title={msg.nullTitle} id="val-nulls" action={<HelpTrigger target={{ article: "validation", section: "nulls" }} />}>
      <p className="text-sm">{msg.nullLead(n.completed, n.requested, n.presences)}</p>
      <p className="text-sm">
        <span className="text-muted-foreground">{msg.nullSampling}:</span>{" "}
        {n.sampling === "effort_weighted" ? msg.nullEffort : n.sampling === "uniform" ? msg.nullUniform : n.sampling}
      </p>
      <Table>
        <caption className="sr-only">{msg.nullCaption}</caption>
        <TableHeader>
          <TableRow>
            <TableHead className="whitespace-normal">{msg.nullMetric}</TableHead>
            <TableHead className="text-right whitespace-normal">{msg.nullObserved}</TableHead>
            <TableHead className="text-right whitespace-normal">{msg.nullMean}</TableHead>
            <TableHead className="text-right whitespace-normal">{msg.pValue}</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody className="num">
          {row(msg.cbiFull, n.cbi, (v) => formatDecimal(v))}
          {row(msg.or10Full, n.or10, (v) => formatPercent(v))}
        </TableBody>
      </Table>
      <p className="text-xs text-muted-foreground">{msg.nullNote}</p>
    </Section>
  );
}

export function ValidationTab({ r, spec, layers }: { r: SpeciesResult; spec: JobSpec; layers: LayerInfo[] | undefined }) {
  const msg = useResultsMessages();
  const v = r.validation;
  const algorithm = r.selected.algorithm ?? "maxent";
  return (
    <div className="space-y-5">
      <p className="flex items-start justify-between gap-2 text-sm">
        <span>{msg.evaluatedWith(schemeText(r))}</span>
        <HelpTrigger target={{ article: "validation", section: "cv" }} className="shrink-0" />
      </p>
      <MethodLines r={r} spec={spec} />
      <Section title={msg.primaryMetrics} id="val-primary">
        <KeyValue
          className="grid-cols-[minmax(0,1fr)_auto]"
          items={[
            [<Term key="cbi" target={{ article: "validation", section: "boyce" }}>{msg.cbiFull}</Term>, meanSd(v.cbi_mean, v.folds.map((f) => f.cbi))],
            [
              <Term key="or10" target={{ article: "validation", section: "or10" }}>{msg.or10Full}</Term>,
              `${percentMeanSd(v.or10_mean, v.folds.map((f) => f.or10))} · ${msg.expected("10 %")}`,
            ],
            [
              <Term key="ormtp" target={{ article: "validation", section: "ormtp" }}>{msg.ormtpFull}</Term>,
              `${percentMeanSd(v.ormtp_mean, v.folds.map((f) => f.ormtp))} · ${msg.expected("0 %")}`,
            ],
          ]}
        />
      </Section>
      {v.members && v.members.length > 0 && (
        <Section title={msg.membersTitle} id="val-members" action={<HelpTrigger target={{ article: "algorithms", section: "ensemble" }} />}>
          <p className="text-xs text-muted-foreground">{msg.membersNote}</p>
          <Table>
            <caption className="sr-only">{msg.membersTitle}</caption>
            <TableHeader>
              <TableRow>
                <TableHead>{msg.algorithm}</TableHead>
                <TableHead className="text-right">CBI</TableHead>
                <TableHead className="text-right">OR10</TableHead>
                <TableHead className="text-right">{msg.aucShort}</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody className="num">
              <TableRow className="font-medium">
                <TableCell>{msg.algorithmShort("ensemble")}</TableCell>
                <TableCell className="text-right">{formatDecimal(v.cbi_mean)}</TableCell>
                <TableCell className="text-right">{formatPercent(v.or10_mean)}</TableCell>
                <TableCell className="text-right">{formatDecimal(v.auc_mean)}</TableCell>
              </TableRow>
              {v.members.map((m) => (
                <TableRow key={m.algorithm}>
                  <TableCell>{msg.algorithmShort(m.algorithm)}</TableCell>
                  <TableCell className="text-right">{formatDecimal(m.cbi_mean)}</TableCell>
                  <TableCell className="text-right">{formatPercent(m.or10_mean)}</TableCell>
                  <TableCell className="text-right">{formatDecimal(m.auc_mean)}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </Section>
      )}
      {v.null_model && <NullModelSection n={v.null_model} />}
      {v.blocks && <BlockDesignSection b={v.blocks} layers={layers} />}
      {r.independent_validation && <IndependentValidationSection v={r.independent_validation} />}
      <Section title={msg.secondary} id="val-secondary" action={<HelpTrigger target={{ article: "validation", section: "auc" }} />}>
        <KeyValue className="grid-cols-[minmax(0,1fr)_auto]" items={[[msg.aucPb, meanSd(v.auc_mean, v.folds.map((f) => f.auc))]]} />
        <p className="text-xs text-muted-foreground">
          {msg.aucNote}
        </p>
      </Section>
      <Section title={msg.perFold} id="val-folds">
        <Table>
          <caption className="sr-only">{msg.perFoldCaption}</caption>
          <TableHeader>
            <TableRow>
              <TableHead>{msg.fold}</TableHead>
              <TableHead className="text-right">{msg.testN}</TableHead>
              <TableHead className="text-right">CBI</TableHead>
              <TableHead className="text-right">OR10</TableHead>
              <TableHead className="text-right">ORmtp</TableHead>
              <TableHead className="text-right">{msg.aucShort}</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody className="num">
            {v.folds.map((f) => (
              <TableRow key={f.fold}>
                <TableCell>{f.fold}</TableCell>
                <TableCell className="text-right">{formatCount(f.n_test)}</TableCell>
                <TableCell className="text-right">{formatDecimal(f.cbi)}</TableCell>
                <TableCell className="text-right">{formatPercent(f.or10)}</TableCell>
                <TableCell className="text-right">{formatPercent(f.ormtp)}</TableCell>
                <TableCell className="text-right">{formatDecimal(f.auc)}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
        {v.folds.some((f) => f.cbi === null) && (
          <p className="text-xs text-muted-foreground">{msg.cbiMissing}</p>
        )}
      </Section>

      <Collapsible>
        <CollapsibleTrigger className="group flex w-full items-center justify-between text-sm font-semibold">
          {msg.tuning(r.tuning.length)}
          <ChevronDown className="size-4 transition-transform group-data-[panel-open]:rotate-180" aria-hidden />
        </CollapsibleTrigger>
        <CollapsibleContent className="pt-2">
          <p className="mb-2 flex items-center justify-between gap-2 text-xs text-muted-foreground">
            <span>
              {algorithm === "rf"
                ? msg.tuningSelectedRf(r.selected.rf_mtry ?? 0, r.selected.rf_trees ?? 0)
                : algorithm === "ensemble"
                  ? msg.tuningSelectedEnsemble(r.selected.feature_classes, r.selected.beta_multiplier, r.selected.rf_mtry ?? 0, r.selected.rf_trees ?? 0)
                  : msg.tuningSelected(r.selected.feature_classes, r.selected.beta_multiplier)}
            </span>
            <HelpTrigger target={{ article: "validation", section: "selection" }} className="shrink-0" />
          </p>
          <Table>
            <caption className="sr-only">{msg.tuningCaption}</caption>
            <TableHeader>
              <TableRow>
                <TableHead>{r.tuning.some((t) => t.mtry != null) ? msg.setting : msg.features}</TableHead>
                <TableHead className="text-right">β</TableHead>
                <TableHead className="text-right">OR10</TableHead>
                <TableHead className="text-right">CBI</TableHead>
                <TableHead className="text-right">{msg.aucShort}</TableHead>
                <TableHead className="text-right">{msg.coef}</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody className="num">
              {r.tuning.map((t) => {
                const forest = t.mtry != null;
                return (
                  <TableRow
                    key={forest ? `rf-${t.mtry}` : `${t.feature_classes}-${t.beta_multiplier}`}
                    className={cn(t.selected && "bg-accent font-medium")}
                    aria-selected={t.selected}
                  >
                    <TableCell className="font-mono">
                      {forest ? msg.rfSetting(t.mtry ?? 0) : t.feature_classes}
                      {t.selected && <span className="sr-only">{msg.selectedSr}</span>}
                    </TableCell>
                    <TableCell className="text-right">{forest ? EM_DASH : t.beta_multiplier}</TableCell>
                    <TableCell className="text-right">{formatPercent(t.or10_mean)}</TableCell>
                    <TableCell className="text-right">{formatDecimal(t.cbi_mean)}</TableCell>
                    <TableCell className="text-right">{formatDecimal(t.auc_mean)}</TableCell>
                    <TableCell className="text-right">{forest ? EM_DASH : formatCount(t.n_nonzero)}</TableCell>
                  </TableRow>
                );
              })}
            </TableBody>
          </Table>
        </CollapsibleContent>
      </Collapsible>
    </div>
  );
}
