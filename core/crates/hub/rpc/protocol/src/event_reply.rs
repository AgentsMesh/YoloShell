//! Correlation for platform-event pages and subscriptions.

use crate::{Command, Reply};

pub(crate) fn matches_target(reply: &Reply, command: &Command) -> Option<bool> {
    Some(match (reply, command) {
        (Reply::PlatformEvents { events }, Command::ListPlatformEvents { request }) => {
            events
                .events
                .windows(2)
                .all(|pair| pair[0].sequence < pair[1].sequence)
                && events.events.iter().all(|event| {
                    event.sequence > request.fields().after.map_or(0, |cursor| cursor.sequence)
                        || events.lost
                })
        }
        (Reply::EventSubscription { subscription }, Command::SubscribeEvents { request }) => {
            let after = request.fields().after.map_or(0, |cursor| cursor.sequence);
            subscription.events.len() <= request.fields().limit as usize
                && subscription
                    .events
                    .windows(2)
                    .all(|pair| pair[0].sequence < pair[1].sequence)
                && subscription
                    .events
                    .iter()
                    .all(|event| event.sequence > after || subscription.lost)
                && (!subscription.timed_out
                    || (subscription.events.is_empty() && !subscription.lost))
        }
        _ => return None,
    })
}
