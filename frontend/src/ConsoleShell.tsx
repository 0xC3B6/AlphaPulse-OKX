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
  const themeOptions: Array<[ThemeMode, string]> = [
    ["light", "Light"],
    ["dark", "Dark"],
    ["system", "System"],
  ];
  const languageOptions: Array<[Language, string]> = [
    ["zh", "ZH"],
    ["en", "EN"],
  ];
  const selectedThemeLabel =
    themeOptions.find(([value]) => value === themeMode)?.[1] ?? "System";
  const selectedLanguageLabel =
    languageOptions.find(([value]) => value === language)?.[1] ?? "ZH";
  const taskItems: Array<[ViewMode, string]> = [
    ["monitor", copy.views.monitor],
    ["trade", copy.views.trade],
    ["review", copy.views.review],
    ["macro", copy.views.macro],
  ];
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
    <main className="app-shell console-shell task-console-shell">
      <nav className="task-rail" aria-label={copy.aria.taskNavigation}>
        <div className="task-rail-brand">
          <strong>AlphaPulse</strong>
          <span>OKX</span>
        </div>
        <div className="task-rail-items">
          {taskItems.map(([value, label]) => (
            <button
              aria-current={viewMode === value ? "page" : undefined}
              className={`task-rail-button ${viewMode === value ? "active" : ""}`}
              key={value}
              onClick={() => onViewModeChange(value)}
              type="button"
            >
              {label}
            </button>
          ))}
        </div>
      </nav>
      <section className="console-main">
        <header className="console-topbar">
          <div className="console-page-title">
            <span>AlphaPulse OKX</span>
            <h1>{copy.views[viewMode]}</h1>
            <p>{copy.pageDescriptions[viewMode]}</p>
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
            <div className="console-menu">
              <button
                aria-haspopup="menu"
                aria-label={`${copy.aria.themeMode}: ${selectedThemeLabel}`}
                className="console-menu-trigger"
                title={copy.aria.themeMode}
                type="button"
              >
                <span>{selectedThemeLabel}</span>
              </button>
              <div className="console-menu-popover" role="menu">
                {themeOptions.map(([value, label]) => (
                  <button
                    aria-checked={themeMode === value}
                    className={themeMode === value ? "active" : ""}
                    key={value}
                    onClick={() => onThemeModeChange(value)}
                    role="menuitemradio"
                    type="button"
                  >
                    {label}
                  </button>
                ))}
              </div>
            </div>
            <div className="console-menu">
              <button
                aria-haspopup="menu"
                aria-label={`${copy.aria.languageMode}: ${selectedLanguageLabel}`}
                className="console-menu-trigger console-language-trigger"
                title={copy.aria.languageMode}
                type="button"
              >
                {selectedLanguageLabel}
              </button>
              <div className="console-menu-popover" role="menu">
                {languageOptions.map(([value, label]) => (
                  <button
                    aria-checked={language === value}
                    className={language === value ? "active" : ""}
                    key={value}
                    onClick={() => onLanguageChange(value)}
                    role="menuitemradio"
                    type="button"
                  >
                    {label}
                  </button>
                ))}
              </div>
            </div>
            <button onClick={onRequestNotifications} type="button">
              {copy.actions.enableNotifications}
            </button>
          </div>
        </header>
        {children}
      </section>
    </main>
  );
}
