use super::MacosProcessEvents;
use crate::process::LiveProcessIdentity;
use anyhow::Result;
use std::collections::VecDeque;
use std::sync::Mutex;

#[path = "process_events_macos_registration.rs"]
mod registration;
use registration::RegistrationFaults;

pub(super) const READ_FAILURE_DETAIL: &str = "injected process event read failure";
pub(super) const RECONCILE_FAILURE_DETAIL: &str = "injected process event reconcile failure";
const SINGLE_TEST_EVENT_COUNT: usize = super::SINGLE_EVENT as usize;

#[derive(Default)]
pub(super) struct TestFaults {
    fail_next_read: bool,
    fail_next_reconcile: bool,
    before_unregister_read: Option<TestRead>,
    reconcile_read: Option<TestRead>,
    registration: RegistrationFaults,
    reads: Mutex<VecDeque<TestRead>>,
}

enum TestRead {
    Fork(u32),
    ForkExit(u32),
    Exit(u32),
    Invalid,
    Valid(usize),
}

impl TestFaults {
    pub(super) fn check_read(&mut self) -> Result<()> {
        if std::mem::take(&mut self.fail_next_read) {
            anyhow::bail!(READ_FAILURE_DETAIL);
        }
        Ok(())
    }

    pub(super) fn check_reconcile(&mut self) -> Result<()> {
        if std::mem::take(&mut self.fail_next_reconcile) {
            anyhow::bail!(RECONCILE_FAILURE_DETAIL);
        }
        if let Some(read) = self.reconcile_read.take() {
            self.queue_read(read);
        }
        Ok(())
    }

    pub(super) fn queue_before_unregister_read(&mut self) {
        if let Some(read) = self.before_unregister_read.take() {
            self.queue_read(read);
        }
    }

    pub(super) fn before_change(&mut self, change: &libc::kevent) -> Result<()> {
        self.registration.before_change(change)
    }

    pub(super) fn after_change(&mut self, change: &libc::kevent) {
        let Some((identity, action)) = self.registration.after_change(change) else {
            return;
        };
        self.queue_read(TestRead::Fork(identity.pid()));
        action();
    }

    pub(super) fn read(&self, events: &mut [libc::kevent]) -> Option<usize> {
        match self.reads.lock().unwrap().pop_front()? {
            TestRead::Fork(pid) => {
                events[usize::default()] =
                    super::process_event(pid, u16::default(), libc::NOTE_FORK);
                Some(SINGLE_TEST_EVENT_COUNT)
            }
            TestRead::ForkExit(pid) => {
                events[usize::default()] =
                    super::process_event(pid, u16::default(), libc::NOTE_FORK | libc::NOTE_EXIT);
                Some(SINGLE_TEST_EVENT_COUNT)
            }
            TestRead::Exit(pid) => {
                events[usize::default()] =
                    super::process_event(pid, u16::default(), libc::NOTE_EXIT);
                Some(SINGLE_TEST_EVENT_COUNT)
            }
            TestRead::Invalid => {
                let mut event =
                    super::process_event(std::process::id(), u16::default(), u32::default());
                event.filter = libc::EVFILT_READ;
                events[usize::default()] = event;
                Some(SINGLE_TEST_EVENT_COUNT)
            }
            TestRead::Valid(count) => {
                events[..count].fill(super::process_event(
                    std::process::id(),
                    u16::default(),
                    u32::default(),
                ));
                Some(count)
            }
        }
    }

    fn queue_read(&self, read: TestRead) {
        self.reads.lock().unwrap().push_back(read);
    }
}

impl MacosProcessEvents {
    pub(in crate::process) fn unresolved_fork_lineages_for_test(&self) -> Vec<u64> {
        let mut lineages = self.unresolved_forks.iter().copied().collect::<Vec<_>>();
        lineages.sort_unstable();
        lineages
    }

    pub(in crate::process) fn fail_next_read(&mut self) {
        self.test_faults.fail_next_read = true;
    }

    pub(in crate::process) fn fail_next_reconcile(&mut self) {
        self.test_faults.fail_next_reconcile = true;
    }

    pub(in crate::process) fn fail_next_registration_as_missing(
        &mut self,
        identity: LiveProcessIdentity,
    ) {
        self.test_faults.registration.fail_as_missing(identity);
    }

    pub(in crate::process) fn retire_during_next_registration(
        &mut self,
        identity: LiveProcessIdentity,
        action: impl FnOnce() + Send + Sync + 'static,
    ) {
        self.test_faults
            .registration
            .retire_after_add(identity, action);
    }

    pub(in crate::process) fn queue_invalid_test_read(&self) {
        self.test_faults.queue_read(TestRead::Invalid);
    }

    pub(in crate::process) fn queue_valid_test_read(&self, count: usize) {
        self.test_faults.queue_read(TestRead::Valid(count));
    }

    pub(in crate::process) fn inject_unresolved_fork(&mut self, parent: LiveProcessIdentity) {
        self.observe_fork(parent.lineage_id()).unwrap();
    }

    pub(in crate::process) fn inject_gone_unresolved_fork(&mut self, parent: LiveProcessIdentity) {
        self.observe_fork(parent.lineage_id()).unwrap();
        self.gone_fork_sources.insert(parent.lineage_id());
    }

    pub(in crate::process) fn queue_fork_test_read(&self, parent: LiveProcessIdentity) {
        self.test_faults.queue_read(TestRead::Fork(parent.pid()));
    }

    pub(in crate::process) fn queue_fork_exit_test_read(&self, parent: LiveProcessIdentity) {
        self.test_faults
            .queue_read(TestRead::ForkExit(parent.pid()));
    }

    pub(in crate::process) fn queue_exit_test_read(&self, parent: LiveProcessIdentity) {
        self.test_faults.queue_read(TestRead::Exit(parent.pid()));
    }

    pub(in crate::process) fn queue_reconcile_fork_exit_test_read(
        &mut self,
        parent: LiveProcessIdentity,
    ) {
        self.test_faults.reconcile_read = Some(TestRead::ForkExit(parent.pid()));
    }

    pub(in crate::process) fn queue_before_unregister_fork_exit_test_read(
        &mut self,
        parent: LiveProcessIdentity,
    ) {
        self.test_faults.before_unregister_read = Some(TestRead::ForkExit(parent.pid()));
    }
}
