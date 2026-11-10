struct Fixture {
    root: ProcessIdentity,
    tree: Option<CapturedProcessTree>,
    authority: Option<RootWaitAuthority>,
}

impl Fixture {
    fn spawn() -> Self {
        let child = crate::process::spawn_waitable_test_child(
            Command::new(Path::new(SLEEP_EXECUTABLE)).arg(SLEEP_DURATION),
        );
        let root = ProcessIdentity::capture(child.id()).unwrap();
        let mut tree = CapturedProcessTree::capture(root).unwrap();
        tree.authorize_destructive_for_test();
        let child = Box::new(child) as crate::process::OwnedPtyChild;
        let authority = RootWaitAuthority::prove_owned_child(&child, &tree)
            .unwrap()
            .bind(child);
        Self {
            root,
            tree: Some(tree),
            authority: Some(authority),
        }
    }

    fn take_tree(&mut self) -> CapturedProcessTree {
        self.tree.take().unwrap()
    }

    fn observe_unreaped(&self) -> crate::process::UnreapedRootExit {
        let deadline = Instant::now() + TRANSITION_TIMEOUT;
        loop {
            let observed = self.authority.as_ref().unwrap().poll_exited_unreaped();
            if let Some(proof) = observed.unwrap() {
                return proof;
            }
            assert!(Instant::now() < deadline, "{TRANSITION_TIMEOUT_DETAIL}");
            std::thread::sleep(TRANSITION_POLL_INTERVAL);
        }
    }

    fn reap_and_lose(&mut self) -> (LostRootWaitAuthority, RootGoneWithoutWait) {
        wait_direct_child(self.root.pid).unwrap();
        let authority = self.authority.take().unwrap();
        let error = authority.poll_exited_unreaped().unwrap_err();
        let loss = WaitAuthorityLoss::from_error(&error).unwrap();
        let lost = authority.lose(loss);
        let proof = lost.observe_gone().unwrap().unwrap();
        (lost, proof)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = self.root.signal(libc::SIGCONT);
        let _ = self.root.signal(libc::SIGKILL);
        let _ = wait_direct_child(self.root.pid);
    }
}
