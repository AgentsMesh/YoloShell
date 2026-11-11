use crate::host::tests::*;
const PUBLISH_WAVE4_HOOK_ERROR: &str = "publish receipt hook failure";
const PUBLISH_WAVE4_SAVED_SOURCE: &str = "saved-publish-source";
const PUBLISH_WAVE4_SAVED_TARGET: &str = "saved-publish-target";
const PUBLISH_WAVE4_SOURCE_REPLACEMENT: &[u8] = b"publish source replacement";
const PUBLISH_WAVE4_ACTIVE_REAPPEARANCE: &[u8] = b"active staging reappearance";
const PUBLISH_WAVE4_SLOT_REAPPEARANCE: &[u8] = b"retired slot reappearance";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PublishWave4Fault {
    None,
    ExistingIdentical,
    ExistingConflict,
    HookFailure,
    TargetIdentical,
    TargetConflict,
    SourceReplacement,
    PublishedRemoval,
    PublishedReplacement,
    ActiveReappearance,
    ActiveAndSlotReappearance,
}

#[test]
fn publish_receipt_faults_converge_through_one_hook_instance() {
    for fault in [
        PublishWave4Fault::None,
        PublishWave4Fault::ExistingIdentical,
        PublishWave4Fault::ExistingConflict,
        PublishWave4Fault::HookFailure,
        PublishWave4Fault::TargetIdentical,
        PublishWave4Fault::TargetConflict,
        PublishWave4Fault::SourceReplacement,
        PublishWave4Fault::PublishedRemoval,
        PublishWave4Fault::PublishedReplacement,
        PublishWave4Fault::ActiveReappearance,
        PublishWave4Fault::ActiveAndSlotReappearance,
    ] {
        let (_directory, store, binding, _) = receipt_test_setup();
        let receipt = CreationReceipt::from_binding(&binding, CreationCompletion::Finalized);
        let mut conflict = receipt.clone();
        conflict.completion = CreationCompletion::Archived;
        let encoded = serde_json::to_vec(&receipt).unwrap();
        let conflicting = serde_json::to_vec(&conflict).unwrap();
        let name = binding.creation_token_digest.as_str();
        let target = store.token_index.path.join(name);
        let active = receipt_active_path(&store.staging, &store.token_index, name);
        let slot = cleanup_wave4_slot_path(&store.staging, &active);
        let saved_source = store.staging.path.join(PUBLISH_WAVE4_SAVED_SOURCE);
        let saved_target = store.token_index.path.join(PUBLISH_WAVE4_SAVED_TARGET);
        match fault {
            PublishWave4Fault::ExistingIdentical => {
                publish_wave4_write_target(&target, &encoded);
            }
            PublishWave4Fault::ExistingConflict => {
                publish_wave4_write_target(&target, &conflicting);
            }
            _ => {}
        }

        let result = publish_receipt_at_with_test_hook(
            &store.staging,
            &store.token_index,
            name,
            &receipt,
            |stage| match (fault, stage) {
                (PublishWave4Fault::HookFailure, ReceiptPublishStage::BeforePublish) => {
                    anyhow::bail!(PUBLISH_WAVE4_HOOK_ERROR)
                }
                (PublishWave4Fault::TargetIdentical, ReceiptPublishStage::BeforePublish) => {
                    publish_wave4_write_target(&target, &encoded);
                    Ok(())
                }
                (PublishWave4Fault::TargetConflict, ReceiptPublishStage::BeforePublish) => {
                    publish_wave4_write_target(&target, &conflicting);
                    Ok(())
                }
                (PublishWave4Fault::SourceReplacement, ReceiptPublishStage::BeforePublish) => {
                    std::fs::rename(&active, &saved_source)?;
                    publish_wave4_write_target(&active, PUBLISH_WAVE4_SOURCE_REPLACEMENT);
                    Ok(())
                }
                (PublishWave4Fault::PublishedRemoval, ReceiptPublishStage::Published) => {
                    std::fs::remove_file(&target).map_err(Into::into)
                }
                (PublishWave4Fault::PublishedReplacement, ReceiptPublishStage::Published) => {
                    std::fs::rename(&target, &saved_target)?;
                    publish_wave4_write_target(&target, &conflicting);
                    Ok(())
                }
                (PublishWave4Fault::ActiveReappearance, ReceiptPublishStage::Published) => {
                    publish_wave4_write_target(&active, PUBLISH_WAVE4_ACTIVE_REAPPEARANCE);
                    Ok(())
                }
                (PublishWave4Fault::ActiveAndSlotReappearance, ReceiptPublishStage::Published) => {
                    publish_wave4_write_target(&active, PUBLISH_WAVE4_ACTIVE_REAPPEARANCE);
                    std::fs::write(&slot, PUBLISH_WAVE4_SLOT_REAPPEARANCE)?;
                    set_private_file_mode(&slot);
                    Ok(())
                }
                _ => Ok(()),
            },
        );

        let succeeds = matches!(
            fault,
            PublishWave4Fault::None
                | PublishWave4Fault::ExistingIdentical
                | PublishWave4Fault::TargetIdentical
        );
        assert_eq!(result.is_ok(), succeeds, "unexpected outcome for {fault:?}");
        match fault {
            PublishWave4Fault::PublishedRemoval => assert!(!target.exists()),
            PublishWave4Fault::PublishedReplacement
            | PublishWave4Fault::ExistingConflict
            | PublishWave4Fault::TargetConflict => {
                assert_eq!(std::fs::read(target).unwrap(), conflicting)
            }
            PublishWave4Fault::SourceReplacement => {
                assert!(!target.exists());
                assert_eq!(
                    std::fs::read(active).unwrap(),
                    PUBLISH_WAVE4_SOURCE_REPLACEMENT
                );
                assert_eq!(std::fs::read(saved_source).unwrap(), encoded);
            }
            PublishWave4Fault::HookFailure => assert!(!target.exists()),
            _ => {
                assert_eq!(std::fs::read(target).unwrap(), encoded);
            }
        }
    }
}

fn publish_wave4_write_target(path: &Path, bytes: &[u8]) {
    std::fs::write(path, bytes).unwrap();
    set_receipt_path_mode(path, CREATION_RECEIPT_FILE_MODE);
}
