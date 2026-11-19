use super::WorkerNamespace;
use crate::model::RuntimeRecord;
use crate::paths::Paths;
use crate::paths::identity::FileIdentity;
use crate::pty_custody::{
    CustodyDurableRecord, CustodyGuardianPreparation, CustodyOffer, CustodyProfile, CustodyRequest,
    CustodyRuntimeIdentity,
};
use anyhow::{Result, ensure};

const ACTIVE_GUARDIAN_DETAIL: &str = "active PTY custody requires an exact detached guardian";
const GUARDIAN_IDENTITY_DETAIL: &str = "PTY custody guardian binding is not exact";
const RELEASED_GUARDIAN_DETAIL: &str = "released PTY custody cannot retain a guardian binding";
const RUNTIME_IDENTITY_DETAIL: &str = "PTY custody guardian runtime publication changed";

pub(crate) struct ValidatedPreparation {
    pub(crate) record: CustodyDurableRecord,
    pub(crate) identity: FileIdentity,
}

pub(crate) struct ValidatedHandshake {
    pub(crate) runtime_identity: FileIdentity,
    pub(crate) prepared: Option<ValidatedPreparation>,
}

pub(crate) fn validate(
    namespace: &WorkerNamespace,
    paths: &Paths,
    runtime: &RuntimeRecord,
    durable: &CustodyDurableRecord,
    durable_identity: FileIdentity,
    request: &CustodyRequest,
    offer: &CustodyOffer,
    guardian: Option<CustodyGuardianPreparation>,
) -> Result<ValidatedHandshake> {
    let path = paths.runtime_record(runtime.terminal_id);
    let (current, runtime_identity): (RuntimeRecord, FileIdentity) =
        namespace.read_json_with_identity(&path)?;
    ensure!(current == *runtime, RUNTIME_IDENTITY_DETAIL);
    if offer.profile == CustodyProfile::ReleasedTerminal {
        ensure!(guardian.is_none(), RELEASED_GUARDIAN_DETAIL);
        return Ok(ValidatedHandshake {
            runtime_identity,
            prepared: None,
        });
    }
    #[cfg(target_os = "macos")]
    ensure!(guardian.is_some(), ACTIVE_GUARDIAN_DETAIL);
    let Some(preparation) = guardian else {
        return Ok(ValidatedHandshake {
            runtime_identity,
            prepared: None,
        });
    };
    let guardian = preparation.binding();
    let runtime = CustodyRuntimeIdentity::from_record(runtime);
    let (custody, custody_identity): (CustodyDurableRecord, _) =
        namespace.read_json_with_identity(&paths.pty_custody(runtime.terminal_id))?;
    ensure!(
        guardian.matches(request, offer)
            && guardian.matches_runtime(&runtime)
            && guardian.process().matches(),
        GUARDIAN_IDENTITY_DETAIL
    );
    ensure!(
        preparation.runtime_file_identity() == runtime_identity
            && preparation.predecessor_file_identity() == durable_identity
            && custody_identity == preparation.custody_file_identity()
            && custody
                .prepared_guardian_candidate()
                .is_some_and(|lease| guardian.matches_lease_identity(&lease)),
        GUARDIAN_IDENTITY_DETAIL
    );
    let expected = durable
        .prepare_guardian(request, offer, guardian)
        .or_else(|| durable.supersede_prepared_guardian(request, offer, guardian));
    ensure!(expected == Some(custody.clone()), GUARDIAN_IDENTITY_DETAIL);
    Ok(ValidatedHandshake {
        runtime_identity,
        prepared: Some(ValidatedPreparation {
            record: custody,
            identity: custody_identity,
        }),
    })
}
