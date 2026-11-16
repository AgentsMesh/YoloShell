use crate::constants::*;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    io::Read,
    path::{Path, PathBuf},
};
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_state::HubState;
use yolo_paths::{identity::FileIdentity, private_file::PrivateFileEntry};
use yolo_remote_broker::{BrokerConfigGeneration, BrokerLaunchDeadline, BrokerPaths, BrokerStatus};
use yolo_ssh_profile_protocol::SshProfileRef;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HubBrokerLaunchScope {
    pub owner: PeerIdentity,
    pub hub_state: PathBuf,
    pub host_home: PathBuf,
    pub profile: SshProfileRef,
    pub config_revision: String,
    pub config_source: PathBuf,
    pub config_identity: FileIdentity,
    pub config_sha256: String,
    pub broker_state: PathBuf,
    pub broker_state_anchor: String,
    pub deadline: String,
}

impl HubBrokerLaunchScope {
    #[allow(clippy::too_many_arguments)]
    pub fn capture(
        owner: &HubState,
        host_home: &Path,
        profile: SshProfileRef,
        config_source: PathBuf,
        broker_paths: &BrokerPaths,
        config_revision: String,
        deadline: BrokerLaunchDeadline,
    ) -> Result<Self> {
        owner.verify()?;
        broker_paths.verify_state()?;
        let (config_identity, config_sha256) = fingerprint(&config_source)?;
        let scope = Self {
            owner: owner.peer_identity(),
            hub_state: owner.directory()?.path().to_owned(),
            host_home: host_home.to_owned(),
            profile,
            config_revision,
            config_source,
            config_identity,
            config_sha256,
            broker_state: broker_paths.state_dir().to_owned(),
            broker_state_anchor: broker_paths.state_anchor().to_string(),
            deadline: deadline.to_string(),
        };
        scope.verify()?;
        Ok(scope)
    }
    pub fn deadline(&self) -> Result<tokio::time::Instant> {
        self.deadline.parse::<BrokerLaunchDeadline>()?.to_instant()
    }
    pub fn config_sha256(&self) -> &str {
        &self.config_sha256
    }
    pub fn verify(&self) -> Result<()> {
        self.deadline()?;
        for path in [
            &self.hub_state,
            &self.host_home,
            &self.config_source,
            &self.broker_state,
        ] {
            yolo_paths::validation::validate_absolute(path)?;
        }
        yolo_remote_broker::validate_broker_config_revision(&self.config_revision)?;
        ensure!(
            HubState::inspect_peer_identity(&self.hub_state, &self.host_home)? == Some(self.owner),
            SCOPE_MISMATCH
        );
        let broker = BrokerPaths::open_existing(self.broker_state.clone())?;
        ensure!(
            broker.state_anchor().to_string() == self.broker_state_anchor,
            SCOPE_MISMATCH
        );
        broker.verify_state()?;
        ensure!(
            fingerprint(&self.config_source)? == (self.config_identity, self.config_sha256.clone()),
            SCOPE_MISMATCH
        );
        Ok(())
    }
    pub fn verify_generation(&self, generation: &BrokerConfigGeneration) -> Result<()> {
        self.verify()?;
        ensure!(
            generation.hub_transport()
                && generation.host_id() == self.profile.profile_id()
                && generation.profile_authority_sha256() == self.profile.authority_sha256()
                && generation.revision() == self.config_revision,
            SCOPE_MISMATCH
        );
        Ok(())
    }
    pub(crate) fn verify_status(&self, status: &BrokerStatus) -> Result<()> {
        ensure!(
            status.protocol_version == yolo_remote_broker::BROKER_PROTOCOL_VERSION
                && status.host_id == self.profile.profile_id()
                && status.config_revision == self.config_revision
                && !status.instance_id.is_nil()
                && status.connection_id.is_some_and(|id| !id.is_nil())
                && status.connection_generation > u64::default()
                && status.supports(yolo_remote_broker::HUB_TRANSPORT_CAPABILITY),
            SCOPE_MISMATCH
        );
        Ok(())
    }
}

fn fingerprint(path: &Path) -> Result<(FileIdentity, String)> {
    let entry = PrivateFileEntry::open(path)?;
    let opened = entry.open_read(MAX_CONFIG_BYTES)?;
    let identity = opened.identity();
    let mut bytes = zeroize::Zeroizing::new(Vec::new());
    opened
        .file()
        .take(MAX_CONFIG_BYTES + COUNTER_STEP)
        .read_to_end(&mut bytes)?;
    identity.verify_file_bounded(
        opened.file(),
        bytes.len() as u64,
        path,
        MAX_CONFIG_BYTES,
    )?;
    entry.verify_exact_identity(identity, MAX_CONFIG_BYTES)?;
    Ok((identity, yolo_crypto::sha256_hex(&bytes)))
}
