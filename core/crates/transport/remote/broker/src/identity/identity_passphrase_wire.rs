use serde::Serialize;
use zeroize::Zeroizing;

pub(super) struct Envelope {
    pub(super) version: u16,
    pub(super) binding_nonce: String,
    pub(super) host_id: String,
    pub(super) identity_credential_revision: Option<String>,
    pub(super) identity_paths_sha256: String,
    pub(super) passphrases: Vec<Option<SecretText>>,
}

#[derive(Clone)]
pub(in crate::remote_broker) struct EnvelopeBinding {
    pub(in crate::remote_broker) binding_nonce: String,
    pub(in crate::remote_broker) host_id: String,
    pub(in crate::remote_broker) identity_credential_revision: Option<String>,
    pub(in crate::remote_broker) identity_paths_sha256: String,
}

pub(super) struct SecretText(pub(super) Zeroizing<String>);

#[derive(Serialize)]
pub(super) struct EncodedEnvelope<'a> {
    pub(super) version: u16,
    pub(super) binding_nonce: &'a str,
    pub(super) host_id: &'a str,
    pub(super) identity_credential_revision: Option<&'a str>,
    pub(super) identity_paths_sha256: &'a str,
    pub(super) passphrases: Vec<Option<&'a str>>,
}
