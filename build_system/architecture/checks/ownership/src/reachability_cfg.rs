use super::reachability_config::Configuration;
use yolo_architecture_syntax::rust_source::{identifier, string};
use yolo_architecture_syntax::targeted_tokens::{Token, closing};

const CFG_TEST: &str = "test";
const FEATURE: &str = "feature";
const ALL: &str = "all";
const ANY: &str = "any";
const NOT: &str = "not";
const OPEN: &str = "(";
const CLOSE: &str = ")";
const COMMA: &str = ",";
const EQUALS: &str = "=";
const NEXT: usize = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Truth {
    Yes,
    No,
    Unknown,
}

impl Truth {
    fn combine(self, other: Self, conjunction: bool) -> Self {
        let decisive = if conjunction { Self::No } else { Self::Yes };
        if self == decisive || other == decisive {
            decisive
        } else if self == Self::Unknown || other == Self::Unknown {
            Self::Unknown
        } else {
            self
        }
    }
}

pub(super) fn evaluate(tokens: &[Token], configuration: &Configuration) -> Truth {
    let mut parser = Parser {
        tokens,
        cursor: usize::default(),
        configuration,
    };
    let result = parser.predicate();
    if parser.cursor == tokens.len() {
        result.unwrap_or(Truth::Unknown)
    } else {
        Truth::Unknown
    }
}

struct Parser<'a> {
    tokens: &'a [Token],
    cursor: usize,
    configuration: &'a Configuration,
}

impl Parser<'_> {
    fn predicate(&mut self) -> Option<Truth> {
        let name = identifier(self.tokens.get(self.cursor)?)?;
        self.cursor += NEXT;
        match name {
            CFG_TEST => Some(if self.configuration.test {
                Truth::Yes
            } else {
                Truth::No
            }),
            ALL | ANY => self.group(name == ALL),
            NOT => {
                self.take(OPEN)?;
                let value = self.predicate()?;
                self.take(CLOSE)?;
                Some(match value {
                    Truth::Yes => Truth::No,
                    Truth::No => Truth::Yes,
                    Truth::Unknown => Truth::Unknown,
                })
            }
            FEATURE => {
                self.take(EQUALS)?;
                let feature = string(self.tokens.get(self.cursor)?)?;
                self.cursor += NEXT;
                Some(match &self.configuration.features {
                    Some(features) if features.contains(feature) => Truth::Yes,
                    Some(_) => Truth::No,
                    None => Truth::Unknown,
                })
            }
            _ => self.unknown_atom(),
        }
    }

    fn group(&mut self, conjunction: bool) -> Option<Truth> {
        self.take(OPEN)?;
        let mut result = if conjunction { Truth::Yes } else { Truth::No };
        while !self.at(CLOSE) {
            result = result.combine(self.predicate()?, conjunction);
            if self.at(CLOSE) {
                break;
            }
            self.take(COMMA)?;
        }
        self.take(CLOSE)?;
        Some(result)
    }

    fn unknown_atom(&mut self) -> Option<Truth> {
        if self.at(EQUALS) {
            self.cursor += NEXT;
            self.tokens.get(self.cursor)?;
            self.cursor += NEXT;
        } else if self.at(OPEN) {
            self.cursor = closing(self.tokens, self.cursor, OPEN, CLOSE)? + NEXT;
        }
        Some(Truth::Unknown)
    }

    fn at(&self, symbol: &str) -> bool {
        self.tokens
            .get(self.cursor)
            .is_some_and(|token| token.symbol(symbol))
    }
    fn take(&mut self, symbol: &str) -> Option<()> {
        if !self.at(symbol) {
            return None;
        }
        self.cursor += NEXT;
        Some(())
    }
}
