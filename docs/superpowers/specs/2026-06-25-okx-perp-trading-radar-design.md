# OKX Perpetual Trading Radar Design

Date: 2026-06-25

## Goal

Build a local Web application that scans OKX USDT perpetual markets and surfaces fast, explainable trading opportunities for a semi-discretionary crypto trading workflow.

The first version is a decision-support tool, not an auto-trading system. It should help identify which symbols deserve attention, why they are interesting, and when a symbol newly enters a high-score opportunity zone. Final trade decisions remain manual.

## Trading Context

The user's current approach is semi-discretionary, split into two styles:

- Short-term trend trades on major coins such as BTC, ETH, SOL, XRP, and DOGE. These use smaller account exposure, higher leverage, daily or multi-day holding periods, historical trend context, prior highs/lows, sharp daily drawdowns, ICT-style concepts, and FVG areas.
- Intraday scalps on active altcoins. These use faster entries and exits, higher account exposure, 5m/15m/1h structure, OKX hot symbols, short-term momentum, volume expansion, FVG, clear range support/resistance, and occasional funding-rate context.

The system should support both styles, but the first version is focused on scanning and alerting rather than execution.

## Product Scope

Version 1 includes:

- OKX USDT perpetual market scanning.
- A dynamic symbol pool built from market activity.
- A fixed watchlist for symbols the user cares about.
- Dual scoring for each symbol: trend score and range score.
- Simplified FVG detection on 5m, 15m, and 1h candles.
- Support/resistance estimation from recent candles.
- Volume expansion detection.
- Funding-rate display as a supporting signal.
- Real-time price updates through WebSocket.
- Browser desktop notifications when a symbol newly enters a high-score state.
- A read-only account module placeholder that says no API key is connected.

Version 1 excludes:

- Auto trading.
- Manual order buttons.
- API key configuration.
- Position management.
- Full historical backtesting.
- Full TradingView-style charting.
- Exact replication of OKX App hot-ranking behavior.

## Architecture

Use a local Web app with a frontend and a local backend.

The backend owns market data collection, normalization, caching, and signal calculation. The frontend receives normalized scan results and real-time updates, then displays rankings, signal details, and browser notifications.

Core components:

- OKX market data client.
- Symbol universe builder.
- Real-time stream service.
- Candle and indicator cache.
- Signal engine.
- Alert state tracker.
- HTTP/WebSocket API.
- Web dashboard.

## Technology Stack

Use Rust for the local backend and TypeScript for the frontend.

Backend:

- Rust async runtime: `tokio`.
- HTTP server and WebSocket API: `axum`.
- OKX REST client: `reqwest`.
- OKX WebSocket client: `tokio-tungstenite`.
- Serialization and typed DTOs: `serde`.
- Logging and diagnostics: `tracing`.
- Initial state storage: in-memory caches plus local configuration files.

Frontend:

- TypeScript.
- Vite.
- React.
- Browser Notification API.
- Native WebSocket client for real-time backend updates.

The backend should expose a small local API boundary instead of leaking exchange-specific payloads to the frontend. OKX raw responses are normalized into internal market, candle, funding, score, and alert types before they leave the backend.

This stack is chosen for low-latency stream handling, strong typing around trading state, and a maintainable path toward future backtesting, account-read integration, and risk controls.

## Data Flow

Public OKX APIs are the only external data source in Version 1.

The backend uses two data paths:

- Fast path: OKX WebSocket subscriptions for ticker/trade/candle-style real-time updates. This path updates prices, short-horizon movement, volume burst state, proximity to key areas, and alert triggers as soon as data arrives.
- Slow path: REST and scheduled recalculation, defaulting to every 30 seconds. This path refreshes dynamic symbol pools, candle windows, support/resistance, simplified FVG areas, funding-rate snapshots, and full trend/range scores.

The frontend subscribes to backend updates and renders:

- Real-time price changes.
- Latest scan table.
- Selected-symbol details.
- Connection and latency status.
- Browser desktop notifications.

Target behavior:

- Price display and alert checks should update immediately after backend stream events arrive.
- Full-market scan and structural signals should refresh every 30 seconds by default.
- The app should show the latest scan timestamp so the user can tell whether a score is fresh.

Absolute millisecond delivery cannot be guaranteed because exchange and network latency are outside the app's control. The design target is low local processing latency after the backend receives exchange data.

## Symbol Universe

The symbol universe has two parts.

Dynamic pool:

- OKX USDT perpetual instruments ranked by market activity.
- Initial ranking factors: 24h quote volume, 15m price movement, 1h price movement, short-term volatility, and volume expansion.
- The dynamic pool should emphasize symbols likely to appear on hot lists or draw intraday attention.

Fixed pool:

- User-configured watchlist.
- Initial examples: BTC, ETH, SOL, XRP, DOGE, LAB, RAVE, BEAT.
- Fixed symbols are always scanned even if they are not currently hot.

The merged pool should deduplicate symbols and label each symbol as dynamic, fixed, or both.

## Scoring Model

Each symbol receives two separate scores from 0 to 100.

Trend score:

- Measures whether the symbol is suitable for short-term directional follow-through.
- Inputs include 5m/15m/1h direction alignment, breakout or breakdown of recent highs/lows, volume expansion, volatility, FVG proximity, and dynamic-pool inclusion.
- Funding rate is a supporting context rather than a primary score driver.

Range score:

- Measures whether the symbol is suitable for support/resistance range trading.
- Inputs include clear recent high/low boundaries, price proximity to support or resistance, moderate volatility, volume expansion near a boundary, FVG overlap with key levels, and extreme funding-rate context.

Each score must include an explanation list. The UI should never show only a number. Example:

`LAB trend short 84: 15m drop expanded, volume 3.1x, broke range low, lower FVG is 1.4% away`

The first scoring version should be heuristic and editable. It should prioritize explainability over statistical sophistication.

## Simplified FVG Detection

Version 1 uses a simplified three-candle imbalance detector on 5m, 15m, and 1h timeframes.

Bullish FVG candidate:

- Candle 1 high is below candle 3 low.
- The gap size is above a configurable minimum threshold.
- The zone is tracked until price revisits or invalidates it.

Bearish FVG candidate:

- Candle 1 low is above candle 3 high.
- The gap size is above a configurable minimum threshold.
- The zone is tracked until price revisits or invalidates it.

For each detected zone, store:

- Timeframe.
- Direction.
- Upper and lower price.
- Distance from current price.
- Whether the zone has been filled.
- Whether it overlaps a recent support/resistance area.

This is intentionally not a complete ICT implementation. It is a practical signal component for the first version.

## Support And Resistance

Version 1 estimates support and resistance from recent candle windows.

Initial approach:

- Use local swing highs and swing lows from 15m and 1h candles.
- Cluster nearby levels within a configurable percentage band.
- Prefer levels with repeated touches or high-volume reactions.
- Mark whether current price is near a support zone, resistance zone, or range middle.

The UI should show support/resistance as approximate zones, not precise single-price predictions.

## Alerts

Alerts are browser desktop notifications plus visual highlighting in the Web dashboard.

Notification rule:

- Notify only when a symbol newly enters a high-score state.
- Avoid repeated notifications for the same unchanged state.
- Notify again if direction changes, score crosses a higher threshold, or key signal reasons materially change.

Initial alert thresholds:

- Trend opportunity: trend score >= 80.
- Range opportunity: range score >= 80.
- Watch state: either score >= 65, shown in the UI but not notified by default.

Example notification:

`LAB trend short 84: 15m drop expanded, volume 3.1x, broke range low, lower FVG is 1.4% away`

The frontend should request browser notification permission and clearly show permission status in the header.

## Web Dashboard

Header:

- OKX WebSocket connection state.
- Backend connection state.
- Latest full scan time.
- Estimated stream latency.
- Dynamic pool count.
- Fixed pool count.
- Notification permission state.

Main table:

- Symbol.
- Current price.
- 5m, 15m, and 1h percentage change.
- Trend score.
- Range score.
- Direction label: long, short, or neutral.
- Pool label: dynamic, fixed, or both.
- Latest trigger reason.
- Last updated time.

Filters:

- All.
- Trend opportunities.
- Range opportunities.
- Hot symbols.
- Fixed watchlist.

Selected-symbol detail panel:

- Current price and short-horizon moves.
- Support and resistance zones.
- Simplified FVG zones by timeframe.
- Volume expansion metrics.
- Funding-rate snapshot.
- Recent signal timeline.
- Account placeholder showing that no read-only OKX API key is connected.

Version 1 should not include full candlestick charting. The user can keep using exchange charts while this app focuses on scanning, sorting, and explaining opportunities. Charting can be added later with TradingView Lightweight Charts.

## Configuration

Initial configurable values:

- Fixed watchlist symbols.
- Dynamic pool size.
- Scan interval, default 30 seconds.
- Timeframes: 5m, 15m, 1h.
- Trend alert threshold, default 80.
- Range alert threshold, default 80.
- Watch threshold, default 65.
- Volume expansion lookback.
- Support/resistance lookback.
- FVG minimum gap threshold.
- Notification cooldown per symbol and signal type.

Configuration should be local and easy to edit.

## Error Handling

The app should handle:

- OKX REST request failures.
- WebSocket disconnects.
- Partial symbol data.
- Rate limits.
- Empty or stale candle windows.
- Browser notification permission denied.

The UI should show degraded states rather than silently failing. If WebSocket disconnects, the app should continue showing the last known scan with stale-data indicators and attempt reconnects.

## Testing Strategy

Version 1 should include focused tests for:

- Rust unit tests for symbol universe merge and deduplication.
- Rust unit tests for dynamic pool ranking.
- Rust unit tests for trend score explanation generation.
- Rust unit tests for range score explanation generation.
- Rust unit tests for simplified FVG detection.
- Rust unit tests for support/resistance clustering.
- Rust unit tests for alert deduplication and re-alert rules.
- TypeScript tests for table filtering, score display, stale-state display, and notification trigger state.

Manual verification should cover:

- Connecting to OKX public streams.
- Real-time price updates appearing in the dashboard.
- Scan refresh every 30 seconds.
- Browser notifications firing only on new high-score states.
- Stale or disconnected data being visible in the UI.

## Future Extensions

Likely next steps after Version 1:

- Add read-only OKX account integration.
- Add Telegram or mobile push notifications.
- Add TradingView Lightweight Charts.
- Add trade journal and screenshot-based review.
- Add backtesting for the heuristic scoring components.
- Add Binance as a secondary data source.
- Add OKX App hot-ranking adapter if a stable source is found.
- Add semi-automated order templates only after the signal and review workflow has proven reliable.
