use yolo_release_archive_audit::constants::*;
use yolo_release_archive_audit_test_support::*;

fn set_entry_sizes(fixture: &mut ZipFixture, index: usize, compressed: u32, uncompressed: u32) {
    let local = fixture.entries[index].local;
    let central = fixture.entries[index].central;
    set_u32(
        &mut fixture.bytes,
        local + LOCAL_COMPRESSED_OFFSET,
        compressed,
    );
    set_u32(
        &mut fixture.bytes,
        local + LOCAL_UNCOMPRESSED_OFFSET,
        uncompressed,
    );
    set_u32(
        &mut fixture.bytes,
        central + CENTRAL_COMPRESSED_OFFSET,
        compressed,
    );
    set_u32(
        &mut fixture.bytes,
        central + CENTRAL_UNCOMPRESSED_OFFSET,
        uncompressed,
    );
}

#[test]
fn declared_entry_sizes_and_compression_ratio_are_bounded() {
    let mut compressed = valid_archive();
    set_entry_sizes(
        &mut compressed,
        PAYLOAD_INDEX,
        (MAX_ENTRY_COMPRESSED_BYTES + 1) as u32,
        0,
    );
    assert_rejected(&compressed.bytes);

    let mut uncompressed = valid_archive();
    set_entry_sizes(
        &mut uncompressed,
        PAYLOAD_INDEX,
        1,
        (MAX_ENTRY_UNCOMPRESSED_BYTES + 1) as u32,
    );
    assert_rejected(&uncompressed.bytes);

    let mut ratio = valid_archive();
    let local = ratio.entries[PAYLOAD_INDEX].local;
    let central = ratio.entries[PAYLOAD_INDEX].central;
    set_u16(
        &mut ratio.bytes,
        local + LOCAL_METHOD_OFFSET,
        DEFLATE_METHOD,
    );
    set_u16(
        &mut ratio.bytes,
        central + CENTRAL_METHOD_OFFSET,
        DEFLATE_METHOD,
    );
    set_entry_sizes(
        &mut ratio,
        PAYLOAD_INDEX,
        1,
        (MAX_COMPRESSION_RATIO + COMPRESSION_RATIO_SLACK_BYTES + 1) as u32,
    );
    assert_rejected(&ratio.bytes);
}

#[test]
fn total_declared_uncompressed_size_is_bounded() {
    let mut fixture = build(&[
        FixtureEntry::directory(ROOT_DIRECTORY_ENTRY.as_bytes()),
        FixtureEntry::regular(b"Yolo.app/Contents/one"),
        FixtureEntry::regular(b"Yolo.app/Contents/two"),
        FixtureEntry::regular(b"Yolo.app/Contents/three"),
    ]);
    let per_entry = MAX_ENTRY_UNCOMPRESSED_BYTES as u32;
    let compressed = (MAX_ENTRY_UNCOMPRESSED_BYTES / MAX_COMPRESSION_RATIO) as u32;
    for index in 1..fixture.entries.len() {
        let local = fixture.entries[index].local;
        let central = fixture.entries[index].central;
        set_u16(
            &mut fixture.bytes,
            local + LOCAL_METHOD_OFFSET,
            DEFLATE_METHOD,
        );
        set_u16(
            &mut fixture.bytes,
            central + CENTRAL_METHOD_OFFSET,
            DEFLATE_METHOD,
        );
        set_entry_sizes(&mut fixture, index, compressed, per_entry);
    }
    assert_rejected(&fixture.bytes);
}

#[test]
fn entry_count_central_size_and_name_length_are_bounded() {
    let mut count = valid_archive();
    let eocd = count.eocd_offset;
    let excessive_count = (MAX_ENTRY_COUNT + 1) as u16;
    set_u16(
        &mut count.bytes,
        eocd + EOCD_DISK_ENTRIES_OFFSET,
        excessive_count,
    );
    set_u16(
        &mut count.bytes,
        eocd + EOCD_TOTAL_ENTRIES_OFFSET,
        excessive_count,
    );
    assert_rejected(&count.bytes);

    let mut central = valid_archive();
    let eocd = central.eocd_offset;
    set_u32(
        &mut central.bytes,
        eocd + EOCD_CENTRAL_SIZE_OFFSET,
        (MAX_CENTRAL_BYTES + 1) as u32,
    );
    assert_rejected(&central.bytes);

    let long_name = vec![b'a'; MAX_ENTRY_NAME_BYTES + 1];
    let mut path = ROOT_DIRECTORY_ENTRY.as_bytes().to_vec();
    path.extend_from_slice(&long_name);
    assert_rejected(
        &build(&[
            FixtureEntry::directory(ROOT_DIRECTORY_ENTRY.as_bytes()),
            FixtureEntry::regular(&path),
        ])
        .bytes,
    );
}

#[test]
fn zip64_entry_fields_are_rejected() {
    for offset in [
        CENTRAL_COMPRESSED_OFFSET,
        CENTRAL_UNCOMPRESSED_OFFSET,
        CENTRAL_LOCAL_OFFSET,
    ] {
        let mut fixture = valid_archive();
        let central = fixture.entries[PAYLOAD_INDEX].central;
        set_u32(&mut fixture.bytes, central + offset, ZIP64_U32_SENTINEL);
        assert_rejected(&fixture.bytes);
    }
}
