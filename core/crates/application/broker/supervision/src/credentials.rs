use anyhow::Result;
use yolo_broker_supervision_state::CredentialSource;
use yolo_remote_broker::IdentityPassphrases;

pub(crate) fn read(source: CredentialSource) -> Result<IdentityPassphrases> {
    match source {
        CredentialSource::None => Ok(IdentityPassphrases::none()),
        CredentialSource::PrivateStdin => IdentityPassphrases::read_stdin(),
    }
}
