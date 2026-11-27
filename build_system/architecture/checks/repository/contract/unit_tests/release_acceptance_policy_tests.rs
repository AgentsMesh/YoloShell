use yolo_architecture_test_support::Fixture;

const DEVELOPER_RELEASE_PATH: &str =
    "build_system/release/developer_release_system/src/developer_release.rs";
const DEVELOPER_RELEASE_FLOW_PATH: &str =
    "build_system/release/developer_release_system/src/developer_release_flow.rs";
const TEST_ONLY_POLICY: &str = "release distribution without acceptance";
const PRODUCTION_POLICY: &str = "acceptance-enforcing distribute entrypoint";

#[test]
fn no_acceptance_entrypoints_require_production_excluding_test_guards() {
    let fixture = Fixture::new();
    fixture.write(
        DEVELOPER_RELEASE_PATH,
        "pub fn distribute() {}\npub(crate) fn distribute_with() {}\n#[cfg(feature = \"adversarial-test-hooks\")]\npub(crate) fn distribute_with_hooks() {}\n",
    );
    fixture.write(
        DEVELOPER_RELEASE_FLOW_PATH,
        "#[cfg(any(test, feature = \"adversarial-test-hooks\"))]\npub(crate) fn run_without_acceptance() {}\n",
    );

    let result = messages(&fixture);
    assert_eq!(matching(&result, TEST_ONLY_POLICY), 3, "{result:?}");
}

#[test]
fn exact_test_guards_isolate_every_no_acceptance_entrypoint() {
    let fixture = Fixture::new();
    fixture.write(
        DEVELOPER_RELEASE_PATH,
        "pub fn distribute() {}\n#[cfg(test)]\npub(crate) fn distribute_with() {}\n#[cfg(all(test, feature = \"yolo-unit-tests\"))]\npub(crate) fn distribute_with_hooks() {}\n",
    );
    fixture.write(
        DEVELOPER_RELEASE_FLOW_PATH,
        "#[cfg(test)]\npub(crate) fn run_without_acceptance() {}\n",
    );

    assert!(messages(&fixture).is_empty());
}

#[test]
fn public_distribution_entrypoint_must_remain_in_production() {
    let fixture = Fixture::new();
    fixture.write(
        DEVELOPER_RELEASE_PATH,
        "#[cfg(test)]\npub fn distribute() {}\n#[cfg(test)]\npub(crate) fn distribute_with() {}\n#[cfg(test)]\npub(crate) fn distribute_with_hooks() {}\n",
    );

    let result = messages(&fixture);
    assert_eq!(matching(&result, PRODUCTION_POLICY), 1, "{result:?}");
}

fn messages(fixture: &Fixture) -> Vec<String> {
    crate::inspect(fixture.root())
        .unwrap()
        .into_iter()
        .map(|violation| violation.to_string())
        .collect()
}

fn matching(messages: &[String], expected: &str) -> usize {
    messages
        .iter()
        .filter(|message| message.contains(expected))
        .count()
}
