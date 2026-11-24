use yolo_hub_file_helper_protocol::{Command, Payload, Request, RootBinding};
use yolo_hub_file_reader::ReadRoot;
use yolo_hub_terminal_protocol::PublicError;

pub fn execute(request: &Request) -> Payload {
    match perform(request) {
        Ok(payload) => payload,
        Err(code) => Payload::Error { code },
    }
}

fn perform(request: &Request) -> Result<Payload, PublicError> {
    let protected = request
        .fields()
        .protected
        .iter()
        .map(std::path::PathBuf::from)
        .collect::<Vec<_>>();
    match &request.fields().command {
        Command::Build {} => yolo_executable_identity::capture_current()
            .map(|image| Payload::Build { image })
            .map_err(|_| PublicError::Internal),
        Command::Inspect { roots } => {
            let readers = roots
                .iter()
                .map(|root| ReadRoot::open(root.clone(), &protected))
                .collect::<Result<Vec<_>, _>>()?;
            let bindings = readers
                .iter()
                .map(|reader| {
                    Ok(RootBinding {
                        root: reader.summary().clone(),
                        fingerprint: reader.fingerprint()?,
                    })
                })
                .collect::<Result<Vec<_>, PublicError>>()?;
            for (reader, binding) in readers.iter().zip(&bindings) {
                if reader.fingerprint()? != binding.fingerprint {
                    return Err(PublicError::Conflict);
                }
            }
            Ok(Payload::Inspected { bindings })
        }
        Command::Read { binding, request } => {
            let reader = ReadRoot::open(binding.root.clone(), &protected)?;
            if reader.fingerprint()? != binding.fingerprint {
                return Err(PublicError::Conflict);
            }
            let file = reader.read(request)?;
            if reader.fingerprint()? != binding.fingerprint {
                return Err(PublicError::Conflict);
            }
            Ok(Payload::Read { file })
        }
    }
}
