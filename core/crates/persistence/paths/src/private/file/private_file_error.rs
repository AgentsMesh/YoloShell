pub fn io_kind(error: &anyhow::Error) -> Option<std::io::ErrorKind> {
    error.chain().find_map(|source| {
        source
            .downcast_ref::<std::io::Error>()
            .map(std::io::Error::kind)
    })
}

pub(crate) fn is_creation_contention(error: &anyhow::Error) -> bool {
    matches!(
        io_kind(error),
        Some(std::io::ErrorKind::AlreadyExists | std::io::ErrorKind::WouldBlock)
    )
}
