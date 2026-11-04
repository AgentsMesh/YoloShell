use super::bootstrap_gate::BootstrapChildSpec;
use anyhow::{Context, Result};
use base64::{Engine, engine::general_purpose::STANDARD_NO_PAD};

pub(crate) const ENCODE_CONTEXT: &str = "encode bootstrap child specification";
pub(crate) const DECODE_CONTEXT: &str = "decode bootstrap child specification";

pub(crate) fn encode(spec: &BootstrapChildSpec) -> Result<String> {
    Ok(STANDARD_NO_PAD.encode(yolo_capnp_codec::encode(spec).context(ENCODE_CONTEXT)?))
}

pub(crate) fn decode(value: &str) -> Result<BootstrapChildSpec> {
    yolo_capnp_codec::decode(&STANDARD_NO_PAD.decode(value).context(DECODE_CONTEXT)?)
        .context(DECODE_CONTEXT)
}
