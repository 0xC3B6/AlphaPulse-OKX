use std::{str::FromStr, time::Duration};

use anyhow::{anyhow, Context};
use redis::AsyncCommands;
use rust_decimal::Decimal;
use serde::Serialize;
use serde_json::json;
use sqlx::{postgres::PgPoolOptions, types::Json, PgPool};

use crate::{
    config::AppConfig,
    paper::{PaperClosedPositionSnapshot, PaperPositionSnapshot, PaperTrade},
    state::DashboardSnapshot,
    strategy::{
        RiskGuardEvent, StrategyCenterSnapshot, StrategyEquitySnapshot, VersionedPaperState,
    },
};

#[derive(Clone)]
pub struct PersistenceLayer {
    postgres: Option<PgPool>,
    redis: Option<redis::Client>,
    redis_ttl_secs: u64,
}

impl PersistenceLayer {
    pub fn disabled() -> Self {
        Self {
            postgres: None,
            redis: None,
            redis_ttl_secs: 30,
        }
    }

    pub async fn connect(config: &AppConfig) -> anyhow::Result<Self> {
        let postgres = match config.database_url.as_deref() {
            Some(url) => Some(
                PgPoolOptions::new()
                    .max_connections(5)
                    .acquire_timeout(Duration::from_secs(5))
                    .connect(url)
                    .await
                    .with_context(|| "failed to connect to PostgreSQL")?,
            ),
            None if config.require_database => {
                return Err(anyhow!(
                    "DATABASE_URL or ALPHAPULSE_DATABASE_URL is required when ALPHAPULSE_REQUIRE_DATABASE=true"
                ));
            }
            None => None,
        };
        let redis = match config.redis_url.as_deref() {
            Some(url) => Some(redis::Client::open(url).with_context(|| "invalid Redis URL")?),
            None => None,
        };
        Ok(Self {
            postgres,
            redis,
            redis_ttl_secs: config.redis_ttl_secs,
        })
    }

    pub fn is_postgres_enabled(&self) -> bool {
        self.postgres.is_some()
    }

    pub fn is_redis_enabled(&self) -> bool {
        self.redis.is_some()
    }

    pub async fn initialize(&self) -> anyhow::Result<()> {
        let Some(pool) = &self.postgres else {
            return Ok(());
        };
        for statement in postgres_schema_statements() {
            sqlx::query(statement).execute(pool).await?;
        }
        Ok(())
    }

    pub async fn load_versioned_paper_state(&self) -> anyhow::Result<Option<VersionedPaperState>> {
        let Some(pool) = &self.postgres else {
            return Ok(None);
        };
        let row = sqlx::query_as::<_, (Json<serde_json::Value>,)>(
            "SELECT payload_json FROM app_state_snapshots \
             WHERE snapshot_key = 'versioned_paper_state' \
             ORDER BY id DESC LIMIT 1",
        )
        .fetch_optional(pool)
        .await?;
        row.map(|(payload,)| {
            serde_json::from_value(payload.0).context("failed to decode persisted paper state")
        })
        .transpose()
    }

    pub async fn persist_versioned_paper_state(
        &self,
        state: &VersionedPaperState,
    ) -> anyhow::Result<()> {
        let Some(pool) = &self.postgres else {
            return Ok(());
        };
        let payload = serde_json::to_value(state)?;
        sqlx::query("INSERT INTO app_state_snapshots (snapshot_key, payload_json) VALUES ($1, $2)")
            .bind("versioned_paper_state")
            .bind(Json(payload))
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn persist_dashboard_snapshot(
        &self,
        event_type: &str,
        snapshot: &DashboardSnapshot,
    ) -> anyhow::Result<()> {
        if self.postgres.is_none() && self.redis.is_none() {
            return Ok(());
        }
        let payload = serde_json::to_value(snapshot)?;
        if let Some(pool) = &self.postgres {
            self.insert_event_log(
                pool,
                event_type,
                None,
                None,
                "dashboard",
                "latest",
                &payload,
            )
            .await?;
            sqlx::query(
                "INSERT INTO app_state_snapshots (snapshot_key, payload_json) VALUES ($1, $2)",
            )
            .bind("dashboard_snapshot")
            .bind(Json(payload.clone()))
            .execute(pool)
            .await?;
            self.persist_strategy_center(pool, &snapshot.strategy_center)
                .await?;
        }
        self.cache_json("alphapulse:dashboard:snapshot", &payload)
            .await?;
        Ok(())
    }

    async fn persist_strategy_center(
        &self,
        pool: &PgPool,
        center: &StrategyCenterSnapshot,
    ) -> anyhow::Result<()> {
        for version in &center.versions {
            let config_json = Json(version.version.config_json.clone());
            sqlx::query(
                "INSERT INTO strategy_versions \
                 (version_code, name, description, status, config_json, config_hash, created_at_ms, updated_at_ms) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8) \
                 ON CONFLICT (version_code) DO UPDATE SET \
                 name = EXCLUDED.name, description = EXCLUDED.description, status = EXCLUDED.status, \
                 config_json = EXCLUDED.config_json, config_hash = EXCLUDED.config_hash, updated_at_ms = EXCLUDED.updated_at_ms",
            )
            .bind(&version.version.version_code)
            .bind(&version.version.name)
            .bind(&version.version.description)
            .bind(format!("{:?}", version.version.status).to_ascii_lowercase())
            .bind(config_json)
            .bind(&version.version.config_hash)
            .bind(version.version.created_at_ms)
            .bind(version.version.updated_at_ms)
            .execute(pool)
            .await?;

            sqlx::query(
                "INSERT INTO strategy_runs \
                 (run_id, version_code, mode, initial_equity, current_equity, realized_pnl, unrealized_pnl, \
                  fee_total, max_drawdown, status, start_time_ms, end_time_ms, fee_model, slippage_model, config_snapshot) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15) \
                 ON CONFLICT (run_id) DO UPDATE SET \
                 current_equity = EXCLUDED.current_equity, realized_pnl = EXCLUDED.realized_pnl, \
                 unrealized_pnl = EXCLUDED.unrealized_pnl, fee_total = EXCLUDED.fee_total, \
                 max_drawdown = EXCLUDED.max_drawdown, status = EXCLUDED.status, end_time_ms = EXCLUDED.end_time_ms",
            )
            .bind(&version.run.run_id)
            .bind(&version.run.version_code)
            .bind(format!("{:?}", version.run.mode).to_ascii_lowercase())
            .bind(decimal_or_zero(version.run.initial_equity))
            .bind(decimal_or_zero(version.run.current_equity))
            .bind(decimal_or_zero(version.run.realized_pnl))
            .bind(decimal_or_zero(version.run.unrealized_pnl))
            .bind(decimal_or_zero(version.run.fee_total))
            .bind(decimal_or_zero(version.run.max_drawdown))
            .bind(format!("{:?}", version.run.status).to_ascii_lowercase())
            .bind(version.run.start_time_ms)
            .bind(version.run.end_time_ms)
            .bind(&version.run.fee_model)
            .bind(&version.run.slippage_model)
            .bind(Json(version.run.config_snapshot.clone()))
            .execute(pool)
            .await?;

            sqlx::query("DELETE FROM positions WHERE run_id = $1 AND status = 'open'")
                .bind(&version.run.run_id)
                .execute(pool)
                .await?;
            for position in &version.paper.positions {
                self.upsert_position(pool, position).await?;
            }
            for trade in &version.paper.trades {
                self.upsert_fill(pool, trade).await?;
            }
            for closed in &version.paper.position_history {
                self.upsert_closed_trade(pool, closed).await?;
            }
            for equity in &version.equity {
                self.upsert_equity_snapshot(pool, equity).await?;
            }
            for event in &version.risk_guard_events {
                self.upsert_risk_guard_event(pool, event).await?;
            }
        }
        Ok(())
    }

    async fn upsert_position(
        &self,
        pool: &PgPool,
        position: &PaperPositionSnapshot,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO positions \
             (position_key, run_id, version_code, symbol, side, entry_price, mark_price, margin, leverage, quantity, \
              unrealized_pnl, status, opened_at_ms, closed_at_ms) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, 'open', $12, NULL) \
             ON CONFLICT (position_key) DO UPDATE SET \
             mark_price = EXCLUDED.mark_price, margin = EXCLUDED.margin, leverage = EXCLUDED.leverage, \
             quantity = EXCLUDED.quantity, unrealized_pnl = EXCLUDED.unrealized_pnl, status = 'open'",
        )
        .bind(format!("{}:{}", position.run_id, position.inst_id))
        .bind(&position.run_id)
        .bind(&position.version_code)
        .bind(&position.inst_id)
        .bind(format!("{:?}", position.side).to_ascii_lowercase())
        .bind(decimal_or_zero(position.entry_price))
        .bind(decimal_or_zero(position.mark_price))
        .bind(decimal_or_zero(position.margin))
        .bind(decimal_or_zero(position.leverage))
        .bind(decimal_or_zero(position.qty))
        .bind(decimal_or_zero(position.unrealized_pnl))
        .bind(position.opened_at_ms)
        .execute(pool)
        .await?;
        Ok(())
    }

    async fn upsert_fill(&self, pool: &PgPool, trade: &PaperTrade) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO fills \
             (trade_id, order_intent_id, run_id, version_code, symbol, side, action, price, quantity, fee, slippage, filled_at_ms) \
             VALUES ($1, NULL, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) \
             ON CONFLICT (run_id, trade_id) DO UPDATE SET \
             price = EXCLUDED.price, quantity = EXCLUDED.quantity, fee = EXCLUDED.fee, slippage = EXCLUDED.slippage",
        )
        .bind(trade.id as i64)
        .bind(&trade.run_id)
        .bind(&trade.version_code)
        .bind(&trade.inst_id)
        .bind(format!("{:?}", trade.side).to_ascii_lowercase())
        .bind(format!("{:?}", trade.action).to_ascii_lowercase())
        .bind(decimal_or_zero(trade.price))
        .bind(decimal_or_zero(trade.qty))
        .bind(decimal_or_zero(trade.fee))
        .bind(decimal_or_zero(trade.slippage_rate))
        .bind(trade.ts_ms)
        .execute(pool)
        .await?;
        Ok(())
    }

    async fn upsert_closed_trade(
        &self,
        pool: &PgPool,
        trade: &PaperClosedPositionSnapshot,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO closed_trades \
             (closed_position_id, run_id, version_code, symbol, side, entry_price, exit_price, margin, leverage, \
              quantity, gross_pnl, fee, net_pnl, primary_signal, tags_json, exit_reason, hold_seconds, opened_at_ms, closed_at_ms) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19) \
             ON CONFLICT (run_id, closed_position_id) DO UPDATE SET \
             exit_price = EXCLUDED.exit_price, gross_pnl = EXCLUDED.gross_pnl, net_pnl = EXCLUDED.net_pnl, exit_reason = EXCLUDED.exit_reason",
        )
        .bind(trade.id as i64)
        .bind(&trade.run_id)
        .bind(&trade.version_code)
        .bind(&trade.inst_id)
        .bind(format!("{:?}", trade.side).to_ascii_lowercase())
        .bind(decimal_or_zero(trade.entry_price))
        .bind(decimal_or_zero(trade.exit_price))
        .bind(decimal_or_zero(trade.margin))
        .bind(decimal_or_zero(trade.leverage))
        .bind(decimal_or_zero(trade.qty))
        .bind(decimal_or_zero(trade.realized_pnl))
        .bind(decimal_or_zero(trade.fees))
        .bind(decimal_or_zero(trade.realized_pnl - trade.fees))
        .bind(&trade.primary_signal)
        .bind(Json(json!(trade.tags)))
        .bind(&trade.close_reason)
        .bind(trade.duration_ms / 1_000)
        .bind(trade.opened_at_ms)
        .bind(trade.closed_at_ms)
        .execute(pool)
        .await?;
        Ok(())
    }

    async fn upsert_equity_snapshot(
        &self,
        pool: &PgPool,
        snapshot: &StrategyEquitySnapshot,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO equity_snapshots \
             (run_id, version_code, timestamp_ms, equity, realized_pnl, unrealized_pnl, drawdown, open_positions_count) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8) \
             ON CONFLICT (run_id, timestamp_ms) DO UPDATE SET \
             equity = EXCLUDED.equity, realized_pnl = EXCLUDED.realized_pnl, \
             unrealized_pnl = EXCLUDED.unrealized_pnl, drawdown = EXCLUDED.drawdown, \
             open_positions_count = EXCLUDED.open_positions_count",
        )
        .bind(&snapshot.run_id)
        .bind(&snapshot.version_code)
        .bind(snapshot.timestamp_ms)
        .bind(decimal_or_zero(snapshot.equity))
        .bind(decimal_or_zero(snapshot.realized_pnl))
        .bind(decimal_or_zero(snapshot.unrealized_pnl))
        .bind(decimal_or_zero(snapshot.drawdown))
        .bind(snapshot.open_positions_count as i64)
        .execute(pool)
        .await?;
        Ok(())
    }

    async fn upsert_risk_guard_event(
        &self,
        pool: &PgPool,
        event: &RiskGuardEvent,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO risk_guard_events \
             (event_id, run_id, version_code, symbol, side, action, reason, risk_flags_json, \
              original_order_intent_json, final_decision, created_at_ms) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) \
             ON CONFLICT (run_id, event_id) DO UPDATE SET \
             action = EXCLUDED.action, reason = EXCLUDED.reason, risk_flags_json = EXCLUDED.risk_flags_json",
        )
        .bind(event.id as i64)
        .bind(&event.run_id)
        .bind(&event.version_code)
        .bind(&event.symbol)
        .bind(format!("{:?}", event.side).to_ascii_lowercase())
        .bind(&event.action)
        .bind(&event.reason)
        .bind(Json(json!(event.risk_flags)))
        .bind(Json(serde_json::to_value(&event.original_order_intent)?))
        .bind(event.final_order_intent.as_ref().map(|_| "adjusted").unwrap_or("blocked"))
        .bind(event.timestamp_ms)
        .execute(pool)
        .await?;
        Ok(())
    }

    async fn insert_event_log(
        &self,
        pool: &PgPool,
        event_type: &str,
        run_id: Option<&str>,
        version_code: Option<&str>,
        aggregate_type: &str,
        aggregate_id: &str,
        payload: &serde_json::Value,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO event_log \
             (event_type, run_id, version_code, aggregate_type, aggregate_id, payload_json, created_at_ms) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(event_type)
        .bind(run_id)
        .bind(version_code)
        .bind(aggregate_type)
        .bind(aggregate_id)
        .bind(Json(payload.clone()))
        .bind(chrono::Utc::now().timestamp_millis())
        .execute(pool)
        .await?;
        Ok(())
    }

    async fn cache_json<T: Serialize + ?Sized>(&self, key: &str, value: &T) -> anyhow::Result<()> {
        let Some(client) = &self.redis else {
            return Ok(());
        };
        let mut connection = client.get_multiplexed_async_connection().await?;
        let payload = serde_json::to_string(value)?;
        connection
            .set_ex::<_, _, ()>(key, payload, self.redis_ttl_secs)
            .await?;
        Ok(())
    }
}

pub fn decimal_from_f64(value: f64) -> Option<Decimal> {
    if !value.is_finite() {
        return None;
    }
    Decimal::from_str(&format!("{value:.8}"))
        .ok()
        .map(|decimal| decimal.normalize())
}

fn decimal_or_zero(value: f64) -> Decimal {
    decimal_from_f64(value).unwrap_or(Decimal::ZERO)
}

pub fn postgres_schema_statements() -> Vec<&'static str> {
    vec![
        "CREATE TABLE IF NOT EXISTS strategy_versions (
            version_code TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT NOT NULL,
            status TEXT NOT NULL,
            config_json JSONB NOT NULL,
            config_hash TEXT NOT NULL,
            created_at_ms BIGINT NOT NULL,
            updated_at_ms BIGINT NOT NULL
        )",
        "CREATE TABLE IF NOT EXISTS strategy_runs (
            run_id TEXT PRIMARY KEY,
            version_code TEXT NOT NULL REFERENCES strategy_versions(version_code),
            mode TEXT NOT NULL,
            initial_equity NUMERIC NOT NULL,
            current_equity NUMERIC NOT NULL,
            realized_pnl NUMERIC NOT NULL,
            unrealized_pnl NUMERIC NOT NULL,
            fee_total NUMERIC NOT NULL,
            max_drawdown NUMERIC NOT NULL,
            status TEXT NOT NULL,
            start_time_ms BIGINT NOT NULL,
            end_time_ms BIGINT,
            fee_model TEXT NOT NULL,
            slippage_model TEXT NOT NULL,
            config_snapshot JSONB NOT NULL
        )",
        "CREATE TABLE IF NOT EXISTS order_intents (
            id BIGSERIAL PRIMARY KEY,
            run_id TEXT NOT NULL,
            version_code TEXT NOT NULL,
            symbol TEXT NOT NULL,
            side TEXT NOT NULL,
            action TEXT NOT NULL,
            score INTEGER NOT NULL,
            primary_signal TEXT NOT NULL,
            reason TEXT NOT NULL,
            tags_json JSONB NOT NULL,
            risk_flags_json JSONB NOT NULL,
            status TEXT NOT NULL,
            created_at_ms BIGINT NOT NULL
        )",
        "CREATE TABLE IF NOT EXISTS fills (
            id BIGSERIAL PRIMARY KEY,
            trade_id BIGINT NOT NULL,
            order_intent_id BIGINT REFERENCES order_intents(id),
            run_id TEXT NOT NULL,
            version_code TEXT NOT NULL,
            symbol TEXT NOT NULL,
            side TEXT NOT NULL,
            action TEXT NOT NULL,
            price NUMERIC NOT NULL,
            quantity NUMERIC NOT NULL,
            fee NUMERIC NOT NULL,
            slippage NUMERIC NOT NULL,
            filled_at_ms BIGINT NOT NULL,
            UNIQUE (run_id, trade_id)
        )",
        "CREATE TABLE IF NOT EXISTS positions (
            position_key TEXT PRIMARY KEY,
            run_id TEXT NOT NULL,
            version_code TEXT NOT NULL,
            symbol TEXT NOT NULL,
            side TEXT NOT NULL,
            entry_price NUMERIC NOT NULL,
            mark_price NUMERIC NOT NULL,
            margin NUMERIC NOT NULL,
            leverage NUMERIC NOT NULL,
            quantity NUMERIC NOT NULL,
            unrealized_pnl NUMERIC NOT NULL,
            status TEXT NOT NULL,
            opened_at_ms BIGINT NOT NULL,
            closed_at_ms BIGINT
        )",
        "CREATE TABLE IF NOT EXISTS closed_trades (
            id BIGSERIAL PRIMARY KEY,
            closed_position_id BIGINT NOT NULL,
            run_id TEXT NOT NULL,
            version_code TEXT NOT NULL,
            symbol TEXT NOT NULL,
            side TEXT NOT NULL,
            entry_price NUMERIC NOT NULL,
            exit_price NUMERIC NOT NULL,
            margin NUMERIC NOT NULL,
            leverage NUMERIC NOT NULL,
            quantity NUMERIC NOT NULL,
            gross_pnl NUMERIC NOT NULL,
            fee NUMERIC NOT NULL,
            net_pnl NUMERIC NOT NULL,
            primary_signal TEXT NOT NULL,
            tags_json JSONB NOT NULL,
            exit_reason TEXT NOT NULL,
            hold_seconds BIGINT NOT NULL,
            opened_at_ms BIGINT NOT NULL,
            closed_at_ms BIGINT NOT NULL,
            UNIQUE (run_id, closed_position_id)
        )",
        "CREATE TABLE IF NOT EXISTS equity_snapshots (
            id BIGSERIAL PRIMARY KEY,
            run_id TEXT NOT NULL,
            version_code TEXT NOT NULL,
            timestamp_ms BIGINT NOT NULL,
            equity NUMERIC NOT NULL,
            realized_pnl NUMERIC NOT NULL,
            unrealized_pnl NUMERIC NOT NULL,
            drawdown NUMERIC NOT NULL,
            open_positions_count BIGINT NOT NULL,
            UNIQUE (run_id, timestamp_ms)
        )",
        "CREATE TABLE IF NOT EXISTS risk_guard_events (
            id BIGSERIAL PRIMARY KEY,
            event_id BIGINT NOT NULL,
            run_id TEXT NOT NULL,
            version_code TEXT NOT NULL,
            symbol TEXT NOT NULL,
            side TEXT NOT NULL,
            action TEXT NOT NULL,
            reason TEXT NOT NULL,
            risk_flags_json JSONB NOT NULL,
            original_order_intent_json JSONB NOT NULL,
            final_decision TEXT NOT NULL,
            created_at_ms BIGINT NOT NULL,
            UNIQUE (run_id, event_id)
        )",
        "CREATE TABLE IF NOT EXISTS event_log (
            id BIGSERIAL PRIMARY KEY,
            event_type TEXT NOT NULL,
            run_id TEXT,
            version_code TEXT,
            aggregate_type TEXT NOT NULL,
            aggregate_id TEXT NOT NULL,
            payload_json JSONB NOT NULL,
            created_at_ms BIGINT NOT NULL
        )",
        "CREATE TABLE IF NOT EXISTS app_state_snapshots (
            id BIGSERIAL PRIMARY KEY,
            snapshot_key TEXT NOT NULL,
            payload_json JSONB NOT NULL,
            created_at_ms BIGINT NOT NULL DEFAULT (EXTRACT(EPOCH FROM NOW()) * 1000)::BIGINT
        )",
    ]
}
