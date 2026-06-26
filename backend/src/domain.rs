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
}

impl Timeframe {
    pub fn okx_bar(self) -> &'static str {
        match self {
            Self::M5 => "5m",
            Self::M15 => "15m",
            Self::H1 => "1H",
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
    pub fvgs: Vec<FvgZone>,
    pub levels: Vec<LevelZone>,
    pub updated_at_ms: i64,
}
