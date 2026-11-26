use crate::constants::AUTHORITY_CHECK_INTERVAL;
use std::{
    future::Future,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use yolo_hub_client_access_registry::ClientAdmission;
use yolo_hub_terminal_protocol::PublicError;

pub(crate) fn now_ms() -> Result<u64, PublicError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| PublicError::Internal)?
        .as_millis()
        .try_into()
        .map_err(|_| PublicError::Internal)
}

pub(crate) fn verify(admission: &ClientAdmission) -> Result<(), PublicError> {
    if admission.is_current(now_ms()?) {
        Ok(())
    } else {
        Err(PublicError::Unauthorized)
    }
}

/// Retain one operation future while watching its authenticated authority.
/// Revocation only retires the client work/stream; it never kills an origin PTY.
/// A caller losing a dispatched mutation must preserve its uncertain outcome.
pub async fn while_authorized<T>(
    admission: &ClientAdmission,
    operation: impl Future<Output = Result<T, PublicError>>,
) -> Result<T, PublicError> {
    tokio::pin!(operation);
    loop {
        verify(admission)?;
        let remaining = admission.grant().expires_at_ms.saturating_sub(now_ms()?);
        let wait = Duration::from_millis(remaining).min(AUTHORITY_CHECK_INTERVAL);
        tokio::select! {
            biased;
            _ = admission.revoked() => return Err(PublicError::Unauthorized),
            _ = tokio::time::sleep(wait) => {},
            result = &mut operation => {
                verify(admission)?;
                return result;
            }
        }
    }
}
