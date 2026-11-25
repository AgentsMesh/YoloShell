//! Connector observation ingress. Enrollment scopes are owner-authorized; connector
//! publications never declare custody, recovery authority, capabilities, or effects.

mod activation;
mod command;
mod constants;
mod credential;
mod enrollment;
mod error;
mod publication;
mod reply;
mod reply_wire;
mod reported_state;
mod request;
mod response;
mod scope;
mod validation;

pub use activation::Activation;
pub use command::Command;
pub use constants::*;
pub use credential::ConnectorCredential;
pub use enrollment::{Enrollment, EnrollmentStatus, EnrollmentSummary};
pub use error::ConnectorError;
pub use publication::{Publication, PublicationAck};
pub use reply::Reply;
pub use reported_state::ReportedState;
pub use request::Request;
pub use response::Response;
pub use scope::ConnectorScope;
