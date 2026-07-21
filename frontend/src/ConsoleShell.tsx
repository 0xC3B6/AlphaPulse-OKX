import type { ReactNode } from "react";
import {
  Activity,
  BarChart2,
  BookOpen,
  Bell,
  Languages,
  Moon,
  Radio,
  Sun,
  SunMoon,
  Target,
  TrendingUp,
  type LucideIcon,
} from "lucide-react";
import type { Copy, Language } from "./i18n";
import type { SymbolSnapshot } from "./types";
import {
  formatPct,
  formatPrice,
  formatSignedUsdt,
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
  activeSignalCount,
  positionCount,
  streamState,
  symbolCount,
  tickerSymbols,
  themeMode,
  unrealizedPnl,
  viewMode,
}: {
  activeSignalCount: number;
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
  positionCount: number;
  streamState: "connected" | "idle";
  symbolCount: number;
  tickerSymbols: SymbolSnapshot[];
  themeMode: ThemeMode;
  unrealizedPnl: number;
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
    ["ja", "JA"],
  ];
  const selectedThemeLabel =
    themeOptions.find(([value]) => value === themeMode)?.[1] ?? "System";
  const selectedLanguageLabel =
    languageOptions.find(([value]) => value === language)?.[1] ?? "ZH";
  const ThemeIcon = themeMode === "light" ? Sun : themeMode === "dark" ? Moon : SunMoon;
  const taskItems: Array<{
    icon: LucideIcon;
    label: string;
    legacyLabel: string;
    sub: string;
    value: ViewMode;
  }> = [
    {
      icon: Activity,
      label: "Radar",
      legacyLabel: copy.views.monitor,
      sub: copy.terminal.monitorSub,
      value: "monitor",
    },
    {
      icon: TrendingUp,
      label: copy.terminal.macroLabel,
      legacyLabel: copy.views.macro,
      sub: copy.terminal.macroSub,
      value: "macro",
    },
    {
      icon: Target,
      label: copy.views.strategy,
      legacyLabel: copy.views.strategy,
      sub: copy.terminal.strategySub,
      value: "strategy",
    },
    {
      icon: BookOpen,
      label: copy.terminal.tradeLabel,
      legacyLabel: copy.views.trade,
      sub: copy.terminal.tradeSub,
      value: "trade",
    },
    {
      icon: BarChart2,
      label: copy.terminal.reviewLabel,
      legacyLabel: copy.views.review,
      sub: copy.terminal.reviewSub,
      value: "review",
    },
  ];
  const activeTask = taskItems.find((item) => item.value === viewMode) ?? taskItems[0];
  const hotTicker = tickerSymbols[0] ?? null;
  const connectionLabel =
    backendState === "connected" && streamState === "connected"
      ? `WS ${copy.states.connected}`
      : backendState === "connected"
        ? `API ${copy.states.connected}`
        : copy.states.disconnected;
  const lastScanLabel =
    lastScanAt === null
      ? "--:-- UTC"
      : new Date(lastScanAt).toLocaleTimeString("en", {
          hour: "2-digit",
          minute: "2-digit",
          timeZone: "UTC",
        }) + " UTC";

  return (
    <main
      className="app-shell console-shell task-console-shell terminal-shell"
      data-testid="terminal-shell"
    >
      <nav className="task-rail figma-sidebar" data-testid="figma-sidebar" aria-label={copy.aria.taskNavigation}>
        <div className="task-rail-brand">
          <span className="task-rail-mark" aria-hidden="true">
            <Radio size={16} />
          </span>
          <span className="task-rail-brand-text">
            <strong>CRYPTO</strong>
            <span>RADAR SYSTEM</span>
          </span>
        </div>
        <div className="task-rail-items">
          {taskItems.map(({ icon: TaskIcon, label, legacyLabel, sub, value }) => {
            return (
              <button
                aria-label={legacyLabel}
                aria-current={viewMode === value ? "page" : undefined}
                className={`task-rail-button ${viewMode === value ? "active" : ""}`}
                key={value}
                onClick={() => onViewModeChange(value)}
                type="button"
              >
                <span className="task-rail-button-icon" aria-hidden="true">
                  <TaskIcon size={15} />
                </span>
                <span className="task-rail-button-copy">
                  <strong>{label}</strong>
                  <small>{sub}</small>
                  <span className="sr-only">{legacyLabel}</span>
                </span>
              </button>
            );
          })}
        </div>
        <div className="task-rail-footer" data-testid="terminal-live-status">
          <div>
            <span
              className={`live-dot ${streamState === "connected" ? "is-live" : ""}`}
              aria-hidden="true"
            />
            <strong>LIVE</strong>
            <small>{lastScanLabel}</small>
          </div>
          <p>BTC {hotTicker ? formatPrice(hotTicker.price) : "-"}</p>
          <p>Strategy v0.1.3</p>
          <div className="task-rail-footer-actions">
            <button aria-label={copy.actions.enableNotifications} onClick={onRequestNotifications} type="button">
              <Bell size={11} aria-hidden="true" />
            </button>
          </div>
        </div>
      </nav>
      <section className="console-main">
        <header className="console-topbar figma-radar-header" data-testid="figma-radar-header">
          <div className="console-page-title">
            <h1>{activeTask.label}</h1>
            <p>{activeTask.sub}</p>
            <span className="sr-only">{copy.views[viewMode]} {copy.pageDescriptions[viewMode]}</span>
          </div>
          <div className="terminal-quick-stats" data-testid="terminal-quick-stats">
            <span>
              {copy.terminal.positions} <strong>{positionCount}</strong>
            </span>
            <span>
              {copy.terminal.signals} <strong>{activeSignalCount}</strong>
            </span>
            <span>
              {copy.terminal.unrealized} <strong className={unrealizedPnl < 0 ? "negative" : "positive"}>
                {formatSignedUsdt(unrealizedPnl)}
              </strong>
            </span>
          </div>
          <div className="terminal-market-tape" data-testid="terminal-market-tape">
            <span className={`terminal-ws-pill ${streamState === "connected" ? "is-live" : ""}`}>
              <i aria-hidden="true" />
              {connectionLabel}
            </span>
            {hotTicker ? (
              <span className="terminal-hot-pill">
                ⚠ {hotTicker.inst_id.replace(/-USDT-SWAP$/u, "")} {formatPct(hotTicker.amplitude_24h_pct ?? 0)}
              </span>
            ) : (
              <span className="terminal-hot-pill">{copy.terminal.waitingForMarketData}</span>
            )}
            <span className="sr-only">
              {tickerSymbols.map((symbol) => symbol.inst_id).join(" ")} {symbolCount} {notificationPermission}
            </span>
          </div>
          <div className="console-actions">
            <div className="console-menu">
              <button
                aria-haspopup="menu"
                aria-label={`${copy.aria.themeMode}: ${selectedThemeLabel}`}
                className="console-menu-trigger"
                title={copy.aria.themeMode}
                type="button"
              >
                <ThemeIcon aria-hidden="true" size={15} />
                <span className="sr-only">{selectedThemeLabel}</span>
              </button>
              <div className="console-menu-popover" role="menu">
                {themeOptions.map(([value, label]) => (
                  <button
                    aria-checked={themeMode === value}
                    className={themeMode === value ? "active" : ""}
                    key={value}
                    onClick={(event) => {
                      onThemeModeChange(value);
                      event.currentTarget.blur();
                    }}
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
                <Languages aria-hidden="true" size={15} />
                <span className="sr-only">{selectedLanguageLabel}</span>
              </button>
              <div className="console-menu-popover" role="menu">
                {languageOptions.map(([value, label]) => (
                  <button
                    aria-checked={language === value}
                    className={language === value ? "active" : ""}
                    key={value}
                    onClick={(event) => {
                      onLanguageChange(value);
                      event.currentTarget.blur();
                    }}
                    role="menuitemradio"
                    type="button"
                  >
                    {label}
                  </button>
                ))}
              </div>
            </div>
            <button onClick={onRequestNotifications} type="button">
              <Bell size={13} aria-hidden="true" />
              {copy.actions.enableNotifications}
            </button>
          </div>
        </header>
        {children}
      </section>
    </main>
  );
}
