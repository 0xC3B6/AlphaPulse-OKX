# OKX Perpetual Trading Radar Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the first working local OKX USDT perpetual trading radar with a Rust backend, TypeScript frontend, real-time price updates, dual scoring, simplified FVG detection, support/resistance context, and browser notifications.

**Architecture:** The backend owns all exchange communication, normalization, market-state caching, indicator calculation, scoring, and alert state. The frontend connects to the local backend through HTTP for snapshots and WebSocket for live updates. Version 1 is a manual decision-support system and contains no trading actions or account API keys.

**Tech Stack:** Rust, tokio, axum, reqwest, tokio-tungstenite, serde, tracing, TypeScript, Vite, React, Vitest.

---

## Source Spec

- `docs/superpowers/specs/2026-06-25-okx-perp-trading-radar-design.md`

## File Structure

Create this structure:

```text
Cargo.toml
backend/Cargo.toml
backend/src/lib.rs
backend/src/main.rs
backend/src/config.rs
backend/src/domain.rs
backend/src/indicators/mod.rs
backend/src/indicators/fvg.rs
backend/src/indicators/levels.rs
backend/src/universe.rs
backend/src/scoring.rs
backend/src/alerts.rs
backend/src/okx/mod.rs
backend/src/okx/rest.rs
backend/src/okx/ws.rs
backend/src/state.rs
backend/src/server.rs
backend/src/runtime.rs
backend/tests/domain_config.rs
backend/tests/fvg.rs
backend/tests/levels.rs
backend/tests/universe.rs
backend/tests/scoring.rs
backend/tests/alerts.rs
backend/tests/okx_parsing.rs
backend/tests/server.rs
frontend/package.json
frontend/index.html
frontend/vite.config.ts
frontend/tsconfig.json
frontend/tsconfig.node.json
frontend/src/main.tsx
frontend/src/App.tsx
frontend/src/api.ts
frontend/src/types.ts
frontend/src/notifications.ts
frontend/src/styles.css
frontend/src/App.test.tsx
frontend/src/notifications.test.ts
```

Responsibility map:

- `domain.rs`: shared backend data types for candles, symbols, scores, FVG zones, levels, alerts, and snapshots.
- `config.rs`: local app configuration defaults and load path.
- `indicators/*`: pure, deterministic indicator functions.
- `universe.rs`: fixed and dynamic symbol pool construction.
- `scoring.rs`: heuristic trend/range scoring and explanation generation.
- `alerts.rs`: high-score transition detection and deduplication.
- `okx/*`: OKX REST parsing and WebSocket subscription boundary.
- `state.rs`: in-memory radar state shared by REST, WebSocket, and runtime tasks.
- `server.rs`: axum router, REST routes, and browser-facing WebSocket route.
- `runtime.rs`: scan loop and stream event handling.
- `frontend/src/*`: browser dashboard, API/WebSocket client, notification state, and styling.

## Task 1: Workspace And Backend Skeleton

**Files:**
- Create: `Cargo.toml`
- Create: `backend/Cargo.toml`
- Create: `backend/src/lib.rs`
- Create: `backend/src/main.rs`

- [ ] **Step 1: Create the Rust workspace files**

Use this root `Cargo.toml`:

```toml
[workspace]
members = ["backend"]
resolver = "2"

[workspace.package]
edition = "2021"
license = "MIT"
```

Use this `backend/Cargo.toml`:

```toml
[package]
name = "alphapulse_okx_backend"
version = "0.1.0"
edition.workspace = true
license.workspace = true

[dependencies]
anyhow = "1"
axum = { version = "0.7", features = ["ws"] }
chrono = { version = "0.4", features = ["serde"] }
futures-util = "0.3"
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"
tokio = { version = "1", features = ["macros", "rt-multi-thread", "signal", "sync", "time"] }
tokio-tungstenite = { version = "0.24", features = ["rustls-tls-webpki-roots"] }
tower-http = { version = "0.5", features = ["cors", "trace"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
url = "2"

[dev-dependencies]
tower = "0.5"
```

- [ ] **Step 2: Create minimal backend entry points**

Use this `backend/src/lib.rs`:

```rust
pub mod alerts;
pub mod config;
pub mod domain;
pub mod indicators;
pub mod okx;
pub mod runtime;
pub mod scoring;
pub mod server;
pub mod state;
pub mod universe;
```

Use this initial `backend/src/main.rs`:

```rust
fn main() {
    println!("AlphaPulse OKX backend skeleton");
}
```

- [ ] **Step 3: Add temporary empty module files**

Create each referenced module with the smallest valid content:

```rust
// backend/src/<module>.rs
```

For `backend/src/indicators/mod.rs`:

```rust
pub mod fvg;
pub mod levels;
```

For `backend/src/okx/mod.rs`:

```rust
pub mod rest;
pub mod ws;
```

For the remaining module files, add an empty file so the module tree compiles.

- [ ] **Step 4: Run backend compilation**

Run:

```bash
cargo check -p alphapulse_okx_backend
```

Expected: PASS.

- [ ] **Step 5: Commit the skeleton**

```bash
git add Cargo.toml backend/Cargo.toml backend/src
git commit -m "chore: scaffold Rust backend workspace"
```

## Task 2: Domain Types And Configuration

**Files:**
- Modify: `backend/src/domain.rs`
- Modify: `backend/src/config.rs`
- Modify: `backend/src/server.rs`
- Modify: `backend/src/main.rs`
- Create: `backend/tests/domain_config.rs`

- [ ] **Step 1: Write config and domain tests**

Use this `backend/tests/domain_config.rs`:

```rust
use alphapulse_okx_backend::{
    config::AppConfig,
    domain::{Direction, Score, Timeframe},
};

#[test]
fn default_config_matches_v1_decisions() {
    let config = AppConfig::default();

    assert_eq!(config.host, "127.0.0.1");
    assert_eq!(config.port, 8787);
    assert_eq!(config.scan_interval_secs, 30);
    assert_eq!(config.dynamic_pool_size, 40);
    assert_eq!(config.trend_alert_threshold, 80);
    assert_eq!(config.range_alert_threshold, 80);
    assert!(config.fixed_watchlist.contains(&"BTC-USDT-SWAP".to_string()));
    assert!(config.fixed_watchlist.contains(&"LAB-USDT-SWAP".to_string()));
}

#[test]
fn domain_types_serialize_with_stable_names() {
    let score = Score {
        value: 84,
        direction: Direction::Short,
        reasons: vec!["15m drop expanded".to_string(), "volume 3.1x".to_string()],
    };

    let json = serde_json::to_string(&score).unwrap();

    assert!(json.contains("\"value\":84"));
    assert!(json.contains("\"direction\":\"short\""));
    assert!(json.contains("15m drop expanded"));
}

#[test]
fn timeframe_maps_to_okx_bar_names() {
    assert_eq!(Timeframe::M5.okx_bar(), "5m");
    assert_eq!(Timeframe::M15.okx_bar(), "15m");
    assert_eq!(Timeframe::H1.okx_bar(), "1H");
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run:

```bash
cargo test -p alphapulse_okx_backend --test domain_config
```

Expected: FAIL with unresolved imports for `AppConfig`, `Direction`, `Score`, and `Timeframe`.

- [ ] **Step 3: Implement domain types**

Use this shape in `backend/src/domain.rs`:

```rust
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
```

- [ ] **Step 4: Implement app config and minimal server function**

Use this `backend/src/config.rs`:

```rust
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub scan_interval_secs: u64,
    pub dynamic_pool_size: usize,
    pub trend_alert_threshold: u8,
    pub range_alert_threshold: u8,
    pub watch_threshold: u8,
    pub fixed_watchlist: Vec<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8787,
            scan_interval_secs: 30,
            dynamic_pool_size: 40,
            trend_alert_threshold: 80,
            range_alert_threshold: 80,
            watch_threshold: 65,
            fixed_watchlist: vec![
                "BTC-USDT-SWAP",
                "ETH-USDT-SWAP",
                "SOL-USDT-SWAP",
                "XRP-USDT-SWAP",
                "DOGE-USDT-SWAP",
                "LAB-USDT-SWAP",
                "RAVE-USDT-SWAP",
                "BEAT-USDT-SWAP",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
        }
    }
}
```

Use this temporary `backend/src/server.rs` so the crate compiles:

```rust
use crate::config::AppConfig;

pub async fn serve(_config: AppConfig) -> anyhow::Result<()> {
    Ok(())
}
```

Replace `backend/src/main.rs` with:

```rust
use alphapulse_okx_backend::{config::AppConfig, server};
use anyhow::Context;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = AppConfig::default();
    server::serve(config)
        .await
        .context("backend server exited with an error")
}
```

- [ ] **Step 5: Run the tests and compilation**

Run:

```bash
cargo test -p alphapulse_okx_backend --test domain_config
cargo check -p alphapulse_okx_backend
```

Expected: both PASS.

- [ ] **Step 6: Commit**

```bash
git add backend/src/domain.rs backend/src/config.rs backend/src/server.rs backend/src/main.rs backend/tests/domain_config.rs
git commit -m "feat: add backend domain and config types"
```

## Task 3: Simplified FVG Detection

**Files:**
- Modify: `backend/src/indicators/fvg.rs`
- Create: `backend/tests/fvg.rs`

- [ ] **Step 1: Write FVG tests**

Use this `backend/tests/fvg.rs`:

```rust
use alphapulse_okx_backend::{
    domain::{Candle, Direction, Timeframe},
    indicators::fvg::detect_fvgs,
};

fn candle(ts_ms: i64, high: f64, low: f64, close: f64) -> Candle {
    Candle {
        ts_ms,
        open: close,
        high,
        low,
        close,
        volume: 100.0,
    }
}

#[test]
fn detects_bullish_three_candle_gap() {
    let candles = vec![
        candle(1, 10.0, 9.5, 9.8),
        candle(2, 10.4, 9.7, 10.1),
        candle(3, 11.4, 10.8, 11.2),
    ];

    let zones = detect_fvgs(&candles, Timeframe::M15, 0.02, 11.2);

    assert_eq!(zones.len(), 1);
    assert_eq!(zones[0].direction, Direction::Long);
    assert_eq!(zones[0].lower, 10.0);
    assert_eq!(zones[0].upper, 10.8);
    assert!(!zones[0].filled);
}

#[test]
fn marks_bullish_gap_filled_when_later_low_revisits_zone() {
    let candles = vec![
        candle(1, 10.0, 9.5, 9.8),
        candle(2, 10.4, 9.7, 10.1),
        candle(3, 11.4, 10.8, 11.2),
        candle(4, 11.1, 10.6, 10.7),
    ];

    let zones = detect_fvgs(&candles, Timeframe::M15, 0.02, 10.7);

    assert_eq!(zones.len(), 1);
    assert!(zones[0].filled);
}

#[test]
fn detects_bearish_three_candle_gap() {
    let candles = vec![
        candle(1, 20.4, 20.0, 20.2),
        candle(2, 20.2, 19.4, 19.8),
        candle(3, 19.1, 18.5, 18.8),
    ];

    let zones = detect_fvgs(&candles, Timeframe::M5, 0.02, 18.8);

    assert_eq!(zones.len(), 1);
    assert_eq!(zones[0].direction, Direction::Short);
    assert_eq!(zones[0].lower, 19.1);
    assert_eq!(zones[0].upper, 20.0);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test -p alphapulse_okx_backend --test fvg
```

Expected: FAIL because `detect_fvgs` is not implemented.

- [ ] **Step 3: Implement FVG detector**

Use this function contract in `backend/src/indicators/fvg.rs`:

```rust
use crate::domain::{Candle, Direction, FvgZone, Timeframe};

pub fn detect_fvgs(
    candles: &[Candle],
    timeframe: Timeframe,
    min_gap_pct: f64,
    current_price: f64,
) -> Vec<FvgZone> {
    if candles.len() < 3 || current_price <= 0.0 {
        return Vec::new();
    }

    let mut zones = Vec::new();

    for index in 0..=(candles.len() - 3) {
        let first = &candles[index];
        let third = &candles[index + 2];
        let later = &candles[(index + 3)..];

        if first.high < third.low {
            let lower = first.high;
            let upper = third.low;
            let gap_pct = (upper - lower) / lower;
            if gap_pct >= min_gap_pct {
                let filled = later.iter().any(|candle| candle.low <= upper);
                zones.push(FvgZone {
                    timeframe,
                    direction: Direction::Long,
                    lower,
                    upper,
                    gap_pct,
                    distance_pct: zone_distance_pct(current_price, lower, upper),
                    filled,
                });
            }
        }

        if first.low > third.high {
            let lower = third.high;
            let upper = first.low;
            let gap_pct = (upper - lower) / lower;
            if gap_pct >= min_gap_pct {
                let filled = later.iter().any(|candle| candle.high >= lower);
                zones.push(FvgZone {
                    timeframe,
                    direction: Direction::Short,
                    lower,
                    upper,
                    gap_pct,
                    distance_pct: zone_distance_pct(current_price, lower, upper),
                    filled,
                });
            }
        }
    }

    zones
}

fn zone_distance_pct(price: f64, lower: f64, upper: f64) -> f64 {
    if price >= lower && price <= upper {
        0.0
    } else if price < lower {
        (lower - price) / price
    } else {
        (price - upper) / price
    }
}
```

- [ ] **Step 4: Run tests**

Run:

```bash
cargo test -p alphapulse_okx_backend --test fvg
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add backend/src/indicators/fvg.rs backend/tests/fvg.rs
git commit -m "feat: detect simplified FVG zones"
```

## Task 4: Support And Resistance Zones

**Files:**
- Modify: `backend/src/indicators/levels.rs`
- Create: `backend/tests/levels.rs`

- [ ] **Step 1: Write support/resistance tests**

Use this `backend/tests/levels.rs`:

```rust
use alphapulse_okx_backend::{
    domain::{Candle, LevelKind},
    indicators::levels::{find_levels, LevelConfig},
};

fn candle(ts_ms: i64, high: f64, low: f64, close: f64, volume: f64) -> Candle {
    Candle { ts_ms, open: close, high, low, close, volume }
}

#[test]
fn clusters_repeated_support_levels() {
    let candles = vec![
        candle(1, 18.0, 15.2, 16.0, 100.0),
        candle(2, 17.0, 15.3, 16.2, 130.0),
        candle(3, 18.2, 15.25, 17.5, 160.0),
        candle(4, 19.0, 16.4, 18.5, 100.0),
    ];

    let levels = find_levels(&candles, 16.0, LevelConfig { cluster_pct: 0.01, min_touches: 2 });

    let support = levels.iter().find(|level| level.kind == LevelKind::Support).unwrap();
    assert!(support.lower <= 15.2);
    assert!(support.upper >= 15.3);
    assert_eq!(support.touches, 3);
}

#[test]
fn clusters_repeated_resistance_levels() {
    let candles = vec![
        candle(1, 20.0, 17.0, 18.0, 100.0),
        candle(2, 20.2, 18.0, 19.0, 120.0),
        candle(3, 20.1, 18.4, 19.5, 110.0),
        candle(4, 18.8, 16.4, 17.0, 180.0),
    ];

    let levels = find_levels(&candles, 19.0, LevelConfig { cluster_pct: 0.015, min_touches: 2 });

    let resistance = levels.iter().find(|level| level.kind == LevelKind::Resistance).unwrap();
    assert!(resistance.lower <= 20.0);
    assert!(resistance.upper >= 20.2);
    assert_eq!(resistance.touches, 3);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test -p alphapulse_okx_backend --test levels
```

Expected: FAIL because `find_levels` and `LevelConfig` are not implemented.

- [ ] **Step 3: Implement level clustering**

Use this contract in `backend/src/indicators/levels.rs`:

```rust
use crate::domain::{Candle, LevelKind, LevelZone};

#[derive(Debug, Clone, Copy)]
pub struct LevelConfig {
    pub cluster_pct: f64,
    pub min_touches: usize,
}

pub fn find_levels(candles: &[Candle], current_price: f64, config: LevelConfig) -> Vec<LevelZone> {
    if candles.is_empty() || current_price <= 0.0 {
        return Vec::new();
    }

    let mut levels = Vec::new();
    levels.extend(cluster_prices(
        candles.iter().map(|candle| candle.low).collect(),
        LevelKind::Support,
        current_price,
        config,
    ));
    levels.extend(cluster_prices(
        candles.iter().map(|candle| candle.high).collect(),
        LevelKind::Resistance,
        current_price,
        config,
    ));
    levels.sort_by(|left, right| {
        left.distance_pct
            .partial_cmp(&right.distance_pct)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    levels
}

fn cluster_prices(
    mut prices: Vec<f64>,
    kind: LevelKind,
    current_price: f64,
    config: LevelConfig,
) -> Vec<LevelZone> {
    prices.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));

    let mut zones = Vec::new();
    let mut index = 0;
    while index < prices.len() {
        let anchor = prices[index];
        let mut cluster = vec![anchor];
        index += 1;

        while index < prices.len() && (prices[index] - anchor).abs() / anchor <= config.cluster_pct {
            cluster.push(prices[index]);
            index += 1;
        }

        if cluster.len() >= config.min_touches {
            let lower = cluster.iter().copied().fold(f64::INFINITY, f64::min);
            let upper = cluster.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            zones.push(LevelZone {
                kind,
                lower,
                upper,
                touches: cluster.len(),
                distance_pct: distance_pct(current_price, lower, upper),
            });
        }
    }

    zones
}

fn distance_pct(price: f64, lower: f64, upper: f64) -> f64 {
    if price >= lower && price <= upper {
        0.0
    } else if price < lower {
        (lower - price) / price
    } else {
        (price - upper) / price
    }
}
```

- [ ] **Step 4: Run tests**

Run:

```bash
cargo test -p alphapulse_okx_backend --test levels
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add backend/src/indicators/levels.rs backend/tests/levels.rs
git commit -m "feat: cluster support and resistance zones"
```

## Task 5: Symbol Universe Ranking

**Files:**
- Modify: `backend/src/universe.rs`
- Create: `backend/tests/universe.rs`

- [ ] **Step 1: Write universe tests**

Use this `backend/tests/universe.rs`:

```rust
use alphapulse_okx_backend::universe::{build_symbol_universe, MarketActivity};

#[test]
fn merges_dynamic_and_fixed_symbols_without_duplicates() {
    let activity = vec![
        MarketActivity::new("LAB-USDT-SWAP", 10_000_000.0, 0.12, 0.18, 0.20, 3.2),
        MarketActivity::new("BTC-USDT-SWAP", 50_000_000.0, 0.01, 0.02, 0.03, 1.1),
        MarketActivity::new("RAVE-USDT-SWAP", 8_000_000.0, -0.08, -0.16, 0.25, 2.8),
    ];
    let fixed = vec!["BTC-USDT-SWAP".to_string(), "DOGE-USDT-SWAP".to_string()];

    let universe = build_symbol_universe(&activity, &fixed, 2);

    assert_eq!(universe.iter().filter(|symbol| symbol.inst_id == "BTC-USDT-SWAP").count(), 1);
    assert!(universe.iter().any(|symbol| symbol.inst_id == "DOGE-USDT-SWAP" && symbol.tags.contains(&"fixed".to_string())));
    assert!(universe.iter().any(|symbol| symbol.inst_id == "LAB-USDT-SWAP" && symbol.tags.contains(&"dynamic".to_string())));
}

#[test]
fn ranks_activity_by_hotness_score() {
    let activity = vec![
        MarketActivity::new("SLOW-USDT-SWAP", 100_000_000.0, 0.001, 0.002, 0.01, 1.0),
        MarketActivity::new("FAST-USDT-SWAP", 20_000_000.0, 0.10, 0.18, 0.22, 3.0),
    ];

    let universe = build_symbol_universe(&activity, &[], 1);

    assert_eq!(universe[0].inst_id, "FAST-USDT-SWAP");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test -p alphapulse_okx_backend --test universe
```

Expected: FAIL because `MarketActivity` and `build_symbol_universe` are not implemented.

- [ ] **Step 3: Implement universe ranking**

Use this contract in `backend/src/universe.rs`:

```rust
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct MarketActivity {
    pub inst_id: String,
    pub quote_volume_24h: f64,
    pub change_15m_pct: f64,
    pub change_1h_pct: f64,
    pub volatility_1h_pct: f64,
    pub volume_ratio: f64,
}

impl MarketActivity {
    pub fn new(
        inst_id: &str,
        quote_volume_24h: f64,
        change_15m_pct: f64,
        change_1h_pct: f64,
        volatility_1h_pct: f64,
        volume_ratio: f64,
    ) -> Self {
        Self {
            inst_id: inst_id.to_string(),
            quote_volume_24h,
            change_15m_pct,
            change_1h_pct,
            volatility_1h_pct,
            volume_ratio,
        }
    }

    pub fn hotness_score(&self) -> f64 {
        let volume_component = (self.quote_volume_24h.max(1.0).log10() / 10.0).min(1.0) * 20.0;
        let movement_component = (self.change_15m_pct.abs() * 160.0).min(25.0)
            + (self.change_1h_pct.abs() * 100.0).min(20.0);
        let volatility_component = (self.volatility_1h_pct * 100.0).min(20.0);
        let volume_ratio_component = ((self.volume_ratio - 1.0).max(0.0) * 8.0).min(15.0);
        volume_component + movement_component + volatility_component + volume_ratio_component
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UniverseSymbol {
    pub inst_id: String,
    pub tags: Vec<String>,
}

pub fn build_symbol_universe(
    activity: &[MarketActivity],
    fixed_watchlist: &[String],
    dynamic_pool_size: usize,
) -> Vec<UniverseSymbol> {
    let mut ranked = activity.to_vec();
    ranked.sort_by(|left, right| {
        right
            .hotness_score()
            .partial_cmp(&left.hotness_score())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut symbols: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for item in ranked.into_iter().take(dynamic_pool_size) {
        symbols.entry(item.inst_id).or_default().push("dynamic".to_string());
    }

    for inst_id in fixed_watchlist {
        symbols.entry(inst_id.clone()).or_default().push("fixed".to_string());
    }

    let mut output: Vec<UniverseSymbol> = symbols
        .into_iter()
        .map(|(inst_id, mut tags)| {
            tags.sort();
            tags.dedup();
            UniverseSymbol { inst_id, tags }
        })
        .collect();

    output.sort_by(|left, right| {
        let left_score = activity
            .iter()
            .find(|item| item.inst_id == left.inst_id)
            .map(MarketActivity::hotness_score)
            .unwrap_or(0.0);
        let right_score = activity
            .iter()
            .find(|item| item.inst_id == right.inst_id)
            .map(MarketActivity::hotness_score)
            .unwrap_or(0.0);
        right_score
            .partial_cmp(&left_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    output
}
```

- [ ] **Step 4: Run tests**

Run:

```bash
cargo test -p alphapulse_okx_backend --test universe
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add backend/src/universe.rs backend/tests/universe.rs
git commit -m "feat: rank dynamic and fixed symbol universe"
```

## Task 6: Scoring Engine And Alert Deduplication

**Files:**
- Modify: `backend/src/scoring.rs`
- Modify: `backend/src/alerts.rs`
- Create: `backend/tests/scoring.rs`
- Create: `backend/tests/alerts.rs`

- [ ] **Step 1: Write scoring tests**

Use this `backend/tests/scoring.rs`:

```rust
use alphapulse_okx_backend::{
    domain::Direction,
    scoring::{score_symbol, ScoringInput},
};

#[test]
fn scores_strong_short_trend_with_explanations() {
    let input = ScoringInput {
        inst_id: "LAB-USDT-SWAP".to_string(),
        change_5m_pct: -0.045,
        change_15m_pct: -0.082,
        change_1h_pct: -0.12,
        broke_recent_high: false,
        broke_recent_low: true,
        volume_ratio: 3.1,
        nearest_fvg_distance_pct: Some(0.014),
        dynamic_pool: true,
        near_support: false,
        near_resistance: false,
        clear_range: false,
        funding_rate: Some(-0.003),
    };

    let scored = score_symbol(input);

    assert!(scored.trend_score.value >= 80);
    assert_eq!(scored.trend_score.direction, Direction::Short);
    assert!(scored.trend_score.reasons.iter().any(|reason| reason.contains("15m move")));
    assert!(scored.trend_score.reasons.iter().any(|reason| reason.contains("volume")));
}

#[test]
fn scores_range_short_near_resistance() {
    let input = ScoringInput {
        inst_id: "LAB-USDT-SWAP".to_string(),
        change_5m_pct: 0.012,
        change_15m_pct: 0.018,
        change_1h_pct: 0.021,
        broke_recent_high: false,
        broke_recent_low: false,
        volume_ratio: 2.4,
        nearest_fvg_distance_pct: Some(0.006),
        dynamic_pool: true,
        near_support: false,
        near_resistance: true,
        clear_range: true,
        funding_rate: Some(0.002),
    };

    let scored = score_symbol(input);

    assert!(scored.range_score.value >= 80);
    assert_eq!(scored.range_score.direction, Direction::Short);
    assert!(scored.range_score.reasons.iter().any(|reason| reason.contains("resistance")));
}
```

- [ ] **Step 2: Write alert tests**

Use this `backend/tests/alerts.rs`:

```rust
use alphapulse_okx_backend::{
    alerts::{AlertTracker, AlertThresholds},
    domain::{Direction, Score, SymbolSnapshot},
};

fn snapshot(inst_id: &str, trend: u8, range: u8, direction: Direction) -> SymbolSnapshot {
    SymbolSnapshot {
        inst_id: inst_id.to_string(),
        price: 17.2,
        change_5m_pct: -0.03,
        change_15m_pct: -0.07,
        change_1h_pct: -0.11,
        trend_score: Score { value: trend, direction, reasons: vec!["volume 3.1x".to_string()] },
        range_score: Score { value: range, direction: Direction::Neutral, reasons: vec![] },
        pool_tags: vec!["dynamic".to_string()],
        trigger_reason: "trend short 84: volume 3.1x".to_string(),
        funding_rate: Some(-0.003),
        fvgs: vec![],
        levels: vec![],
        updated_at_ms: 1_782_400_000_000,
    }
}

#[test]
fn alerts_only_when_symbol_newly_enters_high_score_state() {
    let mut tracker = AlertTracker::default();
    let thresholds = AlertThresholds { trend: 80, range: 80 };

    let first = tracker.evaluate(&snapshot("LAB-USDT-SWAP", 84, 20, Direction::Short), thresholds);
    let second = tracker.evaluate(&snapshot("LAB-USDT-SWAP", 84, 20, Direction::Short), thresholds);

    assert_eq!(first.len(), 1);
    assert!(second.is_empty());
}

#[test]
fn re_alerts_when_direction_changes() {
    let mut tracker = AlertTracker::default();
    let thresholds = AlertThresholds { trend: 80, range: 80 };

    let _ = tracker.evaluate(&snapshot("LAB-USDT-SWAP", 84, 20, Direction::Short), thresholds);
    let changed = tracker.evaluate(&snapshot("LAB-USDT-SWAP", 86, 20, Direction::Long), thresholds);

    assert_eq!(changed.len(), 1);
    assert_eq!(changed[0].direction, Direction::Long);
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run:

```bash
cargo test -p alphapulse_okx_backend --test scoring --test alerts
```

Expected: FAIL because scoring and alert modules are not implemented.

- [ ] **Step 4: Implement scoring**

Use this contract in `backend/src/scoring.rs`:

```rust
use crate::domain::{Direction, Score};

#[derive(Debug, Clone)]
pub struct ScoringInput {
    pub inst_id: String,
    pub change_5m_pct: f64,
    pub change_15m_pct: f64,
    pub change_1h_pct: f64,
    pub broke_recent_high: bool,
    pub broke_recent_low: bool,
    pub volume_ratio: f64,
    pub nearest_fvg_distance_pct: Option<f64>,
    pub dynamic_pool: bool,
    pub near_support: bool,
    pub near_resistance: bool,
    pub clear_range: bool,
    pub funding_rate: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct ScoredSymbol {
    pub trend_score: Score,
    pub range_score: Score,
}

pub fn score_symbol(input: ScoringInput) -> ScoredSymbol {
    ScoredSymbol {
        trend_score: trend_score(&input),
        range_score: range_score(&input),
    }
}

fn trend_score(input: &ScoringInput) -> Score {
    let mut value = 0_u8;
    let mut reasons = Vec::new();
    let direction = if input.change_15m_pct < -0.025 && input.change_1h_pct < -0.03 {
        Direction::Short
    } else if input.change_15m_pct > 0.025 && input.change_1h_pct > 0.03 {
        Direction::Long
    } else {
        Direction::Neutral
    };

    if direction != Direction::Neutral {
        value += 25;
        reasons.push(format!("15m move {:.1}% aligns with 1h move {:.1}%", input.change_15m_pct * 100.0, input.change_1h_pct * 100.0));
    }
    if input.broke_recent_low && direction == Direction::Short {
        value += 20;
        reasons.push("broke recent low".to_string());
    }
    if input.broke_recent_high && direction == Direction::Long {
        value += 20;
        reasons.push("broke recent high".to_string());
    }
    if input.volume_ratio >= 2.0 {
        value += 20;
        reasons.push(format!("volume {:.1}x", input.volume_ratio));
    }
    if input.nearest_fvg_distance_pct.is_some_and(|distance| distance <= 0.02) {
        value += 15;
        reasons.push("near FVG zone".to_string());
    }
    if input.dynamic_pool {
        value += 10;
        reasons.push("in dynamic hot pool".to_string());
    }
    if input.funding_rate.is_some_and(|rate| rate.abs() >= 0.002) {
        value += 5;
        reasons.push("funding rate is elevated".to_string());
    }

    Score { value: value.min(100), direction, reasons }
}

fn range_score(input: &ScoringInput) -> Score {
    let mut value = 0_u8;
    let mut reasons = Vec::new();
    let mut direction = Direction::Neutral;

    if input.clear_range {
        value += 25;
        reasons.push("clear recent range".to_string());
    }
    if input.near_resistance {
        value += 25;
        direction = Direction::Short;
        reasons.push("near resistance".to_string());
    }
    if input.near_support {
        value += 25;
        direction = Direction::Long;
        reasons.push("near support".to_string());
    }
    if input.volume_ratio >= 2.0 {
        value += 20;
        reasons.push(format!("boundary volume {:.1}x", input.volume_ratio));
    }
    if input.nearest_fvg_distance_pct.is_some_and(|distance| distance <= 0.01) {
        value += 15;
        reasons.push("FVG overlaps nearby area".to_string());
    }
    if input.funding_rate.is_some_and(|rate| rate.abs() >= 0.002) {
        value += 10;
        reasons.push("funding rate supports caution".to_string());
    }
    if input.change_15m_pct.abs() > 0.10 {
        value = value.saturating_sub(15);
        reasons.push("very high 15m movement reduces range quality".to_string());
    }

    Score { value: value.min(100), direction, reasons }
}
```

- [ ] **Step 5: Implement alerts**

Use this contract in `backend/src/alerts.rs`:

```rust
use std::collections::HashMap;

use crate::domain::{Direction, SymbolSnapshot};

#[derive(Debug, Clone, Copy)]
pub struct AlertThresholds {
    pub trend: u8,
    pub range: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlertEvent {
    pub inst_id: String,
    pub kind: String,
    pub score: u8,
    pub direction: Direction,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AlertKey {
    kind: String,
    direction: Direction,
    score_bucket: u8,
}

#[derive(Default)]
pub struct AlertTracker {
    last: HashMap<String, AlertKey>,
}

impl AlertTracker {
    pub fn evaluate(&mut self, snapshot: &SymbolSnapshot, thresholds: AlertThresholds) -> Vec<AlertEvent> {
        let mut events = Vec::new();
        self.maybe_push(
            snapshot,
            "trend",
            snapshot.trend_score.value,
            snapshot.trend_score.direction,
            thresholds.trend,
            &mut events,
        );
        self.maybe_push(
            snapshot,
            "range",
            snapshot.range_score.value,
            snapshot.range_score.direction,
            thresholds.range,
            &mut events,
        );
        events
    }

    fn maybe_push(
        &mut self,
        snapshot: &SymbolSnapshot,
        kind: &str,
        score: u8,
        direction: Direction,
        threshold: u8,
        events: &mut Vec<AlertEvent>,
    ) {
        if score < threshold || direction == Direction::Neutral {
            return;
        }

        let key_name = format!("{}:{}", snapshot.inst_id, kind);
        let next = AlertKey {
            kind: kind.to_string(),
            direction,
            score_bucket: score / 10,
        };

        if self.last.get(&key_name) == Some(&next) {
            return;
        }

        self.last.insert(key_name, next);
        events.push(AlertEvent {
            inst_id: snapshot.inst_id.clone(),
            kind: kind.to_string(),
            score,
            direction,
            message: format!("{} {} {:?} {}: {}", snapshot.inst_id, kind, direction, score, snapshot.trigger_reason),
        });
    }
}
```

- [ ] **Step 6: Run tests**

Run:

```bash
cargo test -p alphapulse_okx_backend --test scoring --test alerts
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add backend/src/scoring.rs backend/src/alerts.rs backend/tests/scoring.rs backend/tests/alerts.rs
git commit -m "feat: score opportunities and dedupe alerts"
```

## Task 7: OKX REST Parsing And WebSocket Message Boundary

**Files:**
- Modify: `backend/src/okx/rest.rs`
- Modify: `backend/src/okx/ws.rs`
- Create: `backend/tests/okx_parsing.rs`

- [ ] **Step 1: Write parsing tests**

Use this `backend/tests/okx_parsing.rs`:

```rust
use alphapulse_okx_backend::okx::{
    rest::{parse_candles, parse_tickers},
    ws::parse_ticker_event,
};

#[test]
fn parses_okx_candle_arrays() {
    let json = r#"{
        "code":"0",
        "msg":"",
        "data":[["1782387000000","16.938","17.241","16.936","17.182","9847.3","98473","1685633.336","1"]]
    }"#;

    let candles = parse_candles(json).unwrap();

    assert_eq!(candles.len(), 1);
    assert_eq!(candles[0].ts_ms, 1782387000000);
    assert_eq!(candles[0].open, 16.938);
    assert_eq!(candles[0].close, 17.182);
    assert_eq!(candles[0].volume, 9847.3);
}

#[test]
fn parses_okx_ticker_rows() {
    let json = r#"{
        "code":"0",
        "msg":"",
        "data":[{"instId":"LAB-USDT-SWAP","last":"17.187","volCcy24h":"20113997","ts":"1782387679663"}]
    }"#;

    let tickers = parse_tickers(json).unwrap();

    assert_eq!(tickers.len(), 1);
    assert_eq!(tickers[0].inst_id, "LAB-USDT-SWAP");
    assert_eq!(tickers[0].last, 17.187);
    assert_eq!(tickers[0].quote_volume_24h, 20113997.0);
}

#[test]
fn parses_okx_ws_ticker_event() {
    let json = r#"{
        "arg":{"channel":"tickers","instId":"LAB-USDT-SWAP"},
        "data":[{"instId":"LAB-USDT-SWAP","last":"17.187","ts":"1782387679663"}]
    }"#;

    let event = parse_ticker_event(json).unwrap().unwrap();

    assert_eq!(event.inst_id, "LAB-USDT-SWAP");
    assert_eq!(event.last, 17.187);
    assert_eq!(event.ts_ms, 1782387679663);
}
```

- [ ] **Step 2: Run parsing tests to verify they fail**

Run:

```bash
cargo test -p alphapulse_okx_backend --test okx_parsing
```

Expected: FAIL because OKX parsing functions do not exist.

- [ ] **Step 3: Implement REST parsing and client types**

In `backend/src/okx/rest.rs`, implement:

```rust
use crate::domain::Candle;
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq)]
pub struct TickerRow {
    pub inst_id: String,
    pub last: f64,
    pub quote_volume_24h: f64,
    pub ts_ms: i64,
}

#[derive(Debug, Deserialize)]
struct OkxResponse<T> {
    data: T,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawTicker {
    inst_id: String,
    last: String,
    vol_ccy24h: String,
    ts: String,
}

pub fn parse_tickers(json: &str) -> anyhow::Result<Vec<TickerRow>> {
    let response: OkxResponse<Vec<RawTicker>> = serde_json::from_str(json)?;
    response
        .data
        .into_iter()
        .map(|row| {
            Ok(TickerRow {
                inst_id: row.inst_id,
                last: row.last.parse()?,
                quote_volume_24h: row.vol_ccy24h.parse()?,
                ts_ms: row.ts.parse()?,
            })
        })
        .collect()
}

pub fn parse_candles(json: &str) -> anyhow::Result<Vec<Candle>> {
    let response: OkxResponse<Vec<Vec<String>>> = serde_json::from_str(json)?;
    response
        .data
        .into_iter()
        .map(|row| {
            anyhow::ensure!(row.len() >= 6, "OKX candle row has fewer than 6 fields");
            Ok(Candle {
                ts_ms: row[0].parse()?,
                open: row[1].parse()?,
                high: row[2].parse()?,
                low: row[3].parse()?,
                close: row[4].parse()?,
                volume: row[5].parse()?,
            })
        })
        .collect()
}

#[derive(Clone)]
pub struct OkxRestClient {
    http: reqwest::Client,
    base_url: String,
}

impl OkxRestClient {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: "https://www.okx.com".to_string(),
        }
    }

    pub async fn get_json(&self, path: &str) -> anyhow::Result<String> {
        let url = format!("{}{}", self.base_url, path);
        Ok(self.http.get(url).send().await?.error_for_status()?.text().await?)
    }
}
```

- [ ] **Step 4: Implement WebSocket message parsing**

In `backend/src/okx/ws.rs`, implement:

```rust
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq)]
pub struct TickerEvent {
    pub inst_id: String,
    pub last: f64,
    pub ts_ms: i64,
}

#[derive(Debug, Deserialize)]
struct WsEnvelope {
    data: Option<Vec<RawWsTicker>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawWsTicker {
    inst_id: String,
    last: String,
    ts: String,
}

pub fn parse_ticker_event(json: &str) -> anyhow::Result<Option<TickerEvent>> {
    let envelope: WsEnvelope = serde_json::from_str(json)?;
    let Some(mut data) = envelope.data else {
        return Ok(None);
    };
    let Some(row) = data.pop() else {
        return Ok(None);
    };
    Ok(Some(TickerEvent {
        inst_id: row.inst_id,
        last: row.last.parse()?,
        ts_ms: row.ts.parse()?,
    }))
}
```

- [ ] **Step 5: Run parsing tests**

Run:

```bash
cargo test -p alphapulse_okx_backend --test okx_parsing
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add backend/src/okx/rest.rs backend/src/okx/ws.rs backend/tests/okx_parsing.rs
git commit -m "feat: parse OKX public market payloads"
```

## Task 8: Backend State And HTTP/WebSocket API

**Files:**
- Modify: `backend/src/state.rs`
- Modify: `backend/src/server.rs`
- Modify: `backend/src/runtime.rs`
- Create: `backend/tests/server.rs`

- [ ] **Step 1: Write server tests**

Use this `backend/tests/server.rs`:

```rust
use alphapulse_okx_backend::{
    config::AppConfig,
    server::build_router,
    state::RadarState,
};
use axum::{body::Body, http::{Request, StatusCode}};
use tower::ServiceExt;

#[tokio::test]
async fn health_route_returns_ok() {
    let router = build_router(AppConfig::default(), RadarState::default());

    let response = router
        .oneshot(Request::builder().uri("/api/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn snapshot_route_returns_empty_symbol_list_initially() {
    let router = build_router(AppConfig::default(), RadarState::default());

    let response = router
        .oneshot(Request::builder().uri("/api/snapshot").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
```

- [ ] **Step 2: Run server tests to verify they fail**

Run:

```bash
cargo test -p alphapulse_okx_backend --test server
```

Expected: FAIL because `RadarState` and `build_router` are not implemented.

- [ ] **Step 3: Implement state**

Use this contract in `backend/src/state.rs`:

```rust
use std::{collections::BTreeMap, sync::Arc};

use serde::Serialize;
use tokio::sync::{broadcast, RwLock};

use crate::domain::SymbolSnapshot;

#[derive(Debug, Clone, Serialize)]
pub struct DashboardSnapshot {
    pub symbols: Vec<SymbolSnapshot>,
    pub last_scan_at_ms: Option<i64>,
    pub websocket_connected: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BackendEvent {
    Snapshot { data: DashboardSnapshot },
    SymbolUpdated { data: SymbolSnapshot },
}

#[derive(Clone)]
pub struct RadarState {
    inner: Arc<RwLock<RadarStateInner>>,
    events: broadcast::Sender<BackendEvent>,
}

#[derive(Debug, Default)]
struct RadarStateInner {
    symbols: BTreeMap<String, SymbolSnapshot>,
    last_scan_at_ms: Option<i64>,
    websocket_connected: bool,
}

impl Default for RadarState {
    fn default() -> Self {
        let (events, _) = broadcast::channel(256);
        Self {
            inner: Arc::new(RwLock::new(RadarStateInner::default())),
            events,
        }
    }
}

impl RadarState {
    pub async fn snapshot(&self) -> DashboardSnapshot {
        let inner = self.inner.read().await;
        DashboardSnapshot {
            symbols: inner.symbols.values().cloned().collect(),
            last_scan_at_ms: inner.last_scan_at_ms,
            websocket_connected: inner.websocket_connected,
        }
    }

    pub async fn upsert_symbol(&self, symbol: SymbolSnapshot) {
        {
            let mut inner = self.inner.write().await;
            inner.symbols.insert(symbol.inst_id.clone(), symbol.clone());
        }
        let _ = self.events.send(BackendEvent::SymbolUpdated { data: symbol });
    }

    pub async fn mark_scan(&self, ts_ms: i64) {
        let mut inner = self.inner.write().await;
        inner.last_scan_at_ms = Some(ts_ms);
    }

    pub async fn set_websocket_connected(&self, connected: bool) {
        let mut inner = self.inner.write().await;
        inner.websocket_connected = connected;
    }

    pub fn subscribe(&self) -> broadcast::Receiver<BackendEvent> {
        self.events.subscribe()
    }
}
```

- [ ] **Step 4: Implement server router**

Use this contract in `backend/src/server.rs`:

```rust
use std::net::SocketAddr;

use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, State},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use futures_util::SinkExt;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{config::AppConfig, state::{BackendEvent, RadarState}};

#[derive(Clone)]
struct AppCtx {
    config: AppConfig,
    state: RadarState,
}

pub fn build_router(config: AppConfig, state: RadarState) -> Router {
    let ctx = AppCtx { config, state };
    Router::new()
        .route("/api/health", get(health))
        .route("/api/snapshot", get(snapshot))
        .route("/ws", get(ws_handler))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(ctx)
}

pub async fn serve(config: AppConfig) -> anyhow::Result<()> {
    let state = RadarState::default();
    let app = build_router(config.clone(), state);
    let addr: SocketAddr = format!("{}:{}", config.host, config.port).parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("backend listening on http://{}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health() -> &'static str {
    "ok"
}

async fn snapshot(State(ctx): State<AppCtx>) -> impl IntoResponse {
    Json(ctx.state.snapshot().await)
}

async fn ws_handler(State(ctx): State<AppCtx>, ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(move |socket| ws_loop(socket, ctx.state))
}

async fn ws_loop(mut socket: WebSocket, state: RadarState) {
    let snapshot = state.snapshot().await;
    let _ = socket
        .send(Message::Text(serde_json::to_string(&BackendEvent::Snapshot { data: snapshot }).unwrap()))
        .await;

    let mut rx = state.subscribe();
    while let Ok(event) = rx.recv().await {
        if socket
            .send(Message::Text(serde_json::to_string(&event).unwrap()))
            .await
            .is_err()
        {
            break;
        }
    }
}
```

- [ ] **Step 5: Keep runtime as an explicit empty boundary**

Use this `backend/src/runtime.rs`:

```rust
use crate::{config::AppConfig, state::RadarState};

pub async fn run_scanner(_config: AppConfig, _state: RadarState) -> anyhow::Result<()> {
    Ok(())
}
```

- [ ] **Step 6: Run tests**

Run:

```bash
cargo test -p alphapulse_okx_backend --test server
cargo test -p alphapulse_okx_backend
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add backend/src/state.rs backend/src/server.rs backend/src/runtime.rs backend/tests/server.rs
git commit -m "feat: expose backend snapshot and websocket API"
```

## Task 9: Frontend Scaffold And Data Client

**Files:**
- Create: `frontend/package.json`
- Create: `frontend/index.html`
- Create: `frontend/vite.config.ts`
- Create: `frontend/tsconfig.json`
- Create: `frontend/tsconfig.node.json`
- Create: `frontend/src/main.tsx`
- Create: `frontend/src/types.ts`
- Create: `frontend/src/api.ts`
- Create: `frontend/src/App.tsx`
- Create: `frontend/src/styles.css`
- Create: `frontend/src/App.test.tsx`

- [ ] **Step 1: Create frontend package files**

Use this `frontend/package.json`:

```json
{
  "name": "alphapulse-okx-frontend",
  "version": "0.1.0",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "vite --host 127.0.0.1 --port 5173",
    "build": "tsc -b && vite build",
    "test": "vitest run",
    "lint": "tsc -b --noEmit"
  },
  "dependencies": {
    "react": "^18.3.1",
    "react-dom": "^18.3.1"
  },
  "devDependencies": {
    "@vitejs/plugin-react": "^4.3.0",
    "@testing-library/jest-dom": "^6.4.8",
    "@testing-library/react": "^15.0.7",
    "@types/react": "^18.3.3",
    "@types/react-dom": "^18.3.0",
    "jsdom": "^24.1.1",
    "typescript": "^5.5.0",
    "vite": "^5.4.0",
    "vitest": "^2.0.5"
  }
}
```

Use standard Vite React TypeScript config files:

```ts
// frontend/vite.config.ts
/// <reference types="vitest" />
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [react()],
  server: {
    host: "127.0.0.1",
    port: 5173,
  },
  test: {
    environment: "jsdom",
    globals: true,
  },
});
```

- [ ] **Step 2: Write frontend types and API client**

Use `frontend/src/types.ts`:

```ts
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
  updated_at_ms: number;
}

export interface DashboardSnapshot {
  symbols: SymbolSnapshot[];
  last_scan_at_ms: number | null;
  websocket_connected: boolean;
}

export type BackendEvent =
  | { type: "snapshot"; data: DashboardSnapshot }
  | { type: "symbol_updated"; data: SymbolSnapshot };
```

Use `frontend/src/api.ts`:

```ts
import type { BackendEvent, DashboardSnapshot } from "./types";

export async function fetchSnapshot(): Promise<DashboardSnapshot> {
  const response = await fetch("http://127.0.0.1:8787/api/snapshot");
  if (!response.ok) {
    throw new Error(`snapshot request failed: ${response.status}`);
  }
  return response.json();
}

export function connectEvents(onEvent: (event: BackendEvent) => void): WebSocket {
  const socket = new WebSocket("ws://127.0.0.1:8787/ws");
  socket.addEventListener("message", (message) => {
    onEvent(JSON.parse(String(message.data)) as BackendEvent);
  });
  return socket;
}
```

- [ ] **Step 3: Write basic app and test**

Use `frontend/src/App.test.tsx`:

```tsx
import "@testing-library/jest-dom/vitest";
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import App from "./App";

describe("App", () => {
  it("renders the radar title and connection status", () => {
    render(<App />);
    expect(screen.getByText("AlphaPulse OKX")).toBeInTheDocument();
    expect(screen.getByText("Backend")).toBeInTheDocument();
  });
});
```

Implement `frontend/src/App.tsx` with the title, header status, and empty table state.

- [ ] **Step 4: Run frontend tests**

Run:

```bash
cd frontend
npm install
npm test
npm run build
```

Expected: tests and build PASS.

- [ ] **Step 5: Commit**

```bash
git add frontend
git commit -m "feat: scaffold TypeScript dashboard"
```

## Task 10: Dashboard Table, Filters, And Browser Notifications

**Files:**
- Modify: `frontend/src/App.tsx`
- Modify: `frontend/src/notifications.ts`
- Modify: `frontend/src/styles.css`
- Modify: `frontend/src/App.test.tsx`
- Create: `frontend/src/notifications.test.ts`

- [ ] **Step 1: Write notification state tests**

Use `frontend/src/notifications.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { shouldNotify } from "./notifications";
import type { SymbolSnapshot } from "./types";

function symbol(value: number, direction: "long" | "short" | "neutral"): SymbolSnapshot {
  return {
    inst_id: "LAB-USDT-SWAP",
    price: 17.2,
    change_5m_pct: -0.03,
    change_15m_pct: -0.07,
    change_1h_pct: -0.11,
    trend_score: { value, direction, reasons: ["volume 3.1x"] },
    range_score: { value: 20, direction: "neutral", reasons: [] },
    pool_tags: ["dynamic"],
    trigger_reason: "trend short 84: volume 3.1x",
    funding_rate: -0.003,
    updated_at_ms: 1782400000000,
  };
}

describe("shouldNotify", () => {
  it("notifies when a symbol newly enters high trend score", () => {
    const seen = new Map<string, string>();
    expect(shouldNotify(symbol(84, "short"), seen, 80)).toBe(true);
    expect(shouldNotify(symbol(84, "short"), seen, 80)).toBe(false);
  });

  it("notifies again when direction changes", () => {
    const seen = new Map<string, string>();
    expect(shouldNotify(symbol(84, "short"), seen, 80)).toBe(true);
    expect(shouldNotify(symbol(86, "long"), seen, 80)).toBe(true);
  });
});
```

- [ ] **Step 2: Implement browser notification helper**

Use `frontend/src/notifications.ts`:

```ts
import type { SymbolSnapshot } from "./types";

export function shouldNotify(
  symbol: SymbolSnapshot,
  seen: Map<string, string>,
  threshold: number,
): boolean {
  const score = symbol.trend_score.value >= symbol.range_score.value ? symbol.trend_score : symbol.range_score;
  if (score.value < threshold || score.direction === "neutral") {
    return false;
  }

  const key = `${symbol.inst_id}:${score.direction}`;
  const value = `${Math.floor(score.value / 10)}:${symbol.trigger_reason}`;
  if (seen.get(key) === value) {
    return false;
  }

  seen.set(key, value);
  return true;
}

export function sendBrowserNotification(symbol: SymbolSnapshot): void {
  if (!("Notification" in window) || Notification.permission !== "granted") {
    return;
  }

  const score = symbol.trend_score.value >= symbol.range_score.value ? symbol.trend_score : symbol.range_score;
  new Notification(`${symbol.inst_id} ${score.direction} ${score.value}`, {
    body: symbol.trigger_reason,
  });
}
```

- [ ] **Step 3: Implement table and filters**

In `frontend/src/App.tsx`, add:

- Header status for backend, stream, latest scan time, and notification permission.
- Filter buttons: All, Trend, Range, Hot, Fixed.
- Symbol table sorted by the max of trend score and range score.
- Detail panel for selected symbol.
- WebSocket event handling through `connectEvents`.
- Notification permission request button.

- [ ] **Step 4: Run tests and build**

Run:

```bash
cd frontend
npm test
npm run build
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add frontend/src
git commit -m "feat: add dashboard filters and browser notifications"
```

## Task 11: Runtime Scanner MVP

**Files:**
- Modify: `backend/src/runtime.rs`
- Modify: `backend/src/server.rs`
- Modify: `backend/src/main.rs`
- Modify: `backend/src/okx/rest.rs`
- Modify: `backend/src/okx/ws.rs`

- [ ] **Step 1: Wire runtime into server startup**

Change `server::serve` to create `RadarState`, spawn `runtime::run_scanner(config.clone(), state.clone())`, then start axum. Keep `run_scanner` non-fatal: log errors and keep HTTP serving if the scanner exits.

- [ ] **Step 2: Implement slow scan loop**

In `runtime.rs`, every `config.scan_interval_secs`:

- Fetch OKX tickers through `OkxRestClient`.
- Fetch 5m, 15m, and 1h candles for each symbol in the merged universe with a bounded limit of 120 candles per timeframe.
- Build `MarketActivity` rows from ticker data plus candle-derived 15m movement, 1h movement, volatility, and volume ratio.
- Build the merged symbol universe.
- For each symbol, detect simplified FVG zones, cluster support/resistance, build `ScoringInput`, calculate trend/range scores, and create a `SymbolSnapshot`.
- Store each snapshot through `RadarState::upsert_symbol`.
- Call `RadarState::mark_scan`.

- [ ] **Step 3: Implement fast ticker stream**

In `okx/ws.rs`, add a function that connects to `wss://ws.okx.com:8443/ws/v5/public`, subscribes to the current symbol universe tickers, parses ticker events, and sends them to `runtime.rs` over a Tokio channel. In `runtime.rs`, apply ticker events to existing snapshots and call `RadarState::upsert_symbol` immediately.

- [ ] **Step 4: Run backend checks**

Run:

```bash
cargo test -p alphapulse_okx_backend
cargo check -p alphapulse_okx_backend
```

Expected: PASS.

- [ ] **Step 5: Manually verify public API connectivity**

Run:

```bash
cargo run -p alphapulse_okx_backend
```

In another terminal:

```bash
curl -s http://127.0.0.1:8787/api/health
curl -s http://127.0.0.1:8787/api/snapshot
```

Expected:

- `/api/health` returns `ok`.
- `/api/snapshot` returns JSON with a `symbols` array after the first scan.
- Backend logs show OKX scan attempts and WebSocket connection state.

- [ ] **Step 6: Commit**

```bash
git add backend/src
git commit -m "feat: run OKX scanner and ticker stream"
```

## Task 12: End-To-End Local Verification And Docs

**Files:**
- Modify: `README.md`
- Modify: `.gitignore`

- [ ] **Step 1: Update `.gitignore` for Rust and frontend artifacts**

Add:

```gitignore
# Rust
target/

# Frontend
node_modules/
frontend/dist/
frontend/.vite/
```

Keep `.env`, `.env.local`, `logs/`, `data/`, and `reports/` ignored.

- [ ] **Step 2: Update README quickstart**

Document:

```markdown
## Local Development

Backend:

```bash
cargo run -p alphapulse_okx_backend
```

Frontend:

```bash
cd frontend
npm install
npm run dev
```

Open `http://127.0.0.1:5173`.

Version 1 uses OKX public market data only. It does not require an API key and cannot place orders.
```

- [ ] **Step 3: Run full verification**

Run:

```bash
cargo test -p alphapulse_okx_backend
cargo check -p alphapulse_okx_backend
cd frontend
npm test
npm run build
```

Expected: all commands PASS.

- [ ] **Step 4: Run the app locally**

Run backend:

```bash
cargo run -p alphapulse_okx_backend
```

Run frontend:

```bash
cd frontend
npm run dev
```

Open `http://127.0.0.1:5173` and verify:

- Header shows backend connection state.
- Dashboard loads snapshot data.
- Symbol rows update after backend events.
- Notification permission control is visible.
- Account area shows no read-only API key connected.

- [ ] **Step 5: Commit**

```bash
git add README.md .gitignore
git commit -m "docs: add local development instructions"
```

## Self-Review Checklist

- Spec coverage:
  - OKX public market scanning: Tasks 7 and 11.
  - Dynamic and fixed symbol pools: Task 5 and Task 11.
  - Trend/range dual scoring: Task 6.
  - Simplified FVG detection: Task 3.
  - Support/resistance zones: Task 4.
  - Browser notification deduplication: Task 6 and Task 10.
  - Rust backend and TypeScript frontend: Tasks 1, 2, 8, 9, 10, 11.
  - No account API key and no trading actions: preserved in Task 12 README and no order endpoints in Task 8.
- Placeholder scan:
  - No task relies on unspecified files.
  - Each command includes expected results.
  - Each commit point lists exact files.
- Type consistency:
  - Backend JSON uses snake_case field names from serde.
  - Frontend TypeScript types match backend serialized names.
  - Alert and scoring directions use `long`, `short`, and `neutral`.
