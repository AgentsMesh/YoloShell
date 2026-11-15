use super::*;
use yolo_architecture_syntax::targeted_tokens::tokenize;

#[test]
fn outer_impl_type_ignores_nested_generic_names_and_where_constraints() {
    for (text, expected) in [
        ("", None),
        ("<T>", None),
        ("Owner<T>", Some("Owner")),
        ("Owner<Nested<T>>", Some("Owner")),
        ("Owner<T> where T: Other", Some("Owner")),
        ("::module::Owner", Some("Owner")),
        (">> Owner", Some("Owner")),
    ] {
        let tokens = tokenize(text);
        assert_eq!(
            outer_type_name(&tokens).map(|(_, name)| name),
            expected,
            "{text}"
        );
    }
    for text in [
        "impl",
        "impl Owner",
        "impl Owner {",
        "impl {}",
        "impl for {}",
    ] {
        assert!(impls(&tokenize(text)).next().is_none(), "{text}");
    }
}

#[test]
fn deref_trait_aliases_and_target_assignments_keep_their_exact_meaning() {
    let tokens = tokenize(
        "as Unknown; 5 as Other; Deref as Read; Read as Again; DerefMut as Write; Deref as",
    );
    let (read, write) = trait_names(&tokens);
    assert!(read.contains("Read") && read.contains("Again"));
    assert!(write.contains("Write"));
    assert!(!read.contains("Unknown") && !read.contains("Other"));
    for (body, expected) in [
        ("type Target = RetainedExactProcess;", true),
        ("type Target = RetainedExactProcess", true),
        ("type Other = RetainedExactProcess;", false),
        ("type Target; RetainedExactProcess;", false),
        ("type Target = Other;", false),
        ("type", false),
    ] {
        let tokens = tokenize(body);
        assert_eq!(
            target_is_retained(&tokens, &crate::types::type_names(&tokens)),
            expected,
            "{body}"
        );
    }
}
