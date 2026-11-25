#[path = "../test_support/channel_peer.rs"]
mod channel_peer;
mod chat_dispatch_evidence;
#[path = "../test_support/chat_projection.rs"]
mod chat_projection;
mod chat_queries;
#[path = "../test_support/chat.rs"]
mod chat_support;
mod chat_transport;
mod client_capabilities;
#[path = "../test_support/constants.rs"]
mod constants;
mod creation_intent_not_sent;
mod creation_intent_recovery;
mod creation_intent_storage;
#[path = "../test_support/creation_recovery.rs"]
mod creation_recovery_support;
#[path = "../test_support/creations.rs"]
mod creation_support;
mod finalization_journal;
mod finalization_persistence_failure;
#[path = "../test_support/finalization.rs"]
mod finalization_support;
mod finalization_transport;
#[path = "../test_support/fixtures.rs"]
mod fixtures;
#[path = "../test_support/input_transport.rs"]
mod input_transport;
#[path = "../test_support/interrupts.rs"]
mod interrupt_support;
mod layout_protocol;
mod management;
mod native_interrupt;
mod pressure;
mod terminal_input_pressure;
mod terminal_input_renewal;
