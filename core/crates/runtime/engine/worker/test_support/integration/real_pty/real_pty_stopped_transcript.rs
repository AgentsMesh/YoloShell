const STOPPED_OUTPUT_TRANSCRIPT_DETAIL: &str =
    "journal is missing one or more acknowledged stopped-output frames";
const STOPPED_OUTPUT_EMPTY_TRANSCRIPT_DETAIL: &str =
    "stopped-output writer acknowledged no transcript frames";

fn prove_stopped_output_transcript(fixture: &Fixture, progress: &Path) {
    let frame_count = read_stopped_output_frame_count(progress).unwrap();
    assert!(
        frame_count > u64::default(),
        "{STOPPED_OUTPUT_EMPTY_TRANSCRIPT_DETAIL}"
    );
    let expected = expected_stopped_output_frames(frame_count);
    let deadline = Instant::now() + STOPPED_OUTPUT_PROGRESS_TIMEOUT;
    loop {
        let output: Vec<u8> = lock_recover(&fixture.state.journal)
            .replay_after(u64::default(), usize::MAX)
            .unwrap()
            .into_iter()
            .flat_map(|chunk| chunk.data)
            .collect();
        if output
            .windows(expected.len())
            .any(|window| window == expected)
        {
            return;
        }
        if Instant::now() >= deadline {
            panic!(
                "{STOPPED_OUTPUT_TRANSCRIPT_DETAIL}: frames={frame_count}, expected={}, journal={}, first_missing={:?}, root_live={}, child_waited={}, exit_present={}, current_frames={:?}",
                expected.len(),
                output.len(),
                first_missing_stopped_output_frame(&output, frame_count),
                fixture_root_identity(fixture).matches(),
                fixture.state.child_waited.load(Ordering::Acquire),
                fixture
                    .state
                    .paths
                    .exit_record(fixture.terminal_id())
                    .exists(),
                read_stopped_output_frame_count(progress).ok(),
            );
        }
        std::thread::sleep(PROCESS_DISCOVERY_POLL_INTERVAL);
    }
}

fn first_missing_stopped_output_frame(output: &[u8], frame_count: u64) -> Option<u64> {
    (0..frame_count).find(|frame| {
        let payload = stopped_output_payload(*frame);
        !output
            .windows(payload.len())
            .any(|window| window == payload)
    })
}

fn expected_stopped_output_frames(frame_count: u64) -> Vec<u8> {
    let frame_count = usize::try_from(frame_count).expect(STOPPED_OUTPUT_TRANSCRIPT_DETAIL);
    let mut expected = Vec::with_capacity(frame_count.saturating_mul(STOPPED_OUTPUT_PAYLOAD_BYTES));
    for frame in 0..frame_count {
        expected.extend_from_slice(&stopped_output_payload(frame as u64));
    }
    expected
}
