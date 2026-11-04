mod catalog;
mod chat;
mod client;
mod control;
mod creation;
mod drawing;
mod finalization;
mod form;
mod initial_control;
mod input;
mod interrupt;
mod inventory;
mod layout;
mod lease;
mod lifecycle;
mod pane;
mod pointer;
mod query;
mod resize;
mod selection;
mod startup;
mod tab;
mod terminal;
mod writer;

#[path = "../test_support/fixtures.rs"]
mod fixtures;

#[path = "../test_support/panes.rs"]
mod panes;

#[path = "../test_support/queries.rs"]
mod query_support;

#[path = "../test_support/resize_input.rs"]
mod resize_input_support;

mod actions;

mod agent_directory;

mod channel_events;

#[path = "../test_support/chat_projection.rs"]
mod chat_projection;

#[path = "../test_support/chat.rs"]
mod chat_support;

mod compact_chrome;

#[path = "../test_support/creations.rs"]
mod creation_support;

#[path = "../test_support/drawing_readiness.rs"]
mod drawing_support;

mod entry_interactions;

mod events;

#[path = "../test_support/finalization.rs"]
mod finalization_support;

mod help_navigation;

mod hub_management;

#[path = "../test_support/interrupts.rs"]
mod interrupt_support;

#[path = "../test_support/layout_sync.rs"]
mod layout_support;

mod modal_shortcuts;

mod not_sent;

mod observer_capabilities;

mod recovery_feedback;
mod respond;

mod runtime_transitions;

#[path = "../test_support/selection.rs"]
mod selection_support;

mod sidebar_preference;

#[path = "../test_support/startup.rs"]
mod startup_support;

mod state;

#[path = "../test_support/terminal_input_navigation.rs"]
mod terminal_input_navigation_support;

mod text_elision;

mod update_queue;

mod control_recovery_support;
mod initial_control_late_grants;
mod terminal_input_automatic_loss;
mod terminal_input_ordering;
