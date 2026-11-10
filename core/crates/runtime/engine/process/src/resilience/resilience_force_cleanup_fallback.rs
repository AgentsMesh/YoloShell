use super::{
    CLEANUP_CONFIRM_TIMEOUT, CLEANUP_POLL_INTERVAL, CleanupIntent, RetainedProcessCleanup, terminal,
};
use crate::process::LiveProcessIdentity;
use crate::process::live_identity::ProcessExecutionState;
use std::time::Instant;
const CLEANUP_FALLBACK_UNSETTLED_DETAIL: &str =
    "fail-closed retained process cleanup left an unsettled target";

impl RetainedProcessCleanup {
    pub(super) fn settle_contained(&mut self) {
        let intent = self.intent;
        let Some(signal) = intent_signal(intent) else {
            return;
        };
        let targets = self.sorted_targets();
        let settled = settle_until_deadline(&targets, intent, signal);
        if !settled {
            terminal::report_message(CLEANUP_FALLBACK_UNSETTLED_DETAIL);
        }
        self.disarm();
    }
}

fn intent_signal(intent: CleanupIntent) -> Option<libc::c_int> {
    match intent {
        CleanupIntent::Resume => Some(libc::SIGCONT),
        CleanupIntent::Kill => Some(libc::SIGKILL),
        CleanupIntent::Disarmed => None,
    }
}

fn settle_until_deadline(
    targets: &[(LiveProcessIdentity, crate::process::ProcessIdentity)],
    intent: CleanupIntent,
    signal: libc::c_int,
) -> bool {
    let deadline = Instant::now() + CLEANUP_CONFIRM_TIMEOUT;
    loop {
        let mut unsettled = false;
        for (live, _) in targets {
            if target_is_settled(live, intent) {
                continue;
            }
            unsettled = true;
            let _ = terminal::contain(|| drop(live.signal(signal)));
        }
        if !unsettled {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(CLEANUP_POLL_INTERVAL);
    }
}

fn target_is_settled(live: &LiveProcessIdentity, intent: CleanupIntent) -> bool {
    terminal::contain(|| match live.execution_state() {
        Ok(ProcessExecutionState::Gone) => true,
        Ok(ProcessExecutionState::Running) => intent == CleanupIntent::Resume,
        Ok(ProcessExecutionState::Stopped) => false,
        Err(error) => {
            drop(error);
            false
        }
    })
    .unwrap_or(false)
}
