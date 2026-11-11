#[path = "native_control_assets.rs"]
mod assets;
#[path = "native_control_constants.rs"]
mod constants;
#[path = "../management/management_diagnostics.rs"]
mod diagnostics;
#[path = "native_control_flow.rs"]
mod flow;
#[path = "native_control_graph.rs"]
mod graph;
#[path = "native_control_launch.rs"]
mod launch;
#[cfg(target_os = "macos")]
#[path = "mac_native_control.rs"]
mod mac;
#[path = "native_control_observe.rs"]
mod observe;
#[path = "native_control_requests.rs"]
mod requests;
#[path = "native_control_tui.rs"]
mod tui;

#[tokio::test(flavor = "multi_thread")]
async fn real_multihop_native_interrupt_delivers_once_and_fences_stale_source_and_writer() {
    diagnostics::initialize();
    graph::run().await;
}
