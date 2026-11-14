use crate::environment::{select_additional_objects, select_test_binary};
use std::path::PathBuf;

const WRAPPER_BINARY: &str = "core/unit_tests_isolated_binary";
const OUTER_BINARY: &str = "core/unit_tests";
const PROFILE_DIRECTORY: &str = "profiles";
const FIRST_RAW: &str = "a.profraw";
const LAST_RAW: &str = "z.profraw";
const IGNORED_ENTRY: &str = "notes.txt";
const WRONG_CASE: &str = "ignored.PROFRAW";
const ENTRY_ERROR: &str = "directory iterator failed after partial progress";

#[test]
fn wrapper_binary_overrides_bazel_test_binary() {
    assert_eq!(
        select_test_binary(Some(WRAPPER_BINARY), Some(OUTER_BINARY)).unwrap(),
        PathBuf::from(WRAPPER_BINARY),
    );
}

#[test]
fn ordinary_rust_test_uses_bazel_test_binary() {
    assert_eq!(
        select_test_binary(None, Some(OUTER_BINARY)).unwrap(),
        PathBuf::from(OUTER_BINARY),
    );
}

#[test]
fn binary_selection_fails_closed() {
    assert!(select_test_binary(None, None).is_err());
    for path in ["", "/tmp/test", "../test", "core/./test"] {
        assert!(select_test_binary(Some(path), Some(OUTER_BINARY)).is_err());
    }
}

#[test]
fn additional_objects_are_json_encoded_normalized_workspace_paths() {
    assert_eq!(
        select_additional_objects(None).unwrap(),
        Vec::<PathBuf>::new()
    );
    assert_eq!(
        select_additional_objects(Some(r#"["core/guard","core/helper"]"#)).unwrap(),
        [PathBuf::from("core/guard"), PathBuf::from("core/helper")],
    );
}

#[test]
fn additional_object_selection_fails_closed() {
    for value in [
        "not-json",
        r#"{"object":"core/guard"}"#,
        r#"[""]"#,
        r#"["/tmp/guard"]"#,
        r#"["../guard"]"#,
        r#"["core/./guard"]"#,
    ] {
        assert!(select_additional_objects(Some(value)).is_err(), "{value}");
    }
}

#[test]
fn complete_profile_inventory_keeps_exact_extension_selection_and_stable_order() {
    let directory = std::path::Path::new(PROFILE_DIRECTORY);
    let entries = [LAST_RAW, IGNORED_ENTRY, FIRST_RAW, WRONG_CASE]
        .into_iter()
        .map(|name| Ok(directory.join(name)));
    assert_eq!(
        crate::raw_profiles::select(directory, entries).unwrap(),
        [directory.join(FIRST_RAW), directory.join(LAST_RAW)]
    );
    assert!(crate::raw_profiles::select(directory, [Ok(directory.join(IGNORED_ENTRY))]).is_err());
}

#[test]
fn entry_errors_before_or_after_accepted_profiles_never_return_partial_inventory() {
    let directory = std::path::Path::new(PROFILE_DIRECTORY);
    for prefix in [vec![], vec![FIRST_RAW], vec![IGNORED_ENTRY, FIRST_RAW]] {
        let entries = prefix
            .into_iter()
            .map(|name| Ok(directory.join(name)))
            .chain(std::iter::once(Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                ENTRY_ERROR,
            ))))
            .chain(std::iter::once(Ok(directory.join(LAST_RAW))));
        let error = crate::raw_profiles::select(directory, entries).unwrap_err();
        assert!(error.contains(PROFILE_DIRECTORY));
        assert!(error.contains(ENTRY_ERROR));
    }
}
