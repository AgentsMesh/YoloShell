use super::{FaultOutput, ReaderBackedPtyOutput};

impl FaultOutput {
    pub(crate) fn reader(&self) -> Option<&ReaderBackedPtyOutput> {
        match self {
            Self::Reader(output) => Some(output),
            Self::Exclusive(_) => None,
        }
    }
}
