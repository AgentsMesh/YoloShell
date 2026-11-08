use crate::{ControlRequest, constants::INPUT_BATCH_WINDOW};
use std::collections::VecDeque;
use std::time::Duration;
use tokio::{
    sync::mpsc,
    time::{Instant, sleep_until},
};
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::{InputFields, InputRequest, MAX_INPUT_BYTES};

const INPUT_BATCH_VALIDATION_DETAIL: &str = "validated input batch construction failed";

pub(crate) enum Dispatch {
    Single(ControlRequest),
    Inputs {
        combined: ControlRequest,
        originals: Vec<ControlRequest>,
    },
}

pub(crate) async fn run(
    mut incoming: mpsc::Receiver<ControlRequest>,
    outgoing: mpsc::Sender<Dispatch>,
) {
    let mut pending = VecDeque::new();
    loop {
        let first = if let Some(first) = pending.pop_front() {
            first
        } else if let Ok(first) = incoming.try_recv() {
            first
        } else {
            let Some(first) = incoming.recv().await else {
                break;
            };
            first
        };
        if send_single_or_batch(first, &mut incoming, &mut pending, &outgoing)
            .await
            .is_err()
        {
            break;
        }
    }
}

async fn send_single_or_batch(
    first: ControlRequest,
    incoming: &mut mpsc::Receiver<ControlRequest>,
    pending: &mut VecDeque<ControlRequest>,
    outgoing: &mpsc::Sender<Dispatch>,
) -> Result<(), ()> {
    let Command::Input { .. } = first.command else {
        return outgoing.send(Dispatch::Single(first)).await.map_err(|_| ());
    };
    if first
        .validity
        .as_ref()
        .is_none_or(|validity| !validity.is_current())
    {
        return outgoing.send(Dispatch::Single(first)).await.map_err(|_| ());
    }
    let mut originals = vec![first.clone()];
    let mut combined = first;
    let now = std::time::Instant::now();
    let window = combined
        .validity
        .as_ref()
        .filter(|validity| validity.is_current())
        .map_or(Duration::ZERO, |validity| {
            INPUT_BATCH_WINDOW.min(validity.deadline().saturating_duration_since(now))
        });
    let deadline = Instant::now() + window;
    loop {
        let next = tokio::select! {
            next = incoming.recv() => next,
            _ = sleep_until(deadline) => break,
        };
        let Some(next) = next else { break };
        if append_input(&mut combined, &next) {
            originals.push(next);
        } else {
            pending.push_back(next);
            break;
        }
    }
    if originals.len() == 1 {
        outgoing
            .send(Dispatch::Single(combined))
            .await
            .map_err(|_| ())
    } else {
        outgoing
            .send(Dispatch::Inputs {
                combined,
                originals,
            })
            .await
            .map_err(|_| ())
    }
}

fn append_input(combined: &mut ControlRequest, next: &ControlRequest) -> bool {
    let (Command::Input { request: current }, Command::Input { request: incoming }) =
        (&combined.command, &next.command)
    else {
        return false;
    };
    if combined.generation != next.generation
        || combined.destination != next.destination
        || current.fields().grant != incoming.fields().grant
        || !combined
            .validity
            .as_ref()
            .zip(next.validity.as_ref())
            .is_some_and(|(left, right)| left.same_scope(right) && right.is_current())
        || current.fields().data.len() + incoming.fields().data.len() > MAX_INPUT_BYTES
    {
        return false;
    }
    let mut fields: InputFields = current.fields().clone();
    fields.data.extend_from_slice(&incoming.fields().data);
    combined.command = Command::Input {
        request: InputRequest::new(fields).expect(INPUT_BATCH_VALIDATION_DETAIL),
    };
    true
}
