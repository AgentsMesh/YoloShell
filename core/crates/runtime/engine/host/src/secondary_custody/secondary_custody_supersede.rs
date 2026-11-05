use super::*;

const MONITOR_UNAVAILABLE_DETAIL: &str = "PTY custody monitor is unavailable";
const CONTROL_TIMEOUT_DETAIL: &str = "PTY custody control acknowledgement timed out";

impl SecondaryCustodyLease {
    pub(super) async fn supersede(
        &self,
        successor: &SecondaryCustodyLease,
    ) -> Result<(), WorkerRpcError> {
        tokio::time::timeout(
            crate::session_timing::PTY_CAPABILITY_SETTLEMENT_TIMEOUT,
            self.supersede_inner(successor),
        )
        .await
        .map_err(|_| WorkerRpcError::transport(CONTROL_TIMEOUT_DETAIL))?
    }

    async fn supersede_inner(
        &self,
        successor: &SecondaryCustodyLease,
    ) -> Result<(), WorkerRpcError> {
        let mut states = self.subscribe_state();
        loop {
            let state = *states.borrow_and_update();
            match state {
                SecondaryCustodyState::Released | SecondaryCustodyState::Superseded => {
                    return Ok(());
                }
                SecondaryCustodyState::ReleaseAckPending
                | SecondaryCustodyState::Disconnected
                | SecondaryCustodyState::Failed => {
                    await_state_change(&mut states).await?;
                }
                SecondaryCustodyState::Activating | SecondaryCustodyState::Ready => break,
                _ => return Err(unavailable()),
            }
        }
        let (completion, completed) = tokio::sync::oneshot::channel();
        let command = SecondaryCustodyCommand::Supersede {
            successor: successor.offer.clone(),
            successor_generation: successor.controller_generation,
            completion,
        };
        if self.monitor_commands.send(command).await.is_err() {
            return self.await_retirement_or(unavailable()).await;
        }
        match completed.await {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => self.await_retirement_or(error).await,
            Err(_) => self.await_retirement_or(unavailable()).await,
        }
    }

    async fn await_retirement_or(&self, error: WorkerRpcError) -> Result<(), WorkerRpcError> {
        let mut states = self.subscribe_state();
        loop {
            let state = *states.borrow_and_update();
            match state {
                SecondaryCustodyState::Released | SecondaryCustodyState::Superseded => {
                    return Ok(());
                }
                SecondaryCustodyState::Activating
                | SecondaryCustodyState::Ready
                | SecondaryCustodyState::ReleaseAckPending
                | SecondaryCustodyState::Disconnected
                | SecondaryCustodyState::Failed => {
                    if states.changed().await.is_err() {
                        return Err(error);
                    }
                }
                _ => return Err(error),
            }
        }
    }
}

async fn await_state_change(
    states: &mut tokio::sync::watch::Receiver<SecondaryCustodyState>,
) -> Result<(), WorkerRpcError> {
    states.changed().await.map_err(|_| unavailable())
}

fn unavailable() -> WorkerRpcError {
    WorkerRpcError::transport(MONITOR_UNAVAILABLE_DETAIL)
}
