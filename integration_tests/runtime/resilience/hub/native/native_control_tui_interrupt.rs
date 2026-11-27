use super::{
    Assets, SECOND_DELIVERY, capture,
    constants::*,
    outcome::{self, Outcome},
    send,
};
use tokio::time::{Instant, timeout_at};
use yolo_runtime::{model::WriterLeaseGrant, worker::RuntimeRef};

const PHASE_FAILED: &str = "native interrupt screen phase failed";
const COMMAND_TIMED_OUT: &str = "native interrupt keyboard request exceeded the original deadline";
const DELIVERY_TIMED_OUT: &str = "native interrupt delivery exceeded the original deadline";

pub(super) async fn deliver(
    client: &yolo_client::Client,
    runtime: RuntimeRef,
    grant: &WriterLeaseGrant,
    output: &mut yolo_client::Attachment,
    screen: &mut vt100::Parser,
    assets: &Assets,
    selection: (&str, &str),
) {
    let until = super::deadline();
    let mut driver = Driver {
        client,
        runtime,
        grant,
        output,
        screen,
        selection,
        until,
    };
    driver.refresh().await;
    loop {
        unchanged_signals(assets);
        driver.press(INTERRUPT).await;
        // Seeing Search and then leaving it proves this key was handled. The
        // cached receipt from the previous attempt cannot authorize another i.
        driver.barrier().await;
        driver
            .wait(|text| outcome::outcome(text) != Outcome::Pending)
            .await;
        match outcome::outcome(&driver.screen.screen().contents()) {
            Outcome::Sent => {
                let expected = SECOND_DELIVERY.checked_add(EXTRA_DELIVERY).unwrap();
                timeout_at(
                    until,
                    super::observe::count(&assets.target_marker, expected),
                )
                .await
                .expect(DELIVERY_TIMED_OUT);
                assert_eq!(
                    std::fs::read(&assets.other_marker).unwrap().len(),
                    usize::default()
                );
                assert!(Instant::now() < until, "{DELIVERY_TIMED_OUT}");
                return;
            }
            Outcome::PreconditionsChanged | Outcome::NotSent => {
                unchanged_signals(assets);
                driver.refresh().await;
            }
            result => panic!(
                "{PHASE_FAILED}: {result:?}; {}",
                driver.screen.screen().contents()
            ),
        }
    }
}

struct Driver<'a> {
    client: &'a yolo_client::Client,
    runtime: RuntimeRef,
    grant: &'a WriterLeaseGrant,
    output: &'a mut yolo_client::Attachment,
    screen: &'a mut vt100::Parser,
    selection: (&'a str, &'a str),
    until: Instant,
}

impl Driver<'_> {
    async fn press(&self, key: &[u8]) {
        assert!(Instant::now() < self.until, "{COMMAND_TIMED_OUT}");
        timeout_at(self.until, send(self.client, self.runtime, self.grant, key))
            .await
            .expect(COMMAND_TIMED_OUT);
    }

    async fn wait(&mut self, ready: impl Fn(&str) -> bool) {
        capture::try_wait_screen_until(self.output, self.screen, self.until, ready)
            .await
            .unwrap_or_else(|error| panic!("{PHASE_FAILED}: {error}"));
    }

    async fn barrier(&mut self) {
        self.press(SEARCH).await;
        self.wait(outcome::searching).await;
        self.press(ESCAPE).await;
        self.wait(|text| !outcome::searching(text)).await;
    }

    async fn refresh(&mut self) {
        self.press(REFRESH).await;
        self.barrier().await;
        let (identity, writing) = self.selection;
        self.wait(|text| outcome::ready(text, identity, writing))
            .await;
    }
}

fn unchanged_signals(assets: &Assets) {
    assert_eq!(
        std::fs::read(&assets.target_marker).unwrap().len(),
        SECOND_DELIVERY
    );
    assert_eq!(
        std::fs::read(&assets.other_marker).unwrap().len(),
        usize::default()
    );
}
