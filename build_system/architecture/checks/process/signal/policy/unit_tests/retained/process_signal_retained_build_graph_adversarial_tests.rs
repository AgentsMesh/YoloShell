use super::fixture::Fixture;
use super::process_signal_test_support::{messages, write_contract};
use std::collections::BTreeSet;
use std::path::PathBuf;
use yolo_architecture_foundation::SourceFile;
use yolo_architecture_policy_contracts::process_signal_retained::RUST_BINARY_RULE;
use yolo_architecture_build_graph::crates::{Catalog, CrateRoot};
use yolo_architecture_syntax::rust_source::RustSource;

const RESOLUTION_POLICY: &str = "retained authority resolution is indeterminate";
const CONSTRUCTOR_POLICY: &str = "authority constructor moved";
const ROOT_SOURCE: &str = "core/src/lib.rs";
const RULES_RUST_LOAD: &str = "load(\"@rules_rust//rust:defs.bzl\", rb = \"rust_binary\")";
const RETAINED_SOURCE: &str = "struct RetainedExactProcess;\n\
    struct DurableContainment { target: RetainedExactProcess }\n\
    fn bypass(target: &mut DurableContainment) { target.enforce_containment(); }\n";

#[test]
fn product_rule_keyword_forwarding_fails_closed() {
    for prefix in ["", RULES_RUST_LOAD] {
        let fixture = Fixture::new();
        write_contract(&fixture);
        fixture.write(
            "tools/product.bzl",
            format!(
                "{prefix}\ndef product(**kwargs):\n    {}(**kwargs)\n",
                if prefix.is_empty() {
                    RUST_BINARY_RULE
                } else {
                    "rb"
                }
            ),
        );
        assert_resolution_violation(&messages(&fixture));
    }
}

#[test]
fn proven_rules_rust_alias_discovers_the_local_product_root() {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(
        "core/BUILD.bazel",
        format!("{RULES_RUST_LOAD}\nrb(name = \"guarded\", srcs = [\"src/lib.rs\"])\n"),
    );
    fixture.write(ROOT_SOURCE, RETAINED_SOURCE);
    assert!(
        messages(&fixture)
            .iter()
            .any(|message| message.contains(CONSTRUCTOR_POLICY))
    );
}

#[test]
fn unproven_dynamic_and_conflicting_aliases_fail_closed() {
    for definition in [
        "load(\"//tools:defs.bzl\", rb = \"rust_binary\")",
        "load(\"@rules_rust//rust:defs.bzl\", rb = selected_rule())",
        "load(\"@rules_rust//rust:defs.bzl\", rb = \"rust_binary\", rb = \"rust_library\")",
    ] {
        let fixture = Fixture::new();
        write_contract(&fixture);
        fixture.write("tools/product.bzl", definition);
        assert_resolution_violation(&messages(&fixture));
    }
}

#[test]
fn generated_or_filegroup_crate_roots_fail_closed() {
    for attribute in [
        "srcs = [\":generated_main\"]",
        "crate_root = \":generated_main\"",
    ] {
        let fixture = Fixture::new();
        write_contract(&fixture);
        fixture.write(
            "core/BUILD.bazel",
            format!("rust_binary(name = \"generated\", {attribute})\n"),
        );
        assert_resolution_violation(&messages(&fixture));
    }
}

#[test]
fn targets_reusing_one_crate_root_share_one_module_owner() {
    let file = SourceFile {
        absolute: PathBuf::from(ROOT_SOURCE),
        relative: ROOT_SOURCE.to_owned(),
        text: "struct Value;".to_owned(),
    };
    let parsed = RustSource::parse(&file.text);
    let crates = Catalog {
        roots: BTreeSet::from([
            root("core/BUILD.bazel:1:runtime", true),
            root("core/BUILD.bazel:2:tests", false),
        ]),
        unresolved: BTreeSet::new(),
    };
    let modules =
        yolo_architecture_build_graph::modules::discover(&[(&file, parsed)], &crates);
    assert!(modules.is_product(ROOT_SOURCE));
    assert_eq!(modules.paths_at(ROOT_SOURCE, 0).len(), 1);
}

fn root(id: &str, product: bool) -> CrateRoot {
    CrateRoot {
        id: id.to_owned(),
        source: ROOT_SOURCE.to_owned(),
        product,
    }
}

fn assert_resolution_violation(result: &[String]) {
    assert!(
        result
            .iter()
            .any(|message| message.contains(RESOLUTION_POLICY)),
        "{result:#?}"
    );
}
