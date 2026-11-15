use super::config_snapshot::{
    BrokerConfigGeneration, stage_broker_config_guarded_with_passphrases_hook,
};
use super::{BrokerPaths, BrokerReconcileGuard, IdentityPassphrases};
use anyhow::Result;
use std::path::Path;

pub fn stage_broker_config_guarded_with_passphrases(
    source: &Path,
    paths: &BrokerPaths,
    guard: &BrokerReconcileGuard,
    passphrases: &IdentityPassphrases,
) -> Result<BrokerConfigGeneration> {
    stage_broker_config_guarded_with_passphrases_hook(source, paths, guard, passphrases, |_| Ok(()))
}
