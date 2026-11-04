use crate::targeted_tokens::Token;

pub fn evaluate(tokens: &[Token]) -> Option<u64> {
    let mut parser = Parser { tokens, cursor: 0 };
    let value = parser.shift()?;
    (parser.cursor == tokens.len()).then_some(value)
}

pub fn contains_value(tokens: &[Token], expected: u64) -> bool {
    (0..tokens.len()).any(|start| {
        (start + 1..=tokens.len()).any(|end| evaluate(&tokens[start..end]) == Some(expected))
    })
}

struct Parser<'a> {
    tokens: &'a [Token],
    cursor: usize,
}

impl Parser<'_> {
    fn shift(&mut self) -> Option<u64> {
        let mut value = self.sum()?;
        while self.take("<<") {
            value = value.checked_shl(u32::try_from(self.sum()?).ok()?)?;
        }
        Some(value)
    }

    fn sum(&mut self) -> Option<u64> {
        let mut value = self.product()?;
        loop {
            if self.take("+") {
                value = value.checked_add(self.product()?)?;
            } else if self.take("-") {
                value = value.checked_sub(self.product()?)?;
            } else {
                return Some(value);
            }
        }
    }

    fn product(&mut self) -> Option<u64> {
        let mut value = self.primary()?;
        while self.take("*") {
            value = value.checked_mul(self.primary()?)?;
        }
        Some(value)
    }

    fn primary(&mut self) -> Option<u64> {
        if self.take("(") {
            let value = self.shift()?;
            self.take(")").then_some(value)
        } else {
            let value = self.tokens.get(self.cursor)?.number()?;
            self.cursor += 1;
            Some(value)
        }
    }

    fn take(&mut self, symbol: &str) -> bool {
        if self
            .tokens
            .get(self.cursor)
            .is_some_and(|token| token.symbol(symbol))
        {
            self.cursor += 1;
            true
        } else {
            false
        }
    }
}
