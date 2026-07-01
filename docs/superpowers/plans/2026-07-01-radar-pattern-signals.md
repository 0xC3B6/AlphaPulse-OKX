# Radar Pattern Signals Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add W bottom, M top, and sweep-failure pattern signals to the Radar pipeline, including backend detection, scoring, API fields, and frontend display/overlays.

**Architecture:** Add a focused `backend/src/indicators/patterns.rs` detector that turns `m15` and `h1` candles into explainable `PatternSignal` objects. Runtime and chart endpoints expose those signals; scoring consumes only active confirmed/retest/holding signals as capped boosts. Frontend renders compact pattern badges in Radar, detailed pattern metadata in the symbol panel, and optional overlays on the chart.

**Tech Stack:** Rust backend with serde/axum/tokio, React + TypeScript frontend, Vitest, Cargo tests, lightweight-charts overlay DOM elements.

---

## File Structure

- Create `backend/src/indicators/patterns.rs`: pivot extraction, W/M/sweep detection, status classification, scoring, unit tests.
- Modify `backend/src/indicators/mod.rs`: export `patterns`.
- Modify `backend/src/domain.rs`: add `PatternKind`, `PatternStatus`, `PatternPivotRole`, `PatternPivot`, `PatternLevelZone`, `PatternSignal`; add `pattern_signals` to `SymbolSnapshot` and `ChartSnapshot`.
- Modify `backend/src/runtime.rs`: detect `m15` and `h1` patterns and pass them into scoring/snapshots.
- Modify `backend/src/server.rs`: detect chart-timeframe patterns and return them in `ChartSnapshot`.
- Modify `backend/src/scoring.rs`: accept active pattern signals and add capped directional boosts.
- Modify `backend/src/paper.rs`: update test fixture snapshots with empty `pattern_signals`.
- Modify `frontend/src/types.ts`: mirror pattern types and add `pattern_signals`.
- Modify `frontend/src/App.test.tsx`, `frontend/src/notifications.test.ts`, `frontend/src/uiFormat.test.ts`: update fixtures and add pattern UI coverage.
- Modify `frontend/src/i18n.ts`: add pattern labels and chart toggle copy.
- Modify `frontend/src/RadarTable.tsx`: add compact Pattern column.
- Modify `frontend/src/SymbolDetailPanel.tsx`: add Patterns section.
- Modify `frontend/src/ChartPanel.tsx`: fetch/render pattern overlays with a Show Patterns checkbox.
- Modify `frontend/src/styles.css`: add table/detail/chart overlay styling.

---

### Task 1: Backend Domain Types and Pattern Detector

**Files:**
- Create: `backend/src/indicators/patterns.rs`
- Modify: `backend/src/indicators/mod.rs`
- Modify: `backend/src/domain.rs`

- [ ] **Step 1: Write failing detector tests**

Add unit tests at the bottom of `backend/src/indicators/patterns.rs` before implementation. The tests should construct deterministic candles and assert the expected pattern kind/status/direction.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Candle, Direction, PatternKind, PatternStatus, Timeframe};

    fn candle(index: i64, open: f64, high: f64, low: f64, close: f64) -> Candle {
        Candle {
            ts_ms: index * 60_000,
            open,
            high,
            low,
            close,
            volume: 100.0 + index as f64,
        }
    }

    #[test]
    fn detects_double_bottom_retest_holding() {
        let candles = vec![
            candle(0, 101.0, 102.0, 100.0, 101.0),
            candle(1, 101.0, 102.0, 96.0, 97.0),
            candle(2, 97.0, 104.0, 97.0, 103.0),
            candle(3, 103.0, 104.0, 98.0, 99.0),
            candle(4, 99.0, 106.0, 98.0, 105.0),
            candle(5, 105.0, 106.0, 102.7, 103.4),
            candle(6, 103.4, 107.0, 103.2, 106.2),
        ];

        let signals = detect_patterns(&candles, Timeframe::M15, 106.2);
        let signal = signals
            .iter()
            .find(|signal| signal.kind == PatternKind::DoubleBottom)
            .expect("double bottom should be detected");
        assert_eq!(signal.direction, Direction::Long);
        assert_eq!(signal.status, PatternStatus::Holding);
        assert_eq!(signal.neckline, Some(104.0));
        assert!(signal.score >= 60);
        assert!(signal.reasons.iter().any(|reason| reason.contains("neckline retest holding")));
    }

    #[test]
    fn detects_double_top_retest_holding() {
        let candles = vec![
            candle(0, 99.0, 100.0, 98.0, 99.0),
            candle(1, 99.0, 106.0, 98.0, 105.0),
            candle(2, 105.0, 105.0, 96.0, 97.0),
            candle(3, 97.0, 104.0, 96.5, 103.0),
            candle(4, 103.0, 103.5, 95.0, 96.0),
            candle(5, 96.0, 97.4, 94.5, 97.0),
            candle(6, 97.0, 97.2, 93.0, 94.0),
        ];

        let signals = detect_patterns(&candles, Timeframe::H1, 94.0);
        let signal = signals
            .iter()
            .find(|signal| signal.kind == PatternKind::DoubleTop)
            .expect("double top should be detected");
        assert_eq!(signal.direction, Direction::Short);
        assert_eq!(signal.status, PatternStatus::Holding);
        assert_eq!(signal.neckline, Some(96.0));
        assert!(signal.score >= 60);
    }

    #[test]
    fn detects_failed_high_sweep() {
        let candles = vec![
            candle(0, 100.0, 105.0, 99.0, 104.0),
            candle(1, 104.0, 104.5, 100.0, 101.0),
            candle(2, 101.0, 106.5, 100.5, 102.0),
            candle(3, 102.0, 103.0, 98.5, 99.0),
        ];

        let signals = detect_patterns(&candles, Timeframe::M15, 99.0);
        let signal = signals
            .iter()
            .find(|signal| signal.kind == PatternKind::SweepFailure)
            .expect("failed high sweep should be detected");
        assert_eq!(signal.direction, Direction::Short);
        assert_eq!(signal.status, PatternStatus::Holding);
        assert!(signal.score >= 55);
    }
}
```

- [ ] **Step 2: Run detector tests and verify they fail**

Run:

```bash
cargo test -p alphapulse_okx_backend indicators::patterns
```

Expected: compile failure because `patterns.rs`, `PatternKind`, `PatternStatus`, and `detect_patterns` do not exist yet.

- [ ] **Step 3: Add domain types**

Add these types to `backend/src/domain.rs` after `LevelZone`, and add `pattern_signals: Vec<PatternSignal>` to both `SymbolSnapshot` and `ChartSnapshot`.

```rust
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
    pub neckline: Option<f64>,
    pub invalidation_level: Option<f64>,
    pub start_ts_ms: i64,
    pub confirm_ts_ms: Option<i64>,
    pub pivots: Vec<PatternPivot>,
    pub level_zone: Option<PatternLevelZone>,
    pub reasons: Vec<String>,
}
```

- [ ] **Step 4: Implement `detect_patterns`**

Create `backend/src/indicators/patterns.rs` with:

```rust
use crate::domain::{
    Candle, Direction, PatternKind, PatternLevelZone, PatternPivot, PatternPivotRole,
    PatternSignal, PatternStatus, Timeframe,
};

pub fn detect_patterns(candles: &[Candle], timeframe: Timeframe, current_price: f64) -> Vec<PatternSignal> {
    if candles.len() < 6 || current_price <= 0.0 {
        return Vec::new();
    }

    let tolerance = pattern_tolerance(candles, current_price);
    let mut signals = Vec::new();
    if let Some(signal) = detect_double_bottom(candles, timeframe, tolerance) {
        signals.push(signal);
    }
    if let Some(signal) = detect_double_top(candles, timeframe, tolerance) {
        signals.push(signal);
    }
    if let Some(signal) = detect_sweep_failure(candles, timeframe, tolerance) {
        signals.push(signal);
    }

    signals.retain(|signal| signal.status != PatternStatus::Invalidated);
    signals.sort_by(|left, right| right.score.cmp(&left.score));
    signals.truncate(4);
    signals
}
```

The helper functions should:

- Use recent candles only, with a cap of the last 80 bars.
- Compute tolerance as `max(0.003, min(0.025, recent_range_pct * 0.18))`.
- Use simple pivot candidates from the latest visible swings.
- Return explainable `reasons`, including `"neckline retest holding"` for the W-bottom holding test.

- [ ] **Step 5: Export module and run tests**

Add to `backend/src/indicators/mod.rs`:

```rust
pub mod patterns;
```

Run:

```bash
cargo test -p alphapulse_okx_backend indicators::patterns
```

Expected: all pattern detector tests pass.

---

### Task 2: Backend Runtime and Chart API Integration

**Files:**
- Modify: `backend/src/runtime.rs`
- Modify: `backend/src/server.rs`
- Modify: `backend/src/paper.rs`

- [ ] **Step 1: Write failing API/runtime coverage**

Add backend tests where existing backend tests are located or at the bottom of the touched modules:

```rust
#[test]
fn symbol_snapshot_can_serialize_pattern_signals() {
    let snapshot = SymbolSnapshot {
        inst_id: "ETH-USDT-SWAP".to_string(),
        price: 100.0,
        change_5m_pct: 0.0,
        change_15m_pct: 0.0,
        change_1h_pct: 0.0,
        trend_score: Score { value: 0, direction: Direction::Neutral, reasons: vec![] },
        range_score: Score { value: 0, direction: Direction::Neutral, reasons: vec![] },
        pool_tags: vec![],
        trigger_reason: "none".to_string(),
        funding_rate: None,
        fvgs: vec![],
        levels: vec![],
        pattern_signals: vec![PatternSignal {
            kind: PatternKind::DoubleBottom,
            direction: Direction::Long,
            timeframe: Timeframe::M15,
            status: PatternStatus::Holding,
            score: 72,
            neckline: Some(104.0),
            invalidation_level: Some(98.0),
            start_ts_ms: 0,
            confirm_ts_ms: Some(4 * 60_000),
            pivots: vec![],
            level_zone: Some(PatternLevelZone { lower: 103.5, upper: 104.5 }),
            reasons: vec!["neckline retest holding".to_string()],
        }],
        updated_at_ms: 0,
    };

    let json = serde_json::to_string(&snapshot).unwrap();
    assert!(json.contains("pattern_signals"));
    assert!(json.contains("double_bottom"));
    assert!(json.contains("holding"));
}
```

- [ ] **Step 2: Run backend tests and verify missing fields fail**

Run:

```bash
cargo test -p alphapulse_okx_backend
```

Expected: failures in fixtures that still omit `pattern_signals`.

- [ ] **Step 3: Wire runtime detection**

In `backend/src/runtime.rs`, import `detect_patterns`, detect from `m15` and `h1`, and pass signals into `SymbolSnapshot`:

```rust
let mut pattern_signals = Vec::new();
pattern_signals.extend(detect_patterns(&candles_15m, Timeframe::M15, price));
pattern_signals.extend(detect_patterns(&candles_1h, Timeframe::H1, price));
pattern_signals.sort_by(|left, right| right.score.cmp(&left.score));
pattern_signals.truncate(6);
```

- [ ] **Step 4: Wire chart endpoint detection**

In `backend/src/server.rs`, detect selected timeframe patterns:

```rust
let pattern_signals = detect_patterns(&candles, timeframe, current_price);
```

Return `pattern_signals` in `ChartSnapshot`.

- [ ] **Step 5: Update all backend fixtures**

Any test fixture creating `SymbolSnapshot` or `ChartSnapshot` must add:

```rust
pattern_signals: Vec::new(),
```

- [ ] **Step 6: Run backend tests**

Run:

```bash
cargo test -p alphapulse_okx_backend
```

Expected: all backend tests pass.

---

### Task 3: Scoring Integration

**Files:**
- Modify: `backend/src/scoring.rs`
- Modify: `backend/src/runtime.rs`

- [ ] **Step 1: Write failing scoring tests**

Add to `backend/src/scoring.rs` tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        Direction, PatternKind, PatternSignal, PatternStatus, Timeframe,
    };

    fn pattern(kind: PatternKind, direction: Direction, status: PatternStatus, score: u8) -> PatternSignal {
        PatternSignal {
            kind,
            direction,
            timeframe: Timeframe::M15,
            status,
            score,
            neckline: Some(104.0),
            invalidation_level: Some(98.0),
            start_ts_ms: 0,
            confirm_ts_ms: Some(1),
            pivots: vec![],
            level_zone: None,
            reasons: vec!["test pattern".to_string()],
        }
    }

    fn base_input(pattern_signals: Vec<PatternSignal>) -> ScoringInput {
        ScoringInput {
            inst_id: "ETH-USDT-SWAP".to_string(),
            change_5m_pct: 0.0,
            change_15m_pct: 0.0,
            change_1h_pct: 0.0,
            broke_recent_high: false,
            broke_recent_low: false,
            volume_ratio: 1.0,
            nearest_fvg_distance_pct: None,
            dynamic_pool: false,
            near_support: false,
            near_resistance: false,
            clear_range: false,
            funding_rate: None,
            pattern_signals,
        }
    }

    #[test]
    fn w_bottom_holding_adds_long_pattern_boost() {
        let scored = score_symbol(base_input(vec![pattern(
            PatternKind::DoubleBottom,
            Direction::Long,
            PatternStatus::Holding,
            72,
        )]));

        assert_eq!(scored.trend_score.direction, Direction::Long);
        assert!(scored.trend_score.value >= 15);
        assert!(scored.trend_score.reasons.iter().any(|reason| reason.contains("pattern")));
    }

    #[test]
    fn forming_pattern_does_not_boost_score() {
        let scored = score_symbol(base_input(vec![pattern(
            PatternKind::DoubleBottom,
            Direction::Long,
            PatternStatus::Forming,
            80,
        )]));

        assert_eq!(scored.trend_score.direction, Direction::Neutral);
        assert_eq!(scored.trend_score.value, 0);
    }
}
```

- [ ] **Step 2: Run scoring tests and verify failure**

Run:

```bash
cargo test -p alphapulse_okx_backend scoring
```

Expected: compile failure because `ScoringInput` does not contain `pattern_signals`.

- [ ] **Step 3: Add pattern boost logic**

Modify `ScoringInput`:

```rust
pub pattern_signals: Vec<PatternSignal>,
```

Add helper:

```rust
fn best_pattern_boost(input: &ScoringInput) -> Option<(Direction, u8, String)> {
    input
        .pattern_signals
        .iter()
        .filter(|signal| matches!(
            signal.status,
            PatternStatus::Confirmed | PatternStatus::Retest | PatternStatus::Holding
        ))
        .filter_map(|signal| {
            let base = match signal.status {
                PatternStatus::Retest => 22,
                PatternStatus::Holding => 18,
                PatternStatus::Confirmed => 12,
                PatternStatus::Forming | PatternStatus::Invalidated => 0,
            };
            if base == 0 {
                return None;
            }
            let scaled = ((signal.score as f64 / 100.0) * base as f64).round() as u8;
            Some((
                signal.direction,
                scaled.max(8),
                format!(
                    "pattern {:?} {:?} {}",
                    signal.kind, signal.status, signal.score
                ),
            ))
        })
        .max_by_key(|(_, boost, _)| *boost)
}
```

Apply the boost in `trend_score`. If current direction is neutral, set it to the pattern direction. If current direction conflicts with the pattern, add no boost and add no reason.

- [ ] **Step 4: Pass pattern signals from runtime**

In `backend/src/runtime.rs`, pass `pattern_signals.clone()` into `ScoringInput`.

- [ ] **Step 5: Run scoring and backend tests**

Run:

```bash
cargo test -p alphapulse_okx_backend scoring
cargo test -p alphapulse_okx_backend
```

Expected: all backend tests pass.

---

### Task 4: Frontend Types, Radar Table, Detail Panel, and Chart Overlays

**Files:**
- Modify: `frontend/src/types.ts`
- Modify: `frontend/src/i18n.ts`
- Modify: `frontend/src/RadarTable.tsx`
- Modify: `frontend/src/SymbolDetailPanel.tsx`
- Modify: `frontend/src/ChartPanel.tsx`
- Modify: `frontend/src/App.test.tsx`
- Modify: `frontend/src/notifications.test.ts`
- Modify: `frontend/src/uiFormat.test.ts`
- Modify: `frontend/src/styles.css`

- [ ] **Step 1: Write failing frontend tests**

In `frontend/src/App.test.tsx`, add a pattern signal to the LAB fixture and assert the Radar table/detail/chart display it:

```ts
pattern_signals: [
  {
    kind: "double_bottom",
    direction: "long",
    timeframe: "m15",
    status: "holding",
    score: 72,
    neckline: 16.8,
    invalidation_level: 16.2,
    start_ts_ms: 1782398800000,
    confirm_ts_ms: 1782399700000,
    pivots: [
      { role: "left_low", ts_ms: 1782398800000, price: 16.1 },
      { role: "neckline", ts_ms: 1782399100000, price: 16.8 },
      { role: "right_low", ts_ms: 1782399400000, price: 16.2 },
    ],
    level_zone: { lower: 16.72, upper: 16.88 },
    reasons: ["neckline retest holding"],
  },
],
```

Add expectations:

```ts
expect(screen.getByRole("columnheader", { name: "形态" })).toBeInTheDocument();
expect(screen.getAllByText("W holding 72").length).toBeGreaterThan(0);
expect(screen.getByTestId("symbol-detail-panel")).toHaveTextContent("Patterns");
expect(screen.getByTestId("symbol-detail-panel")).toHaveTextContent("neckline retest holding");
expect(screen.getByLabelText("显示形态")).toBeChecked();
```

- [ ] **Step 2: Run frontend test and verify failure**

Run:

```bash
npm test -- src/App.test.tsx -t "pattern"
```

Expected: failure because pattern types and UI do not exist.

- [ ] **Step 3: Add frontend pattern types**

Add to `frontend/src/types.ts`:

```ts
export type PatternKind = "double_bottom" | "double_top" | "sweep_failure";
export type PatternStatus = "forming" | "confirmed" | "retest" | "holding" | "invalidated";
export type PatternPivotRole =
  | "left_low"
  | "right_low"
  | "left_high"
  | "right_high"
  | "neckline"
  | "sweep_reference"
  | "sweep";

export interface PatternPivot {
  role: PatternPivotRole;
  ts_ms: number;
  price: number;
}

export interface PatternLevelZone {
  lower: number;
  upper: number;
}

export interface PatternSignal {
  kind: PatternKind;
  direction: Direction;
  timeframe: "m15" | "h1";
  status: PatternStatus;
  score: number;
  neckline: number | null;
  invalidation_level: number | null;
  start_ts_ms: number;
  confirm_ts_ms: number | null;
  pivots: PatternPivot[];
  level_zone: PatternLevelZone | null;
  reasons: string[];
}
```

Add `pattern_signals: PatternSignal[]` to `SymbolSnapshot` and `ChartSnapshot`.

- [ ] **Step 4: Add labels and formatting helpers**

In `frontend/src/i18n.ts`, add Chinese and English labels:

```ts
patterns: "Patterns",
noPatterns: "暂无形态信号",
showPatterns: "显示形态",
patternColumn: "形态",
patternKinds: {
  double_bottom: "W",
  double_top: "M",
  sweep_failure: "Sweep",
},
patternStatuses: {
  forming: "forming",
  confirmed: "confirmed",
  retest: "retest",
  holding: "holding",
  invalidated: "invalidated",
},
neckline: "颈线",
invalidationLevel: "失效位",
```

- [ ] **Step 5: Render Radar table and detail section**

In `RadarTable.tsx`, add a `Pattern` column using the strongest signal:

```tsx
const pattern = symbol.pattern_signals[0];
const patternLabel = pattern ? `${formatPatternKind(pattern.kind, copy)} ${pattern.status} ${pattern.score}` : "-";
```

In `SymbolDetailPanel.tsx`, add a section:

```tsx
<section>
  <h3>{copy.detail.patterns}</h3>
  {symbol.pattern_signals.length === 0 ? (
    <p className="muted">{copy.detail.noPatterns}</p>
  ) : (
    <ul className="detail-list">
      {symbol.pattern_signals.map((pattern) => (
        <li key={`${pattern.kind}-${pattern.timeframe}-${pattern.start_ts_ms}`}>
          <strong>{formatPatternKind(pattern.kind, copy)} {pattern.status} {pattern.score}</strong>
          <span>{pattern.timeframe} · neckline {formatPrice(pattern.neckline ?? 0)} · invalid {formatPrice(pattern.invalidation_level ?? 0)}</span>
          <em>{pattern.reasons.join(" / ")}</em>
        </li>
      ))}
    </ul>
  )}
</section>
```

- [ ] **Step 6: Render chart overlays**

In `ChartPanel.tsx`:

- Keep `showPatterns` state defaulting to true.
- Add checkbox label `copy.chart.showPatterns`.
- Convert `chartData.pattern_signals` into DOM overlay lines/boxes using existing chart coordinate logic.
- Draw neckline and invalidation level as horizontal lines.
- Draw level zone as a translucent band.

- [ ] **Step 7: Run frontend tests**

Run:

```bash
npm test -- src/App.test.tsx -t "pattern"
npm test
```

Expected: all frontend tests pass.

---

### Task 5: Full Verification and Tuning Pass

**Files:**
- Review all modified backend and frontend files.

- [ ] **Step 1: Run backend checks**

Run:

```bash
cargo test -p alphapulse_okx_backend
cargo build -p alphapulse_okx_backend
```

Expected: tests and build pass.

- [ ] **Step 2: Run frontend checks**

Run:

```bash
npm test
npm run build
```

Expected: tests and production build pass.

- [ ] **Step 3: Check API shape locally**

With backend running, query:

```bash
curl -s 'http://127.0.0.1:8787/api/snapshot' | jq '.symbols[0] | {inst_id, pattern_signals}'
curl -s 'http://127.0.0.1:8787/api/symbols/ETH-USDT-SWAP/chart?timeframe=m15' | jq '{timeframe, patterns: .pattern_signals}'
```

Expected: `pattern_signals` exists and is an array. It can be empty if no active pattern is found.

- [ ] **Step 4: Inspect UI manually**

Open `http://127.0.0.1:5173/` and verify:

- Radar table has a Pattern column.
- Symbol detail shows a Patterns section.
- Chart has a Show Patterns checkbox.
- FVG overlay still works when patterns are toggled.

- [ ] **Step 5: Final diff review**

Run:

```bash
git diff --check
git status --short
```

Expected: no whitespace errors; only intended files are modified.
