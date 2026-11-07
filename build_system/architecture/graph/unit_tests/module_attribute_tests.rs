use super::*;
use yolo_architecture_syntax::targeted_tokens::tokenize;

#[test]
fn direct_path_attributes_require_one_string_value_and_one_declaration() {
    for (prefix, expected, uncertain) in [
        ("#[path = \"child.rs\"]", Some("child.rs"), false),
        ("#[path = VALUE]", None, true),
        ("#[path(\"child.rs\")]", None, true),
        (
            "#[path = \"child.rs\"] #[path = \"child.rs\"]",
            Some("child.rs"),
            true,
        ),
        (
            "#[path = \"child.rs\"] #[path = \"other.rs\"]",
            Some("child.rs"),
            true,
        ),
        ("#[path = \"child.rs\"", None, true),
        ("#[doc = \"module docs\"]", None, false),
    ] {
        let tokens = tokenize(&format!("{prefix} mod child;"));
        let item = tokens
            .iter()
            .position(|token| token.identifier("mod"))
            .unwrap();
        let result = before(&tokens, item);
        assert_eq!(result.path.as_deref(), expected, "{prefix}");
        assert_eq!(result.uncertain, uncertain, "{prefix}");
    }
}

#[test]
fn only_direct_empty_any_disables_a_module_and_cfg_attr_remains_uncertain() {
    for (prefix, disabled, test_only, uncertain) in [
        ("#[cfg(any())]", true, false, false),
        ("#[cfg(all())]", false, false, false),
        ("#[cfg(any(unix))]", false, false, false),
        ("#[cfg(test)]", false, true, false),
        ("#[cfg(all(test, unix))]", false, true, false),
        ("#[cfg_attr(unix, path = \"child.rs\")]", false, false, true),
        ("#[cfg_attr(unix, cfg(any()))]", false, false, true),
        ("#[cfg_attr(unix, inline)]", false, false, false),
    ] {
        let tokens = tokenize(&format!("{prefix} mod child;"));
        let item = tokens
            .iter()
            .position(|token| token.identifier("mod"))
            .unwrap();
        let result = before(&tokens, item);
        assert_eq!(
            (result.disabled, result.test_only, result.uncertain),
            (disabled, test_only, uncertain),
            "{prefix}"
        );
    }
}

#[test]
fn previous_items_cannot_lend_attributes_to_a_later_module() {
    for previous in [
        "#[cfg(test)] fn previous() {}",
        "#[cfg(test)] struct Previous;",
        "mod previous {}",
    ] {
        let tokens = tokenize(&format!("{previous} mod child;"));
        let item = tokens
            .iter()
            .rposition(|token| token.identifier("mod"))
            .unwrap();
        let result = before(&tokens, item);
        assert!(!result.test_only && !result.disabled && !result.uncertain);
    }
}

#[test]
fn disabled_attribute_requires_every_delimiter_and_predicate_token() {
    use yolo_architecture_syntax::targeted_tokens::Kind;
    let tokens = tokenize("#[cfg(any())]");
    let open = 1;
    let close = tokens.len() - 1;
    assert!(direct_empty_any(&tokens, open, close));
    for index in open + 1..close {
        for kind in [
            Kind::Identifier("other".into()),
            Kind::Symbol("other".into()),
        ] {
            let mut changed = tokens.clone();
            changed[index].kind = kind;
            assert!(!direct_empty_any(&changed, open, close), "{index}");
        }
    }
    assert!(!direct_empty_any(&tokens[..open + 1], open, close));
    let tokens = tokenize("#[path = \"child.rs\"]");
    let close = tokens.len() - 1;
    for index in open + 1..close {
        let mut changed = tokens.clone();
        changed[index].kind = Kind::Symbol("other".into());
        assert!(direct_path(&changed, open, close).is_none());
    }
    assert!(direct_path(&tokens[..open + 1], open, close).is_none());
}
