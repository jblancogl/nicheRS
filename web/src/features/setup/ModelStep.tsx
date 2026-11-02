import { zodResolver } from "@hookform/resolvers/zod";
import { RotateCcw } from "lucide-react";
import { Controller, useForm, useWatch } from "react-hook-form";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Collapsible, CollapsibleContent } from "@/components/ui/collapsible";
import { Field, FieldDescription, FieldError, FieldLabel, FieldLegend, FieldSet } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import {
  DEFAULT_MODEL,
  DEFAULT_SEED,
  DEFAULT_VALIDATION,
  FEATURE_SETS,
  describeModel,
  fromFormValues,
  modelDiffs,
  modelFormSchema,
  toFormValues,
  withThinning,
  type ModelFormValues,
} from "@/lib/schema";
import { HelpTrigger } from "@/features/help/HelpTrigger";
import { ProjectionsField } from "@/features/projections/ProjectionsField";
import { projectionProblems } from "@/lib/future";
import { useDraft } from "@/stores/draft";

import { useModelMessages } from "./messages";
import { WizardFrame } from "./WizardFrame";

export function ModelStep() {
  const model = useDraft((s) => s.model);
  const validation = useDraft((s) => s.validation);
  const seed = useDraft((s) => s.seed);
  const filters = useDraft((s) => s.filters);
  const setFilters = useDraft((s) => s.setFilters);
  const expert = useDraft((s) => s.expert);
  const setExpert = useDraft((s) => s.setExpert);
  const setModel = useDraft((s) => s.setModel);
  const setStep = useDraft((s) => s.setStep);
  const m = useModelMessages();

  const form = useForm<ModelFormValues>({
    resolver: zodResolver(modelFormSchema),
    defaultValues: toFormValues(model, validation, seed, filters.thin_km ?? null),
    mode: "onBlur",
  });
  const { register, control, handleSubmit, formState, reset, setValue } = form;
  const values = useWatch({ control });
  const errors = formState.errors;

  // Live plain-language summary from the current (possibly invalid) form state.
  const parsed = modelFormSchema.safeParse(values);
  const live = parsed.success ? fromFormValues(parsed.data) : null;
  const summary = live ? describeModel(live.model, live.validation) : m.fixFields;
  const diffs = live ? modelDiffs(live.model, live.validation, live.seed, live.thin_km) : [];

  const onSubmit = handleSubmit(
    (v) => {
      // Future projections are kept in the draft directly; invalid ones block the step.
      const d = useDraft.getState();
      if (projectionProblems(d.projections ?? [], d.layers).length > 0) {
        setExpert(true);
        return;
      }
      const r = fromFormValues(v);
      setModel(r.model, r.validation, r.seed);
      // Distance thinning is an occurrence filter (filters.thin_km), set here with the Expert options.
      setFilters(withThinning(useDraft.getState().filters, r.thin_km));
      setStep(5);
    },
    () => setExpert(true), // reveal the invalid fields; RHF focuses the first one
  );

  const errorCount = Object.keys(errors).length;
  // Null-model counts offered (plus a value from a duplicated study, if different).
  const nullOptions = [...new Set([0, 20, 50, 100, values.null_models ?? 0])].sort((a, b) => a - b);

  return (
    <WizardFrame
      title={m.title}
      subtitle={m.subtitle}
      nextType="submit"
      formId="model-form"
    >
      <form id="model-form" onSubmit={(e) => void onSubmit(e)} noValidate className="space-y-5">
        <div className="space-y-2 rounded-lg border p-3">
          <p className="text-xs font-semibold tracking-wide text-muted-foreground uppercase">{m.computed}</p>
          <p className="text-sm font-medium" aria-live="polite">
            {summary}
          </p>
          <p className="text-sm text-muted-foreground">
            {m.explanation}
          </p>
          <p className="num text-xs text-muted-foreground">
            {m.bgLine(values.background_points?.toLocaleString("en-GB").replace(/,/g, " ") ?? "", String(values.seed))}
          </p>
        </div>

        <div className="flex items-center justify-between gap-2">
          <Label htmlFor="expert-mode" className="flex items-center gap-2">
            <Switch id="expert-mode" checked={expert} onCheckedChange={setExpert} />
            {m.expert}
          </Label>
          {diffs.length > 0 && <Badge variant="secondary" className="num">{m.modified(diffs.length)}</Badge>}
        </div>

        {errorCount > 0 && !expert && (
          <p className="text-sm text-destructive" role="alert">
            {m.invalidExpert}
          </p>
        )}

        <Collapsible open={expert} onOpenChange={setExpert}>
          <CollapsibleContent className="space-y-5">
            <FieldSet>
              <FieldLegend variant="label" className="flex items-center gap-1">
                {m.algorithm} <HelpTrigger target={{ article: "algorithms", section: "choose" }} />
              </FieldLegend>
              <Controller
                control={control}
                name="algorithm"
                render={({ field }) => (
                  <RadioGroup value={field.value} onValueChange={(v) => field.onChange(v)} aria-label={m.algorithm}>
                    {(["maxent", "rf", "ensemble"] as const).map((a) => (
                      <Label key={a} className="flex items-start gap-2 font-normal">
                        <RadioGroupItem value={a} className="mt-0.5" />
                        <span>
                          {m[`algo_${a}`]}
                          <span className="block text-xs text-muted-foreground">{m[`algo_${a}_hint`]}</span>
                        </span>
                      </Label>
                    ))}
                  </RadioGroup>
                )}
              />
              {values.algorithm !== "maxent" && (
                <>
                  <Field data-invalid={Boolean(errors.rf_trees)}>
                    <FieldLabel htmlFor="rf-trees">{m.rfTrees}</FieldLabel>
                    <Input
                      id="rf-trees"
                      type="number"
                      step={100}
                      className="num w-40"
                      aria-invalid={Boolean(errors.rf_trees)}
                      {...register("rf_trees", { valueAsNumber: true })}
                    />
                    <FieldDescription>{m.rfTreesHint}</FieldDescription>
                    <FieldError errors={[errors.rf_trees]} />
                  </Field>
                  <Field>
                    <Controller
                      control={control}
                      name="rf_tune_mtry"
                      render={({ field }) => (
                        <Label className="flex items-center gap-2 font-normal">
                          <Checkbox checked={field.value} onCheckedChange={(c) => field.onChange(c === true)} />
                          {m.rfTuneMtry}
                        </Label>
                      )}
                    />
                    <FieldDescription>{m.rfTuneMtryHint}</FieldDescription>
                  </Field>
                </>
              )}
            </FieldSet>

            {values.algorithm !== "rf" && (
            <>
            <FieldSet>
              <FieldLegend variant="label">{values.algorithm === "ensemble" ? m.tuningMaxent : m.tuning}</FieldLegend>
              <Controller
                control={control}
                name="tuning"
                render={({ field }) => (
                  <RadioGroup value={field.value} onValueChange={(v) => field.onChange(v)} aria-label={m.tuning}>
                    <Label className="flex items-start gap-2 font-normal">
                      <RadioGroupItem value="auto" className="mt-0.5" />
                      <span>{m.tuningAuto}</span>
                    </Label>
                    <Label className="flex items-start gap-2 font-normal">
                      <RadioGroupItem value="fixed" className="mt-0.5" />
                      <span>{m.tuningFixed}</span>
                    </Label>
                  </RadioGroup>
                )}
              />
            </FieldSet>

            <FieldSet data-invalid={Boolean(errors.feature_classes)}>
              <FieldLegend variant="label">{m.featureSets}</FieldLegend>
              <Controller
                control={control}
                name="feature_classes"
                render={({ field }) => (
                  <div className="grid gap-1.5">
                    {FEATURE_SETS.map((f) => (
                      <Label key={f.id} className="flex items-center gap-2 font-normal">
                        <Checkbox
                          checked={field.value.includes(f.id)}
                          aria-invalid={Boolean(errors.feature_classes)}
                          onCheckedChange={(c) =>
                            field.onChange(
                              c
                                ? FEATURE_SETS.map((x) => x.id).filter((id) => id === f.id || field.value.includes(id))
                                : field.value.filter((x) => x !== f.id),
                            )
                          }
                        />
                        <span className="w-12 font-mono text-xs">{f.id}</span>
                        <span>{m[`fs_${f.id}`]}</span>
                      </Label>
                    ))}
                  </div>
                )}
              />
              <FieldError errors={[errors.feature_classes]} />
            </FieldSet>

            <Field data-invalid={Boolean(errors.beta_text)}>
              <FieldLabel htmlFor="beta">{m.beta}</FieldLabel>
              <Input id="beta" className="num" aria-invalid={Boolean(errors.beta_text)} {...register("beta_text")} />
              <FieldDescription>{m.betaHint}</FieldDescription>
              <FieldError errors={[errors.beta_text]} />
            </Field>
            </>
            )}

            <div className="grid grid-cols-2 gap-3">
              <Field data-invalid={Boolean(errors.background_points)}>
                <FieldLabel htmlFor="bg">{m.bg}</FieldLabel>
                <Input id="bg" type="number" className="num" aria-invalid={Boolean(errors.background_points)} {...register("background_points", { valueAsNumber: true })} />
                <FieldError errors={[errors.background_points]} />
              </Field>
              <Field data-invalid={Boolean(errors.collinearity_threshold)}>
                <FieldLabel htmlFor="r">{m.collinearity}</FieldLabel>
                <Input id="r" type="number" step="0.05" className="num" aria-invalid={Boolean(errors.collinearity_threshold)} {...register("collinearity_threshold", { valueAsNumber: true })} />
                <FieldError errors={[errors.collinearity_threshold]} />
              </Field>
            </div>

            <FieldSet>
              <FieldLegend variant="label" className="flex items-center gap-1">
                {m.biasLegend} <HelpTrigger target={{ article: "data", section: "bias" }} />
              </FieldLegend>
              <Field>
                <FieldLabel className="flex items-center gap-1">
                  {m.background} <HelpTrigger target={{ article: "data", section: "targetgroup" }} />
                </FieldLabel>
                <Controller
                  control={control}
                  name="background_method"
                  render={({ field }) => (
                    <RadioGroup value={field.value} onValueChange={(v) => field.onChange(v)} aria-label={m.background}>
                      <Label className="flex items-start gap-2 font-normal">
                        <RadioGroupItem value="random" className="mt-0.5" />
                        <span>{m.bgRandom}</span>
                      </Label>
                      <Label className="flex items-start gap-2 font-normal">
                        <RadioGroupItem value="target_group" className="mt-0.5" />
                        <span>{m.bgTargetGroup}</span>
                      </Label>
                    </RadioGroup>
                  )}
                />
              </Field>
              {values.background_method === "target_group" && (
                <Field data-invalid={Boolean(errors.target_group)}>
                  <FieldLabel htmlFor="target-group">{m.targetGroup}</FieldLabel>
                  <Input
                    id="target-group"
                    autoComplete="off"
                    spellCheck={false}
                    placeholder={m.targetGroupPlaceholder}
                    aria-invalid={Boolean(errors.target_group)}
                    {...register("target_group")}
                  />
                  <FieldDescription>{m.targetGroupHint}</FieldDescription>
                  <FieldError errors={[errors.target_group]} />
                </Field>
              )}
              <Field data-invalid={Boolean(errors.thin_km_text)}>
                <FieldLabel htmlFor="thin-km" className="flex items-center gap-1">
                  {m.thinKm} <HelpTrigger target={{ article: "data", section: "thinning" }} />
                </FieldLabel>
                <Input
                  id="thin-km"
                  inputMode="decimal"
                  className="num w-40"
                  placeholder={m.thinKmPlaceholder}
                  aria-invalid={Boolean(errors.thin_km_text)}
                  {...register("thin_km_text")}
                />
                <FieldDescription>{m.thinKmHint}</FieldDescription>
                <FieldError errors={[errors.thin_km_text]} />
              </Field>
            </FieldSet>

            <FieldSet>
              <FieldLegend variant="label" className="flex items-center gap-1">
                {m.validation} <HelpTrigger target={{ article: "validation", section: "cv" }} />
              </FieldLegend>
              <Controller
                control={control}
                name="method"
                render={({ field }) => (
                  <RadioGroup
                    value={field.value}
                    onValueChange={(v) => {
                      field.onChange(v);
                      if (v === "spatial_block") setValue("folds", 4, { shouldValidate: true });
                      if (v === "spatial_grid") setValue("folds", 5, { shouldValidate: true });
                    }}
                    aria-label={m.validationMethod}
                  >
                    <Label className="flex items-start gap-2 font-normal">
                      <RadioGroupItem value="spatial_block" className="mt-0.5" />
                      <span>{m.spatialBlock}</span>
                    </Label>
                    <Label className="flex items-start gap-2 font-normal">
                      <RadioGroupItem value="spatial_grid" className="mt-0.5" />
                      <span className="inline-flex items-center gap-1">
                        {m.spatialGrid} <HelpTrigger target={{ article: "validation", section: "grid" }} />
                      </span>
                    </Label>
                    <Label className="flex items-start gap-2 font-normal">
                      <RadioGroupItem value="random_kfold" className="mt-0.5" />
                      <span>{m.randomKfold}</span>
                    </Label>
                  </RadioGroup>
                )}
              />
              <div className="grid grid-cols-2 gap-3">
                <Field data-invalid={Boolean(errors.folds)}>
                  <FieldLabel htmlFor="folds">{m.folds}</FieldLabel>
                  <Input
                    id="folds"
                    type="number"
                    className="num"
                    disabled={values.method === "spatial_block"}
                    aria-invalid={Boolean(errors.folds)}
                    {...register("folds", { valueAsNumber: true })}
                  />
                  <FieldError errors={[errors.folds]} />
                </Field>
                <Field data-invalid={Boolean(errors.seed)}>
                  <FieldLabel htmlFor="seed">{m.seed}</FieldLabel>
                  <Input id="seed" type="number" className="num" aria-invalid={Boolean(errors.seed)} {...register("seed", { valueAsNumber: true })} />
                  <FieldError errors={[errors.seed]} />
                </Field>
              </div>
              <FieldDescription>{m.seedHint}</FieldDescription>
              {values.method === "spatial_grid" && (
                <Field data-invalid={Boolean(errors.block_km_text)}>
                  <FieldLabel htmlFor="block-km">{m.blockKm}</FieldLabel>
                  <Input
                    id="block-km"
                    inputMode="decimal"
                    className="num"
                    placeholder={m.blockKmPlaceholder}
                    aria-invalid={Boolean(errors.block_km_text)}
                    {...register("block_km_text")}
                  />
                  <FieldDescription>{m.blockKmHint}</FieldDescription>
                  <FieldError errors={[errors.block_km_text]} />
                </Field>
              )}
            </FieldSet>

            <FieldSet>
              <FieldLegend variant="label">{m.extraChecks}</FieldLegend>
              <Field data-invalid={Boolean(errors.null_models)}>
                <FieldLabel htmlFor="null-models" className="flex items-center gap-1">
                  {m.nullModels} <HelpTrigger target={{ article: "validation", section: "nulls" }} />
                </FieldLabel>
                <Controller
                  control={control}
                  name="null_models"
                  render={({ field }) => (
                    <Select
                      items={nullOptions.map((n) => ({ value: String(n), label: n === 0 ? m.nullModelsOff : String(n) }))}
                      value={String(field.value)}
                      onValueChange={(v) => v !== null && field.onChange(Number(v))}
                    >
                      <SelectTrigger id="null-models" className="w-40">
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        {nullOptions.map((n) => (
                          <SelectItem key={n} value={String(n)}>
                            {n === 0 ? m.nullModelsOff : n}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  )}
                />
                <FieldDescription>{m.nullModelsHint}</FieldDescription>
                <FieldError errors={[errors.null_models]} />
              </Field>
              <Field>
                <Controller
                  control={control}
                  name="aoa"
                  render={({ field }) => (
                    <Label className="flex items-center gap-2 font-normal">
                      <Checkbox checked={field.value} onCheckedChange={(c) => field.onChange(c === true)} />
                      {m.aoa} <HelpTrigger target={{ article: "uncertainty", section: "aoa" }} />
                    </Label>
                  )}
                />
                <FieldDescription>{m.aoaHint}</FieldDescription>
              </Field>
            </FieldSet>

            <ProjectionsField />

            <Button
              type="button"
              variant="ghost"
              size="sm"
              onClick={() => {
                reset(toFormValues(DEFAULT_MODEL, DEFAULT_VALIDATION, DEFAULT_SEED));
                useDraft.getState().setProjections([]);
              }}
            >
              <RotateCcw aria-hidden /> {m.resetRecommended}
            </Button>
          </CollapsibleContent>
        </Collapsible>
      </form>
    </WizardFrame>
  );
}
