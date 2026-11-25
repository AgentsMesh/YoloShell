#[path = "supervisor_activation.rs"]
mod activation;
use activation::activate_child;

use super::server_command;
use super::supervision_activation::{self, ActivationSender};
use crate::supervisor_custody::SupervisorCustody;
use crate::supervisor_lifecycle::SupervisionOutcome;
use anyhow::{Context, Result, ensure};
use yolo_broker_child_custody::{
    combine_cleanup_failure, initialize_termination_reaper, reserve_termination_custody,
};
use yolo_broker_supervision_state::SupervisionPaths;
use yolo_broker_supervision_state::{ChildLaunch, CredentialSource};
use yolo_broker_supervision_state::{SupervisionPhase, SupervisionRecord};
use yolo_cli::constants;
use yolo_process::{DetachedChildCustody, ProcessIdentity, ProcessIdentityObservation};
use yolo_remote_broker::{BrokerConfig, validate_broker_config_revision, validate_broker_host_id};

const STDIN_CONTEXT: &str = "open supervised broker private input";
const CHILD_EXIT_DETAIL: &str = "supervised broker exited unsuccessfully";
const LAUNCHER_GONE_DETAIL: &str = "broker launcher exited before adoption";
const READINESS_REVISION_MISMATCH: &str = "supervised broker readiness revision mismatch";
const READINESS_HOST_MISMATCH: &str = "supervised broker readiness host identity mismatch";
const READINESS_INSTANCE_INVALID: &str = "supervised broker readiness instance is invalid";
const CONTROL_MISSING: &str = "supervised broker control custody is missing";

pub async fn run_supervisor(
    launch: ChildLaunch,
    credential_source: CredentialSource,
    launcher: ProcessIdentity,
) -> Result<()> {
    initialize_termination_reaper()?;
    let deadline = crate::supervisor_process::deadline(&launch)?;
    wait_for_launcher(launcher, deadline)?;
    let supervisor = ProcessIdentity::capture(std::process::id())?;
    let paths = SupervisionPaths::for_launch(&launch)?;
    let mut record = SupervisionRecord::intent(&launch, supervisor, launcher)?;
    paths.write_record(&record)?;
    let passphrases = crate::credentials::read(credential_source)?;
    let expected_revision = launch.config_revision();
    let expected_host = BrokerConfig::load_expected_with_passphrases(
        launch.config(),
        expected_revision,
        &passphrases,
    )?
    .host_id()
    .to_owned();
    let private_input = matches!(credential_source, CredentialSource::PrivateStdin)
        .then(|| passphrases.encode())
        .transpose()?;
    drop(passphrases);
    let (control, receiver) = supervision_activation::channel()?;
    let server = server_command::build(
        constants::SUPERVISED_SERVE_COMMAND,
        &launch,
        credential_source,
        Some(receiver.descriptor()),
    )?;
    let mut command = crate::supervisor_process::command(
        &server.arguments,
        private_input.is_some(),
        record.launch_id(),
    )?;
    receiver.configure_inheritance(&mut command);
    let termination = reserve_termination_custody()?;
    let mut raw_child =
        yolo_process::spawn_waitable_child_before_deadline(&mut command, server.deadline)
            .context(crate::supervisor_process::SPAWN_CONTEXT)?;
    drop(receiver);
    let input = raw_child.stdin.take();
    let child = match DetachedChildCustody::capture(raw_child) {
        Ok(child) => child,
        Err(error) => {
            let primary = anyhow::anyhow!("{}: {error}", crate::supervisor_process::SPAWN_CONTEXT);
            let custody = SupervisorCustody::new(error.into_custody(), termination);
            let cleanup = crate::supervisor_cleanup::settle_unrecorded(custody, Some(control));
            return Err(combine_cleanup_failure(primary, cleanup));
        }
    };
    let mut custody = SupervisorCustody::new(child, termination);
    let mut control = Some(control);
    let activation = activate_child(
        custody.child()?,
        control.as_mut().context(CONTROL_MISSING)?,
        input,
        private_input,
        &mut record,
        &paths,
        &expected_host,
        expected_revision,
        server.deadline,
    );
    if let Err(primary) = activation {
        return crate::supervisor_cleanup::after_error(
            custody,
            control,
            &mut record,
            &paths,
            primary,
        );
    }
    let outcome = crate::supervisor_lifecycle::supervise(
        custody.child()?,
        control.as_mut().context(CONTROL_MISSING)?,
        &mut record,
        &paths,
        server.deadline,
    );
    finish_lifecycle(outcome, custody, control, record, paths)
}

fn finish_lifecycle(
    outcome: Result<SupervisionOutcome>,
    custody: SupervisorCustody,
    control: Option<ActivationSender>,
    mut record: SupervisionRecord,
    paths: SupervisionPaths,
) -> Result<()> {
    match outcome {
        Ok(SupervisionOutcome::Reaped(status)) => {
            custody.finish_reaped();
            let was_running = record.phase() == SupervisionPhase::Running;
            record.mark_settled()?;
            paths.write_record(&record)?;
            ensure!(status.success(), CHILD_EXIT_DETAIL);
            if was_running {
                paths.remove_all()?;
            }
            Ok(())
        }
        Ok(SupervisionOutcome::Terminate) => {
            crate::supervisor_cleanup::terminate(custody, control, &mut record, &paths)
        }
        Err(primary) => {
            crate::supervisor_cleanup::after_error(custody, control, &mut record, &paths, primary)
        }
    }
}

fn wait_for_launcher(launcher: ProcessIdentity, deadline: std::time::Instant) -> Result<()> {
    loop {
        match launcher.observe() {
            ProcessIdentityObservation::Matching => return Ok(()),
            ProcessIdentityObservation::Gone => anyhow::bail!(LAUNCHER_GONE_DETAIL),
            ProcessIdentityObservation::Unknown => {}
        }
        let now = std::time::Instant::now();
        ensure!(now < deadline, LAUNCHER_GONE_DETAIL);
        std::thread::sleep(
            deadline
                .saturating_duration_since(now)
                .min(yolo_remote_broker::BROKER_READY_POLL_INTERVAL),
        );
    }
}
