use super::super::*;

#[path = "../test_support/outbox_integrity_assertions.rs"]
mod assertions;
#[path = "../test_support/outbox_integrity_constants.rs"]
mod constants;
#[path = "../test_support/outbox_integrity_events.rs"]
mod events;
#[path = "../test_support/outbox_integrity_fixture.rs"]
mod fixture;
#[path = "outbox_integrity_identity_tests.rs"]
mod identity_tests;
#[path = "../test_support/outbox_integrity_mutations.rs"]
mod mutations;
#[path = "outbox_integrity_queue_tests.rs"]
mod queue_tests;
#[path = "outbox_integrity_receipt_tests.rs"]
mod receipt_tests;
#[path = "../test_support/outbox_integrity_receipts.rs"]
mod receipts;

use constants::*;
use fixture::{Case, Fixture};
use types::{AuthenticatedSessionStart, OutboxState};
