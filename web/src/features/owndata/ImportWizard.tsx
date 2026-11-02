// Import of the organisation's CSV (ADR-0108): the file is read and parsed in the browser; the
// user maps columns and chooses the coordinate system; only the chosen columns are sent.

import { FileUp } from "lucide-react";
import { useMemo, useState } from "react";

import { useCoordinateSystems, useImportDataset } from "@/api/hooks";
import type { DatasetInfo, ImportRequest } from "@/api/types";
import { ErrorState, WarningsList } from "@/components/domain/common";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { formatCount } from "@/lib/format";

import { column, guessColumn, looksAbsent, looksLikeDegrees, readCsv, type Field, type ParsedCsv } from "./csv";
import { useOwnDataMessages } from "./messages";

const FIELDS: { id: Field; required: boolean }[] = [
  { id: "taxon", required: true },
  { id: "x", required: true },
  { id: "y", required: true },
  { id: "date", required: false },
  { id: "uncertainty", required: false },
  { id: "status", required: false },
  { id: "survey", required: false },
];

const SELECT = "h-8 w-full rounded-lg border border-input bg-transparent px-2 text-sm";

/** The import report: what was kept, what was rejected and why (file lines). */
export function ImportReportView({ d }: { d: DatasetInfo }) {
  const m = useOwnDataMessages();
  const r = d.report;
  return (
    <div className="space-y-2 rounded-lg border p-3 text-sm">
      <p className="font-medium">
        {m.report} · {m.readImported(formatCount(r.rows_read), formatCount(r.rows_imported))}
      </p>
      <p className="text-muted-foreground">{m.presAbs(formatCount(r.presences), formatCount(r.absences))}</p>
      {r.rejected.length > 0 && (
        <div>
          <p className="font-medium">{m.rejected}</p>
          <ul className="list-disc pl-5">
            {r.rejected.map((x) => (
              <li key={x.reason}>
                {m.rejection(x.reason) ?? x.reason}: {formatCount(x.count)} ({m.lines(x.lines.join(", "))}
                {x.count > x.lines.length ? "…" : ""})
              </li>
            ))}
          </ul>
        </div>
      )}
      <WarningsList warnings={r.warnings} />
    </div>
  );
}

export function ImportWizard({ onDone, onCancel }: { onDone: (d: DatasetInfo) => void; onCancel: () => void }) {
  const m = useOwnDataMessages();
  const crsList = useCoordinateSystems();
  const imp = useImportDataset();
  const [file, setFile] = useState<ParsedCsv | null>(null);
  const [readError, setReadError] = useState<string | null>(null);
  const [reading, setReading] = useState(false);
  const [cols, setCols] = useState<Record<Field, number>>({ taxon: -1, x: -1, y: -1, date: -1, uncertainty: -1, status: -1, survey: -1 });
  const [crs, setCrs] = useState("EPSG:2154");
  const [absent, setAbsent] = useState<string[]>([]);
  const [name, setName] = useState("");

  const onFile = async (f: File | undefined) => {
    if (!f) return;
    setReading(true);
    setReadError(null);
    try {
      const p = await readCsv(f);
      if (p.rows.length === 0) {
        setReadError(m.noRows);
        setFile(null);
        return;
      }
      const guessed = Object.fromEntries(FIELDS.map((x) => [x.id, guessColumn(p.headers, x.id)])) as Record<Field, number>;
      setCols(guessed);
      setFile(p);
      setName(f.name.replace(/\.[^.]+$/, ""));
      if (guessed.x >= 0 && guessed.y >= 0 && looksLikeDegrees(p, guessed.x, guessed.y)) setCrs("EPSG:4326");
      setAbsent(guessed.status >= 0 ? [...new Set(column(p, guessed.status).map((v) => v.trim()))].filter(looksAbsent) : []);
    } catch (e) {
      setReadError(e instanceof Error && e.message === "too_large" ? m.tooLarge : m.unreadable);
      setFile(null);
    } finally {
      setReading(false);
    }
  };

  const statusValues = useMemo(() => {
    if (!file || cols.status < 0) return [];
    const counts = new Map<string, number>();
    for (const v of column(file, cols.status)) counts.set(v.trim(), (counts.get(v.trim()) ?? 0) + 1);
    return [...counts.entries()].sort((a, b) => b[1] - a[1]);
  }, [file, cols.status]);

  const degrees = file && cols.x >= 0 && cols.y >= 0 ? looksLikeDegrees(file, cols.x, cols.y) : null;
  const ready = file !== null && cols.taxon >= 0 && cols.x >= 0 && cols.y >= 0 && name.trim() !== "";
  const chosen = FIELDS.filter((f) => cols[f.id] >= 0);

  const submit = () => {
    if (!file || !ready) return;
    const pick = (f: Field) => (cols[f] >= 0 ? column(file, cols[f]) : null);
    const req: ImportRequest = {
      name: name.trim(),
      file_name: file.fileName,
      file_sha256: file.sha256,
      crs,
      columns: {
        taxon: pick("taxon") ?? [],
        x: pick("x") ?? [],
        y: pick("y") ?? [],
        date: pick("date"),
        uncertainty: pick("uncertainty"),
        status: pick("status"),
        survey: pick("survey"),
      },
      lines: file.lines,
      absent_values: cols.status >= 0 ? absent : [],
      mapping: Object.fromEntries(chosen.map((f) => [f.id, file.headers[cols[f.id]] ?? ""])),
    };
    imp.mutate(req, { onSuccess: onDone });
  };

  return (
    <div className="space-y-4 rounded-lg border p-3">
      <div className="space-y-1.5">
        <Label htmlFor="own-file">{m.chooseFile}</Label>
        <Input id="own-file" type="file" accept=".csv,.txt,text/csv" onChange={(e) => void onFile(e.target.files?.[0])} />
        {reading && <p className="text-sm text-muted-foreground">{m.reading}</p>}
        {readError && <p className="text-sm text-destructive">{readError}</p>}
      </div>

      {file && (
        <>
          <p className="text-xs text-muted-foreground">
            {m.fileSummary(formatCount(file.rows.length), formatCount(file.headers.length), file.encoding)}
          </p>
          <div className="overflow-x-auto">
            <p className="mb-1 text-xs font-medium">{m.extract}</p>
            <Table>
              <TableHeader>
                <TableRow>
                  {file.headers.map((h, i) => (
                    <TableHead key={i}>{h}</TableHead>
                  ))}
                </TableRow>
              </TableHeader>
              <TableBody>
                {file.rows.slice(0, 5).map((r, i) => (
                  <TableRow key={i}>
                    {file.headers.map((_, j) => (
                      <TableCell key={j} className="max-w-40 truncate">
                        {r[j] ?? ""}
                      </TableCell>
                    ))}
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>

          <fieldset className="grid grid-cols-1 gap-3 sm:grid-cols-2">
            <legend className="mb-1 text-sm font-medium">{m.mapping}</legend>
            {FIELDS.map((f) => (
              <div key={f.id} className="space-y-1">
                <Label htmlFor={`own-col-${f.id}`}>
                  {m.field(f.id)} <span className="text-xs font-normal text-muted-foreground">({f.required ? m.required : m.optional})</span>
                </Label>
                <select
                  id={`own-col-${f.id}`}
                  className={SELECT}
                  value={cols[f.id]}
                  onChange={(e) => {
                    const v = Number(e.target.value);
                    setCols({ ...cols, [f.id]: v });
                    if (f.id === "status") setAbsent(v >= 0 ? [...new Set(column(file, v).map((x) => x.trim()))].filter(looksAbsent) : []);
                  }}
                >
                  <option value={-1}>{m.notUsed}</option>
                  {file.headers.map((h, i) => (
                    <option key={i} value={i}>
                      {h || `#${i + 1}`}
                    </option>
                  ))}
                </select>
              </div>
            ))}
          </fieldset>

          <div className="space-y-1">
            <Label htmlFor="own-crs">{m.crs}</Label>
            <select id="own-crs" className={SELECT} value={crs} onChange={(e) => setCrs(e.target.value)}>
              {(crsList.data ?? [{ code: "EPSG:4326", name: "WGS 84", usage: "" }, { code: "EPSG:2154", name: "RGF93 / Lambert-93", usage: "" }]).map((c) => (
                <option key={c.code} value={c.code}>
                  {c.code} — {c.name}
                </option>
              ))}
            </select>
            {degrees === true && crs !== "EPSG:4326" && <p className="text-xs text-state-warning">{m.crsDegreesHint}</p>}
            {degrees === false && crs === "EPSG:4326" && <p className="text-xs text-state-warning">{m.crsProjectedHint}</p>}
          </div>

          {cols.status >= 0 && (
            <fieldset className="space-y-1.5">
              <legend className="mb-1 text-sm font-medium">{m.absentValues}</legend>
              {statusValues.length === 0 && <p className="text-sm text-muted-foreground">{m.noStatusValues}</p>}
              {statusValues.length > 30 && <p className="text-xs text-state-warning">{m.tooManyValues(formatCount(statusValues.length))}</p>}
              <div className="flex flex-wrap gap-x-4 gap-y-1">
                {statusValues.slice(0, 30).map(([v, n]) => (
                  <Label key={v} className="flex items-center gap-2 font-normal">
                    <Checkbox
                      checked={absent.includes(v)}
                      onCheckedChange={(c) => setAbsent(c ? [...absent, v] : absent.filter((x) => x !== v))}
                    />
                    {v === "" ? "∅" : v} <span className="num text-xs text-muted-foreground">({formatCount(n)})</span>
                  </Label>
                ))}
              </div>
            </fieldset>
          )}

          <div className="space-y-1">
            <Label htmlFor="own-name">{m.name}</Label>
            <Input id="own-name" value={name} maxLength={200} onChange={(e) => setName(e.target.value)} />
          </div>

          <p className="text-xs text-muted-foreground">{m.minimisation(chosen.map((f) => `${m.field(f.id)} (${file.headers[cols[f.id]] ?? ""})`).join(", "))}</p>
          {imp.isError && <ErrorState error={imp.error} />}
        </>
      )}

      <div className="flex justify-end gap-2">
        <Button variant="ghost" onClick={onCancel}>
          {m.cancel}
        </Button>
        <Button onClick={submit} disabled={!ready || imp.isPending}>
          <FileUp aria-hidden /> {imp.isPending ? m.importing : m.import}
        </Button>
      </div>
    </div>
  );
}
