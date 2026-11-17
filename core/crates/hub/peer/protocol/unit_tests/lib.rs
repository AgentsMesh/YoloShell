mod agent_control;
mod agent_control_forgery;
mod agent_control_replies;
mod calls;
mod catalogue_resources;
use yolo_hub_peer_protocol_test_support as fixtures;
mod creation_cleanup;
mod creation_query;
#[path = "../test_support/creation_query.rs"]
mod creation_query_support;
mod directory;
mod hubs;
mod management;
mod signatures;
use yolo_hub_delegation_test_support as support;
