use super::contract::{HookRequest, HookResponse, HookScope};
use super::profile::{HookCodec, HookIngressProfile};
use anyhow::{Context, Result, ensure};
use std::path::PathBuf;
use uuid::Uuid;
use yolo_local_unix::LocalUnixStream;

const INVALID: &str = "provider hook environment is invalid";
const STDIN: &str = "provider hook payload is unavailable on stdin";

pub struct ClientContext {
    socket: PathBuf,
    outbox: PathBuf,
    pub scope: HookScope,
    pub provider: &'static str,
}

impl ClientContext {
    pub fn from_environment(profile: &HookIngressProfile) -> Result<Option<Self>> {
        let Some(socket) = std::env::var_os(profile.socket_env) else {
            return Ok(None);
        };
        let outbox = std::env::var_os(profile.outbox_env)
            .map(PathBuf::from)
            .context(INVALID)?;
        let scope = HookScope {
            terminal_id: required_uuid(yolo_agent_core::constants::YOLO_TERMINAL_ID_ENV)?,
            runtime_epoch: required_uuid(yolo_agent_core::constants::YOLO_RUNTIME_EPOCH_ENV)?,
            agent: super::parent::candidate(profile.max_ancestors)?,
        };
        ensure!(
            PathBuf::from(&socket).is_absolute() && outbox.is_absolute() && scope.is_valid(),
            INVALID
        );
        Ok(Some(Self {
            socket: PathBuf::from(socket),
            outbox,
            scope,
            provider: profile.provider,
        }))
    }

    pub async fn deliver_pending(&self) -> Result<()> {
        let deadline = tokio::time::Instant::now() + super::constants::DELIVERY_BUDGET;
        while let Some(request) = super::outbox::next(&self.outbox, self.scope)? {
            loop {
                if tokio::time::Instant::now() >= deadline {
                    return Ok(());
                }
                if let Ok(Ok(Some(response))) = tokio::time::timeout_at(
                    deadline,
                    self.send(&request),
                )
                .await
                {
                    if response.acknowledges(&request) {
                        super::outbox::acknowledge(&self.outbox, self.scope, request.event_sequence)?;
                        break;
                    }
                }
                tokio::time::sleep_until(
                    (tokio::time::Instant::now() + super::constants::RETRY_DELAY).min(deadline),
                )
                .await;
            }
        }
        Ok(())
    }

    async fn send(&self, request: &HookRequest) -> Result<Option<HookResponse>> {
        let mut stream = LocalUnixStream::connect(&self.socket).await?;
        let peer = stream.peer_cred()?;
        // SAFETY: geteuid has no preconditions.
        ensure!(peer.uid() == unsafe { libc::geteuid() }, INVALID);
        super::contract::write_frame(&mut stream, request).await?;
        Ok(Some(super::contract::read_frame(&mut stream).await?))
    }
}

/// Entrypoint shared by every provider hook command: read the provider's stdin
/// payload, enqueue it durably, then make one bounded delivery attempt.
pub async fn run(codec: &dyn HookCodec) -> Result<()> {
    let profile = &codec.profile();
    let mut input = Vec::new();
    use std::io::Read;
    std::io::stdin().read_to_end(&mut input).context(STDIN)?;
    let Ok(payload) = serde_json::from_slice::<serde_json::Value>(&input) else {
        // An unparseable provider payload carries no durable fact; enqueueing it
        // would head-of-line block every later event behind a permanent reject.
        return Ok(());
    };
    if !payload.is_object()
        || serde_json::to_vec(&payload)
            .map_or(true, |encoded| encoded.len() > super::contract::MAX_PAYLOAD_BYTES)
    {
        // The server rejects an over-cap payload forever; dropping it here keeps
        // the durable queue drainable. Lifecycle payloads are far below the cap.
        return Ok(());
    }
    if !matches!(codec.decode(&payload), Ok(Some(_))) {
        // The server would reject this payload forever; dropping it here keeps
        // the durable queue drainable. Lifecycle payloads decode cleanly.
        return Ok(());
    }
    let Some(client) = ClientContext::from_environment(profile)? else {
        return Ok(());
    };
    super::outbox::enqueue(
        &client.outbox,
        client.scope,
        profile.provider,
        Uuid::new_v4(),
        payload,
    )?;
    client.deliver_pending().await
}

fn required_uuid(name: &str) -> Result<Uuid> {
    Uuid::parse_str(&std::env::var(name).context(INVALID)?).context(INVALID)
}
