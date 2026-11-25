use crate::RUST_CFG_TEST_PREDICATE;
use crate::rust_source::{identifier, string};
use crate::targeted_tokens::{Token, closing};

const ATTRIBUTE_MARKER: &str = "#";
const ATTRIBUTE_OPEN: &str = "[";
const ATTRIBUTE_CLOSE: &str = "]";
const CFG_ATTRIBUTE: &str = "cfg";
const PREDICATE_OPEN: &str = "(";
const PREDICATE_CLOSE: &str = ")";
const SEPARATOR: &str = ",";
const NEGATION: &str = "not";
const ALL: &str = "all";
const ANY: &str = "any";
const FEATURE_PREDICATE: &str = "feature";
const ASSIGNMENT: &str = "=";
const ADVERSARIAL_FEATURE: &str = "adversarial-test-hooks";

pub fn is_test_guarded(tokens: &[Token]) -> bool {
    is_guarded(tokens, true)
}

pub fn is_unit_test_guarded(tokens: &[Token]) -> bool {
    is_guarded(tokens, false)
}

fn is_guarded(tokens: &[Token], feature_is_guarded: bool) -> bool {
    let mut cursor = 0_usize;
    while cursor + 2 < tokens.len() {
        if !tokens[cursor].symbol(ATTRIBUTE_MARKER)
            || !tokens[cursor + 1].symbol(ATTRIBUTE_OPEN)
            || !tokens[cursor + 2].identifier(CFG_ATTRIBUTE)
        {
            cursor += 1;
            continue;
        }
        let Some(attribute_end) = closing(tokens, cursor + 1, ATTRIBUTE_OPEN, ATTRIBUTE_CLOSE)
        else {
            return false;
        };
        if PredicateParser::new(&tokens[cursor + 3..attribute_end], feature_is_guarded).is_guarded()
        {
            return true;
        }
        cursor = attribute_end + 1;
    }
    false
}

struct PredicateParser<'a> {
    tokens: &'a [Token],
    cursor: usize,
    feature_is_guarded: bool,
}

impl<'a> PredicateParser<'a> {
    fn new(tokens: &'a [Token], feature_is_guarded: bool) -> Self {
        Self {
            tokens,
            cursor: 0,
            feature_is_guarded,
        }
    }

    fn is_guarded(mut self) -> bool {
        if !self.take_symbol(PREDICATE_OPEN) {
            return false;
        }
        let guarded = self.predicate();
        guarded == Some(true)
            && self.take_symbol(PREDICATE_CLOSE)
            && self.cursor == self.tokens.len()
    }

    fn predicate(&mut self) -> Option<bool> {
        let name = identifier(self.next()?)?;
        match name {
            RUST_CFG_TEST_PREDICATE => Some(true),
            FEATURE_PREDICATE => self.feature(),
            ALL => self.group(true),
            ANY => self.group(false),
            NEGATION => self.negation(),
            _ => self.unguarded_atom(),
        }
    }

    fn feature(&mut self) -> Option<bool> {
        if !self.take_symbol(ASSIGNMENT) {
            return None;
        }
        let is_adversarial = string(self.next()?) == Some(ADVERSARIAL_FEATURE);
        Some(self.feature_is_guarded && is_adversarial)
    }

    fn group(&mut self, conjunction: bool) -> Option<bool> {
        if !self.take_symbol(PREDICATE_OPEN) || self.take_symbol(PREDICATE_CLOSE) {
            return None;
        }
        let mut guarded = self.predicate()?;
        loop {
            if self.take_symbol(PREDICATE_CLOSE) {
                return Some(guarded);
            }
            if !self.take_symbol(SEPARATOR) {
                return None;
            }
            if self.take_symbol(PREDICATE_CLOSE) {
                return Some(guarded);
            }
            let next = self.predicate()?;
            guarded = if conjunction {
                guarded || next
            } else {
                guarded && next
            };
        }
    }

    fn negation(&mut self) -> Option<bool> {
        if !self.take_symbol(PREDICATE_OPEN) {
            return None;
        }
        self.predicate()?;
        self.take_symbol(PREDICATE_CLOSE).then_some(false)
    }

    fn unguarded_atom(&mut self) -> Option<bool> {
        if self.take_symbol(ASSIGNMENT) {
            self.next()?;
        }
        Some(false)
    }

    fn next(&mut self) -> Option<&'a Token> {
        let token = self.tokens.get(self.cursor)?;
        self.cursor += 1;
        Some(token)
    }

    fn take_symbol(&mut self, expected: &str) -> bool {
        if !self
            .tokens
            .get(self.cursor)
            .is_some_and(|token| token.symbol(expected))
        {
            return false;
        }
        self.cursor += 1;
        true
    }
}
