use bellatrix_mde_cordis::presentation_contract::UmlNotation;
use bellatrix_mde_cordis::projection::{Contract, Diagram, ModelingProjection as ModelingSection};
use bellatrix_mde_cordis::{Geometry, PathSegment};

pub(crate) struct ReconciledSelection {
    pub active_diagram: Option<String>,
    pub selected_view: Option<String>,
    pub selected_element: Option<String>,
    pub preserve_editor: bool,
}

pub(crate) fn reconcile_selection(
    section: &ModelingSection,
    active_diagram: Option<&str>,
    selected_view: Option<&str>,
    selected_element: Option<&str>,
    editing_subject: Option<&str>,
) -> ReconciledSelection {
    let diagram = section
        .diagrams
        .iter()
        .find(|diagram| Some(diagram.id.as_str()) == active_diagram)
        .or_else(|| section.diagrams.first());
    let diagram_id = diagram.map(|diagram| diagram.id.as_str());
    let same_diagram = diagram_id == active_diagram;
    let subject_exists = |id: &str| {
        section.elements.iter().any(|element| element.id == id)
            || section.relations.iter().any(|relation| relation.id == id)
    };
    let view = diagram.filter(|_| same_diagram).and_then(|diagram| {
        diagram
            .all_views()
            .find(|view| Some(view.id.as_str()) == selected_view)
    });
    let element = if !same_diagram {
        None
    } else if selected_view.is_some() {
        view.and_then(|view| view.subject.as_deref())
            .filter(|id| subject_exists(id))
    } else {
        selected_element.filter(|id| subject_exists(id))
    };
    let preserve_editor = same_diagram
        && editing_subject.is_some_and(|id| {
            (Some(id) == element
                && diagram.is_some_and(|diagram| has_inline_representation(section, diagram, id)))
                || view.is_some_and(|view| {
                    view.id == id
                        && view.subject.is_none()
                        && matches!(view.geometry, Geometry::Rect { .. } | Geometry::Path { .. })
                })
        });
    ReconciledSelection {
        active_diagram: diagram_id.map(str::to_owned),
        selected_view: view.map(|view| view.id.clone()),
        selected_element: element.map(str::to_owned),
        preserve_editor,
    }
}

pub(crate) fn inline_relation_name(section: &ModelingSection, id: &str) -> Option<String> {
    section
        .relations
        .iter()
        .find(|relation| relation.id == id)?;
    section.contract(id)?;
    if !section.contracts.conforms_to(id, "NamedElement") {
        return None;
    }
    let property = section
        .inspections
        .get(id)?
        .properties
        .iter()
        .find(|property| property.path == "properties.name" && property.editable)?;
    match &property.value {
        serde_json::Value::String(name) => Some(name.clone()),
        serde_json::Value::Null => Some(String::new()),
        _ => None,
    }
}

pub(crate) fn supports_inline_relation_view(
    view: &bellatrix_mde_cordis::projection::View,
    target: &str,
) -> bool {
    if view.subject.as_deref() != Some(target) || view.relationship.is_none() {
        return false;
    }
    let Geometry::Path { start, segments } =
        view.routed_geometry.as_ref().unwrap_or(&view.geometry)
    else {
        return false;
    };
    let finite = |point: &bellatrix_mde_cordis::Point| {
        point.x.is_finite()
            && point.y.is_finite()
            && (point.x as f32).is_finite()
            && (point.y as f32).is_finite()
    };
    finite(start)
        && !segments.is_empty()
        && segments.iter().all(|segment| match segment {
            PathSegment::Line { to } => finite(to),
            PathSegment::Cubic {
                control1,
                control2,
                to,
            } => finite(control1) && finite(control2) && finite(to),
        })
}

fn has_inline_representation(section: &ModelingSection, diagram: &Diagram, target: &str) -> bool {
    if section
        .relations
        .iter()
        .any(|relation| relation.id == target)
    {
        if inline_relation_name(section, target).is_none() {
            return false;
        }
        return super::canvas_presentation::ordered_views(section, &diagram.views)
            .into_iter()
            .any(|view| supports_inline_relation_view(view, target));
    }
    diagram.all_views().any(|view| {
        let Geometry::Rect {
            x,
            y,
            width,
            height,
        } = &view.geometry
        else {
            return false;
        };
        if ![x, y, width, height].iter().all(|value| value.is_finite()) {
            return false;
        }
        let Some(owner) = section
            .elements
            .iter()
            .find(|element| Some(element.id.as_str()) == view.subject.as_deref())
        else {
            return false;
        };
        let notation = view.figure.notation;
        if owner.id == target {
            return notation != UmlNotation::Unsupported;
        }
        if matches!(
            notation,
            UmlNotation::ClassifierRectangle | UmlNotation::PackageTab
        ) && owner
            .template_parameters
            .iter()
            .any(|parameter| parameter == target)
        {
            return true;
        }
        if notation != UmlNotation::ClassifierRectangle
            || !owner.members.iter().any(|member| member == target)
        {
            return false;
        }
        section.elements.iter().any(|element| {
            element.id == target
                && match section.contract(&element.id) {
                    Some(Contract::Property(_)) => {
                        section.contracts.owns_attribute(&owner.id, &element.id)
                            && view.style.uml.suppress_attributes != Some(true)
                    }
                    Some(Contract::Operation(_)) => {
                        view.style.uml.suppress_operations != Some(true)
                    }
                    Some(Contract::Reception(_)) => {
                        view.style.uml.suppress_receptions != Some(true)
                    }
                    _ => false,
                }
        })
    })
}

pub(crate) fn supports_view_editing(diagram: &Diagram, view_id: &str) -> bool {
    diagram.all_views().any(|view| {
        view.id == view_id && matches!(view.geometry, Geometry::Rect { .. } | Geometry::Path { .. })
    })
}

pub(crate) fn supports_view_style_editing(diagram: &Diagram, view_id: &str) -> bool {
    // Appearance changes do not reposition a nested View or resolve its layout.
    diagram.all_views().any(|view| view.id == view_id)
}

pub(crate) fn supports_view_layout_editing(diagram: &Diagram, view_id: &str) -> bool {
    supports_view_editing(diagram, view_id)
}
