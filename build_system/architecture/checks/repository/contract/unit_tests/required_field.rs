use super::*;
use yolo_architecture_test_support::Fixture;

#[path = "../test_support/required_field_constants.rs"]
mod constants;
use constants::*;

fn report(path: &str, source: &str) -> Vec<Violation> {
    let fixture = Fixture::new();
    fixture.write(path, source);
    let files = yolo_architecture_test_support::load(fixture.root()).unwrap();
    let mut result = Vec::new();
    for file in files {
        inspect(&file, &mut result);
    }
    result
}

fn record(selected: &str, attributes: &str) -> String {
    RECORD_MARKERS
        .iter()
        .fold(RECORD.to_owned(), |source, marker| {
            source.replace(marker, if *marker == selected { attributes } else { "" })
        })
}

fn envelopes(selected: &str, container: &str, field: &str) -> String {
    ENVELOPE_TYPES
        .iter()
        .map(|name| {
            let (container, field) = if *name == selected {
                (container, field)
            } else {
                (DENY_ATTRIBUTE, "")
            };
            format!("{container} pub struct {name} {{ {field} pub payload: Value, }}\n")
        })
        .collect()
}

#[test]
fn custody_offer_requires_profile_without_default_or_serialization_omission() {
    for attributes in INFERRED_ATTRIBUTES {
        assert!(
            !report(OFFER_OWNER, &OFFER.replace(FIELD_MARKER, attributes)).is_empty(),
            "{attributes}"
        );
    }
    assert!(report(OFFER_OWNER, &OFFER.replace(FIELD_MARKER, EXPLANATION)).is_empty());
}

#[test]
fn every_persisted_profile_is_required_in_armed_retired_and_durable_lease() {
    for marker in RECORD_MARKERS {
        for attributes in INFERRED_ATTRIBUTES {
            assert!(
                !report(RECORD_OWNER, &record(marker, attributes)).is_empty(),
                "{marker}: {attributes}"
            );
        }
    }
    assert!(report(RECORD_OWNER, &record("", "")).is_empty());
}

#[test]
fn profile_cannot_regain_derived_or_manual_implicit_authority() {
    for attributes in PROFILE_DEFAULTS {
        assert!(!report(PROFILE_OWNER, &PROFILE.replace(PROFILE_MARKER, attributes)).is_empty());
    }
    let current = PROFILE.replace(PROFILE_MARKER, CURRENT_PROFILE_ATTRIBUTES);
    assert!(!report(PROFILE_OWNER, &format!("{current}{PROFILE_IMPLEMENTATION}")).is_empty());
    assert!(
        report(
            PROFILE_OWNER,
            &format!("{IRRELEVANT_DEFAULT}{EXPLANATION}{current}")
        )
        .is_empty()
    );
}

#[test]
fn every_envelope_requires_payload_and_unknown_field_rejection() {
    for name in ENVELOPE_TYPES {
        for attributes in INFERRED_ATTRIBUTES {
            assert!(
                !report(ENVELOPE_OWNER, &envelopes(name, DENY_ATTRIBUTE, attributes)).is_empty()
            );
        }
        assert!(!report(ENVELOPE_OWNER, &envelopes(name, "", "")).is_empty());
        assert!(!report(ENVELOPE_OWNER, &envelopes(name, CONDITIONAL_DENY, "")).is_empty());
        let container_default = format!("{DENY_ATTRIBUTE} {}", INFERRED_ATTRIBUTES[0]);
        assert!(!report(ENVELOPE_OWNER, &envelopes(name, &container_default, "")).is_empty());
    }
    assert!(report(ENVELOPE_OWNER, &envelopes("", "", "")).is_empty());
}

#[test]
fn guard_does_not_ban_unrelated_optional_fields_or_rejection_fixtures() {
    let source = OFFER.replace(FIELD_MARKER, INFERRED_ATTRIBUTES[0]);
    for path in [
        "core/crates/runtime/engine/pty_custody/unit_tests/old_offer.rs",
        "core/crates/other/src/protocol.rs",
    ] {
        assert!(report(path, &source).is_empty());
    }
    assert!(!report(OFFER_OWNER, "pub struct Other {}").is_empty());
}

#[test]
fn optional_and_other_wrappers_cannot_make_required_fields_implicitly_absent() {
    for (path, current, field_type) in [
        (OFFER_OWNER, OFFER.replace(FIELD_MARKER, ""), PROFILE_TYPE),
        (RECORD_OWNER, record("", ""), PROFILE_TYPE),
        (ENVELOPE_OWNER, envelopes("", "", ""), PAYLOAD_TYPE),
    ] {
        for wrapped in WRAPPED_TYPES {
            let replacement = wrapped.replace(TYPE_MARKER, field_type);
            let source = current.replace(&format!(": {field_type}"), &format!(": {replacement}"));
            assert!(!report(path, &source).is_empty(), "{path}: {replacement}");
        }
    }
}

#[test]
fn direct_required_type_paths_remain_valid_when_fully_qualified() {
    for (path, current, field_type, qualified) in [
        (
            OFFER_OWNER,
            OFFER.replace(FIELD_MARKER, ""),
            PROFILE_TYPE,
            QUALIFIED_PROFILE,
        ),
        (
            RECORD_OWNER,
            record("", ""),
            PROFILE_TYPE,
            QUALIFIED_PROFILE,
        ),
        (
            ENVELOPE_OWNER,
            envelopes("", "", ""),
            PAYLOAD_TYPE,
            QUALIFIED_PAYLOAD,
        ),
    ] {
        let source = current.replace(&format!(": {field_type}"), &format!(": {qualified}"));
        assert!(report(path, &source).is_empty(), "{path}");
    }
}
