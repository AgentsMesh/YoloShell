use super::fixture::Fixture;
use super::process_signal_test_support::{messages, write_contract};

const CONSTRUCTOR_POLICY: &str = "authority constructor moved";
const TYPE_POLICY: &str = "process signal authority type escaped";
const RETAINED_SOURCE: &str = r#"
struct RetainedExactProcess;
pub struct DurableContainment { target: RetainedExactProcess }
"#;

#[test]
fn nonstandard_crate_root_resolves_out_of_line_path_modules() {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(
        "core/BUILD.bazel",
        r#"
rust_binary(
    name = "attach",
    crate_root = "src/attach_main.rs",
)
"#,
    );
    fixture.write(
        "core/src/attach_main.rs",
        r#"
#[path = "attach/authority.rs"]
mod authority;
#[path = "attach/consumer.rs"]
mod consumer;
"#,
    );
    fixture.write("core/src/attach/authority.rs", RETAINED_SOURCE);
    fixture.write(
        "core/src/attach/consumer.rs",
        r#"
fn bypass(target: &mut crate::authority::DurableContainment) {
    target.enforce_containment();
}
"#,
    );
    assert_has_constructor(&messages(&fixture));
}

#[test]
fn parent_private_type_import_is_visible_to_child_files() {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(
        "core/src/lib.rs",
        "mod authority; use authority::DurableContainment as Hidden; mod child;\n",
    );
    fixture.write("core/src/authority.rs", RETAINED_SOURCE);
    fixture.write(
        "core/src/child.rs",
        r#"
fn bypass(target: &mut super::Hidden) {
    target.enforce_containment();
}
"#,
    );
    assert_has_constructor(&messages(&fixture));
}

#[test]
fn non_root_path_attribute_is_relative_to_the_source_directory() {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write("core/src/lib.rs", "mod remote;\n");
    fixture.write("core/src/remote.rs", "mod local_file;\n");
    fixture.write(
        "core/src/remote/local_file.rs",
        r#"
#[path = "local_file_identity.rs"]
mod identity;
fn bypass(target: &mut identity::DurableContainment) {
    target.enforce_containment();
}
"#,
    );
    fixture.write("core/src/remote/local_file_identity.rs", RETAINED_SOURCE);
    assert_has_constructor(&messages(&fixture));
}

#[test]
fn inline_path_attribute_keeps_the_non_mod_file_stem_directory() {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write("core/src/lib.rs", "mod remote;\n");
    fixture.write("core/src/remote.rs", "mod local_file;\n");
    fixture.write(
        "core/src/remote/local_file.rs",
        r#"
mod inline {
    #[path = "identity.rs"]
    mod identity;
    fn bypass(target: &mut identity::DurableContainment) {
        target.enforce_containment();
    }
}
"#,
    );
    fixture.write(
        "core/src/remote/local_file/inline/identity.rs",
        RETAINED_SOURCE,
    );
    assert_has_constructor(&messages(&fixture));
}

#[test]
fn path_overridden_lib_named_module_resolves_adjacent_children() {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(
        "core/src/lib.rs",
        "#[path = \"nested/lib.rs\"] mod outer;\n",
    );
    fixture.write(
        "core/src/nested/lib.rs",
        r#"
mod child;
fn bypass(target: &mut child::DurableContainment) {
    target.enforce_containment();
}
"#,
    );
    fixture.write("core/src/nested/child.rs", RETAINED_SOURCE);
    assert_has_constructor(&messages(&fixture));
}

#[test]
fn local_product_crates_are_scanned_outside_the_runtime_directory() {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(
        "tools/runtime/BUILD.bazel",
        "rust_library(name = \"escaped\", crate_root = \"src/lib.rs\")\n",
    );
    fixture.write(
        "tools/runtime/src/lib.rs",
        r#"
struct RetainedExactProcess;
struct DurableContainment { target: RetainedExactProcess }
fn bypass(target: &mut DurableContainment) { target.enforce_containment(); }
"#,
    );
    assert_has_constructor(&messages(&fixture));
}

#[test]
fn cross_crate_extern_prelude_cannot_bypass_producer_visibility() {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(
        "core/BUILD.bazel",
        r#"
rust_library(
    name = "authority",
    crate_name = "authority",
    crate_root = "src/process/exact_termination.rs",
)
rust_binary(
    name = "consumer",
    crate_root = "src/consumer_main.rs",
    deps = [":authority"],
)
"#,
    );
    fixture.write(
        "core/src/process/exact_termination.rs",
        "struct RetainedExactProcess;\npub struct DurableContainment { target: RetainedExactProcess }\n",
    );
    fixture.write(
        "core/src/consumer_main.rs",
        "fn bypass(target: &mut authority::DurableContainment) { target.enforce_containment(); }\n",
    );
    assert_has(&messages(&fixture), TYPE_POLICY);
}

fn assert_has_constructor(result: &[String]) {
    assert!(
        result
            .iter()
            .any(|message| message.contains(CONSTRUCTOR_POLICY)),
        "{result:#?}"
    );
}

fn assert_has(result: &[String], expected: &str) {
    assert!(
        result.iter().any(|message| message.contains(expected)),
        "{result:#?}"
    );
}
