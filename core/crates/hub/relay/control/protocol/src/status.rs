use crate::{MAX_HOLDER_BYTES, MAX_STATUS_BYTES, ProtocolError, codec};
use capnp::message::Builder;
use yolo_native_relay_schema::relay_capnp::{frame, status};

pub use yolo_native_relay_schema::relay_capnp::StatusKind;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Status {
    pub kind: StatusKind,
    pub holder_id: Option<String>,
}

impl Status {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        if self
            .holder_id
            .as_ref()
            .is_some_and(|holder| holder.is_empty() || holder.len() > MAX_HOLDER_BYTES)
            || (self.holder_id.is_some()
                && !matches!(self.kind, StatusKind::ReadOnly | StatusKind::ControlLost))
        {
            return Err(ProtocolError::InvalidFrame);
        }
        Ok(())
    }
}

pub fn encode_status(value: &Status) -> Result<Vec<u8>, ProtocolError> {
    value.validate()?;
    let mut message = Builder::new_default();
    let mut status = message.init_root::<frame::Builder<'_>>().init_status();
    status.set_kind(value.kind);
    match &value.holder_id {
        Some(holder) => status.init_holder().set_id(holder.as_str()),
        None => status.init_holder().set_none(()),
    }
    let bytes = codec::encode(message)?;
    if bytes.len() > MAX_STATUS_BYTES {
        return Err(ProtocolError::FrameTooLarge);
    }
    Ok(bytes)
}

pub fn decode_status(bytes: &[u8]) -> Result<Status, ProtocolError> {
    if bytes.len() > MAX_STATUS_BYTES {
        return Err(ProtocolError::FrameTooLarge);
    }
    let message = codec::decode(bytes)?;
    let root = message
        .get_root::<frame::Reader<'_>>()
        .map_err(codec::invalid)?;
    let frame::Status(value) = root.which().map_err(codec::invalid)? else {
        return Err(ProtocolError::InvalidFrame);
    };
    if !root.has_status() {
        return Err(ProtocolError::InvalidFrame);
    }
    let value = value.map_err(codec::invalid)?;
    let holder_id = match value.get_holder().which().map_err(codec::invalid)? {
        status::holder::None(()) => None,
        status::holder::Id(holder) => Some(
            holder
                .map_err(codec::invalid)?
                .to_str()
                .map_err(codec::invalid)?
                .to_owned(),
        ),
    };
    let status = Status {
        kind: value.get_kind().map_err(codec::invalid)?,
        holder_id,
    };
    status.validate()?;
    Ok(status)
}
