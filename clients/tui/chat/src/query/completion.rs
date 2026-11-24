use crate::{Chat, Completion, Query, QueryError, QueryKind, ReadKind};
use yolo_conversation_protocol::{ConversationDescriptor, ConversationReadResult};

impl Chat {
    pub fn resolved(&mut self, query: &Query, descriptor: ConversationDescriptor) -> Completion {
        if !self.take(query) {
            return Completion::Ignored;
        }
        if !matches!(query.kind, QueryKind::Resolve { .. })
            || descriptor.validate_for(self.binding).is_err()
        {
            self.record_error(QueryError::InvalidResponse);
            return Completion::Rejected;
        }
        self.descriptor = Some(descriptor);
        self.error = None;
        Completion::Applied
    }
    pub fn loaded(&mut self, query: &Query, result: ConversationReadResult) -> Completion {
        if !self.take(query) {
            return Completion::Ignored;
        }
        let QueryKind::Read { request, kind } = &query.kind else {
            self.record_error(QueryError::InvalidResponse);
            return Completion::Rejected;
        };
        if result.validate_for(request).is_err() {
            self.record_error(QueryError::InvalidResponse);
            return Completion::Rejected;
        }
        if let ConversationReadResult::Page { page } = result {
            if self.descriptor.as_ref() != Some(&page.conversation) {
                self.record_error(QueryError::InvalidResponse);
                return Completion::Rejected;
            }
            match kind {
                ReadKind::Latest => {
                    if self.history.latest(page) {
                        self.scroll = 0;
                    }
                }
                ReadKind::Older => self.history.prepend(page),
            }
        }
        self.error = None;
        Completion::Applied
    }
}
