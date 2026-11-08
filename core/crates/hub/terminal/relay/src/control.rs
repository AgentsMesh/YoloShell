use crate::{Options, RelayError, Status, authority::Authority, constants::*};
use tokio::{sync::mpsc, time::Instant};
use yolo_hub_client::{Client, ClientError, LocalClient};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{LeaseGrant, PublicError, ResizeRequest, TerminalSize};

pub(crate) struct Control {
    pub client: Option<LocalClient>,
    /// A control call that started while the input lane owned the primary
    /// channel can keep this spare channel warm for the next control call.
    pub lane_client: Option<LocalClient>,
    /// True while the ordered input lane owns the primary client.  Renewal,
    /// resize, and semantic commands must use a separate channel in that
    /// interval so a stalled input RPC cannot hold the lease hostage.
    pub input_busy: bool,
    pub options: Options,
    pub authority: Option<Authority>,
    pub cleanup_grant: Option<LeaseGrant>,
    pub initial: Status,
}


impl Control {
    pub fn origin(&self) -> yolo_hub_identity::HubId {
        self.options.target.address.hub_id
    }
}

include!("control_lease.rs");
include!("control_resize.rs");
include!("control_client.rs");
include!("control_publish.rs");
