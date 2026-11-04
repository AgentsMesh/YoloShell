use crate::process::{ProcessIdentity, ProcessIdentityObservation};
use crate::pty_custody::CustodyHolderIdentity;
use anyhow::{Context, Result, ensure};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use uuid::Uuid;

const CANONICAL_HOME_CONTEXT: &str = "canonicalize test custody guardian home";
const CLEANUP_TIMEOUT: Duration = Duration::from_secs(3);
const CLEANUP_POLL_INTERVAL: Duration = Duration::from_millis(10);
const CLEANUP_TIMEOUT_DETAIL: &str = "exact test custody guardian cleanup timed out";

static PROCESSES: OnceLock<Mutex<HashMap<GuardianKey, HashSet<ProcessIdentity>>>> = OnceLock::new();

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct GuardianKey {
    home: PathBuf,
    terminal_id: Uuid,
}

pub(super) fn register(home: &Path, terminal_id: Uuid, holder: CustodyHolderIdentity) -> Result<()> {
    let key = GuardianKey {
        home: canonical_home(home)?,
        terminal_id,
    };
    let identity = ProcessIdentity {
        pid: holder.pid,
        start_time: holder.start_time,
    };
    processes()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .entry(key)
        .or_default()
        .insert(identity);
    Ok(())
}

pub fn terminate_for_home(home: &Path) -> Result<()> {
    let home = canonical_home(home)?;
    let identities = drain_home(&home);
    let mut killed = Vec::new();
    for identity in &identities {
        if identity.observe() == ProcessIdentityObservation::Matching
            && (*identity).signal(libc::SIGKILL)?
        {
            killed.push(identity.pid);
        }
    }
    wait_until_gone(&identities)?;
    for pid in killed {
        yolo_coverage_test_support::discard_aborted_process_profile(pid)?;
    }
    Ok(())
}

pub fn contains(identity: ProcessIdentity) -> bool {
    processes()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .values()
        .any(|identities| identities.contains(&identity))
}

fn canonical_home(home: &Path) -> Result<PathBuf> {
    std::fs::canonicalize(home).context(CANONICAL_HOME_CONTEXT)
}

fn processes() -> &'static Mutex<HashMap<GuardianKey, HashSet<ProcessIdentity>>> {
    PROCESSES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn drain_home(home: &Path) -> Vec<ProcessIdentity> {
    let mut processes = processes()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let keys: Vec<_> = processes
        .keys()
        .filter(|key| key.home == home)
        .cloned()
        .collect();
    keys.into_iter()
        .flat_map(|key| processes.remove(&key).into_iter().flatten())
        .collect()
}

fn wait_until_gone(identities: &[ProcessIdentity]) -> Result<()> {
    let deadline = Instant::now() + CLEANUP_TIMEOUT;
    while identities
        .iter()
        .any(|identity| identity.observe() != ProcessIdentityObservation::Gone)
    {
        ensure!(Instant::now() < deadline, CLEANUP_TIMEOUT_DETAIL);
        std::thread::sleep(CLEANUP_POLL_INTERVAL);
    }
    Ok(())
}

#[cfg(feature = "yolo-integration-tests")]
#[path = "../integration_tests/test_processes_tests.rs"]
mod tests;
