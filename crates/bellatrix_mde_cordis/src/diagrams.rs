//! Diagram lifecycle requests share the same revision-checked mutation boundary as element edits.
use bellatrix_mde::{
    StructuralDiagnostic, StructuralDiagramKind, commands::Command, validate_structural_diagram,
};
use bellatrix_project::{ProjectId, ProjectService};
use cordis::event::responder;
use cordis::{Context, Event};
use std::{convert::Infallible, io, sync::Arc};

#[derive(Clone, Debug)]
pub struct ModelingCreateDiagramRequested {
    pub project_id: ProjectId,
    pub kind: StructuralDiagramKind,
    pub name: String,
    pub model_elements: Vec<String>,
    pub expected_revision: u64,
}
pub struct ModelingCreateDiagram;
impl Event for ModelingCreateDiagram {
    const NAME: &'static str = "bellatrix.mde.create-diagram.v1";
    type Args = ModelingCreateDiagramRequested;
    type Output = Result<u64, String>;
}

#[derive(Clone, Debug)]
pub struct ModelingRenameDiagramRequested {
    pub project_id: ProjectId,
    pub diagram_id: String,
    pub name: String,
    pub expected_revision: u64,
}
pub struct ModelingRenameDiagram;
impl Event for ModelingRenameDiagram {
    const NAME: &'static str = "bellatrix.mde.rename-diagram.v1";
    type Args = ModelingRenameDiagramRequested;
    type Output = Result<u64, String>;
}

#[derive(Clone, Debug)]
pub struct ModelingDeleteDiagramRequested {
    pub project_id: ProjectId,
    pub diagram_id: String,
    pub expected_revision: u64,
}
pub struct ModelingDeleteDiagram;
impl Event for ModelingDeleteDiagram {
    const NAME: &'static str = "bellatrix.mde.delete-diagram.v1";
    type Args = ModelingDeleteDiagramRequested;
    type Output = Result<u64, String>;
}

/// Validation uses the required stored UMLDI type; callers cannot override it.
#[derive(Clone, Debug)]
pub struct ModelingValidateDiagramRequested {
    pub project_id: ProjectId,
    pub diagram_id: String,
}
pub struct ModelingValidateDiagram;
impl Event for ModelingValidateDiagram {
    const NAME: &'static str = "bellatrix.mde.validate-diagram.v1";
    type Args = ModelingValidateDiagramRequested;
    type Output = Result<Vec<StructuralDiagnostic>, String>;
}

fn validate(
    projects: &ProjectService,
    request: ModelingValidateDiagramRequested,
) -> Result<Vec<StructuralDiagnostic>, String> {
    let document = projects
        .snapshot(&request.project_id)
        .map_err(|error| error.to_string())?;
    let model = crate::project_integration::from_document(&document)?
        .to_model(document.id().as_str(), document.name());
    let diagram = model
        .diagrams
        .iter()
        .find(|diagram| diagram.id == request.diagram_id)
        .ok_or("diagram not found")?;
    let stored = diagram.uml_di_type.as_str();
    let kind = StructuralDiagramKind::from_diagram(diagram)
        .ok_or_else(|| format!("unsupported UMLDI diagram type: {stored}"))?;
    Ok(validate_structural_diagram(
        &model,
        &request.diagram_id,
        kind,
    ))
}

pub(crate) fn register(context: &Context, projects: Arc<ProjectService>) -> Result<(), io::Error> {
    let create_projects = Arc::clone(&projects);
    let _create = context
        .on::<ModelingCreateDiagram, _>(responder(
            move |context: Context, request: ModelingCreateDiagramRequested| {
                let projects = Arc::clone(&create_projects);
                async move {
                    let result = crate::mutation::execute(
                        &context,
                        &projects,
                        request.project_id,
                        request.expected_revision,
                        Command::CreateDiagram {
                            kind: request.kind,
                            name: request.name,
                            model_elements: request.model_elements,
                        },
                    )
                    .await;
                    Ok::<_, Infallible>(Some(result))
                }
            },
        ))
        .map_err(|error| io::Error::other(error.to_string()))?;
    let rename_projects = Arc::clone(&projects);
    let _rename = context
        .on::<ModelingRenameDiagram, _>(responder(
            move |context: Context, request: ModelingRenameDiagramRequested| {
                let projects = Arc::clone(&rename_projects);
                async move {
                    let result = crate::mutation::execute(
                        &context,
                        &projects,
                        request.project_id,
                        request.expected_revision,
                        Command::RenameDiagram {
                            diagram_id: request.diagram_id,
                            name: request.name,
                        },
                    )
                    .await;
                    Ok::<_, Infallible>(Some(result))
                }
            },
        ))
        .map_err(|error| io::Error::other(error.to_string()))?;
    let delete_projects = Arc::clone(&projects);
    let _delete = context
        .on::<ModelingDeleteDiagram, _>(responder(
            move |context: Context, request: ModelingDeleteDiagramRequested| {
                let projects = Arc::clone(&delete_projects);
                async move {
                    let result = crate::mutation::execute(
                        &context,
                        &projects,
                        request.project_id,
                        request.expected_revision,
                        Command::DeleteDiagram {
                            diagram_id: request.diagram_id,
                        },
                    )
                    .await;
                    Ok::<_, Infallible>(Some(result))
                }
            },
        ))
        .map_err(|error| io::Error::other(error.to_string()))?;
    let _validate = context
        .on::<ModelingValidateDiagram, _>(responder(
            move |_context: Context, request: ModelingValidateDiagramRequested| {
                let projects = Arc::clone(&projects);
                async move { Ok::<_, Infallible>(Some(validate(&projects, request))) }
            },
        ))
        .map_err(|error| io::Error::other(error.to_string()))?;
    Ok(())
}
