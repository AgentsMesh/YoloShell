use crate::{NotSentReason, Outcome, ProtocolError, Request, Response, capnp_request};
use yolo_hub_agent_protocol::{CommandOutcome, CommandReceipt, FailureCode};
use yolo_native_relay_schema::relay_capnp::{Failure, NotSent, control_response};

pub(crate) fn encode(
    mut value: control_response::Builder<'_>,
    response: &Response,
) -> Result<(), ProtocolError> {
    let fields = response.fields();
    let request = Request::new(crate::RequestFields {
        schema_version: fields.schema_version,
        launch_id: fields.launch_id,
        terminal: fields.terminal,
        request: fields.request.clone(),
    })?;
    capnp_request::encode(value.reborrow().init_request(), &request);
    let mut target = value.init_outcome();
    match fields.outcome {
        Outcome::Unknown {} => target.set_unknown(()),
        Outcome::NotSent { reason } => target.set_not_sent(match reason {
            NotSentReason::NoWriter => NotSent::NoWriter,
            NotSentReason::Expired => NotSent::Expired,
            NotSentReason::TargetChanged => NotSent::TargetChanged,
            NotSentReason::Unavailable => NotSent::Unavailable,
        }),
        Outcome::Receipt { receipt } => match receipt.outcome() {
            CommandOutcome::Accepted {} => target.set_accepted(()),
            CommandOutcome::Delivered {} => target.set_delivered(()),
            CommandOutcome::Confirmed {} => target.set_confirmed(()),
            CommandOutcome::Unknown {} => target.set_receipt_unknown(()),
            CommandOutcome::Failed { code } => target.set_failed(match code {
                FailureCode::PermissionDenied => Failure::PermissionDenied,
                FailureCode::PreconditionsChanged => Failure::PreconditionsChanged,
                FailureCode::Unsupported => Failure::Unsupported,
                FailureCode::Expired => Failure::Expired,
                FailureCode::AdapterRejected => Failure::AdapterRejected,
            }),
        },
    }
    Ok(())
}

pub(crate) fn decode(value: control_response::Reader<'_>) -> Result<Response, ProtocolError> {
    use control_response::outcome::Which;
    fn fail<E>(_: E) -> ProtocolError {
        ProtocolError::InvalidFrame
    }
    let request = capnp_request::decode(value.get_request().map_err(fail)?)?;
    let outcome = match value.get_outcome().which().map_err(fail)? {
        Which::Unknown(()) => return Response::new(&request, Outcome::Unknown {}),
        Which::NotSent(reason) => {
            return Response::new(
                &request,
                Outcome::NotSent {
                    reason: match reason.map_err(fail)? {
                        NotSent::NoWriter => NotSentReason::NoWriter,
                        NotSent::Expired => NotSentReason::Expired,
                        NotSent::TargetChanged => NotSentReason::TargetChanged,
                        NotSent::Unavailable => NotSentReason::Unavailable,
                    },
                },
            );
        }
        Which::Accepted(()) => CommandOutcome::Accepted {},
        Which::Delivered(()) => CommandOutcome::Delivered {},
        Which::Confirmed(()) => CommandOutcome::Confirmed {},
        Which::ReceiptUnknown(()) => CommandOutcome::Unknown {},
        Which::Failed(code) => CommandOutcome::Failed {
            code: match code.map_err(fail)? {
                Failure::PermissionDenied => FailureCode::PermissionDenied,
                Failure::PreconditionsChanged => FailureCode::PreconditionsChanged,
                Failure::Unsupported => FailureCode::Unsupported,
                Failure::Expired => FailureCode::Expired,
                Failure::AdapterRejected => FailureCode::AdapterRejected,
            },
        },
    };
    let mut receipt = CommandReceipt::accepted(
        request.fields().request.fields().request_id,
        request.fields().request.fields().target,
    )
    .map_err(fail)?;
    receipt.advance(outcome).map_err(fail)?;
    Response::new(&request, Outcome::Receipt { receipt })
}
