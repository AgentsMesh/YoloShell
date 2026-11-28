use super::declarations;
use std::collections::BTreeSet;
use yolo_architecture_syntax::targeted_tokens::Token;

const BLOCK_OPEN: &str = "{";
const BLOCK_CLOSE: &str = "}";

#[derive(Clone, Default, Eq, PartialEq)]
pub(super) struct FileIndex {
    positions: Vec<Position>,
}

#[derive(Clone, Default, Eq, PartialEq)]
struct Position {
    ancestors: Vec<String>,
    blocks: Vec<usize>,
    module_item: bool,
}

impl FileIndex {
    pub fn build(tokens: &[Token]) -> Self {
        let declarations = declarations::parse(tokens);
        let module_bodies = declarations
            .iter()
            .filter_map(|declaration| declaration.body.as_ref().map(|body| body.start))
            .collect::<BTreeSet<_>>();
        let mut blocks = Vec::new();
        let mut positions = Vec::with_capacity(tokens.len());
        for (index, token) in tokens.iter().enumerate() {
            positions.push(Position {
                ancestors: declarations::ancestors(&declarations, index)
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
                module_item: blocks.iter().all(|open| module_bodies.contains(open)),
                blocks: blocks.clone(),
            });
            if token.symbol(BLOCK_OPEN) {
                blocks.push(index);
            } else if token.symbol(BLOCK_CLOSE) {
                blocks.pop();
            }
        }
        Self { positions }
    }

    pub fn ancestors_at(&self, index: usize) -> &[String] {
        self.positions
            .get(index)
            .map(|position| position.ancestors.as_slice())
            .unwrap_or_default()
    }

    pub fn blocks_at(&self, index: usize) -> &[usize] {
        self.positions
            .get(index)
            .map(|position| position.blocks.as_slice())
            .unwrap_or_default()
    }

    pub fn module_item(&self, index: usize) -> bool {
        self.positions
            .get(index)
            .is_some_and(|position| position.module_item)
    }
}
