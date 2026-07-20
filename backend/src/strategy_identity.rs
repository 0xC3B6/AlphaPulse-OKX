use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::auto_strategy::AutoStrategyConfig;

pub const STRATEGY_VERSION_CODE: &str = "v0.1.3";
pub const STRATEGY_BUILD_ID: &str = "legacy-v3-replay-2026-07-10";
pub const INITIAL_RUN_ID: &str = "v0.1.3-restored-paper-1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StrategyIdentity {
    pub version_code: String,
    pub strategy_build_id: String,
    pub config_hash: String,
}

impl StrategyIdentity {
    pub fn restored_v3() -> Self {
        let config = serde_json::to_vec(&AutoStrategyConfig::default())
            .expect("serialize restored v3 strategy config");
        let config_hash = format!("{:x}", Sha256::digest(config));
        Self {
            version_code: STRATEGY_VERSION_CODE.to_string(),
            strategy_build_id: STRATEGY_BUILD_ID.to_string(),
            config_hash,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restored_identity_is_stable_and_bound_to_v3_config() {
        let first = StrategyIdentity::restored_v3();
        let second = StrategyIdentity::restored_v3();
        assert_eq!(first, second);
        assert_eq!(first.version_code, STRATEGY_VERSION_CODE);
        assert_eq!(first.strategy_build_id, STRATEGY_BUILD_ID);
        assert_eq!(first.config_hash.len(), 64);
    }
}
