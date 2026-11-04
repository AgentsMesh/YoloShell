use crate::constants::*;
use crate::{
    InviteClaims, InviteDial, InviteError, InvitePolicy, SignedInvite, SignedInviteFields,
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use std::num::NonZeroU16;
use uuid::Uuid;
use yolo_hub_delegation::{PeerIdentity, ED25519_SIGNATURE_BYTES};
use yolo_hub_identity::HubId;

const POLICY_MASK: u8 = POLICY_EXPORT_LOCAL | POLICY_IMPORT_PEER;

pub fn encode(invite: &SignedInvite) -> Result<String, InviteError> {
    let claims = invite.claims();
    let host = claims.dial.host.as_bytes();
    if host.len() > MAX_HOST_BYTES || invite.signature().len() != ED25519_SIGNATURE_BYTES {
        return Err(InviteError::Code);
    }
    let mut payload = Vec::with_capacity(MAX_PAYLOAD_BYTES);
    payload.push(CODE_VERSION);
    payload.extend_from_slice(claims.invite_id.as_bytes());
    payload.extend_from_slice(claims.issuer.hub().as_uuid().as_bytes());
    payload.extend_from_slice(claims.issuer.key());
    payload.push(host.len() as u8);
    payload.extend_from_slice(host);
    payload.extend_from_slice(&claims.dial.port.get().to_be_bytes());
    payload.extend_from_slice(&claims.not_before_ms.to_be_bytes());
    payload.extend_from_slice(&claims.expires_at_ms.to_be_bytes());
    payload.push(policy_bits(claims.policy));
    payload.extend_from_slice(invite.signature());
    Ok(format!("{CODE_PREFIX}{}", URL_SAFE_NO_PAD.encode(payload)))
}

pub fn decode(code: &str) -> Result<SignedInvite, InviteError> {
    let encoded = code.trim().strip_prefix(CODE_PREFIX).ok_or(InviteError::Code)?;
    if encoded.is_empty() || encoded.len() > MAX_CODE_BYTES {
        return Err(InviteError::Code);
    }
    let bytes = URL_SAFE_NO_PAD
        .decode(encoded)
        .map_err(|_| InviteError::Code)?;
    let mut reader = Reader::new(&bytes);
    if reader.u8()? != CODE_VERSION {
        return Err(InviteError::Code);
    }
    let invite_id = Uuid::from_bytes(reader.array()?);
    let hub = HubId::from_uuid(Uuid::from_bytes(reader.array()?)).map_err(|_| InviteError::Code)?;
    let issuer = PeerIdentity::new(hub, reader.array()?).map_err(|_| InviteError::Code)?;
    let host_len = usize::from(reader.u8()?);
    if host_len == 0 || host_len > MAX_HOST_BYTES {
        return Err(InviteError::Code);
    }
    let host = std::str::from_utf8(reader.take(host_len)?)
        .map_err(|_| InviteError::Code)?
        .to_owned();
    let port = NonZeroU16::new(u16::from_be_bytes(reader.array()?)).ok_or(InviteError::Code)?;
    let not_before_ms = u64::from_be_bytes(reader.array()?);
    let expires_at_ms = u64::from_be_bytes(reader.array()?);
    let bits = reader.u8()?;
    if bits & !POLICY_MASK != 0 {
        return Err(InviteError::Code);
    }
    let policy = InvitePolicy {
        export_local: bits & POLICY_EXPORT_LOCAL != 0,
        import_peer: bits & POLICY_IMPORT_PEER != 0,
    };
    let signature = reader.take(ED25519_SIGNATURE_BYTES)?.to_vec();
    if !reader.is_empty() {
        return Err(InviteError::Code);
    }
    SignedInviteFields {
        claims: InviteClaims {
            invite_id,
            issuer,
            dial: InviteDial { host, port },
            not_before_ms,
            expires_at_ms,
            policy,
        },
        signature,
    }
    .try_into()
}

fn policy_bits(policy: InvitePolicy) -> u8 {
    (if policy.export_local { POLICY_EXPORT_LOCAL } else { 0 })
        | (if policy.import_peer {
            POLICY_IMPORT_PEER
        } else {
            0
        })
}

struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], InviteError> {
        let end = self.offset.checked_add(length).ok_or(InviteError::Code)?;
        let slice = self.bytes.get(self.offset..end).ok_or(InviteError::Code)?;
        self.offset = end;
        Ok(slice)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], InviteError> {
        self.take(N)?
            .try_into()
            .map_err(|_| InviteError::Code)
    }

    fn u8(&mut self) -> Result<u8, InviteError> {
        Ok(self.array::<1>()?[0])
    }

    fn is_empty(&self) -> bool {
        self.offset == self.bytes.len()
    }
}
