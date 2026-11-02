//! Staged publication and explicit, ownership-checked regeneration.
use crate::{Plan, ownership};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

pub fn publish_new(plan: &Plan, destination: &Path) -> io::Result<()> {
    publish(plan, destination, false)
}

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
