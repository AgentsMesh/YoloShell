use crate::{
    Options, State,
    constants::*,
    network::{ObservationUpdate, Update},
};
use std::collections::BTreeMap;
use tokio::{
    sync::mpsc,
    task::{AbortHandle, JoinSet},
    time::Instant,
};
use yolo_hub_identity::{RuntimeTarget, TerminalAddress};
use yolo_hub_terminal_client::Attachment;
use yolo_hub_terminal_protocol::{AttachRequest, RuntimeStatus};

struct Running {
    target: RuntimeTarget,
    handle: AbortHandle,
    retry_after: Instant,
}

#[derive(Default)]
pub struct Observers {
    running: BTreeMap<TerminalAddress, Running>,
    tasks: JoinSet<()>,
}

impl Observers {
    pub fn sync(&mut self, options: &Options, state: &mut State, updates: &mpsc::Sender<Update>) {
        for address in state.take_resync() {
            if let Some(running) = self.running.remove(&address) {
                running.handle.abort();
            }
            state.clear_observer_generation(address);
            // Keep the last confirmed screen (including keyboard modes) while
            // a planned geometry refresh obtains a replacement snapshot.
            if !state.is_geometry_syncing(address) {
                state.surfaces.remove(&address);
            }
        }
        while self.tasks.try_join_next().is_some() {}
        let visible = state.bound_terminals();
        self.running.retain(|address, running| {
            let keep = visible.contains(address)
                && state
                    .terminals
                    .get(address)
                    .and_then(|session| session.fields().runtime)
                    .is_some_and(|runtime| runtime.target == running.target);
            if !keep {
                running.handle.abort();
                state.clear_observer_generation(*address);
            }
            keep
        });
        for address in visible {
            let Some(runtime) = state
                .terminals
                .get(&address)
                .and_then(|session| session.fields().runtime)
            else {
                continue;
            };
            if !matches!(
                runtime.status,
                RuntimeStatus::Running
                    | RuntimeStatus::Finalizing
                    | RuntimeStatus::OutputBlocked
                    | RuntimeStatus::Exited
            ) || state.ended_epoch(address) == Some(runtime.target.runtime_epoch)
            {
                continue;
            }
            if self.running.get(&address).is_some_and(|running| {
                !running.handle.is_finished() || Instant::now() < running.retry_after
            }) {
                continue;
            }
            let cursor = state
                .surfaces
                .get(&address)
                .map(|surface| surface.cursor())
                .filter(|cursor| {
                    cursor.target == runtime.target && !state.is_geometry_syncing(address)
                });
            let request = if state.is_geometry_syncing(address) {
                AttachRequest::new(runtime.target, None)
            } else if let Some(cursor) = cursor {
                AttachRequest::new(runtime.target, Some(cursor))
            } else {
                AttachRequest::history(runtime.target, None)
            };
            let Ok(request) = request else {
                state.clear_observer_generation(address);
                continue;
            };
            let options = options.clone();
            let updates = updates.clone();
            let target = runtime.target;
            let generation = uuid::Uuid::new_v4();
            state.set_observer_generation(address, generation);
            let handle = self.tasks.spawn(async move {
                let result = observe(options, request, generation, &updates).await;
                let _ = updates
                    .send(Update::Observation {
                        target,
                        generation,
                        update: ObservationUpdate::Closed {
                            message: result.err().map(|error| error.to_string()),
                        },
                    })
                    .await;
            });
            self.running.insert(
                address,
                Running {
                    target,
                    handle,
                    retry_after: Instant::now() + STREAM_RECONNECT_DELAY,
                },
            );
        }
    }

    pub async fn close(&mut self) {
        self.tasks.abort_all();
        while self.tasks.join_next().await.is_some() {}
        self.running.clear();
    }
}

async fn observe(
    options: Options,
    request: AttachRequest,
    generation: uuid::Uuid,
    updates: &mpsc::Sender<Update>,
) -> anyhow::Result<()> {
    let target = request.target();
    let client = options
        .endpoint
        .connect(options.hub, Instant::now() + RPC_TIMEOUT)
        .await?;
    let mut attachment = Attachment::open(client, request, Instant::now() + RPC_TIMEOUT).await?;
    let initial = attachment
        .take_initial()
        .ok_or_else(|| anyhow::anyhow!("terminal attachment seed is missing"))?;
    updates
        .send(Update::Observation {
            target,
            generation,
            update: ObservationUpdate::Attached {
                request,
                attachment: initial,
            },
        })
        .await?;
    loop {
        // One task owns each read; UI redraws never cancel partially received frames.
        let event = attachment
            .next_event(Instant::now() + STREAM_DEADLINE)
            .await?;
        let terminal = event.is_terminal();
        updates
            .send(Update::Observation {
                target,
                generation,
                update: ObservationUpdate::Terminal { event },
            })
            .await?;
        if terminal {
            break;
        }
    }
    Ok(())
}
