import { Plus, Trash2 } from "lucide-react";

import type { ProjectionSpec } from "@/api/types";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { FieldDescription, FieldLegend, FieldSet } from "@/components/ui/field";
import { Label } from "@/components/ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { HelpTrigger } from "@/features/help/HelpTrigger";
import {
  FUTURE_GCMS,
  FUTURE_PERIODS,
  FUTURE_SSPS,
  MAX_PROJECTION_RUNS,
  projectionKey,
  projectionProblems,
} from "@/lib/future";
import { useDraft } from "@/stores/draft";

import { useProjectionMessages } from "./messages";

/** Stable empty list: a selector returning a new `[]` on every call loops forever (React #185). */
const NONE: ProjectionSpec[] = [];

/** Expert setting of the model step: future scenarios to project the final model to (ADR-0016). */
export function ProjectionsField() {
  const m = useProjectionMessages();
  const projections = useDraft((s) => s.projections ?? NONE);
  const setProjections = useDraft((s) => s.setProjections);
  const layers = useDraft((s) => s.layers);
  const runs = projections.reduce((n, p) => n + p.gcms.length, 0);
  const problems = projectionProblems(projections, layers);

  const patch = (i: number, p: Partial<ProjectionSpec>) =>
    setProjections(projections.map((x, j) => (j === i ? { ...x, ...p } : x)));
  const add = () => {
    // The next scenario not yet listed, all five models.
    const used = new Set(projections.map(projectionKey));
    const next =
      FUTURE_PERIODS.flatMap((period) => FUTURE_SSPS.map((ssp) => ({ period, ssp }))).find(
        (c) => c.period === "2041-2070" && !used.has(projectionKey(c)),
      ) ??
      FUTURE_PERIODS.flatMap((period) => FUTURE_SSPS.map((ssp) => ({ period, ssp }))).find((c) => !used.has(projectionKey(c)));
    if (next) setProjections([...projections, { ...next, gcms: [...FUTURE_GCMS] }]);
  };

  return (
    <FieldSet>
      <FieldLegend variant="label" className="flex items-center gap-1">
        {m.legend} <HelpTrigger target={{ article: "future", section: "overview" }} />
      </FieldLegend>
      <FieldDescription>{m.intro}</FieldDescription>
      {projections.length === 0 && <p className="text-sm text-muted-foreground">{m.none}</p>}
      {projections.map((p, i) => {
        const all = FUTURE_GCMS.every((g) => p.gcms.includes(g));
        return (
          <div key={i} className="space-y-3 rounded-lg border p-3">
            <div className="flex flex-wrap items-end gap-3">
              <div className="space-y-1">
                <Label htmlFor={`proj-period-${i}`} className="text-xs">
                  {m.period}
                </Label>
                <Select
                  items={FUTURE_PERIODS.map((v) => ({ value: v, label: v }))}
                  value={p.period}
                  onValueChange={(v) => v !== null && patch(i, { period: v })}
                >
                  <SelectTrigger id={`proj-period-${i}`} className="num w-36">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {FUTURE_PERIODS.map((v) => (
                      <SelectItem key={v} value={v}>
                        {v}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
              <div className="order-last w-full space-y-1">
                <Label htmlFor={`proj-ssp-${i}`} className="flex items-center gap-1 text-xs">
                  {m.ssp} <HelpTrigger target={{ article: "future", section: "scenarios" }} />
                </Label>
                <Select
                  items={FUTURE_SSPS.map((v) => ({ value: v, label: m[`ssp_${v}`] }))}
                  value={p.ssp}
                  onValueChange={(v) => v !== null && patch(i, { ssp: v })}
                >
                  <SelectTrigger id={`proj-ssp-${i}`} className="w-full">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {FUTURE_SSPS.map((v) => (
                      <SelectItem key={v} value={v}>
                        {m[`ssp_${v}`]}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
              <Button
                type="button"
                variant="ghost"
                size="icon"
                aria-label={m.remove}
                onClick={() => setProjections(projections.filter((_, j) => j !== i))}
              >
                <Trash2 aria-hidden />
              </Button>
            </div>
            <p className="text-xs text-muted-foreground">{m[`sspHint_${p.ssp}`]}</p>
            <fieldset className="space-y-1.5">
              <legend className="mb-1 text-xs font-medium">{m.gcms}</legend>
              <Label className="flex items-center gap-2 font-normal">
                <Checkbox
                  checked={all}
                  onCheckedChange={(c) => patch(i, { gcms: c === true ? [...FUTURE_GCMS] : [] })}
                />
                {m.allGcms}
              </Label>
              <div className="grid grid-cols-2 gap-1.5 pl-6">
                {FUTURE_GCMS.map((g) => (
                  <Label key={g} className="flex items-center gap-2 font-mono text-xs font-normal">
                    <Checkbox
                      checked={p.gcms.includes(g)}
                      onCheckedChange={(c) =>
                        patch(i, {
                          gcms: c === true ? FUTURE_GCMS.filter((x) => x === g || p.gcms.includes(x)) : p.gcms.filter((x) => x !== g),
                        })
                      }
                    />
                    {g}
                  </Label>
                ))}
              </div>
              {!all && <p className="text-xs text-muted-foreground">{m.allGcmsHint}</p>}
            </fieldset>
          </div>
        );
      })}
      <div className="flex flex-wrap items-center gap-3">
        <Button type="button" variant="outline" size="sm" onClick={add} disabled={projections.length >= FUTURE_PERIODS.length * FUTURE_SSPS.length}>
          <Plus aria-hidden /> {m.add}
        </Button>
        {projections.length > 0 && (
          <span className="num text-xs text-muted-foreground">{m.runs(runs, MAX_PROJECTION_RUNS)}</span>
        )}
      </div>
      {projections.length > 0 && (
        <>
          <FieldDescription>{m.sspNote}</FieldDescription>
          <FieldDescription>{m.cost}</FieldDescription>
        </>
      )}
      {problems.length > 0 && (
        <ul className="space-y-1 text-sm text-destructive" role="alert">
          {problems.includes("tooMany") && <li>{m.errTooMany(MAX_PROJECTION_RUNS)}</li>}
          {problems.includes("noGcm") && <li>{m.errNoGcm}</li>}
          {problems.includes("duplicate") && <li>{m.errDuplicate}</li>}
          {problems.includes("noClimate") && <li>{m.errNoClimate}</li>}
        </ul>
      )}
    </FieldSet>
  );
}
