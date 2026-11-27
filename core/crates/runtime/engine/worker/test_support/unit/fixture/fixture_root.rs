struct FixtureRoot {
    child: Option<Box<dyn portable_pty::Child + Send + Sync>>,
    startup: Option<StartupChildGuard>,
    identity: process::ProcessIdentity,
}

type FixtureRootSpawn = fn(
    &portable_pty::PtyPair,
    CommandBuilder,
    &Paths,
    Uuid,
    Uuid,
    Arc<Mutex<Journal>>,
) -> FixtureRoot;

fn ordinary_fixture_root(
    pair: &portable_pty::PtyPair,
    command: CommandBuilder,
    _paths: &Paths,
    _session_id: Uuid,
    _runtime_epoch: Uuid,
    _journal: Arc<Mutex<Journal>>,
) -> FixtureRoot {
    let child = spawn_waitable_pty_root(pair.slave.as_ref(), command).unwrap();
    let identity = process::ProcessIdentity::capture(child.process_id().unwrap()).unwrap();
    FixtureRoot {
        child: Some(child),
        startup: None,
        identity,
    }
}

impl Fixture {
    fn spawn_reader(&self, reader: Box<dyn Read + Send>) {
        let mut startup = lock_recover(&self.startup);
        if let Some(startup) = startup.as_mut() {
            startup
                .activate_reader(
                    pty_reader_activation::stage_reader(reader, Arc::clone(&self.state)).unwrap(),
                )
                .unwrap();
        } else {
            spawn_reader(reader, Arc::clone(&self.state)).unwrap();
        }
    }
}
