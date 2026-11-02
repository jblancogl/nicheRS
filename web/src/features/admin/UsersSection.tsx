// Every account of every organisation, searched, filtered by organisation and paged on the server
// (ADR-0113): see role and last sign-in, and open one (UserDetail) to change its role or access.

import { Search } from "lucide-react";
import { useState } from "react";
import { Link } from "react-router";

import { useAdminOrgs, useAdminUsers } from "@/api/hooks";
import type { UserInfo } from "@/api/types";
import { ErrorState } from "@/components/domain/common";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { formatCount, formatDateTime } from "@/lib/format";
import { cn } from "@/lib/utils";
import { useDebounced } from "@/lib/useDebounced";

import { useRowLink } from "./links";
import { useAdminMessages } from "./messages";
import { Loading, MobileItem, MobileList, RoleBadge, RowChevron, Status } from "./shared";
import { adminUrl, type AdminSectionProps } from "./sections";

/** Name with the marks that matter: you, disabled. */
function UserName({ u, self, link }: { u: UserInfo; self: boolean; link?: boolean }) {
  const m = useAdminMessages();
  return (
    <span className="flex flex-wrap items-center gap-x-2 gap-y-1">
      {link ? (
        <Link to={adminUrl("users", u.id)} className="font-medium hover:underline">
          {u.name}
        </Link>
      ) : (
        <span className="font-medium">{u.name}</span>
      )}
      {self && <span className="text-xs text-muted-foreground">{m.you}</span>}
      {u.disabled_at && <Status disabledAt={u.disabled_at} />}
    </span>
  );
}

export function UsersSection({ me }: AdminSectionProps) {
  const m = useAdminMessages();
  const rowLink = useRowLink();
  const [q, setQ] = useState("");
  const [org, setOrg] = useState("");
  const orgs = useAdminOrgs();
  const users = useAdminUsers({ q: useDebounced(q.trim(), 250), org: org || undefined });
  const shown = users.data?.pages.flatMap((p) => p.users) ?? [];
  const total = users.data?.pages[0]?.total ?? 0;
  const lastSignIn = (u: UserInfo) => (u.last_login_at ? formatDateTime(u.last_login_at) : <span className="text-muted-foreground">{m.never}</span>);
  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center gap-2">
        <div className="relative min-w-0 flex-1 basis-56 sm:max-w-80">
          <Search className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground" aria-hidden />
          <Input type="search" value={q} onChange={(e) => setQ(e.target.value)} placeholder={m.filterPlaceholder} aria-label={m.filter} className="pl-8" />
        </div>
        <select
          aria-label={m.colOrg}
          value={org}
          onChange={(e) => setOrg(e.target.value)}
          className="h-8 max-w-full min-w-0 rounded-lg border border-input bg-transparent px-2 text-sm"
        >
          <option value="">{m.allOrgs}</option>
          {(orgs.data ?? [])
            .slice()
            .sort((a, b) => a.name.localeCompare(b.name))
            .map((o) => (
              <option key={o.id} value={o.id}>
                {o.name}
              </option>
            ))}
        </select>
        {users.data && <p className="num text-sm text-muted-foreground">{m.accountsShown(shown.length, total)}</p>}
      </div>
      {users.isPending && <Loading />}
      {users.isError && <ErrorState error={users.error} onRetry={() => void users.refetch()} title={m.loadUsersFailed} />}
      {users.data && (
        <>
          <div className="hidden md:block">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>{m.colName}</TableHead>
                  <TableHead>{m.colOrg}</TableHead>
                  <TableHead>{m.colRole}</TableHead>
                  <TableHead>{m.colLastSignIn}</TableHead>
                  <TableHead className="w-8" />
                </TableRow>
              </TableHeader>
              <TableBody>
                {shown.length === 0 && (
                  <TableRow>
                    <TableCell colSpan={5} className="text-muted-foreground">
                      {m.noAccount}
                    </TableCell>
                  </TableRow>
                )}
                {shown.map((u: UserInfo) => (
                  <TableRow key={u.id} {...rowLink(adminUrl("users", u.id))} className={cn("cursor-pointer", u.disabled_at && "text-muted-foreground")}>
                    <TableCell>
                      <UserName u={u} self={u.email === me.email} link />
                      <span className="block text-xs text-muted-foreground">{u.email}</span>
                    </TableCell>
                    <TableCell className="whitespace-normal">{u.org_name}</TableCell>
                    <TableCell>
                      <RoleBadge u={u} />
                      {u.is_admin && !u.in_operator_org && <span className="ml-2 text-xs text-muted-foreground">{m.outsideOperator}</span>}
                    </TableCell>
                    <TableCell className="num">{lastSignIn(u)}</TableCell>
                    <TableCell>
                      <RowChevron />
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
          <MobileList empty={shown.length === 0 ? m.noAccount : undefined}>
            {shown.map((u) => (
              <MobileItem
                key={u.id}
                to={adminUrl("users", u.id)}
                muted={u.disabled_at !== null}
                title={<UserName u={u} self={u.email === me.email} />}
                aside={<RoleBadge u={u} />}
                meta={
                  <>
                    <span className="block truncate">{u.email}</span>
                    <span className="block">
                      {u.org_name} · {m.colLastSignIn} {lastSignIn(u)}
                    </span>
                  </>
                }
              />
            ))}
          </MobileList>
        </>
      )}
      {users.hasNextPage && (
        <Button variant="outline" size="sm" disabled={users.isFetchingNextPage} onClick={() => void users.fetchNextPage()}>
          {m.showMore(formatCount(total - shown.length))}
        </Button>
      )}
      <p className="text-xs text-muted-foreground">{m.usersHint}</p>
    </div>
  );
}
