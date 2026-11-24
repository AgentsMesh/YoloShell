use super::{IDENTITY, SIGNATURE};
use yolo_coverage_protocol::*;

const INVALID_DIGEST: &str = "invalid";
const UNKNOWN_FIELD: &str = "unknown";
const INCREMENT: u64 = 1;
const ZERO: u64 = 0;

fn rejected(token: String) {
    assert!(parse_token(&token).is_err(), "{token}");
}

#[test]
fn every_inventory_kind_rejects_invalid_digests_and_member_counts() {
    rejected(source_inventory_token(INVALID_DIGEST, INCREMENT));
    rejected(source_inventory_token(IDENTITY, ZERO));
    rejected(source_inventory_token(
        IDENTITY,
        MAXIMUM_INSTANCE_VECTOR_LENGTH + INCREMENT,
    ));
    rejected(instance_inventory_token(INVALID_DIGEST, ZERO));
    rejected(instance_inventory_token(
        IDENTITY,
        MAXIMUM_INSTANCE_VECTOR_LENGTH + INCREMENT,
    ));
    rejected(instance_member_token(INVALID_DIGEST, SIGNATURE));
    rejected(instance_member_token(IDENTITY, INVALID_DIGEST));
    for prefix in [
        SOURCE_INVENTORY_PREFIX,
        INSTANCE_INVENTORY_PREFIX,
        INSTANCE_MEMBER_PREFIX,
    ] {
        rejected(prefix.to_owned());
        rejected(format!("{prefix}{IDENTITY}"));
        rejected(format!("{prefix}{IDENTITY}/{UNKNOWN_FIELD}"));
        rejected(format!("{prefix}{IDENTITY}/{SIGNATURE}/{UNKNOWN_FIELD}"));
    }
}

#[test]
fn all_metrics_accept_exact_position_and_vector_limits() {
    for kind in MetricKind::ALL {
        for position in [MINIMUM_SOURCE_POSITION, MAXIMUM_SOURCE_POSITION] {
            for count in [ZERO, MAXIMUM_INSTANCE_VECTOR_LENGTH] {
                let header =
                    header_token(kind, position, position, IDENTITY, SIGNATURE, count, count);
                assert!(matches!(
                    parse_token(&header).unwrap(),
                    Some(FunctionToken::Header { .. })
                ));
                let ordinal = ordinal_token(kind, position, position, IDENTITY, SIGNATURE, count);
                assert!(matches!(
                    parse_token(&ordinal).unwrap(),
                    Some(FunctionToken::Ordinal { .. })
                ));
            }
        }
    }
}

#[test]
fn headers_and_ordinals_reject_each_invalid_position_digest_and_bound() {
    let kind = MetricKind::Regions;
    for bad in [ZERO, MAXIMUM_SOURCE_POSITION + INCREMENT] {
        rejected(header_token(
            kind, bad, INCREMENT, IDENTITY, SIGNATURE, INCREMENT, INCREMENT,
        ));
        rejected(header_token(
            kind, INCREMENT, bad, IDENTITY, SIGNATURE, INCREMENT, INCREMENT,
        ));
        rejected(ordinal_token(
            kind, bad, INCREMENT, IDENTITY, SIGNATURE, ZERO,
        ));
        rejected(ordinal_token(
            kind, INCREMENT, bad, IDENTITY, SIGNATURE, ZERO,
        ));
    }
    for (identity, signature) in [(INVALID_DIGEST, SIGNATURE), (IDENTITY, INVALID_DIGEST)] {
        rejected(header_token(
            kind, INCREMENT, INCREMENT, identity, signature, INCREMENT, INCREMENT,
        ));
        rejected(ordinal_token(
            kind, INCREMENT, INCREMENT, identity, signature, ZERO,
        ));
    }
    let oversized = MAXIMUM_INSTANCE_VECTOR_LENGTH + INCREMENT;
    rejected(header_token(
        kind, INCREMENT, INCREMENT, IDENTITY, SIGNATURE, oversized, ZERO,
    ));
    rejected(header_token(
        kind, INCREMENT, INCREMENT, IDENTITY, SIGNATURE, INCREMENT, oversized,
    ));
    rejected(ordinal_token(
        kind, INCREMENT, INCREMENT, IDENTITY, SIGNATURE, oversized,
    ));
}

#[test]
fn nonnumeric_fields_and_incomplete_vectors_cannot_enter_metric_inventory() {
    for prefix in [HEADER_PREFIX, ORDINAL_PREFIX] {
        rejected(prefix.to_owned());
        let fields = if prefix == HEADER_PREFIX {
            vec![
                MetricKind::Regions.path(),
                "1",
                "1",
                IDENTITY,
                SIGNATURE,
                "1",
                "1",
            ]
        } else {
            vec![
                MetricKind::Regions.path(),
                "1",
                "1",
                IDENTITY,
                SIGNATURE,
                "1",
            ]
        };
        assert!(
            parse_token(&format!("{prefix}{}", fields.join("/")))
                .unwrap()
                .is_some()
        );
        for index in 0..fields.len() {
            let mut corrupt = fields.clone();
            corrupt[index] = UNKNOWN_FIELD;
            rejected(format!("{prefix}{}", corrupt.join("/")));
        }
    }
    assert_eq!(parse_token(UNKNOWN_FIELD).unwrap(), None);
}
