use crate::version_identity_constants::{
    DUPLICATE_MESSAGE, LINUX_DECLARATION, LINUX_PATH, MISMATCH_MESSAGE, MISSING_MESSAGE,
    RUST_DECLARATION, SNAPSHOT_PATH, VERSION_NAME, VERSION_PATH,
};
use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_foundation::text::quoted_literals;
use yolo_architecture_syntax::starlark_lists::string_assignments;

const FIRST_LINE: usize = 1;

pub fn inspect(files: &[SourceFile], violations: &mut Vec<Violation>) {
    let version = files.iter().find(|file| file.relative == VERSION_PATH);
    let snapshot = files.iter().find(|file| file.relative == SNAPSHOT_PATH);
    let (Some(version), Some(snapshot)) = (version, snapshot) else {
        return;
    };
    let bazel = string_assignments(&version.text).get(VERSION_NAME).cloned();
    let rust = literal_after(&snapshot.text, RUST_DECLARATION);
    match (bazel, rust) {
        (Some(bazel), Some(rust)) if bazel == rust => {}
        (Some(_), Some(_)) => {
            violations.push(Violation::file(SNAPSHOT_PATH, MISMATCH_MESSAGE));
        }
        _ => {
            violations.push(Violation::file(VERSION_PATH, MISSING_MESSAGE));
        }
    }
    if let Some(linux) = files.iter().find(|file| file.relative == LINUX_PATH)
        && let Some(line) = linux
            .text
            .lines()
            .position(|line| line.trim_start().starts_with(LINUX_DECLARATION))
    {
        violations.push(Violation::line(
            LINUX_PATH,
            line + FIRST_LINE,
            DUPLICATE_MESSAGE,
        ));
    }
}

fn literal_after(text: &str, prefix: &str) -> Option<String> {
    let line = text
        .lines()
        .find(|line| line.trim_start().starts_with(prefix))?;
    quoted_literals(line)
        .into_iter()
        .next()
        .map(|literal| literal.value)
}
