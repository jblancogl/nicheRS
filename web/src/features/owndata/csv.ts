// Reading an organisation's CSV file in the browser (ADR-0108). The file never leaves the
// browser: it is decoded and parsed here, and only the columns the user chooses are sent, as
// strings (numbers, dates and coordinates are interpreted on the server).

export interface ParsedCsv {
  fileName: string;
  /** SHA-256 of the original bytes (hex), so the organisation can later prove which file was used. */
  sha256: string;
  encoding: "utf-8" | "windows-1252";
  delimiter: string;
  headers: string[];
  /** Data rows (blank lines dropped) with the file line each starts on (header = line 1). */
  rows: string[][];
  lines: number[];
}

/** Largest file read in the browser (the request sent is far smaller: chosen columns only). */
export const MAX_FILE_BYTES = 50 * 1024 * 1024;

async function sha256Hex(buf: ArrayBuffer): Promise<string> {
  const d = await crypto.subtle.digest("SHA-256", buf);
  return Array.from(new Uint8Array(d), (b) => b.toString(16).padStart(2, "0")).join("");
}

/** UTF-8 when the bytes are valid UTF-8, otherwise Windows-1252 (French Excel exports). */
export function decode(buf: ArrayBuffer): { text: string; encoding: ParsedCsv["encoding"] } {
  let text: string;
  let encoding: ParsedCsv["encoding"] = "utf-8";
  try {
    text = new TextDecoder("utf-8", { fatal: true }).decode(buf);
  } catch {
    text = new TextDecoder("windows-1252").decode(buf);
    encoding = "windows-1252";
  }
  return { text: text.replace(/^\uFEFF/, ""), encoding };
}

/** The separator used most often outside quotes on the first line: `;`, `,` or a tab. */
export function detectDelimiter(text: string): string {
  const counts: Record<string, number> = { ";": 0, ",": 0, "\t": 0 };
  let quoted = false;
  for (const c of text) {
    if (c === '"') quoted = !quoted;
    else if (!quoted && (c === "\n" || c === "\r")) break;
    else if (!quoted && c in counts) counts[c] = (counts[c] ?? 0) + 1;
  }
  return Object.entries(counts).sort((a, b) => b[1] - a[1])[0]?.[0] ?? ",";
}

/** RFC 4180 parser: quoted fields (with doubled quotes, separators and line breaks inside),
 *  CRLF or LF. Rows whose fields are all empty are dropped; `lines` keeps the file line. */
export function parseCsv(text: string, delimiter: string): { rows: string[][]; lines: number[] } {
  const rows: string[][] = [];
  const lines: number[] = [];
  let field = "";
  let row: string[] = [];
  let quoted = false;
  let line = 1;
  let rowLine = 1;
  const endRow = () => {
    row.push(field);
    field = "";
    if (row.some((f) => f.trim() !== "")) {
      rows.push(row);
      lines.push(rowLine);
    }
    row = [];
  };
  for (let i = 0; i < text.length; i++) {
    const c = text[i];
    if (quoted) {
      if (c === '"') {
        if (text[i + 1] === '"') {
          field += '"';
          i++;
        } else quoted = false;
      } else {
        if (c === "\n") line++;
        field += c;
      }
    } else if (c === '"' && field === "") {
      quoted = true;
    } else if (c === delimiter) {
      row.push(field);
      field = "";
    } else if (c === "\n" || c === "\r") {
      if (c === "\r" && text[i + 1] === "\n") i++;
      endRow();
      line++;
      rowLine = line;
    } else {
      field += c;
    }
  }
  if (field !== "" || row.length > 0) endRow();
  return { rows, lines };
}

/** Read, decode, fingerprint and parse a file chosen by the user. */
export async function readCsv(file: File): Promise<ParsedCsv> {
  if (file.size > MAX_FILE_BYTES) throw new Error("too_large");
  const buf = await file.arrayBuffer();
  const sha256 = await sha256Hex(buf);
  const { text, encoding } = decode(buf);
  const delimiter = detectDelimiter(text);
  const { rows, lines } = parseCsv(text, delimiter);
  const [headers = [], ...data] = rows;
  return { fileName: file.name, sha256, encoding, delimiter, headers: headers.map((h) => h.trim()), rows: data, lines: lines.slice(1) };
}

export type Field = "taxon" | "x" | "y" | "date" | "uncertainty" | "status" | "survey";

/** Usual column names of field exports (GeoNature/SINP, spreadsheets, GPS), lower case. */
const GUESSES: Record<Field, string[]> = {
  taxon: ["nom_cite", "nom_valide", "lb_nom", "taxon", "species", "espece", "espèce", "scientificname", "scientific_name", "nom_scientifique", "especie"],
  x: ["x", "x_l93", "x_2154", "longitude", "lon", "lng", "decimallongitude", "x_centroid_4326", "easting", "coord_x"],
  y: ["y", "y_l93", "y_2154", "latitude", "lat", "decimallatitude", "y_centroid_4326", "northing", "coord_y"],
  date: ["date_debut", "date", "date_obs", "eventdate", "date_observation", "fecha", "year", "annee", "année"],
  uncertainty: ["precision", "précision", "precision_m", "coordinateuncertaintyinmeters", "incertitude", "uncertainty", "precision_metres"],
  status: ["statut_observation", "statut", "status", "occurrencestatus", "presence", "présence", "estado"],
  survey: ["protocole", "protocol", "id_protocole", "survey", "samplingprotocol", "jdd_nom", "campagne", "releve", "relevé"],
};

/** Index of the column a field most likely comes from, or -1. */
export function guessColumn(headers: string[], field: Field): number {
  const lower = headers.map((h) => h.toLowerCase());
  for (const g of GUESSES[field]) {
    const i = lower.indexOf(g);
    if (i >= 0) return i;
  }
  return -1;
}

/** Values of a status column usually meaning "surveyed, not found". */
const ABSENT = ["absent", "absence", "non observé", "non observe", "non-observé", "no observado", "ausente", "0", "no", "non", "false"];

export const looksAbsent = (v: string) => ABSENT.includes(v.trim().toLowerCase());

/** One column of the rows (missing cells as empty strings). */
export const column = (p: ParsedCsv, i: number) => p.rows.map((r) => r[i] ?? "");

/** Coordinate systems suggested by the values: degrees or a projected system. */
export function looksLikeDegrees(p: ParsedCsv, xi: number, yi: number): boolean {
  const sample = p.rows.slice(0, 50);
  const num = (s: string | undefined) => Number((s ?? "").replace(",", ".").trim());
  return sample.length > 0 && sample.every((r) => Math.abs(num(r[xi])) <= 180 && Math.abs(num(r[yi])) <= 90);
}
