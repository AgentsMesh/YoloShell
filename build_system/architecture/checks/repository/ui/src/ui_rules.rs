use crate::constants::{RUST_UI_TERMS, joined};
use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_foundation::text::{
    contains_ascii, contains_identifier, first_call_line, first_line_ascii,
};

const RETIRED_UI_TERMS: &[&[&str]] = &[&["wg", "pu"], &["e", "gui"]];
pub fn inspect(file: &SourceFile, violations: &mut Vec<Violation>) {
    inspect_rust_boundary(file, violations);
    inspect_retired_ui(file, violations);
    inspect_test_activation(file, violations);
}

fn inspect_retired_ui(file: &SourceFile, violations: &mut Vec<Violation>) {
    for parts in RETIRED_UI_TERMS {
        let term = joined(parts);
        if let Some(line) = first_line_ascii(&file.text, &term) {
            violations.push(Violation::line(
                &file.relative,
                line,
                format!("retired cross-platform UI technology {term}"),
            ));
        } else if contains_ascii(&file.relative, &term) {
            violations.push(Violation::file(
                &file.relative,
                format!("retired cross-platform UI technology {term}"),
            ));
        }
    }
}

fn inspect_rust_boundary(file: &SourceFile, violations: &mut Vec<Violation>) {
    let rust_owned = file.relative == "Cargo.toml"
        || (file.relative.starts_with("core/") && file.relative.ends_with(".rs"));
    if !rust_owned {
        return;
    }
    for parts in RUST_UI_TERMS {
        let term = joined(parts);
        if crate::foundation_directory::permits(file, &term) {
            continue;
        }
        if let Some(line) = file
            .text
            .lines()
            .position(|line| contains_identifier(line, &term))
            .map(|index| index + 1)
        {
            violations.push(Violation::line(
                &file.relative,
                line,
                format!("Rust runtime depends on UI technology {term}"),
            ));
        }
    }
}

fn inspect_test_activation(file: &SourceFile, violations: &mut Vec<Violation>) {
    if !test_source(&file.relative) {
        return;
    }
    let actions = [
        ["acti", "vate"].concat(),
        ["bring", "ToFront"].concat(),
        ["makeKey", "AndOrderFront"].concat(),
        ["show", "Window"].concat(),
        ["order", "Front"].concat(),
        ["orderFront", "Regardless"].concat(),
        ["make", "Key"].concat(),
        ["make", "Main"].concat(),
        ["become", "Key"].concat(),
        ["become", "Main"].concat(),
        ["demini", "aturize"].concat(),
        ["un", "hide"].concat(),
        ["launch", "Application"].concat(),
        ["open", "Application"].concat(),
    ];
    for action in actions {
        if let Some(line) = first_call_line(&file.text, &action) {
            violations.push(Violation::line(
                &file.relative,
                line,
                "default tests must not activate or focus a native window",
            ));
        }
    }
}

fn test_source(path: &str) -> bool {
    path.starts_with("integration_tests/")
}
