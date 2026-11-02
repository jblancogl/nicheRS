// The administration is a list of sections grouped in the side navigation, like the settings area.
// Adding one = one entry here (id, group, icon, component, optional detail page) plus its title and
// description in ./messages.ts; the navigation and the URL (/admin/<id>, /admin/<id>/<item>)
// follow. Detail pages (one organisation, one account) keep their section highlighted.

import { Activity, BarChart3, Building2, History, LayoutDashboard, MessageSquare, Server, Users } from "lucide-react";
import type { ComponentType } from "react";

import type { Me } from "@/api/types";

import { ActivitySection } from "./ActivitySection";
import { AuditSection } from "./AuditList";
import { FeedbackSection } from "./FeedbackSection";
import { OrgDetail } from "./OrgDetail";
import { OrgsSection } from "./OrgsSection";
import { OverviewSection } from "./OverviewSection";
import { SystemTab } from "./SystemTab";
import { UsageSection } from "./UsageSection";
import { UserDetail } from "./UserDetail";
import { UsersSection } from "./UsersSection";

export type AdminSectionId = "overview" | "orgs" | "users" | "activity" | "usage" | "feedback" | "system" | "audit";
export type AdminGroup = "home" | "clients" | "operations" | "platform";

export interface AdminSectionProps {
  me: Me;
}

export interface AdminSection {
  id: AdminSectionId;
  group: AdminGroup;
  icon: ComponentType<{ className?: string; "aria-hidden"?: boolean }>;
  Component: ComponentType<AdminSectionProps>;
  /** Page of one item of the section, at /admin/<id>/<item id>. */
  Detail?: ComponentType<AdminSectionProps & { id: string }>;
}

export const GROUPS: AdminGroup[] = ["home", "clients", "operations", "platform"];

export const ADMIN_SECTIONS: AdminSection[] = [
  { id: "overview", group: "home", icon: LayoutDashboard, Component: OverviewSection },
  { id: "orgs", group: "clients", icon: Building2, Component: OrgsSection, Detail: OrgDetail },
  { id: "users", group: "clients", icon: Users, Component: UsersSection, Detail: UserDetail },
  { id: "activity", group: "operations", icon: Activity, Component: ActivitySection },
  { id: "usage", group: "operations", icon: BarChart3, Component: UsageSection },
  { id: "feedback", group: "operations", icon: MessageSquare, Component: FeedbackSection },
  { id: "system", group: "platform", icon: Server, Component: SystemTab },
  { id: "audit", group: "platform", icon: History, Component: AuditSection },
];

export const adminUrl = (id: AdminSectionId, item?: string) => `/admin/${id}${item ? `/${encodeURIComponent(item)}` : ""}`;
