use crate::constants::*;
use anyhow::{Result, ensure};
use yolo_distribution_bundle::ArtifactRole;
use yolo_hub_build_protocol::ServiceBuild;
use yolo_hub_federation_settings::{ExportPolicy, Peer};
use yolo_hub_management_protocol::{ConfigurationChange, ConfigurationSnapshot};
use yolo_remote_provisioning_protocol::{BootstrapError, RemoteBootstrapRequest};

pub(crate) fn verify_images(build: &ServiceBuild, request: &RemoteBootstrapRequest) -> Result<()> {
    let expected = request.fields().bundle.artifact(ArtifactRole::Runtime);
    for image in [&build.host, &build.hub, &build.file_helper] {
        ensure!(
            image.sha256() == expected.sha256() && image.size_bytes() == expected.size_bytes(),
            BootstrapError::UpgradeRequired
        );
    }
    Ok(())
}

pub(crate) fn plan(
    snapshot: &ConfigurationSnapshot,
    request: &RemoteBootstrapRequest,
) -> Result<Option<ConfigurationChange>> {
    snapshot.validate()?;
    ensure!(snapshot.applied, CONFIGURATION_NOT_READY);
    let expected = request.fields();
    let remote = snapshot.state.identity;
    ensure!(
        expected.expected_peer.is_none_or(|peer| peer == remote),
        PIN_MISMATCH
    );
    ensure!(
        remote.hub() != expected.consumer.hub() && remote.key() != expected.consumer.key(),
        PIN_MISMATCH
    );
    let mut configuration = snapshot.state.configuration.clone();
    match configuration.listen {
        Some(address) => ensure!(address.ip().is_loopback(), CONFIGURATION_CONFLICT),
        None => configuration.listen = Some((std::net::Ipv4Addr::LOCALHOST, DYNAMIC_PORT).into()),
    }
    let policy = ExportPolicy {
        origin: remote.hub(),
        rights: expected.rights.clone(),
        remaining_reexports: expected.remaining_reexports,
    };
    if let Some(peer) = configuration
        .peers
        .iter_mut()
        .find(|peer| peer.identity.hub() == expected.consumer.hub())
    {
        ensure!(peer.identity == expected.consumer, CONFIGURATION_CONFLICT);
        if let Some(export) = peer
            .exports
            .iter()
            .find(|export| export.origin == remote.hub())
        {
            ensure!(*export == policy, CONFIGURATION_CONFLICT);
        } else {
            peer.exports.push(policy);
        }
    } else {
        configuration.peers.push(Peer {
            identity: expected.consumer,
            dial: None,
            consume: false,
            exports: vec![policy],
        });
    }
    configuration.validate(remote)?;
    if configuration == snapshot.state.configuration {
        ensure!(
            snapshot
                .listen_address
                .is_some_and(|address| address.ip().is_loopback()),
            LISTENER_UNAVAILABLE
        );
        return Ok(None);
    }
    let change = ConfigurationChange {
        request_id: expected.request_id,
        expected_revision: snapshot.state.revision,
        configuration,
    };
    change.validate(remote)?;
    Ok(Some(change))
}
