use super::super::config::BrokerConfig;
use super::super::config_generation_pending::{GenerationRoot, OpenedGeneration};
use super::super::constants::GENERATION_CONFIG_FILE;
use super::{BrokerConfigGeneration, GenerationPublishStage};
use crate::remote_broker::IdentityPassphrases;
use anyhow::{Result, ensure};
use std::ffi::CStr;
use std::path::PathBuf;

pub(super) fn verified_generation<F>(
    root: &GenerationRoot,
    name: &CStr,
    opened: OpenedGeneration,
    directory: PathBuf,
    revision: String,
    passphrases: &IdentityPassphrases,
    hook: &mut F,
) -> Result<BrokerConfigGeneration>
where
    F: FnMut(GenerationPublishStage) -> Result<()>,
{
    hook(GenerationPublishStage::ExistingOpened)?;
    opened.verify(root, name)?;
    let config_path = directory.join(GENERATION_CONFIG_FILE);
    let (loaded, credential_binding_nonce) =
        BrokerConfig::load_snapshot_rebound(&config_path, passphrases)?;
    ensure!(
        loaded.revision() == revision,
        "broker config generation revision differs"
    );
    let host_id = loaded.host_id().to_owned();
    let profile_authority_sha256 = loaded.profile_authority_sha256().to_owned();
    opened.verify(root, name)?;
    Ok(BrokerConfigGeneration {
        hub_transport: loaded.hub_transport(),
        config_path,
        host_id,
        profile_authority_sha256,
        revision,
        credential_binding_nonce,
    })
}
