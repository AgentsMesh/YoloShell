use super::{fixture::Fixture, lifecycle_constants::*, lifecycle_support::*};
use std::sync::Arc;
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::PublicError;
use yolo_hub_ssh_profile_runtime::SshProfileRuntime;

#[tokio::test(flavor = "current_thread")]
async fn final_runtime_owner_drop_closes_live_tunnels_and_releases_the_store_without_a_scheduler_turn()
 {
    let fixture = Fixture::new();
    let tunnels = Tunnels::open(&fixture).await;
    let Fixture {
        root,
        state,
        runtime,
        dispatcher,
    } = fixture;
    drop(dispatcher);
    assert_eq!(Arc::strong_count(&runtime), SOLE_RUNTIME_OWNER);
    let previous = Arc::downgrade(&runtime);
    drop(runtime);
    assert!(previous.upgrade().is_none());
    // Deliberately keep both authenticated PeerConnections and their watcher handles alive.
    let reopened = SshProfileRuntime::open(state.clone(), None).unwrap();
    // No await occurred after the last owner was dropped or before reopening its store.
    drop(reopened);
    tunnels.require_closed().await;
    tunnels.finish().await;
    drop(tunnels);
    drop(state);
    drop(root);
}

#[tokio::test(flavor = "current_thread")]
async fn registry_identity_loss_read_closes_every_profile_tunnel_before_its_response() {
    for replacement in [Replacement::Record, Replacement::Root] {
        let fixture = Fixture::new();
        let tunnels = Tunnels::open(&fixture).await;
        replace_same_json(&fixture, replacement);
        let result = ready_now(fixture.call(Command::GetSshProfiles {}));
        assert_eq!(result, Err(PublicError::Unreachable));
        assert_eq!(
            ready_now(fixture.call(Command::GetSshProfiles {})),
            Err(PublicError::Unreachable)
        );
        tunnels.require_closed().await;
        tunnels.finish().await;
        drop(tunnels);
    }
}
