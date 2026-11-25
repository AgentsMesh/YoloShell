use super::super::instance_lock::BrokerInstanceLock;
use super::*;
use std::cell::RefCell;
use std::fs;

const STATE_DIRECTORY: &str = crate::remote_broker::constants::TEST_STATE_DIRECTORY;
const DISPLACED_DIRECTORY: &str = "state-displaced";
const ACTION_FAILURE: &str = "injected socket set action failure";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    Success,
    NamespaceHookFailure,
    NamespaceLoss,
    RuntimeHookFailure,
    RuntimeNamespaceLoss,
    ManagementHookFailure,
    ManagementNamespaceLoss,
}

#[tokio::test]
async fn one_socket_set_instance_covers_success_failures_and_compensation() {
    for action in [
        Action::Success,
        Action::NamespaceHookFailure,
        Action::NamespaceLoss,
        Action::RuntimeHookFailure,
        Action::RuntimeNamespaceLoss,
        Action::ManagementHookFailure,
        Action::ManagementNamespaceLoss,
    ] {
        let root = tempfile::tempdir().unwrap();
        let paths = BrokerPaths::new(root.path().join(STATE_DIRECTORY)).unwrap();
        let replacements = RefCell::new(Vec::new());
        let lock = BrokerInstanceLock::acquire(paths.instance_lock()).unwrap();
        let result = BrokerSockets::bind_locked(
            &paths,
            lock,
            || match action {
                Action::NamespaceHookFailure => anyhow::bail!(ACTION_FAILURE),
                Action::NamespaceLoss => {
                    replace_state(&paths, &replacements)?;
                    Ok(())
                }
                _ => Ok(()),
            },
            |_| Ok(()),
            |stage| match (action, stage) {
                (Action::RuntimeHookFailure, BrokerSocketBindStage::RuntimePublished)
                | (Action::ManagementHookFailure, BrokerSocketBindStage::ManagementPublished) => {
                    anyhow::bail!(ACTION_FAILURE)
                }
                (Action::RuntimeNamespaceLoss, BrokerSocketBindStage::RuntimePublished)
                | (Action::ManagementNamespaceLoss, BrokerSocketBindStage::ManagementPublished) => {
                    replace_state(&paths, &replacements)?;
                    Ok(())
                }
                _ => Ok(()),
            },
        )
        .await;

        if action == Action::Success {
            drop(result.unwrap());
            assert!(!paths.runtime_socket().exists());
            assert!(!paths.management_socket().exists());
        } else {
            assert!(result.is_err(), "action {action:?} unexpectedly succeeded");
        }
    }
}

fn replace_state(paths: &BrokerPaths, retained: &RefCell<Vec<BrokerPaths>>) -> anyhow::Result<()> {
    let displaced = paths
        .state_dir()
        .parent()
        .unwrap()
        .join(DISPLACED_DIRECTORY);
    fs::rename(paths.state_dir(), displaced)?;
    retained
        .borrow_mut()
        .push(BrokerPaths::new(paths.state_dir())?);
    Ok(())
}
