use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_policy_contracts::process_signal as contract;
use yolo_architecture_process_signal_foundation::process_signal_authority_sites::CONSTRUCTOR_SITES;
use yolo_architecture_syntax::rust_source::{RustSource, identifier};
use yolo_architecture_syntax::targeted_tokens::Token;

const ALIAS_KEYWORD: &str = "as";
const ASSIGNMENT: &str = "=";
const GROUP_OPEN: &str = "{";
const ITEM_END: &str = ";";
const PATH_SEPARATOR: &str = "::";
const SELF_KEYWORD: &str = "self";
const TYPE_KEYWORD: &str = "type";
const USE_KEYWORD: &str = "use";
const SELF_ALIAS_SUFFIX_LENGTH: usize = 4;

pub fn inspect(file: &SourceFile, source: &RustSource, violations: &mut Vec<Violation>) {
    for (index, token) in source.tokens.iter().enumerate() {
        let Some(name) = identifier(token) else {
            continue;
        };
        if !sensitive_qualifier(name)
            || !import_alias(&source.tokens, index) && !type_alias(&source.tokens, index)
        {
            continue;
        }
        violations.push(Violation::line(
            &file.relative,
            token.line,
            contract::AUTHORITY_CONSTRUCTOR_MESSAGE,
        ));
    }
}

fn sensitive_qualifier(name: &str) -> bool {
    CONSTRUCTOR_SITES.iter().any(|site| site.qualifier == name)
}

fn import_alias(tokens: &[Token], index: usize) -> bool {
    let direct = tokens
        .get(index + 1)
        .is_some_and(|token| token.identifier(ALIAS_KEYWORD));
    let self_alias = matches!(
        tokens.get(index + 1..index + 1 + SELF_ALIAS_SUFFIX_LENGTH),
        Some([separator, open, receiver, alias])
            if separator.symbol(PATH_SEPARATOR)
                && open.symbol(GROUP_OPEN)
                && receiver.identifier(SELF_KEYWORD)
                && alias.identifier(ALIAS_KEYWORD)
    );
    (direct || self_alias)
        && item_prefix(tokens, index)
            .iter()
            .any(|token| token.identifier(USE_KEYWORD))
}

fn type_alias(tokens: &[Token], index: usize) -> bool {
    let prefix = item_prefix(tokens, index);
    prefix.iter().any(|token| token.identifier(TYPE_KEYWORD))
        && prefix.iter().any(|token| token.symbol(ASSIGNMENT))
}

fn item_prefix(tokens: &[Token], index: usize) -> &[Token] {
    let start = tokens[..index]
        .iter()
        .rposition(|token| token.symbol(ITEM_END))
        .map_or(0, |position| position + 1);
    &tokens[start..index]
}
