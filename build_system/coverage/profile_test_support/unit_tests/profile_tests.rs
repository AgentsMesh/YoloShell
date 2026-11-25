use super::{ProfileName, discard_pattern, hostname};
use std::os::unix::fs::symlink;

#[test]
fn only_waited_sigkill_statuses_enter_profile_discard() {
    use std::os::unix::process::ExitStatusExt;
    for raw in [0, libc::SIGTERM] {
        assert_eq!(
            super::discard_reaped_kill_profile(0, std::process::ExitStatus::from_raw(raw)).unwrap(),
            0
        );
    }
    assert!(
        super::discard_reaped_kill_profile(
            std::process::id(),
            std::process::ExitStatus::from_raw(libc::SIGKILL),
        )
        .is_err()
    );
}

const HOST: &str = "fixture-host.example";
const PID: u32 = 1234;
const PROFILE_PATTERN: &str = "%h-%p-%m.profraw";
const EXPECTED: &str = "fixture-host.example-1234-987654_0.profraw";
const SECOND_MODULE: &str = "fixture-host.example-1234-321_1.profraw";
const OTHER_PID: &str = "fixture-host.example-12345-987654_0.profraw";
const OTHER_HOST: &str = "other-host.example-1234-987654_0.profraw";
const INVALID_MODULE: &str = "fixture-host.example-1234-invalid.profraw";
const TEST_SOURCE: &str =
    "build_system/coverage/profile_test_support/unit_tests/profile_tests.rs";

#[test]
fn hostname_pid_and_each_module_are_matched_exactly() {
    let matcher = ProfileName::parse(PROFILE_PATTERN, PID, HOST).unwrap();
    assert!(matcher.matches(EXPECTED));
    assert!(matcher.matches(SECOND_MODULE));
    for name in [
        OTHER_PID,
        OTHER_HOST,
        INVALID_MODULE,
        "fixture-host.example-1234-_0.profraw",
        "fixture-host.example-1234-1_.profraw",
        "fixture-host.example-1234-1_0_0.profraw",
    ] {
        assert!(!matcher.matches(name), "{name}");
    }
}

#[test]
fn only_matching_regular_profiles_are_removed_and_cleanup_is_idempotent() {
    let directory = tempfile::tempdir().unwrap();
    let retained = [OTHER_PID, OTHER_HOST, INVALID_MODULE];
    for name in [EXPECTED, SECOND_MODULE].into_iter().chain(retained) {
        std::fs::write(directory.path().join(name), []).unwrap();
    }
    let linked = "fixture-host.example-1234-111_0.profraw";
    symlink(OTHER_PID, directory.path().join(linked)).unwrap();
    let nested = "fixture-host.example-1234-222_0.profraw";
    std::fs::create_dir(directory.path().join(nested)).unwrap();
    let pattern = directory.path().join(PROFILE_PATTERN);
    assert_eq!(discard_pattern(&pattern, PID, HOST).unwrap(), 2);
    assert_eq!(discard_pattern(&pattern, PID, HOST).unwrap(), 0);
    for name in retained.into_iter().chain([linked, nested]) {
        assert!(directory.path().join(name).exists(), "{name}");
    }
}

#[test]
fn exact_pid_paths_do_not_match_a_pid_prefix() {
    let matcher = ProfileName::parse("profile-%p.profraw", PID, HOST).unwrap();
    assert!(matcher.matches("profile-1234.profraw"));
    assert!(!matcher.matches("profile-12345.profraw"));
}

#[test]
fn ambiguous_or_unknown_templates_are_rejected() {
    for pattern in [
        "default.profraw",
        "%p%p.profraw",
        "%h-%p%m.profraw",
        "%h-%p1%m.profraw",
        "%h-%p01%m.profraw",
        "%h-%p-%m-%m.profraw",
        "%h-%p-%x.profraw",
        "%x-%p.profraw",
    ] {
        assert!(ProfileName::parse(pattern, PID, HOST).is_err(), "{pattern}");
    }
    assert!(ProfileName::parse(PROFILE_PATTERN, 0, HOST).is_err());
}

#[test]
fn absent_directories_are_not_a_cleanup_failure() {
    let directory = tempfile::tempdir().unwrap();
    let pattern = directory.path().join("not-created").join(PROFILE_PATTERN);
    assert_eq!(discard_pattern(&pattern, PID, HOST).unwrap(), 0);
}

#[test]
fn a_regular_file_parent_is_not_treated_as_an_absent_directory() {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("file-parent");
    std::fs::write(&file, []).unwrap();
    assert!(discard_pattern(&file.join(PROFILE_PATTERN), PID, HOST).is_err());
}

#[test]
fn the_os_hostname_is_available_for_the_llvm_host_placeholder() {
    assert!(!hostname().unwrap().is_empty());
}

#[test]
fn current_and_invalid_process_ids_cannot_discard_the_running_test_profile() {
    assert!(super::discard_aborted_process_profile(std::process::id()).is_err());
    assert!(super::discard_aborted_process_profile(u32::default()).is_err());
}

#[test]
fn fixture_sources_are_excluded_from_production_coverage() {
    assert!(
        yolo_coverage_protocol::production_key(TEST_SOURCE)
            .unwrap()
            .is_none()
    );
}
