use crate::pty_custody::record::CUSTODY_RECORD_SCHEMA_VERSION;
use crate::pty_custody::{CustodyDurableRecord, CustodyDurableState, CustodyRuntimeIdentity};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _, ser::Error as _};

const UNSUPPORTED_RECORD_DETAIL: &str = "unsupported durable PTY custody record schema";

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/wire_contract_tests.rs"]
mod tests;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RecordWire {
    schema_version: u16,
    runtime: CustodyRuntimeIdentity,
    state: CustodyDurableState,
}

impl Serialize for CustodyDurableRecord {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if self.schema_version != CUSTODY_RECORD_SCHEMA_VERSION {
            return Err(S::Error::custom(UNSUPPORTED_RECORD_DETAIL));
        }
        RecordWire {
            schema_version: self.schema_version,
            runtime: self.runtime.clone(),
            state: self.state.clone(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for CustodyDurableRecord {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = RecordWire::deserialize(deserializer)?;
        if wire.schema_version != CUSTODY_RECORD_SCHEMA_VERSION {
            return Err(D::Error::custom(UNSUPPORTED_RECORD_DETAIL));
        }
        Ok(Self {
            schema_version: wire.schema_version,
            runtime: wire.runtime,
            state: wire.state,
        })
    }
}
