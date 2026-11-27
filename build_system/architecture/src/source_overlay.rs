use crate::sources::{self, SourceFile};
use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::io;
use std::path::Path;

const COLLISION_DETAIL: &str =
    "workspace source conflicts with a distinct Bazel runfile at the same repository path";

pub fn load(root: &Path, runfiles: Option<&Path>) -> io::Result<Vec<SourceFile>> {
    let primary = sources::load(root)?;
    let Some(runfiles) = runfiles else {
        return Ok(primary);
    };
    merge(primary, sources::load(runfiles)?)
}

fn merge(primary: Vec<SourceFile>, supplemental: Vec<SourceFile>) -> io::Result<Vec<SourceFile>> {
    let mut files = primary
        .into_iter()
        .map(|file| (file.relative.clone(), file))
        .collect::<BTreeMap<_, _>>();
    for file in supplemental {
        match files.entry(file.relative.clone()) {
            Entry::Vacant(entry) => {
                entry.insert(file);
            }
            Entry::Occupied(entry) => validate_identity(entry.get(), &file)?,
        }
    }
    Ok(files.into_values().collect())
}

fn validate_identity(primary: &SourceFile, supplemental: &SourceFile) -> io::Result<()> {
    if primary.absolute.canonicalize()? == supplemental.absolute.canonicalize()? {
        return Ok(());
    }
    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        format!("{}: {COLLISION_DETAIL}", primary.relative),
    ))
}
