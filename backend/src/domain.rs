use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Long,
    Short,
    Neutral,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Timeframe {
    M5,
    M15,
    H1,
    D1,
    W1,
}

impl Timeframe {
    pub fn okx_bar(self) -> &'static str {
        match self {
            Self::M5 => "5m",
            Self::M15 => "15m",
            Self::H1 => "1H",
            Self::D1 => "1D",
            Self::W1 => "1W",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Candle {
    pub ts_ms: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Score {
    pub value: u8,
    pub direction: Direction,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FvgZone {
    pub timeframe: Timeframe,
    pub direction: Direction,
    pub start_ts_ms: i64,
    pub end_ts_ms: i64,
    pub lower: f64,
    pub upper: f64,
    pub gap_pct: f64,
    pub distance_pct: f64,
    pub filled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LevelKind {
    Support,
    Resistance,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LevelZone {
    pub kind: LevelKind,
    pub lower: f64,
    pub upper: f64,
    pub touches: usize,
    pub distance_pct: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatternKind {
    DoubleBottom,
    DoubleTop,
    SweepFailure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatternStatus {
    Forming,
    Confirmed,
    Retest,
    Holding,
    Invalidated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatternPivotRole {
    LeftLow,
    RightLow,
    LeftHigh,
    RightHigh,
    Neckline,
    SweepReference,
    Sweep,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatternPivot {
    pub role: PatternPivotRole,
    pub ts_ms: i64,
    pub price: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatternLevelZone {
    pub lower: f64,
    pub upper: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatternSignal {
    pub kind: PatternKind,
    pub direction: Direction,
    pub timeframe: Timeframe,
    pub status: PatternStatus,
    pub score: u8,
    pub structure_score: u8,
    pub confirmation_score: u8,
    pub hold_score: u8,
    pub trade_score: u8,
    pub neckline: Option<f64>,
    pub invalidation_level: Option<f64>,
    pub start_ts_ms: i64,
    pub confirm_ts_ms: Option<i64>,
    pub pivots: Vec<PatternPivot>,
    pub level_zone: Option<PatternLevelZone>,
    pub reasons: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScalpingMetrics {
    pub volume_ratio: f64,
    pub vwap: Option<f64>,
    pub vwap_distance_atr: Option<f64>,
    pub latest_move_atr: Option<f64>,
    pub atr_15m_pct: Option<f64>,
    pub adx_15m: Option<f64>,
    pub bollinger_width_pct: Option<f64>,
}

impl Default for ScalpingMetrics {
    fn default() -> Self {
        Self {
            volume_ratio: 1.0,
            vwap: None,
            vwap_distance_atr: None,
            latest_move_atr: None,
            atr_15m_pct: None,
            adx_15m: None,
            bollinger_width_pct: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SymbolSnapshot {
    pub inst_id: String,
    pub price: f64,
    pub change_5m_pct: f64,
    pub change_15m_pct: f64,
    pub change_1h_pct: f64,
    pub trend_score: Score,
    pub range_score: Score,
    pub pool_tags: Vec<String>,
    pub trigger_reason: String,
    pub funding_rate: Option<f64>,
    #[serde(default)]
    pub scalping_metrics: ScalpingMetrics,
    pub fvgs: Vec<FvgZone>,
    pub levels: Vec<LevelZone>,
    pub pattern_signals: Vec<PatternSignal>,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChartSnapshot {
    pub inst_id: String,
    pub timeframe: Timeframe,
    pub candles: Vec<Candle>,
    pub fvgs: Vec<FvgZone>,
    pub pattern_signals: Vec<PatternSignal>,
    pub updated_at_ms: i64,
}
