use super::super::*;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use hmac::{Hmac, Mac};
use sha1::Sha1;

const KEY: &str = "AAAAC3NzaC1lZDI1NTE5AAAAITestKey";
const OTHER_KEY: &str = "AAAAC3NzaC1lZDI1NTE5AAAan internal working treetherKey";

fn endpoint(port: u16) -> RemoteEndpoint {
    RemoteEndpoint::new("stone", "Build.Example", port).unwrap()
}

fn key(value: &str) -> HostKey {
    HostKey::new("ssh-ed25519", value).unwrap()
}

#[test]
fn exact_nonstandard_and_wildcard_hosts_are_verified() {
    let exact = KnownHosts::parse(&format!("[build.example]:2202 ssh-ed25519 {KEY}")).unwrap();
    exact.verify(&endpoint(2202), &key(KEY)).unwrap();
    let wildcard = KnownHosts::parse(&format!("*.example,!bad.example ssh-ed25519 {KEY}")).unwrap();
    wildcard.verify(&endpoint(22), &key(KEY)).unwrap();
}

#[test]
fn changed_unknown_and_revoked_keys_fail_closed() {
    let trusted = KnownHosts::parse(&format!("build.example ssh-ed25519 {KEY}")).unwrap();
    assert!(matches!(
        trusted.verify(&endpoint(22), &key(OTHER_KEY)),
        Err(RemoteError::ChangedHostKey { .. })
    ));
    assert!(matches!(
        trusted.verify(
            &RemoteEndpoint::standard("stone", "new.example").unwrap(),
            &key(KEY)
        ),
        Err(RemoteError::UnknownHostKey { .. })
    ));
    let revoked = KnownHosts::parse(&format!("@revoked build.example ssh-ed25519 {KEY}")).unwrap();
    assert!(matches!(
        revoked.verify(&endpoint(22), &key(KEY)),
        Err(RemoteError::RevokedHostKey { .. })
    ));
}

#[test]
fn openssh_hashed_hostname_is_verified() {
    let salt = b"01234567890123456789";
    let mut mac = Hmac::<Sha1>::new_from_slice(salt).unwrap();
    mac.update(b"build.example");
    let hash = mac.finalize().into_bytes();
    let host = format!("|1|{}|{}", STANDARD.encode(salt), STANDARD.encode(hash));
    let known = KnownHosts::parse(&format!("{host} ssh-ed25519 {KEY}")).unwrap();
    known.verify(&endpoint(22), &key(KEY)).unwrap();
}

#[test]
fn malformed_markers_fields_and_hashes_never_create_trust() {
    assert!(KnownHosts::parse("@unknown host ssh-ed25519 key").is_err());
    assert!(KnownHosts::parse("host ssh-ed25519").is_err());
    assert!(KnownHosts::parse("host,,other ssh-ed25519 key").is_err());
    let malformed_hash = KnownHosts::parse(&format!("|1|bad ssh-ed25519 {KEY}")).unwrap();
    assert!(matches!(
        malformed_hash.verify(&endpoint(22), &key(KEY)),
        Err(RemoteError::UnknownHostKey { .. })
    ));
}

#[test]
fn negation_and_revocation_override_positive_trust() {
    let negated =
        KnownHosts::parse(&format!("*.example,!build.example ssh-ed25519 {KEY}")).unwrap();
    assert!(matches!(
        negated.verify(&endpoint(22), &key(KEY)),
        Err(RemoteError::UnknownHostKey { .. })
    ));
    let entries = KnownHosts::parse(&format!(
        "build.example ssh-ed25519 {KEY}\n@revoked build.example ssh-ed25519 {KEY}"
    ))
    .unwrap();
    assert!(matches!(
        entries.verify(&endpoint(22), &key(KEY)),
        Err(RemoteError::RevokedHostKey { .. })
    ));
}

#[test]
fn known_hosts_file_and_question_wildcard_are_supported() {
    let state = tempfile::tempdir().unwrap();
    let path = state
        .path()
        .join(crate::remote::constants::TEST_KNOWN_HOSTS_FILE_NAME);
    std::fs::write(&path, format!("build.ex?mple ssh-ed25519 {KEY}\n")).unwrap();
    KnownHosts::from_path(path)
        .unwrap()
        .verify(&endpoint(22), &key(KEY))
        .unwrap();
}

#[test]
fn blank_comments_and_certificate_authorities_do_not_create_host_trust() {
    let known = KnownHosts::parse(&format!(
        "\n# ignored\n@cert-authority build.example ssh-ed25519 {KEY}\n"
    ))
    .unwrap();
    assert!(matches!(
        known.verify(&endpoint(22), &key(KEY)),
        Err(RemoteError::UnknownHostKey { .. })
    ));
    assert!(KnownHosts::parse(&format!(", ssh-ed25519 {KEY}")).is_err());
}
