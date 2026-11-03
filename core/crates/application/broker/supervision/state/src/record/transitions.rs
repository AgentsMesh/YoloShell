use super::{ProcessIdentity, SupervisionPhase, SupervisionRecord};
use crate::{BrokerReadiness, constants::*};
use anyhow::{Result, ensure};

impl SupervisionRecord {
    pub fn phase(&self) -> SupervisionPhase {
        self.phase
    }
    pub fn launch_id(&self) -> &str {
        &self.launch_id
    }
    pub fn generation(&self) -> &str {
        &self.generation
    }

    pub fn activate(&mut self, broker: ProcessIdentity) -> Result<()> {
        self.require_phase(&[SupervisionPhase::Intent])?;
        ensure!(
            broker.pid != u32::default() && broker.start_time != u64::default(),
            RECORD_READINESS_DETAIL
        );
        self.broker = Some(broker.into());
        self.phase = SupervisionPhase::Active;
        Ok(())
    }

    pub fn ready(&mut self, readiness: &BrokerReadiness) -> Result<()> {
        self.require_phase(&[SupervisionPhase::Active])?;
        ensure!(!readiness.instance_id.is_nil(), RECORD_READINESS_DETAIL);
        yolo_remote_broker::validate_broker_host_id(&readiness.host_id)?;
        yolo_remote_broker::validate_broker_config_revision(&readiness.config_revision)?;
        ensure!(
            readiness.config_revision == self.generation,
            RECORD_GENERATION_DETAIL
        );
        self.instance_id = Some(readiness.instance_id);
        self.broker_host_id = Some(readiness.host_id.clone());
        self.phase = SupervisionPhase::Ready;
        Ok(())
    }

    pub fn mark_running(&mut self) -> Result<()> {
        self.require_phase(&[SupervisionPhase::Ready])?;
        self.phase = SupervisionPhase::Running;
        Ok(())
    }

    pub fn begin_termination(&mut self) -> Result<()> {
        self.require_phase(&[
            SupervisionPhase::Active,
            SupervisionPhase::Ready,
            SupervisionPhase::Running,
            SupervisionPhase::Terminating,
        ])?;
        self.phase = SupervisionPhase::Terminating;
        Ok(())
    }

    pub fn mark_settled(&mut self) -> Result<()> {
        self.require_phase(&[
            SupervisionPhase::Active,
            SupervisionPhase::Ready,
            SupervisionPhase::Running,
            SupervisionPhase::Terminating,
            SupervisionPhase::Settled,
        ])?;
        self.phase = SupervisionPhase::Settled;
        Ok(())
    }

    fn require_phase(&self, phases: &[SupervisionPhase]) -> Result<()> {
        ensure!(
            self.shape_valid() && phases.contains(&self.phase),
            RECORD_TRANSITION_DETAIL
        );
        Ok(())
    }
}
