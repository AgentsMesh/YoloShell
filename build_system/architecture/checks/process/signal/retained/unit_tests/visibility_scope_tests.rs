use super::*;
use yolo_architecture_syntax::targeted_tokens::{Kind, tokenize};

#[test]
fn visibility_scope_must_resolve_to_an_existing_ancestor() {
    let owner = ["root", "outer", "inner"].map(str::to_owned);
    for (text, expected) in [
        ("pub", Some(1)),
        ("pub(crate)", Some(1)),
        ("pub(self)", Some(3)),
        ("pub(super)", Some(2)),
        ("pub(in crate::outer)", Some(2)),
        ("pub(super::super)", Some(1)),
        ("pub(super::super::super)", None),
        ("pub(super::other)", None),
        ("pub(in crate::other)", None),
        ("pub(other)", None),
        ("pub()", None),
        ("pub(in)", None),
        ("pub(crate::)", None),
        ("pub(crate.outer)", None),
        ("pub(crate::5)", None),
        ("pub(", None),
        ("private", None),
    ] {
        let tokens = tokenize(text);
        assert_eq!(
            audience(&tokens, 0, &owner).map(|path| path.len()),
            expected,
            "{text}"
        );
        assert_eq!(
            restricted(&tokens, 0, &owner).is_some(),
            expected == Some(2),
            "{text}"
        );
    }
    assert!(audience(&[], 0, &owner).is_none());
    assert!(audience(&tokenize("pub(crate)"), 0, &[]).is_none());
}

#[test]
fn restricted_path_separators_cannot_be_replaced_by_other_token_kinds() {
    let tokens = tokenize("crate::outer");
    for index in 0..tokens.len() {
        let mut changed = tokens.clone();
        changed[index].kind = Kind::Number(0);
        assert!(separated_identifiers(&changed).is_none(), "{index}");
    }
    assert!(separated_identifiers(&[]).is_none());
    assert!(separated_identifiers(&tokenize("crate::")).is_none());
    assert!(parent_scope(&[], 1).is_none());
}
