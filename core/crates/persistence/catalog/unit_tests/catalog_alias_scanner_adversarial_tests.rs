include!("../test_support/catalog_alias_scanner_adversarial_support.rs");

#[test]
fn released_last_shared_lease_allows_shard_reclamation() {
    let fixture = Fixture::new();
    let catalog = fixture._temp.path().join("live/catalog.sqlite");
    let shard_path = fixture.shard_path("live");
    let retained =
        CatalogAliasShard::retain_for_test(&catalog, fixture.root.try_clone().unwrap()).unwrap();
    run_pass(&mut fixture.scanner());
    assert!(shard_path.is_dir());

    drop(retained);
    run_pass(&mut fixture.scanner());

    assert!(!shard_path.exists());
}

#[test]
fn aliases_beyond_one_chunk_converge() {
    let fixture = Fixture::new();
    let shard = fixture.shard("many");
    drop(CatalogAliasLease::try_exclusive(&shard).unwrap().unwrap());
    for offset in 0..(ENTRY_BUDGET + EXTRA_ALIAS_COUNT) {
        let id = uuid::Uuid::from_u128(FIRST_ALIAS_ID + offset as u128);
        let name = format!("{}{id}", super::secure_vfs_alias_io::ALIAS_PREFIX);
        create_unlocked_alias(&shard.path().join(name));
    }

    run_pass(&mut fixture.scanner());

    assert!(!shard.path().exists());
}

#[test]
fn busy_alias_owner_does_not_starve_an_unrelated_shard() {
    let fixture = Fixture::new();
    let busy_shard = fixture.shard("busy");
    let healthy_shard = fixture.shard("healthy");
    drop(
        CatalogAliasLease::try_exclusive(&healthy_shard)
            .unwrap()
            .unwrap(),
    );
    let id = uuid::Uuid::from_u128(BUSY_ALIAS_ID);
    let alias = busy_shard
        .path()
        .join(format!("{}{id}", super::secure_vfs_alias_io::ALIAS_PREFIX));
    private_directory(&alias);
    let opened = super::secure_vfs_alias_directory::open_verified(&alias).unwrap();
    let lock = super::secure_vfs_alias_io::create_lock_file(&opened, &alias).unwrap();

    run_pass(&mut fixture.scanner());

    assert!(busy_shard.path().is_dir());
    assert!(!healthy_shard.path().exists());
    drop(lock);
}

#[test]
fn lifecycle_contention_is_busy_without_mutating_aliases() {
    let fixture = Fixture::new();
    let _lifecycle = crate::coordination::try_lifecycle(&fixture.root)
        .unwrap()
        .unwrap();
    let mut scanner = fixture.scanner();

    assert!(!scanner.label().is_empty());
    assert_eq!(scanner.scan_chunk().unwrap(), ScanChunk::Busy);
    assert!(fixture.aliases.path().is_dir());
}

#[test]
fn shard_remove_boundary_preserves_new_content_and_competing_leases() {
    let content_fixture = Fixture::new();
    let content_shard = content_fixture.shard(CONTENT_SHARD);
    drop(
        CatalogAliasLease::try_exclusive(&content_shard)
            .unwrap()
            .unwrap(),
    );
    super::catalog_alias_maintenance_hooks::set_remove_hook(|path| {
        std::fs::write(path.join(UNKNOWN_SHARD_ENTRY), UNKNOWN_SHARD_BYTES).unwrap();
    });

    run_pass(&mut content_fixture.scanner());

    assert_eq!(
        std::fs::read(content_shard.path().join(UNKNOWN_SHARD_ENTRY)).unwrap(),
        UNKNOWN_SHARD_BYTES
    );

    let lease_fixture = Fixture::new();
    let lease_shard = lease_fixture.shard(LEASE_SHARD);
    drop(
        CatalogAliasLease::try_exclusive(&lease_shard)
            .unwrap()
            .unwrap(),
    );
    let held = std::rc::Rc::new(std::cell::RefCell::new(None));
    let captured = std::rc::Rc::clone(&held);
    let aliases = lease_fixture.aliases.try_clone().unwrap();
    super::catalog_alias_maintenance_hooks::set_remove_hook(move |path| {
        let reopened = aliases.open_descendant(path).unwrap();
        *captured.borrow_mut() = CatalogAliasLease::try_shared(&reopened).unwrap();
    });

    run_pass(&mut lease_fixture.scanner());

    assert!(lease_shard.path().is_dir());
    drop(held.borrow_mut().take());
    run_pass(&mut lease_fixture.scanner());
    assert!(!lease_shard.path().exists());
}

#[test]
fn disappeared_shard_is_a_preserved_remove_boundary() {
    let fixture = Fixture::new();
    let shard = fixture.shard(DISAPPEARED_SHARD);
    drop(CatalogAliasLease::try_exclusive(&shard).unwrap().unwrap());
    let path = shard.path().to_path_buf();
    super::catalog_alias_maintenance_hooks::set_remove_hook(|candidate| {
        std::fs::remove_dir_all(candidate).unwrap();
    });

    run_pass(&mut fixture.scanner());

    assert!(!path.exists());
}
