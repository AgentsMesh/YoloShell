use crate::{ArtifactRole, BundleFields, constants::IDENTITY_DOMAIN};

/// Schema 2: domain bytes, big-endian u16 schema, then revision/version/platform/architecture
/// as UTF-8 strings each prefixed by a big-endian u64 byte length. Append the Runtime
/// artifact's length-prefixed role and lowercase digest, then big-endian u64 size.
/// The build_id field never participates. JSON key order and whitespace are irrelevant.
pub(crate) fn build_id(fields: &BundleFields) -> String {
    let mut bytes = IDENTITY_DOMAIN.to_vec();
    bytes.extend_from_slice(&fields.schema_version.to_be_bytes());
    for value in [
        fields.source_revision.as_str(),
        fields.version.as_str(),
        fields.target.platform.as_str(),
        fields.target.architecture.as_str(),
    ] {
        append(&mut bytes, value);
    }
    for role in ArtifactRole::ALL {
        let artifact = fields.artifacts.artifact(role);
        append(&mut bytes, role.as_str());
        append(&mut bytes, artifact.sha256());
        bytes.extend_from_slice(&artifact.size_bytes().to_be_bytes());
    }
    yolo_crypto::sha256_hex(&bytes)
}

fn append(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend_from_slice(&(value.len() as u64).to_be_bytes());
    bytes.extend_from_slice(value.as_bytes());
}
