use super::fixture::Fixture;
use super::process_signal_test_support::{messages, write_contract};
use yolo_architecture_policy_contracts::process_signal as contract;
use yolo_architecture_policy_contracts::process_signal_retained as retained;

const CONSUMER_PATH: &str = "core/crates/runtime/engine/process/src/module_scope_consumer.rs";
const CONSTRUCTOR_POLICY: &str = "authority constructor moved";
const ROOT_PATH: &str = "core/src/lib.rs";
const BASE_AUTHORITY_PATH: &str = "core/src/authority.rs";
const SCOPED_AUTHORITY_PATH: &str = "core/src/outer/authority.rs";
const VISIBILITY_FINDINGS: usize = 3;

const RETAINED_SOURCE: &str = r#"
struct RetainedExactProcess;
pub struct DurableContainment { target: RetainedExactProcess }
"#;
const CLEAN_SOURCE: &str = "pub struct DurableContainment;\n";

#[test]
fn nested_path_attribute_selects_only_the_scoped_file() {
    let retained = path_mapping_messages(CLEAN_SOURCE, RETAINED_SOURCE);
    assert_has_constructor(&retained);

    let clean = path_mapping_messages(RETAINED_SOURCE, CLEAN_SOURCE);
    assert_lacks_constructor(&clean);
}

#[test]
fn nested_inline_declarations_map_out_of_line_modules() {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(ROOT_PATH, "pub mod outer { pub mod nested; }\n");
    fixture.write("core/src/outer/nested.rs", RETAINED_SOURCE);
    fixture.write(
        CONSUMER_PATH,
        r#"
use crate::outer::nested::DurableContainment;
fn inspect(target: &mut DurableContainment) { target.begin_containment(); }
"#,
    );
    assert_has_constructor(&messages(&fixture));
}

#[test]
fn scoped_inline_same_name_exports_resolve_independently() {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(ROOT_PATH, "pub mod bridge;\n");
    fixture.write(
        "core/src/bridge.rs",
        r#"
struct RetainedExactProcess;
pub mod retained {
    use super::RetainedExactProcess;
    pub struct Shared { target: RetainedExactProcess }
}
pub mod clean { pub struct Shared; }
"#,
    );
    fixture.write(
        CONSUMER_PATH,
        r#"
use crate::bridge::clean::Shared;
fn ambiguous(target: &mut Shared) { target.enforce_containment(); }
"#,
    );
    let result = messages(&fixture);
    assert_lacks_constructor(&result);
    assert_eq!(
        result
            .iter()
            .filter(|message| message.contains(contract::AUTHORITY_TYPE_MESSAGE))
            .count(),
        VISIBILITY_FINDINGS,
        "{result:#?}",
    );
}

#[test]
fn mixed_same_destination_reexports_fail_closed() {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(ROOT_PATH, "pub mod bridge;\n");
    fixture.write(
        "core/src/bridge.rs",
        r#"
struct RetainedExactProcess;
pub struct Aggregate { target: RetainedExactProcess }
pub type Canonical = RetainedExactProcess;
pub use self::Aggregate as Mixed;
pub use self::Canonical as Mixed;
"#,
    );
    fixture.write(
        CONSUMER_PATH,
        "use crate::bridge::Mixed;\nfn inspect(_: Mixed) {}\n",
    );
    let result = messages(&fixture);
    assert!(
        result
            .iter()
            .any(|message| message.contains(retained::RESOLUTION_INDETERMINATE_MESSAGE)),
        "{result:#?}",
    );
}

fn path_mapping_messages(base: &str, scoped: &str) -> Vec<String> {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(
        ROOT_PATH,
        r#"
pub mod outer {
    #[path = "authority.rs"]
    pub mod selected;
}
"#,
    );
    fixture.write(BASE_AUTHORITY_PATH, base);
    fixture.write(SCOPED_AUTHORITY_PATH, scoped);
    fixture.write(
        CONSUMER_PATH,
        r#"
use crate::outer::selected::DurableContainment;
fn inspect(target: &mut DurableContainment) { target.enforce_containment(); }
"#,
    );
    messages(&fixture)
}

fn assert_has_constructor(result: &[String]) {
    assert!(
        result
            .iter()
            .any(|message| message.contains(CONSTRUCTOR_POLICY)),
        "{result:#?}"
    );
}

fn assert_lacks_constructor(result: &[String]) {
    assert!(
        result
            .iter()
            .all(|message| !message.contains(CONSTRUCTOR_POLICY)),
        "{result:#?}"
    );
}
