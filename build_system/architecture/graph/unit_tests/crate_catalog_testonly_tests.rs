use super::*;

#[test]
fn literal_testonly_libraries_are_not_product_roots() {
    for (value, expected) in [("True", false), ("False", true), ("TEST_ONLY", true)] {
        let catalog = one(
            "core/BUILD.bazel",
            &format!(
                "rust_library(name = \"library\", crate_root = \"src/lib.rs\", testonly = {value})"
            ),
        );
        assert_eq!(root(&catalog, "core/src/lib.rs").product, expected);
    }
}

#[test]
fn schema_templates_and_output_groups_do_not_hide_other_roots_in_the_same_file() {
    use yolo_architecture_policy_contracts::entrypoint_macros::{
        CAPNP_DEFINITIONS, CAPNP_GENERATION,
    };
    for (path, template) in [
        (
            CAPNP_DEFINITIONS,
            "def schema(crate_root):\n    rust_library(crate_root = crate_root)\n",
        ),
        (
            CAPNP_GENERATION,
            "def generate(ctx):\n    return [OutputGroupInfo(crate_root = depset([root]))]\n",
        ),
    ] {
        assert!(one(path, template).unresolved.is_empty());
        let concrete = format!("{template}\nrust_library(crate_root = \"src/authority.rs\")\n");
        let catalog = one(path, &concrete);
        assert_eq!(
            sources(&catalog),
            set(&["build_system/bazel/capnp/src/authority.rs"])
        );
        let unknown = format!("{template}\nrust_binary(crate_root = hidden())\n");
        assert_eq!(one(path, &unknown).unresolved, set(&[path]));
        assert!(!one("unreviewed/schema.bzl", template).unresolved.is_empty());
    }
    let authored = one(
        "core/schema/BUILD.bazel",
        "capnp_rust_library(crate_root = \"src/lib.rs\")",
    );
    assert_eq!(sources(&authored), set(&["core/schema/src/lib.rs"]));
    assert!(
        !one(
            CAPNP_GENERATION,
            "    OutputGroupInfo(crate_root = arbitrary())"
        )
        .unresolved
        .is_empty()
    );
}

#[test]
fn near_match_output_groups_cannot_hide_unresolved_crate_authority() {
    use yolo_architecture_policy_contracts::entrypoint_macros::CAPNP_GENERATION;
    for expression in [
        "arbitrary([root])",
        "depset[[root]]",
        "depset((root))",
        "depset([unverified])",
    ] {
        let definition = format!(
            "def generate(ctx):\n    return [OutputGroupInfo(crate_root = {expression})]\n"
        );
        let catalog = one(CAPNP_GENERATION, &definition);
        assert!(catalog.roots.is_empty(), "{expression}");
        assert_eq!(catalog.unresolved, set(&[CAPNP_GENERATION]), "{expression}");
    }
}
