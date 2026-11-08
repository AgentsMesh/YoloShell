use crate::control::Control;
use crate::{ControlIntent, Options, RelayError, Status, authority::Authority, constants::*};
use tokio::time::{Instant, timeout};
use yolo_hub_client::{Client, ClientError};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{LeaseMode, LeaseRequest, PublicError};

impl Control {
    pub async fn connect(options: Options) -> Result<Self, RelayError> {
        options
            .principal
            .validate()
            .map_err(|_| RelayError::InvalidOptions)?;
        let client = Client::connect_local(
            &options.socket,
            options.entry,
            Instant::now() + REQUEST_TIMEOUT,
        )
        .await?;
        Ok(Self {
            client: Some(client),
            lane_client: None,
            input_busy: false,
            options,
            authority: None,
            cleanup_grant: None,
            initial: Status::ReadOnly { holder_id: None },
        })
    }

    pub async fn start(&mut self) -> Result<(), RelayError> {
        let intent = self.options.intent;
        if intent == ControlIntent::Observe {
            self.initial = Status::ReadOnly {
                holder_id: self.holder().await,
            };
            return Ok(());
        }
        let mode = if intent == ControlIntent::TakeControl {
            let mut client = self
                .client
                .take()
                .ok_or(RelayError::Client(ClientError::Poisoned))?;
            let observed = client
                .call(
                    self.origin(),
                    Command::ObserveLease {
                        target: self.options.target,
                    },
                    Instant::now() + REQUEST_TIMEOUT,
                )
                .await?;
            self.client = Some(client);
            match observed {
                Reply::LeaseObservation { lease: Some(lease) } => LeaseMode::TakeControl {
                    observed_generation: lease.fields().generation,
                },
                Reply::LeaseObservation { lease: None } => LeaseMode::Acquire {},
                _ => return Err(RelayError::InvalidReply),
            }
        } else {
            LeaseMode::Acquire {}
        };
        let sent = Instant::now();
        let mut client = self
            .client
            .take()
            .ok_or(RelayError::Client(yolo_hub_client::ClientError::Poisoned))?;
        let result = client
            .call(
                self.origin(),
                Command::Lease {
                    request: LeaseRequest {
                        request_id: uuid::Uuid::new_v4(),
                        target: self.options.target,
                        principal: self.options.principal,
                        mode,
                    },
                },
                sent + REQUEST_TIMEOUT,
            )
            .await;
        self.client = Some(client);
        match result {
            Ok(Reply::Lease { grant }) => {
                self.cleanup_grant = Some(grant);
                self.authority = Authority::received(grant, sent, Instant::now());
                if self.authority.is_some() {
                    if let Some(size) = self.options.size {
                        self.resize(size).await?;
                    }
                }
                self.initial = if self.authority.is_some() {
                    Status::ControlAcquired
                } else {
                    Status::ReadOnly { holder_id: None }
                };
                Ok(())
            }
            Err(ClientError::Remote(PublicError::Conflict)) => {
                self.initial = Status::ReadOnly {
                    holder_id: self.holder().await,
                };
                Ok(())
            }
            Err(error) => Err(error.into()),
            _ => Err(RelayError::InvalidReply),
        }
    }

    async fn holder(&mut self) -> Option<String> {
        let mut client = self.client.take()?;
        let result = timeout(
            HOLDER_LOOKUP_TIMEOUT,
            client.call(
                self.origin(),
                Command::ObserveLease {
                    target: self.options.target,
                },
                Instant::now() + HOLDER_LOOKUP_TIMEOUT,
            ),
        )
        .await;
        if client.is_usable() {
            self.client = Some(client);
        }
        match result {
            Ok(Ok(Reply::LeaseObservation { lease: Some(lease) })) => {
                Some(lease.fields().holder_id.clone())
            }
            _ => None,
        }
    }
}
