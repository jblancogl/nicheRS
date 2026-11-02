import { useEffect } from "react";
import { Outlet } from "react-router";

import { ApiError, redirectToLogin } from "@/api/client";
import { useAccountLang, useMe } from "@/api/hooks";

import { useShellMessages } from "./messages";
import { Notices } from "./Notices";
import { TopBar } from "./TopBar";
import { useRunWatcher } from "./useRunWatcher";

/**
 * Application frame: session gate (hosted mode), top bar and notices around every workspace
 * page. The organisation's workspace is a plain page; studies and the setup wizard open in
 * the map layout (MapLayout.tsx).
 */
export function Shell() {
  const me = useMe();
  const signedOut = me.error instanceof ApiError && me.error.status === 401;
  useEffect(() => {
    if (signedOut) redirectToLogin();
  }, [signedOut]);
  // Hosted mode: nothing of the workspace is shown before the session is confirmed. Local mode
  // (me.data === null) and an unreachable server fall through to the normal workspace.
  if (me.isPending || signedOut) return null;
  return <Frame />;
}

function Frame() {
  useAccountLang();
  useRunWatcher();
  const m = useShellMessages();
  return (
    <div className="flex h-dvh flex-col overflow-hidden">
      <a
        href="#main"
        className="sr-only z-50 rounded bg-primary px-3 py-1.5 text-primary-foreground focus:not-sr-only focus:absolute focus:top-2 focus:left-2"
      >
        {m.skipPanel}
      </a>
      <TopBar />
      <Notices />
      <div id="main" className="relative flex min-h-0 flex-1 flex-col" tabIndex={-1}>
        <Outlet />
      </div>
    </div>
  );
}
