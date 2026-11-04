use bellatrix_mde::{Diagram, Geometry, ModelingSection, UmlNotation};

pub(super) struct ReconciledSelection {
    pub active_diagram: Option<String>,
    pub selected_view: Option<String>,
    pub selected_element: Option<String>,
    pub preserve_editor: bool,
}

pub(super) fn reconcile_selection(
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
    let view = diagram
        .filter(|_| same_diagram)
        .and_then(|diagram| {
            diagram
                .views
                .iter()
                .find(|view| Some(view.id.as_str()) == selected_view)
        })
        .filter(|view| match view.subject.as_deref() {
            Some(subject) => subject_exists(subject),
            None => view.draft.is_some(),
        });
    let element = if !same_diagram {
        None
    } else if selected_view.is_some() {
        view.and_then(|view| view.subject.as_deref())
    } else {
        selected_element.filter(|id| subject_exists(id))
    };
    let preserve_editor = same_diagram
        && editing_subject.is_some_and(|id| {
            (Some(id) == element
                && diagram.is_some_and(|diagram| has_inline_representation(section, diagram, id)))
                || view.is_some_and(|view| {
                    view.id == id && view.subject.is_none() && view.draft.is_some()
                })
        });
    ReconciledSelection {
        active_diagram: diagram_id.map(str::to_owned),
        selected_view: view.map(|view| view.id.clone()),
        selected_element: element.map(str::to_owned),
        preserve_editor,
    }
}

fn has_inline_representation(section: &ModelingSection, diagram: &Diagram, target: &str) -> bool {
    diagram.views.iter().any(|view| {
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
        let notation = bellatrix_mde::uml_notation(&owner.kind);
        if owner.id == target {
            return notation != UmlNotation::Unsupported;
        }
        if matches!(
            notation,
            UmlNotation::ClassifierRectangle | UmlNotation::PackageTab
        ) && bellatrix_mde::template_parameters(section, &owner.id)
            .iter()
            .any(|parameter| parameter.id == target)
        {
            return true;
        }
        if notation != UmlNotation::ClassifierRectangle
            || !section.relations.iter().any(|relation| {
                relation.kind == "owns" && relation.from == owner.id && relation.to == target
            })
        {
            return false;
        }
        section.elements.iter().any(|element| {
            element.id == target
                && match element.kind.as_str() {
                    "rust.field" | "uml.attribute" => view.style.uml.suppress_attributes != Some(true),
                    "rust.function" | "uml.operation" => {
                        view.style.uml.suppress_operations != Some(true)
                    }
                    "uml.reception" => view.style.uml.suppress_receptions != Some(true),
                    _ => false,
                }
        })
    })
}
