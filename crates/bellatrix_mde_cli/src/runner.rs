use bellatrix_mde_cordis::ModelingOperation;
use bellatrix_mde_model::{Format, Project, decode, encode};
use bellatrix_mde_modeling::{Graph, publish_new, publish_replace};
use std::{fs, path::Path};

fn format(path: &Path) -> Format {
    if matches!(
        path.extension().and_then(|s| s.to_str()),
        Some("pb" | "bellatrix")
    ) {
        Format::Protobuf
    } else {
        Format::Json
    }
}

fn load(path: &Path) -> Result<Project, String> {
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    decode(&bytes, format(path)).map_err(|e| e.to_string())
}

pub fn run(op: ModelingOperation) -> Result<String, String> {
    match op {
        ModelingOperation::Check { source } => {
            let model = load(&source)?;
            Graph::new(&model).validate().map_err(|e| e.to_string())?;
            Ok(format!(
                "Valid bellatrix-mde-model: {} elements, {} relations, {} diagrams",
                model.elements.len(),
                model.relations.len(),
                model.diagrams.len()
            ))
        }
        ModelingOperation::Inspect { source } => {
            let model = load(&source)?;
            Graph::new(&model).validate().map_err(|e| e.to_string())?;
            let mut lines = vec![format!("Project: {} ({})", model.name, model.id)];
            for e in &model.elements {
                lines.push(format!("{} | {} | {}", e.id, e.kind, e.name));
            }
            for r in &model.relations {
                lines.push(format!("{} --[{}]--> {}", r.from, r.kind, r.to));
            }
            Ok(lines.join("\n"))
        }
        ModelingOperation::Analyze {
            source,
            from,
            to,
            relation,
        } => {
            let model = load(&source)?;
            Graph::new(&model).validate().map_err(|e| e.to_string())?;
            crate::graph_report::report(&model, from.as_deref(), to.as_deref(), relation.as_deref())
        }
        ModelingOperation::Plan {
            source,
            scope,
            skeleton,
        } => {
            let model = load(&source)?;
            let plan = bellatrix_mde_rust_generator::plan_with_options(
                &model,
                &scope,
                bellatrix_mde_rust_generator::GenerationOptions { skeleton },
            )
            .map_err(|e| e.to_string())?;
            let mut lines = plan
                .artifacts()
                .map(|a| format!("{} [{}]", a.path.display(), a.origin))
                .collect::<Vec<_>>();
            for d in &plan.diagnostics {
                lines.push(format!("WARN {}: {}", d.code, d.message));
            }
            Ok(lines.join("\n"))
        }
        ModelingOperation::Generate {
            source,
            scope,
            destination,
            replace,
            skeleton,
        } => {
            let model = load(&source)?;
            let plan = bellatrix_mde_rust_generator::plan_with_options(
                &model,
                &scope,
                bellatrix_mde_rust_generator::GenerationOptions { skeleton },
            )
            .map_err(|e| e.to_string())?;
            if replace {
                publish_replace(&plan, &destination)
            } else {
                publish_new(&plan, &destination)
            }
            .map_err(|e| e.to_string())?;
            Ok(format!(
                "Generated {} artifacts in {} ({} warnings)",
                plan.count(),
                destination.display(),
                plan.diagnostics.len()
            ))
        }
        ModelingOperation::ExportHtml {
            source,
            destination,
            replace,
        } => {
            let model = load(&source)?;
            let plan = bellatrix_mde_html_exporter::plan(&model).map_err(|e| e.to_string())?;
            if replace {
                publish_replace(&plan, &destination)
            } else {
                publish_new(&plan, &destination)
            }
            .map_err(|e| e.to_string())?;
            Ok(format!(
                "Exported {} HTML/SVG artifacts to {}",
                plan.count(),
                destination.display()
            ))
        }
        ModelingOperation::Convert {
            source,
            destination,
        } => {
            let model = load(&source)?;
            let bytes = encode(&model, format(&destination)).map_err(|e| e.to_string())?;
            if destination.exists() {
                return Err("output exists; refusing to overwrite".into());
            }
            fs::write(&destination, bytes).map_err(|e| e.to_string())?;
            Ok(format!("Wrote {}", destination.display()))
        }
    }
}
