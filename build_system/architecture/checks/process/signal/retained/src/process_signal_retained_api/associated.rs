use super::block;
use std::ops::Range;
use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_process_signal_path_analysis::types::RetainedTypes;
use yolo_architecture_process_signal_path_analysis::visibility_fields as visibility;
use yolo_architecture_syntax::rust_source::RustSource;
use yolo_architecture_syntax::targeted_tokens::Token;

const ASSIGNMENT: &str = "=";
const BLOCK_CLOSE: &str = "}";
const BLOCK_OPEN: &str = "{";
const FOR_KEYWORD: &str = "for";
const IMPL_KEYWORD: &str = "impl";
const NEXT_TOKEN_OFFSET: usize = 1;
const TYPE_CLOSE: &str = ">";
const TYPE_DOUBLE_CLOSE: &str = ">>";
const TYPE_KEYWORD: &str = "type";
const TYPE_OPEN: &str = "<";
const TYPE_SEPARATOR: &str = ":";
const WHERE_KEYWORD: &str = "where";

pub(crate) struct Surface {
    pub implementation: usize,
    pub open: usize,
    pub item: usize,
    pub range: Range<usize>,
    pub trait_impl: bool,
}

pub(crate) fn inspect(
    file: &SourceFile,
    source: &RustSource,
    retained: &RetainedTypes,
    allows: &dyn Fn(&Surface) -> bool,
    violations: &mut Vec<Violation>,
) {
    for (implementation, token) in source.tokens.iter().enumerate() {
        if !token.identifier(IMPL_KEYWORD) {
            continue;
        }
        let Some((open, close)) = block(&source.tokens, implementation) else {
            continue;
        };
        let trait_impl = trait_separator(&source.tokens, implementation + NEXT_TOKEN_OFFSET, open);
        inspect_block(
            file,
            source,
            retained,
            allows,
            violations,
            implementation,
            open,
            close,
            trait_impl,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn inspect_block(
    file: &SourceFile,
    source: &RustSource,
    retained: &RetainedTypes,
    allows: &dyn Fn(&Surface) -> bool,
    violations: &mut Vec<Violation>,
    implementation: usize,
    open: usize,
    close: usize,
    trait_impl: bool,
) {
    let mut depth = 0_usize;
    for item in open + NEXT_TOKEN_OFFSET..close {
        let current = &source.tokens[item];
        if depth == 0 && current.identifier(TYPE_KEYWORD) {
            let end = visibility::statement_end(&source.tokens, item, close);
            let Some(range) = value_range(&source.tokens, item, end) else {
                continue;
            };
            let surface = Surface {
                implementation,
                open,
                item,
                range: range.clone(),
                trait_impl,
            };
            if (trait_impl || visibility::item_visible(&source.tokens, item))
                && range
                    .clone()
                    .any(|index| retained.contains(&source.tokens, index))
                && !allowed_contract(file, source, implementation, open, item, end)
                && !allows(&surface)
            {
                visibility::violation(file, current, violations);
            }
        }
        depth += usize::from(current.symbol(BLOCK_OPEN));
        depth = depth.saturating_sub(usize::from(current.symbol(BLOCK_CLOSE)));
    }
}

fn value_range(tokens: &[Token], start: usize, end: usize) -> Option<Range<usize>> {
    let separator = (start..end)
        .find(|index| tokens[*index].symbol(ASSIGNMENT) || tokens[*index].symbol(TYPE_SEPARATOR))?;
    Some(separator + NEXT_TOKEN_OFFSET..end)
}

fn allowed_contract(
    file: &SourceFile,
    source: &RustSource,
    _implementation: usize,
    _open: usize,
    _associated: usize,
    _end: usize,
) -> bool {
    let _ = (file, source);
    false
}

fn trait_separator(tokens: &[Token], start: usize, end: usize) -> bool {
    let mut generic = 0_usize;
    for token in &tokens[start..end] {
        if token.symbol(TYPE_OPEN) {
            generic += 1;
        } else if token.symbol(TYPE_CLOSE) {
            generic = generic.saturating_sub(1);
        } else if token.symbol(TYPE_DOUBLE_CLOSE) {
            generic = generic.saturating_sub(2);
        } else if generic == 0 && token.identifier(WHERE_KEYWORD) {
            return false;
        } else if generic == 0 && token.identifier(FOR_KEYWORD) {
            return true;
        }
    }
    false
}
