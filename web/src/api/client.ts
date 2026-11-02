// Typed fetch wrapper for the NicheRS HTTP API v1 (contract: ./types.ts).
// Errors are normalised into ApiError: RFC 9457 problem+json bodies are parsed; network
// failures (server down, offline) become status 0 so the UI can show "API unreachable".

import type {
  Analysis,
  AnalysisRequest,
  AreaCheck,
  CrsOption,
  DatasetInfo,
  ImportRequest,
  TaxonMatch,
  AuditPage,
  AuditQuery,
  UserPage,
  UserQuery,
  DataPreview,
  DataPreviewRequest,
  Feedback,
  FeedbackRequest,
  FeedbackStatus,
  GbifAccountInfo,
  GrantAdminRequest,
  GbifSettings,
  GbifWait,
  Health,
  Job,
  JobResults,
  JobSpec,
  LayerInfo,
  ActivityRow,
  InviteInfo,
  LinkCreated,
  LinkInfo,
  Me,
  Org,
  PlaceGeometry,
  PlaceHit,
  Project,
  OrgMetrics,
  OrgOverview,
  OrgProfile,
  SystemStatus,
  UsageSummary,
  UserInfo,
  OccurrencePoint,
  ProblemDetails,
  SampleValue,
  SpeciesHit,
  StudyArea,
} from "./types";
import { apiMessages } from "./messages";

export const API_BASE = "/api/v1";

export class ApiError extends Error {
  readonly status: number;
  readonly problem: ProblemDetails | null;

  constructor(status: number, message: string, problem: ProblemDetails | null) {
    super(message);
    this.name = "ApiError";
    this.status = status;
    this.problem = problem;
  }

  /** True when the server could not be reached at all. */
  get unreachable(): boolean {
    return this.status === 0;
  }
}

function isProblem(value: unknown): value is ProblemDetails {
  return (
    typeof value === "object" &&
    value !== null &&
    "title" in value &&
    typeof value.title === "string"
  );
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  let res: Response;
  try {
    res = await fetch(`${API_BASE}${path}`, {
      ...init,
      headers: {
        Accept: "application/json, application/problem+json",
        ...(init?.body ? { "Content-Type": "application/json" } : {}),
        ...init?.headers,
      },
    });
  } catch (err) {
    if (err instanceof DOMException && err.name === "AbortError") throw err;
    throw new ApiError(
      0,
      apiMessages.get().unreachable,
      null,
    );
  }

  const contentType = res.headers.get("content-type") ?? "";
  // Hosted mode: the session expired or was closed. Send the user to sign in, then back here.
  if (res.status === 401 && !path.startsWith("/auth/")) {
    redirectToLogin();
  }
  if (!res.ok) {
    let problem: ProblemDetails | null = null;
    if (contentType.includes("json")) {
      try {
        const body: unknown = await res.json();
        if (isProblem(body)) problem = body;
      } catch {
        /* body not JSON: fall through */
      }
    }
    // A proxy with no backend answers 502/504 with an HTML page: treat as unreachable.
    if (!problem && (res.status === 502 || res.status === 503 || res.status === 504)) {
      throw new ApiError(
        0,
        apiMessages.get().notResponding,
        null,
      );
    }
    const message = problem
      ? problem.detail
        ? `${problem.title}: ${problem.detail}`
        : problem.title
      : apiMessages.get().httpFailed(res.status);
    throw new ApiError(res.status, message, problem);
  }
  if (res.status === 204) return undefined as T;
  if (!contentType.includes("json")) {
    throw new ApiError(
      res.status,
      apiMessages.get().notJson,
      null,
    );
  }
  return (await res.json()) as T;
}

/** Go to the sign-in page, remembering the current page (same origin only). */
export function redirectToLogin() {
  const here = window.location.pathname + window.location.search;
  if (window.location.pathname.startsWith("/login")) return;
  window.location.assign(`/login?next=${encodeURIComponent(here)}`);
}

const enc = encodeURIComponent;

/** `?a=1&b=x` from the defined, non-empty values; "" when there are none. */
const query = (q: Record<string, string | number | undefined>) => {
  const p = new URLSearchParams();
  for (const [k, v] of Object.entries(q)) if (v !== undefined && v !== "") p.set(k, String(v));
  const s = p.toString();
  return s ? `?${s}` : "";
};

const post = (body: unknown): RequestInit => ({ method: "POST", body: JSON.stringify(body) });

export const api = {
  health: (signal?: AbortSignal) => request<Health>("/health", { signal }),
  searchSpecies: (q: string, signal?: AbortSignal) =>
    request<SpeciesHit[]>(`/species/search?q=${enc(q)}`, { signal }),
  // ?demo=true returns the synthetic demo predictors (ids demo_*)
  layers: (demo: boolean, signal?: AbortSignal) =>
    request<LayerInfo[]>(demo ? "/layers?demo=true" : "/layers", { signal }),
  checkArea: (area: StudyArea, signal?: AbortSignal) =>
    request<AreaCheck>("/study-areas/check", { method: "POST", body: JSON.stringify({ area }), signal }),
  previewOccurrences: (body: DataPreviewRequest, signal?: AbortSignal) =>
    request<DataPreview>("/occurrences/preview", {
      method: "POST",
      body: JSON.stringify(body),
      signal,
    }),
  // The organisation's own datasets (ADR-0108): only the chosen columns are sent, never the file.
  datasets: (signal?: AbortSignal) => request<DatasetInfo[]>("/datasets", { signal }),
  importDataset: (b: ImportRequest) => request<DatasetInfo>("/datasets", post(b)),
  deleteDataset: (id: string) => request<undefined>(`/datasets/${enc(id)}`, { method: "DELETE" }),
  coordinateSystems: (signal?: AbortSignal) => request<CrsOption[]>("/coordinate-systems", { signal }),
  /** Only the names are sent to GBIF. */
  matchSpecies: (names: string[]) => request<TaxonMatch[]>("/species/match", post({ names })),
  listJobs: (signal?: AbortSignal) => request<Job[]>("/jobs", { signal }),
  createJob: (spec: JobSpec, projectId?: string | null) =>
    request<Job>(projectId ? `/jobs?project_id=${enc(projectId)}` : "/jobs", { method: "POST", body: JSON.stringify(spec) }),
  updateJob: (id: string, b: { name?: string; project_id?: string | null }) =>
    request<Job>(`/jobs/${enc(id)}`, { method: "PATCH", body: JSON.stringify(b) }),
  projects: (signal?: AbortSignal) => request<Project[]>("/projects", { signal }),
  places: (q: string, signal?: AbortSignal) => request<PlaceHit[]>(`/places?q=${enc(q)}&limit=12`, { signal }),
  placesAt: (lng: number, lat: number, signal?: AbortSignal) =>
    request<PlaceHit[]>(`/places/at?lng=${lng}&lat=${lat}`, { signal }),
  place: (id: string, signal?: AbortSignal) => request<PlaceGeometry>(`/places/${enc(id)}`, { signal }),
  createProject: (name: string) => request<Project>("/projects", post({ name })),
  renameProject: (id: string, name: string) =>
    request<Project>(`/projects/${enc(id)}`, { method: "PATCH", body: JSON.stringify({ name }) }),
  deleteProject: (id: string) => request<undefined>(`/projects/${enc(id)}`, { method: "DELETE" }),
  getJob: (id: string, signal?: AbortSignal) => request<Job>(`/jobs/${enc(id)}`, { signal }),
  cancelJob: (id: string) => request<Job>(`/jobs/${enc(id)}/cancel`, { method: "POST" }),
  gbifWait: (id: string, signal?: AbortSignal) => request<GbifWait | null>(`/jobs/${enc(id)}/gbif-wait`, { signal }),
  skipDownload: (id: string) => request<undefined>(`/jobs/${enc(id)}/skip-download`, { method: "POST" }),
  deleteJob: (id: string) => request<undefined>(`/jobs/${enc(id)}`, { method: "DELETE" }),
  // Hosted mode (404 in local mode).
  me: (signal?: AbortSignal) => request<Me>("/auth/me", { signal }),
  linkInfo: (token: string) => request<LinkInfo>("/auth/link", post({ token })),
  acceptInvite: (b: { token: string; name: string; email: string; password: string }) =>
    request<Me>("/auth/accept", post(b)),
  resetPassword: (b: { token: string; password: string }) => request<Me>("/auth/reset", post(b)),
  login: (b: { email: string; password: string }) => request<Me>("/auth/login", post(b)),
  logout: () => request<undefined>("/auth/logout", { method: "POST" }),
  setLang: (lang: string) => request<undefined>("/auth/lang", post({ lang })),
  /** The signed-in account's properties and GBIF account (ADR-0110). */
  account: {
    rename: (name: string) => request<undefined>("/account", { method: "PATCH", body: JSON.stringify({ name }) }),
    changePassword: (b: { current: string; new: string }) => request<undefined>("/account/password", post(b)),
    gbif: (signal?: AbortSignal) => request<GbifSettings>("/account/gbif", { signal }),
    setGbif: (b: { username: string; password: string }) =>
      request<GbifSettings>("/account/gbif", { method: "PUT", body: JSON.stringify(b) }),
    clearGbif: () => request<undefined>("/account/gbif", { method: "DELETE" }),
    checkGbif: () => request<GbifAccountInfo>("/account/gbif/check", { method: "POST" }),
  },
  sendFeedback: (b: FeedbackRequest) => request<{ id: string }>("/feedback", post(b)),
  admin: {
    feedback: (signal?: AbortSignal) => request<Feedback[]>("/admin/feedback", { signal }),
    setFeedbackStatus: (id: string, status: FeedbackStatus) =>
      request<undefined>(`/admin/feedback/${enc(id)}`, { method: "PATCH", body: JSON.stringify({ status }) }),
    deleteFeedback: (id: string) => request<undefined>(`/admin/feedback/${enc(id)}`, { method: "DELETE" }),
    orgs: (signal?: AbortSignal) => request<OrgOverview[]>("/admin/orgs", { signal }),
    createOrg: (b: { name: string; monthly_quota: number }) => request<Org>("/admin/orgs", post(b)),
    updateOrg: (id: string, b: { monthly_quota?: number; disabled?: boolean }) =>
      request<Org>(`/admin/orgs/${enc(id)}`, { method: "PATCH", body: JSON.stringify(b) }),
    invites: (org: string, signal?: AbortSignal) =>
      request<InviteInfo[]>(`/admin/orgs/${enc(org)}/invites`, { signal }),
    createInvite: (org: string) =>
      request<LinkCreated>(`/admin/orgs/${enc(org)}/invites`, { method: "POST" }),
    revokeInvite: (id: string) =>
      request<undefined>(`/admin/invites/${enc(id)}/revoke`, { method: "POST" }),
    org: (id: string, signal?: AbortSignal) => request<OrgOverview>(`/admin/orgs/${enc(id)}`, { signal }),
    users: (q: UserQuery & { offset?: number }, signal?: AbortSignal) =>
      request<UserPage>(`/admin/users${query({ org: q.org, q: q.q, limit: q.limit, offset: q.offset })}`, { signal }),
    user: (id: string, signal?: AbortSignal) => request<UserInfo>(`/admin/users/${enc(id)}`, { signal }),
    setUserDisabled: (email: string, disabled: boolean) =>
      request<undefined>("/admin/users/disable", post({ email, disabled })),
    resetLink: (email: string) => request<LinkCreated>("/admin/users/reset", post({ email })),
    deleteUser: (email: string, confirm_email: string) =>
      request<undefined>("/admin/users/delete", post({ email, confirm_email })),
    usage: (month: string | null, signal?: AbortSignal) =>
      request<UsageSummary[]>(month ? `/admin/usage?month=${enc(month)}` : "/admin/usage", { signal }),
    activity: (org: string | undefined, signal?: AbortSignal) => request<ActivityRow[]>(`/admin/activity${query({ org })}`, { signal }),
    system: (signal?: AbortSignal) => request<SystemStatus>("/admin/system", { signal }),
    metrics: (month: string | null, signal?: AbortSignal) =>
      request<OrgMetrics[]>(month ? `/admin/metrics?month=${enc(month)}` : "/admin/metrics", { signal }),
    setManager: (email: string, manager: boolean) => request<undefined>("/admin/users/manager", post({ email, manager })),
    grantAdmin: (b: GrantAdminRequest) => request<undefined>("/admin/users/admin", post(b)),
    audit: (q: AuditQuery & { before?: number }, signal?: AbortSignal) =>
      request<AuditPage>(`/admin/audit${query({ org: q.org, user: q.user, limit: q.limit, before: q.before })}`, { signal }),
  },
  /** Organisation managers (ADR-0109): their own organisation. */
  org: {
    feedback: (signal?: AbortSignal) => request<Feedback[]>("/org/feedback", { signal }),
    audit: (before: number | undefined, signal?: AbortSignal) => request<AuditPage>(`/org/audit${query({ before })}`, { signal }),
    profile: (signal?: AbortSignal) => request<OrgProfile>("/org", { signal }),
    setGbif: (b: { username: string; password: string }) =>
      request<GbifSettings>("/org/gbif", { method: "PUT", body: JSON.stringify(b) }),
    clearGbif: () => request<undefined>("/org/gbif", { method: "DELETE" }),
    checkGbif: () => request<GbifAccountInfo>("/org/gbif/check", { method: "POST" }),
    members: (signal?: AbortSignal) => request<UserInfo[]>("/org/members", { signal }),
    invites: (signal?: AbortSignal) => request<InviteInfo[]>("/org/invites", { signal }),
    invite: () => request<LinkCreated>("/org/invites", { method: "POST" }),
    revokeInvite: (id: string) => request<undefined>(`/org/invites/${enc(id)}/revoke`, { method: "POST" }),
    resetLink: (email: string) => request<LinkCreated>("/org/members/reset", post({ email })),
    setDisabled: (email: string, disabled: boolean) => request<undefined>("/org/members/disable", post({ email, disabled })),
  },
  results: (id: string, signal?: AbortSignal) =>
    request<JobResults>(`/jobs/${enc(id)}/results`, { signal }),
  manifest: (id: string, signal?: AbortSignal) =>
    request<unknown>(`/jobs/${enc(id)}/manifest`, { signal }),
  occurrences: (id: string, key: number, signal?: AbortSignal) =>
    request<OccurrencePoint[]>(`/jobs/${enc(id)}/species/${key}/occurrences`, { signal }),
  /** Decision-support analyses of a finished study (ADR-0018). */
  analyses: (id: string, signal?: AbortSignal) => request<Analysis[]>(`/jobs/${enc(id)}/analyses`, { signal }),
  createAnalysis: (id: string, body: AnalysisRequest) => request<Analysis>(`/jobs/${enc(id)}/analyses`, post(body)),
  deleteAnalysis: (id: string, aid: string) =>
    request<undefined>(`/jobs/${enc(id)}/analyses/${enc(aid)}`, { method: "DELETE" }),
  sample: (id: string, key: number, lng: number, lat: number, signal?: AbortSignal) =>
    request<SampleValue>(
      `/jobs/${enc(id)}/species/${key}/sample?lng=${lng.toFixed(6)}&lat=${lat.toFixed(6)}`,
      { signal },
    ),
};

export const urls = {
  /** Screenshot or recording sent with a feedback (author, its managers, administrators). */
  feedbackFile: (id: string, file: string) => `${API_BASE}/feedback/${enc(id)}/files/${enc(file)}`,
  usageCsv: (month: string | null) =>
    `${API_BASE}/admin/usage.csv${month ? `?month=${enc(month)}` : ""}`,
  /** Absolute one-time link from a server `path` ("/invite#…"). */
  link: (path: string) => `${window.location.origin}${path}`,
  events: (id: string) => `${API_BASE}/jobs/${enc(id)}/events`,
  manifest: (id: string) => `${API_BASE}/jobs/${enc(id)}/manifest`,
  file: (id: string, key: number, name: string) =>
    `${API_BASE}/jobs/${enc(id)}/species/${key}/files/${enc(name)}`,
  /** Small suitability map of a species (workspace cards). */
  thumbnail: (id: string, key: number) => `${API_BASE}/jobs/${enc(id)}/species/${key}/thumbnail.png`,
  /** HTML report shown in the browser (sandboxed by the server). */
  report: (id: string, key: number) => `${API_BASE}/jobs/${enc(id)}/species/${key}/files/report.html?inline=1`,
  /** Absolute tile template for MapLibre (it needs an absolute URL in workers). */
  tiles: (id: string, key: number, layer: string, threshold?: number) => {
    const base = `${window.location.origin}${API_BASE}/jobs/${enc(id)}/species/${key}/tiles/${layer}/{z}/{x}/{y}.png`;
    return threshold !== undefined ? `${base}?threshold=${threshold.toFixed(4)}` : base;
  },
  /** File of a decision-support analysis; `inline` shows report.html in the browser (sandboxed). */
  analysisFile: (id: string, aid: string, name: string, inline = false) =>
    `${API_BASE}/jobs/${enc(id)}/analyses/${enc(aid)}/files/${enc(name)}${inline ? "?inline=1" : ""}`,
  /** Absolute tile template of an analysis layer. */
  analysisTiles: (id: string, aid: string, layer: string) =>
    `${window.location.origin}${API_BASE}/jobs/${enc(id)}/analyses/${enc(aid)}/tiles/${layer}/{z}/{x}/{y}.png`,
};

/** Human message for any thrown value (used by error states). */
export function errorMessage(err: unknown): string {
  if (err instanceof ApiError) return err.message;
  if (err instanceof Error) return err.message;
  return apiMessages.get().unknown;
}
