use crate::arguments::parse;
use crate::constants::{CORE_SCOPE_ARGUMENT, TOOLING_SCOPE_ARGUMENT, USAGE};
use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::path::PathBuf;
use yolo_coverage_protocol::ProductionScope;

#[path = "../test_support/argument_constants.rs"]
mod argument_constants;
use argument_constants::UNKNOWN_SCOPE_ARGUMENT;

const REPORT_PATH: &str = "coverage.dat";

#[test]
fn default_and_explicit_scopes_preserve_the_report_path() {
    for (arguments, scope) in [
        (vec![REPORT_PATH], ProductionScope::Core),
        (
            vec![CORE_SCOPE_ARGUMENT, REPORT_PATH],
            ProductionScope::Core,
        ),
        (
            vec![TOOLING_SCOPE_ARGUMENT, REPORT_PATH],
            ProductionScope::Tooling,
        ),
    ] {
        let parsed = parse(arguments.into_iter().map(OsString::from)).unwrap();
        assert_eq!(parsed.path, PathBuf::from(REPORT_PATH));
        assert_eq!(parsed.scope, scope);
    }
}

#[test]
fn missing_unknown_duplicate_or_trailing_arguments_are_rejected() {
    for arguments in [
        vec![],
        vec![""],
        vec![UNKNOWN_SCOPE_ARGUMENT, REPORT_PATH],
        vec![TOOLING_SCOPE_ARGUMENT],
        vec![TOOLING_SCOPE_ARGUMENT, ""],
        vec![TOOLING_SCOPE_ARGUMENT, CORE_SCOPE_ARGUMENT],
        vec![REPORT_PATH, TOOLING_SCOPE_ARGUMENT],
        vec![TOOLING_SCOPE_ARGUMENT, REPORT_PATH, REPORT_PATH],
    ] {
        let error = parse(arguments.into_iter().map(OsString::from))
            .err()
            .unwrap();
        assert_eq!(error, USAGE);
    }
}

#[test]
fn non_utf8_report_paths_remain_supported() {
    let path = OsString::from_vec(vec![b'c', 0xff, b'.', b'd', b'a', b't']);
    let parsed = parse([OsString::from(TOOLING_SCOPE_ARGUMENT), path.clone()]).unwrap();
    assert_eq!(parsed.path, PathBuf::from(path));
}
