use yolo_hub_client_access_registry::ClientAdmission;
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::{Command, Method, Request};
use yolo_hub_terminal_protocol::PublicError;

pub(crate) fn authorize(admission: &ClientAdmission, request: &Request) -> Result<(), PublicError> {
    crate::guard::verify(admission)?;
    let fields = request.fields();
    fields.command.validate_origin(fields.destination)?;
    if matches!(
        fields.command,
        Command::Hello {}
            | Command::List { .. }
            | Command::ListAgents { .. }
            | Command::ListHubs { .. }
    ) {
        // Catalogue requests always enter here; their individual rows are scoped
        // by origin. A caller cannot send an aggregate query directly to a peer.
        return if fields.destination == admission.entry_hub() {
            Ok(())
        } else {
            Err(PublicError::Unauthorized)
        };
    }
    let rights = fields
        .command
        .method()
        .required_rights()
        .ok_or(PublicError::Unauthorized)?;
    if !admission.grant().allows(fields.destination, &rights) {
        return Err(PublicError::Unauthorized);
    }
    let principal = match &fields.command {
        Command::Lease { request } => Some(request.principal),
        Command::Renew { grant }
        | Command::Release { grant }
        | Command::AgentControl { grant, .. } => Some(grant.fields().principal),
        Command::Input { request } => Some(request.fields().grant.fields().principal),
        Command::Resize { request } => Some(request.grant.fields().principal),
        _ => None,
    };
    if principal
        .is_some_and(|principal| principal.client_id != admission.grant().identity.client_id)
    {
        return Err(PublicError::Unauthorized);
    }
    Ok(())
}

pub(crate) fn visible(admission: &ClientAdmission, origin: HubId) -> bool {
    admission.grant().allows(
        origin,
        &[yolo_hub_routing::Right::Observe].into_iter().collect(),
    )
}

pub(crate) fn advertised(admission: &ClientAdmission, method: Method) -> bool {
    if method == Method::Hello {
        return true;
    }
    method.required_rights().is_some_and(|rights| {
        admission
            .grant()
            .origins
            .iter()
            .any(|origin| origin.rights.allows(&rights))
    })
}
