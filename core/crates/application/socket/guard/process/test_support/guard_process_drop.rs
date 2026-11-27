use super::constants::{CLEANUP_TIMEOUT, FALLBACK_CLEANUP_TIMEOUT, POLL_INTERVAL};
use super::guard_process::GuardProcess;
use super::process_support;
use std::time::{Duration, Instant};

impl Drop for GuardProcess {
    fn drop(&mut self) {
        if self.settled {
            return;
        }
        self.refresh_evidence();
        if self.guard_identity.matches() {
            let _ = process_support::kill_guard(self.guard_identity);
        }
        if wait_until_settled(self, CLEANUP_TIMEOUT) {
            return;
        }
        for identity in [
            self.leaf,
            self.late_leaf,
            self.refresh_holder,
            self.watchdog,
            Some(self.guard_identity),
        ]
        .into_iter()
        .flatten()
        {
            if identity.matches() {
                let _ = identity.signal(libc::SIGKILL);
            }
        }
        let _ = wait_until_settled(self, FALLBACK_CLEANUP_TIMEOUT);
    }
}

fn wait_until_settled(process: &mut GuardProcess, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        process.refresh_evidence();
        let guard_reaped = process.reap_guard().is_some();
        let identities_gone = process.leaf.is_none_or(|value| !value.matches())
            && process.late_leaf.is_none_or(|value| !value.matches())
            && process.refresh_holder.is_none_or(|value| !value.matches())
            && process.watchdog.is_none_or(|value| !value.matches());
        let root_gone = process
            .socket_root
            .as_ref()
            .is_none_or(|path| !path.exists());
        if guard_reaped && identities_gone && root_gone {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}
