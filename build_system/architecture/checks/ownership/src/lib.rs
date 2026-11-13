mod budgets;
mod constants;
mod evaluator;
mod expression_parser;
mod glob_evaluator;
mod layout;
mod model;
mod parser;
mod path_rules;
mod paths;
mod patterns;
mod production;
mod reachability;
mod reachability_cfg;
mod reachability_config;
mod reachability_tokens;
mod repository;
mod rules;
mod targets;
mod test_ownership;

pub use constants::{
    MAX_BUILD_SYSTEM_PRODUCTION_LINES_PER_PACKAGE, MAX_BUILD_SYSTEM_PRODUCTION_SOURCES_PER_PACKAGE,
    MAX_BUILD_SYSTEM_PRODUCTION_SOURCES_PER_TARGET, MAX_BUILD_SYSTEM_TEST_SOURCES_PER_TARGET,
    MAX_CORE_PRODUCTION_LINES_PER_PACKAGE, MAX_CORE_PRODUCTION_SOURCES_PER_PACKAGE,
    OWNERSHIP_POLICY_BUILD_FILE, RUNTIME_SOURCES_TARGET, SOURCES_ATTRIBUTE, TRUE_LITERAL,
};
pub use model::{Document, Expression, RuleCall};
pub use paths::{is_build_file, label};
pub use repository::Repository;
pub use rules::inspect;

pub fn inspect_repository(
    files: &[yolo_architecture_foundation::SourceFile],
    violations: &mut Vec<yolo_architecture_foundation::model::Violation>,
) {
    if files
        .iter()
        .any(|file| file.relative == OWNERSHIP_POLICY_BUILD_FILE)
    {
        inspect(files, violations);
    }
}

#[cfg(test)]
#[path = "../unit_tests/mod.rs"]
mod tests;
