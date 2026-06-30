export type Direction = "long" | "short" | "neutral";

export type MacroRegime =
  | "bull_expansion"
  | "late_cycle_distribution"
  | "bear_market"
  | "bear_market_rally"
  | "bottoming_accumulation"
  | "neutral";

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

export interface Candle {
  ts_ms: number;
  open: number;
  high: number;
  low: number;
  close: number;
  volume: number;
}

export interface FvgZone {
  timeframe: "m5" | "m15" | "h1";
  direction: Direction;
  start_ts_ms: number;
  end_ts_ms: number;
  lower: number;
  upper: number;
  gap_pct: number;
  distance_pct: number;
  filled: boolean;
}

export interface ChartSnapshot {
  inst_id: string;
  timeframe: "m5" | "m15" | "h1";
  candles: Candle[];
  fvgs: FvgZone[];
  updated_at_ms: number;
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
  paper: PaperAccountSnapshot;
}

export type BackendEvent =
  | { type: "snapshot"; data: DashboardSnapshot }
  | { type: "symbol_updated"; data: SymbolSnapshot }
  | { type: "paper_updated"; data: PaperAccountSnapshot };

export type PaperSide = "long" | "short";

export type PaperTradeAction = "open" | "close";

export interface PaperOrderRequest {
  inst_id: string;
  side: PaperSide;
  margin: number;
  leverage: number;
}

export interface PaperAccountSnapshot {
  mode: "paper";
  initial_balance: number;
  realized_pnl: number;
  unrealized_pnl: number;
  equity: number;
  used_margin: number;
  available_balance: number;
  positions: PaperPositionSnapshot[];
  trades: PaperTrade[];
}

export interface PaperPositionSnapshot {
  inst_id: string;
  side: PaperSide;
  qty: number;
  entry_price: number;
  mark_price: number;
  margin: number;
  leverage: number;
  notional: number;
  unrealized_pnl: number;
  pnl_pct: number;
  opened_at_ms: number;
}

export interface PaperTrade {
  id: number;
  inst_id: string;
  side: PaperSide;
  action: PaperTradeAction;
  price: number;
  qty: number;
  margin: number;
  notional: number;
  realized_pnl: number;
  ts_ms: number;
}

export interface BtcMacroSnapshot {
  asset: string;
  updated_at_ms: number;
  price: number;
  regime: MacroRegime;
  confidence: number;
  summary: string;
  cycle: HalvingCycleSnapshot;
  trend: MacroTrendSnapshot;
  momentum: MacroMomentumSnapshot;
  events: MacroEvent[];
  valuation_metrics: ExternalMetricStatus[];
  ahr999_history?: Ahr999History | null;
  analogs: HistoricalAnalog[];
  analog_comparisons?: AnalogComparisonSet[];
  trading_bias: string[];
}

export interface HalvingCycleSnapshot {
  last_halving_ms: number;
  next_halving_estimate_ms: number;
  days_since_halving: number;
  estimated_cycle_progress_pct: number;
  cycle_year: number;
  cycle_quarter: number;
  phase: string;
}

export interface MacroTrendSnapshot {
  window_ath: number;
  window_ath_ts_ms: number;
  drawdown_from_window_ath_pct: number;
  ma_200w: number | null;
  price_vs_200w_pct: number | null;
  weekly_ma200_status: string;
}

export interface MacroMomentumSnapshot {
  change_30d_pct: number | null;
  change_90d_pct: number | null;
  change_26w_pct: number | null;
  volatility_90d_pct: number | null;
}

export interface MacroEvent {
  id: string;
  title: string;
  event_type: string;
  date_ms: number;
  days_to_event: number;
  phase: string;
  impact_tags: string[];
}

export interface ExternalMetricStatus {
  id: string;
  name: string;
  status: string;
  note: string;
  value?: number | null;
  date?: string | null;
  source?: string | null;
  zone?: string | null;
  updated_at_ms?: number | null;
}

export interface HistoricalAnalog {
  label: string;
  score: number;
  rationale: string[];
  components?: AnalogScoreComponent[];
}

export interface Ahr999History {
  source: string;
  points: Ahr999HistoryPoint[];
  bands: Ahr999RangeBand[];
}

export interface Ahr999HistoryPoint {
  ts_ms: number;
  date: string;
  value: number;
  btc_price: number;
  gma200: number;
  model_price: number;
  zone: string;
}

export interface Ahr999RangeBand {
  id: string;
  label: string;
  lower?: number | null;
  upper?: number | null;
  days: number;
  recommendation: string;
}

export interface AnalogScoreComponent {
  label: string;
  points: number;
  max_points: number;
  detail: string;
}

export interface AnalogComparisonSet {
  timeframe_days: number;
  current?: AnalogPathSummary | null;
  matches: AnalogMatch[];
}

export interface AnalogPathSummary {
  start_ts_ms: number;
  end_ts_ms: number;
  final_return_pct: number;
  max_drawdown_pct: number;
  max_runup_pct: number;
  candles: AnalogKline[];
  path: AnalogPathPoint[];
}

export interface AnalogKline {
  ts_ms: number;
  offset_days: number;
  open: number;
  high: number;
  low: number;
  close: number;
  index_open: number;
  index_high: number;
  index_low: number;
  index_close: number;
}

export interface AnalogPathPoint {
  offset_days: number;
  return_pct: number;
}

export interface AnalogMatch {
  id: string;
  label: string;
  score: number;
  start_ts_ms: number;
  end_ts_ms: number;
  final_return_pct: number;
  max_drawdown_pct: number;
  max_runup_pct: number;
  components: AnalogScoreComponent[];
  lookback: AnalogPathSummary;
  forward?: AnalogPathSummary | null;
  path: AnalogPathPoint[];
}
