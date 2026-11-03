use yolo_architecture_syntax::targeted_tokens::Token;

const ARRAY_CLOSE: &str = "]";
const ARRAY_OPEN: &str = "[";
const TUPLE_CLOSE: &str = ")";
const TUPLE_OPEN: &str = "(";
const TYPE_CLOSE: &str = ">";
const TYPE_DOUBLE_CLOSE: &str = ">>";
const TYPE_OPEN: &str = "<";

#[derive(Default)]
pub(super) struct TypeNesting {
    tuple: usize,
    generic: usize,
    array: usize,
}

impl TypeNesting {
    pub(super) fn is_top_level(&self) -> bool {
        self.tuple == 0 && self.generic == 0 && self.array == 0
    }

    pub(super) fn observe(&mut self, token: &Token) {
        if token.symbol(TUPLE_OPEN) {
            self.tuple += 1;
        } else if token.symbol(TUPLE_CLOSE) {
            self.tuple = self.tuple.saturating_sub(1);
        } else if token.symbol(TYPE_OPEN) {
            self.generic += 1;
        } else if token.symbol(TYPE_CLOSE) {
            self.generic = self.generic.saturating_sub(1);
        } else if token.symbol(TYPE_DOUBLE_CLOSE) {
            self.generic = self.generic.saturating_sub(2);
        } else if token.symbol(ARRAY_OPEN) {
            self.array += 1;
        } else if token.symbol(ARRAY_CLOSE) {
            self.array = self.array.saturating_sub(1);
        }
    }
}
