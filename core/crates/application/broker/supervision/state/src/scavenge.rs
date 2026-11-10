use crate::constants::{ADOPTION_SUFFIX, RECORD_PREFIX, RECORD_SUFFIX, TERMINATION_SUFFIX};
use crate::record::SupervisionRecord;
use anyhow::Result;
use std::collections::HashSet;
use yolo_process::{BootIdentityRelation, ProcessIdentityObservation};
use yolo_remote_broker::BrokerSupervisionStore;

const MAX_ACTIVE_RECORDS: usize = 128;
const MAX_RESIDUE_ENTRIES_PER_RUN: usize = 2_048;
const CAPACITY_EXHAUSTED: &str = "broker supervision record capacity exhausted";
const RECOVERY_FAILURE_SEPARATOR: &str = "; later recovery failure: ";

struct Family {
    stem: String,
    launch_id: String,
    record: String,
    adoption: String,
    termination: String,
    authority: yolo_remote_broker::BrokerLaunchAuthority,
}

enum Disposition {
    Removed,
    RequestResidue,
    Retained,
}

pub(super) fn run(store: &BrokerSupervisionStore) -> Result<()> {
    let mut retained = HashSet::new();
    let mut capacity_exhausted = false;
    let mut recovery_failure = None;
    let mut residue = Vec::new();
    let scan = store.visit_names_with_prefix(RECORD_PREFIX, |name| {
        let Some(family) = Family::parse(&name) else {
            return;
        };
        if retained.contains(&family.stem) {
            return;
        }
        match classify(store, &family) {
            Ok(Disposition::Removed) => {}
            Ok(Disposition::RequestResidue) => {
                if residue.len() + 2 <= MAX_RESIDUE_ENTRIES_PER_RUN {
                    residue.extend([family.adoption, family.termination]);
                }
            }
            Ok(Disposition::Retained) => {
                retain(&mut retained, family.stem, &mut capacity_exhausted);
            }
            Err(error) => {
                retain(&mut retained, family.stem, &mut capacity_exhausted);
                record_error(&mut recovery_failure, error);
            }
        }
    });
    if capacity_exhausted || retained.len() >= MAX_ACTIVE_RECORDS {
        anyhow::bail!(CAPACITY_EXHAUSTED);
    }
    let scan = combine(scan, store.remove_inert_entries(&residue).err());
    combine(scan, recovery_failure)
}

fn classify(store: &BrokerSupervisionStore, family: &Family) -> Result<Disposition> {
    let record = match store.read::<SupervisionRecord>(&family.record) {
        Ok(record) => record,
        Err(error) if io_kind(&error) == Some(std::io::ErrorKind::NotFound) => {
            return Ok(Disposition::RequestResidue);
        }
        Err(_) => return Ok(Disposition::Retained),
    };
    if record.verify_recovery(&family.launch_id).is_err() {
        return Ok(Disposition::Retained);
    }
    let current_boot = yolo_process::boot_id();
    match yolo_process::boot_identity_relation(record.recovery_boot_id(), &current_boot) {
        BootIdentityRelation::Different => remove(store, family),
        BootIdentityRelation::Unknown => Ok(Disposition::Retained),
        BootIdentityRelation::Same => recover_same_boot(store, family, &record),
    }
}

fn recover_same_boot(
    store: &BrokerSupervisionStore,
    family: &Family,
    record: &SupervisionRecord,
) -> Result<Disposition> {
    match record.supervisor_identity().observe() {
        ProcessIdentityObservation::Matching | ProcessIdentityObservation::Unknown => {
            Ok(Disposition::Retained)
        }
        ProcessIdentityObservation::Gone => match record.broker_identity() {
            None => remove(store, family),
            Some(broker) => match broker.observe() {
                ProcessIdentityObservation::Gone => remove(store, family),
                ProcessIdentityObservation::Unknown => Ok(Disposition::Retained),
                ProcessIdentityObservation::Matching => {
                    store.contain_recorded_broker(&family.record, family.authority)?;
                    remove(store, family)
                }
            },
        },
    }
}

fn remove(store: &BrokerSupervisionStore, family: &Family) -> Result<Disposition> {
    let mut failure = None;
    for name in [&family.adoption, &family.termination, &family.record] {
        if let Err(error) = store.remove(name) {
            record_error(&mut failure, error);
        }
    }
    combine(Ok(()), failure).map(|_| Disposition::Removed)
}

fn retain(retained: &mut HashSet<String>, stem: String, exhausted: &mut bool) {
    if retained.len() < MAX_ACTIVE_RECORDS {
        retained.insert(stem);
    } else {
        *exhausted = true;
    }
}

fn record_error(slot: &mut Option<anyhow::Error>, error: anyhow::Error) {
    if let Some(previous) = slot.take() {
        *slot = Some(anyhow::anyhow!(
            "{previous:#}{RECOVERY_FAILURE_SEPARATOR}{error:#}"
        ));
    } else {
        *slot = Some(error);
    }
}

fn combine(primary: Result<()>, secondary: Option<anyhow::Error>) -> Result<()> {
    match (primary, secondary) {
        (result, None) => result,
        (Ok(()), Some(error)) => Err(error),
        (Err(primary), Some(secondary)) => {
            anyhow::bail!("{primary:#}{RECOVERY_FAILURE_SEPARATOR}{secondary:#}")
        }
    }
}

impl Family {
    fn parse(name: &str) -> Option<Self> {
        let tail = name.strip_prefix(RECORD_PREFIX)?;
        let launch_id = [TERMINATION_SUFFIX, ADOPTION_SUFFIX, RECORD_SUFFIX]
            .into_iter()
            .find_map(|suffix| tail.strip_suffix(suffix))?;
        let authority = launch_id
            .parse::<yolo_remote_broker::BrokerLaunchAuthority>()
            .ok()?;
        if authority.to_string() != launch_id {
            return None;
        }
        let stem = format!("{RECORD_PREFIX}{launch_id}");
        Some(Self {
            record: format!("{stem}{RECORD_SUFFIX}"),
            adoption: format!("{stem}{ADOPTION_SUFFIX}"),
            termination: format!("{stem}{TERMINATION_SUFFIX}"),
            authority,
            stem,
            launch_id: launch_id.to_owned(),
        })
    }
}

fn io_kind(error: &anyhow::Error) -> Option<std::io::ErrorKind> {
    error.chain().find_map(|source| {
        source
            .downcast_ref::<std::io::Error>()
            .map(std::io::Error::kind)
    })
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/scavenge_capacity.rs"]
mod capacity_tests;
#[cfg(all(test, target_os = "macos", feature = "yolo-integration-tests"))]
#[path = "../integration_tests/scavenge_containment.rs"]
mod containment_tests;
#[cfg(all(test, target_os = "macos", feature = "yolo-integration-tests"))]
#[path = "../integration_tests/scavenge_family.rs"]
mod family_tests;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/scavenge.rs"]
mod tests;
