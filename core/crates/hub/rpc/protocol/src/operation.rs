use crate::access::Access;
use serde::{Deserialize, Serialize};
use yolo_hub_routing::Rights;

/// How a caller may obtain the public rights for an operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mutation {
    ReadOnly,
    Mutating,
    /// The command payload decides whether the operation mutates.
    PayloadDefined,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Operation {
    pub method: Method,
    pub access: Access,
    pub mutation: Mutation,
}

impl Operation {
    pub fn mutates(self) -> bool {
        !matches!(self.mutation, Mutation::ReadOnly)
    }

    /// Resource reads take the federation egress-admission path so a growing
    /// reply cannot outrun the caller's demand.
    pub fn requires_read_reply_admission(self) -> bool {
        matches!(
            self.access,
            Access::FileRead | Access::PresentationRead | Access::ConversationRead
        )
    }
}

/// The single operation table. Each entry defines the method's advertised
/// set, access requirement and mutation semantics; the method enum and the
/// category lists are generated from the same invocation.
macro_rules! operations {
    (
        $(
            $methods:ident {
                $( $variant:ident : $access:ident, $mutation:ident ),* $(,)?
            }
        )+
    ) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
        #[repr(usize)]
        #[serde(rename_all = "snake_case")]
        pub enum Method {
            $( $( $variant, )* )+
        }

        pub const ALL_METHODS: &[Method] = &[ $( $( Method::$variant, )* )+ ];

        $(
            pub const $methods: &[Method] = &[ $( Method::$variant, )* ];
        )+

        pub const OPERATIONS: &[Operation] = &[
            $(
                $(
                    Operation {
                        method: Method::$variant,
                        access: Access::$access,
                        mutation: Mutation::$mutation,
                    },
                )*
            )+
        ];
    };
}

/// Methods implemented by an origin-side service layer rather than the raw
/// local backend. The origin decorator method lists must equal this table;
/// a new federated service call updates both together.
pub const ORIGIN_SERVICE_METHODS: &[Method] = &[
    Method::ReadTerminalHistory,
    Method::ListProviders,
];

operations! {
    SUPPORTED_METHODS {
        ListLayouts: PresentationRead, ReadOnly,
        GetLayout: PresentationRead, ReadOnly,
        UpdateLayout: PresentationWrite, Mutating,
        ResolveConversation: ConversationRead, ReadOnly,
        ReadConversation: ConversationRead, ReadOnly,
        WatchConversation: ConversationRead, ReadOnly,
        Hello: Observe, ReadOnly,
        List: Observe, ReadOnly,
        Get: Observe, ReadOnly,
        Create: Lifecycle, Mutating,
        TerminalRun: TerminalRun, Mutating,
        AgentStart: AgentStart, Mutating,
        AgentStop: AgentLifecycle, Mutating,
        CreateWorkspace: WorkspaceWrite, Mutating,
        UpdateWorkspace: WorkspaceWrite, Mutating,
        GetWorkspace: WorkspaceRead, ReadOnly,
        ListWorkspaces: WorkspaceRead, ReadOnly,
        CreateWorktree: WorkspaceWrite, Mutating,
        RemoveWorktree: WorkspaceWrite, Mutating,
        ListWorktrees: WorkspaceRead, ReadOnly,
        GetAgentRun: Observe, ReadOnly,
        ListAgentRuns: Observe, ReadOnly,
        UpdateAgentRun: AgentMetadata, Mutating,
        QueryCreation: Lifecycle, ReadOnly,
        Finalize: Lifecycle, Mutating,
        Lease: TerminalWrite, Mutating,
        Renew: TerminalWrite, Mutating,
        Release: TerminalWrite, Mutating,
        Input: TerminalWrite, Mutating,
        Resize: TerminalWrite, Mutating,
        Terminal: Lifecycle, Mutating,
        Attach: TerminalRead, ReadOnly,
        ReadTerminalText: TerminalRead, ReadOnly,
        ReadTerminalHistory: TerminalRead, ReadOnly,
        ObserveLease: Observe, ReadOnly,
        ListAgents: Observe, ReadOnly,
        WaitAgents: Observe, ReadOnly,
        GetAgent: Observe, ReadOnly,
        GetCurrentAgent: Observe, ReadOnly,
        ListHubs: Observe, ReadOnly,
        ListProviders: Observe, ReadOnly,
        GetPlatformSnapshot: Observe, ReadOnly,
        ListPlatformEvents: Observe, ReadOnly,
        ExportAudit: Observe, ReadOnly,
        SubscribeEvents: Observe, ReadOnly,
        CleanupTerminate: Lifecycle, Mutating,
        CleanupArchive: Lifecycle, Mutating,
        ListFileRoots: FileRead, ReadOnly,
        ReadFile: FileRead, ReadOnly,
        AgentControl: AgentControl, Mutating,
        AgentPrompt: AgentControl, Mutating,
    }
    MANAGEMENT_METHODS {
        GetConfiguration: LocalManagement, ReadOnly,
        Configure: LocalManagement, Mutating,
    }
    BUILD_METHODS {
        GetBuild: LocalManagement, ReadOnly,
    }
    SSH_PROFILE_METHODS {
        GetSshProfiles: LocalManagement, ReadOnly,
        ConfigureSshProfiles: LocalManagement, Mutating,
        ConnectSshProfile: LocalManagement, Mutating,
    }
    REMOTE_SETUP_METHODS {
        RemoteSetup: LocalManagement, PayloadDefined,
        InspectRemoteSetup: LocalManagement, ReadOnly,
    }
    CLIENT_ACCESS_METHODS {
        GetClientAccess: LocalManagement, ReadOnly,
        ConfigureClientAccess: LocalManagement, Mutating,
    }
}

impl Method {
    pub fn operation(self) -> &'static Operation {
        let operation = &OPERATIONS[self as usize];
        debug_assert_eq!(operation.method, self);
        operation
    }

    pub fn required_rights(self) -> Option<Rights> {
        self.operation().access.required_rights()
    }

    pub fn mutates(self) -> bool {
        self.operation().mutates()
    }
}
