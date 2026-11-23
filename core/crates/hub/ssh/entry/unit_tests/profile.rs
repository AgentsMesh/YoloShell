use super::*;
use crate::{ProfileError, SshEntry, constants::*, validation};

#[test]
fn strict_wire_round_trip_rejects_unknown_fields_duplicate_fields_and_credentials() {
    let original = profile();
    let encoded = serde_json::to_vec(&original).unwrap();
    let decoded = EntryProfile::decode(&encoded).unwrap();
    assert_eq!(decoded.client_id, original.client_id);
    assert_eq!(decoded.hub, original.hub);
    for field in super::constants::CREDENTIAL_FIELDS {
        let mut wire = serde_json::to_value(&original).unwrap();
        wire.as_object_mut()
            .unwrap()
            .insert(field.into(), serde_json::Value::Bool(true));
        assert!(EntryProfile::decode(&serde_json::to_vec(&wire).unwrap()).is_err());
    }
    let mut nested = serde_json::to_value(&original).unwrap();
    nested["hub"]["unknown"] = true.into();
    assert!(EntryProfile::decode(&serde_json::to_vec(&nested).unwrap()).is_err());
    let duplicate = format!(
        "{{\"host\":\"example.test\",{}",
        &String::from_utf8(encoded).unwrap()[1..]
    );
    assert!(EntryProfile::decode(duplicate.as_bytes()).is_err());
    assert_eq!(
        EntryProfile::decode(&vec![b' '; MAX_PROFILE_BYTES + 1]).unwrap_err(),
        ProfileError::Size
    );
}

#[test]
fn profile_fields_reject_empty_controls_invalid_scope_and_unbounded_values() {
    let original = profile();
    for value in [
        "",
        "a\n.test",
        "-bad",
        "bad-",
        "bad..name",
        "ssh://host",
        "host/path",
    ] {
        let mut invalid = original.clone();
        invalid.host = value.into();
        assert_eq!(invalid.validate(), Err(ProfileError::Host));
    }
    let mut invalid = original.clone();
    invalid.host = "a".repeat(MAX_HOST_BYTES + 1);
    assert_eq!(invalid.validate(), Err(ProfileError::Host));
    for value in ["", "user name", "user\n", "user@host"] {
        let mut invalid = original.clone();
        invalid.username = value.into();
        assert_eq!(invalid.validate(), Err(ProfileError::Username));
    }
    for value in ["", "relative", "/", "/a/../b", "/a//b", "/a/./b", "/a\n"] {
        let mut invalid = original.clone();
        invalid.remote_socket = value.into();
        assert_eq!(invalid.validate(), Err(ProfileError::Socket));
    }
    for value in ["", "bad-key", "ssh-ed25519 bad\n"] {
        let mut invalid = original.clone();
        invalid.outer_host_key = value.into();
        assert_eq!(invalid.validate(), Err(ProfileError::HostKey));
    }
    let mut invalid = original.clone();
    invalid.port = u16::default();
    assert_eq!(invalid.validate(), Err(ProfileError::Port));
    invalid = original.clone();
    invalid.schema_version += 1;
    assert_eq!(invalid.validate(), Err(ProfileError::Schema));
    invalid = original.clone();
    invalid.hub.public_key = EMPTY_KEY;
    assert_eq!(invalid.validate(), Err(ProfileError::HubPin));
    invalid = original;
    invalid.client_id = uuid::Uuid::nil();
    assert_eq!(invalid.validate(), Err(ProfileError::ClientId));
}

#[test]
fn debug_omits_host_key_private_location_and_credential_material() {
    let profile = profile();
    let (_, outer) = keys::keys();
    let (_, inner) = keys::keys();
    let entry = SshEntry::new(profile.clone(), outer, inner).unwrap();
    let debug = format!("{entry:?}");
    for private in [&profile.outer_host_key, HOST, USERNAME, SOCKET] {
        assert!(!debug.contains(private));
    }
}

#[test]
fn validated_host_supports_dns_ipv4_ipv6_and_exact_size_bounds() {
    for host in ["localhost", "a.example.", "127.0.0.1", "::1", "2001:db8::1"] {
        assert!(validation::host(host));
    }
    assert!(validation::username(&"u".repeat(MAX_USERNAME_BYTES)));
    assert!(!validation::username(&"u".repeat(MAX_USERNAME_BYTES + 1)));
    assert!(validation::socket(std::path::Path::new(&format!(
        "/{}",
        "a".repeat(MAX_SOCKET_BYTES - 1)
    ))));
    assert!(!validation::socket(std::path::Path::new(&format!(
        "/{}",
        "a".repeat(MAX_SOCKET_BYTES)
    ))));
}
