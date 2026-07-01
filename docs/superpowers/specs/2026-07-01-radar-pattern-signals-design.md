# Radar Pattern Signals Design

Date: 2026-07-01

## Purpose

Add short-term price-structure pattern detection to the Radar system. The feature should detect classic discretionary structures such as W bottoms, M tops, and failed breakout sweeps, then expose them as Radar signals and chart overlays.

This module belongs only to Radar. It must not be part of the Macro page or BTC cycle analysis. Macro can later gate or downgrade Radar notifications, but pattern detection itself is a short-term market-structure module.

The goal is not to automatically trade every pattern. The goal is to turn the user's discretionary pattern reading into explicit, inspectable structure:

1. A pattern is forming.
2. It confirms by breaking a neckline or failing a sweep.
3. It retests or rejects the important level.
4. It either holds or invalidates.

## Scope

Version 1 detects all monitored instruments without special cases for BTC, ETH, or altcoins.

Supported timeframes:

- `m15`
- `h1`

Supported pattern kinds:

- `double_bottom`: W bottom, long bias.
- `double_top`: M top, short bias.
- `sweep_failure`: failed high/low sweep, directional bias based on the swept side.

Supported pattern statuses:

- `forming`: structure exists, but no confirmation.
- `confirmed`: neckline break or sweep failure has confirmed.
- `retest`: price has returned to the key level after confirmation.
- `holding`: retest/rejection is holding.
- `invalidated`: invalidation level has been breached.

Only `confirmed`, `retest`, and `holding` can affect Radar scoring. `forming` is display-only. `invalidated` is hidden by default and can be exposed later through a debug option.

## Non-Goals

- Do not add the feature to Macro.
- Do not create real order execution rules.
- Do not require TradingView server-side data.
- Do not make pattern signals override liquidity, universe quality, market-cap filters, or Macro risk gates.
- Do not try to detect every chart pattern in the first version.

## Data Model

Add a new domain type to the backend and mirror it in the frontend:

```ts
type PatternKind = "double_bottom" | "double_top" | "sweep_failure";
type PatternStatus = "forming" | "confirmed" | "retest" | "holding" | "invalidated";

interface PatternSignal {
  kind: PatternKind;
  direction: "long" | "short";
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

interface PatternPivot {
  role: "left_low" | "right_low" | "left_high" | "right_high" | "neckline" | "sweep_reference" | "sweep";
  ts_ms: number;
  price: number;
}

interface PatternLevelZone {
  lower: number;
  upper: number;
}
```

Add `pattern_signals: Vec<PatternSignal>` to `SymbolSnapshot` and `ChartSnapshot`. Runtime detection can calculate patterns from the already fetched `m15` and `h1` candles. The chart endpoint should calculate patterns for the selected timeframe so overlays stay consistent with chart data.

## Detection Pipeline

Create `backend/src/indicators/patterns.rs`.

The detector pipeline should be deterministic:

1. Extract swing pivots from candles.
2. Detect candidate pattern geometry.
3. Classify status from the latest bars.
4. Score the candidate.
5. Keep only the highest-quality recent signals per timeframe.

Pivot extraction should use a local fractal rule. A pivot low is a candle whose low is lower than the surrounding `left/right` bars; a pivot high is symmetric. Version 1 can use `2` bars on each side for `m15` and `3` bars for `h1`. This is intentionally simple and testable.

Tolerance should be volatility-aware:

- Use ATR or recent high-low range as the base.
- Require a minimum tolerance so very small price noise does not invalidate a pattern.
- Use percentage caps so very wide, sloppy patterns do not pass.

## Double Bottom Rules

A valid W bottom has:

1. A left low.
2. A neckline pivot high after the left low.
3. A right low after the neckline.
4. The right low is close to the left low within tolerance.
5. The neckline is meaningfully above the two lows.

Status:

- `forming`: the two lows and neckline exist, but price has not closed above the neckline.
- `confirmed`: a later candle closes above the neckline with a small buffer.
- `retest`: after confirmation, price returns to a band around the neckline.
- `holding`: price retests the neckline band and closes back above it, or forms a higher low near it.
- `invalidated`: price closes below the right low or below the neckline band after a failed retest.

Score components:

- Geometry quality: lows are close, neckline height is meaningful.
- Confirmation quality: close above neckline, not just a wick.
- Retest quality: retest stays near the neckline and does not deeply lose it.
- Freshness: recent patterns score higher.
- Volume confirmation: optional boost if breakout/retest volume expands.

## Double Top Rules

Double top is the symmetric short-bias version:

1. A left high.
2. A neckline pivot low after the left high.
3. A right high after the neckline.
4. The right high is close to the left high within tolerance.
5. The neckline is meaningfully below the two highs.

Status:

- `forming`: two highs and neckline exist, but price has not closed below the neckline.
- `confirmed`: a later candle closes below the neckline with a small buffer.
- `retest`: after confirmation, price returns to the neckline band from below.
- `holding`: price rejects near the neckline or forms a lower high.
- `invalidated`: price closes above the right high or reclaims the neckline band after a failed retest.

This covers the user's M-top description where price reaches toward a previous high, fails to break or hold it, then follows through with bearish candles.

## Sweep Failure Rules

A failed high sweep has:

1. A recent reference high.
2. A later candle wicks above that high.
3. The candle closes back below the reference high.
4. The upper wick is large relative to the candle body or range.
5. Later candles fail to reclaim the swept high.

Direction is `short`.

A failed low sweep is symmetric:

1. A recent reference low.
2. A later candle wicks below that low.
3. The candle closes back above the reference low.
4. The lower wick is large relative to the candle body or range.
5. Later candles fail to lose the swept low.

Direction is `long`.

Status:

- `confirmed`: the sweep candle itself confirms the failure.
- `holding`: subsequent candles continue to reject/recover away from the swept level.
- `invalidated`: price closes beyond the swept extreme again.

## Radar Scoring Integration

Pattern signals should not replace existing `trend_score` and `range_score`.

Add pattern as an input into `scoring.rs`:

- `holding` W bottom: modest long boost.
- `retest` W bottom: strongest long boost.
- `confirmed` W bottom: smaller long boost until retest behavior is known.
- `holding` M top: modest short boost.
- `retest` M top: strongest short boost.
- `sweep_failure`: short or long boost depending on direction and freshness.
- `forming`: no scoring boost.
- `invalidated`: no scoring boost.

Pattern boost should be capped so bad liquidity, noisy universe quality, or Macro risk policy can still suppress notifications. The boost should show up in `Score.reasons` so the user can see why the Radar score changed.

## Frontend Design

Radar table:

- Add a compact `Pattern` column.
- Show the strongest active pattern, for example:
  - `W retest 72`
  - `M holding 68`
  - `Sweep fail 74`

Symbol detail panel:

- Add a `Patterns` section.
- Show kind, timeframe, status, score, neckline, invalidation level, and reasons.

Chart panel:

- Add a `Show Patterns` checkbox next to the FVG controls.
- Draw:
  - pivot markers for W/M lows/highs,
  - neckline,
  - retest or rejection zone,
  - invalidation level.

Pattern overlays should be visual aids only. They do not need to reproduce TradingView scripts exactly.

## API Behavior

`GET /api/dashboard` should include the latest pattern signals for each symbol.

`GET /api/chart/:inst_id?timeframe=m15|h1` should include pattern signals for that chart timeframe.

The endpoint should remain useful if pattern detection finds nothing. Empty arrays are valid.

## Edge Cases

- If candle history is too short, return no pattern signals.
- If pivots are too close together, reject the candidate.
- If price movement from lows/highs to neckline is too small, reject the candidate as noise.
- If a pattern is very old, downgrade or drop it.
- If multiple patterns overlap, keep the highest score per direction/timeframe.
- If current price is already far from the actionable level, keep the pattern visible but reduce score.

## Testing Plan

Backend unit tests:

- Detect a confirmed W bottom.
- Detect a W bottom neckline retest holding.
- Reject a W bottom when the second low is too far away.
- Detect a confirmed M top.
- Detect an M top rejection near the prior high.
- Detect failed high sweep and failed low sweep.
- Hide invalidated patterns by default.
- Confirm scoring reasons include pattern boosts.

Frontend tests:

- Radar table renders a pattern column.
- Symbol detail renders pattern reasons and levels.
- Chart panel toggles pattern overlays without affecting FVG overlays.

## Rollout

Implement in small steps:

1. Backend pattern types and detector tests.
2. Runtime and chart API integration.
3. Scoring integration with capped boosts.
4. Frontend table/detail/chart rendering.
5. Tune thresholds after observing real BTC, ETH, and altcoin examples.

The first implementation should prioritize explainability over precision. Every pattern signal must include enough `reasons` for the user to decide whether the structure matches their discretionary read.
