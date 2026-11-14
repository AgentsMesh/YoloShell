use super::RetainedTypes;
use crate::paths::VisibilityBoundaries;
use yolo_architecture_syntax::rust_source::identifier;
use yolo_architecture_syntax::targeted_tokens::Token;

impl RetainedTypes {
    pub fn aggregate_boundaries(
        &self,
        tokens: &[Token],
        index: usize,
    ) -> Option<VisibilityBoundaries> {
        if self.canonical_qualified.contains(&index) {
            return None;
        }
        if let Some(boundaries) = self.aggregate_qualified.get(&index) {
            return Some(boundaries.clone());
        }
        if self.clean_qualified.contains(&index) {
            return None;
        }
        let name = identifier(&tokens[index])?;
        (!self.canonical_names.contains(name))
            .then(|| self.aggregate_names.get(name).cloned())
            .flatten()
    }
}
