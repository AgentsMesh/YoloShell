pub fn matches(value: &str, pattern: &str) -> bool {
    let values = split(value);
    let patterns = split(pattern);
    matches_segments(&values, &patterns)
}

fn split(value: &str) -> Vec<&str> {
    value.split('/').filter(|part| !part.is_empty()).collect()
}

fn matches_segments(values: &[&str], patterns: &[&str]) -> bool {
    let Some((pattern, remaining_patterns)) = patterns.split_first() else {
        return values.is_empty();
    };
    if *pattern == "**" {
        return matches_segments(values, remaining_patterns)
            || values
                .split_first()
                .is_some_and(|(_, rest)| matches_segments(rest, patterns));
    }
    values.split_first().is_some_and(|(value, rest)| {
        matches_component(value, pattern) && matches_segments(rest, remaining_patterns)
    })
}

fn matches_component(value: &str, pattern: &str) -> bool {
    let value = value.as_bytes();
    let pattern = pattern.as_bytes();
    let mut states = vec![false; value.len() + 1];
    states[0] = true;
    for token in pattern {
        if *token == b'*' {
            for index in 1..states.len() {
                states[index] |= states[index - 1];
            }
        } else {
            for index in (1..states.len()).rev() {
                states[index] = states[index - 1] && value[index - 1] == *token;
            }
            states[0] = false;
        }
    }
    states[value.len()]
}
