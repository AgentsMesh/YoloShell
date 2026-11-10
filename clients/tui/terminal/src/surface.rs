use crate::SCROLLBACK_ROWS;
use yolo_hub_terminal_protocol::{
    AttachRequest, AttachResult, OutputCursor, PublicError, TerminalEvent, TerminalHistory,
    TerminalSize, TerminalSnapshot,
};
use yolo_tui_terminal_queries::{QueryCallbacks, QueryProcessor};

pub struct TerminalSurface {
    pub(crate) parser: vt100::Parser<QueryCallbacks>,
    pub(crate) cursor: OutputCursor,
    pub(crate) queries: QueryProcessor,
}

impl TerminalSurface {
    pub fn new(snapshot: &TerminalSnapshot) -> Self {
        let fields = snapshot.fields();
        Self::from_data(
            fields.size,
            &fields.data,
            OutputCursor {
                target: fields.target,
                after_sequence: fields.barrier_sequence,
            },
        )
    }

    /// Replays the retained raw stream into a fresh parser, recovering
    /// scrollback, alternate-screen transitions and input modes. Bytes do not
    /// record resizes, so wrapping is interpreted at the current runtime size.
    pub fn from_history(history: &TerminalHistory) -> Self {
        let fields = history.fields();
        Self::from_data(
            fields.size,
            &fields.data,
            OutputCursor {
                target: fields.target,
                after_sequence: fields.barrier_sequence,
            },
        )
    }

    fn from_data(size: TerminalSize, data: &[u8], cursor: OutputCursor) -> Self {
        let mut parser = vt100::Parser::new_with_callbacks(
            size.rows(),
            size.cols(),
            SCROLLBACK_ROWS,
            QueryCallbacks::default(),
        );
        let mut queries = QueryProcessor::default();
        queries.process_quiet(&mut parser, data);
        Self {
            parser,
            queries,
            cursor,
        }
    }

    pub fn cursor(&self) -> OutputCursor {
        self.cursor
    }
    pub fn screen(&self) -> &vt100::Screen {
        self.parser.screen()
    }

    /// The program asked the terminal to defer presentation until it closes
    /// the synchronized update. Painting now would expose intermediate cursor
    /// positions and partial frames.
    pub fn synchronized_output(&self) -> bool {
        self.parser.callbacks().synchronized_output()
    }

    pub fn apply_event(&mut self, event: &TerminalEvent) -> Result<(), PublicError> {
        self.reset_query_scope();
        self.apply_live_event(event, None).map(|_| ())
    }

    /// Validate the entire response before modifying the parser or output cursor.
    pub fn from_result(result: &AttachResult) -> Option<Self> {
        result
            .fields()
            .history
            .as_ref()
            .map(Self::from_history)
            .or_else(|| result.fields().snapshot.as_ref().map(Self::new))
    }

    pub fn apply(
        &mut self,
        request: AttachRequest,
        result: &AttachResult,
    ) -> Result<(), PublicError> {
        self.reset_query_scope();
        if request.target() != self.cursor.target {
            return Err(PublicError::TargetMismatch);
        }
        if request.cursor().is_some_and(|cursor| cursor != self.cursor) {
            return Err(PublicError::OutputGap);
        }
        let next = result.validate_for_request(request)?;
        if next.after_sequence < self.cursor.after_sequence {
            return Err(PublicError::OutputGap);
        }
        if let Some(replacement) = Self::from_result(result) {
            *self = replacement;
        } else {
            for chunk in &result.fields().chunks {
                self.queries
                    .process_quiet(&mut self.parser, &chunk.fields().data);
            }
        }
        self.cursor = next;
        Ok(())
    }

    /// Scrolling is local observation state and must not resize the remote PTY.
    pub fn set_scrollback(&mut self, rows: usize) {
        self.parser
            .screen_mut()
            .set_scrollback(rows.min(SCROLLBACK_ROWS));
    }
}
