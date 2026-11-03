use crate::{QueryCallbacks, QueryScope, constants::*, detector::QueryDetector};

#[derive(Default)]
pub struct QueryProcessor {
    eligible: vte::Parser,
    scope: Option<QueryScope>,
}

impl QueryProcessor {
    pub fn reset(&mut self) {
        self.eligible = vte::Parser::new();
        self.scope = None;
    }

    /// Process restored/observed bytes without reply authority. This never resets
    /// the screen parser's partial UTF-8, OSC, DCS or CSI state.
    pub fn process_quiet(&mut self, parser: &mut vt100::Parser<QueryCallbacks>, bytes: &[u8]) {
        self.reset();
        parser.callbacks_mut().clear();
        parser.process(bytes);
    }

    pub fn process(
        &mut self,
        parser: &mut vt100::Parser<QueryCallbacks>,
        bytes: &[u8],
        scope: Option<QueryScope>,
    ) -> Vec<Vec<u8>> {
        let Some(scope) = scope else {
            self.process_quiet(parser, bytes);
            return Vec::new();
        };
        if self.scope != Some(scope) {
            self.reset();
            self.scope = Some(scope);
        }
        let mut remaining = bytes;
        let mut replies = Vec::new();
        let mut total = usize::default();
        while !remaining.is_empty() {
            let mut detector = QueryDetector::default();
            let consumed = self
                .eligible
                .advance_until_terminated(&mut detector, remaining);
            let (prefix, rest) = remaining.split_at(consumed);
            remaining = rest;
            parser.callbacks_mut().clear();
            let Some(query) = detector.query else {
                parser.process(prefix);
                continue;
            };
            let (final_byte, before) = prefix
                .split_last()
                .expect("query dispatch consumes its final byte");
            parser.process(before);
            parser.callbacks_mut().arm(query);
            parser.process(std::slice::from_ref(final_byte));
            if let Some(reply) = parser.callbacks_mut().take() {
                if replies.len() < MAX_REPLY_COUNT
                    && reply.len() <= MAX_REPLY_BYTES.saturating_sub(total)
                {
                    total += reply.len();
                    replies.push(reply);
                }
            }
        }
        replies
    }
}
