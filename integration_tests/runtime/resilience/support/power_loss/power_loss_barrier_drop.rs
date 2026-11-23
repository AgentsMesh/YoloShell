use super::*;
use std::time::Instant;

const DROP_CONFIRM_TIMEOUT: Duration = Duration::from_secs(2);
const DROP_CONFIRM_POLL: Duration = Duration::from_millis(10);
const DROP_SIGNAL_REJECTED_DETAIL: &str = "power-loss Drop cleanup signal was rejected";
const DROP_SIGNAL_FAILURE_DETAIL: &str = "power-loss Drop cleanup signal failed";
const DROP_OBSERVATION_FAILURE_DETAIL: &str = "power-loss Drop cleanup observation failed";
const DROP_UNSETTLED_DETAIL: &str = "power-loss Drop cleanup left an unsettled target";
const DROP_CLEANUP_FAILURE_DETAIL: &str = "power-loss Drop cleanup was incomplete";
const DROP_FAILURE_SEPARATOR: &str = "; ";

#[derive(Clone, Copy)]
enum BarrierCleanupIntent {
    Resume,
    Kill,
}

impl Drop for PowerLossFreezeBarrier {
    fn drop(&mut self) {
        let Some((intent, mut targets)) = self.cleanup_request() else {
            return;
        };
        sort_targets(&mut targets);
        let mut failures = Vec::new();
        signal_all(&targets, intent.signal(), &mut failures);
        let unsettled = wait_for_settlement(&targets, intent, &mut failures);
        signal_all(&unsettled, libc::SIGKILL, &mut failures);
        for identity in wait_for_settlement(&unsettled, BarrierCleanupIntent::Kill, &mut failures) {
            failures.push(format!("{DROP_UNSETTLED_DETAIL}: {identity:?}"));
        }
        report_failures(failures);
    }
}

impl PowerLossFreezeBarrier {
    fn cleanup_request(&self) -> Option<(BarrierCleanupIntent, Vec<RuntimeProcessIdentity>)> {
        match self.phase {
            PowerLossBarrierPhase::Freezing => Some((
                BarrierCleanupIntent::Resume,
                self.thaw_on_drop.iter().copied().collect(),
            )),
            PowerLossBarrierPhase::Killing => {
                Some((BarrierCleanupIntent::Kill, self.sorted_direct_targets()))
            }
            PowerLossBarrierPhase::Complete => None,
        }
    }
}

impl BarrierCleanupIntent {
    fn signal(self) -> i32 {
        match self {
            Self::Resume => libc::SIGCONT,
            Self::Kill => libc::SIGKILL,
        }
    }

    fn settled(self, identity: RuntimeProcessIdentity) -> Result<bool> {
        match self {
            Self::Resume => Ok(!identity.is_stably_stopped()?),
            Self::Kill => match identity.observe() {
                ProcessIdentityObservation::Gone => Ok(true),
                ProcessIdentityObservation::Matching => Ok(false),
                ProcessIdentityObservation::Unknown => {
                    anyhow::bail!(DROP_OBSERVATION_FAILURE_DETAIL)
                }
            },
        }
    }
}

fn signal_all(targets: &[RuntimeProcessIdentity], signal: i32, failures: &mut Vec<String>) {
    for identity in targets {
        match identity.signal(signal) {
            Ok(true) => {}
            Ok(false) if identity.observe() == ProcessIdentityObservation::Gone => {}
            Ok(false) => failures.push(format!("{DROP_SIGNAL_REJECTED_DETAIL}: {identity:?}")),
            Err(error) => failures.push(format!(
                "{DROP_SIGNAL_FAILURE_DETAIL}: {identity:?}: {error:#}"
            )),
        }
    }
}

fn wait_for_settlement(
    targets: &[RuntimeProcessIdentity],
    intent: BarrierCleanupIntent,
    failures: &mut Vec<String>,
) -> Vec<RuntimeProcessIdentity> {
    let deadline = Instant::now() + DROP_CONFIRM_TIMEOUT;
    loop {
        let mut unsettled = Vec::new();
        let mut observations = Vec::new();
        for identity in targets {
            match intent.settled(*identity) {
                Ok(true) => {}
                Ok(false) => unsettled.push(*identity),
                Err(error) => {
                    unsettled.push(*identity);
                    observations.push(format!(
                        "{DROP_OBSERVATION_FAILURE_DETAIL}: {identity:?}: {error:#}"
                    ));
                }
            }
        }
        if unsettled.is_empty() || Instant::now() >= deadline {
            failures.extend(observations);
            return unsettled;
        }
        std::thread::sleep(DROP_CONFIRM_POLL);
    }
}

fn report_failures(failures: Vec<String>) {
    if failures.is_empty() {
        return;
    }
    let detail = failures.join(DROP_FAILURE_SEPARATOR);
    tracing::error!(detail, message = DROP_CLEANUP_FAILURE_DETAIL);
    if !std::thread::panicking() {
        panic!("{DROP_CLEANUP_FAILURE_DETAIL}: {detail}");
    }
}
