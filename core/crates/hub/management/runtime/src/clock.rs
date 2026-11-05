use yolo_hub_terminal_protocol::PublicError;

pub(crate) fn wall_clock_ms() -> Result<u64, PublicError> {
    let elapsed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| PublicError::Internal)?;
    u64::try_from(elapsed.as_millis()).map_err(|_| PublicError::Internal)
}
