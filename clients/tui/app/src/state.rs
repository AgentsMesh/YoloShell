use crate::constants::*;
use std::{collections::BTreeMap, time::Instant};
use uuid::Uuid;
use yolo_builtin_client_contract::CapabilityNegotiation;
use yolo_hub_identity::TerminalAddress;
use yolo_hub_terminal_protocol::{LeaseGrant, TerminalCommand, TerminalSummary, WriterPrincipal};
use yolo_presentation_layout::Layout;
use yolo_tui_inventory::Inventory;
use yolo_tui_terminal::TerminalSurface;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mode {
    Inventory,
    Terminal,
    Chat,
    Document,
    Search,
    Directory,
    ConfirmTerminate,
    ConfirmArchive,
    ConfirmRecreate,
    HubConfiguration,
}

pub struct Writer {
    pub grant: LeaseGrant,
    pub expires: Instant,
    pub renew: Instant,
}

pub struct State {
    pub terminals: BTreeMap<TerminalAddress, TerminalSummary>,
    pub selected: Option<TerminalAddress>,
    pub filter: String,
    pub entry: String,
    pub(crate) form_input: crate::form_input::FormState,
    pub mode: Mode,
    /// The tab catalog owns the active layout. Keeping topology and focus
    /// under one aggregate prevents a stale UI projection from diverging from
    /// the persisted catalog.
    pub(crate) tabs: yolo_tui_tab::TabCatalog,
    pub(crate) tab_revision: u64,
    pub(crate) tab_manager: Option<yolo_tui_tab_manager::Manager>,
    pub(crate) shared_layout: Option<crate::layout_sync::Sync>,
    pub(crate) chats: crate::chat::Chats,
    pub(crate) sidebar_visible: bool,
    pub(crate) terminal_picker: Option<crate::tabs::terminal_picker::TerminalPicker>,
    pub surfaces: BTreeMap<TerminalAddress, TerminalSurface>,
    pub writers: BTreeMap<TerminalAddress, Writer>,
    /// Per-terminal runtime facts. Keeping them in one value makes runtime
    /// retirement a single removal and prevents a new fact from escaping
    /// cleanup.
    pub(crate) terminal_runtime:
        BTreeMap<TerminalAddress, crate::terminal_runtime::TerminalRuntimeFacts>,
    pub(crate) retired_writers: Vec<Writer>,
    pub(crate) control: crate::control::Controls,
    pub principal: WriterPrincipal,
    pub status: String,
    /// The current status is a recoverable failure; a successful catalogue
    /// refresh or reconnect may replace it with the connected state.
    pub(crate) recoverable_status: Option<String>,
    pub(crate) notifications: yolo_tui_notification::Notifications,
    pub control_generation: Option<Uuid>,
    pub(crate) workspaces_supported: bool,
    pub(crate) workspaces: yolo_tui_workspace::Picker,
    pub(crate) client_capabilities: Option<CapabilityNegotiation>,
    pub(crate) client_startup_failure: Option<String>,
    pub prefix: bool,
    pub(crate) help: bool,
    pub(crate) help_offset: u16,
    pub(crate) startup: crate::startup::Entry,
    pub catalog_pending: Option<Uuid>,
    pub confirmation: Option<TerminalCommand>,
    pub(crate) ui_epoch: u64,
    pub(crate) initial_control: Option<crate::initial_control::Intent>,
    pub inventory: Inventory,
    pub(crate) interrupts: crate::interrupt_state::Interrupts,
    /// Answers already dispatched from the details view; cleared by their
    /// exact completion so key repeat cannot send a second Respond.
    pub(crate) respond_pending: std::collections::BTreeSet<yolo_hub_identity::AgentTarget>,
    pub(crate) queries: crate::query_state::Queries,
    pub(crate) inputs: crate::terminal_input::Inputs,
    pub(crate) creations: crate::creation_state::Creations,
    pub(crate) presentation: crate::presentation::Presentation,
    pub(crate) pointer: crate::pointer::Pointer,
    pub(crate) pane_view: crate::pane_view::PaneView,
    pub(crate) document: yolo_tui_document::View,
    pub(crate) selection: crate::selection_state::SelectionState,
    pub(crate) hub_manager: Option<yolo_tui_hub_manager::Manager>,
}
impl State {
    /// Whether the control channel has completed its Hello handshake. This is
    /// independent of the status line, which also carries transient hints and
    /// errors and therefore cannot gate lifecycle recovery UI.
    pub(crate) fn connection_ready(&self) -> bool {
        self.client_capabilities.is_some() && self.client_startup_failure.is_none()
    }
}

impl Default for State {
    fn default() -> Self {
        let tabs = yolo_tui_tab::TabCatalog::default();
        Self {
            terminals: BTreeMap::new(),
            selected: None,
            filter: String::new(),
            entry: String::new(),
            form_input: Default::default(),
            mode: Mode::Inventory,
            tabs,
            tab_revision: 0,
            tab_manager: None,
            shared_layout: None,
            chats: Default::default(),
            sidebar_visible: false,
            terminal_picker: None,
            surfaces: BTreeMap::new(),
            writers: BTreeMap::new(),
            terminal_runtime: BTreeMap::new(),
            retired_writers: Vec::new(),
            control: Default::default(),
            principal: WriterPrincipal {
                client_id: Uuid::new_v4(),
                controller_instance_id: Uuid::new_v4(),
            },
            status: CONNECTING_LABEL.into(),
            recoverable_status: None,
            notifications: Default::default(),
            control_generation: None,
            workspaces_supported: false,
            workspaces: Default::default(),
            client_capabilities: None,
            client_startup_failure: None,
            prefix: false,
            help: false,
            help_offset: 0,
            startup: crate::startup::Entry::default(),
            catalog_pending: None,
            confirmation: None,
            ui_epoch: u64::default(),
            initial_control: None,
            inventory: Inventory::default(),
            interrupts: crate::interrupt_state::Interrupts::default(),
            respond_pending: Default::default(),
            queries: crate::query_state::Queries::default(),
            inputs: crate::terminal_input::Inputs::default(),
            creations: crate::creation_state::Creations::default(),
            presentation: crate::presentation::Presentation::default(),
            pointer: crate::pointer::Pointer::default(),
            pane_view: crate::pane_view::PaneView::default(),
            document: Default::default(),
            selection: crate::selection_state::SelectionState::default(),
            hub_manager: None,
        }
    }
}
impl State {
    pub(crate) fn tab_layout(&self) -> &Layout {
        self.tabs.active_layout()
    }

    pub(crate) fn tab_layout_mut(&mut self) -> &mut Layout {
        self.tab_revision = self.tab_revision.saturating_add(1);
        self.tabs.active_layout_mut()
    }

    pub(crate) fn replace_active_tab_layout(&mut self, layout: Layout) {
        self.tab_revision = self.tab_revision.saturating_add(1);
        self.tabs.replace_active_layout(layout);
    }

    pub(crate) fn bind_focused(
        &mut self,
        terminal: Option<TerminalAddress>,
    ) -> Result<(), yolo_presentation_layout::LayoutError> {
        let focused = self.tab_layout().focused();
        self.tab_layout_mut().bind(focused, terminal)
    }
}
