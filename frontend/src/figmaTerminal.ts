import type {
  DashboardSnapshot,
  Direction,
  PaperAccountSnapshot,
  SymbolSnapshot,
} from "./types";
import { primaryScore } from "./uiFormat";

export type TerminalTimeframeDirection = "UP" | "DOWN" | "FLAT";
export type TerminalSignal = "LONG" | "SHORT" | "FLAT";
export type TerminalSymbolCategory = "fixed" | "dynamic" | "hot";

export interface TerminalSymbol {
  id: string;
  base: string;
  price: number;
  chg: number;
  m5: TerminalTimeframeDirection;
  m15: TerminalTimeframeDirection;
  h1: TerminalTimeframeDirection;
  trend: number;
  range: number;
  signal: TerminalSignal;
  tags: string[];
  cat: TerminalSymbolCategory;
  hasPosition: boolean;
  triggerReason: string;
  source: SymbolSnapshot;
}

export interface TerminalOverview {
  activeSignalCount: number;
  positionCount: number;
  symbols: TerminalSymbol[];
  unrealizedPnl: number;
}

export function mapTerminalDirection(changePct: number): TerminalTimeframeDirection {
  if (changePct > 0) {
    return "UP";
  }
  if (changePct < 0) {
    return "DOWN";
  }
  return "FLAT";
}

export function mapTerminalSignal(direction: Direction): TerminalSignal {
  if (direction === "long") {
    return "LONG";
  }
  if (direction === "short") {
    return "SHORT";
  }
  return "FLAT";
}

export function toTerminalSymbol(
  symbol: SymbolSnapshot,
  paper?: PaperAccountSnapshot,
): TerminalSymbol {
  const signal = primaryScore(symbol);
  const tags = symbol.pool_tags.length > 0 ? symbol.pool_tags : ["unlabeled"];

  return {
    id: symbol.inst_id,
    base: formatBaseSymbol(symbol.inst_id),
    price: symbol.price,
    chg: symbol.change_1h_pct,
    m5: mapTerminalDirection(symbol.change_5m_pct),
    m15: mapTerminalDirection(symbol.change_15m_pct),
    h1: mapTerminalDirection(symbol.change_1h_pct),
    trend: Math.round(symbol.trend_score.value),
    range: Math.round(symbol.range_score.value),
    signal: mapTerminalSignal(signal.direction),
    tags,
    cat: inferTerminalCategory(tags),
    hasPosition: paper?.positions.some((position) => position.inst_id === symbol.inst_id) ?? false,
    triggerReason: symbol.trigger_reason,
    source: symbol,
  };
}

export function buildTerminalOverview(snapshot: DashboardSnapshot): TerminalOverview {
  const symbols = snapshot.symbols.map((symbol) => toTerminalSymbol(symbol, snapshot.paper));
  return {
    activeSignalCount: symbols.filter((symbol) => symbol.signal !== "FLAT").length,
    positionCount: snapshot.paper.positions.length,
    symbols,
    unrealizedPnl: snapshot.paper.unrealized_pnl,
  };
}

function formatBaseSymbol(instId: string): string {
  return instId
    .replace(/-USDT-SWAP$/u, "")
    .replace(/-USDT$/u, "")
    .replace(/USDT\.P$/u, "");
}

function inferTerminalCategory(tags: string[]): TerminalSymbolCategory {
  if (tags.includes("fixed")) {
    return "fixed";
  }
  if (tags.includes("hot") || tags.includes("mover")) {
    return "hot";
  }
  return "dynamic";
}
