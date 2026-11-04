//! Origin-local read service. The caller supplies and revalidates authenticated run bindings.
mod cache;
mod descriptor;
mod evidence;
mod history;
mod prefix;
mod reader;
mod registry;

pub use descriptor::resolve_conversation;
pub use reader::ConversationReader;
pub use yolo_conversation_source::ConversationSources;

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod tests;
