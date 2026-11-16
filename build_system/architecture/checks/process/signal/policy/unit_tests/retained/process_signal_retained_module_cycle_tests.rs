use super::fixture::Fixture;
use super::process_signal_test_support::{messages, write_contract};

const CONSUMER_PATH: &str = "core/crates/runtime/engine/process/src/module_cycle_consumer.rs";
const CONSTRUCTOR_POLICY: &str = "authority constructor moved";
const RETAINED_SOURCE: &str = r#"
struct RetainedExactProcess;
pub struct DurableContainment { target: RetainedExactProcess }
"#;

#[test]
fn public_self_crate_aliases_converge_without_path_growth() {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(
        "core/src/lib.rs",
        "pub extern crate self as yolo;\npub mod authority;\n",
    );
    fixture.write("core/src/authority.rs", RETAINED_SOURCE);
    fixture.write(
        CONSUMER_PATH,
        r#"
use crate::yolo::authority::DurableContainment;
fn bypass(target: &mut DurableContainment) { target.enforce_containment(); }
"#,
    );
    assert_has_constructor(&messages(&fixture));
}

#[test]
fn cyclic_public_module_aliases_resolve_finite_consumer_paths() {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(
        "core/src/lib.rs",
        "pub mod authority;\npub mod left;\npub mod right;\n",
    );
    fixture.write("core/src/authority.rs", RETAINED_SOURCE);
    fixture.write(
        "core/src/left.rs",
        "pub use crate::right as hop;\npub use crate::authority as payload;\n",
    );
    fixture.write("core/src/right.rs", "pub use crate::left as hop;\n");
    fixture.write(
        CONSUMER_PATH,
        r#"
use crate::left::hop::hop::payload::DurableContainment;
fn bypass(target: &mut DurableContainment) { target.begin_containment(); }
"#,
    );
    assert_has_constructor(&messages(&fixture));
}

fn assert_has_constructor(result: &[String]) {
    assert!(
        result
            .iter()
            .any(|message| message.contains(CONSTRUCTOR_POLICY)),
        "{result:#?}"
    );
}
