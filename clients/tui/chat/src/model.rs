use crate::{Composer, Query, QueryError, history::History};
use uuid::Uuid;
use yolo_conversation_protocol::{ConversationBinding, ConversationDescriptor, ConversationError};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Mode {
    #[default]
    Terminal,
    Chat,
}

/// Bound to one execution. A new AgentRun gets a separate view and draft.
pub struct Chat {
    pub(crate) binding: ConversationBinding,
    pub(crate) mode: Mode,
    pub(crate) retired: bool,
    pub(crate) epoch: Uuid,
    pub(crate) descriptor: Option<ConversationDescriptor>,
    pub(crate) pending: Option<Query>,
    pub(crate) history: History,
    pub(crate) error: Option<QueryError>,
    pub(crate) scroll: usize,
    pub(crate) rendered: std::cell::RefCell<Option<crate::render::Rendered>>,
    pub composer: Composer,
}

impl Chat {
    pub fn new(binding: ConversationBinding) -> Result<Self, ConversationError> {
        binding.validate()?;
        Ok(Self {
            binding,
            mode: Mode::Terminal,
            retired: false,
            epoch: Uuid::new_v4(),
            descriptor: None,
            pending: None,
            history: History::default(),
            error: None,
            scroll: 0,
            composer: Composer::default(),
            rendered: Default::default(),
        })
    }
    pub fn binding(&self) -> ConversationBinding {
        self.binding
    }
    pub fn mode(&self) -> Mode {
        self.mode
    }
    pub fn retired(&self) -> bool {
        self.retired
    }
    /// Retire the execution's query epoch while retaining its history and unsent draft.
    pub fn retire(&mut self) {
        if !self.retired {
            self.retired = true;
            self.epoch = Uuid::new_v4();
            self.pending = None;
        }
    }
    pub fn set_mode(&mut self, mode: Mode) {
        if mode != self.mode {
            self.mode = mode;
            self.epoch = Uuid::new_v4();
            self.pending = None;
            if self.error == Some(QueryError::Unsupported) {
                self.error = None;
            }
        }
    }
    pub fn descriptor(&self) -> Option<&ConversationDescriptor> {
        self.descriptor.as_ref()
    }
    pub fn error(&self) -> Option<QueryError> {
        self.error
    }
    pub fn pending(&self) -> Option<&Query> {
        self.pending.as_ref()
    }
    pub fn page_count(&self) -> usize {
        self.history.page_count()
    }
    pub fn entries(&self) -> Vec<&yolo_conversation_protocol::ConversationEntry> {
        self.history.entries()
    }
    pub fn older_available(&self) -> bool {
        self.history.older_cursor().is_some()
    }
    pub fn scroll(&mut self, rows: usize, older: bool) {
        self.scroll = if older {
            self.scroll.saturating_add(rows).min(crate::MAX_RENDER_ROWS)
        } else {
            self.scroll.saturating_sub(rows)
        };
    }
    pub fn follow_tail(&mut self) {
        self.scroll = 0;
    }
    pub fn scroll_offset(&self) -> usize {
        self.scroll
    }
}
