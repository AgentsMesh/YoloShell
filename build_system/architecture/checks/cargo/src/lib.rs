pub mod cargo_manifest_rules;
mod cargo_manifest_syntax;
pub mod cargo_rules;
mod cargo_shell;
pub mod cargo_shell_prefixes;
pub mod cargo_shell_substitutions;
pub mod cargo_shell_words;
mod cargo_starlark;
pub mod policy_exemptions;

pub(crate) use yolo_architecture_policy_contracts::constants as policy_constants;
pub(crate) use yolo_architecture_syntax::cargo_starlark_tokens;

#[cfg(test)]
pub fn inspect(
    root: &std::path::Path,
) -> std::io::Result<Vec<yolo_architecture_foundation::model::Violation>> {
    let files = yolo_architecture_test_support::load(root)?;
    let mut violations = Vec::new();
    for file in &files {
        cargo_rules::inspect(file, &mut violations);
    }
    violations.sort();
    violations.dedup();
    Ok(violations)
}

#[cfg(test)]
#[path = "../unit_tests/mod.rs"]
mod tests;
