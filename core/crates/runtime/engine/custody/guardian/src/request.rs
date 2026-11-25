use super::settlement::GuardianState;
use crate::namespace_lease::NamespaceLease;
use crate::paths::Paths;
use crate::process::MacosSecondaryCustody;
use crate::protocol::RequestEnvelope;
use crate::pty_custody::CustodyUnixStream;
use anyhow::Result;

const METHOD_DETAIL: &str = "unsupported PTY custody Guardian method";

pub(super) async fn handle(
    namespace: &NamespaceLease,
    paths: &Paths,
    custody: &MacosSecondaryCustody,
    state: &mut GuardianState,
    stream: tokio::net::UnixStream,
) -> Result<()> {
    let mut stream = CustodyUnixStream::from_accepted(stream)?;
    let opening: RequestEnvelope = stream.read().await?;
    namespace.verify()?;
    if opening.method == super::protocol::GUARDIAN_PREPARE_METHOD {
        return super::prepare::handle(namespace, paths, custody, state, stream, opening).await;
    }
    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    if opening.method == super::recovery_protocol::GUARDIAN_RECOVERY_METHOD {
        return super::recovery_server::handle(namespace, paths, custody, state, stream, opening)
            .await;
    }
    anyhow::bail!(METHOD_DETAIL)
}
