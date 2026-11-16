use crate::protocol::{ProtocolError, delivery};
use crate::relay::RelayError;
use crate::relay::constants::{INPUT_BUFFER_BYTES, PENDING_INPUT_BYTES};
use crate::relay::control::{ControlChannel, client_error};
use crate::relay::lease::LeaseController;
use crate::worker::InputResult;
use std::collections::VecDeque;
use uuid::Uuid;

#[derive(Clone, Debug, Eq, PartialEq)]
struct PendingDelivery {
    input_id: Uuid,
    data: Vec<u8>,
}

pub(in crate::relay::session) struct PendingInputState {
    bytes: VecDeque<u8>,
    closed: bool,
    delivery: Option<PendingDelivery>,
}

impl PendingInputState {
    pub(in crate::relay::session) fn new() -> Self {
        Self {
            bytes: VecDeque::with_capacity(PENDING_INPUT_BYTES),
            closed: false,
            delivery: None,
        }
    }

    pub(super) fn append(&mut self, data: &[u8]) {
        debug_assert!(data.len() <= self.read_capacity(data.len()));
        self.bytes.extend(data);
    }

    pub(super) fn close(&mut self) {
        self.closed = true;
    }

    pub(super) fn has_data(&self) -> bool {
        !self.bytes.is_empty()
    }

    pub(super) fn is_closed(&self) -> bool {
        self.closed
    }

    pub(super) fn read_capacity(&self, requested: usize) -> usize {
        if self.closed {
            return 0;
        }
        PENDING_INPUT_BYTES
            .saturating_sub(self.bytes.len())
            .min(requested)
    }

    fn next_delivery(&mut self) -> Option<PendingDelivery> {
        if self.delivery.is_none() {
            let count = self.bytes.len().min(INPUT_BUFFER_BYTES);
            if count == 0 {
                return None;
            }
            self.delivery = Some(PendingDelivery {
                input_id: Uuid::new_v4(),
                data: self.bytes.iter().take(count).copied().collect(),
            });
        }
        self.delivery.clone()
    }

    fn confirm(
        &mut self,
        expected: &PendingDelivery,
        result: &InputResult,
    ) -> Result<(), RelayError> {
        let payload_matches = self
            .bytes
            .iter()
            .take(expected.data.len())
            .eq(expected.data.iter());
        if self.delivery.as_ref() != Some(expected)
            || !payload_matches
            || result.input_id != expected.input_id
            || result.delivery != delivery::DELIVERED
        {
            return Err(ProtocolError::ResponseMismatch.into());
        }
        self.bytes.drain(..expected.data.len());
        self.delivery = None;
        Ok(())
    }
}

enum InputDelivery {
    Delivered(InputResult),
    Deferred,
}

pub(super) async fn send_pending_input(
    control: &mut ControlChannel,
    lease: &mut LeaseController,
    pending: &mut PendingInputState,
) -> Result<(), RelayError> {
    let Some(delivery) = pending.next_delivery() else {
        return Ok(());
    };
    match send_input(control, lease, delivery.input_id, &delivery.data).await? {
        InputDelivery::Delivered(result) => pending.confirm(&delivery, &result)?,
        InputDelivery::Deferred => {}
    }
    Ok(())
}

async fn send_input(
    control: &mut ControlChannel,
    lease: &mut LeaseController,
    input_id: Uuid,
    data: &[u8],
) -> Result<InputDelivery, RelayError> {
    let Some((generation, writer_capability)) = lease.mutation_credentials() else {
        return Ok(InputDelivery::Deferred);
    };
    let result = control
        .input(generation, &writer_capability, input_id, data)
        .await;
    match result {
        Ok(result) => Ok(InputDelivery::Delivered(result)),
        Err(error) => match client_error(&error) {
            Some(client) if lease.mutation_rejected(client).await? => {
                lease.retain_control_intent();
                Ok(InputDelivery::Deferred)
            }
            _ => Err(error),
        },
    }
}

#[cfg(test)]
#[cfg(feature = "yolo-unit-tests")]
#[path = "../../unit_tests/session/attachment_input_inline_tests.rs"]
mod tests;
