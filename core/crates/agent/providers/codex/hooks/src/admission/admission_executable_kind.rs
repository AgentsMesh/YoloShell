/// Selected by the trusted caller's field or process role, never deserialized
/// from executable evidence and never inferred from a path or its size.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodexExecutableKind {
    Runtime,
    Provider,
}

impl CodexExecutableKind {
    pub(super) fn accepts_length(self, bytes: u64) -> bool {
        bytes > 0
            && (self == Self::Provider || bytes <= yolo_build_image::MAX_EXECUTABLE_IMAGE_BYTES)
    }
}
