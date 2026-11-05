use anyhow::{Result, ensure};
use yolo_hub_federation_settings::{Peer, PeerAddress};
use yolo_hub_management_protocol::{ConfigurationChange, ConfigurationSnapshot};
use yolo_remote_provisioning_protocol::{ProvisionOutcome, ProvisionStatus};
use yolo_ssh_profile_protocol::SshProfileRef;

pub fn configuration_contains_link(
    configuration: &ConfigurationSnapshot,
    provision: &ProvisionStatus,
) -> Result<bool> {
    let ProvisionOutcome::Ready { enrollment, .. } = &provision.fields().outcome else {
        return Ok(false);
    };
    let Some(peer) = configuration
        .state
        .configuration
        .peer(enrollment.fields().peer.hub())
    else {
        return Ok(false);
    };
    let binding = &provision.fields().request.fields().profile;
    let profile = SshProfileRef::new(
        binding.host_id.clone(),
        binding.profile_authority_sha256.clone(),
    )?;
    Ok(configuration.applied && peer.identity == enrollment.fields().peer && peer.consume
        && peer.dial.as_ref().is_some_and(|dial| {
            dial.profile == profile.profile_id()
                && dial.socket == enrollment.fields().peer_socket
        }))
}

pub fn plan_link(
    baseline: &ConfigurationSnapshot,
    request: &crate::RemoteSetupRequest,
    provision: &ProvisionStatus,
) -> Result<Option<ConfigurationChange>> {
    baseline.validate()?;
    provision.validate_for(&request.request)?;
    ensure!(
        baseline.applied
            && baseline.state.revision == request.expected_configuration_revision
            && baseline.state.identity == request.request.fields().bootstrap.fields().consumer,
        crate::constants::INVALID_REQUEST
    );
    let ProvisionOutcome::Ready { enrollment, .. } = &provision.fields().outcome else {
        anyhow::bail!(crate::constants::INVALID_REQUEST);
    };
    let remote = enrollment.fields().peer;
    let dial = PeerAddress::ssh(
        request.profile()?.profile_id().to_owned(),
        enrollment.fields().peer_socket.clone(),
    )?;
    let mut configuration = baseline.state.configuration.clone();
    match configuration
        .peers
        .iter_mut()
        .find(|peer| peer.identity.hub() == remote.hub())
    {
        Some(peer) => {
            ensure!(
                peer.identity == remote && peer.dial.as_ref().is_none_or(|value| value == &dial),
                crate::constants::INVALID_REQUEST
            );
            peer.dial = Some(dial);
            peer.consume = true;
        }
        None => configuration.peers.push(Peer {
            identity: remote,
            dial: Some(dial),
            consume: true,
            exports: Vec::new(),
        }),
    }
    configuration.validate(baseline.state.identity)?;
    if configuration == baseline.state.configuration {
        return Ok(None);
    }
    Ok(Some(ConfigurationChange {
        request_id: request.request.fields().request_id,
        expected_revision: baseline.state.revision,
        configuration,
    }))
}
