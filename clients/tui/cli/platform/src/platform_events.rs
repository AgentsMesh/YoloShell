use crate::constants::{
    CATALOG_TIMEOUT, MAX_EVENTS_LIMIT, PLATFORM_EVENTS_SUBSCRIBE_UNSUPPORTED,
    PLATFORM_EVENTS_UNSUPPORTED, UNEXPECTED_EVENT_SUBSCRIPTION_REPLY, UNEXPECTED_EVENTS_REPLY,
};
use crate::platform_label::label;
use std::io::{self, Write};
use std::time::Duration;
use tokio::signal::unix::{Signal, SignalKind, signal};
use uuid::Uuid;
use yolo_hub_events_protocol::{
    ListPlatformEventsFields, ListPlatformEventsRequest, MAX_PLATFORM_EVENT_WAIT_MS,
    PLATFORM_EVENT_SCHEMA_VERSION, PlatformEvent, PlatformEventCursor, PlatformEventStream,
    SubscribeEventsFields, SubscribeEventsRequest,
};
use yolo_hub_rpc_protocol::{Command, Method, Reply};
use yolo_tui_app::Options;

pub(crate) async fn events(
    options: &Options,
    epoch: Option<Uuid>,
    after: Option<u64>,
    limit: u16,
    follow: bool,
    json: bool,
) -> anyhow::Result<()> {
    anyhow::ensure!(
        (1..=MAX_EVENTS_LIMIT).contains(&limit),
        "event limit must be between 1 and {MAX_EVENTS_LIMIT}"
    );
    let deadline = tokio::time::Instant::now() + CATALOG_TIMEOUT;
    let mut client = options.endpoint.connect(options.hub, deadline).await?;
    let hub = client.hello().hub_id;
    let mut after = epoch.map(|epoch| PlatformEventCursor {
        epoch,
        sequence: after.unwrap_or(0),
    });
    if follow {
        anyhow::ensure!(
            client.hello().methods.contains(&Method::SubscribeEvents),
            PLATFORM_EVENTS_SUBSCRIBE_UNSUPPORTED
        );
        let mut cancellation = Cancellation::install()?;
        loop {
            let request = SubscribeEventsRequest::new(SubscribeEventsFields {
                schema_version: PLATFORM_EVENT_SCHEMA_VERSION,
                after,
                limit,
                wait_ms: MAX_PLATFORM_EVENT_WAIT_MS,
            })
            .map_err(|error| anyhow::anyhow!(error))?;
            let wait = Duration::from_millis(u64::from(MAX_PLATFORM_EVENT_WAIT_MS));
            let call_deadline = tokio::time::Instant::now() + CATALOG_TIMEOUT + wait;
            let reply = tokio::select! {
                _ = cancellation.cancelled() => return Ok(()),
                reply = client.call(hub, Command::SubscribeEvents { request }, call_deadline) => {
                    reply?
                }
            };
            let Reply::EventSubscription { subscription } = reply else {
                anyhow::bail!(UNEXPECTED_EVENT_SUBSCRIPTION_REPLY);
            };
            if !subscription.events.is_empty() || subscription.lost {
                emit(
                    &subscription.stream,
                    &subscription.events,
                    subscription.lost,
                    json,
                )?;
            }
            after = subscription.next.or(after);
        }
    }
    anyhow::ensure!(
        client.hello().methods.contains(&Method::ListPlatformEvents),
        PLATFORM_EVENTS_UNSUPPORTED
    );
    let request = ListPlatformEventsRequest::new(ListPlatformEventsFields {
        schema_version: PLATFORM_EVENT_SCHEMA_VERSION,
        after,
        limit,
    })
    .map_err(|error| anyhow::anyhow!(error))?;
    let reply = client
        .call(hub, Command::ListPlatformEvents { request }, deadline)
        .await?;
    let Reply::PlatformEvents { events } = reply else {
        anyhow::bail!(UNEXPECTED_EVENTS_REPLY);
    };
    emit(&events.stream, &events.events, events.lost, json)
}

/// The binary installs handlers for these signals, so a long-running
/// subscription must observe them instead of relying on the default action.
struct Cancellation {
    interrupt: Signal,
    terminate: Signal,
    hangup: Signal,
    quit: Signal,
}

impl Cancellation {
    fn install() -> anyhow::Result<Self> {
        Ok(Self {
            interrupt: signal(SignalKind::interrupt())?,
            terminate: signal(SignalKind::terminate())?,
            hangup: signal(SignalKind::hangup())?,
            quit: signal(SignalKind::quit())?,
        })
    }

    async fn cancelled(&mut self) {
        tokio::select! {
            _ = self.interrupt.recv() => {},
            _ = self.terminate.recv() => {},
            _ = self.hangup.recv() => {},
            _ = self.quit.recv() => {},
        }
    }
}

fn emit(
    stream: &PlatformEventStream,
    events: &[PlatformEvent],
    lost: bool,
    json: bool,
) -> anyhow::Result<()> {
    let mut output = io::stdout().lock();
    if json {
        serde_json::to_writer_pretty(
            &mut output,
            &serde_json::json!({
                "epoch": stream.epoch,
                "sequence": stream.sequence,
                "first_sequence": stream.first_sequence,
                "lost": lost,
                "events": events,
            }),
        )?;
        writeln!(output)?;
    } else {
        writeln!(
            output,
            "epoch={} sequence={} first={} lost={lost}",
            stream.epoch, stream.sequence, stream.first_sequence
        )?;
        for event in events {
            writeln!(output, "{} {}", event.sequence, label(&event.kind))?;
        }
    }
    Ok(())
}
