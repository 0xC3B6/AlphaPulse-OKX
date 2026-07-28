use std::collections::{BTreeMap, BTreeSet};

use crate::{
    auto_strategy::AutoStrategyConfig,
    observability::{PositionContextEvent, StrategyCandidateEvent},
    paper::{PaperEquityCurves, PaperState},
    strategy_identity::{strategy_config_hash, StrategyRunMode},
};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum StrategyRuntimeError {
    #[error("active strategy runtime must use active paper mode")]
    ActiveModeRequired,
    #[error("shadow strategy runtime must use shadow paper mode")]
    ShadowModeRequired,
    #[error("strategy config hash does not match runtime identity for {0}")]
    ConfigHashMismatch(String),
    #[error("strategy run id already registered: {0}")]
    DuplicateRunId(String),
    #[error("strategy experiment already has a live runtime: {0}")]
    DuplicateExperiment(String),
    #[error("strategy runtime not found: {0}")]
    RunNotFound(String),
    #[error("restored strategy state does not match runtime {0}")]
    RestoredStateMismatch(String),
}

#[derive(Debug, Clone)]
pub struct StrategyRuntime {
    config: AutoStrategyConfig,
    paper: PaperState,
    equity_curves: PaperEquityCurves,
    pending_candidate_events: BTreeMap<String, StrategyCandidateEvent>,
    pending_position_context_events: BTreeMap<String, PositionContextEvent>,
}

impl StrategyRuntime {
    pub fn new(
        paper: PaperState,
        config: AutoStrategyConfig,
        equity_curves: PaperEquityCurves,
    ) -> Result<Self, StrategyRuntimeError> {
        let experiment_key = paper.strategy_identity().experiment_key();
        if paper.strategy_identity().config_hash != strategy_config_hash(&config) {
            return Err(StrategyRuntimeError::ConfigHashMismatch(experiment_key));
        }
        Ok(Self {
            config,
            paper,
            equity_curves,
            pending_candidate_events: BTreeMap::new(),
            pending_position_context_events: BTreeMap::new(),
        })
    }

    pub fn config(&self) -> AutoStrategyConfig {
        self.config
    }

    pub fn paper(&self) -> &PaperState {
        &self.paper
    }

    pub fn paper_mut(&mut self) -> &mut PaperState {
        &mut self.paper
    }

    pub fn equity_curves(&self) -> &PaperEquityCurves {
        &self.equity_curves
    }

    pub fn equity_curves_mut(&mut self) -> &mut PaperEquityCurves {
        &mut self.equity_curves
    }

    pub fn replace_persisted_state(
        &mut self,
        paper: PaperState,
        equity_curves: PaperEquityCurves,
    ) -> Result<(), StrategyRuntimeError> {
        if paper.run_id() != self.paper.run_id()
            || paper.strategy_identity() != self.paper.strategy_identity()
            || paper.run_mode() != self.paper.run_mode()
        {
            return Err(StrategyRuntimeError::RestoredStateMismatch(
                self.paper.run_id().to_string(),
            ));
        }
        self.paper = paper;
        self.equity_curves = equity_curves;
        Ok(())
    }

    pub fn queue_candidate_event(&mut self, event: StrategyCandidateEvent) {
        match self.pending_candidate_events.entry(event.event_key.clone()) {
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                entry.get_mut().merge(event);
            }
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(event);
            }
        }
    }

    pub fn queue_position_context_event(&mut self, event: PositionContextEvent) {
        match self
            .pending_position_context_events
            .entry(event.event_key.clone())
        {
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                entry.get_mut().merge(event);
            }
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(event);
            }
        }
    }

    pub fn take_observability_events(
        &mut self,
    ) -> (Vec<StrategyCandidateEvent>, Vec<PositionContextEvent>) {
        (
            std::mem::take(&mut self.pending_candidate_events)
                .into_values()
                .collect(),
            std::mem::take(&mut self.pending_position_context_events)
                .into_values()
                .collect(),
        )
    }

    pub fn restore_observability_events(
        &mut self,
        candidates: Vec<StrategyCandidateEvent>,
        positions: Vec<PositionContextEvent>,
    ) {
        for event in candidates {
            self.queue_candidate_event(event);
        }
        for event in positions {
            self.queue_position_context_event(event);
        }
    }
}

#[derive(Debug)]
pub struct StrategyRuntimeContainer {
    active_run_id: String,
    runs: BTreeMap<String, StrategyRuntime>,
}

impl StrategyRuntimeContainer {
    pub fn new(active: StrategyRuntime) -> Result<Self, StrategyRuntimeError> {
        if !active.paper().run_mode().is_active() {
            return Err(StrategyRuntimeError::ActiveModeRequired);
        }
        let active_run_id = active.paper().run_id().to_string();
        let mut runs = BTreeMap::new();
        runs.insert(active_run_id.clone(), active);
        Ok(Self {
            active_run_id,
            runs,
        })
    }

    pub fn register_shadow(&mut self, shadow: StrategyRuntime) -> Result<(), StrategyRuntimeError> {
        self.validate_shadow(&shadow)?;
        let run_id = shadow.paper().run_id().to_string();
        self.runs.insert(run_id, shadow);
        Ok(())
    }

    pub fn validate_shadow(&self, shadow: &StrategyRuntime) -> Result<(), StrategyRuntimeError> {
        if shadow.paper().run_mode() != StrategyRunMode::ShadowPaper {
            return Err(StrategyRuntimeError::ShadowModeRequired);
        }
        let run_id = shadow.paper().run_id().to_string();
        if self.runs.contains_key(&run_id) {
            return Err(StrategyRuntimeError::DuplicateRunId(run_id));
        }
        let experiment_key = shadow.paper().strategy_identity().experiment_key();
        if self
            .runs
            .values()
            .any(|runtime| runtime.paper().strategy_identity().experiment_key() == experiment_key)
        {
            return Err(StrategyRuntimeError::DuplicateExperiment(experiment_key));
        }
        Ok(())
    }

    pub fn active_run_id(&self) -> &str {
        &self.active_run_id
    }

    pub fn active(&self) -> &StrategyRuntime {
        self.runs
            .get(&self.active_run_id)
            .expect("active strategy runtime exists")
    }

    pub fn active_mut(&mut self) -> &mut StrategyRuntime {
        self.runs
            .get_mut(&self.active_run_id)
            .expect("active strategy runtime exists")
    }

    pub fn get(&self, run_id: &str) -> Result<&StrategyRuntime, StrategyRuntimeError> {
        self.runs
            .get(run_id)
            .ok_or_else(|| StrategyRuntimeError::RunNotFound(run_id.to_string()))
    }

    pub fn get_mut(&mut self, run_id: &str) -> Result<&mut StrategyRuntime, StrategyRuntimeError> {
        self.runs
            .get_mut(run_id)
            .ok_or_else(|| StrategyRuntimeError::RunNotFound(run_id.to_string()))
    }

    pub fn ordered_run_ids(&self) -> Vec<String> {
        std::iter::once(self.active_run_id.clone())
            .chain(
                self.runs
                    .keys()
                    .filter(|run_id| *run_id != &self.active_run_id)
                    .cloned(),
            )
            .collect()
    }

    pub fn open_position_inst_ids(&self) -> Vec<String> {
        self.runs
            .values()
            .flat_map(|runtime| runtime.paper().open_position_inst_ids())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn len(&self) -> usize {
        self.runs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.runs.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        paper::{PaperOrderRequest, PaperSide},
        strategy_identity::{StrategyIdentity, StrategyRunMode},
    };

    fn runtime(variant: &str, run_id: &str, mode: StrategyRunMode) -> StrategyRuntime {
        let config = AutoStrategyConfig::default();
        let identity = if variant == "baseline" {
            StrategyIdentity::restored_v3()
        } else {
            StrategyIdentity::research_variant_from_config(
                "v0.1.3",
                variant,
                format!("{variant}-build"),
                &config,
            )
        };
        StrategyRuntime::new(
            PaperState::fresh_isolated(identity, run_id, mode).unwrap(),
            config,
            PaperEquityCurves::new(),
        )
        .unwrap()
    }

    #[test]
    fn active_and_shadow_hold_independent_accounts() {
        let active = runtime("baseline", "baseline-run", StrategyRunMode::ActivePaper);
        let shadow = runtime("guard", "guard-run", StrategyRunMode::ShadowPaper);
        let mut container = StrategyRuntimeContainer::new(active).unwrap();
        container.register_shadow(shadow).unwrap();

        let order = PaperOrderRequest::manual("BTC-USDT-SWAP", PaperSide::Long, 100.0, 1.0);
        container
            .get_mut("guard-run")
            .unwrap()
            .paper_mut()
            .open(order, 100.0, 10_000.0, 1)
            .unwrap();

        assert!(container
            .active()
            .paper()
            .open_position_inst_ids()
            .is_empty());
        assert_eq!(
            container
                .get("guard-run")
                .unwrap()
                .paper()
                .open_position_inst_ids(),
            vec!["BTC-USDT-SWAP"]
        );
    }

    #[test]
    fn duplicate_run_and_experiment_are_rejected() {
        let active = runtime("baseline", "baseline-run", StrategyRunMode::ActivePaper);
        let mut container = StrategyRuntimeContainer::new(active).unwrap();
        container
            .register_shadow(runtime("guard", "guard-run", StrategyRunMode::ShadowPaper))
            .unwrap();

        assert_eq!(
            container
                .register_shadow(runtime("other", "guard-run", StrategyRunMode::ShadowPaper,))
                .unwrap_err(),
            StrategyRuntimeError::DuplicateRunId("guard-run".to_string())
        );
        assert_eq!(
            container
                .register_shadow(runtime(
                    "guard",
                    "guard-run-2",
                    StrategyRunMode::ShadowPaper,
                ))
                .unwrap_err(),
            StrategyRuntimeError::DuplicateExperiment("v0.1.3/guard".to_string())
        );
    }
}
