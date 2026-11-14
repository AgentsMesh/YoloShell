use super::*;

const INVOCATION_COUNT_EXCEEDED: &str = "agent invocation count exceeded the expected value";
const INVOCATION_LOG_INCOMPLETE: &str = "agent invocation log has an incomplete record";
const INVOCATION_LOG_INVALID: &str = "agent invocation log is invalid";
const INVOCATION_LOG_MISSING: &str = "agent invocation log is unavailable";
const INVOCATION_LOG_TIMEOUT: &str = "agent invocation log did not settle";

pub(super) async fn wait_for_invocation_count(
    data_home: &Path,
    expected: usize,
) -> Vec<fake_agent_invocation::Invocation> {
    let deadline = tokio::time::Instant::now() + AGENT_STATE_SETTLEMENT_TIMEOUT;
    loop {
        match try_invocation_log(data_home) {
            Ok(records) if records.len() == expected => return records,
            Ok(records) => assert!(records.len() < expected, "{INVOCATION_COUNT_EXCEEDED}"),
            Err(_) => {}
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "{INVOCATION_LOG_TIMEOUT}"
        );
        tokio::time::sleep(POLL_INTERVAL).await;
    }
}

pub(super) fn invocation_log(data_home: &Path) -> Vec<fake_agent_invocation::Invocation> {
    try_invocation_log(data_home)
        .unwrap_or_else(|error| panic!("{INVOCATION_LOG_INVALID}: {error}"))
}

pub(super) async fn assert_invocation_log_stable(
    data_home: &Path,
    expected: &[fake_agent_invocation::Invocation],
) {
    let deadline = tokio::time::Instant::now() + INVOCATION_QUIET_PERIOD;
    loop {
        assert_eq!(invocation_log(data_home), expected);
        if tokio::time::Instant::now() >= deadline {
            return;
        }
        tokio::time::sleep(POLL_INTERVAL).await;
    }
}

fn try_invocation_log(data_home: &Path) -> Result<Vec<fake_agent_invocation::Invocation>, String> {
    let path = data_home.join(fake_agent_constants::INVOCATION_LOG_FILE);
    let contents =
        std::fs::read(&path).map_err(|error| format!("{INVOCATION_LOG_MISSING}: {error}"))?;
    if !contents.is_empty()
        && contents.last().copied() != Some(fake_agent_constants::INVOCATION_LINE_ENDING)
    {
        return Err(INVOCATION_LOG_INCOMPLETE.into());
    }
    contents
        .split(|byte| *byte == fake_agent_constants::INVOCATION_LINE_ENDING)
        .filter(|line| !line.is_empty())
        .map(decode_exact)
        .collect()
}

fn decode_exact(line: &[u8]) -> Result<fake_agent_invocation::Invocation, String> {
    let line = std::str::from_utf8(line).map_err(|error| error.to_string())?;
    let record = fake_agent_invocation::decode(line).map_err(|error| error.to_string())?;
    let encoded = fake_agent_invocation::encode(&record.runtime_epoch, &record.arguments)
        .map_err(|error| error.to_string())?;
    let mut expected = line.as_bytes().to_vec();
    expected.push(fake_agent_constants::INVOCATION_LINE_ENDING);
    if encoded != expected {
        return Err(INVOCATION_LOG_INVALID.into());
    }
    Ok(record)
}
