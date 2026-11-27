use crate::reachability_cfg::{Truth, evaluate};
use crate::reachability_config::Configuration;
use std::collections::BTreeSet;
use yolo_architecture_syntax::targeted_tokens::tokenize;

const ROOT: &str = "src/lib.rs";
const ENABLED_FEATURE: &str = "enabled";

#[test]
fn cfg_boolean_composition_distinguishes_false_from_unknown_platforms() {
    let configuration = Configuration {
        root: ROOT.into(),
        test: true,
        features: Some(BTreeSet::from([ENABLED_FEATURE.into()])),
    };
    for (predicate, expected) in [
        ("test", Truth::Yes),
        ("not(test)", Truth::No),
        ("feature = \"enabled\"", Truth::Yes),
        ("feature = \"missing\"", Truth::No),
        ("any()", Truth::No),
        ("all()", Truth::Yes),
        ("all(test, feature = \"missing\")", Truth::No),
        ("any(test, unknown)", Truth::Yes),
        ("all(test, target_os = \"linux\")", Truth::Unknown),
        ("all(not(test), target_os = \"linux\")", Truth::No),
        ("any(not(test), unknown)", Truth::Unknown),
        ("not(unknown)", Truth::Unknown),
        ("not(not(test))", Truth::Yes),
        ("all(test,)", Truth::Yes),
        ("custom(feature = \"missing\")", Truth::Unknown),
    ] {
        assert_eq!(
            evaluate(&tokenize(predicate), &configuration),
            expected,
            "{predicate}"
        );
    }
}

#[test]
fn invalid_predicates_and_unresolved_features_never_disable_an_edge() {
    let configuration = Configuration {
        root: ROOT.into(),
        test: false,
        features: None,
    };
    for predicate in [
        "",
        "test test",
        "feature",
        "feature =",
        "all(test",
        "all(test test)",
        "not test",
        "not()",
        "feature = unknown",
        "feature = \"missing\"",
        "custom(",
        "unknown =",
    ] {
        assert_eq!(
            evaluate(&tokenize(predicate), &configuration),
            Truth::Unknown,
            "{predicate}"
        );
    }
    assert_eq!(evaluate(&tokenize("test"), &configuration), Truth::No);
}

#[test]
fn source_configuration_preserves_unknown_attributes_and_rejects_inner_false_cfg() {
    let configuration = Configuration {
        root: ROOT.into(),
        test: true,
        features: Some(BTreeSet::new()),
    };
    let source = "#[cfg(test)] #[path = \"case.rs\"] mod case;";
    let configured =
        crate::reachability_tokens::configure(&tokenize(source), &configuration).unwrap();
    assert_eq!(
        configured.tokens,
        tokenize("#[cfg(all())] #[path = \"case.rs\"] mod case;")
    );
    assert!(
        crate::reachability_tokens::configure(
            &tokenize("#![cfg(any())] mod case;"),
            &configuration
        )
        .is_none()
    );
    for source in [
        "#[cfg(unknown)] mod case;",
        "#[path = \"case.rs\"] mod case;",
        "#[cfg] mod case;",
        "#[cfg(test] mod case;",
        "#[cfg(test)",
    ] {
        let tokens = tokenize(source);
        assert_eq!(
            crate::reachability_tokens::configure(&tokens, &configuration)
                .unwrap()
                .tokens,
            tokens
        );
    }
}

#[test]
fn disabled_inner_scope_does_not_disable_siblings_or_enable_nested_children() {
    let configuration = Configuration {
        root: ROOT.into(),
        test: true,
        features: Some(BTreeSet::new()),
    };
    for (source, expected) in [
        (
            "mod hidden { #![cfg(not(test))] mod first; mod second; } mod visible;",
            "mod hidden {} mod visible;",
        ),
        (
            "mod outer { mod hidden { #![cfg(any())] mod first; } mod visible; }",
            "mod outer { mod hidden {} mod visible; }",
        ),
        (
            "mod outer { #![cfg(any())] mod hidden { mod first; } } mod visible;",
            "mod outer {} mod visible;",
        ),
        (
            "mod incomplete { mod visible;",
            "mod incomplete { mod visible;",
        ),
    ] {
        assert_eq!(
            crate::reachability_tokens::configure(&tokenize(source), &configuration)
                .unwrap()
                .tokens,
            tokenize(expected),
        );
    }
}
