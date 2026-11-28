use crate::connector_admission::{admission, native_owned};
use yolo_hub_connector_registry::{Catalogue, RuntimeAdmission};
use yolo_hub_identity::{AgentTarget, HubId, RuntimeEpoch, RuntimeTarget, TerminalAddress, TerminalId};
use yolo_terminal_model::TerminalView;

pub(crate) fn primary(
    hub: HubId,
    view: &TerminalView,
    native: Option<AgentTarget>,
    connectors: &Catalogue,
) -> Option<AgentTarget> {
    if native_owned(view) {
        return native;
    }
    let target = current_target(hub, view);
    target
        .filter(|target| matches!(admission(*target, view), RuntimeAdmission::Available(_)))
        .and_then(|target| connectors.primaries.get(&target).copied())
        .or(native)
}

fn current_target(hub: HubId, view: &TerminalView) -> Option<RuntimeTarget> {
    Some(RuntimeTarget {
        address: TerminalAddress {
            hub_id: hub,
            terminal_id: TerminalId::from_uuid(view.manifest.terminal_id).ok()?,
        },
        runtime_epoch: RuntimeEpoch::from_uuid(view.runtime.as_ref()?.runtime_epoch).ok()?,
    })
}
