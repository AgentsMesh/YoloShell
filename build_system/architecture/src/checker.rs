use crate::model::Violation;
use crate::source_overlay;
use crate::sources;
use std::io;
use std::path::Path;
use yolo_architecture_cargo_policy::cargo_rules;
use yolo_architecture_claude_policy::inspect as inspect_claude_policy;
use yolo_architecture_dependency_usage::inspect as inspect_dependency_usage;
use yolo_architecture_hub_policy::inspect as inspect_hub_policy;
use yolo_architecture_process_signal_policy::{
    inspect as inspect_process_signal_policy,
    inspect_repository as inspect_process_signal_repository,
};
use yolo_architecture_repository::repository_rules;
use yolo_architecture_repository_policy::{
    inspect_file as inspect_repository_policy, inspect_repository as inspect_repository_contracts,
    literal_registry,
};
use yolo_architecture_runtime_safety_policy::{
    inspect_file as inspect_runtime_safety, inspect_repository as inspect_runtime_safety_repository,
};
use yolo_architecture_source_policy::{brand_rules, line_rules, source_policy};
use yolo_architecture_target_entrypoint::inspect as inspect_target_entrypoint;

pub fn inspect(root: &Path) -> io::Result<Vec<Violation>> {
    let files = sources::load(root)?;
    inspect_files(root, &files)
}

pub fn inspect_with_runfiles(root: &Path, runfiles: Option<&Path>) -> io::Result<Vec<Violation>> {
    let files = source_overlay::load(root, runfiles)?;
    inspect_files(root, &files)
}

fn inspect_files(root: &Path, files: &[sources::SourceFile]) -> io::Result<Vec<Violation>> {
    let literals = literal_registry(files);
    let mut violations = sources::retired_directories(root)?
        .into_iter()
        .map(|path| {
            Violation::file(
                path,
                "JavaScript and TypeScript tooling is retired from the native repository",
            )
        })
        .collect::<Vec<_>>();
    repository_rules::inspect(files, &sources::directories(root)?, &mut violations);
    inspect_claude_policy(files, &mut violations);
    crate::inspect_target_ownership(files, &mut violations);
    inspect_target_entrypoint(files, &mut violations);
    inspect_dependency_usage(files, &mut violations);
    inspect_hub_policy(files, &mut violations);
    inspect_repository_contracts(files, &mut violations);
    inspect_runtime_safety_repository(files, &mut violations);
    inspect_process_signal_repository(files, &mut violations);
    for file in files {
        line_rules::inspect(file, &mut violations);
        brand_rules::inspect(file, &mut violations);
        cargo_rules::inspect(file, &mut violations);
        source_policy::inspect(file, &mut violations);
        inspect_repository_policy(file, &literals, &mut violations);
        inspect_runtime_safety(file, &mut violations);
        inspect_process_signal_policy(file, &mut violations);
    }
    violations.sort();
    violations.dedup();
    Ok(violations)
}
