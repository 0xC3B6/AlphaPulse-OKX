import { useEffect, useMemo, useRef, useState } from "react";
import { connectEvents, fetchSnapshot } from "./api";
import {
  sendBrowserNotification,
  shouldNotify,
} from "./notifications";
import "./styles.css";
import type { BackendEvent, DashboardSnapshot, SymbolSnapshot } from "./types";

type Filter = "all" | "trend" | "range" | "hot" | "fixed";

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
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [notificationPermission, setNotificationPermission] = useState(() =>
    "Notification" in window ? Notification.permission : "unsupported",
  );
  const notified = useRef(new Map<string, string>());

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
          <p>USDT perpetual radar</p>
        </div>
        <dl className="status-grid" aria-label="connection status">
          <div>
            <dt>Backend</dt>
            <dd>{backendState}</dd>
          </div>
          <div>
            <dt>Stream</dt>
            <dd>{streamState}</dd>
          </div>
          <div>
            <dt>Notifications</dt>
            <dd>{notificationPermission}</dd>
          </div>
          <div>
            <dt>Last scan</dt>
            <dd>{formatTimestamp(snapshot.last_scan_at_ms)}</dd>
          </div>
          <div>
            <dt>Symbols</dt>
            <dd>{snapshot.symbols.length}</dd>
          </div>
        </dl>
      </header>

      <section className="toolbar" aria-label="radar filters">
        {[
          ["all", "All"],
          ["trend", "Trend"],
          ["range", "Range"],
          ["hot", "Hot"],
          ["fixed", "Fixed"],
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
        <button onClick={requestNotifications} type="button">
          Enable notifications
        </button>
      </section>

      {filteredSymbols.length === 0 ? (
        <section className="empty-state">
          <h2>No symbols loaded</h2>
          <p>Start the Rust backend to populate the radar.</p>
        </section>
      ) : (
        <section className="radar-grid">
          <div className="table-panel">
            <table>
              <thead>
                <tr>
                  <th>Symbol</th>
                  <th>Price</th>
                  <th>5m</th>
                  <th>15m</th>
                  <th>1h</th>
                  <th>Trend</th>
                  <th>Range</th>
                  <th>Signal</th>
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
                      <span>{symbol.pool_tags.join(" / ") || "unlabeled"}</span>
                    </td>
                    <td>{formatPrice(symbol.price)}</td>
                    <td>{formatPct(symbol.change_5m_pct)}</td>
                    <td>{formatPct(symbol.change_15m_pct)}</td>
                    <td>{formatPct(symbol.change_1h_pct)}</td>
                    <td>{formatScore(symbol.trend_score)}</td>
                    <td>{formatScore(symbol.range_score)}</td>
                    <td>{symbol.trigger_reason || "watching"}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <aside className="detail-panel">
            {selected ? <SymbolDetail symbol={selected} /> : null}
          </aside>
        </section>
      )}
    </main>
  );
}

function SymbolDetail({ symbol }: { symbol: SymbolSnapshot }) {
  return (
    <>
      <header>
        <h2>{symbol.inst_id}</h2>
        <p>{symbol.trigger_reason || "No active trigger"}</p>
      </header>
      <dl className="detail-list">
        <div>
          <dt>Funding</dt>
          <dd>{symbol.funding_rate === null ? "-" : formatPct(symbol.funding_rate)}</dd>
        </div>
        <div>
          <dt>Updated</dt>
          <dd>{formatTimestamp(symbol.updated_at_ms)}</dd>
        </div>
      </dl>
      <section>
        <h3>FVG</h3>
        {symbol.fvgs.length === 0 ? (
          <p className="muted">No FVG zones</p>
        ) : (
          <ul>
            {symbol.fvgs.map((zone, index) => (
              <li key={`${zone.timeframe}-${zone.direction}-${index}`}>
                {zone.timeframe} {zone.direction} {formatPrice(zone.lower)}-
                {formatPrice(zone.upper)} dist {formatPct(zone.distance_pct)}
              </li>
            ))}
          </ul>
        )}
      </section>
      <section>
        <h3>Levels</h3>
        {symbol.levels.length === 0 ? (
          <p className="muted">No levels</p>
        ) : (
          <ul>
            {symbol.levels.map((level, index) => (
              <li key={`${level.kind}-${index}`}>
                {level.kind} {formatPrice(level.lower)}-{formatPrice(level.upper)}{" "}
                touches {level.touches}
              </li>
            ))}
          </ul>
        )}
      </section>
      <section>
        <h3>Account</h3>
        <p className="muted">No read-only OKX API key connected.</p>
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

function formatScore(score: SymbolSnapshot["trend_score"]): string {
  return `${score.value} ${score.direction}`;
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
