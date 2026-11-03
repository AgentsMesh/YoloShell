use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_syntax::rust_source::RustSource;

const DEVELOPER_RELEASE_PATH: &str =
    "build_system/release/developer_release_system/src/developer_release.rs";
const DEVELOPER_RELEASE_FLOW_PATH: &str =
    "build_system/release/developer_release_system/src/developer_release_flow.rs";
const PRODUCTION_ENTRYPOINT: &str = "distribute";
const TEST_ONLY_ENTRYPOINTS: &[(&str, &str)] = &[
    (DEVELOPER_RELEASE_PATH, "distribute_with"),
    (DEVELOPER_RELEASE_PATH, "distribute_with_hooks"),
    (DEVELOPER_RELEASE_FLOW_PATH, "run_without_acceptance"),
];
const TEST_ONLY_POLICY: &str =
    "release distribution without acceptance must be excluded from production by cfg(test)";
const PRODUCTION_POLICY: &str =
    "the public acceptance-enforcing distribute entrypoint must compile in production";

pub(super) fn inspect(files: &[SourceFile], violations: &mut Vec<Violation>) {
    for (path, entrypoint) in TEST_ONLY_ENTRYPOINTS {
        inspect_test_only_entrypoint(files, path, entrypoint, violations);
    }
    inspect_production_entrypoint(files, violations);
}

fn inspect_test_only_entrypoint(
    files: &[SourceFile],
    path: &str,
    entrypoint: &str,
    violations: &mut Vec<Violation>,
) {
    let Some(file) = files.iter().find(|file| file.relative == path) else {
        return;
    };
    let source = RustSource::parse(&file.text);
    for function in source
        .functions()
        .filter(|function| function.matches(None, entrypoint))
    {
        if !source.is_unit_test_guarded(function) {
            violations.push(Violation::line(path, function.line, TEST_ONLY_POLICY));
        }
    }
}

fn inspect_production_entrypoint(files: &[SourceFile], violations: &mut Vec<Violation>) {
    let Some(file) = files
        .iter()
        .find(|file| file.relative == DEVELOPER_RELEASE_PATH)
    else {
        return;
    };
    let source = RustSource::parse(&file.text);
    let Some(entrypoint) = source
        .functions()
        .find(|function| function.matches(None, PRODUCTION_ENTRYPOINT))
    else {
        violations.push(Violation::file(DEVELOPER_RELEASE_PATH, PRODUCTION_POLICY));
        return;
    };
    if source.is_test_guarded(entrypoint) {
        violations.push(Violation::line(
            DEVELOPER_RELEASE_PATH,
            entrypoint.line,
            PRODUCTION_POLICY,
        ));
    }
}
