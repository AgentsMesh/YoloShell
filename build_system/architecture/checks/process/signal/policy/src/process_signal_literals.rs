use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_policy_contracts::process_signal as contract;
use yolo_architecture_process_signal_foundation::process_signal_sites::{Site, site};
use yolo_architecture_syntax::rust_source::{RustFunction, RustSource, identifier};
use yolo_architecture_syntax::targeted_tokens::{Token, closing};

const STRUCT_KEYWORD: &str = "struct";
const IMPL_KEYWORD: &str = "impl";
const ITEM_END: &str = ";";
const TUPLE_OPEN: &str = "(";
const TUPLE_CLOSE: &str = ")";
const RECORD_OPEN: &str = "{";
const RECORD_CLOSE: &str = "}";
const PATTERN_ASSIGNMENT: &str = "=";
const PATTERN_TYPE_SEPARATOR: &str = ":";
const RETURN_ARROW_START: &str = "-";
const RETURN_ARROW_END: &str = ">";

struct LiteralSite {
    authority: &'static str,
    site: Site,
}

const LITERAL_SITES: &[LiteralSite] = &[
    literal(
        contract::PROCESS_SIGNAL_AUTHORITY,
        contract::SIGNAL_AUTHORITY_PATH,
        Some(contract::PROCESS_SIGNAL_AUTHORITY),
        contract::NEW,
    ),
    literal(
        contract::DESTRUCTIVE_AUTHORITY,
        contract::SIGNAL_AUTHORITY_PATH,
        Some(contract::DESTRUCTIVE_ACCESS),
        contract::CAPTURE,
    ),
    literal(
        contract::DESTRUCTIVE_AUTHORITY,
        contract::SIGNAL_AUTHORITY_PATH,
        Some(contract::DESTRUCTIVE_ACCESS),
        contract::INSTALL_LINUX_CONTAINMENT,
    ),
    literal(
        contract::DESTRUCTIVE_AUTHORITY,
        contract::SIGNAL_AUTHORITY_PATH,
        Some(contract::DESTRUCTIVE_ACCESS),
        contract::AUTHORIZE_FOR_TEST,
    ),
    literal(
        contract::FOREGROUND_AUTHORITY,
        contract::SIGNAL_AUTHORITY_PATH,
        Some(contract::FOREGROUND_AUTHORITY),
        contract::VERIFIED,
    ),
    literal(
        contract::PROCESS_GROUP_IDENTITY,
        contract::INVENTORY_PATH,
        Some(contract::PROCESS_GROUP_IDENTITY),
        contract::VERIFIED,
    ),
];

pub fn inspect(file: &SourceFile, source: &RustSource, violations: &mut Vec<Violation>) {
    for (index, token) in source.tokens.iter().enumerate() {
        let function = source.function_at(index);
        let Some(authority) = literal_authority(token, function) else {
            continue;
        };
        let Some((open, close)) = literal_delimiters(&source.tokens, index) else {
            continue;
        };
        if declaration(&source.tokens, index) || pattern(&source.tokens, index + 1, open, close) {
            continue;
        }
        let allowed = LITERAL_SITES.iter().any(|entry| {
            entry.authority == authority && entry.site.matches(&file.relative, function)
        });
        if !allowed {
            violations.push(Violation::line(
                &file.relative,
                token.line,
                contract::AUTHORITY_CONSTRUCTOR_MESSAGE,
            ));
        }
    }
}

fn literal_authority<'a>(token: &'a Token, function: Option<&'a RustFunction>) -> Option<&'a str> {
    let name = identifier(token)?;
    let authority = if name == contract::SELF_TYPE {
        function?.owner.as_deref()?
    } else {
        name
    };
    LITERAL_SITES
        .iter()
        .any(|entry| entry.authority == authority)
        .then_some(authority)
}

fn literal_delimiters(tokens: &[Token], index: usize) -> Option<(&str, &str)> {
    let next = tokens.get(index + 1)?;
    if next.symbol(TUPLE_OPEN) {
        Some((TUPLE_OPEN, TUPLE_CLOSE))
    } else if next.symbol(RECORD_OPEN) {
        Some((RECORD_OPEN, RECORD_CLOSE))
    } else {
        None
    }
}

fn declaration(tokens: &[Token], index: usize) -> bool {
    if index > 0 && tokens[index - 1].identifier(STRUCT_KEYWORD) {
        return true;
    }
    if index >= 2
        && tokens[index - 2].symbol(RETURN_ARROW_START)
        && tokens[index - 1].symbol(RETURN_ARROW_END)
    {
        return true;
    }
    tokens[..index]
        .iter()
        .rev()
        .take_while(|token| {
            !token.symbol(ITEM_END) && !token.symbol(RECORD_OPEN) && !token.symbol(RECORD_CLOSE)
        })
        .any(|token| token.identifier(IMPL_KEYWORD))
}

fn pattern(tokens: &[Token], open: usize, left: &str, right: &str) -> bool {
    closing(tokens, open, left, right)
        .and_then(|close| tokens.get(close + 1))
        .is_some_and(|token| {
            token.symbol(PATTERN_ASSIGNMENT) || token.symbol(PATTERN_TYPE_SEPARATOR)
        })
}

const fn literal(
    authority: &'static str,
    path: &'static str,
    owner: Option<&'static str>,
    function: &'static str,
) -> LiteralSite {
    LiteralSite {
        authority,
        site: site(path, owner, function),
    }
}
