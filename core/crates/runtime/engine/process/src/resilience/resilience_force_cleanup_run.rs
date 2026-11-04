use super::{
    CLEANUP_CONFIRM_TIMEOUT, CLEANUP_POLL_INTERVAL, CleanupIntent, RetainedProcessCleanup, terminal,
};
use crate::process::live_identity::ProcessExecutionState;
use crate::process::{LiveProcessIdentity, ProcessIdentity};
use anyhow::Result;
use std::time::Instant;
const CLEANUP_SIGNAL_REJECTED_DETAIL: &str = "retained process cleanup signal was rejected";
const CLEANUP_SIGNAL_FAILURE_DETAIL: &str = "retained process cleanup signal failed";
const CLEANUP_OBSERVATION_FAILURE_DETAIL: &str = "retained process cleanup observation failed";
const CLEANUP_UNSETTLED_DETAIL: &str = "retained process cleanup left an unsettled target";
const CLEANUP_FAILURE_SEPARATOR: &str = "; ";

impl RetainedProcessCleanup {
    pub(super) fn run(&mut self) -> Result<()> {
        let intent = self.intent;
        if intent == CleanupIntent::Disarmed {
            return Ok(());
        }
        let targets = self.sorted_targets();
        let signal = signal_for(intent);
        let mut failures = Vec::new();
        for (live, durable) in &targets {
            if should_signal(live, intent, *durable, &mut failures) {
                signal_and_record(*durable, live, signal, &mut failures);
            }
        }
        confirm_cleanup(&targets, intent, &mut failures);
        if !failures.is_empty() {
            anyhow::bail!(failures.join(CLEANUP_FAILURE_SEPARATOR));
        }
        self.disarm();
        Ok(())
    }
}

fn signal_for(intent: CleanupIntent) -> libc::c_int {
    match intent {
        CleanupIntent::Resume => libc::SIGCONT,
        CleanupIntent::Kill => libc::SIGKILL,
        CleanupIntent::Disarmed => unreachable!(),
    }
}

fn should_signal(
    live: &LiveProcessIdentity,
    intent: CleanupIntent,
    durable: ProcessIdentity,
    failures: &mut Vec<String>,
) -> bool {
    match live.execution_state() {
        Ok(ProcessExecutionState::Gone) => false,
        Ok(ProcessExecutionState::Running) => intent == CleanupIntent::Kill,
        Ok(ProcessExecutionState::Stopped) => true,
        Err(error) => {
            let detail = terminal::consume_error(error);
            failures.push(format!(
                "{CLEANUP_OBSERVATION_FAILURE_DETAIL}: {durable:?}: {detail}"
            ));
            true
        }
    }
}

fn signal_and_record(
    durable: ProcessIdentity,
    live: &LiveProcessIdentity,
    signal: libc::c_int,
    failures: &mut Vec<String>,
) {
    match live.signal(signal) {
        Ok(true) => {}
        Ok(false) if live.execution_state().ok() == Some(ProcessExecutionState::Gone) => {}
        Ok(false) => failures.push(format!("{CLEANUP_SIGNAL_REJECTED_DETAIL}: {durable:?}")),
        Err(error) => {
            let detail = terminal::consume_error(error);
            failures.push(format!(
                "{CLEANUP_SIGNAL_FAILURE_DETAIL}: {durable:?}: {detail}"
            ));
        }
    }
}

fn confirm_cleanup(
    targets: &[(LiveProcessIdentity, ProcessIdentity)],
    intent: CleanupIntent,
    failures: &mut Vec<String>,
) {
    let unsettled = wait_for_settlement(targets, intent, failures);
    for (live, durable) in &unsettled {
        signal_and_record(*durable, live, signal_for(intent), failures);
    }
    for (_, durable) in wait_for_settlement(&unsettled, intent, failures) {
        failures.push(format!("{CLEANUP_UNSETTLED_DETAIL}: {durable:?}"));
    }
}

fn wait_for_settlement(
    targets: &[(LiveProcessIdentity, ProcessIdentity)],
    intent: CleanupIntent,
    failures: &mut Vec<String>,
) -> Vec<(LiveProcessIdentity, ProcessIdentity)> {
    let deadline = Instant::now() + CLEANUP_CONFIRM_TIMEOUT;
    loop {
        let mut unsettled = Vec::new();
        let mut observations = Vec::new();
        for (live, durable) in targets {
            match live.execution_state() {
                Ok(ProcessExecutionState::Gone) => {}
                Ok(ProcessExecutionState::Running) if intent == CleanupIntent::Resume => {}
                Ok(ProcessExecutionState::Running | ProcessExecutionState::Stopped) => {
                    unsettled.push((live.clone(), *durable))
                }
                Err(error) => {
                    let detail = terminal::consume_error(error);
                    unsettled.push((live.clone(), *durable));
                    observations.push(format!(
                        "{CLEANUP_OBSERVATION_FAILURE_DETAIL}: {durable:?}: {detail}"
                    ));
                }
            }
        }
        if unsettled.is_empty() || Instant::now() >= deadline {
            failures.extend(observations);
            return unsettled;
        }
        std::thread::sleep(CLEANUP_POLL_INTERVAL);
    }
}
