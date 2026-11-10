use std::path::Path;
use yolo_release_developer_release_system::developer_release_command::{
    parse_notary_response, validate_profile,
};
use yolo_release_developer_release_system::developer_release_constants::*;
use yolo_release_developer_release_system::{DeveloperReleaseRequest, distribute};

#[test]
fn profile_validation_accepts_the_exact_ascii_alphabet_and_byte_limit() {
    for profile in [
        "a".to_owned(),
        "A09.b-c_d".to_owned(),
        "a".repeat(MAX_NOTARY_PROFILE_BYTES),
    ] {
        validate_profile(&profile).unwrap();
    }
    for profile in [
        "", " ", "-name", "_name", ".name", "name ", "name\n", "a b", "a/b", "a;b", "a\u{e9}",
    ] {
        let error = validate_profile(profile).unwrap_err();
        assert_eq!(error.to_string(), NOTARY_PROFILE_ERROR);
    }
    assert!(validate_profile(&"a".repeat(MAX_NOTARY_PROFILE_BYTES + 1)).is_err());
}

#[test]
fn public_distribution_rejects_bad_profiles_before_resolving_any_path() {
    for profile in ["", "-invalid", "name with spaces"] {
        let request = DeveloperReleaseRequest {
            candidate_directory: Path::new(""),
            output_directory: Path::new(""),
            notary_profile: profile,
            acceptance_receipt: None,
        };
        assert_eq!(
            distribute(request).unwrap_err().to_string(),
            NOTARY_PROFILE_ERROR
        );
    }
}

#[test]
fn notary_response_requires_accepted_status_and_a_nonempty_submission_identifier() {
    let accepted = format!(r#"{{"id":"submission","status":"{NOTARY_ACCEPTED_STATUS}"}}"#);
    assert_eq!(
        parse_notary_response(accepted.as_bytes()).unwrap(),
        "submission"
    );
    for response in [
        r#"{"id":"submission","status":"In Progress"}"#,
        r#"{"id":"submission","status":"Rejected"}"#,
        r#"{"id":"submission","status":"accepted"}"#,
        r#"{"id":"","status":"Accepted"}"#,
        r#"{"id":"  ","status":"Accepted"}"#,
        r#"{"id":"submission"}"#,
        r#"{"status":"Accepted"}"#,
        r#"{"id":null,"status":"Accepted"}"#,
        r#"{"id":"first","id":"second","status":"Accepted"}"#,
        r#"{"id":"submission","status":"Rejected","status":"Accepted"}"#,
        "not json",
    ] {
        let error = parse_notary_response(response.as_bytes()).unwrap_err();
        assert!(
            error.to_string().contains(NOTARY_RESPONSE_ERROR),
            "{response}: {error}"
        );
    }
}
