use super::fixture::Fixture;
use super::process_signal_test_support::{messages, write_contract};

const CONSTRUCTOR_POLICY: &str = "authority constructor moved";
const RETAINED_SOURCE: &str = r#"
struct RetainedExactProcess;
pub struct DurableContainment { target: RetainedExactProcess }
"#;

#[test]
fn pub_crate_grouped_alias_is_visible_across_the_crate() {
    let fixture = rooted_fixture("mod authority; mod parent; mod outsider;\n");
    fixture.write(
        "core/src/parent.rs",
        "pub(crate) use crate::authority::{DurableContainment as Hidden};\n",
    );
    fixture.write("core/src/outsider.rs", consumer("crate::parent::Hidden"));
    assert_constructor_count(&messages(&fixture), 1);
}

#[test]
fn pub_super_nested_grouped_alias_is_visible_to_a_sibling() {
    let fixture = parent_fixture();
    fixture.write(
        "core/src/parent/bridge.rs",
        "pub(super) use crate::{authority::{DurableContainment as Hidden}};\n",
    );
    fixture.write(
        "core/src/parent/sibling.rs",
        consumer("super::bridge::Hidden"),
    );
    assert_constructor_count(&messages(&fixture), 1);
}

#[test]
fn pub_in_grouped_alias_stays_inside_the_named_subtree() {
    let fixture = parent_fixture();
    fixture.write(
        "core/src/parent/bridge.rs",
        "pub(in crate::parent) use crate::{authority::{DurableContainment as Hidden}};\n",
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

#[test]
fn self_restricted_grouped_alias_is_visible_only_to_descendants() {
    for visibility in ["pub(self)", "pub(in self)"] {
        let fixture = rooted_fixture("mod authority; mod parent; mod outsider;\n");
        fixture.write(
            "core/src/parent.rs",
            format!(
                "{visibility} use crate::authority::{{DurableContainment as Hidden}}; mod child;\n"
            ),
        );
        fixture.write("core/src/parent/child.rs", consumer("super::Hidden"));
        fixture.write("core/src/outsider.rs", consumer("crate::parent::Hidden"));
        assert_constructor_count(&messages(&fixture), 1);
    }
}

fn parent_fixture() -> Fixture {
    let fixture = rooted_fixture("mod authority; mod parent; mod outsider;\n");
    fixture.write("core/src/parent.rs", "mod bridge; mod sibling;\n");
    fixture.write("core/src/outsider.rs", "pub fn clean() {}\n");
    fixture
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

fn consumer(retained_type: &str) -> String {
    format!("fn bypass(target: &mut {retained_type}) {{ target.enforce_containment(); }}\n")
}

fn assert_constructor_count(result: &[String], expected: usize) {
    let actual = result
        .iter()
        .filter(|message| message.contains(CONSTRUCTOR_POLICY))
        .count();
    assert_eq!(actual, expected, "{result:#?}");
}
