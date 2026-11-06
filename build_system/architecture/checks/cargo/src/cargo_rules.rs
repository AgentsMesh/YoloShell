use crate::policy_constants::STARLARK_EXTENSIONS;
use std::path::Path;
use yolo_architecture_foundation::{SourceFile, model::Violation};

const DIRECT_EXECUTION_MESSAGE: &str =
    "Bazel and build shell scripts must not execute Cargo directly";
const CARGO_TEST_BINARY_MESSAGE: &str =
    "Rust tests must resolve binaries from Bazel inputs, not Cargo test environments";
const CARGO_TEST_BINARY_ENVIRONMENT: &str = "CARGO_BIN_EXE_";
const SHELL_EXTENSIONS: &[&str] = &["bash", "sh", "zsh"];
const SHELL_INTERPRETERS: &[&str] = &["bash", "sh", "zsh"];

pub fn inspect(file: &SourceFile, violations: &mut Vec<Violation>) {
    if file.relative == crate::policy_constants::CARGO_MANIFEST_PATH {
        crate::cargo_manifest_rules::inspect(file, violations);
    }
    if shell_script(file) {
        report_lines(
            file,
            crate::cargo_shell::execution_lines(&file.text),
            DIRECT_EXECUTION_MESSAGE,
            violations,
        );
    } else if starlark(&file.relative) {
        report_lines(
            file,
            crate::cargo_starlark::execution_lines(&file.text),
            DIRECT_EXECUTION_MESSAGE,
            violations,
        );
    } else if rust_source(&file.relative) {
        for (index, line) in file.text.lines().enumerate() {
            if line.contains(CARGO_TEST_BINARY_ENVIRONMENT)
                && !crate::policy_exemptions::owns_cargo_environment_fixture_token(
                    &file.relative,
                    line,
                )
            {
                violations.push(Violation::line(
                    &file.relative,
                    index + 1,
                    CARGO_TEST_BINARY_MESSAGE,
                ));
            }
        }
    }
}

fn report_lines(
    file: &SourceFile,
    lines: Vec<usize>,
    message: &str,
    violations: &mut Vec<Violation>,
) {
    violations.extend(
        lines
            .into_iter()
            .map(|line| Violation::line(&file.relative, line, message)),
    );
}

fn shell_script(file: &SourceFile) -> bool {
    let extension = Path::new(&file.relative)
        .extension()
        .and_then(|value| value.to_str());
    extension.is_some_and(|value| SHELL_EXTENSIONS.contains(&value)) || shell_shebang(&file.text)
}

fn shell_shebang(text: &str) -> bool {
    let Some(line) = text.lines().next().and_then(|line| line.strip_prefix("#!")) else {
        return false;
    };
    line.split_ascii_whitespace().any(|word| {
        let interpreter = word.rsplit('/').next().unwrap_or(word);
        SHELL_INTERPRETERS.contains(&interpreter)
    })
}

fn rust_source(relative: &str) -> bool {
    Path::new(relative)
        .extension()
        .and_then(|value| value.to_str())
        == Some("rs")
}

fn starlark(relative: &str) -> bool {
    let path = Path::new(relative);
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    matches!(name, "BUILD" | "WORKSPACE")
        || path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|extension| STARLARK_EXTENSIONS.contains(&extension))
}
