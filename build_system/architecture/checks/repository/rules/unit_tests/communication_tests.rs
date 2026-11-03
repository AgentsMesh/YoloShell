use super::{source, violations};

#[test]
fn retired_codecs_cannot_return_through_protocol_implementations_or_test_fixtures() {
    for path in [
        "core/crates/hub/client/src/lib.rs",
        "core/crates/hub/server/test_support/peer.rs",
        "core/crates/transport/client/BUILD.bazel",
    ] {
        for text in [
            "use rmpv as hidden;",
            "rmp_serde::to_vec_named(value)",
            "@crates//:rmp-serde",
        ] {
            let mut file = source(path);
            file.text = text.into();
            assert!(!violations(&[file], &[]).is_empty());
        }
    }
}

#[test]
fn persisted_snapshot_codec_is_an_explicit_storage_boundary_and_not_a_transport_exception() {
    let mut snapshot = source("core/crates/persistence/journal/src/snapshot_format.rs");
    snapshot.text = "rmp_serde::to_vec_named(snapshot)".into();
    assert!(violations(std::slice::from_ref(&snapshot), &[]).is_empty());
    snapshot.text = "use rmpv::Value;".into();
    assert!(!violations(&[snapshot], &[]).is_empty());
    let mut documentation = source("docs/protocol.md");
    documentation.text = "rmp-serde is a journal storage codec".into();
    assert!(violations(&[documentation], &[]).is_empty());
}

#[test]
fn private_handshakes_catalog_credentials_and_startup_cannot_regain_json_wire_codecs() {
    for path in [
        "core/crates/application/host/cli/src/catalog_stream/framing.rs",
        "core/crates/application/broker/supervision/src/readiness.rs",
        "core/crates/transport/remote/broker/src/identity/identity_passphrases.rs",
        "core/crates/transport/capnp/argument/src/lib.rs",
        "core/crates/hub/local/startup/src/readiness.rs",
        "core/crates/hub/cli/src/bootstrap.rs",
        "plugins/tui/client/control/src/framing.rs",
        "core/crates/agent/providers/codex/hooks/src/framing.rs",
        "core/crates/runtime/engine/worker/src/bootstrap_spec_codec.rs",
    ] {
        for text in [
            "serde_json::to_vec(value)",
            "serde_json :: from_slice (bytes)",
            "use serde_json::{Value, to_writer as encode};",
            "use serde_json as wire;",
            "use ::serde_json as wire;",
            "use serde_json::{self as wire, Value};",
            "use serde_json::*;",
            "serde_json::Deserializer::from_reader(stream)",
        ] {
            let mut file = source(path);
            file.text = text.into();
            assert!(!violations(&[file], &[]).is_empty(), "{path}: {text}");
        }
    }
}

#[test]
fn public_storage_provider_and_value_conversion_formats_remain_owned() {
    for path in [
        "core/crates/hub/jsonrpc/transport/src/codec.rs",
        "core/crates/transport/remote/broker/src/config/config.rs",
        "plugins/tui/client/control/src/endpoint.rs",
        "core/crates/agent/providers/codex/hooks/src/native/native_wire.rs",
        "core/crates/conversation/cli/src/document.rs",
    ] {
        let mut file = source(path);
        file.text = "serde_json::from_slice(bytes)".into();
        assert!(violations(&[file], &[]).is_empty(), "{path}");
    }
    let mut domain = source("core/crates/transport/client/src/error.rs");
    domain.text = "use serde_json::{Value, json}; serde_json::to_value(domain)".into();
    assert!(violations(&[domain], &[]).is_empty());
}

#[test]
fn mixed_public_and_private_files_have_exact_single_use_exceptions() {
    let mut serve = source("core/crates/hub/cli/src/serve.rs");
    serve.text = "let public_identity = serde_json::to_string(&state.peer_identity())?;".into();
    assert!(violations(std::slice::from_ref(&serve), &[]).is_empty());
    serve.text.push_str("serde_json::to_vec(&ready)");
    assert!(!violations(&[serve], &[]).is_empty());
    let mut publication = source("core/crates/distribution/publication/protocol/src/wire.rs");
    publication.text = "pub fn decode_selection(bytes: &[u8]) -> Result<Selection> { ensure!(bytes.len() <= MAX_PUBLICATION_BYTES, PUBLICATION_LIMIT); serde_json::from_slice(bytes).map_err(Into::into) }".into();
    assert!(violations(std::slice::from_ref(&publication), &[]).is_empty());
    publication.text.push_str("serde_json::from_slice(request)");
    assert!(!violations(&[publication], &[]).is_empty());
}
