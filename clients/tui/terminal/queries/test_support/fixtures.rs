use yolo_tui_terminal_queries::{QueryCallbacks, QueryProcessor, QueryScope};

#[path = "constants.rs"]
mod constants;
pub use constants::*;

pub struct Fixture {
    pub parser: vt100::Parser<QueryCallbacks>,
    pub queries: QueryProcessor,
    pub scope: QueryScope,
}

impl Fixture {
    pub fn new() -> Self {
        Self {
            parser: vt100::Parser::new_with_callbacks(
                ROWS,
                COLUMNS,
                SCROLLBACK,
                QueryCallbacks::default(),
            ),
            queries: QueryProcessor::default(),
            scope: scope(),
        }
    }
    pub fn live(&mut self, bytes: &[u8]) -> Vec<Vec<u8>> {
        self.queries
            .process(&mut self.parser, bytes, Some(self.scope))
    }
    pub fn quiet(&mut self, bytes: &[u8]) {
        self.queries.process_quiet(&mut self.parser, bytes);
    }
}

pub fn scope() -> QueryScope {
    QueryScope::new(uuid::Uuid::new_v4()).unwrap()
}
