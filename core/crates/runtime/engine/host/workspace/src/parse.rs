use crate::WorktreeRecord;

pub(crate) fn parse(output: &str) -> Vec<WorktreeRecord> {
    output
        .split("\n\n")
        .filter_map(|block| {
            let mut path = None;
            let mut head = None;
            let mut branch = None;
            for line in block.lines() {
                if let Some(value) = line.strip_prefix("worktree ") {
                    path = Some(value.to_owned());
                } else if let Some(value) = line.strip_prefix("HEAD ") {
                    head = Some(value.to_owned());
                } else if let Some(value) = line.strip_prefix("branch refs/heads/") {
                    branch = Some(value.to_owned());
                }
            }
            Some(WorktreeRecord {
                path: path?,
                branch,
                head,
            })
        })
        .collect()
}
