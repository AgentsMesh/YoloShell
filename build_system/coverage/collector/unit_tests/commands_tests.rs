use crate::commands::{add_instrumented_objects, function_report_command};
use crate::constants::{OBJECT_ARGUMENT, PATH_EQUIVALENCE_PREFIX, SOURCES_ARGUMENT};
use crate::environment::Context;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::Command;

const LLVM_COV_PROGRAM: &str = "llvm-cov";
const PRIMARY_OBJECT: &str = "core/process-test";
const GUARD_OBJECT: &str = "core/socket-root-guard";

#[test]
fn every_instrumented_object_is_passed_to_llvm_cov() {
    let objects = [PathBuf::from(PRIMARY_OBJECT), PathBuf::from(GUARD_OBJECT)];
    let mut command = Command::new(LLVM_COV_PROGRAM);
    add_instrumented_objects(&mut command, &objects).unwrap();
    assert_eq!(
        command.get_args().collect::<Vec<_>>(),
        [
            OsStr::new(PRIMARY_OBJECT),
            OsStr::new(OBJECT_ARGUMENT),
            OsStr::new(GUARD_OBJECT),
        ],
    );
}

#[test]
fn an_empty_object_set_fails_closed() {
    let mut command = Command::new(LLVM_COV_PROGRAM);
    assert!(add_instrumented_objects(&mut command, &[]).is_err());
    assert_eq!(command.get_args().count(), 0);
}

#[test]
fn function_reports_resolve_core_and_tooling_from_the_same_execution_root() {
    let root = PathBuf::from("/workspace");
    let context = Context {
        execution_root: root.clone(),
        llvm_cov: LLVM_COV_PROGRAM.into(),
        llvm_profdata: "llvm-profdata".into(),
        profile_data: "coverage.profdata".into(),
        raw_profiles: Vec::new(),
        report: "coverage.dat".into(),
        instrumented_objects: vec![PRIMARY_OBJECT.into()],
    };
    let sources = [
        "core/crates/wire/src/lib.rs".to_owned(),
        "build_system/tool/src/lib.rs".to_owned(),
    ];
    let command = function_report_command(&context, &sources).unwrap();
    let arguments = command.get_args().collect::<Vec<_>>();
    let equivalence = format!("{PATH_EQUIVALENCE_PREFIX}{}", root.display());
    assert!(arguments.contains(&OsStr::new(&equivalence)));
    let source_start = arguments
        .iter()
        .position(|value| *value == SOURCES_ARGUMENT)
        .unwrap()
        + 1;
    let expected = sources
        .iter()
        .map(|source| root.join(source))
        .collect::<Vec<_>>();
    assert_eq!(
        arguments[source_start..],
        expected
            .iter()
            .map(|path| path.as_os_str())
            .collect::<Vec<_>>()
    );
    assert!(function_report_command(&context, &[]).is_err());
}
