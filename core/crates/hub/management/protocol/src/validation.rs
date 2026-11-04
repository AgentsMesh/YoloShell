use crate::{FIRST_CONFIGURATION_REVISION, INVALID_CONFIGURATION_CHANGE};
use serde::{Deserialize, Deserializer, de::Error};

pub(crate) fn revision<'de, D: Deserializer<'de>>(input: D) -> Result<u64, D::Error> {
    let value = u64::deserialize(input)?;
    if value < FIRST_CONFIGURATION_REVISION {
        return Err(D::Error::custom(INVALID_CONFIGURATION_CHANGE));
    }
    Ok(value)
}
