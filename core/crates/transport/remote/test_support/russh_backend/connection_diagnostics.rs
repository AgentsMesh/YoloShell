use crate::remote::RemoteError;
use std::time::Duration;
use tokio::task::JoinError;

macro_rules! disconnected_handler {
    () => {
        async fn disconnected(
            &mut self,
            reason: russh::client::DisconnectReason<Self::Error>,
        ) -> Result<(), Self::Error> {
            match reason {
                russh::client::DisconnectReason::ReceivedDisconnect(info) => {
                    eprintln!("SSH client received disconnect: {:?}", info.reason_code);
                    Ok(())
                }
                russh::client::DisconnectReason::Error(error) => {
                    eprintln!("SSH client transport terminated: {error:?}");
                    Err(error)
                }
            }
        }
    };
}
pub(super) use disconnected_handler;

pub(super) fn driver_error(error: &RemoteError) {
    eprintln!("SSH channel cleanup escalation: driver_error={error:?}");
}

pub(super) fn join_error(error: &JoinError) {
    eprintln!("SSH channel cleanup escalation: join_error={error:?}");
}

pub(super) fn cleanup_timeout(timeout: Duration) {
    eprintln!("SSH channel cleanup escalation: timeout={timeout:?}");
}
