import { describe, expect, it } from "vitest";
import {
  buildTerminalOverview,
  mapTerminalDirection,
  mapTerminalSignal,
  toTerminalSymbol,
} from "./figmaTerminal";
import type { DashboardSnapshot, PaperAccountSnapshot, SymbolSnapshot } from "./types";

const paper: PaperAccountSnapshot = {
  mode: "paper",
  initial_balance: 10000,
  strategy_version: "v0.1.3",
  strategy_build_id: "legacy-v3-replay-2026-07-10",
  config_hash: "fixture-v3-config-hash",
  run_id: "v0.1.3-restored-paper-1",
  persistence: {
    status: "healthy",
    last_committed_at_ms: 1783660000000,
    last_error: null,
  },
  realized_pnl: 120,
  unrealized_pnl: -25,
  equity: 10095,
  used_margin: 300,
  available_balance: 9795,
  positions: [
    {
      inst_id: "LAB-USDT-SWAP",
      side: "short",
      qty: 10,
      entry_price: 18,
      mark_price: 17.5,
      margin: 100,
      leverage: 10,
      notional: 1000,
      unrealized_pnl: 25,
      pnl_pct: 0.25,
      opened_at_ms: 1782400000000,
    },
  ],
  trades: [],
};

const lab: SymbolSnapshot = {
  inst_id: "LAB-USDT-SWAP",
  price: 17.2,
  change_5m_pct: -0.03,
  change_15m_pct: 0,
  change_1h_pct: 0.11,
  trend_score: {
    value: 84,
    direction: "short",
    reasons: ["15m move aligns with 1h move", "volume 3.1x"],
  },
  range_score: {
    value: 42,
    direction: "neutral",
    reasons: ["clear recent range"],
  },
  pool_tags: ["dynamic", "new_listing"],
  trigger_reason: "trend short 84: volume 3.1x",
  funding_rate: -0.003,
  fvgs: [],
  levels: [],
  updated_at_ms: 1782400000000,
};

describe("figma terminal adapters", () => {
  it("maps numeric percent changes to compact timeframe directions", () => {
    expect(mapTerminalDirection(0.0001)).toBe("UP");
    expect(mapTerminalDirection(-0.0001)).toBe("DOWN");
    expect(mapTerminalDirection(0)).toBe("FLAT");
  });

  it("maps backend score directions to Figma signal labels", () => {
    expect(mapTerminalSignal("long")).toBe("LONG");
    expect(mapTerminalSignal("short")).toBe("SHORT");
    expect(mapTerminalSignal("neutral")).toBe("FLAT");
  });

  it("derives a Figma-style symbol row from backend data without mock fields", () => {
    const row = toTerminalSymbol(lab, paper);

    expect(row.id).toBe("LAB-USDT-SWAP");
    expect(row.base).toBe("LAB");
    expect(row.chg).toBe(0.11);
    expect(row.m5).toBe("DOWN");
    expect(row.m15).toBe("FLAT");
    expect(row.h1).toBe("UP");
    expect(row.trend).toBe(84);
    expect(row.range).toBe(42);
    expect(row.signal).toBe("SHORT");
    expect(row.cat).toBe("dynamic");
    expect(row.tags).toEqual(["dynamic", "new_listing"]);
    expect(row.hasPosition).toBe(true);
    expect(row.triggerReason).toBe("trend short 84: volume 3.1x");
  });

  it("builds header metrics from live dashboard data", () => {
    const snapshot: DashboardSnapshot = {
      symbols: [lab],
      last_scan_at_ms: 1782400000000,
      websocket_connected: true,
      paper,
    };

    const overview = buildTerminalOverview(snapshot);

    expect(overview.positionCount).toBe(1);
    expect(overview.activeSignalCount).toBe(1);
    expect(overview.unrealizedPnl).toBe(-25);
    expect(overview.symbols[0]?.id).toBe("LAB-USDT-SWAP");
  });
});
