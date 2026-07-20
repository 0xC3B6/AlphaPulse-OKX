use std::path::PathBuf;

use alphapulse_okx_backend::{
    config::AppConfig, persistence::PersistenceLayer, strategy_identity::StrategyIdentity,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let command = parse_command(std::env::args().skip(1))?;
    let config = AppConfig::load();
    let persistence = PersistenceLayer::connect_required(&config).await?;
    persistence.initialize().await?;

    match command {
        Command::Backup { output } => {
            let backup = persistence
                .export_strategy_backup(&output, &["v0.1.3", "v0.1.4"])
                .await?;
            println!(
                "verified strategy backup: {}",
                backup.manifest_path.display()
            );
        }
        Command::ResetRestoredV3 { backup_manifest } => {
            persistence
                .reset_restored_v3(&backup_manifest, &StrategyIdentity::restored_v3())
                .await?;
            println!(
                "restored v0.1.3 reset committed after verifying {}",
                backup_manifest.display()
            );
        }
    }
    Ok(())
}

enum Command {
    Backup { output: PathBuf },
    ResetRestoredV3 { backup_manifest: PathBuf },
}

fn parse_command(mut args: impl Iterator<Item = String>) -> anyhow::Result<Command> {
    let command = args.next().ok_or_else(|| anyhow::anyhow!(usage()))?;
    let flag = args.next().ok_or_else(|| anyhow::anyhow!(usage()))?;
    let value = args.next().ok_or_else(|| anyhow::anyhow!(usage()))?;
    anyhow::ensure!(args.next().is_none(), usage());
    match (command.as_str(), flag.as_str()) {
        ("backup", "--output") => Ok(Command::Backup {
            output: PathBuf::from(value),
        }),
        ("reset-restored-v3", "--backup-manifest") => Ok(Command::ResetRestoredV3 {
            backup_manifest: PathBuf::from(value),
        }),
        _ => anyhow::bail!(usage()),
    }
}

fn usage() -> &'static str {
    "usage: strategy_admin backup --output <directory> | strategy_admin reset-restored-v3 --backup-manifest <manifest.json>"
}
