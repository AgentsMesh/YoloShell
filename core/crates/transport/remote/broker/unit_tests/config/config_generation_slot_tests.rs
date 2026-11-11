use super::*;
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::sync::mpsc;

#[path = "../../test_support/config_generation_slot.rs"]
mod support;
use support::*;

#[test]
fn late_slot_opener_cannot_claim_or_clean_an_already_published_payload() {
    let fixture = Fixture::new();
    let mut winner = PendingDirectory::create(&fixture.root).unwrap();
    let slot_inode = fs::metadata(&fixture.slot).unwrap().ino();
    let contender_root = GenerationRoot::ensure(fixture.root.path()).unwrap();
    let (opened_tx, opened_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let contender = std::thread::spawn(move || {
        PendingDirectory::create_with_hooks(&contender_root, &mut |_| Ok(()), &mut |_| {
            opened_tx.send(())?;
            release_rx.recv_timeout(GATE_TIMEOUT)?;
            Ok(())
        })
    });
    opened_rx.recv_timeout(GATE_TIMEOUT).unwrap();

    fixture.publish(&mut winner, FIRST_GENERATION, ORIGINAL);
    drop(winner);
    // Another revision reuses the same lock owner before the late opener locks.
    let mut next = PendingDirectory::create(&fixture.root).unwrap();
    fixture.publish(&mut next, SECOND_GENERATION, REPLACEMENT);
    drop(next);
    release_tx.send(()).unwrap();
    let contender = contender.join().unwrap().unwrap();
    assert_eq!(fs::metadata(&fixture.slot).unwrap().ino(), slot_inode);
    assert_eq!(contender.path, fixture.payload);
    assert!(!contender.path.join(GENERATION_CONFIG_FILE).exists());
    drop(contender);

    for (target, bytes) in [
        (FIRST_GENERATION, ORIGINAL),
        (SECOND_GENERATION, REPLACEMENT),
    ] {
        assert_sealed(&fixture.root.path().join(target), bytes);
        fixture
            .root
            .open_generation(&entry_name(target).unwrap())
            .unwrap()
            .unwrap();
    }
}

#[test]
fn changed_fixed_slot_is_rejected_before_any_payload_or_mode_mutation() {
    let fixture = Fixture::new();
    fixture.stale_payload();
    let moved = fixture.base.path().join(MOVED);
    let result = PendingDirectory::create_with_hooks(&fixture.root, &mut |_| Ok(()), &mut |_| {
        fs::rename(&fixture.slot, &moved)?;
        write_sealed(&fixture.slot, REPLACEMENT);
        Ok(())
    });
    assert!(result.is_err());
    assert_sealed(&fixture.payload, ORIGINAL);
    assert_sealed(&fixture.slot, REPLACEMENT);
    assert_eq!(
        fs::metadata(&fixture.slot).unwrap().permissions().mode() & PERMISSION_BITS_MASK,
        IMMUTABLE_DIRECTORY_MODE
    );
}

#[test]
fn changed_root_is_rejected_before_claimed_payload_cleanup() {
    let fixture = Fixture::new();
    fixture.stale_payload();
    let moved = fixture.base.path().join(MOVED);
    let result = PendingDirectory::create_with_hooks(&fixture.root, &mut |_| Ok(()), &mut |_| {
        fs::rename(fixture.root.path(), &moved)?;
        write_sealed(&fixture.payload, REPLACEMENT);
        Ok(())
    });
    assert!(result.is_err());
    let old_payload = moved.join(fixture.payload.file_name().unwrap());
    assert_sealed(&old_payload, ORIGINAL);
    assert_sealed(&fixture.payload, REPLACEMENT);
}

#[test]
fn unexpected_entries_never_become_a_recyclable_slot_payload() {
    let fixture = Fixture::new();
    fs::create_dir(&fixture.slot).unwrap();
    fs::set_permissions(
        &fixture.slot,
        fs::Permissions::from_mode(PRIVATE_DIRECTORY_MODE),
    )
    .unwrap();
    fs::write(fixture.slot.join(GENERATION_CONFIG_FILE), ORIGINAL).unwrap();
    assert!(PendingDirectory::create(&fixture.root).is_err());
    assert_eq!(
        fs::read(fixture.slot.join(GENERATION_CONFIG_FILE)).unwrap(),
        ORIGINAL
    );
    assert!(!fixture.payload.exists());
}

#[test]
fn mode_change_while_opening_a_slot_is_rejected_before_normalization() {
    let fixture = Fixture::new();
    fixture.stale_payload();
    let result = PendingDirectory::create_with_hooks(&fixture.root, &mut |_| Ok(()), &mut |_| {
        fs::set_permissions(
            &fixture.slot,
            fs::Permissions::from_mode(IMMUTABLE_DIRECTORY_MODE),
        )?;
        Ok(())
    });
    assert!(result.is_err());
    assert_sealed(&fixture.payload, ORIGINAL);
    assert_eq!(
        fs::metadata(&fixture.slot).unwrap().permissions().mode() & PERMISSION_BITS_MASK,
        IMMUTABLE_DIRECTORY_MODE
    );
}
