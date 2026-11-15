use std::ops::Range;
use yolo_architecture_syntax::rust_source::RustFunction;
use yolo_architecture_syntax::targeted_tokens::Token;

const GROUP_OPEN: &str = "(";
const GROUP_CLOSE: &str = ")";
const ARRAY_OPEN: &str = "[";
const ARRAY_CLOSE: &str = "]";
const STATEMENT_END: &str = ";";
const BLOCK_OPEN: &str = "{";
const BLOCK_CLOSE: &str = "}";
const ASSIGNMENT: &str = "=";
const ARGUMENT_SEPARATOR: &str = ",";
const BOOLEAN_AND: &str = "&";
const BOOLEAN_OR: &str = "|";
const ADDITION: &str = "+";
const SUBTRACTION: &str = "-";
const MULTIPLICATION: &str = "*";
const DIVISION: &str = "/";
const REMAINDER: &str = "%";
const EQUALITY: &str = "==";
const INEQUALITY: &str = "!=";
const LET_KEYWORD: &str = "let";

pub fn range(tokens: &[Token], function: &RustFunction, method: usize) -> Range<usize> {
    let end = method.saturating_sub(1);
    let mut start = end;
    let mut groups = 0_usize;
    let mut arrays = 0_usize;
    let mut blocks = 0_usize;
    for index in (function.body.start..end).rev() {
        let token = &tokens[index];
        if token.symbol(GROUP_CLOSE) {
            groups += 1;
        } else if token.symbol(GROUP_OPEN) && groups > 0 {
            groups -= 1;
        } else if token.symbol(ARRAY_CLOSE) {
            arrays += 1;
        } else if token.symbol(ARRAY_OPEN) && arrays > 0 {
            arrays -= 1;
        } else if token.symbol(BLOCK_CLOSE) {
            blocks += 1;
        } else if token.symbol(BLOCK_OPEN) && blocks > 0 {
            blocks -= 1;
        } else if groups == 0 && arrays == 0 && blocks == 0 && boundary(token) {
            break;
        }
        start = index;
    }
    start..end
}

pub fn typed_binding_start(tokens: &[Token], lower: usize, colon: usize) -> usize {
    let mut groups = 0_usize;
    for index in (lower..colon).rev() {
        if tokens[index].symbol(GROUP_CLOSE) {
            groups += 1;
        } else if tokens[index].symbol(GROUP_OPEN) && groups > 0 {
            groups -= 1;
        } else if groups == 0
            && (tokens[index].symbol(GROUP_OPEN)
                || tokens[index].symbol(ARGUMENT_SEPARATOR)
                || tokens[index].symbol(STATEMENT_END)
                || tokens[index].symbol(BOOLEAN_OR)
                || tokens[index].identifier(LET_KEYWORD))
        {
            return index + 1;
        }
    }
    lower
}

fn boundary(token: &Token) -> bool {
    [
        STATEMENT_END,
        BLOCK_OPEN,
        BLOCK_CLOSE,
        ASSIGNMENT,
        ARGUMENT_SEPARATOR,
        BOOLEAN_AND,
        BOOLEAN_OR,
        ADDITION,
        SUBTRACTION,
        MULTIPLICATION,
        DIVISION,
        REMAINDER,
        EQUALITY,
        INEQUALITY,
    ]
    .iter()
    .any(|symbol| token.symbol(symbol))
}
