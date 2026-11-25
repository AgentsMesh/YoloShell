use crate::crates::labels::*;
use crate::crates::repositories::Evidence;

const PACKAGE: &str = "core/component";
const ROOT: &str = "lib.rs";

#[test]
fn labels_require_normalized_unambiguous_package_and_target_boundaries() {
    for (label, expected) in [
        ("//other:lib.rs", Some("other/lib.rs")),
        ("@//other:lib.rs", Some("other/lib.rs")),
        ("@@//other:lib.rs", Some("other/lib.rs")),
        (":lib.rs", Some("core/component/lib.rs")),
        ("nested:lib.rs", Some("core/component/nested/lib.rs")),
        ("lib.rs", Some("core/component/lib.rs")),
        ("//:lib.rs", Some("lib.rs")),
        ("//other", None),
        ("//other:lib.rs:extra", None),
        ("nested:lib.rs:extra", None),
        ("/lib.rs", None),
        ("//other:", None),
        ("", None),
        ("../lib.rs", None),
        ("./lib.rs", None),
        ("nested//lib.rs", None),
        ("nested\\lib.rs", None),
    ] {
        let expected = expected.map_or(Resolution::Uncertain, |path| {
            Resolution::Resolved(path.into())
        });
        assert_eq!(resolve(PACKAGE, label), expected, "{label}");
    }
    assert_eq!(
        resolve_with(PACKAGE, "@self//:lib.rs", &Evidence::Known("self".into())),
        Resolution::Resolved(ROOT.into())
    );
    assert_eq!(
        resolve_with(PACKAGE, "@other//:lib.rs", &Evidence::Known("self".into())),
        Resolution::External
    );
    for label in ["@self//:lib.rs", "@broken", "@@broken"] {
        assert_eq!(
            resolve_with(PACKAGE, label, &Evidence::Unknown),
            Resolution::Uncertain
        );
    }
}

#[test]
fn inferred_roots_require_one_best_local_candidate_without_external_ambiguity() {
    let main = Evidence::Known("self".into());
    for (labels, name, crate_name, expected) in [
        (vec![], None, None, Resolution::Uncertain),
        (
            vec!["other.rs"],
            None,
            None,
            Resolution::Resolved("core/component/other.rs".into()),
        ),
        (
            vec!["@dep//:one.rs", "@dep//:two.rs"],
            None,
            None,
            Resolution::External,
        ),
        (
            vec!["@dep//:one.rs", "lib.rs"],
            None,
            None,
            Resolution::Uncertain,
        ),
        (
            vec!["../bad.rs", "lib.rs"],
            None,
            None,
            Resolution::Uncertain,
        ),
        (
            vec!["deep/lib.rs", "lib.rs"],
            None,
            None,
            Resolution::Resolved("core/component/lib.rs".into()),
        ),
        (
            vec!["aa/lib.rs", "bb/lib.rs"],
            None,
            None,
            Resolution::Uncertain,
        ),
        (
            vec!["named.rs", "other.rs"],
            Some("named"),
            None,
            Resolution::Resolved("core/component/named.rs".into()),
        ),
        (
            vec!["named.rs", "other.rs"],
            None,
            Some("named"),
            Resolution::Resolved("core/component/named.rs".into()),
        ),
        (vec!["one.rs", "two.rs"], None, None, Resolution::Uncertain),
    ] {
        let labels = labels.into_iter().map(str::to_owned).collect::<Vec<_>>();
        assert_eq!(
            infer(PACKAGE, &labels, ROOT, name, crate_name, &main),
            expected,
            "{labels:?}"
        );
    }
    for path in ["BUILD", "nested/BUILD.bazel", "nested/rules.bzl"] {
        assert!(build_definition(path));
    }
    for path in ["", "/", "nested/file.rs"] {
        assert!(!build_definition(path));
    }
    assert!(default_root("unknown_rule").is_none());
}
