use crate::{RpcError, constants::*};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use yolo_hub_rpc_protocol::{Method, Reply};
use yolo_hub_terminal_protocol::PublicError;

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum ErrorData {
    Hub {
        code: PublicError,
    },
    Conversation {
        error: yolo_conversation_protocol::ConversationError,
    },
}

pub(crate) fn encode(reply: Reply) -> Result<(Option<Value>, Option<RpcError>), PublicError> {
    if matches!(reply, Reply::Attached { .. }) {
        return Err(PublicError::InvalidPayload);
    }
    let error = match &reply {
        Reply::Error { code } => {
            Some((HUB_ERROR, code.to_string(), ErrorData::Hub { code: *code }))
        }
        Reply::ConversationUnavailable { error } => Some((
            CONVERSATION_ERROR,
            "Conversation unavailable".into(),
            ErrorData::Conversation {
                error: error.clone(),
            },
        )),
        _ => None,
    };
    if let Some((code, message, data)) = error {
        let data = serde_json::to_value(data).map_err(|_| PublicError::InvalidPayload)?;
        return Ok((
            None,
            Some(RpcError {
                code,
                message,
                data: Some(data),
            }),
        ));
    }
    let Value::Object(mut value) =
        serde_json::to_value(reply).map_err(|_| PublicError::InvalidPayload)?
    else {
        return Err(PublicError::InvalidPayload);
    };
    value.remove(RESULT_TAG);
    Ok((Some(Value::Object(value)), None))
}

pub(crate) fn decode_error(error: RpcError) -> Result<Reply, PublicError> {
    let rejection = match error.code {
        PARSE_ERROR | INVALID_REQUEST => Some(PublicError::InvalidRequest),
        METHOD_NOT_FOUND => Some(PublicError::Unsupported),
        INVALID_PARAMS => Some(PublicError::InvalidPayload),
        INTERNAL_ERROR => Some(PublicError::OutcomeUnknown),
        _ => None,
    };
    if let Some(code) = rejection {
        return Ok(Reply::Error { code });
    }
    let data: ErrorData = serde_json::from_value(error.data.ok_or(PublicError::InvalidPayload)?)
        .map_err(|_| PublicError::InvalidPayload)?;
    match (error.code, data) {
        (HUB_ERROR, ErrorData::Hub { code }) => Ok(Reply::Error { code }),
        (CONVERSATION_ERROR, ErrorData::Conversation { error }) => {
            Ok(Reply::ConversationUnavailable { error })
        }
        _ => Err(PublicError::InvalidPayload),
    }
}

pub(crate) fn decode(value: Value, method: Method) -> Result<Reply, PublicError> {
    let Value::Object(mut value) = value else {
        return Err(PublicError::InvalidPayload);
    };
    if value.contains_key(RESULT_TAG) {
        return Err(PublicError::InvalidPayload);
    }
    use Method::*;
    let tag = match method {
        Hello => reply_tag::HELLO,
        GetBuild => reply_tag::BUILD,
        List => reply_tag::TERMINALS,
        Get | Terminal | CleanupArchive => reply_tag::TERMINAL,
        Create => reply_tag::CREATED,
        TerminalRun => reply_tag::TERMINAL_RUN,
        AgentStart => reply_tag::AGENT_START,
        AgentStop => reply_tag::AGENT_STOP,
        AgentPrompt => reply_tag::AGENT_PROMPT,
        CreateWorkspace | UpdateWorkspace | GetWorkspace | CreateWorktree => reply_tag::WORKSPACE,
        ListWorkspaces | ListWorktrees => reply_tag::WORKSPACES,
        GetAgentRun | UpdateAgentRun => reply_tag::AGENT_RUN,
        ListAgentRuns => reply_tag::AGENT_RUNS,
        QueryCreation => reply_tag::CREATION_QUERY,
        Finalize | Release | Resize | RemoveWorktree => reply_tag::COMPLETED,
        Lease | Renew => reply_tag::LEASE,
        Input => reply_tag::INPUT,
        Attach => return Err(PublicError::InvalidPayload),
        ReadTerminalText => reply_tag::TERMINAL_TEXT,
        ReadTerminalHistory => reply_tag::TERMINAL_HISTORY,
        ObserveLease => reply_tag::LEASE_OBSERVATION,
        ListAgents => reply_tag::AGENTS,
        GetAgent | GetCurrentAgent => reply_tag::AGENT,
        WaitAgents => reply_tag::AGENT_WAIT,
        ListHubs => reply_tag::HUBS,
        ListProviders => reply_tag::PROVIDERS,
        ExportAudit => reply_tag::AUDIT,
        GetPlatformSnapshot => reply_tag::PLATFORM_SNAPSHOT,
        ListPlatformEvents => reply_tag::PLATFORM_EVENTS,
        SubscribeEvents => reply_tag::EVENT_SUBSCRIPTION,
        CleanupTerminate => reply_tag::CREATION_TERMINATED,
        GetConfiguration | Configure => reply_tag::CONFIGURATION,
        ListFileRoots => reply_tag::FILE_ROOTS,
        ReadFile => reply_tag::FILE,
        AgentControl => reply_tag::AGENT_CONTROL,
        ListLayouts => reply_tag::LAYOUTS,
        GetLayout | UpdateLayout => reply_tag::LAYOUT,
        ResolveConversation => reply_tag::CONVERSATION,
        ReadConversation | WatchConversation => reply_tag::CONVERSATION_READ,
        GetClientAccess | ConfigureClientAccess => reply_tag::CLIENT_ACCESS,
        GetSshProfiles | ConfigureSshProfiles => reply_tag::SSH_PROFILES,
        ConnectSshProfile => reply_tag::SSH_CONNECTED,
        RemoteSetup => reply_tag::REMOTE_SETUP,
        InspectRemoteSetup => reply_tag::REMOTE_SETUP_HISTORY,
    };
    value.insert(RESULT_TAG.into(), Value::String(tag.into()));
    serde_json::from_value(Value::Object(value)).map_err(|_| PublicError::InvalidPayload)
}
