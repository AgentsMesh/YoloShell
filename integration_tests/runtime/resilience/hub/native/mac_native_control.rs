use super::{assets::Assets, observe};
use crate::Harness;
use crate::hub::federation_support::{RunningNode, TEMP_ROOT, deadline};
use yolo_hub_identity::RuntimeTarget;
use yolo_hub_rpc_protocol::{Command, Reply};

#[path = "mac_native_control_constants.rs"]
mod constants;
#[path = "mac_native_control_probe.rs"]
mod probe;

pub async fn exercise(
    entry: &RunningNode,
    host: &Harness,
    assets: &Assets,
    target: RuntimeTarget,
    bystander: RuntimeTarget,
) {
    let before = observe::coherent_worker(host, target).await;
    let other_before = observe::coherent_worker(host, bystander).await;
    let selected = observe::current(entry, target, None).await;
    let previous_signals = std::fs::read(&assets.target_marker).unwrap().len();
    observe::count(&assets.other_marker, usize::default()).await;
    let root = tempfile::tempdir_in(TEMP_ROOT).unwrap();
    let report = probe::run(entry, root.path(), target, selected.cursor().target()).await;
    assert_eq!(report.entry_hub, entry.node.hub_id());
    assert_eq!(report.terminal, target);
    assert_eq!(report.agent, selected.cursor().target());
    assert!(!report.request_id.is_nil());
    assert!(report.helper_process_id > i32::default());
    assert!(report.helper_unique_id > u64::default());
    assert!(report.readonly_blocked && report.sent && report.focus_unchanged);
    assert!(report.helper_unchanged && report.launch_unchanged && report.selection_unchanged);
    observe::count(
        &assets.target_marker,
        previous_signals
            .checked_add(constants::EXTRA_DELIVERY)
            .unwrap(),
    )
    .await;
    observe::count(&assets.other_marker, usize::default()).await;
    observe::identities_unchanged(host, target, &before).await;
    observe::identities_unchanged(host, bystander, &other_before).await;
    let after = observe::current(entry, target, None).await;
    assert_eq!(after.cursor().target(), selected.cursor().target());
    // The probe closes only its helper; the origin Agent remains live and the
    // retained writer grant must have been released without another acquisition.
    let Reply::LeaseObservation { lease } = entry
        .client()
        .await
        .call(
            target.address.hub_id,
            Command::ObserveLease { target },
            deadline(),
        )
        .await
        .unwrap()
    else {
        panic!()
    };
    assert!(lease.is_none());
}
