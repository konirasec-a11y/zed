//! Tracks which files Bellatrix may replace without deleting unrelated user data.
use crate::{Plan, ownership_manifest::{self, MARKER}};
use std::{fs, io, path::{Path, PathBuf}};

pub(crate) fn write(root: &Path, plan: &Plan) -> io::Result<()> {
    let bytes = ownership_manifest::encode(plan).map_err(io::Error::other)?;
    fs::write(root.join(MARKER), bytes)
}

/// For regeneration, EVERY existing file must be in the previous generation manifest.
/// Symlinks are rejected to prevent writes from following external destinations.
pub(crate) fn verify(root: &Path) -> io::Result<()> {
    let marker = fs::read(root.join(MARKER)).map_err(|_| {
        io::Error::other("no Bellatrix generation manifest; refusing to replace this directory")
    })?;
    let ownership = ownership_manifest::decode(&marker).map_err(io::Error::other)?;
    if !ownership.is_supported() {
        return Err(io::Error::other("unsupported generation manifest"));
    }
    let mut pending = vec![PathBuf::new()];
    while let Some(relative) = pending.pop() {
        for entry in fs::read_dir(root.join(&relative))? {
            let entry = entry?;
            let rel = relative.join(entry.file_name());
            let ft = entry.file_type()?;
            if ft.is_symlink() {
                return Err(io::Error::other(format!(
                    "symlink in generated output: {}",
                    rel.display()
                )));
            }
            if ft.is_dir() {
                pending.push(rel);
            } else if ft.is_file()
                && rel != Path::new(MARKER)
                && !ownership.owns_file(&rel)
            {
                return Err(io::Error::other(format!(
                    "unknown file in generated output: {}",
                    rel.display()
                )));
            }
        }
    }
    Ok(())
}
