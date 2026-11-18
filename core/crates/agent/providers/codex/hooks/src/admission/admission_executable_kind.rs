/// Selected by the trusted caller's field or process role, never deserialized
/// from executable evidence and never inferred from a path or its size.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodexExecutableKind {
    Runtime,
    Provider,
}

impl CodexExecutableKind {
    pub(super) const fn maximum_bytes(self) -> u64 {
        match self {
            Self::Runtime => yolo_build_image::MAX_EXECUTABLE_IMAGE_BYTES,
            Self::Provider => crate::codex_hooks::MAX_PROVIDER_EXECUTABLE_BYTES,
        }
    }
}
