use std::path::Path;
use yolo_crypto::sha256_hex;

pub(super) fn config_revision(
    config: &[u8],
    known_hosts: &[u8],
    identities: &[Vec<u8>],
    agent_socket: Option<&Path>,
    artifact: Option<&[u8]>,
) -> String {
    let mut material = Vec::new();
    append_digest(&mut material, config);
    append_digest(&mut material, known_hosts);
    for identity in identities {
        append_digest(&mut material, identity);
    }
    append_digest(&mut material, &path_bytes(agent_socket));
    append_digest(&mut material, artifact.unwrap_or_default());
    sha256_hex(&material)
}

fn append_digest(material: &mut Vec<u8>, value: &[u8]) {
    material.extend(russh::keys::HashAlg::Sha256.digest(value));
}

#[cfg(unix)]
fn path_bytes(path: Option<&Path>) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    path.map(|value| value.as_os_str().as_bytes().to_vec())
        .unwrap_or_default()
}
