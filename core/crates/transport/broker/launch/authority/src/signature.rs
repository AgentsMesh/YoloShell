use crate::{constants::*, wire::*};
use anyhow::{Result, ensure};
use yolo_hub_state::HubState;

fn message(statement: &Statement) -> Result<Vec<u8>> {
    let mut bytes = DOMAIN.to_vec();
    bytes.extend(yolo_capnp_codec::canonical(statement)?);
    ensure!(bytes.len() <= MAX_FRAME_BYTES, INVALID);
    Ok(bytes)
}
pub(crate) fn sign(owner: &HubState, statement: Statement) -> Result<SignedStatement> {
    ensure!(
        owner.peer_identity() == statement.scope.owner,
        SCOPE_MISMATCH
    );
    let signature = owner.sign(&message(&statement)?)?.as_ref().to_vec();
    Ok(SignedStatement {
        statement,
        signature,
    })
}
pub(crate) fn verify(signed: &SignedStatement, expected: &Request) -> Result<()> {
    ensure!(&signed.statement.request == expected, INVALID);
    signed.statement.scope.verify()?;
    ring::signature::UnparsedPublicKey::new(
        &ring::signature::ED25519,
        signed.statement.scope.owner.key(),
    )
    .verify(&message(&signed.statement)?, &signed.signature)
    .map_err(|_| anyhow::anyhow!(INVALID))
}
