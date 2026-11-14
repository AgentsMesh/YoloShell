//! Provider onboarding policy must live in the provider's own package under
//! `core/crates/agent/providers/<provider>`.
//!
//! Shared crates may reference provider packages, but they must not carry a
//! provider's identity strings, and the retired aggregate policy tables must
//! not come back.
use crate::provider_policy_constants::{
    COMPOSITION_FILES, FORBIDDEN_FILES, FORBIDDEN_PREFIXES, LITERAL_MESSAGE, POLICY_LITERALS,
    PROVIDERS, RETIRED_PATH_MESSAGE, RETIRED_PROVIDER_MESSAGE, WIRE_CONSTANT_FILES,
};
use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_foundation::text::quoted_literals;

pub fn inspect(files: &[SourceFile], violations: &mut Vec<Violation>) {
    for file in files {
        let path = file.relative.as_str();
        if FORBIDDEN_FILES.contains(&path)
            || FORBIDDEN_PREFIXES.iter().any(|prefix| path.starts_with(prefix))
        {
            violations.push(Violation::file(path, RETIRED_PATH_MESSAGE));
            continue;
        }
        if retired_provider_path(path) {
            violations.push(Violation::file(path, RETIRED_PROVIDER_MESSAGE));
            continue;
        }
        let composition = COMPOSITION_FILES.contains(&path);
        let production = production_rust(path)
            && !provider_owned(path)
            && !WIRE_CONSTANT_FILES.contains(&path);
        if !composition && !production {
            continue;
        }
        for literal in quoted_literals(&file.text) {
            if POLICY_LITERALS.contains(&literal.value.as_str()) {
                violations.push(Violation::line(path, literal.line, LITERAL_MESSAGE));
            }
        }
    }
}

fn production_rust(path: &str) -> bool {
    path.starts_with("core/crates/")
        && path.ends_with(".rs")
        && !test_path(path)
}

fn test_path(path: &str) -> bool {
    path.contains("/unit_tests/")
        || path.contains("/integration_tests/")
        || path.contains("/test_support/")
}

fn retired_provider_path(path: &str) -> bool {
    PROVIDERS.iter().any(|provider| {
        path.starts_with(&format!("core/crates/agent/system/{provider}/"))
            || path.starts_with(&format!("core/crates/conversation/{provider}/"))
    })
}

fn provider_owned(path: &str) -> bool {
    PROVIDERS
        .iter()
        .any(|provider| path.starts_with(&format!("core/crates/agent/providers/{provider}/")))
}
