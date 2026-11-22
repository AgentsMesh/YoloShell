use yolo_architecture_foundation::{SourceFile, model::Violation};

use crate::communication_constants::*;

pub(crate) fn inspect(file: &SourceFile, violations: &mut Vec<Violation>) {
    crate::communication_json::inspect(file, violations);
    if !file.relative.starts_with(CORE)
        || !(file.relative.ends_with(".rs") || file.relative.ends_with("BUILD.bazel"))
    {
        return;
    }
    if file.text.contains(RETIRED_VALUE_CRATE) {
        violations.push(Violation::file(&file.relative, RETIRED_VALUE_MESSAGE));
    }
    if !file.relative.starts_with(SNAPSHOT_STORAGE)
        && SNAPSHOT_CODECS
            .iter()
            .any(|codec| file.text.contains(codec))
    {
        violations.push(Violation::file(&file.relative, CODEC_SCOPE_MESSAGE));
    }
}
