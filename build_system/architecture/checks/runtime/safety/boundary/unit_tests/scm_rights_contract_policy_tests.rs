use yolo_architecture_test_support::Fixture;

const CONTRACT_SOURCE: &str = r#"_MAXIMUM_DESCRIPTOR_COUNT = FIXTURE_DESCRIPTOR_COUNT
rust = "pub(super) const MAX_RECEIVED_DESCRIPTORS: usize = %s;" % _MAXIMUM_DESCRIPTOR_COUNT
"#;
const RUST_BUILD: &str = r#"load("//build_system/bazel:scm_rights_descriptor_contract.bzl", "scm_rights_descriptor_rust_contract")
scm_rights_descriptor_rust_contract(name = "scm_rights_descriptor_contract")
compile_data = [":scm_rights_descriptor_contract"]
architecture_check_data = [":scm_rights_descriptor_contract"]
"#;
const RUST_COMPONENT_BUILD: &str = r#"_CONTRACT = ["//core:scm_rights_descriptor_contract"]
compile_data = _CONTRACT
"#;
const RUST_ADVERSARIAL: &str = "compile_data = [\"//core:scm_rights_descriptor_contract\"]\n";
const RUST_CONSUMER: &str =
    "include!(\"../../../../../src/generated/scm_rights_descriptor_contract.rs\");\n";
const MARKER: &str = "build_system/bazel/lifecycle_deadline_contract.bzl";
const CONTRACT: &str = "build_system/bazel/scm_rights_descriptor_contract.bzl";
const RUST_BUILD_PATH: &str = "core/BUILD.bazel";
const RUST_COMPONENT_BUILD_PATH: &str = "core/crates/transport/local/unix/BUILD.bazel";
const RUST_ADVERSARIAL_PATH: &str = "build_system/bazel/rust/adversarial.bzl";
const RUST_CONSUMER_PATH: &str = "core/crates/transport/local/unix/src/constants.rs";

#[test]
fn accepts_one_bazel_authority_consumed_by_rust() {
    let fixture = contract_fixture();
    assert!(violations(&fixture).is_empty());
}

#[test]
fn rejects_a_generator_that_can_diverge_between_languages() {
    let fixture = contract_fixture();
    fixture.write(
        CONTRACT,
        CONTRACT_SOURCE.replacen(
            "% _MAXIMUM_DESCRIPTOR_COUNT",
            "% DIVERGENT_DESCRIPTOR_COUNT",
            1,
        ),
    );
    assert_eq!(violations(&fixture), vec![CONTRACT.to_owned()]);
}

#[test]
fn rejects_missing_rust_build_wiring() {
    let fixture = contract_fixture();
    fixture.write(
        RUST_BUILD_PATH,
        "scm_rights_descriptor_rust_contract = contract\n",
    );
    assert_eq!(
        violations(&fixture),
        vec![RUST_BUILD_PATH.to_owned()]
    );
}

#[test]
fn rejects_missing_component_contract_wiring() {
    let fixture = contract_fixture();
    fixture.write(
        RUST_COMPONENT_BUILD_PATH,
        "rust_library(name = \"local\")\n",
    );
    assert_eq!(
        violations(&fixture),
        vec![RUST_COMPONENT_BUILD_PATH.to_owned()]
    );
}

fn contract_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.write(MARKER, "");
    fixture.write(CONTRACT, CONTRACT_SOURCE);
    fixture.write(RUST_BUILD_PATH, RUST_BUILD);
    fixture.write(RUST_COMPONENT_BUILD_PATH, RUST_COMPONENT_BUILD);
    fixture.write(RUST_ADVERSARIAL_PATH, RUST_ADVERSARIAL);
    fixture.write(RUST_CONSUMER_PATH, RUST_CONSUMER);
    fixture
}

fn violations(fixture: &Fixture) -> Vec<String> {
    let files = yolo_architecture_test_support::load(fixture.root()).unwrap();
    let mut violations = Vec::new();
    crate::scm_rights_contract_rules::inspect(&files, &mut violations);
    violations
        .iter()
        .map(|violation| violation.path.clone())
        .collect()
}
