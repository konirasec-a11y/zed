use bellatrix_mde_model::Project;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PathError {
    #[error("unsafe filesystem name: {0}")]
    InvalidName(String),
    #[error("unknown element: {0}")]
    Unknown(String),
    #[error("physical hierarchy contains a cycle at {0}")]
    Cycle(String),
}

pub fn validate_segment(name: &str) -> Result<(), PathError> {
    let upper = name.trim_end_matches(['.', ' ']).to_ascii_uppercase();
    let base = upper.split('.').next().unwrap_or("");
    let reserved = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    if name.is_empty()
        || name == "."
        || name == ".."
        || (name.ends_with('.') || name.ends_with(' '))
        || name
            .chars()
            .any(|c| c.is_control() || "\\/:*?\"<>|".contains(c))
        || reserved.contains(&base)
    {
        return Err(PathError::InvalidName(name.to_string()));
    }
    Ok(())
}

/// Returns a relative filesystem path. A disconnected node goes to output root.
pub fn physical_path(project: &Project, id: &str) -> Result<PathBuf, PathError> {
    let mut segments = Vec::new();
    let mut current = id.to_string();
    let mut visited = std::collections::HashSet::new();
    loop {
        if !visited.insert(current.clone()) {
            return Err(PathError::Cycle(current));
        }
        let element = project
            .element(&current)
            .ok_or_else(|| PathError::Unknown(current.clone()))?;
        if matches!(element.kind.as_str(), "fs.directory" | "fs.file") {
            validate_segment(&element.name)?;
            segments.push(element.name.clone());
        }
        let parent = project
            .relations
            .iter()
            .find(|r| r.kind == "physical_parent" && r.to == current);
        match parent {
            Some(r) => current = r.from.clone(),
            None => break,
        }
    }
    segments.reverse();
    Ok(segments.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn blocks_traversal_and_windows_reserved() {
        for name in ["..", "../x", "C:", "CON", "name.", "file\\path"] {
            assert!(validate_segment(name).is_err(), "{name}");
        }
        for name in [".cargo", "config.toml", "clippy.toml", "main.rs"] {
            validate_segment(name).unwrap();
        }
    }
}

#[cfg(test)]
mod placement_tests {
    use super::*;
    #[test]
    fn absent_parent_places_file_at_root() {
        let project: Project =
            serde_json::from_str(include_str!("../../bellatrix_mde_model/fixtures/todo.json")).unwrap();
        assert_eq!(
            physical_path(&project, "file-clippy").unwrap(),
            PathBuf::from("clippy.toml")
        );
        assert_eq!(
            physical_path(&project, "file-config").unwrap(),
            PathBuf::from(".cargo/config.toml")
        );
    }
}
