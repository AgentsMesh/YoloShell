use crate::communication_json_constants::*;
use yolo_architecture_foundation::{SourceFile, model::Violation};

pub(crate) fn inspect(file: &SourceFile, violations: &mut Vec<Violation>) {
    let path = file.relative.as_str();
    if PUBLIC_JSON_OWNERS
        .iter()
        .any(|owner| path.starts_with(owner))
        || PUBLIC_DOCUMENT_FILES.contains(&path)
        || STORAGE_FILES.contains(&path)
    {
        return;
    }
    let forbidden = if path.ends_with(".rs")
        && path.contains("/src/")
        && (PRIVATE_RUST_OWNERS
            .iter()
            .any(|owner| path.starts_with(owner))
            || PRIVATE_RUST_FILES.contains(&path))
    {
        let mut text = compact(&file.text);
        // Mixed public-display/storage files retain only their exact, reviewed use.
        for (owner, fragment) in [
            (SELECTION_OWNER, SELECTION_READER),
            (HUB_SERVE_OWNER, PUBLIC_IDENTITY_DISPLAY),
        ] {
            if path == owner {
                text = text.replacen(&compact(fragment), "", 1);
            }
        }
        private_json(&text)
    } else {
        false
    };
    if forbidden {
        violations.push(Violation::file(path, PRIVATE_JSON_MESSAGE));
    }
}

fn compact(text: &str) -> String {
    text.chars()
        .filter(|character| !character.is_whitespace())
        .collect()
}

fn private_json(text: &str) -> bool {
    if text.contains("serde_jsonas") {
        return true;
    }
    text.split("serde_json::").skip(1).any(|tail| {
        if JSON_BYTE_OPERATIONS
            .iter()
            .any(|name| tail.starts_with(name))
        {
            return true;
        }
        if tail.starts_with('*') {
            return true;
        }
        if tail.starts_with('{') {
            let imported = tail.split('}').next().unwrap_or(tail);
            return imported.contains('*')
                || imported.contains("selfas")
                || JSON_BYTE_OPERATIONS
                    .iter()
                    .any(|name| imported.contains(name));
        }
        false
    })
}
