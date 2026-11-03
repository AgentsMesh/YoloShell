use super::LiveProcessIdentity;
use anyhow::{Context, Result, ensure};
use std::collections::HashMap;
use std::os::fd::{AsRawFd, OwnedFd};

#[path = "process_events_macos_event.rs"]
mod event;
#[path = "process_events_macos_fork.rs"]
mod fork;
use event::{empty_event, process_event};

const INVALID_SYSTEM_RESULT: libc::c_int = -1;
const NO_EVENTS: libc::c_int = 0;
const SINGLE_EVENT: libc::c_int = 1;
const EVENT_BATCH_CAPACITY: usize = 64;
const OBSERVED_NOTES: u32 = libc::NOTE_FORK | libc::NOTE_EXEC | libc::NOTE_EXIT;
const REGISTER_CONTEXT: &str = "register macOS process event invalidation";
const READ_CONTEXT: &str = "read macOS process event invalidation";
const EVENT_DETAIL: &str = "invalid macOS process event invalidation";
const INITIAL_ROOT_GENERATION_DETAIL: &str =
    "macOS process event root generation is not uniquely current";
const INVALIDATED_GATE_DETAIL: &str = "macOS process event gate is invalidated";

struct CurrentIdentitySelection {
    current: HashMap<u32, LiveProcessIdentity>,
    invalidated: bool,
}

pub(super) struct MacosProcessEvents {
    queue: Option<OwnedFd>,
    root: LiveProcessIdentity,
    root_exited: bool,
    registered: HashMap<u32, LiveProcessIdentity>,
    unresolved_forks: std::collections::HashSet<u64>,
    fork_source_versions: HashMap<u64, u64>,
    gone_fork_sources: std::collections::HashSet<u64>,
    fork_event_version: u64,
    #[cfg(any(test, feature = "runtime-component-tests"))]
    test_faults: test_hooks::TestFaults,
}
impl MacosProcessEvents {
    pub(super) fn new(root: LiveProcessIdentity) -> Result<Self> {
        let descriptor = super::macos_private_kqueue(REGISTER_CONTEXT)?;
        Self::with_descriptor(root, descriptor)
    }
    pub(super) fn new_in_spawn_boundary(
        root: LiveProcessIdentity,
        boundary: &super::WaitableChildSpawnBoundary<'_>,
    ) -> Result<Self> {
        let descriptor = super::macos_private_kqueue_in_spawn_boundary(REGISTER_CONTEXT, boundary)?;
        Self::with_descriptor(root, descriptor)
    }
    fn with_descriptor(root: LiveProcessIdentity, descriptor: OwnedFd) -> Result<Self> {
        let mut events = Self {
            queue: Some(descriptor),
            root,
            root_exited: false,
            registered: HashMap::new(),
            unresolved_forks: std::collections::HashSet::new(),
            fork_source_versions: HashMap::new(),
            gone_fork_sources: std::collections::HashSet::new(),
            fork_event_version: u64::default(),
            #[cfg(any(test, feature = "runtime-component-tests"))]
            test_faults: test_hooks::TestFaults::default(),
        };
        for root in current_identities([root])?.current.into_values() {
            events.register_verified(root)?;
        }
        ensure!(
            events.registered.get(&root.pid()) == Some(&root),
            INITIAL_ROOT_GENERATION_DETAIL
        );
        Ok(events)
    }
    fn read(&self, events: &mut [libc::kevent]) -> Result<usize> {
        #[cfg(any(test, feature = "runtime-component-tests"))]
        if let Some(count) = self.test_faults.read(events) {
            return Ok(count);
        }
        let timeout = libc::timespec {
            tv_sec: NO_EVENTS.into(),
            tv_nsec: NO_EVENTS.into(),
        };
        self.submit(None, events, Some(&timeout))
            .context(READ_CONTEXT)
    }
    fn submit(
        &self,
        change: Option<&libc::kevent>,
        events: &mut [libc::kevent],
        timeout: Option<&libc::timespec>,
    ) -> Result<usize> {
        let queue = self.queue.as_ref().context(INVALIDATED_GATE_DETAIL)?;
        let changes = change.map_or(std::ptr::null(), std::ptr::from_ref);
        let change_count = change.map_or(NO_EVENTS, |_| SINGLE_EVENT);
        let timeout = timeout.map_or(std::ptr::null(), std::ptr::from_ref);
        // SAFETY: every pointer describes the live value and exact count passed alongside it.
        let result = unsafe {
            libc::kevent(
                queue.as_raw_fd(),
                changes,
                change_count,
                events.as_mut_ptr(),
                events.len().try_into()?,
                timeout,
            )
        };
        if result == INVALID_SYSTEM_RESULT {
            return Err(std::io::Error::last_os_error().into());
        }
        Ok(usize::try_from(result)?)
    }
}

#[path = "process_events_macos_reconcile.rs"]
mod reconcile;

fn current_identities(
    identities: impl IntoIterator<Item = LiveProcessIdentity>,
) -> Result<CurrentIdentitySelection> {
    let mut candidates = HashMap::<u32, Vec<LiveProcessIdentity>>::new();
    for identity in identities {
        candidates.entry(identity.pid()).or_default().push(identity);
    }
    let mut current = HashMap::new();
    let mut invalidated = false;
    for (pid, candidates) in candidates {
        let Some(metadata) = super::macos_kernel::metadata(pid)? else {
            invalidated = true;
            continue;
        };
        let Some(identity) = candidates
            .into_iter()
            .find(|identity| identity.unique_id() == metadata.unique_id)
        else {
            invalidated = true;
            continue;
        };
        if !identity.try_matches()? {
            invalidated = true;
            continue;
        }
        current.insert(pid, identity);
    }
    Ok(CurrentIdentitySelection {
        current,
        invalidated,
    })
}
#[cfg(any(test, feature = "runtime-component-tests"))]
#[path = "../../test_support/process_events/process_events_macos_hooks.rs"]
mod test_hooks;
fn is_missing(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<std::io::Error>()
        .and_then(std::io::Error::raw_os_error)
        .is_some_and(|code| code == libc::ESRCH || code == libc::ENOENT)
}
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/process/process_events_macos_exact_tests.rs"]
mod exact_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/process/process_events_macos_replacement_tests.rs"]
mod replacement_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/process/process_events_macos_tests.rs"]
mod tests;
