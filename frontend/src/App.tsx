import { useEffect, useMemo, useRef, useState } from "react";
import { connectEvents, fetchSnapshot } from "./api";
import {
  sendBrowserNotification,
  shouldNotify,
} from "./notifications";
import "./styles.css";
import { defaultLanguage, translations } from "./i18n";
import type { Copy, Language } from "./i18n";
import type { BackendEvent, DashboardSnapshot, SymbolSnapshot } from "./types";

type Filter = "all" | "trend" | "range" | "hot" | "fixed";
type ThemeMode = "light" | "dark" | "system";

const themeStorageKey = "alphapulse-theme";
const languageStorageKey = "alphapulse-language";

const emptySnapshot: DashboardSnapshot = {
  symbols: [],
  last_scan_at_ms: null,
  websocket_connected: false,
};

export default function App() {
  const [snapshot, setSnapshot] = useState<DashboardSnapshot>(emptySnapshot);
  const [backendState, setBackendState] = useState<"connected" | "disconnected">(
    "disconnected",
  );
  const [streamState, setStreamState] = useState<"connected" | "idle">("idle");
  const [filter, setFilter] = useState<Filter>("all");
  const [themeMode, setThemeMode] = useState<ThemeMode>(() => readStoredTheme());
  const [language, setLanguage] = useState<Language>(() => readStoredLanguage());
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [notificationPermission, setNotificationPermission] = useState(() =>
    "Notification" in window ? Notification.permission : "unsupported",
  );
  const notified = useRef(new Map<string, string>());
  const copy = translations[language];

  useEffect(() => {
    document.documentElement.dataset.theme = themeMode;
    if (themeMode === "system") {
      localStorage.removeItem(themeStorageKey);
      return;
    }
    localStorage.setItem(themeStorageKey, themeMode);
  }, [themeMode]);

  useEffect(() => {
    document.documentElement.lang = language === "zh" ? "zh-CN" : "en";
    if (language === defaultLanguage) {
      localStorage.removeItem(languageStorageKey);
      return;
    }
    localStorage.setItem(languageStorageKey, language);
  }, [language]);

  useEffect(() => {
    let active = true;

    fetchSnapshot()
      .then((data) => {
        if (!active) {
          return;
        }
        setSnapshot(data);
        setBackendState("connected");
        setSelectedId((current) => current ?? data.symbols[0]?.inst_id ?? null);
      })
      .catch(() => {
        if (active) {
          setBackendState("disconnected");
        }
      });

    return () => {
      active = false;
    };
  }, []);

  useEffect(() => {
    if (import.meta.env.MODE === "test" || typeof WebSocket === "undefined") {
      return;
    }

    const socket = connectEvents((event: BackendEvent) => {
      setStreamState("connected");
      if (event.type === "snapshot") {
        setSnapshot(event.data);
        setSelectedId((current) => current ?? event.data.symbols[0]?.inst_id ?? null);
        return;
      }

      setSnapshot((current) => upsertSymbol(current, event.data));
      setSelectedId((current) => current ?? event.data.inst_id);
      if (shouldNotify(event.data, notified.current, 80)) {
        sendBrowserNotification(event.data);
      }
    });

    socket.addEventListener("open", () => setStreamState("connected"));
    socket.addEventListener("close", () => setStreamState("idle"));
    socket.addEventListener("error", () => setStreamState("idle"));

    return () => socket.close();
  }, []);

  const sortedSymbols = useMemo(
    () =>
      [...snapshot.symbols].sort(
        (left, right) => maxScore(right) - maxScore(left),
      ),
    [snapshot.symbols],
  );

  const filteredSymbols = useMemo(
    () => sortedSymbols.filter((symbol) => matchesFilter(symbol, filter)),
    [filter, sortedSymbols],
  );
  const selected =
    filteredSymbols.find((symbol) => symbol.inst_id === selectedId) ??
    filteredSymbols[0] ??
    null;

  async function requestNotifications() {
    if (!("Notification" in window)) {
      setNotificationPermission("unsupported");
      return;
    }
    const permission = await Notification.requestPermission();
    setNotificationPermission(permission);
  }

  return (
    <main className="app-shell">
      <header className="topbar">
        <div>
          <h1>AlphaPulse OKX</h1>
          <p>{copy.subtitle}</p>
        </div>
        <dl className="status-grid" aria-label={copy.aria.connectionStatus}>
          <div>
            <dt>{copy.status.backend}</dt>
            <dd>{formatState(backendState, copy)}</dd>
          </div>
          <div>
            <dt>{copy.status.stream}</dt>
            <dd>{formatState(streamState, copy)}</dd>
          </div>
          <div>
            <dt>{copy.status.notifications}</dt>
            <dd>{formatState(notificationPermission, copy)}</dd>
          </div>
          <div>
            <dt>{copy.status.lastScan}</dt>
            <dd>{formatTimestamp(snapshot.last_scan_at_ms)}</dd>
          </div>
          <div>
            <dt>{copy.status.symbols}</dt>
            <dd>{snapshot.symbols.length}</dd>
          </div>
        </dl>
      </header>

      <section className="toolbar" aria-label={copy.aria.radarControls}>
        <div className="toolbar-group" role="group" aria-label={copy.aria.opportunityFilters}>
          {[
            ["all", copy.filters.all],
            ["trend", copy.filters.trend],
            ["range", copy.filters.range],
            ["hot", copy.filters.hot],
            ["fixed", copy.filters.fixed],
          ].map(([value, label]) => (
            <button
              className={filter === value ? "active" : ""}
              key={value}
              onClick={() => setFilter(value as Filter)}
              type="button"
            >
              {label}
            </button>
          ))}
        </div>
        <div className="toolbar-group theme-toggle" role="group" aria-label={copy.aria.themeMode}>
          {[
            ["light", copy.themes.light],
            ["dark", copy.themes.dark],
            ["system", copy.themes.system],
          ].map(([value, label]) => (
            <button
              className={themeMode === value ? "active" : ""}
              key={value}
              onClick={() => setThemeMode(value as ThemeMode)}
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
              onClick={() => setLanguage(value as Language)}
              type="button"
            >
              {label}
            </button>
          ))}
        </div>
        <button onClick={requestNotifications} type="button">
          {copy.actions.enableNotifications}
        </button>
      </section>

      {filteredSymbols.length === 0 ? (
        <section className="empty-state">
          <h2>{copy.empty.title}</h2>
          <p>{copy.empty.body}</p>
        </section>
      ) : (
        <section className="radar-grid">
          <div className="table-panel">
            <table>
              <thead>
                <tr>
                  <th>{copy.table.symbol}</th>
                  <th>{copy.table.price}</th>
                  <th>5m</th>
                  <th>15m</th>
                  <th>1h</th>
                  <th>{copy.table.trend}</th>
                  <th>{copy.table.range}</th>
                  <th>{copy.table.signal}</th>
                </tr>
              </thead>
              <tbody>
                {filteredSymbols.map((symbol) => (
                  <tr
                    className={symbol.inst_id === selected?.inst_id ? "selected" : ""}
                    key={symbol.inst_id}
                    onClick={() => setSelectedId(symbol.inst_id)}
                  >
                    <td>
                      <strong>{symbol.inst_id}</strong>
                      <span>{formatTags(symbol.pool_tags, copy)}</span>
                    </td>
                    <td>{formatPrice(symbol.price)}</td>
                    <td>{formatPct(symbol.change_5m_pct)}</td>
                    <td>{formatPct(symbol.change_15m_pct)}</td>
                    <td>{formatPct(symbol.change_1h_pct)}</td>
                    <td>{formatScore(symbol.trend_score, copy)}</td>
                    <td>{formatScore(symbol.range_score, copy)}</td>
                    <td>{symbol.trigger_reason || copy.misc.watching}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <aside className="detail-panel">
            {selected ? <SymbolDetail copy={copy} symbol={selected} /> : null}
          </aside>
        </section>
      )}
    </main>
  );
}

function readStoredTheme(): ThemeMode {
  const stored = localStorage.getItem(themeStorageKey);
  if (stored === "light" || stored === "dark") {
    return stored;
  }
  return "system";
}

function readStoredLanguage(): Language {
  const stored = localStorage.getItem(languageStorageKey);
  if (stored === "en" || stored === "zh") {
    return stored;
  }
  return defaultLanguage;
}

function SymbolDetail({
  copy,
  symbol,
}: {
  copy: Copy;
  symbol: SymbolSnapshot;
}) {
  return (
    <>
      <header>
        <h2>{symbol.inst_id}</h2>
        <p>{symbol.trigger_reason || copy.detail.noActiveTrigger}</p>
      </header>
      <dl className="detail-list">
        <div>
          <dt>{copy.detail.funding}</dt>
          <dd>{symbol.funding_rate === null ? "-" : formatPct(symbol.funding_rate)}</dd>
        </div>
        <div>
          <dt>{copy.detail.updated}</dt>
          <dd>{formatTimestamp(symbol.updated_at_ms)}</dd>
        </div>
      </dl>
      <section>
        <h3>{copy.detail.fvg}</h3>
        {symbol.fvgs.length === 0 ? (
          <p className="muted">{copy.detail.noFvgZones}</p>
        ) : (
          <ul>
            {symbol.fvgs.map((zone, index) => (
              <li key={`${zone.timeframe}-${zone.direction}-${index}`}>
                {zone.timeframe} {copy.directions[zone.direction]}{" "}
                {formatPrice(zone.lower)}-{formatPrice(zone.upper)}{" "}
                {copy.detail.distance} {formatPct(zone.distance_pct)}
              </li>
            ))}
          </ul>
        )}
      </section>
      <section>
        <h3>{copy.detail.levels}</h3>
        {symbol.levels.length === 0 ? (
          <p className="muted">{copy.detail.noLevels}</p>
        ) : (
          <ul>
            {symbol.levels.map((level, index) => (
              <li key={`${level.kind}-${index}`}>
                {copy.levelKinds[level.kind]} {formatPrice(level.lower)}-
                {formatPrice(level.upper)} {copy.detail.touches} {level.touches}
              </li>
            ))}
          </ul>
        )}
      </section>
      <section>
        <h3>{copy.detail.account}</h3>
        <p className="muted">{copy.detail.noApiKey}</p>
      </section>
    </>
  );
}

function upsertSymbol(
  snapshot: DashboardSnapshot,
  symbol: SymbolSnapshot,
): DashboardSnapshot {
  const symbols = snapshot.symbols.filter((item) => item.inst_id !== symbol.inst_id);
  symbols.push(symbol);
  return { ...snapshot, symbols };
}

function matchesFilter(symbol: SymbolSnapshot, filter: Filter): boolean {
  if (filter === "trend") {
    return symbol.trend_score.value >= 65;
  }
  if (filter === "range") {
    return symbol.range_score.value >= 65;
  }
  if (filter === "hot") {
    return symbol.pool_tags.includes("dynamic");
  }
  if (filter === "fixed") {
    return symbol.pool_tags.includes("fixed");
  }
  return true;
}

function maxScore(symbol: SymbolSnapshot): number {
  return Math.max(symbol.trend_score.value, symbol.range_score.value);
}

function formatScore(score: SymbolSnapshot["trend_score"], copy: Copy): string {
  return `${score.value} ${copy.directions[score.direction]}`;
}

function formatTags(tags: string[], copy: Copy): string {
  if (tags.length === 0) {
    return copy.misc.unlabeled;
  }
  return tags
    .map((tag) => formatTag(tag, copy))
    .join(" / ");
}

function formatTag(tag: string, copy: Copy): string {
  const labels = copy.poolTags as unknown as Record<string, string>;
  return labels[tag] ?? tag;
}

function formatState(value: string, copy: Copy): string {
  return copy.states[value as keyof Copy["states"]] ?? value;
}

function formatPrice(value: number): string {
  if (value >= 100) {
    return value.toFixed(2);
  }
  if (value >= 1) {
    return value.toFixed(4);
  }
  return value.toFixed(6);
}

function formatPct(value: number): string {
  return `${(value * 100).toFixed(2)}%`;
}

function formatTimestamp(value: number | null): string {
  if (value === null) {
    return "-";
  }
  return new Date(value).toLocaleTimeString();
}
