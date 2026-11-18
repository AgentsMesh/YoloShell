#[path = "handlers/admission.rs"]
mod admission;
#[path = "authority/authority.rs"]
mod authority;
#[path = "authority/authority_control.rs"]
mod authority_control;
#[path = "authority/authority_registry.rs"]
mod authority_registry;
#[path = "handlers/blocking_custody.rs"]
mod blocking_custody;
#[path = "hook/cancellation.rs"]
mod cancellation;
#[path = "hook/constants.rs"]
mod constants;
#[path = "handlers/control_source.rs"]
mod control_source;
#[path = "hook/error.rs"]
mod error;
#[path = "handlers/handler.rs"]
mod handler;
#[path = "hook/hook_context.rs"]
mod hook_context;
#[path = "hook/hook_policy.rs"]
mod hook_policy;
#[path = "ingress/ingress_listener_completion.rs"]
mod ingress_listener_completion;
#[path = "ingress/ingress_operation.rs"]
mod ingress_operation;
#[path = "ingress/ingress_supervisor.rs"]
mod ingress_supervisor;
#[path = "ingress/ingress_task.rs"]
mod ingress_task;
#[path = "intake/intake_task.rs"]
mod intake_task;
#[path = "handlers/live_handler.rs"]
mod live_handler;
#[path = "handlers/live_handoff.rs"]
mod live_handoff;
#[path = "hook/marker_tail.rs"]
mod marker_tail;
#[path = "native/native_admission.rs"]
mod native_admission;
#[path = "native/native_first_event.rs"]
mod native_first_event;
#[path = "outbox/outbox_authority.rs"]
mod outbox_authority;
#[path = "hook/outcome.rs"]
mod outcome;
#[path = "hook/peer_identity.rs"]
mod peer_identity;
#[path = "projection/projection.rs"]
mod projection;
#[path = "projection/projection_completion.rs"]
mod projection_completion;
mod protocol;
#[path = "outbox/publication.rs"]
mod publication;
#[path = "recovery/recovery.rs"]
mod recovery;
#[path = "recovery/recovery_install.rs"]
mod recovery_install;
#[path = "recovery/recovery_retired.rs"]
mod recovery_retired;
#[path = "recovery/recovery_round.rs"]
mod recovery_round;
#[path = "recovery/recovery_route.rs"]
mod recovery_route;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../../test_support/unit/codex_hook/recovery/recovery_test_hook.rs"]
mod recovery_test_hook;
mod reduction_probe;
mod replay;
#[path = "handlers/routed_handler.rs"]
mod routed_handler;
#[path = "router/router.rs"]
mod router;
#[path = "runtime/runtime.rs"]
mod runtime;
#[path = "server/server.rs"]
mod server;

pub(in crate::worker) use ingress_supervisor::CodexHookIngressSupervisor;
pub(in crate::worker) use control_source::with_control_source;
#[cfg(test)]
pub(in crate::worker) use projection::{
    ProjectionRoundProbe, complete_task_for_test as complete_projection_task_for_test,
    run_idle_for_test as run_idle_projection_round_for_test,
    supervise_closed_for_test as supervise_closed_projection_for_test,
};
pub(in crate::worker) use projection::{
    invalidate_for_shutdown as invalidate_projection_for_shutdown, spawn as spawn_projection,
};

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../test_support/codex_process_exit.rs"]
#[cfg(test)]
mod process_exit_integration_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../../unit_tests/src/codex_hook/mod.rs"]
mod tests;
