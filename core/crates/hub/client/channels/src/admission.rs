use crate::ControlError;
use uuid::Uuid;

pub(crate) fn admit(current: Option<Uuid>, requested: Option<Uuid>) -> Result<(), ControlError> {
    let current = current.ok_or(ControlError::NotConnected)?;
    if requested != Some(current) {
        return Err(ControlError::StaleGeneration);
    }
    Ok(())
}
