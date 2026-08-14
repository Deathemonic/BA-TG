use std::collections::HashSet;
use std::fs;
use std::path::Path;

use cunnybuffers::r#gen::GeneratedFile;

use crate::error::BuildError;

pub fn sync(target: &Path, files: &[GeneratedFile]) -> Result<(), BuildError> {
    let mut expected = HashSet::with_capacity(files.len());

    for file in files {
        let path = target.join(&file.path);
        expected.insert(file.path.as_path());

        if fs::read_to_string(&path).is_ok_and(|current| current == file.content) {
            continue;
        }

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, &file.content)?;
    }

    prune(target, target, &expected)
}

fn prune(root: &Path, dir: &Path, expected: &HashSet<&Path>) -> Result<(), BuildError> {
    if !dir.is_dir() {
        return Ok(());
    }

    for entry in fs::read_dir(dir)? {
        let path = entry?.path();

        if path.is_dir() {
            prune(root, &path, expected)?;
            if fs::read_dir(&path)?.next().is_none() {
                fs::remove_dir(&path)?;
            }
        } else if !path.strip_prefix(root).is_ok_and(|path| expected.contains(path)) {
            fs::remove_file(&path)?;
        }
    }

    Ok(())
}
