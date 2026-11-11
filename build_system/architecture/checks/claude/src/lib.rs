mod claude_generation_cas;
mod claude_generation_constants;

use yolo_architecture_foundation::{SourceFile, model::Violation};

pub fn inspect(files: &[SourceFile], violations: &mut Vec<Violation>) {
    claude_generation_cas::inspect(files, violations);
}

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod tests;
