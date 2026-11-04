use crate::constants::MAX_HANDWRITTEN_LINES;
use crate::source_policy::is_handwritten;
use yolo_architecture_foundation::{
    SourceFile,
    model::Violation,
    text::{first_line_ascii, joined, line_count},
};

pub fn inspect(file: &SourceFile, violations: &mut Vec<Violation>) {
    if !is_handwritten(&file.absolute) {
        return;
    }
    let count = line_count(&file.text);
    if count > MAX_HANDWRITTEN_LINES {
        violations.push(Violation::file(
            &file.relative,
            format!(
                "{count} lines exceeds the handwritten source limit of {MAX_HANDWRITTEN_LINES}"
            ),
        ));
    }
    let portable_writer = joined(&[".take_", "writer()"]);
    if file
        .relative
        .starts_with("core/crates/runtime/engine/worker/src/")
        && let Some(line) = first_line_ascii(&file.text, &portable_writer)
    {
        violations.push(Violation::line(
            &file.relative,
            line,
            "Worker must use Yolo's bounded PTY input writer",
        ));
    }
}
