use std::path::Path;
use yolo_paths_core::validation::{validate_absolute, validate_home};

const NORMALIZED_HOME: &str = "/Users/example/.yolo";
const ROOT_PATH: &str = "/";
const INVALID_ABSOLUTE_PATHS: &[&str] = &[
    "",
    "relative/yolo",
    "/tmp/../tmp/yolo",
    "/tmp/./yolo",
    "/tmp//yolo",
    "/tmp/yolo/",
];

#[test]
fn normalized_absolute_home_is_accepted() {
    validate_home(Path::new(NORMALIZED_HOME)).unwrap();
}

#[test]
fn unsafe_home_shapes_are_rejected() {
    assert!(validate_home(Path::new(ROOT_PATH)).is_err());
    for path in INVALID_ABSOLUTE_PATHS {
        assert!(validate_home(Path::new(path)).is_err(), "accepted {path:?}");
    }
}

#[test]
fn absolute_validation_accepts_root_without_home_semantics() {
    validate_absolute(Path::new(ROOT_PATH)).unwrap();
}

#[test]
fn absolute_validation_rejects_relative_and_non_normalized_paths() {
    for path in INVALID_ABSOLUTE_PATHS {
        assert!(
            validate_absolute(Path::new(path)).is_err(),
            "accepted {path:?}"
        );
    }
}
