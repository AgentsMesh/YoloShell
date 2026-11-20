use crate::hub::federation_support::RunningNode;
use tokio::time::{Instant, timeout_at};
use yolo_hub_client::ClientError;
use yolo_hub_identity::RuntimeTarget;

const FORWARD_FILTER: &str = "off,yolo_hub_federation::forward_diagnostics=warn,yolo_hub_local_backend::native_diagnostics=debug,yolo_hub_agent_projection::project=debug";
const SUBSCRIBER_PRESENT: &str = "forward diagnostic subscriber was already configured";
const RECOVERY_ATTACH_FAILED: &str =
    "single recovery attachment failed after live inventory readiness";

pub fn initialize() {
    if tracing_subscriber::fmt()
        .with_env_filter(FORWARD_FILTER)
        .with_ansi(false)
        .with_test_writer()
        .try_init()
        .is_err()
    {
        eprintln!("{SUBSCRIBER_PRESENT}");
    }
}

pub async fn fail(
    entry: &RunningNode,
    origin: &RunningNode,
    target: RuntimeTarget,
    error: &ClientError,
    deadline: Instant,
) -> ! {
    let peers = timeout_at(deadline, entry.node.connected_peers())
        .await
        .ok();
    let publication = origin.node.source();
    let revision = publication.as_ref().map(|source| source.payload.revision);
    let runtime = publication.as_ref().and_then(|source| {
        source
            .payload
            .terminals
            .iter()
            .find(|session| session.fields().address == target.address)
            .and_then(|session| session.fields().runtime)
            .map(|runtime| (runtime.target, runtime.status))
    });
    let code = match error {
        ClientError::Remote(code) => Some(*code),
        _ => None,
    };
    // Public identities/status only: never Debug-print session/request/error bodies.
    panic!(
        "{RECOVERY_ATTACH_FAILED}: entry={} target={target:?} public_error={code:?} connected_peers={peers:?} origin_publication_revision={revision:?} origin_runtime={runtime:?}",
        entry.node.hub_id()
    );
}
