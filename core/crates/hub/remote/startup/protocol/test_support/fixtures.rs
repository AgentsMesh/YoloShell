use uuid::Uuid;
use yolo_build_image::BuildImage;
use yolo_distribution_bundle::{
    Architecture, ArtifactDescriptor, BundleArtifacts, BundleDescriptor, Platform, Target,
};
use yolo_hub_build_protocol::ServiceBuild;
use yolo_hub_delegation::{ED25519_PUBLIC_KEY_BYTES, PeerIdentity};
use yolo_hub_identity::HubId;
use yolo_remote_provisioning_protocol::*;
use yolo_hub_routing::Right;
#[path = "constants.rs"]
mod constants;
pub use constants::*;

pub fn peer(hub: u128, key: u8) -> PeerIdentity {
    PeerIdentity::new(
        HubId::from_uuid(Uuid::from_u128(hub)).unwrap(),
        [key; ED25519_PUBLIC_KEY_BYTES],
    )
    .unwrap()
}
pub fn request() -> RemoteBootstrapRequest {
    RemoteBootstrapRequest::new(RemoteBootstrapRequestFields {
        schema_version: REMOTE_BOOTSTRAP_SCHEMA_VERSION,
        request_id: Uuid::new_v4(),
        bundle: BundleDescriptor::new(
            REVISION,
            VERSION,
            Target {
                platform: Platform::Linux,
                architecture: Architecture::X86_64,
            },
            BundleArtifacts {
                runtime: ArtifactDescriptor::new(RUNTIME_DIGEST, IMAGE_BYTES).unwrap(),
            },
        )
        .unwrap(),
        expected_peer: Some(peer(REMOTE_HUB, REMOTE_KEY)),
        consumer: peer(CONSUMER_HUB, CONSUMER_KEY),
        rights: [Right::Observe, Right::TerminalRead].into_iter().collect(),
        remaining_reexports: 0,
    })
    .unwrap()
}
pub fn receipt(request: &RemoteBootstrapRequest) -> RemoteBootstrapReceiptFields {
    RemoteBootstrapReceiptFields {
        schema_version: REMOTE_BOOTSTRAP_SCHEMA_VERSION,
        request_id: request.fields().request_id,
        peer: peer(REMOTE_HUB, REMOTE_KEY),
        peer_socket: "/run/user/1000/yolo/hub/peer.sock".into(),
        configuration_revision: FIRST_REVISION,
        bundle_id: request.fields().bundle.build_id().into(),
        build: ServiceBuild {
            host: BuildImage::new(RUNTIME_DIGEST.into(), IMAGE_BYTES).unwrap(),
            hub: BuildImage::new(RUNTIME_DIGEST.into(), IMAGE_BYTES).unwrap(),
            file_helper: BuildImage::new(RUNTIME_DIGEST.into(), IMAGE_BYTES).unwrap(),
        },
        consumer: request.fields().consumer,
        rights: request.fields().rights.clone(),
        remaining_reexports: request.fields().remaining_reexports,
    }
}
