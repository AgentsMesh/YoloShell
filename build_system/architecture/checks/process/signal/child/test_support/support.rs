use super::constants::{SOURCE_DIRECTORY_ENV, WORKSPACE_ENV};
use yolo_architecture_foundation::{SourceFile, model::Violation};
use yolo_architecture_syntax::rust_source::RustSource;

pub fn source(path: &str) -> String {
    let root = std::path::PathBuf::from(std::env::var_os(SOURCE_DIRECTORY_ENV).unwrap())
        .join(std::env::var_os(WORKSPACE_ENV).unwrap());
    std::fs::read_to_string(root.join(path)).unwrap()
}

pub fn inspect(
    path: &str,
    text: &str,
    rule: fn(&SourceFile, &RustSource, &mut Vec<Violation>),
) -> Vec<Violation> {
    let file = SourceFile {
        absolute: path.into(),
        relative: path.into(),
        text: text.into(),
    };
    let mut violations = Vec::new();
    rule(&file, &RustSource::parse(text), &mut violations);
    violations
}

pub fn assert_flow(path: &str, text: &str, expected: bool) {
    let violations = inspect(path, text, crate::process_signal_child_flow::inspect);
    assert_eq!(
        violations.is_empty(),
        expected,
        "{path}: {violations:?}\n{text}"
    );
}

pub fn reject_mutations(path: &str, changes: &[(&str, &str)]) {
    let baseline = source(path);
    assert_flow(path, &baseline, true);
    for (before, after) in changes {
        let changed = baseline.replace(before, after);
        assert_ne!(changed, baseline, "mutation did not match {path}: {before}");
        assert_flow(path, &changed, false);
    }
}
