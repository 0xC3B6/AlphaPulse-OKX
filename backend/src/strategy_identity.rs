use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};

use crate::auto_strategy::AutoStrategyConfig;

pub const STRATEGY_VERSION_CODE: &str = "v0.1.3";
pub const STRATEGY_BUILD_ID: &str = "legacy-v3-replay-2026-07-10";
pub const INITIAL_RUN_ID: &str = "v0.1.3-restored-paper-1";
pub const BASELINE_VARIANT_ID: &str = "baseline";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct StrategyIdentity {
    pub version_code: String,
    pub parent_version: String,
    pub variant_id: String,
    pub strategy_build_id: String,
    pub config_hash: String,
}

#[derive(Deserialize)]
struct StrategyIdentityWire {
    version_code: String,
    #[serde(default)]
    parent_version: String,
    #[serde(default)]
    variant_id: String,
    strategy_build_id: String,
    config_hash: String,
}

impl<'de> Deserialize<'de> for StrategyIdentity {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = StrategyIdentityWire::deserialize(deserializer)?;
        let (derived_parent, derived_variant) = split_experiment_version(&wire.version_code);
        Ok(Self {
            version_code: wire.version_code,
            parent_version: non_empty_or(wire.parent_version, derived_parent),
            variant_id: non_empty_or(wire.variant_id, derived_variant),
            strategy_build_id: wire.strategy_build_id,
            config_hash: wire.config_hash,
        })
    }
}

impl StrategyIdentity {
    pub fn restored_v3() -> Self {
        let config = serde_json::to_vec(&AutoStrategyConfig::default())
            .expect("serialize restored v3 strategy config");
        let config_hash = format!("{:x}", Sha256::digest(config));
        Self {
            version_code: STRATEGY_VERSION_CODE.to_string(),
            parent_version: STRATEGY_VERSION_CODE.to_string(),
            variant_id: BASELINE_VARIANT_ID.to_string(),
            strategy_build_id: STRATEGY_BUILD_ID.to_string(),
            config_hash,
        }
    }

    pub fn research_variant(
        parent_version: impl Into<String>,
        variant_id: impl Into<String>,
        strategy_build_id: impl Into<String>,
        config_hash: impl Into<String>,
    ) -> Self {
        let parent_version = parent_version.into();
        let variant_id = variant_id.into();
        let version_code = if variant_id == BASELINE_VARIANT_ID {
            parent_version.clone()
        } else {
            format!("{parent_version}/{variant_id}")
        };
        Self {
            version_code,
            parent_version,
            variant_id,
            strategy_build_id: strategy_build_id.into(),
            config_hash: config_hash.into(),
        }
    }

    pub fn experiment_key(&self) -> String {
        format!("{}/{}", self.parent_version, self.variant_id)
    }
}

pub fn split_experiment_version(version_code: &str) -> (String, String) {
    match version_code.split_once('/') {
        Some((parent, variant)) if !parent.trim().is_empty() && !variant.trim().is_empty() => {
            (parent.trim().to_string(), variant.trim().to_string())
        }
        _ => (
            version_code.trim().to_string(),
            BASELINE_VARIANT_ID.to_string(),
        ),
    }
}

fn non_empty_or(value: String, fallback: String) -> String {
    let value = value.trim();
    if value.is_empty() {
        fallback
    } else {
        value.to_string()
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
        assert_eq!(first.parent_version, STRATEGY_VERSION_CODE);
        assert_eq!(first.variant_id, "baseline");
        assert_eq!(first.experiment_key(), "v0.1.3/baseline");
        assert_eq!(first.strategy_build_id, STRATEGY_BUILD_ID);
        assert_eq!(
            first.config_hash,
            "efc9cef8f04c0bdf7bcc67ccc8d2132ee5fe96e87bb99925f5ce76c7eb6bf179"
        );
    }

    #[test]
    fn legacy_identity_without_experiment_fields_defaults_to_baseline() {
        let identity: StrategyIdentity = serde_json::from_value(serde_json::json!({
            "version_code": "v0.1.3",
            "strategy_build_id": "legacy-v3-replay-2026-07-10",
            "config_hash": "legacy-config"
        }))
        .unwrap();

        assert_eq!(identity.parent_version, "v0.1.3");
        assert_eq!(identity.variant_id, "baseline");
        assert_eq!(identity.experiment_key(), "v0.1.3/baseline");
    }

    #[test]
    fn research_variant_has_unique_version_code_under_parent() {
        let identity = StrategyIdentity::research_variant(
            "v0.1.3",
            "signal_context_guard",
            "shadow-build",
            "shadow-config",
        );

        assert_eq!(identity.version_code, "v0.1.3/signal_context_guard");
        assert_eq!(identity.parent_version, "v0.1.3");
        assert_eq!(identity.variant_id, "signal_context_guard");
        assert_eq!(identity.experiment_key(), "v0.1.3/signal_context_guard");
    }
}
