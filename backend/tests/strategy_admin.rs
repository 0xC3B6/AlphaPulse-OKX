use std::collections::BTreeMap;

use alphapulse_okx_backend::{
    config::AppConfig,
    paper::PaperState,
    persistence::PersistenceLayer,
    strategy_identity::{StrategyIdentity, INITIAL_RUN_ID},
};

#[tokio::test]
#[ignore = "requires docker compose PostgreSQL and Redis"]
async fn verified_backup_precedes_single_restored_v3_reset() -> anyhow::Result<()> {
    let database_url = std::env::var("ALPHAPULSE_TEST_DATABASE_URL")?;
    let redis_url = std::env::var("ALPHAPULSE_TEST_REDIS_URL")?;
    let config = AppConfig::from_env_pairs([
        ("ALPHAPULSE_DATABASE_URL", database_url.as_str()),
        ("ALPHAPULSE_REDIS_URL", redis_url.as_str()),
        ("ALPHAPULSE_REQUIRE_DATABASE", "true"),
    ]);
    let persistence = PersistenceLayer::connect_required(&config).await?;
    persistence.initialize().await?;
    persistence
        .purge_strategy_data(&["v0.1.3", "v0.1.4"])
        .await?;

    let old_v3 = state_with_identity_and_run(
        StrategyIdentity {
            version_code: "v0.1.3".to_string(),
            strategy_build_id: "simplified-v3".to_string(),
            config_hash: "old-v3-config".to_string(),
        },
        "old-simplified-v3",
    );
    persistence
        .persist_checkpoint(
            &old_v3,
            &old_v3.snapshot(&BTreeMap::<String, f64>::new()),
            1,
        )
        .await?;
    let old_v4 = state_with_identity_and_run(
        StrategyIdentity {
            version_code: "v0.1.4".to_string(),
            strategy_build_id: "old-v4".to_string(),
            config_hash: "old-v4-config".to_string(),
        },
        "old-v4-run",
    );
    persistence
        .persist_checkpoint(
            &old_v4,
            &old_v4.snapshot(&BTreeMap::<String, f64>::new()),
            2,
        )
        .await?;

    let output_root = std::env::temp_dir().join(format!(
        "alphapulse-strategy-backup-{}",
        chrono::Utc::now().timestamp_millis()
    ));
    let backup = persistence
        .export_strategy_backup(&output_root, &["v0.1.3", "v0.1.4"])
        .await?;
    assert_eq!(
        backup.manifest.version_codes,
        vec!["v0.1.3".to_string(), "v0.1.4".to_string()]
    );
    assert!(backup
        .manifest
        .files
        .iter()
        .all(|file| file.sha256.len() == 64));
    persistence
        .reset_restored_v3(&backup.manifest_path, &StrategyIdentity::restored_v3())
        .await?;

    assert!(persistence
        .strategy_row_counts("v0.1.4", None)
        .await?
        .values()
        .all(|count| *count == 0));
    assert!(persistence
        .strategy_row_counts("v0.1.3", Some("old-simplified-v3"))
        .await?
        .values()
        .all(|count| *count == 0));
    assert_eq!(
        persistence
            .strategy_row_counts("v0.1.3", Some(INITIAL_RUN_ID))
            .await?["strategy_runs"],
        1
    );
    std::fs::remove_dir_all(output_root)?;
    Ok(())
}

fn state_with_identity_and_run(identity: StrategyIdentity, run_id: &str) -> PaperState {
    let mut value = serde_json::to_value(PaperState::fresh_restored_v3(identity)).unwrap();
    value["run_id"] = serde_json::Value::String(run_id.to_string());
    serde_json::from_value(value).unwrap()
}
