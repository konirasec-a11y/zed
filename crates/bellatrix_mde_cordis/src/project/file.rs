use super::{ProjectDocument, ProjectError};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub(crate) fn load(path: &Path) -> Result<ProjectDocument, ProjectError> {
    let doc: ProjectDocument = serde_json::from_slice(&fs::read(path)?)?;
    doc.validate()?;
    Ok(doc)
}

pub(crate) fn atomic_write(
    path: &Path,
    bytes: &[u8],
    expected_bytes: Option<&[u8]>,
) -> Result<(), ProjectError> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    // Never silently create unrelated parent directories while saving a bellatrix-project.
    if !parent.is_dir() {
        return Err(ProjectError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "bellatrix-project parent directory does not exist",
        )));
    }
    let (temporary, mut file) = create_temporary(parent)?;
    let outcome = (|| -> Result<(), ProjectError> {
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        // Compare after staging and syncing to minimize the external-write window.
        // Another process can still write between this check and rename: this is not CAS.
        match fs::symlink_metadata(path) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
                return Err(ProjectError::DestinationChanged(path.display().to_string()));
            }
            Ok(_) => {
                let current = fs::read(path)?;
                if expected_bytes != Some(current.as_slice()) {
                    return Err(ProjectError::DestinationChanged(path.display().to_string()));
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                if expected_bytes.is_some() {
                    return Err(ProjectError::DestinationChanged(path.display().to_string()));
                }
            }
            Err(error) => return Err(error.into()),
        }
        fs::rename(&temporary, path)?;
        Ok(())
    })();
    if outcome.is_err()
        && let Err(error) = fs::remove_file(&temporary)
    {
        eprintln!(
            "Could not remove failed save staging file {}: {error}",
            temporary.display()
        );
    }
    outcome
}

pub(crate) fn publish_new(path: &Path, bytes: &[u8]) -> Result<Option<String>, ProjectError> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let parent = parent.canonicalize()?;
    let name = path
        .file_name()
        .ok_or_else(|| ProjectError::Io(std::io::Error::other("new file requires a filename")))?;
    let destination = parent.join(name);
    let (temporary, mut file) = create_temporary(&parent)?;
    let written = file.write_all(bytes).and_then(|()| file.sync_all());
    drop(file);
    let published = written.and_then(|()| {
        // A hard link publishes complete bytes atomically and fails if *any*
        // destination entry exists. Rename would permit concurrent overwrite.
        fs::hard_link(&temporary, &destination)
    });
    let cleanup = fs::remove_file(&temporary);
    match published {
        Ok(()) => Ok(cleanup.err().map(|error| {
            format!(
                "File published, but its staging link {} could not be removed: {error}",
                temporary.display()
            )
        })),
        Err(error) => {
            if let Err(cleanup) = cleanup {
                eprintln!(
                    "Could not remove failed save staging file {}: {cleanup}",
                    temporary.display()
                );
            }
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                Err(ProjectError::DestinationExists(path.display().to_string()))
            } else {
                Err(ProjectError::Io(error))
            }
        }
    }
}

fn create_temporary(parent: &Path) -> Result<(PathBuf, std::fs::File), ProjectError> {
    for _ in 0..32 {
        let id = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = parent.join(format!(".bellatrix-{}-{id}.tmp", std::process::id()));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e.into()),
        }
    }
    Err(ProjectError::Io(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "could not allocate a temporary bellatrix-project file",
    )))
}
