use crate::model::{CallExpression, Expression};
use std::collections::BTreeMap;
use yolo_architecture_syntax::cargo_starlark_tokens::Token;

const NEXT_TOKEN: usize = 1;
const ASSIGNMENT_WIDTH: usize = 2;
const COMPREHENSION_KEYWORD: &str = "for";

pub(crate) struct ExpressionParser<'a> {
    tokens: &'a [Token],
    cursor: usize,
}

impl<'a> ExpressionParser<'a> {
    pub(crate) fn new(tokens: &'a [Token], cursor: usize) -> Self {
        Self { tokens, cursor }
    }

    pub(crate) fn parse(&mut self) -> Option<Expression> {
        let first = self.postfix()?;
        let mut values = vec![first];
        while self.take('+') {
            values.push(self.postfix()?);
        }
        Some(if values.len() == NEXT_TOKEN {
            values.pop().expect("one parsed expression")
        } else {
            Expression::Add(values)
        })
    }

    fn postfix(&mut self) -> Option<Expression> {
        let mut value = self.primary()?;
        while self.take('.') {
            self.primary()?;
            value = Expression::Unknown;
        }
        Some(value)
    }

    fn primary(&mut self) -> Option<Expression> {
        let token = self.tokens.get(self.cursor)?;
        if let Some(value) = token.string() {
            self.cursor += NEXT_TOKEN;
            return Some(Expression::String(value.to_owned()));
        }
        if let Some(name) = token.identifier().map(str::to_owned) {
            self.cursor += NEXT_TOKEN;
            return if self.take('(') {
                self.call(name)
            } else {
                Some(Expression::Name(name))
            };
        }
        if self.take('[') {
            return self.sequence(']').map(Expression::List);
        }
        if self.take('(') {
            let expression = self.parse()?;
            return self.take(')').then_some(expression);
        }
        self.skip_balanced()?;
        Some(Expression::Unknown)
    }

    fn call(&mut self, name: String) -> Option<Expression> {
        let mut positional = Vec::new();
        let mut attributes = BTreeMap::new();
        while self.cursor < self.tokens.len() {
            if self.take(')') {
                return Some(Expression::Call(CallExpression {
                    name,
                    positional,
                    attributes,
                }));
            }
            let attribute = self
                .tokens
                .get(self.cursor)
                .and_then(Token::identifier)
                .filter(|_| {
                    self.tokens
                        .get(self.cursor + NEXT_TOKEN)
                        .is_some_and(|token| token.symbol('='))
                })
                .map(str::to_owned);
            if let Some(attribute) = attribute {
                self.cursor += ASSIGNMENT_WIDTH;
                attributes.insert(attribute, self.parse()?);
            } else {
                positional.push(self.parse()?);
            }
            if !self.take(',') && !self.peek(')') {
                return None;
            }
        }
        None
    }

    fn sequence(&mut self, close: char) -> Option<Vec<Expression>> {
        let mut values = Vec::new();
        while self.cursor < self.tokens.len() {
            if self.take(close) {
                return Some(values);
            }
            values.push(self.parse()?);
            if self.tokens.get(self.cursor).and_then(Token::identifier)
                == Some(COMPREHENSION_KEYWORD)
            {
                while !self.take(close) {
                    self.skip_balanced()?;
                }
                return Some(vec![Expression::Unknown]);
            }
            if !self.take(',') && !self.peek(close) {
                return None;
            }
        }
        None
    }

    fn skip_balanced(&mut self) -> Option<()> {
        let token = self.tokens.get(self.cursor)?;
        if [']', '}', ')'].iter().any(|close| token.symbol(*close)) {
            return None;
        }
        let close = [('{', '}'), ('(', ')'), ('[', ']')]
            .into_iter()
            .find_map(|(open, close)| token.symbol(open).then_some(close));
        self.cursor += NEXT_TOKEN;
        if let Some(close) = close {
            while !self.take(close) {
                self.skip_balanced()?;
            }
        }
        Some(())
    }

    fn peek(&self, symbol: char) -> bool {
        self.tokens
            .get(self.cursor)
            .is_some_and(|token| token.symbol(symbol))
    }

    fn take(&mut self, symbol: char) -> bool {
        if !self.peek(symbol) {
            return false;
        }
        self.cursor += NEXT_TOKEN;
        true
    }
}
