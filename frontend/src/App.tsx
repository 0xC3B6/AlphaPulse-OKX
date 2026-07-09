import { useEffect, useMemo, useRef, useState } from "react";
import {
  closePaperPosition,
  connectEvents,
  fetchBtcMacro,
  fetchSnapshot,
  openPaperOrder,
  resetStrategyVersion,
  startStrategyVersion,
  stopStrategyVersion,
} from "./api";
import { ConsoleShell } from "./ConsoleShell";
import { MacroPanel } from "./MacroPanel";
import { MonitorPage } from "./MonitorPage";
import {
  sendBrowserNotification,
  shouldNotify,
} from "./notifications";
import { ReviewPage } from "./ReviewPage";
import "./styles.css";
import { StrategyPage } from "./StrategyPage";
import { TradePage } from "./TradePage";
import { TradingViewModal } from "./TradingViewModal";
import { defaultLanguage, translations } from "./i18n";
import type { Language } from "./i18n";
import { buildTerminalOverview } from "./figmaTerminal";
import {
  matchesFilter,
  maxScore,
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
  StrategyCenterSnapshot,
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
  strategy_center: undefined,
};

export default function App() {
  const [snapshot, setSnapshot] = useState<DashboardSnapshot>(emptySnapshot);
  const [macroSnapshot, setMacroSnapshot] = useState<BtcMacroSnapshot | null>(null);
  const [macroLoading, setMacroLoading] = useState(false);
  const [macroError, setMacroError] = useState<string | null>(null);
  const [backendState, setBackendState] = useState<"connected" | "disconnected">(
    "disconnected",
  );
  const [streamState, setStreamState] = useState<"connected" | "idle" | "reconnecting" | "stale">("idle");
  const [viewMode, setViewMode] = useState<ViewMode>("monitor");
  const [filter, setFilter] = useState<Filter>("all");
  const [themeMode, setThemeMode] = useState<ThemeMode>(() => readStoredTheme());
  const [language, setLanguage] = useState<Language>(() => readStoredLanguage());
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [tradingViewSymbol, setTradingViewSymbol] = useState<SymbolSnapshot | null>(null);
  const [orderInstrument, setOrderInstrument] = useState("");
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
        setOrderInstrument((current) => current || data.symbols[0]?.inst_id || "");
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
    if (viewMode !== "monitor") {
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

    const connection = connectEvents(
      (event: BackendEvent) => {
        setStreamState("connected");
        if (event.type === "snapshot") {
          setSnapshot(event.data);
          setOrderInstrument((current) => current || event.data.symbols[0]?.inst_id || "");
          return;
        }
        if (event.type === "paper_updated") {
          setSnapshot((current) => ({ ...current, paper: event.data }));
          return;
        }
        if (event.type === "strategy_updated") {
          setSnapshot((current) => applyStrategyCenter(current, event.data));
          return;
        }

        setSnapshot((current) => upsertSymbol(current, event.data));
        if (shouldNotify(event.data, notified.current, 80)) {
          sendBrowserNotification(event.data);
        }
      },
      {
        onClose: () => setStreamState("reconnecting"),
        onError: () => setStreamState("reconnecting"),
        onOpen: () => {
          setStreamState("connected");
          fetchSnapshot()
            .then((data) => {
              setSnapshot(data);
              setBackendState("connected");
              setOrderInstrument((current) => current || data.symbols[0]?.inst_id || "");
            })
            .catch(() => setBackendState("disconnected"));
        },
        onReconnectAttempt: () => setStreamState("reconnecting"),
        onStale: () => setStreamState("stale"),
      },
    );

    return () => connection.close();
  }, []);

  const sortedSymbols = useMemo(
    () =>
      [...snapshot.symbols].sort(
        (left, right) => maxScore(right) - maxScore(left),
      ),
    [snapshot.symbols],
  );

  const filteredSymbols = useMemo(
    () => sortedSymbols.filter((symbol) => matchesFilter(symbol, filter, snapshot.paper)),
    [filter, snapshot.paper, sortedSymbols],
  );
  const terminalOverview = useMemo(
    () => buildTerminalOverview(snapshot),
    [snapshot],
  );
  const selected =
    filteredSymbols.find((symbol) => symbol.inst_id === selectedId) ??
    null;
  const selectedTradePosition =
    snapshot.paper.positions.find((position) => position.inst_id === orderInstrument) ??
    snapshot.paper.positions[0] ??
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

  async function submitPaperOrder(side: PaperSide, instId = orderInstrument || selected?.inst_id) {
    const target =
      snapshot.symbols.find((symbol) => symbol.inst_id === instId) ??
      selected ??
      null;
    if (!target) {
      return;
    }

    const margin = Number(orderMargin);
    const leverage = Number(orderLeverage);
    setTradeBusy(true);
    setTradeError(null);
    try {
      const paper = await openPaperOrder({
        inst_id: target.inst_id,
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

  async function submitPaperClose(instId = selectedTradePosition?.inst_id ?? selected?.inst_id) {
    if (!instId) {
      return;
    }

    setTradeBusy(true);
    setTradeError(null);
    try {
      const paper = await closePaperPosition(instId);
      setSnapshot((current) => ({ ...current, paper }));
    } catch (error) {
      setTradeError(error instanceof Error ? error.message : String(error));
    } finally {
      setTradeBusy(false);
    }
  }

  async function submitStrategyStart(versionCode: string) {
    const center = await startStrategyVersion(versionCode);
    setSnapshot((current) => applyStrategyCenter(current, center));
  }

  async function submitStrategyStop(versionCode: string) {
    const center = await stopStrategyVersion(versionCode);
    setSnapshot((current) => applyStrategyCenter(current, center));
  }

  async function submitStrategyReset(versionCode: string) {
    const center = await resetStrategyVersion(versionCode);
    setSnapshot((current) => applyStrategyCenter(current, center));
  }

  function openTradingView(symbol: SymbolSnapshot) {
    setTradingViewSymbol(symbol);
  }

  function tradeSymbol(symbol: SymbolSnapshot) {
    setSelectedId(symbol.inst_id);
    setOrderInstrument(symbol.inst_id);
    setViewMode("trade");
  }

  return (
    <ConsoleShell
      activeSignalCount={terminalOverview.activeSignalCount}
      backendState={backendState}
      copy={copy}
      language={language}
      lastScanAt={snapshot.last_scan_at_ms}
      notificationPermission={notificationPermission}
      onLanguageChange={setLanguage}
      onRequestNotifications={requestNotifications}
      onThemeModeChange={setThemeMode}
      onViewModeChange={setViewMode}
      positionCount={terminalOverview.positionCount}
      streamState={streamState}
      symbolCount={snapshot.symbols.length}
      tickerSymbols={sortedSymbols}
      themeMode={themeMode}
      unrealizedPnl={terminalOverview.unrealizedPnl}
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
      ) : viewMode === "trade" ? (
        <TradePage
          copy={copy}
          onClosePaper={submitPaperClose}
          onInstrumentChange={setOrderInstrument}
          onLeverageChange={setOrderLeverage}
          onMarginChange={setOrderMargin}
          onOpenPaper={submitPaperOrder}
          onSelectPosition={setOrderInstrument}
          orderInstrument={orderInstrument}
          orderLeverage={orderLeverage}
          orderMargin={orderMargin}
          paper={snapshot.paper}
          selectedPosition={selectedTradePosition}
          symbols={snapshot.symbols}
          tradeBusy={tradeBusy}
          tradeError={tradeError}
        />
      ) : viewMode === "strategy" ? (
          <StrategyPage
            copy={copy}
            lastScanAt={snapshot.last_scan_at_ms}
            onResetStrategyVersion={submitStrategyReset}
            onStartStrategyVersion={submitStrategyStart}
            onStopStrategyVersion={submitStrategyStop}
            paper={snapshot.paper}
            strategyCenter={snapshot.strategy_center}
            symbols={sortedSymbols}
          />
      ) : viewMode === "review" ? (
        <ReviewPage copy={copy} paper={snapshot.paper} strategyCenter={snapshot.strategy_center} />
      ) : (
        <MonitorPage
          copy={copy}
          filter={filter}
          filteredSymbols={filteredSymbols}
          macroError={macroError}
          macroLoading={macroLoading}
          macroSnapshot={macroSnapshot}
          onFilterChange={setFilter}
          onOpenTradingView={openTradingView}
          onSelectSymbol={setSelectedId}
          onTradeSymbol={tradeSymbol}
          paper={snapshot.paper}
          selected={selected}
          themeMode={themeMode}
        />
      )}
      {viewMode === "monitor" && tradingViewSymbol ? (
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

function upsertSymbol(
  snapshot: DashboardSnapshot,
  symbol: SymbolSnapshot,
): DashboardSnapshot {
  const symbols = snapshot.symbols.filter((item) => item.inst_id !== symbol.inst_id);
  symbols.push(symbol);
  return { ...snapshot, symbols };
}

function applyStrategyCenter(
  snapshot: DashboardSnapshot,
  strategyCenter: StrategyCenterSnapshot,
): DashboardSnapshot {
  const defaultVersion =
    strategyCenter.versions.find((version) => version.version.version_code === "v0.1.3") ??
    strategyCenter.versions[0];
  return {
    ...snapshot,
    paper: defaultVersion?.paper ?? snapshot.paper,
    strategy_center: strategyCenter,
  };
}
