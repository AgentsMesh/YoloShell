use crate::targeted_tokens::{Kind, tokenize};

const UNICODE_SOURCE: &str = "const 名称: &str = \"终端\";\nlet 图标 = 🚀;";
const FIRST_LINE: usize = 1;
const SECOND_LINE: usize = 2;
const ROCKET_SYMBOL: &str = "🚀";

#[test]
fn unicode_source_is_tokenized_without_splitting_utf8_boundaries() {
    let tokens = tokenize(UNICODE_SOURCE);
    let symbols = tokens
        .iter()
        .filter_map(|token| match &token.kind {
            Kind::Symbol(value) => Some((value.as_str(), token.line)),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert!(symbols.contains(&("名", FIRST_LINE)));
    assert!(symbols.contains(&("称", FIRST_LINE)));
    assert!(symbols.contains(&("图", SECOND_LINE)));
    assert!(symbols.contains(&("标", SECOND_LINE)));
    assert!(symbols.contains(&(ROCKET_SYMBOL, SECOND_LINE)));
    assert!(tokens.iter().any(|token| {
        matches!(&token.kind, Kind::String(value) if value == "终端") && token.line == FIRST_LINE
    }));
}
