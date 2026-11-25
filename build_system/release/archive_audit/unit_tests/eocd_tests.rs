use yolo_release_archive_audit::constants::*;
use yolo_release_archive_audit_test_support::*;

#[test]
fn truncated_and_trailing_archives_are_rejected() {
    let mut truncated = valid_archive().bytes;
    truncated.pop();
    assert_rejected(&truncated);
    let mut trailing = valid_archive().bytes;
    trailing.push(0);
    assert_rejected(&trailing);
}

#[test]
fn structural_duplicate_eocd_is_rejected() {
    let mut duplicate = valid_archive();
    let record = duplicate.bytes[duplicate.eocd_offset..].to_vec();
    duplicate.bytes.extend_from_slice(&record);
    assert_rejected(&duplicate.bytes);
}

#[test]
fn eocd_signature_inside_a_regular_payload_is_accepted() {
    let mut entry = FixtureEntry::regular(PAYLOAD_PATH);
    entry.payload = EOCD_SIGNATURE.to_le_bytes().to_vec();
    entry.crc32 = crc32fast::hash(&entry.payload);
    entry.uncompressed_size = entry.payload.len() as u32;
    assert_accepted(
        &build(&[
            FixtureEntry::directory(ROOT_DIRECTORY_ENTRY.as_bytes()),
            entry,
        ])
        .bytes,
    );
}

#[test]
fn comments_and_multiple_disks_are_rejected() {
    let mut comment = valid_archive();
    let eocd = comment.eocd_offset;
    set_u16(&mut comment.bytes, eocd + EOCD_COMMENT_LENGTH_OFFSET, 1);
    assert_rejected(&comment.bytes);
    for offset in [EOCD_DISK_OFFSET, EOCD_CENTRAL_DISK_OFFSET] {
        let mut fixture = valid_archive();
        let eocd = fixture.eocd_offset;
        set_u16(&mut fixture.bytes, eocd + offset, 1);
        assert_rejected(&fixture.bytes);
    }
}

#[test]
fn zip64_sentinels_and_entry_count_disagreement_are_rejected() {
    for offset in [EOCD_DISK_ENTRIES_OFFSET, EOCD_TOTAL_ENTRIES_OFFSET] {
        let mut fixture = valid_archive();
        let eocd = fixture.eocd_offset;
        set_u16(&mut fixture.bytes, eocd + offset, ZIP64_U16_SENTINEL);
        assert_rejected(&fixture.bytes);
    }
    for offset in [EOCD_CENTRAL_SIZE_OFFSET, EOCD_CENTRAL_OFFSET_OFFSET] {
        let mut fixture = valid_archive();
        let eocd = fixture.eocd_offset;
        set_u32(&mut fixture.bytes, eocd + offset, ZIP64_U32_SENTINEL);
        assert_rejected(&fixture.bytes);
    }
    let mut disagreement = valid_archive();
    let eocd = disagreement.eocd_offset;
    set_u16(&mut disagreement.bytes, eocd + EOCD_DISK_ENTRIES_OFFSET, 1);
    assert_rejected(&disagreement.bytes);
}

#[test]
fn unknown_central_trailing_bytes_are_rejected() {
    let mut fixture = valid_archive();
    let eocd = fixture.eocd_offset;
    set_u16(&mut fixture.bytes, eocd + EOCD_DISK_ENTRIES_OFFSET, 1);
    set_u16(&mut fixture.bytes, eocd + EOCD_TOTAL_ENTRIES_OFFSET, 1);
    assert_rejected(&fixture.bytes);
}
