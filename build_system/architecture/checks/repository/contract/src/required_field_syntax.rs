use std::ops::Range;
use yolo_architecture_syntax::targeted_tokens::{Kind, Token, closing};

pub(super) struct Declaration {
    pub attributes: Vec<Range<usize>>,
    pub body: Range<usize>,
}

pub(super) struct Field {
    pub attributes: Vec<Range<usize>>,
    pub type_tokens: Range<usize>,
}

pub(super) fn declaration(tokens: &[Token], name: &str) -> Option<Declaration> {
    let mut attributes = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        if let Some(attribute) = attribute(tokens, index) {
            index = attribute.end;
            attributes.push(attribute);
            continue;
        }
        if let Some(next) = visibility_end(tokens, index) {
            index = next;
            continue;
        }
        if (tokens[index].identifier("struct") || tokens[index].identifier("enum"))
            && tokens
                .get(index + 1)
                .is_some_and(|token| token.identifier(name))
        {
            let open = (index + 2..tokens.len())
                .find(|position| tokens[*position].symbol("{") || tokens[*position].symbol(";"))?;
            if tokens[open].symbol(";") {
                return None;
            }
            return Some(Declaration {
                attributes,
                body: open + 1..closing(tokens, open, "{", "}")?,
            });
        }
        attributes.clear();
        index += 1;
    }
    None
}

pub(super) fn variant(tokens: &[Token], body: Range<usize>, name: &str) -> Option<Range<usize>> {
    let mut index = body.start;
    while index < body.end {
        if tokens[index].identifier(name) && tokens.get(index + 1)?.symbol("{") {
            return Some(index + 2..closing(tokens, index + 1, "{", "}")?);
        }
        if tokens[index].symbol("{") {
            index = closing(tokens, index, "{", "}")? + 1;
        } else {
            index += 1;
        }
    }
    None
}

pub(super) fn field(tokens: &[Token], body: Range<usize>, name: &str) -> Option<Field> {
    let mut attributes = Vec::new();
    let mut index = body.start;
    while index < body.end {
        if let Some(attribute) = attribute(tokens, index) {
            index = attribute.end;
            attributes.push(attribute);
            continue;
        }
        if let Some(next) = visibility_end(tokens, index) {
            index = next;
            continue;
        }
        if tokens[index].identifier(name) && tokens.get(index + 1)?.symbol(":") {
            let start = index + 2;
            let end = (start..body.end)
                .find(|position| tokens[*position].symbol(","))
                .unwrap_or(body.end);
            return Some(Field {
                attributes,
                type_tokens: start..end,
            });
        }
        attributes.clear();
        index += 1;
    }
    None
}

pub(super) fn direct_type(tokens: &[Token], field: &Field, expected: &str) -> bool {
    let mut path = &tokens[field.type_tokens.clone()];
    if path.first().is_some_and(|token| token.symbol("::")) {
        path = &path[1..];
    }
    path.last().is_some_and(|token| token.identifier(expected))
        && path.iter().enumerate().all(|(index, token)| {
            if index % 2 == 0 {
                matches!(token.kind, Kind::Identifier(_))
            } else {
                token.symbol("::")
            }
        })
}

pub(super) fn attribute_contains(
    tokens: &[Token],
    attributes: &[Range<usize>],
    attribute: &str,
    values: &[&str],
) -> bool {
    attributes.iter().any(|range| {
        let body = &tokens[range.clone()];
        body.iter().any(|token| token.identifier(attribute))
            && body
                .iter()
                .any(|token| values.iter().any(|value| token.identifier(value)))
    })
}

pub(super) fn direct_attribute_contains(
    tokens: &[Token],
    attributes: &[Range<usize>],
    attribute: &str,
    value: &str,
) -> bool {
    attributes.iter().any(|range| {
        tokens
            .get(range.start + 2)
            .is_some_and(|token| token.identifier(attribute))
            && tokens[range.clone()]
                .iter()
                .any(|token| token.identifier(value))
    })
}

fn attribute(tokens: &[Token], index: usize) -> Option<Range<usize>> {
    if !tokens[index].symbol("#") || !tokens.get(index + 1)?.symbol("[") {
        return None;
    }
    Some(index..closing(tokens, index + 1, "[", "]")? + 1)
}

fn visibility_end(tokens: &[Token], index: usize) -> Option<usize> {
    if !tokens[index].identifier("pub") {
        return None;
    }
    if tokens.get(index + 1).is_some_and(|token| token.symbol("(")) {
        Some(closing(tokens, index + 1, "(", ")")? + 1)
    } else {
        Some(index + 1)
    }
}
