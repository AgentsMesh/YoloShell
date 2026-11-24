mod completion;
use crate::{Chat, Mode};
use uuid::Uuid;
use yolo_conversation_protocol::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReadKind {
    Latest,
    Older,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum QueryKind {
    Resolve {
        binding: ConversationBinding,
    },
    Read {
        request: ConversationReadRequest,
        kind: ReadKind,
    },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Query {
    pub(crate) id: Uuid,
    pub(crate) epoch: Uuid,
    pub(crate) kind: QueryKind,
}
impl Query {
    pub fn id(&self) -> Uuid {
        self.id
    }
    pub fn kind(&self) -> &QueryKind {
        &self.kind
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QueryError {
    Conversation(ConversationError),
    Unsupported,
    Disconnected,
    InvalidResponse,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Completion {
    Applied,
    Ignored,
    Rejected,
}

impl Chat {
    pub fn resolve(&mut self) -> Option<Query> {
        if self.descriptor.is_some() {
            return None;
        }
        self.begin(QueryKind::Resolve {
            binding: self.binding,
        })
    }
    pub fn read(&mut self, kind: ReadKind) -> Option<Query> {
        let descriptor = self.descriptor.as_ref()?;
        let cursor = match kind {
            ReadKind::Latest => None,
            ReadKind::Older => Some(self.history.older_cursor()?.clone()),
        };
        let current = (kind == ReadKind::Latest)
            .then_some(self.history.latest.as_ref())
            .flatten();
        let request = ConversationReadRequest {
            address: descriptor.address.clone(),
            binding: self.binding,
            cursor,
            limits: ConversationLimits::default(),
            since_revision: current.map(|page| page.revision),
            since_generation: current.map(|page| page.history_generation),
        };
        self.begin(QueryKind::Read { request, kind })
    }
    fn begin(&mut self, kind: QueryKind) -> Option<Query> {
        if self.retired || self.mode != Mode::Chat || self.pending.is_some() {
            return None;
        }
        let query = Query {
            id: Uuid::new_v4(),
            epoch: self.epoch,
            kind,
        };
        self.pending = Some(query.clone());
        self.error = None;
        Some(query)
    }
    pub(crate) fn take(&mut self, query: &Query) -> bool {
        if self.mode != Mode::Chat
            || query.epoch != self.epoch
            || self.pending.as_ref() != Some(query)
        {
            return false;
        }
        self.pending = None;
        true
    }
    pub fn fail(&mut self, query: &Query, error: QueryError) -> Completion {
        if !self.take(query) {
            return Completion::Ignored;
        }
        self.record_error(error);
        Completion::Applied
    }
    pub(crate) fn record_error(&mut self, error: QueryError) {
        if matches!(
            error,
            QueryError::Conversation(
                ConversationError::HistoryChanged | ConversationError::BindingChanged
            )
        ) {
            self.history = Default::default();
            self.scroll = 0;
        }
        if error == QueryError::Conversation(ConversationError::BindingChanged) {
            self.descriptor = None;
        }
        self.error = Some(error);
    }
}
