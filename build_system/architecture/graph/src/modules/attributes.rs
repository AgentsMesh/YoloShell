use yolo_architecture_policy_contracts::build_constants as vocabulary;
use yolo_architecture_syntax::rust_source::string;
use yolo_architecture_syntax::targeted_tokens::{Token, closing};

const ATTRIBUTE_MARKER: &str = "#";
const ATTRIBUTE_OPEN: &str = "[";
const ATTRIBUTE_CLOSE: &str = "]";
const ASSIGNMENT: &str = "=";
const BLOCK_OPEN: &str = "{";
const BLOCK_CLOSE: &str = "}";
const CFG_ATTRIBUTE: &str = "cfg";
const EMPTY_ANY_PREDICATE: &str = "any";
const GROUP_OPEN: &str = "(";
const GROUP_CLOSE: &str = ")";
const ITEM_END: &str = ";";
const PATH_ATTRIBUTE: &str = "path";
const ATTRIBUTE_NAME_OFFSET: usize = 1;
const SOURCE_START: usize = 0;
const NEXT_TOKEN_OFFSET: usize = 1;

#[derive(Default)]
pub(super) struct Attributes {
    pub path: Option<String>,
    pub disabled: bool,
    pub test_only: bool,
    pub uncertain: bool,
}

pub(super) fn before(tokens: &[Token], item: usize) -> Attributes {
    let start = item_start(tokens, item);
    let mut result = Attributes {
        test_only: yolo_architecture_syntax::rust_source_cfg::is_unit_test_guarded(
            &tokens[start..item],
        ),
        ..Attributes::default()
    };
    let mut cursor = start;
    while cursor < item {
        if !tokens[cursor].symbol(ATTRIBUTE_MARKER)
            || !tokens
                .get(cursor + NEXT_TOKEN_OFFSET)
                .is_some_and(|token| token.symbol(ATTRIBUTE_OPEN))
        {
            cursor += NEXT_TOKEN_OFFSET;
            continue;
        }
        let open = cursor + NEXT_TOKEN_OFFSET;
        let Some(close) = closing(tokens, open, ATTRIBUTE_OPEN, ATTRIBUTE_CLOSE) else {
            result.uncertain = true;
            break;
        };
        inspect(tokens, open, close, &mut result);
        cursor = close + NEXT_TOKEN_OFFSET;
    }
    result
}

fn inspect(tokens: &[Token], open: usize, close: usize, result: &mut Attributes) {
    let name = tokens.get(open + ATTRIBUTE_NAME_OFFSET);
    if name.is_some_and(|token| token.identifier(PATH_ATTRIBUTE)) {
        let value = direct_path(tokens, open, close);
        result.uncertain |= value.is_none() || result.path.is_some();
        result.path = result.path.take().or(value);
    } else if name.is_some_and(|token| token.identifier(CFG_ATTRIBUTE)) {
        result.disabled |= direct_empty_any(tokens, open, close);
    } else if name.is_some_and(|token| token.identifier(vocabulary::CFG_ATTR_ATTRIBUTE)) {
        result.uncertain |= tokens[open + ATTRIBUTE_NAME_OFFSET + NEXT_TOKEN_OFFSET..close]
            .iter()
            .any(|token| token.identifier(PATH_ATTRIBUTE) || token.identifier(CFG_ATTRIBUTE));
    }
}

fn direct_path(tokens: &[Token], open: usize, close: usize) -> Option<String> {
    let body = tokens.get(open + ATTRIBUTE_NAME_OFFSET..close)?;
    let [name, assignment, value] = body else {
        return None;
    };
    (name.identifier(PATH_ATTRIBUTE) && assignment.symbol(ASSIGNMENT))
        .then(|| string(value).map(str::to_owned))
        .flatten()
}

fn direct_empty_any(tokens: &[Token], open: usize, close: usize) -> bool {
    let Some(body) = tokens.get(open + ATTRIBUTE_NAME_OFFSET..close) else {
        return false;
    };
    let [cfg, outer_open, any, inner_open, inner_close, outer_close] = body else {
        return false;
    };
    cfg.identifier(CFG_ATTRIBUTE)
        && outer_open.symbol(GROUP_OPEN)
        && any.identifier(EMPTY_ANY_PREDICATE)
        && inner_open.symbol(GROUP_OPEN)
        && inner_close.symbol(GROUP_CLOSE)
        && outer_close.symbol(GROUP_CLOSE)
}

fn item_start(tokens: &[Token], index: usize) -> usize {
    (SOURCE_START..index)
        .rev()
        .find(|position| {
            tokens[*position].symbol(ITEM_END)
                || tokens[*position].symbol(BLOCK_OPEN)
                || tokens[*position].symbol(BLOCK_CLOSE)
        })
        .map_or(SOURCE_START, |position| position + NEXT_TOKEN_OFFSET)
}

#[cfg(test)]
#[path = "../../unit_tests/module_attribute_tests.rs"]
mod tests;
