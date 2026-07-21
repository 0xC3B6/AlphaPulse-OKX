use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Write,
    path::{Path, PathBuf},
    str::FromStr,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use anyhow::{anyhow, Context};
use redis::AsyncCommands;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::{postgres::PgPoolOptions, types::Json, PgPool, Postgres, Transaction};

use crate::{
    config::AppConfig,
    paper::{
        compact_equity_history, PaperAccountSnapshot, PaperEquityPoint, PaperOrderRequest,
        PaperSide, PaperState, PaperTrade, PaperTradeAction, MAX_EQUITY_HISTORY_POINTS,
    },
    risk_safety::{AccountScope, DEFAULT_ACCOUNT_ID, DEFAULT_TENANT_ID},
    state::DashboardSnapshot,
    strategy_identity::StrategyIdentity,
};

static REJECTED_INTENT_SEQUENCE: AtomicU64 = AtomicU64::new(1);
const STRATEGY_TABLES: &[&str] = &[
    "strategy_versions",
    "strategy_runs",
    "order_intents",
    "fills",
    "positions",
    "closed_trades",
    "equity_snapshots",
    "event_log",
    "app_state_snapshots",
];

#[derive(Clone)]
pub struct PersistenceLayer {
    postgres: PgPool,
    redis: Option<redis::Client>,
    redis_ttl_secs: u64,
    scope: AccountScope,
}

#[derive(Debug, Clone)]
pub struct StrategyBackup {
    pub manifest_path: PathBuf,
    pub manifest: StrategyBackupManifest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyBackupManifest {
    pub created_at_ms: i64,
    pub version_codes: Vec<String>,
    pub files: Vec<StrategyBackupFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyBackupFile {
    pub table: String,
    pub relative_path: String,
    pub row_count: i64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedOrderIntent {
    pub client_intent_id: String,
    pub run_id: String,
    pub identity: StrategyIdentity,
    pub symbol: String,
    pub side: PaperSide,
    pub action: PaperTradeAction,
    pub score: u8,
    pub primary_signal: String,
    pub reason: String,
    pub tags: Vec<String>,
    pub status: String,
    pub rejection_reason: Option<String>,
    pub created_at_ms: i64,
}

impl PersistedOrderIntent {
    pub fn accepted_open(
        state: &PaperState,
        order: &PaperOrderRequest,
        trade: &PaperTrade,
        score: u8,
    ) -> Self {
        Self::accepted_trade(state, trade, score, order.reason.as_deref())
    }

    pub fn accepted_close(state: &PaperState, trade: &PaperTrade, score: u8) -> Self {
        Self::accepted_trade(state, trade, score, Some(&trade.reason))
    }

    pub fn rejected_open(
        state: &PaperState,
        order: &PaperOrderRequest,
        score: u8,
        created_at_ms: i64,
        error: impl Into<String>,
    ) -> Self {
        Self::rejected(
            state,
            &order.inst_id,
            order.side,
            PaperTradeAction::Open,
            score,
            order.primary_signal.as_deref().unwrap_or("manual"),
            order.reason.as_deref().unwrap_or("paper open"),
            order.signal_tags.clone(),
            created_at_ms,
            error,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn rejected_close(
        state: &PaperState,
        symbol: &str,
        side: PaperSide,
        score: u8,
        primary_signal: &str,
        reason: &str,
        tags: Vec<String>,
        created_at_ms: i64,
        error: impl Into<String>,
    ) -> Self {
        Self::rejected(
            state,
            symbol,
            side,
            PaperTradeAction::Close,
            score,
            primary_signal,
            reason,
            tags,
            created_at_ms,
            error,
        )
    }

    fn accepted_trade(
        state: &PaperState,
        trade: &PaperTrade,
        score: u8,
        reason: Option<&str>,
    ) -> Self {
        let mut tags = trade.signal_tags.clone();
        tags.extend(trade.tags.iter().map(|tag| tag.label.clone()));
        Self {
            client_intent_id: format!(
                "{}:trade:{}:{}",
                state.run_id(),
                trade.id,
                action_name(trade.action)
            ),
            run_id: state.run_id().to_string(),
            identity: state.strategy_identity().clone(),
            symbol: trade.inst_id.clone(),
            side: trade.side,
            action: trade.action,
            score,
            primary_signal: if trade.primary_signal.is_empty() {
                "manual".to_string()
            } else {
                trade.primary_signal.clone()
            },
            reason: reason.unwrap_or(&trade.reason).to_string(),
            tags,
            status: "accepted".to_string(),
            rejection_reason: None,
            created_at_ms: trade.ts_ms,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn rejected(
        state: &PaperState,
        symbol: &str,
        side: PaperSide,
        action: PaperTradeAction,
        score: u8,
        primary_signal: &str,
        reason: &str,
        tags: Vec<String>,
        created_at_ms: i64,
        error: impl Into<String>,
    ) -> Self {
        let sequence = REJECTED_INTENT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        Self {
            client_intent_id: format!(
                "{}:rejected:{}:{created_at_ms}:{sequence}",
                state.run_id(),
                action_name(action)
            ),
            run_id: state.run_id().to_string(),
            identity: state.strategy_identity().clone(),
            symbol: symbol.to_string(),
            side,
            action,
            score,
            primary_signal: primary_signal.to_string(),
            reason: reason.to_string(),
            tags,
            status: "rejected".to_string(),
            rejection_reason: Some(error.into()),
            created_at_ms,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedTransition {
    pub event_type: String,
    pub intent: Option<PersistedOrderIntent>,
    pub state: PaperState,
    pub snapshot: PaperAccountSnapshot,
    pub committed_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PersistenceStatus {
    Healthy,
    PersistencePaused,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PersistenceHealthSnapshot {
    pub status: PersistenceStatus,
    pub last_committed_at_ms: Option<i64>,
    pub last_error: Option<String>,
}

impl Default for PersistenceHealthSnapshot {
    fn default() -> Self {
        Self {
            status: PersistenceStatus::Healthy,
            last_committed_at_ms: None,
            last_error: None,
        }
    }
}

impl PersistenceHealthSnapshot {
    pub fn healthy(ts_ms: i64) -> Self {
        Self {
            status: PersistenceStatus::Healthy,
            last_committed_at_ms: Some(ts_ms),
            last_error: None,
        }
    }

    pub fn paused(error: impl Into<String>) -> Self {
        Self {
            status: PersistenceStatus::PersistencePaused,
            last_committed_at_ms: None,
            last_error: Some(error.into()),
        }
    }
}

impl PersistenceLayer {
    pub async fn connect_required(config: &AppConfig) -> anyhow::Result<Self> {
        let database_url = config.database_url.as_deref().ok_or_else(|| {
            anyhow!("ALPHAPULSE_DATABASE_URL is required for restored v0.1.3 durable execution")
        })?;
        let postgres = PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(5))
            .connect(database_url)
            .await
            .context("failed to connect to PostgreSQL")?;
        let redis = config
            .redis_url
            .as_deref()
            .map(redis::Client::open)
            .transpose()
            .context("invalid Redis URL")?;
        let scope = AccountScope::new(&config.tenant_id, &config.account_id)
            .context("invalid tenant/account scope")?;
        Ok(Self {
            postgres,
            redis,
            redis_ttl_secs: config.redis_ttl_secs,
            scope,
        })
    }

    pub async fn initialize(&self) -> anyhow::Result<()> {
        for statement in postgres_schema_statements() {
            sqlx::query(statement).execute(&self.postgres).await?;
        }
        Ok(())
    }

    pub async fn load_paper_state(
        &self,
        identity: &StrategyIdentity,
    ) -> anyhow::Result<Option<PaperState>> {
        let row = sqlx::query_as::<_, (Json<serde_json::Value>,)>(
            "SELECT payload_json FROM app_state_snapshots \
             WHERE version_code = $1 AND strategy_build_id = $2 AND config_hash = $3 \
               AND tenant_id = $4 AND account_id = $5 \
             ORDER BY id DESC LIMIT 1",
        )
        .bind(&identity.version_code)
        .bind(&identity.strategy_build_id)
        .bind(&identity.config_hash)
        .bind(&self.scope.tenant_id)
        .bind(&self.scope.account_id)
        .fetch_optional(&self.postgres)
        .await?;
        row.map(|(payload,)| {
            serde_json::from_value(payload.0).context("failed to decode persisted paper state")
        })
        .transpose()
    }

    pub async fn load_equity_history(
        &self,
        identity: &StrategyIdentity,
        run_id: &str,
    ) -> anyhow::Result<Vec<PaperEquityPoint>> {
        let database_run_id = self.database_run_id(run_id);
        let mut compatible_run_ids = vec![database_run_id.clone()];
        // Snapshots written before account scoping used the raw run id and belong
        // exclusively to the original default paper account.
        if self.scope.tenant_id == DEFAULT_TENANT_ID
            && self.scope.account_id == DEFAULT_ACCOUNT_ID
            && database_run_id != run_id
        {
            compatible_run_ids.push(run_id.to_string());
        }
        let rows = sqlx::query_as::<_, (i64, f64, f64, f64, i64)>(
            "WITH compatible AS (\
                 SELECT DISTINCT ON (timestamp_ms) \
                        timestamp_ms, equity::DOUBLE PRECISION AS equity, \
                        realized_pnl::DOUBLE PRECISION AS realized_pnl, \
                        unrealized_pnl::DOUBLE PRECISION AS unrealized_pnl, open_positions_count \
                 FROM equity_snapshots \
                 WHERE version_code = $1 AND strategy_build_id = $2 AND run_id = ANY($3) \
                 ORDER BY timestamp_ms, (run_id = $4) DESC, id DESC\
             ), ordered AS (\
                 SELECT timestamp_ms, equity, realized_pnl, unrealized_pnl, open_positions_count, \
                        ROW_NUMBER() OVER (ORDER BY timestamp_ms) AS sample_index, \
                        COUNT(*) OVER () AS total_rows \
                 FROM compatible\
             ) \
             SELECT timestamp_ms, equity, realized_pnl, unrealized_pnl, open_positions_count \
             FROM ordered \
             WHERE total_rows <= $5 OR sample_index = 1 OR sample_index = total_rows \
                OR MOD(sample_index - 1, GREATEST(1, (total_rows + $5 - 1) / $5)) = 0 \
             ORDER BY timestamp_ms",
        )
        .bind(&identity.version_code)
        .bind(&identity.strategy_build_id)
        .bind(compatible_run_ids)
        .bind(database_run_id)
        .bind(MAX_EQUITY_HISTORY_POINTS as i64)
        .fetch_all(&self.postgres)
        .await?;
        let mut history = rows
            .into_iter()
            .map(
                |(timestamp_ms, equity, realized_pnl, unrealized_pnl, open_positions_count)| {
                    PaperEquityPoint {
                        timestamp_ms,
                        equity,
                        realized_pnl,
                        unrealized_pnl,
                        open_positions_count: usize::try_from(open_positions_count)
                            .unwrap_or_default(),
                    }
                },
            )
            .collect();
        compact_equity_history(&mut history);
        Ok(history)
    }

    pub async fn persist_transition(&self, transition: &PersistedTransition) -> anyhow::Result<()> {
        let identity = transition
            .intent
            .as_ref()
            .map(|intent| intent.identity.clone())
            .unwrap_or_else(|| transition.state.strategy_identity().clone());
        let run_id = transition
            .intent
            .as_ref()
            .map(|intent| intent.run_id.as_str())
            .unwrap_or_else(|| transition.state.run_id());
        let database_run_id = self.database_run_id(run_id);
        let mut transaction = self.postgres.begin().await?;
        let account_version =
            lock_account_row(&mut transaction, &self.scope, transition.committed_at_ms).await?;
        if let Some(intent) = &transition.intent {
            insert_intent(&mut transaction, &self.scope, &database_run_id, intent).await?;
        }
        persist_state_rows(
            &mut transaction,
            &identity,
            &database_run_id,
            &transition.state,
            &transition.snapshot,
            &transition.event_type,
            transition.committed_at_ms,
            &self.scope,
            account_version,
        )
        .await?;
        transaction.commit().await?;
        Ok(())
    }

    pub async fn persist_rejection(&self, intent: &PersistedOrderIntent) -> anyhow::Result<()> {
        let database_run_id = self.database_run_id(&intent.run_id);
        let mut transaction = self.postgres.begin().await?;
        let account_version =
            lock_account_row(&mut transaction, &self.scope, intent.created_at_ms).await?;
        insert_identity_and_run(
            &mut transaction,
            &intent.identity,
            &database_run_id,
            None,
            intent.created_at_ms,
        )
        .await?;
        insert_intent(&mut transaction, &self.scope, &database_run_id, intent).await?;
        advance_account_version(
            &mut transaction,
            &self.scope,
            account_version,
            intent.created_at_ms,
        )
        .await?;
        sqlx::query(
            "INSERT INTO event_log \
             (tenant_id, account_id, account_version, event_type, run_id, version_code, strategy_build_id, aggregate_type, aggregate_id, payload_json, created_at_ms) \
             VALUES ($1, $2, $3, 'order_rejected', $4, $5, $6, 'order_intent', $7, $8, $9)",
        )
        .bind(&self.scope.tenant_id)
        .bind(&self.scope.account_id)
        .bind(account_version + 1)
        .bind(&database_run_id)
        .bind(&intent.identity.version_code)
        .bind(&intent.identity.strategy_build_id)
        .bind(database_client_intent_id(&self.scope, &intent.client_intent_id))
        .bind(Json(serde_json::to_value(intent)?))
        .bind(intent.created_at_ms)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(())
    }

    pub async fn persist_checkpoint(
        &self,
        state: &PaperState,
        snapshot: &PaperAccountSnapshot,
        ts_ms: i64,
    ) -> anyhow::Result<()> {
        let identity = state.strategy_identity();
        let database_run_id = self.database_run_id(state.run_id());
        let mut transaction = self.postgres.begin().await?;
        let account_version = lock_account_row(&mut transaction, &self.scope, ts_ms).await?;
        persist_state_rows(
            &mut transaction,
            identity,
            &database_run_id,
            state,
            snapshot,
            "checkpoint",
            ts_ms,
            &self.scope,
            account_version,
        )
        .await?;
        transaction.commit().await?;
        Ok(())
    }

    pub async fn rebuild_cache(&self, dashboard: &DashboardSnapshot) -> anyhow::Result<()> {
        let Some(client) = &self.redis else {
            return Ok(());
        };
        let mut connection = client.get_multiplexed_async_connection().await?;
        let payload = serde_json::to_string(dashboard)?;
        let key = self.scope.redis_key("dashboard:snapshot");
        connection
            .set_ex::<_, _, ()>(key, payload, self.redis_ttl_secs)
            .await?;
        Ok(())
    }

    pub async fn postgres_ready(&self) -> bool {
        sqlx::query_scalar::<_, i32>("SELECT 1")
            .fetch_one(&self.postgres)
            .await
            .is_ok()
    }

    pub async fn purge_strategy_data(&self, version_codes: &[&str]) -> anyhow::Result<()> {
        let mut transaction = self.postgres.begin().await?;
        purge_strategy_data_in_transaction(&mut transaction, version_codes).await?;
        transaction.commit().await?;
        Ok(())
    }

    pub async fn clear_cache(&self) -> anyhow::Result<()> {
        let Some(client) = &self.redis else {
            return Ok(());
        };
        let mut connection = client.get_multiplexed_async_connection().await?;
        connection
            .del::<_, ()>(self.scope.redis_key("dashboard:snapshot"))
            .await?;
        Ok(())
    }

    pub fn account_scope(&self) -> &AccountScope {
        &self.scope
    }

    pub fn database_run_id(&self, run_id: &str) -> String {
        format!(
            "{}:{}:{run_id}",
            self.scope.tenant_id, self.scope.account_id
        )
    }

    pub async fn export_strategy_backup(
        &self,
        output_root: &Path,
        version_codes: &[&str],
    ) -> anyhow::Result<StrategyBackup> {
        anyhow::ensure!(
            !version_codes.is_empty(),
            "at least one version code is required"
        );
        let created_at_ms = chrono::Utc::now().timestamp_millis();
        let output_dir = output_root.join(format!("strategy-{created_at_ms}"));
        create_owner_only_dir(&output_dir)?;
        let versions: Vec<String> = version_codes
            .iter()
            .map(|value| value.to_string())
            .collect();
        let mut tables: Vec<&str> = STRATEGY_TABLES.to_vec();
        if self.table_exists("risk_guard_events").await? {
            tables.push("risk_guard_events");
        }

        let mut files = Vec::with_capacity(tables.len());
        for table in tables {
            let rows = self.export_table_rows(table, &versions).await?;
            let row_count = rows
                .as_array()
                .map(|rows| rows.len() as i64)
                .ok_or_else(|| {
                    anyhow!("PostgreSQL backup payload for {table} is not a JSON array")
                })?;
            let bytes = serde_json::to_vec_pretty(&rows)?;
            let relative_path = format!("{table}.json");
            write_owner_only_file(&output_dir.join(&relative_path), &bytes)?;
            files.push(StrategyBackupFile {
                table: table.to_string(),
                relative_path,
                row_count,
                sha256: sha256_hex(&bytes),
            });
        }

        let manifest = StrategyBackupManifest {
            created_at_ms,
            version_codes: versions,
            files,
        };
        let manifest_path = output_dir.join("manifest.json");
        write_owner_only_file(&manifest_path, &serde_json::to_vec_pretty(&manifest)?)?;
        Ok(StrategyBackup {
            manifest_path,
            manifest,
        })
    }

    pub async fn reset_restored_v3(
        &self,
        backup_manifest: &Path,
        identity: &StrategyIdentity,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(
            identity == &StrategyIdentity::restored_v3(),
            "reset identity must be the restored v0.1.3 build"
        );
        let manifest = verify_backup_manifest(backup_manifest)?;
        for required in ["v0.1.3", "v0.1.4"] {
            anyhow::ensure!(
                manifest
                    .version_codes
                    .iter()
                    .any(|version| version == required),
                "backup manifest does not cover {required}"
            );
        }

        let state = PaperState::fresh_restored_v3(identity.clone());
        let snapshot = state.snapshot(&BTreeMap::<String, f64>::new());
        let ts_ms = chrono::Utc::now().timestamp_millis();
        let mut transaction = self.postgres.begin().await?;
        purge_strategy_data_in_transaction(
            &mut transaction,
            &manifest
                .version_codes
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
        )
        .await?;
        let account_version = lock_account_row(&mut transaction, &self.scope, ts_ms).await?;
        let database_run_id = self.database_run_id(state.run_id());
        persist_state_rows(
            &mut transaction,
            identity,
            &database_run_id,
            &state,
            &snapshot,
            "reset_restored_v3",
            ts_ms,
            &self.scope,
            account_version,
        )
        .await?;
        transaction.commit().await?;

        if let Err(error) = self.clear_cache().await {
            tracing::warn!(
                ?error,
                "strategy reset committed but Redis cache cleanup failed"
            );
        }
        Ok(())
    }

    pub async fn strategy_row_counts(
        &self,
        version_code: &str,
        run_id: Option<&str>,
    ) -> anyhow::Result<BTreeMap<String, i64>> {
        let database_run_id = run_id.map(|value| self.database_run_id(value));
        let mut tables: Vec<&str> = STRATEGY_TABLES.to_vec();
        if self.table_exists("risk_guard_events").await? {
            tables.push("risk_guard_events");
        }
        let mut counts = BTreeMap::new();
        for table in tables {
            let count = if table == "strategy_versions" {
                if let Some(run_id) = database_run_id.as_deref() {
                    sqlx::query_scalar::<_, i64>(
                        "SELECT COUNT(*) FROM strategy_versions versions \
                         WHERE versions.version_code = $1 AND EXISTS (SELECT 1 FROM strategy_runs runs \
                         WHERE runs.version_code = versions.version_code AND runs.run_id = $2)",
                    )
                    .bind(version_code)
                    .bind(run_id)
                    .fetch_one(&self.postgres)
                    .await?
                } else {
                    sqlx::query_scalar::<_, i64>(
                        "SELECT COUNT(*) FROM strategy_versions WHERE version_code = $1",
                    )
                    .bind(version_code)
                    .fetch_one(&self.postgres)
                    .await?
                }
            } else if let Some(run_id) = database_run_id.as_deref() {
                let statement =
                    format!("SELECT COUNT(*) FROM {table} WHERE version_code = $1 AND run_id = $2");
                sqlx::query_scalar::<_, i64>(&statement)
                    .bind(version_code)
                    .bind(run_id)
                    .fetch_one(&self.postgres)
                    .await?
            } else {
                let statement = format!("SELECT COUNT(*) FROM {table} WHERE version_code = $1");
                sqlx::query_scalar::<_, i64>(&statement)
                    .bind(version_code)
                    .fetch_one(&self.postgres)
                    .await?
            };
            counts.insert(table.to_string(), count);
        }
        Ok(counts)
    }

    async fn table_exists(&self, table: &str) -> anyhow::Result<bool> {
        let relation = format!("public.{table}");
        Ok(
            sqlx::query_scalar::<_, bool>("SELECT to_regclass($1) IS NOT NULL")
                .bind(relation)
                .fetch_one(&self.postgres)
                .await?,
        )
    }

    async fn export_table_rows(
        &self,
        table: &str,
        version_codes: &[String],
    ) -> anyhow::Result<serde_json::Value> {
        let filter = if table == "app_state_snapshots" {
            "version_code = ANY($1) OR version_code IS NULL"
        } else {
            "version_code = ANY($1)"
        };
        let statement = format!(
            "SELECT COALESCE(jsonb_agg(to_jsonb(selected_rows)), '[]'::jsonb) \
             FROM (SELECT * FROM {table} WHERE {filter}) selected_rows"
        );
        let Json(rows) = sqlx::query_scalar::<_, Json<serde_json::Value>>(&statement)
            .bind(version_codes)
            .fetch_one(&self.postgres)
            .await?;
        Ok(rows)
    }
}

async fn purge_strategy_data_in_transaction(
    transaction: &mut Transaction<'_, Postgres>,
    version_codes: &[&str],
) -> anyhow::Result<()> {
    for version_code in version_codes {
        for table in [
            "fills",
            "positions",
            "closed_trades",
            "equity_snapshots",
            "ledger_entries",
            "order_intents",
        ] {
            let statement = format!("DELETE FROM {table} WHERE version_code = $1");
            sqlx::query(&statement)
                .bind(version_code)
                .execute(&mut **transaction)
                .await?;
        }
        let risk_guard_exists = sqlx::query_scalar::<_, bool>(
            "SELECT to_regclass('public.risk_guard_events') IS NOT NULL",
        )
        .fetch_one(&mut **transaction)
        .await?;
        if risk_guard_exists {
            sqlx::query("DELETE FROM risk_guard_events WHERE version_code = $1")
                .bind(version_code)
                .execute(&mut **transaction)
                .await?;
        }
        sqlx::query("DELETE FROM event_log WHERE version_code = $1")
            .bind(version_code)
            .execute(&mut **transaction)
            .await?;
        sqlx::query("DELETE FROM app_state_snapshots WHERE version_code = $1")
            .bind(version_code)
            .execute(&mut **transaction)
            .await?;
        sqlx::query("DELETE FROM strategy_runs WHERE version_code = $1")
            .bind(version_code)
            .execute(&mut **transaction)
            .await?;
        sqlx::query("DELETE FROM strategy_versions WHERE version_code = $1")
            .bind(version_code)
            .execute(&mut **transaction)
            .await?;
    }
    sqlx::query("DELETE FROM app_state_snapshots WHERE version_code IS NULL")
        .execute(&mut **transaction)
        .await?;
    Ok(())
}

fn verify_backup_manifest(path: &Path) -> anyhow::Result<StrategyBackupManifest> {
    let bytes = fs::read(path)
        .with_context(|| format!("failed to read backup manifest {}", path.display()))?;
    let manifest: StrategyBackupManifest = serde_json::from_slice(&bytes)?;
    let parent = path
        .parent()
        .ok_or_else(|| anyhow!("backup manifest has no parent directory"))?;
    anyhow::ensure!(
        !manifest.files.is_empty(),
        "backup manifest contains no table exports"
    );
    let expected_tables: BTreeSet<&str> = STRATEGY_TABLES.iter().copied().collect();
    let exported_tables: BTreeSet<&str> = manifest
        .files
        .iter()
        .map(|file| file.table.as_str())
        .collect();
    anyhow::ensure!(
        expected_tables.is_subset(&exported_tables),
        "backup manifest is missing required strategy tables"
    );
    anyhow::ensure!(
        exported_tables.len() == manifest.files.len(),
        "backup manifest contains duplicate table exports"
    );
    for file in &manifest.files {
        anyhow::ensure!(
            expected_tables.contains(file.table.as_str()) || file.table == "risk_guard_events",
            "unexpected backup table {}",
            file.table
        );
        anyhow::ensure!(
            file.relative_path == format!("{}.json", file.table),
            "backup file name does not match table {}",
            file.table
        );
        let relative = Path::new(&file.relative_path);
        anyhow::ensure!(
            relative.components().count() == 1 && relative.file_name().is_some(),
            "unsafe backup file path {}",
            file.relative_path
        );
        let payload = fs::read(parent.join(relative))?;
        anyhow::ensure!(
            sha256_hex(&payload) == file.sha256,
            "backup hash mismatch for {}",
            file.relative_path
        );
        let rows: Vec<serde_json::Value> = serde_json::from_slice(&payload)?;
        anyhow::ensure!(
            rows.len() as i64 == file.row_count,
            "backup row count mismatch for {}",
            file.relative_path
        );
    }
    Ok(manifest)
}

fn create_owner_only_dir(path: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

fn write_owner_only_file(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    let mut options = fs::OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

async fn insert_intent(
    transaction: &mut Transaction<'_, Postgres>,
    scope: &AccountScope,
    database_run_id: &str,
    intent: &PersistedOrderIntent,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO order_intents \
         (client_intent_id, run_id, version_code, strategy_build_id, config_hash, symbol, side, action, score, \
          primary_signal, reason, tags_json, status, rejection_reason, created_at_ms) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15) \
         ON CONFLICT (client_intent_id) DO UPDATE SET \
         status = EXCLUDED.status, rejection_reason = EXCLUDED.rejection_reason",
    )
    .bind(database_client_intent_id(scope, &intent.client_intent_id))
    .bind(database_run_id)
    .bind(&intent.identity.version_code)
    .bind(&intent.identity.strategy_build_id)
    .bind(&intent.identity.config_hash)
    .bind(&intent.symbol)
    .bind(side_name(intent.side))
    .bind(action_name(intent.action))
    .bind(intent.score as i32)
    .bind(&intent.primary_signal)
    .bind(&intent.reason)
    .bind(Json(json!(intent.tags)))
    .bind(&intent.status)
    .bind(&intent.rejection_reason)
    .bind(intent.created_at_ms)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

fn database_client_intent_id(scope: &AccountScope, client_intent_id: &str) -> String {
    format!(
        "{}:{}:{client_intent_id}",
        scope.tenant_id, scope.account_id
    )
}

#[allow(clippy::too_many_arguments)]
async fn persist_state_rows(
    transaction: &mut Transaction<'_, Postgres>,
    identity: &StrategyIdentity,
    run_id: &str,
    state: &PaperState,
    snapshot: &PaperAccountSnapshot,
    event_type: &str,
    ts_ms: i64,
    scope: &AccountScope,
    account_version: i64,
) -> anyhow::Result<()> {
    insert_identity_and_run(transaction, identity, run_id, Some(snapshot), ts_ms).await?;

    sqlx::query("DELETE FROM positions WHERE run_id = $1 AND status = 'open'")
        .bind(run_id)
        .execute(&mut **transaction)
        .await?;
    for position in &snapshot.positions {
        sqlx::query(
            "INSERT INTO positions \
             (position_key, run_id, version_code, strategy_build_id, symbol, side, entry_price, mark_price, \
              margin, leverage, quantity, unrealized_pnl, stop_loss, take_profit, expire_at_ms, primary_signal, \
              reason, tags_json, status, opened_at_ms, closed_at_ms) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, 'open', $19, NULL) \
             ON CONFLICT (position_key) DO UPDATE SET mark_price = EXCLUDED.mark_price, \
             unrealized_pnl = EXCLUDED.unrealized_pnl, stop_loss = EXCLUDED.stop_loss, \
             take_profit = EXCLUDED.take_profit, expire_at_ms = EXCLUDED.expire_at_ms, status = 'open'",
        )
        .bind(format!("{run_id}:{}", position.inst_id))
        .bind(run_id)
        .bind(&identity.version_code)
        .bind(&identity.strategy_build_id)
        .bind(&position.inst_id)
        .bind(side_name(position.side))
        .bind(decimal_or_zero(position.entry_price))
        .bind(decimal_or_zero(position.mark_price))
        .bind(decimal_or_zero(position.margin))
        .bind(decimal_or_zero(position.leverage))
        .bind(decimal_or_zero(position.qty))
        .bind(decimal_or_zero(position.unrealized_pnl))
        .bind(position.stop_loss.map(decimal_or_zero))
        .bind(position.take_profit.map(decimal_or_zero))
        .bind(position.expire_at_ms)
        .bind(&position.primary_signal)
        .bind(&position.reason)
        .bind(Json(json!(position.tags)))
        .bind(position.opened_at_ms)
        .execute(&mut **transaction)
        .await?;
    }

    for trade in &snapshot.trades {
        sqlx::query(
            "INSERT INTO fills \
             (trade_id, order_intent_id, run_id, version_code, strategy_build_id, symbol, side, action, \
              price, quantity, fee, slippage, trigger_price, actual_slippage_rate, filled_at_ms) \
             VALUES ($1, (SELECT id FROM order_intents WHERE client_intent_id = \
                 $2 || ':trade:' || $1::text || ':' || $7), $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14) \
             ON CONFLICT (run_id, trade_id) DO UPDATE SET price = EXCLUDED.price, \
             quantity = EXCLUDED.quantity, fee = EXCLUDED.fee, slippage = EXCLUDED.slippage, \
             trigger_price = EXCLUDED.trigger_price, actual_slippage_rate = EXCLUDED.actual_slippage_rate",
        )
        .bind(trade.id as i64)
        .bind(run_id)
        .bind(&identity.version_code)
        .bind(&identity.strategy_build_id)
        .bind(&trade.inst_id)
        .bind(side_name(trade.side))
        .bind(action_name(trade.action))
        .bind(decimal_or_zero(trade.price))
        .bind(decimal_or_zero(trade.qty))
        .bind(decimal_or_zero(trade.fee))
        .bind(decimal_or_zero(trade.slippage_rate))
        .bind(trade.trigger_price.map(decimal_or_zero))
        .bind(trade.actual_slippage_rate.map(decimal_or_zero))
        .bind(trade.ts_ms)
        .execute(&mut **transaction)
        .await?;

        if trade.fee != 0.0 {
            insert_ledger_entry(
                transaction,
                scope,
                identity,
                run_id,
                trade,
                "fee",
                -trade.fee,
            )
            .await?;
        }
        if trade.action == PaperTradeAction::Close {
            insert_ledger_entry(
                transaction,
                scope,
                identity,
                run_id,
                trade,
                "realized_pnl",
                trade.realized_pnl + trade.fee,
            )
            .await?;
        }
    }

    for closed in &snapshot.position_history {
        sqlx::query(
            "INSERT INTO closed_trades \
             (closed_position_id, run_id, version_code, strategy_build_id, symbol, side, entry_price, exit_price, \
              margin, leverage, quantity, gross_pnl, fee, net_pnl, primary_signal, tags_json, exit_reason, \
              stop_loss, take_profit, expire_at_ms, trigger_price, actual_slippage_rate, hold_seconds, opened_at_ms, closed_at_ms) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25) \
             ON CONFLICT (run_id, closed_position_id) DO UPDATE SET exit_price = EXCLUDED.exit_price, \
             net_pnl = EXCLUDED.net_pnl, exit_reason = EXCLUDED.exit_reason, \
             trigger_price = EXCLUDED.trigger_price, actual_slippage_rate = EXCLUDED.actual_slippage_rate",
        )
        .bind(closed.id as i64)
        .bind(run_id)
        .bind(&identity.version_code)
        .bind(&identity.strategy_build_id)
        .bind(&closed.inst_id)
        .bind(side_name(closed.side))
        .bind(decimal_or_zero(closed.entry_price))
        .bind(decimal_or_zero(closed.exit_price))
        .bind(decimal_or_zero(closed.margin))
        .bind(decimal_or_zero(closed.leverage))
        .bind(decimal_or_zero(closed.qty))
        .bind(decimal_or_zero(closed.realized_pnl + closed.fees))
        .bind(decimal_or_zero(closed.fees))
        .bind(decimal_or_zero(closed.realized_pnl))
        .bind(&closed.primary_signal)
        .bind(Json(json!(closed.tags)))
        .bind(&closed.close_reason)
        .bind(closed.stop_loss.map(decimal_or_zero))
        .bind(closed.take_profit.map(decimal_or_zero))
        .bind(closed.expire_at_ms)
        .bind(closed.trigger_price.map(decimal_or_zero))
        .bind(closed.actual_slippage_rate.map(decimal_or_zero))
        .bind(closed.duration_ms / 1_000)
        .bind(closed.opened_at_ms)
        .bind(closed.closed_at_ms)
        .execute(&mut **transaction)
        .await?;
    }

    sqlx::query(
        "INSERT INTO equity_snapshots \
         (run_id, version_code, strategy_build_id, timestamp_ms, equity, realized_pnl, unrealized_pnl, drawdown, open_positions_count) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
         ON CONFLICT (run_id, timestamp_ms) DO UPDATE SET equity = EXCLUDED.equity, \
         realized_pnl = EXCLUDED.realized_pnl, unrealized_pnl = EXCLUDED.unrealized_pnl, \
         drawdown = EXCLUDED.drawdown, open_positions_count = EXCLUDED.open_positions_count",
    )
    .bind(run_id)
    .bind(&identity.version_code)
    .bind(&identity.strategy_build_id)
    .bind(ts_ms)
    .bind(decimal_or_zero(snapshot.equity))
    .bind(decimal_or_zero(snapshot.realized_pnl))
    .bind(decimal_or_zero(snapshot.unrealized_pnl))
    .bind(decimal_or_zero((snapshot.initial_balance - snapshot.equity).max(0.0)))
    .bind(snapshot.positions.len() as i64)
    .execute(&mut **transaction)
    .await?;

    let next_account_version =
        advance_account_version(transaction, scope, account_version, ts_ms).await?;
    let state_json = serde_json::to_value(state)?;
    sqlx::query(
        "INSERT INTO event_log \
         (tenant_id, account_id, account_version, event_type, run_id, version_code, strategy_build_id, aggregate_type, aggregate_id, payload_json, created_at_ms) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, 'paper_account', $5, $8, $9)",
    )
    .bind(&scope.tenant_id)
    .bind(&scope.account_id)
    .bind(next_account_version)
    .bind(event_type)
    .bind(run_id)
    .bind(&identity.version_code)
    .bind(&identity.strategy_build_id)
    .bind(Json(json!({"paper": snapshot, "identity": identity})))
    .bind(ts_ms)
    .execute(&mut **transaction)
    .await?;
    sqlx::query(
        "INSERT INTO app_state_snapshots \
         (tenant_id, account_id, snapshot_key, run_id, version_code, strategy_build_id, config_hash, payload_json, created_at_ms) \
         VALUES ($1, $2, 'paper_state', $3, $4, $5, $6, $7, $8)",
    )
    .bind(&scope.tenant_id)
    .bind(&scope.account_id)
    .bind(run_id)
    .bind(&identity.version_code)
    .bind(&identity.strategy_build_id)
    .bind(&identity.config_hash)
    .bind(Json(state_json))
    .bind(ts_ms)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

async fn lock_account_row(
    transaction: &mut Transaction<'_, Postgres>,
    scope: &AccountScope,
    ts_ms: i64,
) -> anyhow::Result<i64> {
    sqlx::query(
        "INSERT INTO trading_accounts (tenant_id, account_id, account_version, created_at_ms, updated_at_ms) \
         VALUES ($1, $2, 0, $3, $3) ON CONFLICT (tenant_id, account_id) DO NOTHING",
    )
    .bind(&scope.tenant_id)
    .bind(&scope.account_id)
    .bind(ts_ms)
    .execute(&mut **transaction)
    .await?;
    let version = sqlx::query_scalar::<_, i64>(
        "SELECT account_version FROM trading_accounts \
         WHERE tenant_id = $1 AND account_id = $2 FOR UPDATE",
    )
    .bind(&scope.tenant_id)
    .bind(&scope.account_id)
    .fetch_one(&mut **transaction)
    .await?;
    anyhow::ensure!(version >= 0, "account_version must not be negative");
    Ok(version)
}

async fn advance_account_version(
    transaction: &mut Transaction<'_, Postgres>,
    scope: &AccountScope,
    expected_version: i64,
    ts_ms: i64,
) -> anyhow::Result<i64> {
    let next_version = expected_version
        .checked_add(1)
        .ok_or_else(|| anyhow!("account_version overflow"))?;
    let result = sqlx::query(
        "UPDATE trading_accounts SET account_version = $1, updated_at_ms = $2 \
         WHERE tenant_id = $3 AND account_id = $4 AND account_version = $5",
    )
    .bind(next_version)
    .bind(ts_ms)
    .bind(&scope.tenant_id)
    .bind(&scope.account_id)
    .bind(expected_version)
    .execute(&mut **transaction)
    .await?;
    anyhow::ensure!(
        result.rows_affected() == 1,
        "stale account_version: expected {expected_version}"
    );
    Ok(next_version)
}

#[allow(clippy::too_many_arguments)]
async fn insert_ledger_entry(
    transaction: &mut Transaction<'_, Postgres>,
    scope: &AccountScope,
    identity: &StrategyIdentity,
    run_id: &str,
    trade: &PaperTrade,
    entry_type: &str,
    amount: f64,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO ledger_entries \
         (tenant_id, account_id, run_id, version_code, strategy_build_id, trade_id, entry_type, amount, created_at_ms) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
         ON CONFLICT (tenant_id, account_id, run_id, trade_id, entry_type) DO NOTHING",
    )
    .bind(&scope.tenant_id)
    .bind(&scope.account_id)
    .bind(run_id)
    .bind(&identity.version_code)
    .bind(&identity.strategy_build_id)
    .bind(trade.id as i64)
    .bind(entry_type)
    .bind(decimal_or_zero(amount))
    .bind(trade.ts_ms)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

async fn insert_identity_and_run(
    transaction: &mut Transaction<'_, Postgres>,
    identity: &StrategyIdentity,
    run_id: &str,
    snapshot: Option<&PaperAccountSnapshot>,
    ts_ms: i64,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO strategy_versions \
         (version_code, strategy_build_id, name, description, status, config_json, config_hash, created_at_ms, updated_at_ms) \
         VALUES ($1, $2, 'Scalping Optimization Design', 'Restored automatic v3 strategy', 'active', $3, $4, $5, $5) \
         ON CONFLICT (version_code) DO UPDATE SET strategy_build_id = EXCLUDED.strategy_build_id, \
         config_json = EXCLUDED.config_json, config_hash = EXCLUDED.config_hash, updated_at_ms = EXCLUDED.updated_at_ms",
    )
    .bind(&identity.version_code)
    .bind(&identity.strategy_build_id)
    .bind(Json(json!({"config_hash": identity.config_hash})))
    .bind(&identity.config_hash)
    .bind(ts_ms)
    .execute(&mut **transaction)
    .await?;

    let initial = snapshot
        .map(|value| value.initial_balance)
        .unwrap_or(10_000.0);
    let equity = snapshot.map(|value| value.equity).unwrap_or(initial);
    let realized = snapshot.map(|value| value.realized_pnl).unwrap_or(0.0);
    let unrealized = snapshot.map(|value| value.unrealized_pnl).unwrap_or(0.0);
    let fees = snapshot.map(|value| value.total_fees).unwrap_or(0.0);
    let statement = if snapshot.is_some() {
        "INSERT INTO strategy_runs \
         (run_id, version_code, strategy_build_id, config_hash, mode, initial_equity, current_equity, \
          realized_pnl, unrealized_pnl, fee_total, max_drawdown, status, start_time_ms, end_time_ms, \
          fee_model, slippage_model, config_snapshot) \
         VALUES ($1, $2, $3, $4, 'paper', $5, $6, $7, $8, $9, $10, 'running', $11, NULL, '0.05% per fill', '0.02% adverse', $12) \
         ON CONFLICT (run_id) DO UPDATE SET current_equity = EXCLUDED.current_equity, \
         realized_pnl = EXCLUDED.realized_pnl, unrealized_pnl = EXCLUDED.unrealized_pnl, \
         fee_total = EXCLUDED.fee_total, max_drawdown = EXCLUDED.max_drawdown, status = 'running'"
    } else {
        "INSERT INTO strategy_runs \
         (run_id, version_code, strategy_build_id, config_hash, mode, initial_equity, current_equity, \
          realized_pnl, unrealized_pnl, fee_total, max_drawdown, status, start_time_ms, end_time_ms, \
          fee_model, slippage_model, config_snapshot) \
         VALUES ($1, $2, $3, $4, 'paper', $5, $6, $7, $8, $9, $10, 'running', $11, NULL, '0.05% per fill', '0.02% adverse', $12) \
         ON CONFLICT (run_id) DO NOTHING"
    };
    sqlx::query(statement)
        .bind(run_id)
        .bind(&identity.version_code)
        .bind(&identity.strategy_build_id)
        .bind(&identity.config_hash)
        .bind(decimal_or_zero(initial))
        .bind(decimal_or_zero(equity))
        .bind(decimal_or_zero(realized))
        .bind(decimal_or_zero(unrealized))
        .bind(decimal_or_zero(fees))
        .bind(decimal_or_zero((initial - equity).max(0.0)))
        .bind(ts_ms)
        .bind(Json(json!({"config_hash": identity.config_hash})))
        .execute(&mut **transaction)
        .await?;
    Ok(())
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

fn side_name(side: PaperSide) -> &'static str {
    match side {
        PaperSide::Long => "long",
        PaperSide::Short => "short",
    }
}

fn action_name(action: PaperTradeAction) -> &'static str {
    match action {
        PaperTradeAction::Open => "open",
        PaperTradeAction::Close => "close",
    }
}

pub fn postgres_schema_statements() -> Vec<&'static str> {
    vec![
        "CREATE TABLE IF NOT EXISTS trading_accounts (
            tenant_id TEXT NOT NULL,
            account_id TEXT NOT NULL,
            account_version BIGINT NOT NULL DEFAULT 0,
            created_at_ms BIGINT NOT NULL,
            updated_at_ms BIGINT NOT NULL,
            PRIMARY KEY (tenant_id, account_id)
        )",
        "CREATE TABLE IF NOT EXISTS strategy_versions (
            version_code TEXT PRIMARY KEY,
            strategy_build_id TEXT NOT NULL,
            name TEXT NOT NULL,
            description TEXT NOT NULL,
            status TEXT NOT NULL,
            config_json JSONB NOT NULL,
            config_hash TEXT NOT NULL,
            created_at_ms BIGINT NOT NULL,
            updated_at_ms BIGINT NOT NULL
        )",
        "ALTER TABLE strategy_versions ADD COLUMN IF NOT EXISTS strategy_build_id TEXT",
        "CREATE TABLE IF NOT EXISTS strategy_runs (
            run_id TEXT PRIMARY KEY,
            version_code TEXT NOT NULL REFERENCES strategy_versions(version_code),
            strategy_build_id TEXT NOT NULL,
            config_hash TEXT NOT NULL,
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
        "ALTER TABLE strategy_runs ADD COLUMN IF NOT EXISTS strategy_build_id TEXT",
        "ALTER TABLE strategy_runs ADD COLUMN IF NOT EXISTS config_hash TEXT",
        "CREATE TABLE IF NOT EXISTS order_intents (
            id BIGSERIAL PRIMARY KEY,
            client_intent_id TEXT UNIQUE NOT NULL,
            run_id TEXT NOT NULL,
            version_code TEXT NOT NULL,
            strategy_build_id TEXT NOT NULL,
            config_hash TEXT NOT NULL,
            symbol TEXT NOT NULL,
            side TEXT NOT NULL,
            action TEXT NOT NULL,
            score INTEGER NOT NULL,
            primary_signal TEXT NOT NULL,
            reason TEXT NOT NULL,
            tags_json JSONB NOT NULL,
            status TEXT NOT NULL,
            rejection_reason TEXT,
            created_at_ms BIGINT NOT NULL
        )",
        "CREATE TABLE IF NOT EXISTS fills (
            id BIGSERIAL PRIMARY KEY,
            trade_id BIGINT NOT NULL,
            order_intent_id BIGINT REFERENCES order_intents(id),
            run_id TEXT NOT NULL,
            version_code TEXT NOT NULL,
            strategy_build_id TEXT NOT NULL,
            symbol TEXT NOT NULL,
            side TEXT NOT NULL,
            action TEXT NOT NULL,
            price NUMERIC NOT NULL,
            quantity NUMERIC NOT NULL,
            fee NUMERIC NOT NULL,
            slippage NUMERIC NOT NULL,
            trigger_price NUMERIC,
            actual_slippage_rate NUMERIC,
            filled_at_ms BIGINT NOT NULL,
            UNIQUE (run_id, trade_id)
        )",
        "ALTER TABLE fills ADD COLUMN IF NOT EXISTS trigger_price NUMERIC",
        "ALTER TABLE fills ADD COLUMN IF NOT EXISTS actual_slippage_rate NUMERIC",
        "CREATE TABLE IF NOT EXISTS positions (
            position_key TEXT PRIMARY KEY,
            run_id TEXT NOT NULL,
            version_code TEXT NOT NULL,
            strategy_build_id TEXT NOT NULL,
            symbol TEXT NOT NULL,
            side TEXT NOT NULL,
            entry_price NUMERIC NOT NULL,
            mark_price NUMERIC NOT NULL,
            margin NUMERIC NOT NULL,
            leverage NUMERIC NOT NULL,
            quantity NUMERIC NOT NULL,
            unrealized_pnl NUMERIC NOT NULL,
            stop_loss NUMERIC,
            take_profit NUMERIC,
            expire_at_ms BIGINT,
            primary_signal TEXT NOT NULL,
            reason TEXT NOT NULL,
            tags_json JSONB NOT NULL,
            status TEXT NOT NULL,
            opened_at_ms BIGINT NOT NULL,
            closed_at_ms BIGINT
        )",
        "CREATE TABLE IF NOT EXISTS closed_trades (
            id BIGSERIAL PRIMARY KEY,
            closed_position_id BIGINT NOT NULL,
            run_id TEXT NOT NULL,
            version_code TEXT NOT NULL,
            strategy_build_id TEXT NOT NULL,
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
            stop_loss NUMERIC,
            take_profit NUMERIC,
            expire_at_ms BIGINT,
            trigger_price NUMERIC,
            actual_slippage_rate NUMERIC,
            hold_seconds BIGINT NOT NULL,
            opened_at_ms BIGINT NOT NULL,
            closed_at_ms BIGINT NOT NULL,
            UNIQUE (run_id, closed_position_id)
        )",
        "ALTER TABLE closed_trades ADD COLUMN IF NOT EXISTS trigger_price NUMERIC",
        "ALTER TABLE closed_trades ADD COLUMN IF NOT EXISTS actual_slippage_rate NUMERIC",
        "CREATE TABLE IF NOT EXISTS equity_snapshots (
            id BIGSERIAL PRIMARY KEY,
            run_id TEXT NOT NULL,
            version_code TEXT NOT NULL,
            strategy_build_id TEXT NOT NULL,
            timestamp_ms BIGINT NOT NULL,
            equity NUMERIC NOT NULL,
            realized_pnl NUMERIC NOT NULL,
            unrealized_pnl NUMERIC NOT NULL,
            drawdown NUMERIC NOT NULL,
            open_positions_count BIGINT NOT NULL,
            UNIQUE (run_id, timestamp_ms)
        )",
        "CREATE TABLE IF NOT EXISTS ledger_entries (
            id BIGSERIAL PRIMARY KEY,
            tenant_id TEXT NOT NULL,
            account_id TEXT NOT NULL,
            run_id TEXT NOT NULL,
            version_code TEXT NOT NULL,
            strategy_build_id TEXT NOT NULL,
            trade_id BIGINT NOT NULL,
            entry_type TEXT NOT NULL,
            amount NUMERIC NOT NULL,
            created_at_ms BIGINT NOT NULL,
            UNIQUE (tenant_id, account_id, run_id, trade_id, entry_type)
        )",
        "CREATE TABLE IF NOT EXISTS event_log (
            id BIGSERIAL PRIMARY KEY,
            tenant_id TEXT NOT NULL DEFAULT 'default',
            account_id TEXT NOT NULL DEFAULT 'paper',
            account_version BIGINT,
            event_type TEXT NOT NULL,
            run_id TEXT,
            version_code TEXT,
            strategy_build_id TEXT,
            aggregate_type TEXT NOT NULL,
            aggregate_id TEXT NOT NULL,
            payload_json JSONB NOT NULL,
            created_at_ms BIGINT NOT NULL
        )",
        "ALTER TABLE event_log ADD COLUMN IF NOT EXISTS tenant_id TEXT NOT NULL DEFAULT 'default'",
        "ALTER TABLE event_log ADD COLUMN IF NOT EXISTS account_id TEXT NOT NULL DEFAULT 'paper'",
        "ALTER TABLE event_log ADD COLUMN IF NOT EXISTS account_version BIGINT",
        "CREATE UNIQUE INDEX IF NOT EXISTS event_log_account_version_idx ON event_log
            (tenant_id, account_id, account_version) WHERE account_version IS NOT NULL",
        "CREATE TABLE IF NOT EXISTS app_state_snapshots (
            id BIGSERIAL PRIMARY KEY,
            tenant_id TEXT NOT NULL DEFAULT 'default',
            account_id TEXT NOT NULL DEFAULT 'paper',
            snapshot_key TEXT NOT NULL,
            run_id TEXT,
            version_code TEXT,
            strategy_build_id TEXT,
            config_hash TEXT,
            payload_json JSONB NOT NULL,
            created_at_ms BIGINT NOT NULL DEFAULT (EXTRACT(EPOCH FROM NOW()) * 1000)::BIGINT
        )",
        "ALTER TABLE app_state_snapshots ADD COLUMN IF NOT EXISTS tenant_id TEXT NOT NULL DEFAULT 'default'",
        "ALTER TABLE app_state_snapshots ADD COLUMN IF NOT EXISTS account_id TEXT NOT NULL DEFAULT 'paper'",
        "CREATE INDEX IF NOT EXISTS app_state_tenant_identity_idx ON app_state_snapshots
            (tenant_id, account_id, version_code, strategy_build_id, config_hash, id DESC)",
    ]
}
