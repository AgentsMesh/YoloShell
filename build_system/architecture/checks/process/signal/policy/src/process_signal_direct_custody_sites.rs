use yolo_architecture_policy_contracts::process_signal as contract;
use yolo_architecture_process_signal_foundation::process_signal_sites::{
    ReceiverSite, receiver_site,
};
use yolo_architecture_process_signal_foundation::process_signal_unidentified_custody as unidentified;

const SCOPED_CUSTODY_PATH: &str = "core/crates/runtime/engine/process/src/scoped/scoped_waitable_child.rs";
const SCOPED_CUSTODY_OWNER: &str = "ScopedWaitableChild";
const SCOPED_CUSTODY_RECEIVER: &str = "custody";
pub const DIRECT_TERMINATION_SITES: &[ReceiverSite] = &[
    receiver_site(
        contract::CHILD_RECEIVER,
        contract::COMMAND_SETTLEMENT_PATH,
        None,
        contract::TERMINATE_WITHOUT_EXACT_CUSTODY,
    ),
    receiver_site(
        contract::CHILD_RECEIVER,
        unidentified::CALLER_PATH,
        None,
        contract::REQUEST_TERMINATION,
    ),
    receiver_site(
        contract::CHILD_RECEIVER,
        contract::COMMAND_SETTLEMENT_PATH,
        None,
        contract::REQUEST_TERMINATION,
    ),
    receiver_site(
        contract::SELF_RECEIVER,
        contract::CHILD_REAPER_WORKER_PATH,
        Some(contract::DETACHED_CHILD_CUSTODY),
        contract::REQUEST_TERMINATION,
    ),
    receiver_site(
        contract::SELF_RECEIVER,
        contract::BROKER_SETTLEMENT_PATH,
        Some(contract::DETACHED_CHILD_CUSTODY),
        contract::TERMINATE,
    ),
    receiver_site(
        contract::SELF_RECEIVER,
        contract::BROKER_REAPER_WORKER_PATH,
        Some(contract::DETACHED_CHILD_CUSTODY),
        contract::TERMINATE,
    ),
    receiver_site(
        SCOPED_CUSTODY_RECEIVER,
        SCOPED_CUSTODY_PATH,
        Some(SCOPED_CUSTODY_OWNER),
        contract::TERMINATE_AND_WAIT_STATUS,
    ),
    receiver_site(
        SCOPED_CUSTODY_RECEIVER,
        SCOPED_CUSTODY_PATH,
        Some(SCOPED_CUSTODY_OWNER),
        contract::SETTLE_CHILD_ON_DROP,
    ),
];
