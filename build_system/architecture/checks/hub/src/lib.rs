mod coverage;
mod roles;

use yolo_architecture_foundation::{SourceFile, model::Violation};
use yolo_architecture_target_ownership::{OWNERSHIP_POLICY_BUILD_FILE, Repository};

use roles::{ROLES_FILE, read};

pub fn inspect(files: &[SourceFile], violations: &mut Vec<Violation>) {
    let Some(classification) = read(files, violations) else {
        return;
    };
    if !files
        .iter()
        .any(|file| file.relative == OWNERSHIP_POLICY_BUILD_FILE)
    {
        return;
    }
    let repository = Repository::new(files);
    coverage::inspect(&repository, &classification, ROLES_FILE, violations);
}

#[cfg(test)]
#[path = "../unit_tests/hub_policy_tests.rs"]
mod tests;
