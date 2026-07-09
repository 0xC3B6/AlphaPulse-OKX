export type Direction = "long" | "short" | "neutral";

export type MacroRegime =
  | "bull_expansion"
  | "late_cycle_distribution"
  | "bear_market"
  | "bear_market_rally"
  | "bottoming_accumulation"
  | "neutral";

export type TrendStructure =
  | "strong_bull"
  | "bull_pullback"
  | "repair_rally"
  | "bear_trend"
  | "choppy";

export type MacroPermissionState =
  | "trade_allowed"
  | "reduced_risk"
  | "only_btc_eth"
  | "observe_only"
  | "radar_silent";

export type RadarPriority = "high" | "medium" | "low" | "silent";

export type LeverageHint = "normal" | "reduced" | "avoid";

export interface RadarPolicy {
  altcoin_notify: boolean;
  max_priority: RadarPriority;
  leverage_hint: LeverageHint;
}

export interface MacroPermissionSnapshot {
  state: MacroPermissionState;
  radar_policy: RadarPolicy;
  allowed_behaviors: string[];
  reasons: string[];
}

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
  change_24h_pct?: number | null;
  change_48h_pct?: number | null;
  change_72h_pct?: number | null;
  intraday_low_break_count?: number;
  high_volatility_flag?: boolean;
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
  strategy_center?: StrategyCenterSnapshot;
}

export type BackendEvent =
  | { type: "snapshot"; data: DashboardSnapshot }
  | { type: "symbol_updated"; data: SymbolSnapshot }
  | { type: "paper_updated"; data: PaperAccountSnapshot }
  | { type: "strategy_updated"; data: StrategyCenterSnapshot };

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
  fee_rate?: number;
  slippage_rate?: number;
  total_fees?: number;
  total_trades?: number;
  closed_position_count?: number;
  winning_closed_position_count?: number;
  losing_closed_position_count?: number;
  win_rate?: number | null;
  average_holding_duration_ms?: number | null;
  average_closed_position_pnl?: number | null;
  average_winning_pnl?: number | null;
  average_losing_pnl?: number | null;
  profit_factor?: number | null;
  largest_winning_pnl?: number | null;
  largest_losing_pnl?: number | null;
  strategy_stats?: PaperStrategyStats[];
  realized_pnl: number;
  unrealized_pnl: number;
  equity: number;
  used_margin: number;
  available_balance: number;
  positions: PaperPositionSnapshot[];
  position_history?: PaperClosedPositionSnapshot[];
  trades: PaperTrade[];
}

export interface PaperStrategyStats {
  strategy_name: string;
  strategy_version: string;
  total_trades: number;
  closed_position_count: number;
  winning_closed_position_count: number;
  losing_closed_position_count: number;
  win_rate: number | null;
  realized_pnl: number;
  total_fees: number;
  first_trade_ts_ms: number | null;
  last_trade_ts_ms: number | null;
  running_duration_ms: number | null;
  average_holding_duration_ms: number | null;
  average_position_pnl: number | null;
  profit_factor: number | null;
  largest_winning_pnl: number | null;
  largest_losing_pnl: number | null;
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
  source?: string;
  strategy_name?: string;
  strategy_version?: string;
  version_code?: string;
  run_id?: string;
  primary_signal?: string;
  reason?: string;
  tags?: TradeTagLike[];
  risk_flags?: string[];
  risk_guard_decision?: string | null;
  strategy_reason?: string;
  config_hash?: string;
}

export interface PaperClosedPositionSnapshot {
  id: number;
  inst_id: string;
  side: PaperSide;
  qty: number;
  entry_price: number;
  exit_price: number;
  margin: number;
  leverage: number;
  notional: number;
  fees: number;
  realized_pnl: number;
  pnl_pct: number;
  opened_at_ms: number;
  closed_at_ms: number;
  duration_ms: number;
  source: string;
  strategy_name?: string;
  strategy_version?: string;
  version_code?: string;
  run_id?: string;
  primary_signal?: string;
  reason: string;
  close_source: string;
  close_reason: string;
  tags?: TradeTagLike[];
  open_tags?: TradeTagLike[];
  close_tags?: TradeTagLike[];
  risk_flags?: string[];
  risk_guard_decision?: string | null;
  strategy_reason?: string;
  config_hash?: string;
  max_adverse_excursion?: number | null;
  max_favorable_excursion?: number | null;
  planned_risk_usdt?: number | null;
  r_multiple?: number | null;
}

export interface PaperTrade {
  id: number;
  inst_id: string;
  side: PaperSide;
  action: PaperTradeAction;
  source?: string;
  strategy_name?: string;
  strategy_version?: string;
  version_code?: string;
  run_id?: string;
  primary_signal?: string;
  reason?: string;
  price: number;
  qty: number;
  margin: number;
  notional: number;
  fee?: number;
  slippage_rate?: number;
  tags?: TradeTagLike[];
  risk_flags?: string[];
  risk_guard_decision?: string | null;
  strategy_reason?: string;
  config_hash?: string;
  realized_pnl: number;
  ts_ms: number;
}

export type TradeTagLike = TradeTag | string;

export interface TradeTag {
  kind: string;
  label: string;
  score_impact: number;
  reason: string;
  ts_ms: number;
}

export type StrategyVersionStatus = "active" | "testing" | "archived" | "disabled";
export type StrategyRunMode = "paper" | "shadow" | "live" | "disabled";
export type StrategyRunStatus = "running" | "stopped" | "reset" | "archived";
export type OrderAction = "open" | "close" | "reduce";
export type AttributionConfidence = "insufficient_sample" | "low" | "medium" | "high";
export type AttributionSuggestion =
  | "insufficient_sample"
  | "quality"
  | "keep"
  | "observe"
  | "fragile"
  | "downgrade";

export interface StrategyVersion {
  version_code: string;
  name: string;
  description: string;
  status: StrategyVersionStatus;
  config_json: unknown;
  config_hash: string;
  created_at_ms: number;
  updated_at_ms: number;
}

export interface StrategyRun {
  run_id: string;
  version_code: string;
  mode: StrategyRunMode;
  status: StrategyRunStatus;
  initial_equity: number;
  current_equity: number;
  realized_pnl: number;
  unrealized_pnl: number;
  fee_total: number;
  max_drawdown: number;
  start_time_ms: number;
  end_time_ms: number | null;
  fee_model: string;
  slippage_model: string;
  config_snapshot: unknown;
}

export interface OrderIntent {
  version_code: string;
  run_id: string;
  symbol: string;
  side: PaperSide;
  action: OrderAction;
  margin: number;
  leverage: number;
  score: number;
  primary_signal: string;
  reason: string;
  tags: readonly string[];
  stop_loss: number | null;
  take_profit: number | null;
  expire_at: number | null;
  risk_flags: readonly string[];
  risk_guard_decision: string | null;
  config_hash: string;
}

export interface RiskGuardEvent {
  id: number;
  run_id: string;
  version_code: string;
  timestamp_ms: number;
  symbol: string;
  side: PaperSide;
  original_signal: string;
  score: number;
  action: string;
  reason: string;
  risk_flags: string[];
  original_order_intent: OrderIntent;
  final_order_intent: OrderIntent | null;
}

export interface AttributionRow {
  key: string;
  sample_count: number;
  profit_factor: number | null;
  net_pnl: number;
  win_rate: number | null;
  avg_pnl: number | null;
  avg_win: number | null;
  avg_loss: number | null;
  max_loss: number | null;
  stop_loss_rate: number | null;
  take_profit_rate: number | null;
  confidence: AttributionConfidence;
  suggestion: AttributionSuggestion;
}

export interface StrategyEquitySnapshot {
  run_id: string;
  version_code: string;
  timestamp_ms: number;
  equity: number;
  realized_pnl: number;
  unrealized_pnl: number;
  drawdown: number;
  open_positions_count: number;
}

export interface StrategyVersionOverview {
  version_code: string;
  name: string;
  status: StrategyVersionStatus;
  mode: StrategyRunMode;
  run_id: string;
  run_time_ms: number;
  initial_equity: number;
  current_equity: number;
  realized_pnl: number;
  unrealized_pnl: number;
  return_pct: number;
  max_drawdown: number;
  win_rate: number | null;
  profit_factor: number | null;
  closed_trades: number;
  open_positions: number;
  total_fee: number;
  config_hash: string;
  last_updated_ms: number;
}

export interface StrategyVersionSnapshot {
  version: StrategyVersion;
  run: StrategyRun;
  overview: StrategyVersionOverview;
  paper: PaperAccountSnapshot;
  equity: StrategyEquitySnapshot[];
  signal_attribution: AttributionRow[];
  tag_attribution: AttributionRow[];
  combo_attribution: AttributionRow[];
  symbol_attribution: AttributionRow[];
  risk_guard_events: RiskGuardEvent[];
}

export interface StrategyCenterSnapshot {
  versions: StrategyVersionSnapshot[];
  last_updated_ms: number;
}

export interface BtcMacroSnapshot {
  asset: string;
  updated_at_ms: number;
  price: number;
  regime: MacroRegime;
  market_permission: MacroPermissionSnapshot;
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
  ma_50d: number | null;
  ma_200d: number | null;
  price_vs_50d_pct: number | null;
  price_vs_200d_pct: number | null;
  ma_50d_slope_30d_pct: number | null;
  ma_200d_slope_30d_pct: number | null;
  structure: TrendStructure;
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
  cohort_stats: AnalogCohortStats[];
}

export interface AnalogCohortStats {
  requested_size: number;
  sample_size: number;
  up_probability: number;
  median_forward_return_pct: number;
  lower_quartile_forward_return_pct: number;
  median_forward_drawdown_pct: number;
  median_forward_runup_pct: number;
  score_floor?: number | null;
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
