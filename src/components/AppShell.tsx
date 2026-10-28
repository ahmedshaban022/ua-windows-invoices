import type { ReactNode } from "react";
import { NavLink, useLocation } from "react-router-dom";
import {
  Archive,
  FilePlus2,
  FileText,
  Package,
  Settings as SettingsIcon,
  Trash2,
} from "lucide-react";

import { useSettings } from "../state/settings";

const NAV_ITEMS = [
  { to: "/", label: "فاتورة جديدة", icon: FilePlus2, end: true },
  { to: "/history", label: "سجل الفواتير", icon: FileText, end: false },
  { to: "/products", label: "الأصناف", icon: Package, end: false },
  { to: "/trash", label: "المحذوفات", icon: Trash2, end: false },
  { to: "/settings", label: "الإعدادات", icon: SettingsIcon, end: false },
];

const TITLES: Record<string, string> = {
  "/": "فاتورة جديدة",
  "/history": "سجل الفواتير",
  "/products": "الأصناف",
  "/trash": "المحذوفات",
  "/settings": "الإعدادات",
};

export function AppShell({ children }: { children: ReactNode }) {
  const settings = useSettings((state) => state.settings);
  const location = useLocation();

  const title = location.pathname.startsWith("/preview")
    ? "عرض الفاتورة"
    : (TITLES[location.pathname] ?? "فاتورة");

  const logo = settings?.logoSvg ?? "";

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="sidebar__brand">
          {logo ? (
            <div
              aria-hidden="true"
              dangerouslySetInnerHTML={{ __html: logo }}
              style={{ height: 34, display: "flex", alignItems: "center" }}
            />
          ) : (
            <Archive size={26} />
          )}
          <div>
            <div className="sidebar__brand-name">{settings?.companyName ?? "فاتورة"}</div>
            <div className="sidebar__brand-sub">برنامج الفواتير</div>
          </div>
        </div>

        <nav className="sidebar__section" aria-label="التنقل الرئيسي">
          {NAV_ITEMS.map((item) => (
            <NavLink key={item.to} to={item.to} end={item.end} className="nav-item">
              <item.icon size={18} aria-hidden="true" />
              {item.label}
            </NavLink>
          ))}
        </nav>

        <div className="sidebar__footer">
          <div className="row" style={{ gap: 6 }}>
            <Archive size={14} aria-hidden="true" />
            <span>البيانات محفوظة على هذا الجهاز فقط</span>
          </div>
        </div>
      </aside>

      <div className="app-main">
        <header className="topbar">
          <div className="topbar__title">{title}</div>
          <div className="topbar__spacer" />
          <div className="topbar__meta">
            {settings?.currencyCode ?? "EGP"} — {settings?.companyName ?? ""}
          </div>
        </header>
        <main className="page">
          <div className="page__inner">{children}</div>
        </main>
      </div>
    </div>
  );
}
