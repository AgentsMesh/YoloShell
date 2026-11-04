use yolo_architecture_foundation::text::joined;
use yolo_architecture_test_support::Fixture;

const PRODUCTION_PATH: &str = "core/crates/runtime/engine/process/src/linux_containment_future.rs";
const TEST_PATH: &str = "core/crates/runtime/engine/process/src/linux_containment_future_tests.rs";
const POLICY_TEXT: &str = "must never expose or write cgroup thaw authority";

fn messages(fixture: &Fixture) -> Vec<String> {
    crate::inspect(fixture.root())
        .unwrap()
        .into_iter()
        .map(|violation| violation.to_string())
        .collect()
}

#[test]
fn rejects_named_thaw_authority_and_aliases_in_production() {
    let fixture = Fixture::new();
    let constant = joined(&["THAW", "_VALUE"]);
    let method = joined(&["th", "aw"]);
    fixture.write(
        PRODUCTION_PATH,
        format!(
            "use constants::{constant} as RESUME_VALUE;\nfn {method}() {{ write(RESUME_VALUE); }}\n"
        ),
    );
    let result = messages(&fixture);
    assert_eq!(result.len(), 2);
    assert!(result.iter().all(|message| message.contains(POLICY_TEXT)));
}

#[test]
fn rejects_raw_cgroup_thaw_value_in_production() {
    let fixture = Fixture::new();
    let value = joined(&["0", "\\n"]);
    fixture.write(
        PRODUCTION_PATH,
        format!("fn resume() {{ write(\"{value}\"); }}\n"),
    );
    let result = messages(&fixture);
    assert_eq!(result.len(), 1);
    assert!(result[0].contains(POLICY_TEXT));
}

#[test]
fn permits_test_only_fixtures_and_cfg_test_functions() {
    let fixture = Fixture::new();
    let constant = joined(&["LINUX_CGROUP_", "THAW_VALUE"]);
    fixture.write(TEST_PATH, format!("const VALUE: &str = {constant};\n"));
    fixture.write(
        PRODUCTION_PATH,
        format!("#[cfg(test)]\nfn fixture() {{ write({constant}); }}\n"),
    );
    assert!(messages(&fixture).is_empty());
}

#[test]
fn ignores_comments_and_non_authority_strings() {
    let fixture = Fixture::new();
    let name = joined(&["THAW", "_VALUE"]);
    fixture.write(
        PRODUCTION_PATH,
        format!("// {name}\nconst DOCUMENTATION: &str = \"{name}\";\n"),
    );
    assert!(messages(&fixture).is_empty());
}
