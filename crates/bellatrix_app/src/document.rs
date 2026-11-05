use bellatrix_mde_contracts::{
    model::{Diagram, ModelingSection, Project, View},
    ruml::Registry,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub fn blank() -> Project {
    Project {
        schema_version: 2,
        id: "bellatrix-document".into(),
        name: "Untitled model".into(),
        model: ModelingSection {
            registry: Registry::new(),
            root_model: None,
            authoring_knowledge: Default::default(),
            mdj_provenance: None,
            hierarchy: BTreeMap::new(),
            composition: BTreeMap::new(),
            presentation: BTreeMap::new(),
            diagrams: vec![diagram()],
        },
    }
}

pub fn diagram() -> Diagram {
    Diagram {
        id: "diagram-main".into(),
        name: "Class diagram".into(),
        width: 1600.0,
        height: 1000.0,
        views: Vec::new(),
        uml_di_type: "UMLClassDiagram".into(),
        interaction_kind: None,
        model_elements: Vec::new(),
    }
}

fn validate(project: &Project) -> Result<(), String> {
    if project.schema_version != 2 {
        return Err(format!(
            "Expected schemaVersion 2, found {}",
            project.schema_version
        ));
    }
    if project.id.trim().is_empty() {
        return Err("Project ID must not be empty".into());
    }
    project.model.registry.validate()?;
    let mut diagram_ids = BTreeSet::new();
    let mut view_ids = BTreeSet::new();
    for diagram in &project.model.diagrams {
        if diagram.id.trim().is_empty() || !diagram_ids.insert(diagram.id.as_str()) {
            return Err(format!("Empty or duplicate diagram ID: {}", diagram.id));
        }
        if !diagram.width.is_finite() || !diagram.height.is_finite() {
            return Err("Diagram dimensions must be finite".into());
        }
        validate_views(&diagram.views, &mut view_ids)?;
    }
    Ok(())
}

fn validate_views<'a>(views: &'a [View], ids: &mut BTreeSet<&'a str>) -> Result<(), String> {
    for view in views {
        if view.id.trim().is_empty() || !ids.insert(view.id.as_str()) {
            return Err(format!("Empty or duplicate view ID: {}", view.id));
        }
        view.geometry.validate()?;
        validate_views(&view.children, ids)?;
    }
    Ok(())
}

pub fn destination(path: &Path) -> Result<PathBuf, String> {
    if path.exists() {
        return fs::canonicalize(path)
            .map_err(|error| format!("Cannot resolve {}: {error}", path.display()));
    }
    let filename = path.file_name().ok_or("The destination needs a filename")?;
    let parent = path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::canonicalize(parent)
        .map(|parent| parent.join(filename))
        .map_err(|error| {
            format!(
                "Cannot resolve destination directory {}: {error}",
                parent.display()
            )
        })
}

pub fn read(path: &Path) -> Result<(Project, Vec<u8>), String> {
    let bytes =
        fs::read(path).map_err(|error| format!("Cannot read {}: {error}", path.display()))?;
    let project: Project = serde_json::from_slice(&bytes)
        .map_err(|error| format!("Invalid project JSON in {}: {error}", path.display()))?;
    validate(&project)?;
    Ok((project, bytes))
}

pub fn encode(project: &Project) -> Result<Vec<u8>, String> {
    validate(project)?;
    let mut bytes = serde_json::to_vec_pretty(project).map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    Ok(bytes)
}

pub fn write(path: &Path, bytes: &[u8], expected: Option<&[u8]>) -> Result<(), String> {
    if let Some(expected) = expected {
        let current = fs::read(path).map_err(|error| {
            format!("Cannot verify the saved file at {}: {error}. Use Save As to preserve both versions.", path.display())
        })?;
        if current != expected {
            return Err("The file changed on disk. Use Save As to preserve both versions.".into());
        }
    }
    let parent = path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let filename = path.file_name().ok_or("The destination needs a filename")?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_nanos();
    let mut temporary_name = filename.to_os_string();
    temporary_name.push(format!(".bellatrix-{}-{stamp}.tmp", std::process::id()));
    let temporary: PathBuf = parent.join(temporary_name);
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(&temporary)
        .map_err(|error| format!("Cannot prepare {}: {error}", path.display()))?;
    // A failed write must not truncate the user's existing document.
    let result = (|| {
        if let Ok(metadata) = fs::metadata(path) {
            fs::set_permissions(&temporary, metadata.permissions())?;
        }
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, path)
    })();
    if let Err(error) = result {
        let cleanup = fs::remove_file(&temporary).err();
        return Err(format!(
            "Cannot save {}: {error}{}",
            path.display(),
            cleanup
                .map(|error| format!(
                    "; temporary file {} could not be removed: {error}",
                    temporary.display()
                ))
                .unwrap_or_default()
        ));
    }
    Ok(())
}

pub fn is_recovery_path(path: &Path) -> bool {
    let Some(directory) = dirs::data_local_dir() else {
        return false;
    };
    let directory = directory.join("Bellatrix").join("Recovery");
    let directory = fs::canonicalize(&directory).unwrap_or(directory);
    path.starts_with(directory)
}

pub fn recovery_directory() -> Result<PathBuf, String> {
    let directory = dirs::data_local_dir()
        .ok_or("The current user's local data directory is unavailable")?
        .join("Bellatrix")
        .join("Recovery");
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(&directory).map_err(|error| {
        format!(
            "Cannot prepare recovery directory {}: {error}",
            directory.display()
        )
    })?;
    let metadata = fs::symlink_metadata(&directory).map_err(|error| error.to_string())?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(format!(
            "Recovery destination is not a private directory: {}",
            directory.display()
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(format!(
                "Recovery directory is accessible to other users: {}",
                directory.display()
            ));
        }
    }
    Ok(directory)
}

pub fn recovery_files() -> Result<Vec<PathBuf>, String> {
    let directory = recovery_directory()?;
    let entries = fs::read_dir(&directory).map_err(|error| {
        format!(
            "Cannot read recovery directory {}: {error}",
            directory.display()
        )
    })?;
    let mut files = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| error.to_string())?;
        if entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_dir()
            && entry.file_name().to_string_lossy().starts_with("recovery-")
        {
            let path = entry.path().join("project.json");
            if path.is_file() {
                files.push(path);
            }
        }
    }
    files.sort();
    files.reverse();
    Ok(files)
}

pub fn write_recovery(project: &Project) -> Result<PathBuf, String> {
    let bytes = encode(project)?;
    let directory = recovery_directory()?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_nanos();
    for attempt in 0..100 {
        let snapshot = directory.join(format!("recovery-{stamp}-{}-{attempt}", std::process::id()));
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        match builder.create(&snapshot) {
            Ok(()) => {
                let path = snapshot.join("project.json");
                write(&path, &bytes, None)?;
                return Ok(path);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(format!(
                    "Cannot create recovery snapshot {}: {error}",
                    snapshot.display()
                ));
            }
        }
    }
    Err("Cannot reserve a unique recovery snapshot; previous copies were kept".into())
}
