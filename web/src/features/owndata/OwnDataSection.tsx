// The organisation's own data in the data step (ADR-0108): datasets of the organisation, import,
// matching of the dataset's names to the species of the study, and the sources of each species.

import { Database, Plus, Trash2 } from "lucide-react";
import { useState } from "react";

import { useDatasets, useDeleteDataset, useTaxonMatches } from "@/api/hooks";
import type { DatasetInfo, OwnSpeciesData, TaxonMatch } from "@/api/types";
import { ErrorState, SciName, Section } from "@/components/domain/common";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { HelpTrigger } from "@/features/help/HelpTrigger";
import { formatCount, formatDate } from "@/lib/format";
import { useDraft, type DraftSpecies } from "@/stores/draft";

import { ImportReportView, ImportWizard } from "./ImportWizard";
import { useOwnDataMessages } from "./messages";

type SourceChoice = "gbif" | "both" | "own" | "validation";

const SELECT = "h-8 max-w-full min-w-0 rounded-lg border border-input bg-transparent px-2 text-sm";

const choiceOf = (o: OwnSpeciesData | null | undefined): SourceChoice =>
  !o ? "gbif" : o.role === "validation" ? "validation" : o.gbif ? "both" : "own";

const ownOf = (choice: SourceChoice, values: string[]): OwnSpeciesData | null =>
  choice === "gbif" || values.length === 0
    ? null
    : { values, role: choice === "validation" ? "validation" : "training", gbif: choice !== "own" };

/** Names of the dataset assigned to a species (kept while the species uses GBIF only, so that
 *  switching sources does not lose the assignment). */
type Assignments = Record<string, number | null>;

function DatasetRow({ d, selected, onSelect }: { d: DatasetInfo; selected: boolean; onSelect: () => void }) {
  const m = useOwnDataMessages();
  const del = useDeleteDataset();
  const [confirm, setConfirm] = useState(false);
  const setOwnDataset = useDraft((s) => s.setOwnDataset);
  return (
    <li className="flex flex-wrap items-center gap-2 py-1.5 text-sm">
      <label className="flex min-w-0 flex-1 items-center gap-2">
        <input type="radio" name="own-dataset" checked={selected} onChange={onSelect} />
        <span className="min-w-0 truncate font-medium">{d.name}</span>
        {d.synthetic && <Badge variant="outline">{m.syntheticDataset}</Badge>}
      </label>
      <span className="num text-xs text-muted-foreground">
        {formatDate(d.created_at)} · {m.rows(formatCount(d.report.rows_imported))} · {m.presAbs(formatCount(d.report.presences), formatCount(d.report.absences))} · {d.crs} ·{" "}
        <span className="font-mono">{d.file_sha256.slice(0, 8)}</span>
      </span>
      {confirm ? (
        <Button
          size="xs"
          variant="destructive"
          disabled={del.isPending}
          onClick={() =>
            del.mutate(d.id, {
              onSuccess: () => {
                if (selected) setOwnDataset(null);
              },
            })
          }
          title={m.deleteHint}
        >
          {m.confirmDelete}
        </Button>
      ) : (
        <Button size="icon-xs" variant="ghost" aria-label={`${m.delete} ${d.name}`} title={m.deleteHint} onClick={() => setConfirm(true)}>
          <Trash2 aria-hidden />
        </Button>
      )}
    </li>
  );
}

function MatchCell({ t }: { t: TaxonMatch | undefined }) {
  const m = useOwnDataMessages();
  if (!t) return <span className="text-muted-foreground">…</span>;
  return (
    <span className="flex flex-col">
      {t.scientific_name ? <SciName name={t.scientific_name} /> : <span className="text-muted-foreground">—</span>}
      <span className="text-xs text-muted-foreground">
        {t.rank ? `${t.rank.toLowerCase()} · ` : ""}
        {m.matchType(t.match_type) ?? t.match_type}
        {t.key != null && <span className="font-mono"> · GBIF {t.key}</span>}
      </span>
    </span>
  );
}

/** Which species of the study each name of the dataset is used for, and with which sources. */
function SpeciesMapping({ d }: { d: DatasetInfo }) {
  const m = useOwnDataMessages();
  const species = useDraft((s) => s.species);
  const setSpeciesOwn = useDraft((s) => s.setSpeciesOwn);
  const addSpecies = useDraft((s) => s.addSpecies);
  const names = d.report.taxa.slice(0, 200).map((t) => t.value);
  const matches = useTaxonMatches(names.length > 0 ? names : null);
  const byName = new Map((matches.data ?? []).map((t) => [t.name, t]));
  // Assignment of each name: the species whose `own.values` contain it.
  const assigned: Assignments = Object.fromEntries(
    names.map((n) => [n, species.find((s) => s.own?.values.includes(n))?.key ?? null]),
  );
  const [choices, setChoices] = useState<Record<number, SourceChoice>>(() =>
    Object.fromEntries(species.map((s) => [s.key, s.own ? choiceOf(s.own) : "both"])),
  );
  const choice = (s: DraftSpecies): SourceChoice => (s.own ? choiceOf(s.own) : (choices[s.key] ?? "both"));

  const assign = (name: string, key: number | null) => {
    for (const s of species) {
      const has = s.own?.values.includes(name) ?? false;
      if (s.key === key && !has) {
        const values = [...(s.own?.values ?? []), name];
        setSpeciesOwn(s.key, ownOf(s.own ? choiceOf(s.own) : (choices[s.key] ?? "both"), values));
      } else if (s.key !== key && has) {
        const values = (s.own?.values ?? []).filter((v) => v !== name);
        setSpeciesOwn(s.key, ownOf(choiceOf(s.own), values));
      }
    }
  };
  const applyProposals = () => {
    for (const t of matches.data ?? []) {
      if (t.accepted && t.key != null && assigned[t.name] == null && species.some((s) => s.key === t.key)) assign(t.name, t.key);
    }
  };

  return (
    <div className="space-y-4">
      <Section title={m.species} id="own-species">
        <p className="text-xs text-muted-foreground">{m.speciesIntro}</p>
        {matches.isPending && <p className="text-sm text-muted-foreground">{m.matching}</p>}
        {matches.isError && <p className="text-sm text-state-warning">{m.matchFailed}</p>}
        <div className="overflow-x-auto">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>{m.value}</TableHead>
                <TableHead className="text-right">{m.recordsCol}</TableHead>
                <TableHead>{m.gbifMatch}</TableHead>
                <TableHead>{m.assigned}</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {d.report.taxa.slice(0, 200).map((t) => {
                const match = byName.get(t.value);
                const inStudy = match?.key != null && species.some((s) => s.key === match.key);
                return (
                  <TableRow key={t.value}>
                    <TableCell className="whitespace-normal">{t.value}</TableCell>
                    <TableCell className="num text-right">
                      {formatCount(t.presences)}/{formatCount(t.absences)}
                    </TableCell>
                    <TableCell className="whitespace-normal">
                      <MatchCell t={match} />
                    </TableCell>
                    <TableCell>
                      <div className="flex flex-wrap items-center gap-1">
                        <select
                          aria-label={`${m.assigned}: ${t.value}`}
                          className={SELECT}
                          value={assigned[t.value] ?? ""}
                          onChange={(e) => assign(t.value, e.target.value === "" ? null : Number(e.target.value))}
                        >
                          <option value="">{m.unassigned}</option>
                          {species.map((s) => (
                            <option key={s.key} value={s.key}>
                              {s.scientific_name}
                            </option>
                          ))}
                        </select>
                        {match?.key != null && match.scientific_name && !inStudy && match.match_type !== "HIGHERRANK" && (
                          <Button
                            size="xs"
                            variant="outline"
                            onClick={() =>
                              addSpecies({
                                key: match.key ?? 0,
                                scientific_name: match.scientific_name ?? t.value,
                                canonical_name: match.scientific_name ?? t.value,
                                rank: match.rank ?? "",
                                status: match.status ?? "",
                              })
                            }
                          >
                            <Plus aria-hidden /> {m.addToStudy}
                          </Button>
                        )}
                      </div>
                    </TableCell>
                  </TableRow>
                );
              })}
            </TableBody>
          </Table>
        </div>
        {(matches.data ?? []).some((t) => t.accepted) && (
          <Button size="sm" variant="outline" onClick={applyProposals}>
            {m.applyProposals}
          </Button>
        )}
      </Section>

      <Section title={m.sources} id="own-sources">
        <p className="text-xs text-muted-foreground">{m.sourcesIntro}</p>
        <ul className="space-y-3">
          {species.map((s) => {
            const values = s.own?.values ?? [];
            const c = choice(s);
            return (
              <li key={s.key} className="space-y-1">
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <SciName name={s.scientific_name} />
                  <select
                    aria-label={`${m.sources}: ${s.scientific_name}`}
                    className={SELECT}
                    value={values.length === 0 ? "gbif" : c}
                    disabled={values.length === 0}
                    onChange={(e) => {
                      const next = e.target.value as SourceChoice;
                      setChoices({ ...choices, [s.key]: next });
                      setSpeciesOwn(s.key, ownOf(next, values));
                    }}
                  >
                    {(["gbif", "both", "own", "validation"] as const).map((k) => (
                      <option key={k} value={k}>
                        {m.source(k)}
                      </option>
                    ))}
                  </select>
                </div>
                <p className="text-xs text-muted-foreground">
                  {values.length === 0 ? m.noValues : values.join(", ")}
                  {values.length > 0 && m.sourceHint(c) ? ` — ${m.sourceHint(c)}` : ""}
                </p>
              </li>
            );
          })}
        </ul>
      </Section>
    </div>
  );
}

export function OwnDataSection() {
  const m = useOwnDataMessages();
  const list = useDatasets();
  const ownDataset = useDraft((s) => s.ownDataset);
  const setOwnDataset = useDraft((s) => s.setOwnDataset);
  const [importing, setImporting] = useState(false);
  const [last, setLast] = useState<DatasetInfo | null>(null);
  const datasets = list.data ?? [];
  const current = datasets.find((d) => d.id === ownDataset?.id) ?? null;
  const select = (d: DatasetInfo | null) =>
    setOwnDataset(d ? { id: d.id, records_sha256: d.records_sha256, name: d.name } : null);

  return (
    <Collapsible defaultOpen={ownDataset != null}>
      <CollapsibleTrigger className="flex w-full items-center justify-between rounded-md text-sm font-medium hover:underline">
        <span className="flex shrink-0 items-center gap-1.5">
          <Database className="size-4 shrink-0" aria-hidden /> {m.title}
        </span>
        <span className="min-w-0 truncate pl-2 text-xs font-normal text-muted-foreground">{current ? current.name : m.none}</span>
      </CollapsibleTrigger>
      <CollapsibleContent className="space-y-4 pt-3">
        <p className="flex items-start gap-1 text-xs text-muted-foreground">
          {m.intro} <HelpTrigger target={{ article: "field-data", section: "overview" }} />
        </p>
        <Section title={m.datasets} id="own-datasets">
          {list.isError && <ErrorState error={list.error} onRetry={() => void list.refetch()} />}
          {list.isSuccess && datasets.length === 0 && <p className="text-sm text-muted-foreground">{m.noDatasets}</p>}
          {datasets.length > 0 && (
            <ul className="divide-y">
              <li className="py-1.5 text-sm">
                <label className="flex items-center gap-2">
                  <input type="radio" name="own-dataset" checked={ownDataset == null} onChange={() => select(null)} />
                  {m.none}
                </label>
              </li>
              {datasets.map((d) => (
                <DatasetRow key={d.id} d={d} selected={d.id === ownDataset?.id} onSelect={() => select(d)} />
              ))}
            </ul>
          )}
          {importing ? (
            <ImportWizard
              onCancel={() => setImporting(false)}
              onDone={(d) => {
                setImporting(false);
                setLast(d);
                select(d);
              }}
            />
          ) : (
            <Button size="sm" variant="outline" onClick={() => setImporting(true)}>
              <Plus aria-hidden /> {m.importFile}
            </Button>
          )}
          {last && last.id === current?.id && <ImportReportView d={last} />}
        </Section>
        {current && <SpeciesMapping key={current.id} d={current} />}
      </CollapsibleContent>
    </Collapsible>
  );
}
