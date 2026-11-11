use crate::rust_source::RustSource;
use crate::rust_source_cfg::is_unit_test_guarded;

const CONJOINED_TEST_PREDICATE: &str =
    "#[cfg(all(test, feature = \"yolo-unit-tests\"))]\nmod tests;\n";
const ALTERNATIVE_TEST_PREDICATE: &str =
    "#[cfg(any(test, feature = \"yolo-unit-tests\"))]\nmod tests;\n";

#[test]
fn feature_conjoined_test_predicate_is_unit_test_guarded() {
    let source = RustSource::parse(CONJOINED_TEST_PREDICATE);
    assert!(is_unit_test_guarded(&source.tokens), "{:#?}", source.tokens);
}

#[test]
fn feature_alternative_does_not_guard_a_unit_test_module() {
    let source = RustSource::parse(ALTERNATIVE_TEST_PREDICATE);
    assert!(!is_unit_test_guarded(&source.tokens));
}

#[test]
fn malformed_predicates_cannot_claim_a_test_only_boundary() {
    for predicate in [
        "",
        "test",
        "(test) extra",
        "(test",
        "()",
        "(123)",
        "(feature)",
        "(feature =)",
        "(feature = unknown)",
        "(all)",
        "(all())",
        "(all(test unix))",
        "(all(test,)) extra",
        "(any)",
        "(any())",
        "(any(test,)) extra",
        "(any(test, unknown =))",
        "(not)",
        "(not())",
        "(not(test)",
        "(not(test, unix))",
        "(unix =)",
    ] {
        let text = format!("#[cfg{predicate}] mod guarded;");
        let source = RustSource::parse(&text);
        assert!(!is_unit_test_guarded(&source.tokens), "{text}");
        assert!(
            !crate::rust_source_cfg::is_test_guarded(&source.tokens),
            "{text}"
        );
    }
}

#[test]
fn nested_predicates_conservatively_track_production_reachability() {
    for (predicate, expected) in [
        ("test", true),
        ("not(test)", false),
        ("all(test, unix,)", true),
        ("all(unix, test)", true),
        ("all(unix, target_os = \"macos\")", false),
        ("any(test,)", true),
        ("any(test, unix)", false),
        ("any(all(test, unix), all(test, windows))", true),
        ("all(not(unix), test)", true),
    ] {
        let source = RustSource::parse(&format!("#[cfg({predicate})] mod guarded;"));
        assert_eq!(
            is_unit_test_guarded(&source.tokens),
            expected,
            "{predicate}"
        );
    }
}

#[test]
fn adversarial_feature_is_distinct_from_a_unit_test_only_guard() {
    let source = RustSource::parse("#[cfg(feature = \"adversarial-test-hooks\")] mod guarded;");
    assert!(crate::rust_source_cfg::is_test_guarded(&source.tokens));
    assert!(!is_unit_test_guarded(&source.tokens));
    let source = RustSource::parse("#[allow(dead_code)] #[cfg(unix)] #[cfg(test)] mod guarded;");
    assert!(is_unit_test_guarded(&source.tokens));
    for text in ["#[cfg(test)", "# ( cfg(test))", "# [ other(test)]"] {
        assert!(
            !is_unit_test_guarded(&RustSource::parse(text).tokens),
            "{text}"
        );
    }
}
