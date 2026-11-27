use super::tests::Fixture;
use super::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::sync::{Arc, Barrier};

#[path = "config_generation_publish_support.rs"]
pub(super) mod support;
use support::*;

const MOVED_SOURCE_NAME: &str = "pending-moved-generation-source";
const MOVED_DESTINATION_NAME: &str = "pending-moved-generation-destination";
const MOVED_ROOT_NAME: &str = "moved-generation-root";
const CONCURRENT_PUBLISHERS: usize = 8;

#[test]
fn source_replacement_fails_closed_without_deleting_replacement() {
    let fixture = Fixture::new();
    let moved = fixture.paths.generations().join(MOVED_SOURCE_NAME);
    let result = stage_broker_config_with_hook(&fixture.source, &fixture.paths, |stage| {
        if stage == GenerationPublishStage::SourceVerified {
            let source = pending_path(&fixture.paths);
            replace_directory(&source, &moved, IMMUTABLE_DIRECTORY_MODE)?;
        }
        Ok(())
    });

    assert!(result.is_err());
    let replacement = fixture.paths.generations().join(format!(
        "{PENDING_GENERATION_PREFIX}{PRIMARY_STAGING_INDEX}{PENDING_GENERATION_PAYLOAD_SUFFIX}"
    ));
    assert_eq!(
        fs::read(replacement.join(SENTINEL_FILE_NAME)).unwrap(),
        SENTINEL_BYTES
    );
    assert!(!moved.join(GENERATION_CONFIG_FILE).exists());
    assert_eq!(published_count(&fixture.paths), 0);
}

#[test]
fn committed_destination_replacement_is_detected_and_preserved() {
    let fixture = Fixture::new();
    let moved = fixture.paths.generations().join(MOVED_DESTINATION_NAME);
    let result = stage_broker_config_with_hook(&fixture.source, &fixture.paths, |stage| {
        if stage == GenerationPublishStage::DestinationCommitted {
            let destination = published_path(&fixture.paths);
            replace_directory(&destination, &moved, IMMUTABLE_DIRECTORY_MODE)?;
        }
        Ok(())
    });

    assert!(result.is_err());
    let replacement = published_path(&fixture.paths);
    assert_eq!(
        fs::read(replacement.join(SENTINEL_FILE_NAME)).unwrap(),
        SENTINEL_BYTES
    );
    assert!(moved.join(GENERATION_CONFIG_FILE).exists());
}

#[test]
fn generation_root_replacement_is_detected_without_touching_new_root() {
    let fixture = Fixture::new();
    let root = fixture.paths.generations();
    let moved = fixture.paths.state_dir().join(MOVED_ROOT_NAME);
    let result = stage_broker_config_with_hook(&fixture.source, &fixture.paths, |stage| {
        if stage == GenerationPublishStage::SourceVerified {
            fs::rename(&root, &moved)?;
            fs::create_dir(&root)?;
            fs::write(root.join(SENTINEL_FILE_NAME), SENTINEL_BYTES)?;
            fs::set_permissions(&root, fs::Permissions::from_mode(PRIVATE_DIRECTORY_MODE))?;
        }
        Ok(())
    });

    assert!(result.is_err());
    assert_eq!(
        fs::read(root.join(SENTINEL_FILE_NAME)).unwrap(),
        SENTINEL_BYTES
    );
    let staged = first_pending(&moved);
    assert!(!staged.join(GENERATION_CONFIG_FILE).exists());
    assert!(
        fs::read_dir(&moved)
            .unwrap()
            .filter_map(Result::ok)
            .all(|entry| entry
                .file_name()
                .to_string_lossy()
                .starts_with(PENDING_GENERATION_PREFIX))
    );
}

#[test]
fn already_published_destination_must_keep_its_opened_identity() {
    let fixture = Fixture::new();
    let first = stage_broker_config(&fixture.source, &fixture.paths).unwrap();
    let destination = first.config_path().parent().unwrap().to_path_buf();
    let moved = fixture.paths.generations().join(MOVED_DESTINATION_NAME);
    let result = stage_broker_config_with_hook(&fixture.source, &fixture.paths, |stage| {
        if stage == GenerationPublishStage::ExistingOpened {
            replace_directory(&destination, &moved, IMMUTABLE_DIRECTORY_MODE)?;
        }
        Ok(())
    });

    assert!(result.is_err());
    assert_eq!(
        fs::read(destination.join(SENTINEL_FILE_NAME)).unwrap(),
        SENTINEL_BYTES
    );
    assert!(moved.join(GENERATION_CONFIG_FILE).exists());
}

#[test]
fn concurrent_content_addressed_publishers_share_one_generation() {
    let fixture = Fixture::new();
    let barrier = Arc::new(Barrier::new(CONCURRENT_PUBLISHERS));
    let threads = (0..CONCURRENT_PUBLISHERS)
        .map(|_| {
            let barrier = Arc::clone(&barrier);
            let source = fixture.source.clone();
            let paths = fixture.paths.clone();
            std::thread::spawn(move || {
                barrier.wait();
                stage_broker_config(&source, &paths)
            })
        })
        .collect::<Vec<_>>();
    let generations = threads
        .into_iter()
        .map(|thread| thread.join().unwrap().unwrap())
        .collect::<Vec<_>>();

    let expected = generations[0].config_path();
    assert!(
        generations
            .iter()
            .all(|value| value.config_path() == expected)
    );
    assert_eq!(published_count(&fixture.paths), 1);
}

#[test]
fn exclusive_publish_accepts_a_concurrently_installed_generation() {
    let fixture = Fixture::new();
    let source = fixture.source.clone();
    let paths = fixture.paths.clone();
    let (entered_tx, entered_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let contender = std::thread::spawn(move || {
        stage_broker_config_with_hook(&source, &paths, |stage| {
            if stage == GenerationPublishStage::SourceVerified {
                entered_tx.send(())?;
                release_rx.recv()?;
            }
            Ok(())
        })
    });
    entered_rx.recv().unwrap();
    let winner = stage_broker_config(&fixture.source, &fixture.paths).unwrap();
    release_tx.send(()).unwrap();
    let contender = contender.join().unwrap().unwrap();

    assert_eq!(contender.config_path(), winner.config_path());
    assert_eq!(published_count(&fixture.paths), 1);
}

#[test]
fn committed_generation_rejects_entry_added_after_initial_scan() {
    let fixture = Fixture::new();
    let result = stage_broker_config_with_hook(&fixture.source, &fixture.paths, |stage| {
        if stage == GenerationPublishStage::DestinationCommitted {
            let destination = published_path(&fixture.paths);
            fs::set_permissions(
                &destination,
                fs::Permissions::from_mode(PRIVATE_DIRECTORY_MODE),
            )?;
            fs::write(destination.join(SENTINEL_FILE_NAME), SENTINEL_BYTES)?;
            fs::set_permissions(
                &destination,
                fs::Permissions::from_mode(IMMUTABLE_DIRECTORY_MODE),
            )?;
        }
        Ok(())
    });

    assert!(result.is_err());
    let destination = published_path(&fixture.paths);
    assert_eq!(
        fs::read(destination.join(SENTINEL_FILE_NAME)).unwrap(),
        SENTINEL_BYTES
    );
}
