use super::fixture::Fixture;
use super::process_signal_test_support::{messages, write_contract};

const CONSTRUCTOR_POLICY: &str = "authority constructor moved";
const RETAINED_SOURCE: &str = r#"
struct RetainedExactProcess;
pub struct DurableContainment { target: RetainedExactProcess }
"#;

#[test]
fn private_retained_declaration_is_visible_to_an_out_of_line_child() {
    let fixture = base_fixture();
    fixture.write("core/src/lib.rs", "mod authority;\n");
    fixture.write(
        "core/src/authority.rs",
        r#"
struct RetainedExactProcess;
struct Secret { target: RetainedExactProcess }
mod child;
"#,
    );
    fixture.write(
        "core/src/authority/child.rs",
        "fn bypass(target: &mut super::Secret) { target.enforce_containment(); }\n",
    );
    assert_constructor_count(&messages(&fixture), 1);
}

#[test]
fn function_local_retained_type_does_not_leak_to_a_child_file() {
    let fixture = base_fixture();
    fixture.write("core/src/lib.rs", "mod authority;\n");
    fixture.write(
        "core/src/authority.rs",
        r#"
struct RetainedExactProcess;
fn local() { struct Secret { target: RetainedExactProcess } }
mod child;
"#,
    );
    fixture.write(
        "core/src/authority/child.rs",
        "fn clean(target: &mut super::Secret) { target.enforce_containment(); }\n",
    );
    assert_constructor_count(&messages(&fixture), 0);
}

#[test]
fn public_type_alias_is_available_through_a_glob_reexport() {
    let fixture = base_fixture();
    fixture.write(
        "core/src/lib.rs",
        "pub mod authority; pub mod bridge; mod consumer;\n",
    );
    fixture.write("core/src/authority.rs", RETAINED_SOURCE);
    fixture.write(
        "core/src/bridge.rs",
        "pub use crate::authority::DurableContainment as Hidden;\n",
    );
    fixture.write(
        "core/src/consumer.rs",
        "use crate::bridge::*; fn bypass(target: &mut Hidden) { target.enforce_containment(); }\n",
    );
    assert_constructor_count(&messages(&fixture), 1);
}

#[test]
fn pub_self_alias_does_not_escape_its_owner_module() {
    let fixture = base_fixture();
    fixture.write(
        "core/src/lib.rs",
        "mod authority; mod parent; mod outsider;\n",
    );
    fixture.write("core/src/authority.rs", RETAINED_SOURCE);
    fixture.write(
        "core/src/parent.rs",
        "pub(self) use crate::authority::DurableContainment as Hidden; mod child;\n",
    );
    fixture.write(
        "core/src/parent/child.rs",
        "fn bypass(target: &mut super::Hidden) { target.enforce_containment(); }\n",
    );
    fixture.write(
        "core/src/outsider.rs",
        "fn clean(target: &mut crate::parent::Hidden) { target.enforce_containment(); }\n",
    );
    assert_constructor_count(&messages(&fixture), 1);
}

#[test]
fn pub_super_alias_is_visible_to_the_parent_subtree() {
    let fixture = base_fixture();
    fixture.write("core/src/lib.rs", "mod authority; mod parent;\n");
    fixture.write("core/src/authority.rs", RETAINED_SOURCE);
    fixture.write("core/src/parent.rs", "mod bridge; mod sibling;\n");
    fixture.write(
        "core/src/parent/bridge.rs",
        "pub(super) use crate::authority::DurableContainment as Hidden;\n",
    );
    fixture.write(
        "core/src/parent/sibling.rs",
        "fn bypass(target: &mut super::bridge::Hidden) { target.enforce_containment(); }\n",
    );
    assert_constructor_count(&messages(&fixture), 1);
}

#[test]
fn module_item_declaration_shadows_a_crate_scope_alias() {
    let fixture = base_fixture();
    fixture.write(
        "core/src/lib.rs",
        "mod authority; use authority as hidden; mod consumer;\n",
    );
    fixture.write("core/src/authority.rs", RETAINED_SOURCE);
    fixture.write(
        "core/src/consumer.rs",
        r#"
mod hidden { pub struct DurableContainment; }
fn clean(target: &mut hidden::DurableContainment) {
    target.enforce_containment();
}
"#,
    );
    assert_constructor_count(&messages(&fixture), 0);
}

fn base_fixture() -> Fixture {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture
}

fn assert_constructor_count(result: &[String], expected: usize) {
    let actual = result
        .iter()
        .filter(|message| message.contains(CONSTRUCTOR_POLICY))
        .count();
    assert_eq!(actual, expected, "{result:#?}");
}
