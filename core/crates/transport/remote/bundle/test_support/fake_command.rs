use super::*;
use std::path::Path;

impl Fake {
    pub(super) fn execute_command(
        &self,
        command: &CommandRequest,
    ) -> Result<CommandOutput, RemoteError> {
        let mut state = self.state.lock().unwrap();
        state.commands.push(command.clone());
        if command.program() == UNAME {
            let mut result = if command
                .args()
                .first()
                .is_some_and(|arg| arg == PLATFORM_ARGUMENT)
            {
                output(if matches!(state.fault, Fault::WrongPlatform) {
                    MACOS
                } else {
                    LINUX
                })
            } else {
                output(if matches!(state.fault, Fault::WrongArchitecture) {
                    ARM
                } else {
                    X86
                })
            };
            result.timed_out = matches!(state.fault, Fault::Timeout);
            result.truncated = matches!(state.fault, Fault::Truncated);
            if matches!(state.fault, Fault::MissingExit) {
                result.exit_status = None;
            }
            return Ok(result);
        }
        let stage = state.staged.clone().unwrap();
        if command
            .args()
            .first()
            .is_some_and(|arg| arg == VERSION_ARGUMENT)
        {
            let role = ArtifactRole::ALL
                .into_iter()
                .find(|role| {
                    Path::new(command.program()).file_name().unwrap() == role.executable_name()
                })
                .unwrap();
            if matches!(state.fault, Fault::WrongVersion(wrong) if wrong == role) {
                return Ok(output(WRONG_VERSION));
            }
            return Ok(output(
                format!(
                    "{} {}\n",
                    role.executable_name(),
                    stage.descriptor().version()
                )
                .as_bytes(),
            ));
        }
        let arguments = command
            .args()
            .strip_prefix(
                &yolo_launch_contract::ProcessRole::Hub
                    .arguments()
                    .map(str::to_owned),
            )
            .expect(MISSING_HUB_ROLE);
        if arguments
            .first()
            .is_some_and(|arg| arg == INSPECT_INSTALL_ARGUMENT)
        {
            let selected = state.selected.clone().map(|selection| {
                (
                    selection,
                    state.committed.as_ref().unwrap().descriptor().clone(),
                )
            });
            let receipt = InspectionReceipt::new(stage.install_root(), selected).unwrap();
            return Ok(output(&encode_inspection(&receipt).unwrap()));
        }
        assert_eq!(arguments.first().unwrap(), INSTALL_BUNDLE_ARGUMENT);
        if matches!(state.fault, Fault::FailedCommit) {
            let mut result = output(&[]);
            result.exit_status = Some(FAILURE);
            return Ok(result);
        }
        let encoded = command
            .args()
            .windows(2)
            .find(|pair| pair[0] == REQUEST_CAPNP_ARGUMENT)
            .unwrap();
        let request = decode_request_argument(&encoded[1]).unwrap();
        let selected = request.selected().unwrap();
        state.selected = Some(selected.clone());
        state.committed = Some(request.clone());
        for role in ArtifactRole::ALL {
            state.files.remove(&stage.executable(role));
            state.files.insert(
                stage
                    .descriptor()
                    .executable_path(stage.install_root(), role),
                stage.payload(role).to_vec(),
            );
        }
        if matches!(state.fault, Fault::Superseded) {
            state.selected = Some(
                Selection::next(Some(&selected), Uuid::new_v4(), selected.build_id()).unwrap(),
            );
        }
        if matches!(
            state.fault,
            Fault::LostAck | Fault::Superseded | Fault::CorruptRecovery
        ) {
            return Err(RemoteError::Transport(LOST_ACK.into()));
        }
        if matches!(state.fault, Fault::InvalidReceipt) {
            return Ok(output(MALFORMED_JSON));
        }
        let receipt = PublicationReceipt::new(&request, stage.install_root(), selected).unwrap();
        Ok(output(&encode_receipt(&receipt).unwrap()))
    }
}
