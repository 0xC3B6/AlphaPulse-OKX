export type Direction = "long" | "short" | "neutral";

export interface Score {
  value: number;
  direction: Direction;
  reasons: string[];
}

export interface SymbolSnapshot {
  inst_id: string;
  price: number;
  change_5m_pct: number;
  change_15m_pct: number;
  change_1h_pct: number;
  trend_score: Score;
  range_score: Score;
  pool_tags: string[];
  trigger_reason: string;
  funding_rate: number | null;
  fvgs: FvgZone[];
  levels: LevelZone[];
  updated_at_ms: number;
}

export interface FvgZone {
  timeframe: "m5" | "m15" | "h1";
  direction: Direction;
  lower: number;
  upper: number;
  gap_pct: number;
  distance_pct: number;
  filled: boolean;
}

export interface LevelZone {
  kind: "support" | "resistance";
  lower: number;
  upper: number;
  touches: number;
  distance_pct: number;
}

export interface DashboardSnapshot {
  symbols: SymbolSnapshot[];
  last_scan_at_ms: number | null;
  websocket_connected: boolean;
}

export type BackendEvent =
  | { type: "snapshot"; data: DashboardSnapshot }
  | { type: "symbol_updated"; data: SymbolSnapshot };
