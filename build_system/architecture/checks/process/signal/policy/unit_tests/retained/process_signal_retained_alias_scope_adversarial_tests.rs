use super::fixture::Fixture;
use super::process_signal_test_support::{messages, write_contract};

const CONSTRUCTOR_POLICY: &str = "authority constructor moved";
const CONSUMER_PATH: &str = "core/src/consumer.rs";
const RETAINED_SOURCE: &str = r#"
struct RetainedExactProcess;
pub struct DurableContainment { target: RetainedExactProcess }
"#;
const EXPECTED_RETAINED_CALLS: usize = 1;
const AUTHORITY_TYPE_POLICY: &str = "authority type escaped";

#[test]
fn parent_private_module_alias_is_visible_to_child_files() {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(
        "core/src/lib.rs",
        "mod authority; use authority as hidden; mod consumer;\n",
    );
    fixture.write("core/src/authority.rs", RETAINED_SOURCE);
    fixture.write(
        CONSUMER_PATH,
        r#"
fn bypass(target: &mut super::hidden::DurableContainment) {
    target.enforce_containment();
}
"#,
    );
    assert_retained_count(&messages(&fixture), EXPECTED_RETAINED_CALLS);
}

#[test]
fn explicit_roots_expand_the_following_module_alias() {
    let fixture = retained_root();
    fixture.write(
        CONSUMER_PATH,
        r#"
fn crate_path(target: &mut crate::hidden::DurableContainment) {
    target.enforce_containment();
}
mod bridge {
    use crate::authority as hidden;
    fn self_path(target: &mut self::hidden::DurableContainment) {
        target.begin_containment();
    }
    mod child {
        fn super_path(target: &mut super::hidden::DurableContainment) {
            target.enforce_containment();
        }
    }
}
"#,
    );
    assert_retained_count(&messages(&fixture), 3);
}

#[test]
fn block_aliases_shadow_only_within_their_lexical_scope() {
    let fixture = retained_root();
    fixture.write("core/src/clean.rs", "pub struct DurableContainment;\n");
    fixture.write(
        CONSUMER_PATH,
        r#"
fn retained() {
    use crate::authority as hidden;
    fn inspect(target: &mut hidden::DurableContainment) {
        target.enforce_containment();
    }
}
fn clean() {
    use crate::clean as hidden;
    fn inspect(target: &mut hidden::DurableContainment) {
        target.enforce_containment();
    }
}
"#,
    );
    assert_retained_count(&messages(&fixture), EXPECTED_RETAINED_CALLS);
}

#[test]
fn direct_inner_module_declaration_shadows_parent_alias() {
    let fixture = retained_root();
    fixture.write(
        CONSUMER_PATH,
        r#"
use crate::authority as hidden;
fn clean() {
    mod hidden { pub struct DurableContainment; }
    fn inspect(target: &mut hidden::DurableContainment) {
        target.enforce_containment();
    }
}
"#,
    );
    assert_retained_count(&messages(&fixture), 0);
}

#[test]
fn function_type_parameter_shadows_parent_module_alias() {
    let fixture = retained_root();
    fixture.write(
        CONSUMER_PATH,
        r#"
use crate::authority as hidden;
trait Clean { type DurableContainment; }
fn clean<hidden: Clean>(target: &mut hidden::DurableContainment) {
    target.enforce_containment();
}
"#,
    );
    assert_retained_count(&messages(&fixture), 0);
}

#[test]
fn parameter_type_arguments_do_not_forge_generic_shadows() {
    let fixture = retained_root();
    fixture.write(
        CONSUMER_PATH,
        r#"
use crate::authority as hidden;
fn bypass(_: Vec<hidden::Clean>, target: &mut hidden::DurableContainment) {
    target.enforce_containment();
}
"#,
    );
    assert_retained_count(&messages(&fixture), EXPECTED_RETAINED_CALLS);
}

#[test]
fn semicolon_item_generics_shadow_module_aliases() {
    let fixture = retained_root();
    fixture.write(
        CONSUMER_PATH,
        r#"
use crate::authority as hidden;
trait Clean { type DurableContainment; }
pub type Alias<hidden: Clean> = hidden::DurableContainment;
pub trait Adapter {
    fn clean<hidden: Clean>(target: &mut hidden::DurableContainment);
}
"#,
    );
    assert!(messages(&fixture).iter().all(|message| {
        !message.contains(CONSUMER_PATH) || !message.contains(AUTHORITY_TYPE_POLICY)
    }));
}

#[test]
fn lifetime_parameters_do_not_shadow_type_namespace_aliases() {
    let fixture = retained_root();
    fixture.write(
        CONSUMER_PATH,
        r#"
use crate::authority as hidden;
fn bypass<'hidden>(target: &mut hidden::DurableContainment) {
    target.enforce_containment();
}
"#,
    );
    assert_retained_count(&messages(&fixture), EXPECTED_RETAINED_CALLS);
}

fn retained_root() -> Fixture {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(
        "core/src/lib.rs",
        "pub mod authority; pub mod clean; pub mod consumer; use crate::authority as hidden;\n",
    );
    fixture.write(
        "core/src/authority.rs",
        format!("{RETAINED_SOURCE}\npub struct Clean;\n"),
    );
    fixture
}

fn assert_retained_count(result: &[String], expected: usize) {
    let actual = result
        .iter()
        .filter(|message| message.contains(CONSTRUCTOR_POLICY))
        .count();
    assert_eq!(actual, expected, "{result:#?}");
}
