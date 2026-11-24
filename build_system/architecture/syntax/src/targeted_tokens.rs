#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Kind {
    Identifier(String),
    Number(u64),
    String(String),
    Symbol(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Token {
    pub kind: Kind,
    pub line: usize,
}

impl Token {
    pub fn identifier(&self, expected: &str) -> bool {
        matches!(&self.kind, Kind::Identifier(value) if value == expected)
    }

    pub fn identifier_contains(&self, expected: &str) -> bool {
        matches!(&self.kind, Kind::Identifier(value) if value.contains(expected))
    }

    pub fn number(&self) -> Option<u64> {
        match self.kind {
            Kind::Number(value) => Some(value),
            _ => None,
        }
    }

    pub fn symbol(&self, expected: &str) -> bool {
        matches!(&self.kind, Kind::Symbol(value) if value == expected)
    }
}

#[path = "targeted_token_lexer.rs"]
mod lexer;
#[path = "targeted_token_literals.rs"]
mod literals;
pub use lexer::tokenize;

pub fn closing(tokens: &[Token], open: usize, left: &str, right: &str) -> Option<usize> {
    let mut depth = 0;
    for (index, token) in tokens.iter().enumerate().skip(open) {
        if token.symbol(left) {
            depth += 1;
        } else if token.symbol(right) {
            depth -= 1;
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}
