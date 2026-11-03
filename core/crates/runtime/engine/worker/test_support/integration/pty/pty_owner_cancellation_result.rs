fn record_owner_cancellation_result(failures: &mut Vec<String>, result: Result<()>) {
    if let Err(error) = result {
        failures.push(error.to_string());
    }
}
