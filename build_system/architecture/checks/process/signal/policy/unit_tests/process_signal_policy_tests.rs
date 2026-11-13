use super::fixture::Fixture;
use super::process_signal_test_support::{messages, write_contract};

const LIVE_CONTROL_PATH: &str = "core/crates/runtime/engine/process/src/live/live_identity_control.rs";
const FUTURE_PATH: &str = "core/crates/runtime/engine/process/src/future_cleanup.rs";
const CFG_POLICY: &str = "test signal bridge requires cfg(test)";
const OWNER_POLICY: &str = "test signal bridge moved outside its exact owner";
const VALID_FEATURE: &str = "adversarial-test-hooks";
const INVALID_FEATURE: &str = "unrelated-hooks";

#[test]
fn accepts_the_current_authority_and_bridge_graph() {
    let fixture = Fixture::new();
    write_contract(&fixture);
    let violations = messages(&fixture);
    assert!(violations.is_empty(), "{violations:#?}");
}

#[test]
fn bridge_requires_positive_test_or_exact_adversarial_cfg() {
    let missing = Fixture::new();
    write_contract(&missing);
    missing.write(LIVE_CONTROL_PATH, live_control(None));
    let result = messages(&missing);
    assert_eq!(matches(&result, CFG_POLICY), 2);

    let wrong = Fixture::new();
    write_contract(&wrong);
    wrong.write(LIVE_CONTROL_PATH, live_control(Some(INVALID_FEATURE)));
    assert_eq!(matches(&messages(&wrong), CFG_POLICY), 2);

    let valid = Fixture::new();
    write_contract(&valid);
    valid.write(LIVE_CONTROL_PATH, live_control(Some(VALID_FEATURE)));
    assert!(messages(&valid).is_empty());
}

#[test]
fn bridge_cfg_must_imply_a_test_only_build() {
    let production_alternative = Fixture::new();
    write_contract(&production_alternative);
    production_alternative.write(
        LIVE_CONTROL_PATH,
        live_control_with_guard("#[cfg(any(test, unix))]\n"),
    );
    assert_eq!(matches(&messages(&production_alternative), CFG_POLICY), 2);

    let test_conjunction = Fixture::new();
    write_contract(&test_conjunction);
    test_conjunction.write(
        LIVE_CONTROL_PATH,
        live_control_with_guard("#[cfg(all(unix, test))]\n"),
    );
    assert!(messages(&test_conjunction).is_empty());

    let feature_without_test = Fixture::new();
    write_contract(&feature_without_test);
    feature_without_test.write(
        LIVE_CONTROL_PATH,
        live_control_with_guard("#[cfg(all(feature = \"adversarial-test-hooks\", not(test)))]\n"),
    );
    assert!(messages(&feature_without_test).is_empty());

    let negated = Fixture::new();
    write_contract(&negated);
    negated.write(
        LIVE_CONTROL_PATH,
        live_control_with_guard("#[cfg(not(test))]\n"),
    );
    assert_eq!(matches(&messages(&negated), CFG_POLICY), 2);
}

#[test]
fn moved_bridge_fails_even_when_its_name_and_cfg_are_preserved() {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(FUTURE_PATH, "#[cfg(test)]\nfn signal_for_test() {}\n");
    let result = messages(&fixture);
    assert_eq!(matches(&result, OWNER_POLICY), 1);
}

#[test]
fn comments_and_strings_cannot_spoof_code_sites() {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(
        FUTURE_PATH,
        r####"
// identity.signal_authorized(&authority, signal);
// unsafe { libc::kill(pid, libc::SIGKILL) };
/* outer comment /* nested comment */ libc::kill(pid, libc::SIGKILL); */
const NOTE: &str = "ProcessSignalAuthority::new(); libc::kill(1, 9)";
const RAW_NOTE: &str = r###"libc::kill(pid, "quoted"); ProcessSignalAuthority::new()"###;
fn unrelated_signal() {}
"####,
    );
    assert!(messages(&fixture).is_empty());
}

fn live_control(feature: Option<&str>) -> String {
    let guard = feature.map_or(String::new(), |feature| {
        format!("#[cfg(feature = \"{feature}\")]\n")
    });
    live_control_with_guard(&guard)
}

fn live_control_with_guard(guard: &str) -> String {
    format!(
        r#"
impl LiveProcessIdentity {{
    fn signal_authorized(&self) {{}}
    {guard}fn signal_for_test(&self) {{
        self.signal_authorized();
        ProcessSignalAuthority::for_test();
    }}
    {guard}fn signal(&self) {{ self.signal_for_test(); }}
}}
"#
    )
}

fn matches(messages: &[String], expected: &str) -> usize {
    messages
        .iter()
        .filter(|message| message.contains(expected))
        .count()
}
