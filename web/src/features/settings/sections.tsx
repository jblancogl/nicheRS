// The settings area is a list of sections. Adding one = one entry here (id, path, icon, who sees
// it, component) plus its title and description in ./messages.ts; the navigation, the URL
// (/account/<path>, /org/<path>), the access check and the organisation overview follow.

import { Building2, History, KeyRound, LayoutGrid, MessageSquare, UserRound, Users } from "lucide-react";
import type { ComponentType } from "react";

import type { Me } from "@/api/types";
import { GbifSection, ProfileSection, SecuritySection } from "@/features/account/sections";
import { MembersSection, OrgFeedbackSection, OrgGbifSection, OrgHistorySection, OverviewSection } from "@/features/org/sections";

export type Area = "account" | "org";

export type SectionId = "profile" | "security" | "gbif" | "overview" | "members" | "org-gbif" | "feedback" | "history";

export interface SectionProps {
  me: Me;
  /** The other sections of the same area this user may open (the overview links to them). */
  siblings: Section[];
}

export interface Section {
  id: SectionId;
  area: Area;
  /** Path below /account or /org; "" is the area's first page. */
  path: string;
  icon: ComponentType<{ className?: string; "aria-hidden"?: boolean }>;
  /** Who may open it; others do not see it in the navigation and are sent to the area's first page. */
  visible?: (me: Me) => boolean;
  /** Shown with a "Managers only" mark in the navigation. */
  managers?: boolean;
  Component: ComponentType<SectionProps>;
}

const manages = (me: Me) => me.is_manager || me.is_admin;

export const SECTIONS: Section[] = [
  { id: "profile", area: "account", path: "", icon: UserRound, Component: ProfileSection },
  { id: "security", area: "account", path: "security", icon: KeyRound, Component: SecuritySection },
  { id: "gbif", area: "account", path: "gbif", icon: Building2, Component: GbifSection },
  { id: "overview", area: "org", path: "", icon: LayoutGrid, Component: OverviewSection },
  { id: "members", area: "org", path: "members", icon: Users, visible: manages, managers: true, Component: MembersSection },
  { id: "org-gbif", area: "org", path: "gbif", icon: Building2, Component: OrgGbifSection },
  { id: "feedback", area: "org", path: "feedback", icon: MessageSquare, visible: manages, managers: true, Component: OrgFeedbackSection },
  { id: "history", area: "org", path: "history", icon: History, visible: manages, managers: true, Component: OrgHistorySection },
];

export const sectionUrl = (s: Section) => `/${s.area}${s.path ? `/${s.path}` : ""}`;

export const visibleSections = (me: Me, area?: Area) =>
  SECTIONS.filter((s) => (area === undefined || s.area === area) && (s.visible?.(me) ?? true));
