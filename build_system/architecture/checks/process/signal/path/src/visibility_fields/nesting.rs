use yolo_architecture_syntax::targeted_tokens::Token;

const ARRAY_CLOSE: &str = "]";
const ARRAY_OPEN: &str = "[";
const BLOCK_CLOSE: &str = "}";
const BLOCK_OPEN: &str = "{";
const GROUP_CLOSE: &str = ")";
const GROUP_OPEN: &str = "(";
const TYPE_CLOSE: &str = ">";
const TYPE_DOUBLE_CLOSE: &str = ">>";
const TYPE_OPEN: &str = "<";

#[derive(Default)]
pub(super) struct Nesting {
    group: usize,
    generic: usize,
    array: usize,
    block: usize,
}

impl Nesting {
    pub(super) fn is_top_level(&self) -> bool {
        self.group == 0 && self.generic == 0 && self.array == 0 && self.block == 0
    }

    pub(super) fn observe(&mut self, token: &Token) {
        for (left, right, depth) in [
            (GROUP_OPEN, GROUP_CLOSE, &mut self.group),
            (ARRAY_OPEN, ARRAY_CLOSE, &mut self.array),
            (BLOCK_OPEN, BLOCK_CLOSE, &mut self.block),
        ] {
            if token.symbol(left) {
                *depth += 1;
            } else if token.symbol(right) {
                *depth = depth.saturating_sub(1);
            }
        }
        if token.symbol(TYPE_OPEN) {
            self.generic += 1;
        } else if token.symbol(TYPE_CLOSE) {
            self.generic = self.generic.saturating_sub(1);
        } else if token.symbol(TYPE_DOUBLE_CLOSE) {
            self.generic = self.generic.saturating_sub(2);
        }
    }
}
