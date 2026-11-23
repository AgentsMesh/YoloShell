use super::fixture::Fixture;
use super::process_signal_test_support::{messages, write_contract};

const CONSTRUCTOR_POLICY: &str = "authority constructor moved";
const RETAINED_SOURCE: &str = r#"
struct RetainedExactProcess;
pub struct DurableContainment { target: RetainedExactProcess }
"#;

#[test]
fn private_type_alias_is_visible_to_children_not_outsiders() {
    assert_parent_alias_scope("type");
}

#[test]
fn self_restricted_type_alias_is_visible_to_children_not_outsiders() {
    for visibility in ["pub(self) type", "pub(in self) type"] {
        assert_parent_alias_scope(visibility);
    }
}

#[test]
fn pub_crate_type_alias_is_visible_across_the_crate() {
    let fixture = rooted_fixture("mod authority; mod parent; mod outsider;\n");
    fixture.write("core/src/parent.rs", alias("pub(crate) type", "Hidden"));
    fixture.write("core/src/outsider.rs", consumer("crate::parent::Hidden"));
    assert_constructor_count(&messages(&fixture), 1);
}

#[test]
fn pub_in_type_alias_stays_inside_the_named_subtree() {
    let fixture = rooted_fixture("mod authority; mod parent; mod outsider;\n");
    fixture.write("core/src/parent.rs", "mod bridge; mod sibling;\n");
    fixture.write(
        "core/src/parent/bridge.rs",
        alias("pub(in crate::parent) type", "Hidden"),
    );
    fixture.write(
        "core/src/parent/sibling.rs",
        consumer("super::bridge::Hidden"),
    );
    fixture.write(
        "core/src/outsider.rs",
        consumer("crate::parent::bridge::Hidden"),
    );
    assert_constructor_count(&messages(&fixture), 1);
}

fn assert_parent_alias_scope(visibility: &str) {
    let fixture = rooted_fixture("mod authority; mod parent; mod outsider;\n");
    fixture.write(
        "core/src/parent.rs",
        format!("{}mod child;\n", alias(visibility, "Hidden")),
    );
    fixture.write("core/src/parent/child.rs", consumer("super::Hidden"));
    fixture.write("core/src/outsider.rs", consumer("crate::parent::Hidden"));
    assert_constructor_count(&messages(&fixture), 1);
}

fn alias(visibility: &str, name: &str) -> String {
    format!("{visibility} {name} = crate::authority::DurableContainment;\n")
}

fn consumer(retained_type: &str) -> String {
    format!("fn bypass(target: &mut {retained_type}) {{ target.enforce_containment(); }}\n")
}

fn rooted_fixture(root: &str) -> Fixture {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(
        "core/BUILD.bazel",
        "rust_library(name = \"root\", crate_root = \"src/lib.rs\")\n",
    );
    fixture.write("core/src/lib.rs", root);
    fixture.write("core/src/authority.rs", RETAINED_SOURCE);
    fixture
}

fn assert_constructor_count(result: &[String], expected: usize) {
    let actual = result
        .iter()
        .filter(|message| message.contains(CONSTRUCTOR_POLICY))
        .count();
    assert_eq!(actual, expected, "{result:#?}");
}
