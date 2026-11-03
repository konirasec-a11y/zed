//! Staged publication and explicit, ownership-checked regeneration.
use crate::{Plan, ownership};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

/// Publish a plan into an absent or empty output directory.
///
/// # Effects
/// Contains side effects: reads destination metadata and directory entries,
/// consults the process ID for sibling staging/backup names, creates parent and
/// staging directories, writes artifact bytes and `.bellatrix-generated.json`,
/// and renames the staged tree into place. An existing empty destination is moved
/// to a backup and that backup is recursively removed after installation.
/// Cleanup may also recursively remove the staging tree after an error.
/// Does not execute generated programs, install dependencies or access the network.
///
/// # Errors and operational risks
/// Rejects an observed destination symlink, non-directory, nonempty output or
/// existing staging/backup path. Filesystem/permission/storage failures propagate.
/// An error does not guarantee an unchanged filesystem: created parent directories
/// can remain, failed cleanup can leave staging/backup trees, and backup deletion
/// can fail after the new output has already been installed. Rollback attempts
/// after installation failure are best-effort and their errors are not reported.
/// Preflight checks and separate renames are not a lock or an all-or-nothing
/// filesystem transaction. The caller must prevent concurrent modification of
/// the destination and its ancestors; ancestor symlinks are not rejected here.
/// Results depend on filesystem state, platform behavior and process identity.
pub fn publish_new(plan: &Plan, destination: &Path) -> io::Result<()> {
    publish(plan, destination, false)
}

/// Replace a generated output tree using its previous ownership manifest.
///
/// # Effects
/// Contains side effects: performs the reads, directory creation, artifact and
/// manifest writes, process-ID lookup, renames and cleanup described by
/// [`publish_new`]. For nonempty output it additionally reads/parses the existing
/// ownership manifest and recursively inspects directory entries and file types.
/// It moves the old destination to a sibling backup, installs the new staged
/// tree and recursively removes the backup, including formerly generated files
/// omitted by the new plan. An absent or empty destination is also accepted.
/// Does not execute generated programs, install dependencies or access the network.
///
/// # Errors and operational risks
/// Rejects an unreadable/invalid/unsupported manifest, encountered symlinks and
/// regular files absent from the manifest. Ownership is checked by normalized
/// path, not content hash: manually edited files still listed in the manifest
/// can be overwritten or removed. The scan does not explicitly reject every
/// nonregular entry, so the manifest is not proof that the entire tree is safe
/// to discard. Callers must review the target and exclude concurrent writers.
/// Filesystem failures may leave partial external effects, including an installed
/// new output with a retained backup; cleanup/rollback errors may be suppressed.
/// The race, ancestor-symlink and non-transactional limitations of [`publish_new`]
/// apply. This function is not pure or deterministic from the plan alone.
pub fn publish_replace(plan: &Plan, destination: &Path) -> io::Result<()> {
    publish(plan, destination, true)
}

fn publish(plan: &Plan, destination: &Path, replace: bool) -> io::Result<()> {
    if destination.is_symlink() {
        return Err(io::Error::other("output is a symlink"));
    }
    let exists = destination.exists();
    if exists && !destination.is_dir() {
        return Err(io::Error::other("output is not a directory"));
    }
    if exists {
        let nonempty = fs::read_dir(destination)?.next().is_some();
        if nonempty && !replace {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "output is not empty; use --replace-generated for explicit regeneration",
            ));
        }
        if nonempty {
            ownership::verify(destination)?;
        }
    }
    let parent = destination
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    let basename = destination
        .file_name()
        .ok_or_else(|| io::Error::other("invalid output path"))?;
    let prefix = format!(
        ".{}-bellatrix-{}",
        basename.to_string_lossy(),
        std::process::id()
    );
    let stage = parent.join(format!("{prefix}-staging"));
    let backup = parent.join(format!("{prefix}-backup"));
    if stage.exists() || backup.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "staging/backup already exists",
        ));
    }
    fs::create_dir(&stage)?;
    let build = materialize(plan, &stage);
    if let Err(err) = build {
        let _ = fs::remove_dir_all(&stage);
        return Err(err);
    }
    if exists {
        fs::rename(destination, &backup)?;
    }
    match fs::rename(&stage, destination) {
        Ok(()) => {
            if exists {
                fs::remove_dir_all(&backup)?;
            }
            Ok(())
        }
        Err(err) => {
            let _ = fs::remove_dir_all(&stage);
            if exists {
                let _ = fs::rename(&backup, destination);
            }
            Err(err)
        }
    }
}

fn materialize(plan: &Plan, root: &Path) -> io::Result<()> {
    for artifact in plan.artifacts() {
        let path: PathBuf = root.join(&artifact.path);
        if let Some(bytes) = &artifact.contents {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(path, bytes)?;
        } else {
            fs::create_dir_all(path)?;
        }
    }
    ownership::write(root, plan)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn root(name: &str) -> PathBuf {
        let ticks = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("bellatrix-{name}-{}-{ticks}", std::process::id()))
    }
    #[test]
    fn can_regenerate_owned_files_but_never_unknown_files() {
        let dir = root("publish");
        let mut first = Plan::new();
        first.file("src/main.rs", b"one".to_vec(), "test").unwrap();
        publish_new(&first, &dir).unwrap();
        let mut second = Plan::new();
        second.file("src/main.rs", b"two".to_vec(), "test").unwrap();
        assert!(publish_new(&second, &dir).is_err());
        publish_replace(&second, &dir).unwrap();
        assert_eq!(fs::read(dir.join("src/main.rs")).unwrap(), b"two");
        fs::write(dir.join("notes.txt"), b"user data").unwrap();
        assert!(publish_replace(&first, &dir).is_err());
        assert_eq!(fs::read(dir.join("notes.txt")).unwrap(), b"user data");
        fs::remove_dir_all(dir).unwrap();
    }
}
