use super::super::constants::{
    DEFAULT_SSH_PORT, HASHED_HOST_PREFIX, HASHED_HOST_SEPARATOR, KNOWN_HOST_PORT_PREFIX,
    KNOWN_HOST_PORT_SEPARATOR,
};
use super::super::{HostKey, KnownHosts, RemoteEndpoint, RemoteError};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use hmac::{Hmac, Mac};
use sha1::Sha1;

const TEST_USER: &str = "builder";
const HOSTNAME: &str = "build.example";
const IPV4_HOST: &str = "192.0.2.10";
const IPV6_HOST: &str = "2001:db8::10";
const TEST_HOSTS: &[&str] = &[HOSTNAME, IPV4_HOST, IPV6_HOST];
const NONSTANDARD_SSH_PORT: u16 = 2202;
const KEY_ALGORITHM: &str = "ssh-ed25519";
const TRUSTED_KEY: &str = "trusted-key";
const CONFLICTING_KEY: &str = "conflicting-key";
const HASH_SALT: &[u8] = b"known-host-port-salt";

fn endpoint(host: &str, port: u16) -> RemoteEndpoint {
    RemoteEndpoint::new(TEST_USER, host, port).unwrap()
}

fn key(encoded: &str) -> HostKey {
    HostKey::new(KEY_ALGORITHM, encoded).unwrap()
}

fn known(pattern: &str, encoded: &str) -> KnownHosts {
    KnownHosts::parse(&format!("{pattern} {KEY_ALGORITHM} {encoded}")).unwrap()
}

fn port_host(host: &str) -> String {
    format!("{KNOWN_HOST_PORT_PREFIX}{host}{KNOWN_HOST_PORT_SEPARATOR}{NONSTANDARD_SSH_PORT}")
}

fn hashed(host: &str) -> String {
    let mut mac = Hmac::<Sha1>::new_from_slice(HASH_SALT).unwrap();
    mac.update(host.as_bytes());
    let digest = mac.finalize().into_bytes();
    format!(
        "{HASHED_HOST_PREFIX}{}{HASHED_HOST_SEPARATOR}{}",
        STANDARD.encode(HASH_SALT),
        STANDARD.encode(digest)
    )
}

fn assert_unknown(result: Result<(), RemoteError>) {
    assert!(matches!(result, Err(RemoteError::UnknownHostKey { .. })));
}

fn assert_changed(result: Result<(), RemoteError>) {
    assert!(matches!(result, Err(RemoteError::ChangedHostKey { .. })));
}

#[test]
fn plain_host_tokens_follow_openssh_port_semantics_for_every_host_shape() {
    for host in TEST_HOSTS.iter().copied() {
        let trusted = key(TRUSTED_KEY);
        known(host, TRUSTED_KEY)
            .verify(&endpoint(host, DEFAULT_SSH_PORT), &trusted)
            .unwrap();
        known(&port_host(host), TRUSTED_KEY)
            .verify(&endpoint(host, NONSTANDARD_SSH_PORT), &trusted)
            .unwrap();
        assert_unknown(
            known(host, TRUSTED_KEY).verify(&endpoint(host, NONSTANDARD_SSH_PORT), &trusted),
        );
        assert_unknown(
            known(&port_host(host), TRUSTED_KEY)
                .verify(&endpoint(host, DEFAULT_SSH_PORT), &trusted),
        );
    }
}

#[test]
fn plain_bare_key_cannot_override_nonstandard_port_conflict() {
    let explicit = port_host(HOSTNAME);
    let known = KnownHosts::parse(&format!(
        "{explicit} {KEY_ALGORITHM} {CONFLICTING_KEY}\n\
         {HOSTNAME} {KEY_ALGORITHM} {TRUSTED_KEY}"
    ))
    .unwrap();
    assert_changed(known.verify(&endpoint(HOSTNAME, NONSTANDARD_SSH_PORT), &key(TRUSTED_KEY)));
}

#[test]
fn hashed_host_tokens_are_bound_to_the_canonical_port_form() {
    for host in TEST_HOSTS.iter().copied() {
        let trusted = key(TRUSTED_KEY);
        let bare = hashed(host);
        let explicit = hashed(&port_host(host));
        known(&bare, TRUSTED_KEY)
            .verify(&endpoint(host, DEFAULT_SSH_PORT), &trusted)
            .unwrap();
        known(&explicit, TRUSTED_KEY)
            .verify(&endpoint(host, NONSTANDARD_SSH_PORT), &trusted)
            .unwrap();
        assert_unknown(
            known(&bare, TRUSTED_KEY).verify(&endpoint(host, NONSTANDARD_SSH_PORT), &trusted),
        );
        assert_unknown(
            known(&explicit, TRUSTED_KEY).verify(&endpoint(host, DEFAULT_SSH_PORT), &trusted),
        );
    }
}

#[test]
fn hashed_bare_key_cannot_override_nonstandard_port_conflict() {
    let explicit = hashed(&port_host(IPV6_HOST));
    let bare = hashed(IPV6_HOST);
    let known = KnownHosts::parse(&format!(
        "{explicit} {KEY_ALGORITHM} {CONFLICTING_KEY}\n\
         {bare} {KEY_ALGORITHM} {TRUSTED_KEY}"
    ))
    .unwrap();
    assert_changed(known.verify(
        &endpoint(IPV6_HOST, NONSTANDARD_SSH_PORT),
        &key(TRUSTED_KEY),
    ));
}
