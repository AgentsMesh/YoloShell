use super::acceptance_receipt_support::{bytes, digest, expectation, parse};
use crate::AcceptanceExpectation;
use crate::constants::*;

const ALTERNATE_REVISION: &str = "alternate-revision";
const ALTERNATE_CHECKLIST: &str = "yolo-alternate-checklist";
const ALTERNATE_OPERATOR: &str = "alternate@example.test";
const NEWER_MACOS_VERSION: &str = "15.0";

#[test]
fn receipt_rejects_independent_changes_to_each_valid_expectation_binding() {
    let receipt = parse(&bytes()).unwrap();
    let mutations: &[(fn(&mut AcceptanceExpectation), &str)] = &[
        (
            |value| value.candidate.source_revision = ALTERNATE_REVISION.to_owned(),
            RECEIPT_BINDING_ERROR,
        ),
        (
            |value| value.candidate.artifact_sha256 = digest('c'),
            RECEIPT_BINDING_ERROR,
        ),
        (
            |value| value.candidate.provenance_sha256 = digest('c'),
            RECEIPT_BINDING_ERROR,
        ),
        (
            |value| value.checklist_schema = ALTERNATE_CHECKLIST.to_owned(),
            RECEIPT_CHECKLIST_ERROR,
        ),
        (
            |value| value.desktop_os.version = NEWER_MACOS_VERSION.to_owned(),
            RECEIPT_PROVIDER_ERROR,
        ),
        (
            |value| value.remote_os.version = NEWER_MACOS_VERSION.to_owned(),
            RECEIPT_PROVIDER_ERROR,
        ),
        (
            |value| value.operator_identity = ALTERNATE_OPERATOR.to_owned(),
            RECEIPT_OPERATOR_ERROR,
        ),
    ];
    for (mutate, expected_error) in mutations {
        let mut expected = expectation();
        mutate(&mut expected);
        assert_eq!(
            receipt.verify(&expected).unwrap_err().to_string(),
            *expected_error
        );
    }
}

#[test]
fn receipt_revalidates_expectations_after_their_public_fields_are_mutated() {
    let receipt = parse(&bytes()).unwrap();
    let mutations: &[fn(&mut AcceptanceExpectation)] = &[
        |value| value.candidate.source_revision.clear(),
        |value| value.candidate.artifact_sha256.clear(),
        |value| value.candidate.provenance_sha256.clear(),
        |value| value.checklist_schema.clear(),
        |value| value.providers.clear(),
        |value| value.desktop_os.name.clear(),
        |value| value.remote_os.architecture.clear(),
        |value| value.operator_identity.clear(),
    ];
    for mutate in mutations {
        let mut expected = expectation();
        mutate(&mut expected);
        assert!(receipt.verify(&expected).is_err());
    }
}

#[test]
fn primitive_and_nested_nonreceipt_json_is_never_accepted_as_an_envelope() {
    let malformed: &[&[u8]] = &[
        b"true",
        b"-1",
        b"1",
        b"1.5",
        b"null",
        b"\"receipt\"",
        b"[true,-1,1.5,null]",
        b"{\"nested\":[{\"x\":1,\"x\":2}]}",
        b"{\"nested\":{\"x\":1},\"nested\":{\"x\":2}}",
        b"{} {}",
    ];
    for bytes in malformed {
        assert!(parse(bytes).is_err(), "unexpected envelope: {bytes:?}");
    }
}
