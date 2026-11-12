use crate::{QueryScope, TerminalSurface};
use yolo_hub_terminal_protocol::{PublicError, TerminalEvent};

impl TerminalSurface {
    /// Eligibility is supplied by the caller only for new live output under a
    /// current exact writer. Protocol ordering is checked before parsing bytes.
    pub fn apply_live_event(
        &mut self,
        event: &TerminalEvent,
        scope: Option<QueryScope>,
    ) -> Result<Vec<Vec<u8>>, PublicError> {
        let next = match event.advance(self.cursor) {
            Ok(next) => next,
            Err(error) => {
                self.reset_query_scope();
                return Err(error);
            }
        };
        let replies = match event {
            TerminalEvent::Output { chunk } => {
                self.queries
                    .process(&mut self.parser, &chunk.fields().data, scope)
            }
            TerminalEvent::Exited { .. } => {
                self.reset_query_scope();
                Vec::new()
            }
            _ => self.queries.process(&mut self.parser, &[], scope),
        };
        self.cursor = next;
        Ok(replies)
    }

    pub fn reset_query_scope(&mut self) {
        self.queries.reset();
    }
}
