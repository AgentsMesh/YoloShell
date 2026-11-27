use crate::{
    Answer, CHAT_METHOD, Call, Fault, Id, call::WireCall, chat_input, chat_result, constants::*,
};
use serde::Deserialize;
use serde_json::Value;
use yolo_hub_agent_protocol::{AgentAction, AgentPromptReference};
use yolo_hub_rpc_protocol::{Command, Method, Reply, Response};
use yolo_hub_terminal_protocol::PublicError;

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Operation {
    Open,
    Read,
    State,
    Prompt,
    Respond,
    Cancel,
}

impl Operation {
    fn method(self, wire: &mut WireCall) -> Option<Method> {
        Some(match self {
            Self::Open => Method::ResolveConversation,
            Self::Read if wire.params.contains_key("wait_ms") => Method::WatchConversation,
            Self::Read => Method::ReadConversation,
            Self::State => Method::GetAgent,
            Self::Prompt => {
                chat_input::normalize(&mut wire.params)?;
                Method::AgentPrompt
            }
            Self::Respond | Self::Cancel => Method::AgentControl,
        })
    }

    fn accepts(self, command: &Command) -> bool {
        match (self, command) {
            (Self::Respond, Command::AgentControl { request, .. }) => {
                matches!(request.fields().action, AgentAction::Respond { .. })
            }
            (Self::Cancel, Command::AgentControl { request, .. }) => {
                matches!(request.fields().action, AgentAction::Interrupt {})
            }
            (Self::Prompt, Command::AgentPrompt { request }) => {
                matches!(
                    request.fields().reference,
                    AgentPromptReference::Execution { .. }
                )
            }
            _ => true,
        }
    }
}

/// The entry Hub decodes the requested representation while dispatch, rights,
/// operation identity, routing and origin validation use the established Command.
#[derive(Debug)]
pub struct ClientCall {
    pub call: Call,
    chat: bool,
}

impl ClientCall {
    pub fn parse(value: Value) -> Result<Self, Fault> {
        let mut wire = WireCall::parse(value)?;
        if wire.method != CHAT_METHOD {
            return Ok(Self {
                call: wire.into_call()?,
                chat: false,
            });
        }
        let invalid = || Fault::new(Some(wire.id.clone()), INVALID_PARAMS);
        let op: Operation = wire
            .params
            .remove("operation")
            .and_then(|value| serde_json::from_value(value).ok())
            .ok_or_else(invalid)?;
        let id: Id = wire.id.clone();
        let invalid = || Fault::new(Some(id.clone()), INVALID_PARAMS);
        let method = op.method(&mut wire).ok_or_else(invalid)?;
        let Value::String(name) = serde_json::to_value(method).expect("method name") else {
            unreachable!()
        };
        wire.method = name;
        let call = wire.into_call()?;
        if !op.accepts(&call.fields().command) {
            return Err(invalid());
        }
        Ok(Self { call, chat: true })
    }

    pub fn answer(&self, reply: Reply) -> Result<Answer, PublicError> {
        Response::new(&self.call, self.call.fields().destination, reply.clone())
            .validate_for(&self.call)?;
        match reply {
            Reply::Extensions { mut methods } if !self.chat => {
                if !methods.iter().any(|name| name == CHAT_METHOD) {
                    methods.push(CHAT_METHOD.to_owned());
                }
                Answer::new(&self.call, Reply::Extensions { methods })
            }
            reply if self.chat => chat_result::encode(&self.call, reply),
            reply => Answer::new(&self.call, reply),
        }
    }
}
