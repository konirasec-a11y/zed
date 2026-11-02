//! Create model elements and their diagram representations through Cordis.
//! UI gestures only propose commands; ProjectService owns the revision.
use crate::project_integration::{from_document, namespace_for};
use bellatrix_mde_model::{Element, Geometry, Style, View};
use bellatrix_mde_modeling::Graph;
use bellatrix_project::{ProjectId, ProjectReplace, ProjectReplaceRequested, ProjectService};
use cordis::event::QueryOutcome;
use cordis::{Context, Event, Routing};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelingElementKind {
    Class,
    Interface,
}

impl ModelingElementKind {
    fn model_kind(self) -> &'static str {
        match self {
            Self::Class => "rust.struct",
            Self::Interface => "rust.trait",
        }
    }

    fn default_name(self) -> &'static str {
        match self {
            Self::Class => "NewClass",
            Self::Interface => "NewInterface",
        }
    }
}

#[derive(Clone, Debug)]
pub struct ModelingCreateElementRequested {
    pub project_id: ProjectId,
    pub diagram_id: String,
    pub kind: ModelingElementKind,
    pub name: Option<String>,
    pub x: f64,
    pub y: f64,
    pub expected_revision: u64,
}

pub struct ModelingCreateElement;

impl Event for ModelingCreateElement {
    const NAME: &'static str = "bellatrix.mde.create-element.v1";
    type Args = ModelingCreateElementRequested;
    type Output = Result<u64, String>;
}

pub(crate) async fn create_element(
    ctx: &Context,
    projects: &ProjectService,
    request: ModelingCreateElementRequested,
) -> Result<u64, String> {
    if !request.x.is_finite() || !request.y.is_finite() {
        return Err("element position must be finite".into());
    }
    let name = request.name.as_deref().unwrap_or(request.kind.default_name());
    if name.trim().is_empty() {
        return Err("element name must not be empty".into());
    }
    let document = projects
        .snapshot(&request.project_id)
        .map_err(|error| error.to_string())?;
    if document.revision() != request.expected_revision {
        return Err("stale element creation; refresh project snapshot".into());
    }
    let mut section = from_document(&document)?;
    if !section.diagrams.iter().any(|d| d.id == request.diagram_id) {
        return Err("diagram not found".into());
    }
    let namespace = namespace_for(&document);
    let mut sequence = request.expected_revision.checked_add(1)
        .ok_or("identifier sequence overflow")?;
    let (element_id, view_id) = loop {
        let element_id = format!("mde-element-{sequence}");
        let view_id = format!("mde-view-{sequence}");
        let collides = section.elements.iter().any(|e| e.id == element_id || e.id == view_id)
            || section.relations.iter().any(|r| r.id == element_id || r.id == view_id)
            || section.diagrams.iter().any(|d| {
                d.id == element_id
                    || d.id == view_id
                    || d.views.iter().any(|v| v.id == element_id || v.id == view_id)
            });
        if !collides {
            break (element_id, view_id);
        }
        sequence = sequence.checked_add(1).ok_or("identifier sequence overflow")?;
    };

    let diagram = section.diagrams.iter_mut()
        .find(|d| d.id == request.diagram_id)
        .ok_or("diagram not found")?;
    let z_index = diagram.views.iter().map(|v| v.z_index).max().unwrap_or(0)
        .checked_add(1).ok_or("diagram z-index overflow")?;
    diagram.views.push(View {
        id: view_id,
        subject: Some(element_id.clone()),
        label: None,
        geometry: Geometry::Rect { x: request.x, y: request.y, width: 180.0, height: 112.0 },
        style: Style::default(),
        z_index,
    });
    section.elements.push(Element {
        id: element_id,
        kind: request.kind.model_kind().into(),
        name: name.into(),
        label: None,
        description: None,
        properties: Default::default(),
        annotations: Vec::new(),
        implementations: Vec::new(),
        content: None,
    });

    Graph::new(&section.to_model(document.id().as_str(), document.name()))
        .validate().map_err(|error| error.to_string())?;
    let state = serde_json::to_value(section).map_err(|error| error.to_string())?;
    match ctx.query::<ProjectReplace>(
        Routing::Unscoped,
        ProjectReplaceRequested {
            id: request.project_id,
            namespace: namespace.into(),
            expected_revision: request.expected_revision,
            state,
        },
    ).await.map_err(|error| error.to_string())? {
        QueryOutcome::Answer(Ok(updated)) => Ok(updated.revision()),
        QueryOutcome::Answer(Err(error)) => Err(error.to_string()),
        QueryOutcome::Miss => Err("ProjectService is unavailable".into()),
    }
}
