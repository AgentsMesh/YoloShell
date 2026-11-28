use super::append::AppendStageValue;
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppendStage {
    BeforeWrite,
    AfterRecordSync,
    AfterCursorCommit,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct AppendFault {
    pub(super) stage: AppendStageValue,
    pub(super) remaining: usize,
}

impl Journal {
    pub fn remaining_payload_capacity(&self) -> usize {
        let fixed = self.disk_bytes.saturating_add(RECORD_HEADER_BYTES as u64);
        usize::try_from(self.max_disk_bytes.saturating_sub(fixed)).unwrap_or(usize::MAX)
    }

    pub fn inject_append_failure(&mut self, stage: AppendStage) {
        self.inject_append_failures(stage, 1);
    }

    pub fn inject_append_failures(&mut self, stage: AppendStage, count: usize) {
        assert!(count > 0);
        self.append_fault = Some(AppendFault {
            stage: stage.into(),
            remaining: count,
        });
    }

    pub fn clear_append_failures(&mut self) {
        self.append_fault = None;
    }

    pub(super) fn append_with_test_hook<F>(
        &mut self,
        payload: &[u8],
        mut hook: F,
    ) -> Result<OutputChunk, JournalError>
    where
        F: FnMut(AppendStage) -> Result<(), JournalError>,
    {
        self.append_with_hook(payload, |stage| hook(stage.into()))
    }
}

impl From<AppendStage> for AppendStageValue {
    fn from(value: AppendStage) -> Self {
        match value {
            AppendStage::BeforeWrite => Self::BeforeWrite,
            AppendStage::AfterRecordSync => Self::AfterRecordSync,
            AppendStage::AfterCursorCommit => Self::AfterCursorCommit,
        }
    }
}

impl From<AppendStageValue> for AppendStage {
    fn from(value: AppendStageValue) -> Self {
        match value {
            AppendStageValue::BeforeWrite => Self::BeforeWrite,
            AppendStageValue::AfterRecordSync => Self::AfterRecordSync,
            AppendStageValue::AfterCursorCommit => Self::AfterCursorCommit,
        }
    }
}
