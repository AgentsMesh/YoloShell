use russh::keys::{HashAlg, agent::AgentIdentity};
use tokio::sync::oneshot;

/// Offers an enrolled public key, then sends the unsigned authentication bytes.
pub struct Unsigned {
    pub offered: Option<oneshot::Sender<()>>,
    pub release: Option<oneshot::Receiver<()>>,
}

impl russh::Signer for Unsigned {
    type Error = anyhow::Error;
    async fn auth_sign(
        &mut self,
        _: &AgentIdentity,
        _: Option<HashAlg>,
        to_sign: Vec<u8>,
    ) -> Result<Vec<u8>, Self::Error> {
        self.offered.take().unwrap().send(()).unwrap();
        self.release.take().unwrap().await.unwrap();
        Ok(to_sign)
    }
}
