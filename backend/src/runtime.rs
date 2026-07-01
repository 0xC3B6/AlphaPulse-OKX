use std::time::Duration;
use std::{collections::HashMap, sync::Arc};

use chrono::Utc;
use futures_util::{stream, StreamExt};
use tokio::{sync::mpsc, time};

use crate::{
    config::AppConfig,
    domain::{Candle, Direction, Score, SymbolSnapshot, Timeframe},
    indicators::{
        fvg::detect_fvgs,
        levels::{find_levels, LevelConfig},
    },
    okx::{
        rest::{OkxRestClient, TickerRow},
        ws::{self, TickerEvent},
    },
    quality::{add_tag, classify_history},
    scoring::{score_symbol, ScoringInput},
    state::RadarState,
    universe::{build_filtered_symbol_universe, MarketActivity, UniverseSymbol},
};

pub async fn run_scanner(config: AppConfig, state: RadarState) -> anyhow::Result<()> {
    let rest = OkxRestClient::new();
    let (ticker_tx, mut ticker_rx) = mpsc::channel::<TickerEvent>(1024);
    spawn_fixed_ticker_stream(config.fixed_watchlist.clone(), ticker_tx, state.clone());

    if let Err(error) = scan_once(&config, &state, &rest).await {
        tracing::warn!(?error, "initial OKX scan failed");
    }

    let mut interval = time::interval(Duration::from_secs(config.scan_interval_secs));
    interval.set_missed_tick_behavior(time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = interval.tick() => {
                if let Err(error) = scan_once(&config, &state, &rest).await {
                    tracing::warn!(?error, "OKX scan failed");
                }
            }
            Some(event) = ticker_rx.recv() => {
                let _ = state
                    .update_symbol_price(&event.inst_id, event.last, event.ts_ms)
                    .await;
            }
        }
    }
}

fn spawn_fixed_ticker_stream(
    inst_ids: Vec<String>,
    sender: mpsc::Sender<TickerEvent>,
    state: RadarState,
) {
    tokio::spawn(async move {
        loop {
            state.set_websocket_connected(false).await;
            state.set_websocket_connected(true).await;
            match ws::stream_tickers(inst_ids.clone(), sender.clone()).await {
                Ok(()) => tracing::warn!("OKX ticker stream closed"),
                Err(error) => tracing::warn!(?error, "OKX ticker stream failed"),
            }
            state.set_websocket_connected(false).await;
            time::sleep(Duration::from_secs(5)).await;
        }
    });
}

async fn scan_once(
    config: &AppConfig,
    state: &RadarState,
    rest: &OkxRestClient,
) -> anyhow::Result<()> {
    let tickers = rest.fetch_swap_tickers().await?;
    let instruments = rest.fetch_swap_instruments().await?;
    let ticker_map: Arc<HashMap<String, TickerRow>> = Arc::new(
        tickers
            .into_iter()
            .filter(|ticker| ticker.inst_id.ends_with("-USDT-SWAP"))
            .map(|ticker| (ticker.inst_id.clone(), ticker))
            .collect(),
    );

    let seed_activity: Vec<MarketActivity> = ticker_map
        .values()
        .map(|ticker| {
            MarketActivity::new(&ticker.inst_id, ticker.quote_volume_24h, 0.0, 0.0, 0.0, 1.0)
        })
        .collect();
    let policy = config.universe_policy();
    let universe = build_filtered_symbol_universe(
        &seed_activity,
        &config.fixed_watchlist,
        config.dynamic_pool_size,
        &instruments,
        policy,
        Utc::now().timestamp_millis(),
    );

    let mut snapshots = stream::iter(universe.into_iter().map(|symbol| {
        let ticker_map = Arc::clone(&ticker_map);
        let rest = rest.clone();
        async move {
            let inst_id = symbol.inst_id.clone();
            let result = build_symbol_snapshot(&symbol, &ticker_map, &rest, policy).await;
            (inst_id, result)
        }
    }))
    .buffer_unordered(8);

    while let Some((inst_id, result)) = snapshots.next().await {
        match result {
            Ok(snapshot) => state.upsert_symbol(snapshot).await,
            Err(error) => tracing::debug!(%inst_id, ?error, "symbol scan failed"),
        }
    }

    state.mark_scan(Utc::now().timestamp_millis()).await;
    Ok(())
}

async fn build_symbol_snapshot(
    symbol: &UniverseSymbol,
    tickers: &HashMap<String, TickerRow>,
    rest: &OkxRestClient,
    policy: crate::quality::UniversePolicy,
) -> anyhow::Result<SymbolSnapshot> {
    let ticker = tickers
        .get(&symbol.inst_id)
        .ok_or_else(|| anyhow::anyhow!("missing ticker for {}", symbol.inst_id))?;
    let candles_5m = rest
        .fetch_candles(&symbol.inst_id, Timeframe::M5, 120)
        .await
        .unwrap_or_default();
    let candles_15m = rest
        .fetch_candles(&symbol.inst_id, Timeframe::M15, 120)
        .await
        .unwrap_or_default();
    let candles_1h = rest
        .fetch_candles(&symbol.inst_id, Timeframe::H1, 240)
        .await
        .unwrap_or_default();

    let history_decision = classify_history(&candles_1h, policy);
    if !history_decision.allowed {
        anyhow::bail!(
            "insufficient 1h candle history: {:.2} days",
            history_decision.history_days
        );
    }
    let mut pool_tags = symbol.tags.clone();
    for tag in history_decision.tags {
        add_tag(&mut pool_tags, &tag);
    }

    let price = ticker.last;
    let change_5m_pct = last_bar_change(&candles_5m);
    let change_15m_pct = last_bar_change(&candles_15m);
    let change_1h_pct = last_bar_change(&candles_1h);
    let volume_ratio = volume_ratio(&candles_5m).unwrap_or(1.0);
    let volatility_1h = volatility(&candles_1h, price);

    let mut fvgs = Vec::new();
    fvgs.extend(detect_fvgs(&candles_5m, Timeframe::M5, 0.003, price));
    fvgs.extend(detect_fvgs(&candles_15m, Timeframe::M15, 0.003, price));
    fvgs.extend(detect_fvgs(&candles_1h, Timeframe::H1, 0.003, price));
    fvgs.sort_by(|left, right| {
        left.distance_pct
            .partial_cmp(&right.distance_pct)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let level_source = if candles_1h.len() >= 12 {
        &candles_1h
    } else {
        &candles_15m
    };
    let levels = find_levels(
        level_source,
        price,
        LevelConfig {
            cluster_pct: 0.01,
            min_touches: 2,
        },
    );
    let nearest_fvg_distance_pct = fvgs
        .iter()
        .filter(|zone| !zone.filled)
        .map(|zone| zone.distance_pct)
        .min_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    let near_support = levels
        .iter()
        .any(|level| level.kind == crate::domain::LevelKind::Support && level.distance_pct <= 0.01);
    let near_resistance = levels.iter().any(|level| {
        level.kind == crate::domain::LevelKind::Resistance && level.distance_pct <= 0.01
    });
    let clear_range = levels
        .iter()
        .any(|level| level.kind == crate::domain::LevelKind::Support)
        && levels
            .iter()
            .any(|level| level.kind == crate::domain::LevelKind::Resistance)
        && volatility_1h <= 0.12;

    let scored = score_symbol(ScoringInput {
        inst_id: symbol.inst_id.clone(),
        change_5m_pct,
        change_15m_pct,
        change_1h_pct,
        broke_recent_high: broke_recent_high(&candles_15m),
        broke_recent_low: broke_recent_low(&candles_15m),
        volume_ratio,
        nearest_fvg_distance_pct,
        dynamic_pool: symbol.tags.iter().any(|tag| tag == "dynamic"),
        near_support,
        near_resistance,
        clear_range,
        funding_rate: None,
    });

    let trigger_reason = trigger_reason(&symbol.inst_id, &scored.trend_score, &scored.range_score);

    Ok(SymbolSnapshot {
        inst_id: symbol.inst_id.clone(),
        price,
        change_5m_pct,
        change_15m_pct,
        change_1h_pct,
        trend_score: scored.trend_score,
        range_score: scored.range_score,
        pool_tags,
        trigger_reason,
        funding_rate: None,
        fvgs,
        levels,
        updated_at_ms: ticker.ts_ms,
    })
}

fn last_bar_change(candles: &[Candle]) -> f64 {
    let Some((previous, latest)) = candles.iter().rev().nth(1).zip(candles.last()) else {
        return 0.0;
    };
    if previous.close <= 0.0 {
        return 0.0;
    }
    latest.close / previous.close - 1.0
}

fn volume_ratio(candles: &[Candle]) -> Option<f64> {
    let latest = candles.last()?;
    let previous: Vec<_> = candles.iter().rev().skip(1).take(20).collect();
    if previous.is_empty() {
        return None;
    }
    let average = previous.iter().map(|candle| candle.volume).sum::<f64>() / previous.len() as f64;
    if average <= 0.0 {
        return None;
    }
    Some(latest.volume / average)
}

fn volatility(candles: &[Candle], price: f64) -> f64 {
    if candles.is_empty() || price <= 0.0 {
        return 0.0;
    }
    let high = candles
        .iter()
        .rev()
        .take(24)
        .map(|candle| candle.high)
        .fold(f64::NEG_INFINITY, f64::max);
    let low = candles
        .iter()
        .rev()
        .take(24)
        .map(|candle| candle.low)
        .fold(f64::INFINITY, f64::min);
    (high - low).max(0.0) / price
}

fn broke_recent_high(candles: &[Candle]) -> bool {
    let Some(latest) = candles.last() else {
        return false;
    };
    let recent_high = candles
        .iter()
        .rev()
        .skip(1)
        .take(20)
        .map(|candle| candle.high)
        .fold(f64::NEG_INFINITY, f64::max);
    latest.close > recent_high
}

fn broke_recent_low(candles: &[Candle]) -> bool {
    let Some(latest) = candles.last() else {
        return false;
    };
    let recent_low = candles
        .iter()
        .rev()
        .skip(1)
        .take(20)
        .map(|candle| candle.low)
        .fold(f64::INFINITY, f64::min);
    latest.close < recent_low
}

fn trigger_reason(inst_id: &str, trend: &Score, range: &Score) -> String {
    let (kind, score) = if trend.value >= range.value {
        ("trend", trend)
    } else {
        ("range", range)
    };
    let direction = match score.direction {
        Direction::Long => "long",
        Direction::Short => "short",
        Direction::Neutral => "neutral",
    };
    let reason = score
        .reasons
        .first()
        .cloned()
        .unwrap_or_else(|| "watching".to_string());
    format!("{inst_id} {kind} {direction} {}: {reason}", score.value)
}
