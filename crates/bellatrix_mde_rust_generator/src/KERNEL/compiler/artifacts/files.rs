use bellatrix_mde::{Project, Plan, physical_path};
use std::collections::HashSet;

pub fn plan_files(project: &Project, selected: &HashSet<String>, plan: &mut Plan) -> Result<(), Box<dyn std::error::Error>> {
    for element in &project.elements {
        if !selected.contains(&element.id) { continue; }
        match element.kind.as_str() {
            "fs.directory" => {
                let path = physical_path(project, &element.id)?;
                plan.directory(path, &element.id)?;
            }
            "fs.file" => {
                // An empty file can be a placement placeholder for a declaration.
                if element.content.is_none() && project.relations.iter().any(|relation|
                    relation.kind == "declares" && relation.from == element.id && selected.contains(&relation.to)) { continue; }
                let path = physical_path(project, &element.id)?;
                let bytes = match &element.content { Some(content) => content.bytes()?, None => Vec::new() };
                plan.file(path, bytes, &element.id)?;
            }
            _ => (),
        }
    }
    Ok(())
}
