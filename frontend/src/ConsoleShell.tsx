import type { ReactNode } from "react";
import type { Copy, Language } from "./i18n";
import {
  formatState,
  formatTimestamp,
  type ThemeMode,
  type ViewMode,
} from "./uiFormat";

export function ConsoleShell({
  backendState,
  children,
  copy,
  language,
  lastScanAt,
  notificationPermission,
  onLanguageChange,
  onRequestNotifications,
  onThemeModeChange,
  onViewModeChange,
  streamState,
  symbolCount,
  themeMode,
  viewMode,
}: {
  backendState: "connected" | "disconnected";
  children: ReactNode;
  copy: Copy;
  language: Language;
  lastScanAt: number | null;
  notificationPermission: string;
  onLanguageChange: (language: Language) => void;
  onRequestNotifications: () => void;
  onThemeModeChange: (themeMode: ThemeMode) => void;
  onViewModeChange: (viewMode: ViewMode) => void;
  streamState: "connected" | "idle";
  symbolCount: number;
  themeMode: ThemeMode;
  viewMode: ViewMode;
}) {
  const statusItems = [
    { label: copy.status.backend, value: formatState(backendState, copy), tone: backendState },
    { label: copy.status.stream, value: formatState(streamState, copy), tone: streamState },
    {
      label: copy.status.notifications,
      value: formatState(notificationPermission, copy),
      tone: notificationPermission,
    },
    { label: copy.status.lastScan, value: formatTimestamp(lastScanAt), tone: "neutral" },
    { label: copy.status.symbols, value: String(symbolCount), tone: "neutral" },
  ];

  return (
    <main className="app-shell console-shell">
      <header className="console-topbar">
        <div className="console-brand">
          <h1>AlphaPulse OKX</h1>
          <p>{copy.subtitle}</p>
        </div>
        <div className="console-nav" role="group" aria-label={copy.aria.viewMode}>
          {[
            ["radar", copy.views.radar],
            ["macro", copy.views.macro],
          ].map(([value, label]) => (
            <button
              className={viewMode === value ? "active" : ""}
              key={value}
              onClick={() => onViewModeChange(value as ViewMode)}
              type="button"
            >
              {label}
            </button>
          ))}
        </div>
        <dl className="console-status" aria-label={copy.aria.connectionStatus}>
          {statusItems.map((item) => (
            <div className={`status-pill status-pill-${item.tone}`} key={item.label}>
              <dt>{item.label}</dt>
              <dd>{item.value}</dd>
            </div>
          ))}
        </dl>
        <div className="console-actions">
          <div className="toolbar-group" role="group" aria-label={copy.aria.themeMode}>
            {[
              ["light", copy.themes.light],
              ["dark", copy.themes.dark],
              ["system", copy.themes.system],
            ].map(([value, label]) => (
              <button
                className={themeMode === value ? "active" : ""}
                key={value}
                onClick={() => onThemeModeChange(value as ThemeMode)}
                type="button"
              >
                {label}
              </button>
            ))}
          </div>
          <div className="toolbar-group" role="group" aria-label={copy.aria.languageMode}>
            {[
              ["zh", copy.languages.zh],
              ["en", copy.languages.en],
            ].map(([value, label]) => (
              <button
                className={language === value ? "active" : ""}
                key={value}
                onClick={() => onLanguageChange(value as Language)}
                type="button"
              >
                {label}
              </button>
            ))}
          </div>
          <button onClick={onRequestNotifications} type="button">
            {copy.actions.enableNotifications}
          </button>
        </div>
      </header>
      {children}
    </main>
  );
}
