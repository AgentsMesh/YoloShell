mod source_scope_tests;
mod token_boundary_tests;

use yolo_coverage_protocol::{
    FunctionToken, MARKER_TOKEN, MetricKind, encode_fields, header_token,
    instance_inventory_digest, instance_inventory_token, instance_member_token, ordinal_token,
    parse_token, production_key, source_inventory_digest, source_inventory_token,
};

const SOURCE: &str = "core/src/agent/detector.rs";
const COMPONENT_SOURCE: &str = "core/crates/wire/src/lib.rs";
const IDENTITY: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const SIGNATURE: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

#[test]
fn marker_header_and_ordinal_tokens_round_trip() {
    assert_eq!(
        parse_token(MARKER_TOKEN).unwrap(),
        Some(FunctionToken::Marker)
    );
    let header = header_token(MetricKind::Branches, 12, 3, IDENTITY, SIGNATURE, 4, 2);
    assert_eq!(
        parse_token(&header).unwrap(),
        Some(FunctionToken::Header {
            kind: MetricKind::Branches,
            line: 12,
            column: 3,
            identity: IDENTITY,
            signature: SIGNATURE,
            vector_len: 4,
            found: 2,
        })
    );
    let ordinal = ordinal_token(MetricKind::Lines, 12, 3, IDENTITY, SIGNATURE, 7);
    assert_eq!(
        parse_token(&ordinal).unwrap(),
        Some(FunctionToken::Ordinal {
            kind: MetricKind::Lines,
            line: 12,
            column: 3,
            identity: IDENTITY,
            signature: SIGNATURE,
            ordinal: 7,
        })
    );
}

#[test]
fn inventory_tokens_round_trip() {
    let source_digest = source_inventory_digest(&[SOURCE]).unwrap();
    let source_token = source_inventory_token(&source_digest, 1);
    assert_eq!(
        parse_token(&source_token).unwrap(),
        Some(FunctionToken::SourceInventory {
            inventory: &source_digest,
            members: 1,
        })
    );
    let instance_digest = instance_inventory_digest(&[IDENTITY]).unwrap();
    let inventory_token = instance_inventory_token(&instance_digest, 1);
    assert_eq!(
        parse_token(&inventory_token).unwrap(),
        Some(FunctionToken::InstanceInventory {
            inventory: &instance_digest,
            members: 1,
        })
    );
    let member_token = instance_member_token(&instance_digest, IDENTITY);
    assert_eq!(
        parse_token(&member_token).unwrap(),
        Some(FunctionToken::InstanceMember {
            inventory: &instance_digest,
            instance: IDENTITY,
        })
    );
}

#[test]
fn malformed_synthetic_tokens_fail_closed() {
    for token in [
        "__yolo_metric__/unknown",
        "__yolo_metric__/header/regions/1/1/aa/bb/1/1",
        "__yolo_metric__/header/unknown/1/1/aa/bb/1/1",
        "__yolo_metric__/header/lines/0/1/aa/bb/1/1",
        "__yolo_metric__/header/lines/1/1/aa/bb/1/2",
        "__yolo_metric__/ordinal/lines/1/1/aa/bb/0",
    ] {
        assert!(parse_token(token).is_err(), "{token:?}");
    }
}

#[test]
fn identities_are_fixed_length_and_include_field_boundaries() {
    let left = encode_fields(&["ab", "c"]).unwrap();
    let right = encode_fields(&["a", "bc"]).unwrap();
    assert_ne!(left, right);
    assert_eq!(left.len(), 64);
}

#[test]
fn production_source_policy_is_shared_with_the_gate() {
    assert_eq!(production_key(SOURCE).unwrap(), Some(SOURCE.to_owned()));
    assert_eq!(
        production_key(COMPONENT_SOURCE).unwrap(),
        Some(COMPONENT_SOURCE.to_owned())
    );
    for source in [
        "core/src/agent/tests.rs",
        "core/src/agent/edge_tests.rs",
        "core/src/socket_root_guard_process_tests/fixture.rs",
        "core/crates/wire/src/tests_sync.rs",
        "integration_tests/runtime/protocol.rs",
    ] {
        assert_eq!(production_key(source).unwrap(), None, "{source}");
    }
    assert!(production_key("core/src/../secret.rs").is_err());
    assert!(production_key("core/crates/../secret.rs").is_err());
}
