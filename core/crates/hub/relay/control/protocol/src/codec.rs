use crate::{ProtocolError, Request, Response, capnp_request, capnp_response, constants::*};
use capnp::message::{Builder, HeapAllocator, Reader, ReaderOptions};
use capnp::serialize::{OwnedSegments, read_message, write_message_to_words};
use yolo_native_relay_schema::relay_capnp::frame;

pub fn encode_request(request: &Request) -> Result<Vec<u8>, ProtocolError> {
    request.validate()?;
    let mut message = Builder::new_default();
    capnp_request::encode(
        message.init_root::<frame::Builder<'_>>().init_request(),
        request,
    );
    encode(message)
}
pub fn decode_request(payload: &[u8]) -> Result<Request, ProtocolError> {
    let message = decode(payload)?;
    match message
        .get_root::<frame::Reader<'_>>()
        .map_err(invalid)?
        .which()
        .map_err(invalid)?
    {
        frame::Request(request) => capnp_request::decode(request.map_err(invalid)?),
        _ => Err(ProtocolError::InvalidFrame),
    }
}
pub fn encode_response(response: &Response) -> Result<Vec<u8>, ProtocolError> {
    response.validate()?;
    let mut message = Builder::new_default();
    capnp_response::encode(
        message.init_root::<frame::Builder<'_>>().init_response(),
        response,
    )?;
    encode(message)
}
pub fn decode_response(payload: &[u8]) -> Result<Response, ProtocolError> {
    let message = decode(payload)?;
    match message
        .get_root::<frame::Reader<'_>>()
        .map_err(invalid)?
        .which()
        .map_err(invalid)?
    {
        frame::Response(response) => capnp_response::decode(response.map_err(invalid)?),
        _ => Err(ProtocolError::InvalidFrame),
    }
}
pub(crate) fn encode(message: Builder<HeapAllocator>) -> Result<Vec<u8>, ProtocolError> {
    let bytes = write_message_to_words(&message);
    bounded(&bytes)?;
    Ok(bytes)
}
pub(crate) fn decode(payload: &[u8]) -> Result<Reader<OwnedSegments>, ProtocolError> {
    bounded(payload)?;
    let mut bytes = payload;
    let mut options = ReaderOptions::new();
    options
        .traversal_limit_in_words(Some(MAX_TRAVERSAL_WORDS))
        .nesting_limit(MAX_NESTING);
    let message = read_message(&mut bytes, options).map_err(invalid)?;
    if !bytes.is_empty() {
        return Err(ProtocolError::InvalidFrame);
    }
    Ok(message)
}
fn bounded(payload: &[u8]) -> Result<(), ProtocolError> {
    if payload.is_empty() {
        return Err(ProtocolError::EmptyFrame);
    }
    if payload.len() > MAX_FRAME_BYTES {
        return Err(ProtocolError::FrameTooLarge);
    }
    Ok(())
}
pub(crate) fn invalid<E>(_: E) -> ProtocolError {
    ProtocolError::InvalidFrame
}
