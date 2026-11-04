use serde::{Deserialize, Serialize};

pub const INVALID_PIPE_HANDLE: u64 = 0;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CustodyDescriptorSet {
    MacosProcessCapability {
        reader_handle: u64,
        writer_handle: u64,
    },
    LinuxContainment,
    ReleasedSuccessor,
}

impl CustodyDescriptorSet {
    pub const fn macos_process_capability(reader_handle: u64, writer_handle: u64) -> Self {
        Self::MacosProcessCapability {
            reader_handle,
            writer_handle,
        }
    }

    pub const fn descriptor_count(self) -> usize {
        match self {
            Self::MacosProcessCapability { .. } => super::constants::MACOS_DESCRIPTOR_COUNT,
            Self::LinuxContainment => super::constants::LINUX_DESCRIPTOR_COUNT,
            Self::ReleasedSuccessor => super::constants::RELEASED_SUCCESSOR_DESCRIPTOR_COUNT,
        }
    }

    pub const fn is_well_formed(self) -> bool {
        match self {
            Self::MacosProcessCapability {
                reader_handle,
                writer_handle,
            } => {
                reader_handle != INVALID_PIPE_HANDLE
                    && writer_handle != INVALID_PIPE_HANDLE
                    && reader_handle != writer_handle
            }
            Self::LinuxContainment | Self::ReleasedSuccessor => true,
        }
    }
}
