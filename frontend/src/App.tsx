import { useEffect, useMemo, useRef, useState } from "react";
import {
  closePaperPosition,
  connectEvents,
  fetchBtcMacro,
  fetchSnapshot,
  openPaperOrder,
} from "./api";
import { ChartPanel } from "./ChartPanel";
import { ConsoleShell } from "./ConsoleShell";
import { MacroPanel } from "./MacroPanel";
import {
  sendBrowserNotification,
  shouldNotify,
} from "./notifications";
import "./styles.css";
import { defaultLanguage, translations } from "./i18n";
import type { Copy, Language } from "./i18n";
import {
  formatPct,
  formatPrice,
  formatQuantity,
  formatScore,
  formatSignedUsdt,
  formatTags,
  formatTemplate,
  formatTimestamp,
  formatUsdt,
  matchesFilter,
  maxScore,
  pnlClass,
  type Filter,
  type ThemeMode,
  type ViewMode,
} from "./uiFormat";
import type {
  BackendEvent,
  BtcMacroSnapshot,
  DashboardSnapshot,
  PaperAccountSnapshot,
  PaperSide,
  SymbolSnapshot,
} from "./types";

const themeStorageKey = "alphapulse-theme";
const languageStorageKey = "alphapulse-language";

const emptyPaperAccount: PaperAccountSnapshot = {
  mode: "paper",
  initial_balance: 10000,
  realized_pnl: 0,
  unrealized_pnl: 0,
  equity: 10000,
  used_margin: 0,
  available_balance: 10000,
  positions: [],
  trades: [],
};

const emptySnapshot: DashboardSnapshot = {
  symbols: [],
  last_scan_at_ms: null,
  websocket_connected: false,
  paper: emptyPaperAccount,
};

export default function App() {
  const [snapshot, setSnapshot] = useState<DashboardSnapshot>(emptySnapshot);
  const [macroSnapshot, setMacroSnapshot] = useState<BtcMacroSnapshot | null>(null);
  const [macroLoading, setMacroLoading] = useState(false);
  const [macroError, setMacroError] = useState<string | null>(null);
  const [backendState, setBackendState] = useState<"connected" | "disconnected">(
    "disconnected",
  );
  const [streamState, setStreamState] = useState<"connected" | "idle">("idle");
  const [viewMode, setViewMode] = useState<ViewMode>("radar");
  const [filter, setFilter] = useState<Filter>("all");
  const [themeMode, setThemeMode] = useState<ThemeMode>(() => readStoredTheme());
  const [language, setLanguage] = useState<Language>(() => readStoredLanguage());
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [tradingViewSymbol, setTradingViewSymbol] = useState<SymbolSnapshot | null>(null);
  const [orderMargin, setOrderMargin] = useState("100");
  const [orderLeverage, setOrderLeverage] = useState("10");
  const [tradeBusy, setTradeBusy] = useState(false);
  const [tradeError, setTradeError] = useState<string | null>(null);
  const [notificationPermission, setNotificationPermission] = useState(() =>
    "Notification" in window ? Notification.permission : "unsupported",
  );
  const notified = useRef(new Map<string, string>());
  const macroRequest = useRef<Promise<void> | null>(null);
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
    void loadMacro();
  }, []);

  useEffect(() => {
    if (viewMode !== "radar") {
      setTradingViewSymbol(null);
    }
  }, [viewMode]);

  useEffect(() => {
    if (!tradingViewSymbol) {
      return;
    }

    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") {
        setTradingViewSymbol(null);
      }
    }

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [tradingViewSymbol]);

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
      if (event.type === "paper_updated") {
        setSnapshot((current) => ({ ...current, paper: event.data }));
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

  function loadMacro(force = false): Promise<void> {
    if (macroRequest.current && !force) {
      return macroRequest.current;
    }

    setMacroLoading(true);
    setMacroError(null);

    let request: Promise<void>;
    request = fetchBtcMacro()
      .then((data) => {
        if (macroRequest.current === request) {
          setMacroSnapshot(data);
        }
      })
      .catch((requestError) => {
        if (macroRequest.current === request) {
          setMacroError(requestError instanceof Error ? requestError.message : String(requestError));
        }
      })
      .finally(() => {
        if (macroRequest.current === request) {
          macroRequest.current = null;
          setMacroLoading(false);
        }
      });
    macroRequest.current = request;
    return request;
  }

  async function submitPaperOrder(side: PaperSide) {
    if (!selected) {
      return;
    }

    const margin = Number(orderMargin);
    const leverage = Number(orderLeverage);
    setTradeBusy(true);
    setTradeError(null);
    try {
      const paper = await openPaperOrder({
        inst_id: selected.inst_id,
        side,
        margin,
        leverage,
      });
      setSnapshot((current) => ({ ...current, paper }));
    } catch (error) {
      setTradeError(error instanceof Error ? error.message : String(error));
    } finally {
      setTradeBusy(false);
    }
  }

  async function submitPaperClose() {
    if (!selected) {
      return;
    }

    setTradeBusy(true);
    setTradeError(null);
    try {
      const paper = await closePaperPosition(selected.inst_id);
      setSnapshot((current) => ({ ...current, paper }));
    } catch (error) {
      setTradeError(error instanceof Error ? error.message : String(error));
    } finally {
      setTradeBusy(false);
    }
  }

  function openTradingView(symbol: SymbolSnapshot) {
    setSelectedId(symbol.inst_id);
    setTradingViewSymbol(symbol);
  }

  return (
    <ConsoleShell
      backendState={backendState}
      copy={copy}
      language={language}
      lastScanAt={snapshot.last_scan_at_ms}
      notificationPermission={notificationPermission}
      onLanguageChange={setLanguage}
      onRequestNotifications={requestNotifications}
      onThemeModeChange={setThemeMode}
      onViewModeChange={setViewMode}
      streamState={streamState}
      symbolCount={snapshot.symbols.length}
      themeMode={themeMode}
      viewMode={viewMode}
    >
      {viewMode === "macro" ? (
        <MacroPanel
          copy={copy}
          error={macroError}
          loading={macroLoading}
          onRefresh={() => {
            void loadMacro(true);
          }}
          snapshot={macroSnapshot}
          themeMode={themeMode}
        />
      ) : (
        <>
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
                          <div className="symbol-cell">
                            <div className="symbol-cell-main">
                              <strong>{symbol.inst_id}</strong>
                              <span>{formatTags(symbol.pool_tags, copy)}</span>
                            </div>
                            <button
                              aria-label={formatTemplate(copy.actions.openTradingViewChart, symbol.inst_id)}
                              className="symbol-tv-button"
                              onClick={(event) => {
                                event.stopPropagation();
                                openTradingView(symbol);
                              }}
                              title={copy.actions.openTradingView}
                              type="button"
                            >
                              TV
                            </button>
                          </div>
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
                {selected ? (
                  <SymbolDetail
                    copy={copy}
                    onClosePaper={submitPaperClose}
                    onOpenTradingView={openTradingView}
                    onLeverageChange={setOrderLeverage}
                    onMarginChange={setOrderMargin}
                    onOpenPaper={submitPaperOrder}
                    orderLeverage={orderLeverage}
                    orderMargin={orderMargin}
                    paper={snapshot.paper}
                    symbol={selected}
                    themeMode={themeMode}
                    tradeBusy={tradeBusy}
                    tradeError={tradeError}
                  />
                ) : null}
              </aside>
            </section>
          )}
        </>
      )}
      {viewMode === "radar" && tradingViewSymbol ? (
        <TradingViewModal
          copy={copy}
          language={language}
          onClose={() => setTradingViewSymbol(null)}
          symbol={tradingViewSymbol}
          themeMode={themeMode}
        />
      ) : null}
    </ConsoleShell>
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
  onClosePaper,
  onOpenTradingView,
  onLeverageChange,
  onMarginChange,
  onOpenPaper,
  orderLeverage,
  orderMargin,
  paper,
  symbol,
  themeMode,
  tradeBusy,
  tradeError,
}: {
  copy: Copy;
  onClosePaper: () => void;
  onOpenTradingView: (symbol: SymbolSnapshot) => void;
  onLeverageChange: (value: string) => void;
  onMarginChange: (value: string) => void;
  onOpenPaper: (side: PaperSide) => void;
  orderLeverage: string;
  orderMargin: string;
  paper: PaperAccountSnapshot;
  symbol: SymbolSnapshot;
  themeMode: ThemeMode;
  tradeBusy: boolean;
  tradeError: string | null;
}) {
  const position = paper.positions.find((item) => item.inst_id === symbol.inst_id);
  const trades = paper.trades
    .filter((trade) => trade.inst_id === symbol.inst_id)
    .slice(0, 5);

  return (
    <>
      <header className="detail-header">
        <div>
          <h2>{symbol.inst_id}</h2>
          <p>{symbol.trigger_reason || copy.detail.noActiveTrigger}</p>
        </div>
        <button
          aria-label={formatTemplate(copy.actions.openTradingViewChart, symbol.inst_id)}
          className="detail-tv-button"
          onClick={() => onOpenTradingView(symbol)}
          type="button"
        >
          {copy.actions.openTradingView}
        </button>
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
      <ChartPanel copy={copy} symbol={symbol} themeMode={themeMode} />
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
        <dl className="paper-metrics">
          <div>
            <dt>{copy.paper.equity}</dt>
            <dd>{formatUsdt(paper.equity)}</dd>
          </div>
          <div>
            <dt>{copy.paper.available}</dt>
            <dd>{formatUsdt(paper.available_balance)}</dd>
          </div>
          <div>
            <dt>{copy.paper.usedMargin}</dt>
            <dd>{formatUsdt(paper.used_margin)}</dd>
          </div>
          <div>
            <dt>{copy.paper.unrealized}</dt>
            <dd className={pnlClass(paper.unrealized_pnl)}>
              {formatSignedUsdt(paper.unrealized_pnl)}
            </dd>
          </div>
        </dl>
        <div className="paper-order">
          <label>
            <span>{copy.paper.margin}</span>
            <input
              min="1"
              onChange={(event) => onMarginChange(event.target.value)}
              step="1"
              type="number"
              value={orderMargin}
            />
          </label>
          <label>
            <span>{copy.paper.leverage}</span>
            <input
              max="50"
              min="1"
              onChange={(event) => onLeverageChange(event.target.value)}
              step="1"
              type="number"
              value={orderLeverage}
            />
          </label>
        </div>
        <div className="paper-actions">
          <button
            className="buy-button"
            disabled={tradeBusy}
            onClick={() => onOpenPaper("long")}
            type="button"
          >
            {copy.actions.openLong}
          </button>
          <button
            className="sell-button"
            disabled={tradeBusy}
            onClick={() => onOpenPaper("short")}
            type="button"
          >
            {copy.actions.openShort}
          </button>
        </div>
        {tradeError ? (
          <p className="paper-error">
            {copy.paper.orderError}: {tradeError}
          </p>
        ) : null}
        <section className="paper-subsection">
          <h3>{copy.paper.position}</h3>
          {position ? (
            <>
              <dl className="paper-position">
                <div>
                  <dt>{copy.paper.side}</dt>
                  <dd>{copy.directions[position.side]}</dd>
                </div>
                <div>
                  <dt>{copy.paper.entry}</dt>
                  <dd>{formatPrice(position.entry_price)}</dd>
                </div>
                <div>
                  <dt>{copy.paper.mark}</dt>
                  <dd>{formatPrice(position.mark_price)}</dd>
                </div>
                <div>
                  <dt>{copy.paper.qty}</dt>
                  <dd>{formatQuantity(position.qty)}</dd>
                </div>
                <div>
                  <dt>{copy.paper.notional}</dt>
                  <dd>{formatUsdt(position.notional)}</dd>
                </div>
                <div>
                  <dt>{copy.paper.pnl}</dt>
                  <dd className={pnlClass(position.unrealized_pnl)}>
                    {formatSignedUsdt(position.unrealized_pnl)} /{" "}
                    {formatPct(position.pnl_pct)}
                  </dd>
                </div>
              </dl>
              <button
                className="close-button"
                disabled={tradeBusy}
                onClick={onClosePaper}
                type="button"
              >
                {copy.actions.closePosition}
              </button>
            </>
          ) : (
            <p className="muted">{copy.paper.noPosition}</p>
          )}
        </section>
        <section className="paper-subsection">
          <h3>{copy.paper.history}</h3>
          {trades.length === 0 ? (
            <p className="muted">{copy.paper.noTrades}</p>
          ) : (
            <ul className="trade-list">
              {trades.map((trade) => (
                <li key={trade.id}>
                  <span>
                    {copy.paper.tradeActions[trade.action]}{" "}
                    {copy.directions[trade.side]} @ {formatPrice(trade.price)}
                  </span>
                  <strong className={pnlClass(trade.realized_pnl)}>
                    {formatSignedUsdt(trade.realized_pnl)}
                  </strong>
                </li>
              ))}
            </ul>
          )}
        </section>
      </section>
    </>
  );
}

function TradingViewModal({
  copy,
  language,
  onClose,
  symbol,
  themeMode,
}: {
  copy: Copy;
  language: Language;
  onClose: () => void;
  symbol: SymbolSnapshot;
  themeMode: ThemeMode;
}) {
  const tradingViewSymbol = resolveTradingViewSymbol(symbol.inst_id);
  const title = `${symbol.inst_id} TradingView`;
  return (
    <div className="tv-modal-backdrop" onClick={onClose}>
      <section
        aria-label={title}
        aria-modal="true"
        className="tv-modal"
        onClick={(event) => event.stopPropagation()}
        role="dialog"
      >
        <header>
          <div>
            <h2>{symbol.inst_id}</h2>
            <p>{tradingViewSymbol ?? copy.detail.tradingViewUnavailable}</p>
          </div>
          <button
            aria-label={copy.actions.closeTradingView}
            className="tv-modal-close"
            onClick={onClose}
            type="button"
          >
            x
          </button>
        </header>
        <div className="tv-modal-frame-wrap">
          {tradingViewSymbol ? (
            <iframe
              allow="fullscreen"
              src={buildTradingViewEmbedUrl(tradingViewSymbol, themeMode, language)}
              title={title}
            />
          ) : (
            <div className="tv-modal-empty">{copy.detail.tradingViewUnavailable}</div>
          )}
        </div>
      </section>
    </div>
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

function resolveTradingViewSymbol(instId: string): string | null {
  const normalized = instId.toUpperCase().replace(/[^A-Z0-9-]/g, "");
  const swapMatch = normalized.match(/^([A-Z0-9]+)-USDT-SWAP$/);
  if (swapMatch) {
    return `OKX:${swapMatch[1]}USDT.P`;
  }

  const spotMatch = normalized.match(/^([A-Z0-9]+)-USDT$/);
  if (spotMatch) {
    return `OKX:${spotMatch[1]}USDT`;
  }

  const compact = normalized.replace(/-/g, "");
  return compact.length > 0 ? `OKX:${compact}` : null;
}

function buildTradingViewEmbedUrl(symbol: string, themeMode: ThemeMode, language: Language): string {
  const params = new URLSearchParams({
    symbol,
    interval: "15",
    theme: themeMode === "light" ? "light" : "dark",
    style: "1",
    locale: language === "zh" ? "zh_CN" : "en",
    enable_publishing: "0",
    allow_symbol_change: "0",
    hide_top_toolbar: "0",
    withdateranges: "1",
  });
  return `https://s.tradingview.com/widgetembed/?${params.toString()}`;
}
