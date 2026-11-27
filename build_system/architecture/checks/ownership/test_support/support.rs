use yolo_architecture_foundation::{SourceFile, model::Violation};

pub fn source(path: &str, text: impl Into<String>) -> SourceFile {
    SourceFile {
        absolute: path.into(),
        relative: path.to_owned(),
        text: text.into(),
    }
}

pub fn inspect(files: Vec<SourceFile>) -> Vec<Violation> {
    let mut violations = Vec::new();
    crate::inspect(&files, &mut violations);
    violations
}

pub fn messages(violations: &[Violation]) -> String {
    violations
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n")
}
