use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;

const REPOSITORY_MARKER_PATH: &str = "build_system/bazel/lifecycle_deadline_contract.bzl";
const CONTRACT_PATH: &str = "build_system/bazel/scm_rights_descriptor_contract.bzl";
const RUST_BUILD_PATH: &str = "core/BUILD.bazel";
const RUST_COMPONENT_BUILD_PATH: &str = "core/crates/transport/local/unix/BUILD.bazel";
const RUST_ADVERSARIAL_PATH: &str = "build_system/bazel/rust/adversarial.bzl";
const RUST_CONSUMER_PATH: &str = "core/crates/transport/local/unix/src/constants.rs";
const AUTHORITY_NAME: &str = "_MAXIMUM_DESCRIPTOR_COUNT";
const AUTHORITY_DECLARATION: &str = "_MAXIMUM_DESCRIPTOR_COUNT = ";
const EXPECTED_AUTHORITY_REFERENCES: usize = 2;
const EXPECTED_RUST_BUILD_CONSUMERS: usize = 2;
const EXPECTED_RUST_COMPONENT_REFERENCES: usize = 1;
const EXPECTED_RUST_ADVERSARIAL_CONSUMERS: usize = 1;
const RUST_ADVERSARIAL_CONTRACT_LABEL: &str = "\"//core:scm_rights_descriptor_contract\"";
const CONTRACT_MESSAGE: &str =
    "SCM_RIGHTS descriptor ceiling must remain a Bazel-generated Rust contract";

const REQUIREMENTS: &[(&str, &[&str])] = &[
    (
        CONTRACT_PATH,
        &[
            AUTHORITY_DECLARATION,
            "pub(super) const MAX_RECEIVED_DESCRIPTORS: usize = %s;",
        ],
    ),
    (
        RUST_BUILD_PATH,
        &[
            "load(\"//build_system/bazel:scm_rights_descriptor_contract.bzl\", \"scm_rights_descriptor_rust_contract\")",
            crate::constants::SCM_RIGHTS_RUST_CONTRACT_RULE,
            "scm_rights_descriptor_rust_contract(name = \"scm_rights_descriptor_contract\")",
            "\":scm_rights_descriptor_contract\"",
        ],
    ),
    (
        RUST_COMPONENT_BUILD_PATH,
        &[
            "_CONTRACT = [\"//core:scm_rights_descriptor_contract\"]",
            "compile_data = _CONTRACT",
        ],
    ),
    (RUST_ADVERSARIAL_PATH, &[RUST_ADVERSARIAL_CONTRACT_LABEL]),
    (
        RUST_CONSUMER_PATH,
        &["include!(\"../../../../../src/generated/scm_rights_descriptor_contract.rs\");"],
    ),
];

pub(super) fn inspect(files: &[SourceFile], violations: &mut Vec<Violation>) {
    if source(files, REPOSITORY_MARKER_PATH).is_none() {
        return;
    }
    for (path, required) in REQUIREMENTS {
        let Some(file) = source(files, path) else {
            violations.push(Violation::file(*path, CONTRACT_MESSAGE));
            continue;
        };
        let expected_occurrences = match *path {
            CONTRACT_PATH => Some((AUTHORITY_NAME, EXPECTED_AUTHORITY_REFERENCES)),
            RUST_BUILD_PATH => Some((
                "\":scm_rights_descriptor_contract\"",
                EXPECTED_RUST_BUILD_CONSUMERS,
            )),
            RUST_COMPONENT_BUILD_PATH => Some((
                "\"//core:scm_rights_descriptor_contract\"",
                EXPECTED_RUST_COMPONENT_REFERENCES,
            )),
            RUST_ADVERSARIAL_PATH => Some((
                RUST_ADVERSARIAL_CONTRACT_LABEL,
                EXPECTED_RUST_ADVERSARIAL_CONSUMERS,
            )),
            _ => None,
        };
        let occurrences_match = expected_occurrences
            .is_none_or(|(needle, count)| file.text.matches(needle).count() == count);
        if !occurrences_match || required.iter().any(|value| !file.text.contains(value)) {
            violations.push(Violation::file(&file.relative, CONTRACT_MESSAGE));
        }
    }
}

fn source<'a>(files: &'a [SourceFile], path: &str) -> Option<&'a SourceFile> {
    files.iter().find(|file| file.relative == path)
}
