use crate::{constants::TEST_SOURCE, fixture::read};
use yolo_coverage_source_contract::{SourceContract, verify_manifest};

const TOOLCHAIN: &str = "external/rules_rust++rust+rust_linux_aarch64__aarch64-unknown-linux-gnu__stable_tools/lib/rustlib/aarch64-unknown-linux-gnu";
const LINUX_AMD64: &str = "external/rules_rust++rust+rust_linux_x86_64__x86_64-unknown-linux-gnu__stable_tools/lib/rustlib/x86_64-unknown-linux-gnu";
const MACOS_ARM64: &str = "external/rules_rust++rust+rust_macos_aarch64__aarch64-apple-darwin__stable_tools/lib/rustlib/aarch64-apple-darwin";
const MACOS_AMD64: &str = "external/rules_rust++rust+rust_macos_x86_64__x86_64-apple-darwin__stable_tools/lib/rustlib/x86_64-apple-darwin";
const ARTIFACTS: &[&str] = &[
    "bin/llvm-cov",
    "bin/llvm-profdata",
    "lib/libLLVM-21-rust-1.93.1-stable.so",
    "lib/libLLVM.so.21.1-rust-1.93.1-stable",
];

#[test]
fn pinned_linux_tools_are_ancillary_to_executed_repository_tests() {
    for toolchain in [TOOLCHAIN, LINUX_AMD64] {
        let mut manifest = TEST_SOURCE.to_owned();
        for artifact in ARTIFACTS {
            manifest.push_str(&format!("\n{toolchain}/{artifact}"));
        }
        for value in [manifest.clone(), manifest.replace('\n', "\r\n")] {
            assert_eq!(
                verify_manifest(&value, read).unwrap(),
                SourceContract::TestsOnly
            );
        }
    }
}

#[test]
fn macos_distributions_admit_tools_without_inventing_linux_library_inputs() {
    for toolchain in [MACOS_ARM64, MACOS_AMD64] {
        for artifact in ARTIFACTS {
            let result = verify_manifest(&format!("{TEST_SOURCE}\n{toolchain}/{artifact}"), read);
            if artifact.starts_with("bin/") {
                assert_eq!(result.unwrap(), SourceContract::TestsOnly);
            } else {
                assert!(result.is_err());
            }
        }
    }
}

#[test]
fn tool_layout_does_not_admit_unreviewed_sources_or_libraries() {
    for artifact in [
        "bin/rustc",
        "bin/llvm-cov.rs",
        "bin/llvm-cov/extra",
        "lib/libunknown.so",
        "lib/libLLVM.so",
        "lib/libLLVM-21-rust-1.93.2-stable.so",
        "lib/libLLVM.so.21.1-rust-1.93.2-stable",
        "lib/libLLVM.so.21.1-rust-1.93.1-stable.rs",
        "lib/libLLVM.so.21.1-rust-1.93.1-stable/extra",
        "lib/src/runtime.rs",
        "bin/../bin/llvm-cov",
        "bin/./llvm-cov",
        "bin//llvm-cov",
        "bin/llvm-cov/",
        "bin/llvm-cov\0",
    ] {
        let manifest = format!("{TEST_SOURCE}\n{TOOLCHAIN}/{artifact}");
        assert!(verify_manifest(&manifest, read).is_err(), "{artifact}");
    }
}

#[test]
fn pinned_artifacts_require_the_exact_distribution_and_a_real_test_witness() {
    let artifact = format!("{TOOLCHAIN}/bin/llvm-cov");
    for changed in [
        artifact.replacen("external/", "external/unknown/", 1),
        artifact.replace("__stable_tools", "__unknown_tools"),
        artifact.replace("aarch64-unknown-linux-gnu", "unknown-target"),
        artifact.replace("rust_linux_aarch64__", "rust_linux_x86_64__"),
        artifact.replace("rustlib/aarch64", "rustlib/x86_64"),
        format!("{artifact}/extra"),
        format!("/{artifact}"),
    ] {
        let manifest = format!("{TEST_SOURCE}\n{changed}");
        assert!(verify_manifest(&manifest, read).is_err(), "{changed}");
    }
    assert!(verify_manifest(&artifact, read).is_err());
    let production = "core/crates/fixture/src/lib.rs";
    assert!(verify_manifest(&format!("{production}\n{artifact}"), read).is_err());
}
