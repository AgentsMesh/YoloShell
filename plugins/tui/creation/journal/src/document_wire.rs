use crate::{CreationIntent, PendingFinalization, constants::*, record::Document};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error, ser::SerializeStruct};
use yolo_hub_identity::HubId;
use yolo_paths::identity::DurableObjectIdentity;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Current {
    schema_version: u16,
    entry_hub: HubId,
    directory: DurableObjectIdentity,
    revision: u64,
    pending: Vec<PendingFinalization>,
    intents: Vec<CreationIntent>,
}

impl<'de> Deserialize<'de> for Document {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Current::deserialize(deserializer)?;
        if value.schema_version != JOURNAL_SCHEMA_VERSION {
            return Err(D::Error::custom(INVALID_RECORD));
        }
        Ok(Self {
            schema_version: value.schema_version,
            entry_hub: value.entry_hub,
            directory: value.directory,
            revision: value.revision,
            pending: value.pending,
            intents: value.intents,
        })
    }
}

impl Serialize for Document {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if self.schema_version != JOURNAL_SCHEMA_VERSION {
            return Err(serde::ser::Error::custom(INVALID_RECORD));
        }
        let mut output = serializer.serialize_struct(DEBUG_DOCUMENT, CURRENT_FIELD_COUNT)?;
        output.serialize_field(FIELD_SCHEMA, &self.schema_version)?;
        output.serialize_field(FIELD_ENTRY, &self.entry_hub)?;
        output.serialize_field(FIELD_DIRECTORY, &self.directory)?;
        output.serialize_field(FIELD_REVISION, &self.revision)?;
        output.serialize_field(FIELD_PENDING, &self.pending)?;
        output.serialize_field(FIELD_INTENTS, &self.intents)?;
        output.end()
    }
}
