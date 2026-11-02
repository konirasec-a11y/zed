use bellatrix_mde_model::Project;
use bellatrix_mde_modeling::{Plan, physical_path};
use std::collections::HashSet;

pub fn plan_files(
    project: &Project,
    selected: &HashSet<String>,
    plan: &mut Plan,
) -> Result<(), Box<dyn std::error::Error>> {
    for element in &project.elements {
        if !selected.contains(&element.id) {
            continue;
        }
        match element.kind.as_str() {
            "fs.directory" => {
                let path = physical_path(project, &element.id)?;
                plan.directory(path, &element.id)?;
            }
            "fs.file" => {
                // Empty file is a placement placeholder when a declaration owns its content.
                if element.content.is_none()
                    && project.relations.iter().any(|r| {
                        r.kind == "declares" && r.from == element.id && selected.contains(&r.to)
                    })
                {
                    continue;
                }
                let path = physical_path(project, &element.id)?;
                let bytes = match &element.content {
                    Some(content) => content.bytes()?,
                    None => Vec::new(),
                };
                plan.file(path, bytes, &element.id)?;
            }
            _ => (),
        }
    }
    Ok(())
}
