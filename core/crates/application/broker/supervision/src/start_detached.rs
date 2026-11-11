#[path = "detached_readiness.rs"]
mod readiness;
use readiness::{verified_record, wait_for_ready};

use crate::{ChildLaunch, CredentialSource, server_command};
use anyhow::{Context, Result};
use std::path::PathBuf;
use yolo_broker_child_custody::{
    PendingBroker, combine_cleanup_failure, initialize_termination_reaper,
    reserve_termination_custody,
};
use yolo_broker_supervision_state::SupervisionPaths;
use yolo_broker_supervision_state::SupervisionPhase;
use yolo_cli::constants;
use yolo_process::{
    DetachedStandardError, ProcessIdentity, ProcessIdentityObservation,
    spawn_self_detaching_before_deadline, spawn_self_detaching_with_private_input,
};
use yolo_remote_broker::IdentityPassphrases;

const BROKER_CHILD_SPAWN_DEADLINE_EXPIRED: &str = "broker child spawn deadline expired";
#[cfg(test)]
const BROKER_CHILD_LATE_SPAWN_CLEANUP: &str = "settle broker child returned after deadline";
#[cfg(test)]
const BROKER_CHILD_CLEANUP_SEPARATOR: &str = "; ";
const BROKER_SUPERVISOR_EXITED: &str = "broker supervisor exited before readiness";
const BROKER_SUPERVISION_RECORD_MISSING: &str = "broker supervision record was not published";
const BROKER_IDENTITY_MISSING: &str = "broker supervision record is missing process identity";
const BROKER_IDENTITY_GONE: &str = "supervised broker identity exited before readiness";

pub struct DetachedSpawner {
    executable: PathBuf,
    passphrases: IdentityPassphrases,
    credential_source: CredentialSource,
}

impl DetachedSpawner {
    pub fn passphrases(&self) -> &IdentityPassphrases {
        &self.passphrases
    }

    #[cfg(any(test, feature = "yolo-test-support"))]
    pub fn new(
        executable: PathBuf,
        passphrases: IdentityPassphrases,
        credential_source: CredentialSource,
    ) -> Self {
        Self {
            executable,
            passphrases,
            credential_source,
        }
    }

    pub fn current(
        passphrases: IdentityPassphrases,
        credential_source: CredentialSource,
    ) -> Result<Self> {
        initialize_termination_reaper()?;
        Ok(Self {
            executable: std::env::current_exe()?,
            passphrases,
            credential_source,
        })
    }
}

impl DetachedSpawner {
    pub fn spawn(
        &self,
        options: &ChildLaunch,
        credential_binding_nonce: Option<&str>,
    ) -> Result<PendingBroker> {
        let launcher = yolo_process::ProcessIdentity::capture(std::process::id())?;
        let mut command = server_command::build(
            constants::BROKER_SUPERVISOR_COMMAND,
            options,
            self.credential_source,
            None,
        )?;
        let paths = SupervisionPaths::for_launch(options)?;
        command.arguments.extend([
            constants::SUPERVISION_LAUNCHER_PID_ARGUMENT.into(),
            launcher.pid.to_string().into(),
            constants::SUPERVISION_LAUNCHER_START_TIME_ARGUMENT.into(),
            launcher.start_time.to_string().into(),
        ]);
        let private_input_deadline = command.deadline;
        let termination = reserve_termination_custody()?;
        anyhow::ensure!(
            std::time::Instant::now() < private_input_deadline,
            BROKER_CHILD_SPAWN_DEADLINE_EXPIRED
        );
        let child = if matches!(self.credential_source, CredentialSource::PrivateStdin) {
            let input = self.passphrases.encode_rebound(credential_binding_nonce)?;
            spawn_self_detaching_with_private_input(
                &self.executable,
                &command.arguments,
                DetachedStandardError::Null,
                input.as_slice(),
                private_input_deadline,
            )?
        } else {
            spawn_self_detaching_before_deadline(
                &self.executable,
                &command.arguments,
                DetachedStandardError::Null,
                private_input_deadline,
            )?
        };
        let mut child = yolo_process::DetachedChildCustody::capture(child)
            .unwrap_or_else(yolo_process::DetachedChildCustodyError::into_custody);
        let Some(supervisor_identity) = child.identity() else {
            let pending = PendingBroker::detached(child, termination);
            return Err(combine_cleanup_failure(
                anyhow::anyhow!(BROKER_SUPERVISOR_EXITED),
                pending.compensate(),
            ));
        };
        match wait_for_ready(
            &mut child,
            options,
            &paths,
            supervisor_identity,
            launcher,
            private_input_deadline,
        ) {
            Ok(record) => Ok(PendingBroker::from_supervision(
                child,
                termination,
                record,
                paths,
                private_input_deadline,
            )),
            Err(primary) => {
                let pending = match verified_record(&paths, options, supervisor_identity, launcher)
                {
                    Ok(record) => PendingBroker::from_supervision(
                        child,
                        termination,
                        record,
                        paths,
                        private_input_deadline,
                    ),
                    Err(_) => PendingBroker::detached(child, termination),
                };
                Err(combine_cleanup_failure(primary, pending.compensate()))
            }
        }
    }
}

#[cfg(test)]
pub(super) fn finish_spawn(
    spawned: PendingBroker,
    deadline: std::time::Instant,
) -> Result<PendingBroker> {
    if std::time::Instant::now() < deadline {
        return Ok(spawned);
    }
    match spawned.compensate() {
        Ok(()) => anyhow::bail!(BROKER_CHILD_SPAWN_DEADLINE_EXPIRED),
        Err(cleanup) => anyhow::bail!(
            "{BROKER_CHILD_SPAWN_DEADLINE_EXPIRED}{BROKER_CHILD_CLEANUP_SEPARATOR}\
                 {BROKER_CHILD_LATE_SPAWN_CLEANUP}: {cleanup:#}"
        ),
    }
}
