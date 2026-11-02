// Routing: React Router (data router, v8). Chosen over ad-hoc state because the product needs
// deep links (/jobs/:id, /jobs/:id/results), a working back button and URL-encoded view state
// (layer, threshold rule, species — design-principles P9). The Rust server serves index.html for
// unknown paths (SPA fallback), so these URLs survive reloads. Views are lazy-loaded so the
// setup wizard does not pay for the results code.

import { Navigate, createBrowserRouter } from "react-router";

import { NotFound } from "./NotFound";
import { Shell } from "./Shell";

const auth = async () => import("@/features/auth/AuthPages");
const settings = async () => import("@/features/settings/SettingsPage");

export const router = createBrowserRouter([
  // Hosted mode entry points, outside the workspace shell (no map, no account needed).
  { path: "/login", lazy: async () => ({ Component: (await auth()).LoginPage }) },
  { path: "/invite", lazy: async () => ({ Component: (await auth()).InvitePage }) },
  { path: "/reset", lazy: async () => ({ Component: (await auth()).ResetPage }) },
  // Help centre as a page (also reachable as a panel from the top bar of every workspace page).
  { path: "/help/:article?", lazy: async () => ({ Component: (await import("@/features/help/HelpPage")).HelpPage }) },
  // Natural Solutions administration: one URL per section and per organisation or account
  // (features/admin/sections.tsx; ADR-0106, ADR-0113).
  { path: "/admin/:section?/:id?", lazy: async () => ({ Component: (await import("@/features/admin/AdminPage")).AdminPage }) },
  // Settings: "My account" and "My organisation", one URL per section (features/settings/sections.tsx;
  // ADR-0109, ADR-0110, ADR-0111). Unknown or not allowed sections go back to the area's first page.
  { path: "/account/:section?", lazy: async () => ({ Component: (await settings()).AccountSettings }) },
  { path: "/org/:section?", lazy: async () => ({ Component: (await settings()).OrgSettings }) },
  {
    path: "/",
    element: <Shell />,
    HydrateFallback: () => null,
    children: [
      // The organisation's workspace: projects and studies (a plain page, no map).
      { index: true, lazy: async () => ({ Component: (await import("@/features/workspace/WorkspacePage")).WorkspacePage }) },
      { path: "jobs", element: <Navigate to="/" replace /> },
      {
        // Map (and MapLibre) loaded only for the wizard and studies.
        lazy: async () => ({ Component: (await import("./MapLayout")).MapLayout }),
        children: [
          { path: "new", lazy: async () => ({ Component: (await import("@/features/setup/SetupWizard")).SetupWizard }) },
          { path: "jobs/:id", lazy: async () => ({ Component: (await import("@/features/execution/ExecutionView")).ExecutionView }) },
          { path: "jobs/:id/results", lazy: async () => ({ Component: (await import("@/features/results/ResultsView")).ResultsView }) },
          { path: "*", element: <NotFound /> },
        ],
      },
    ],
  },
]);
