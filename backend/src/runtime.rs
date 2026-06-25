use crate::{config::AppConfig, state::RadarState};

pub async fn run_scanner(_config: AppConfig, _state: RadarState) -> anyhow::Result<()> {
    Ok(())
}
