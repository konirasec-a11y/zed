use crate::{
    Element, Geometry, ModelingSection, Relation, UmlType, classifier_contents,
    reroute_relationships,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelingMemberKind {
    Attribute,
    Operation,
    Parameter,
    Reception,
}

pub(super) fn grow_classifier_views(
    section: &mut ModelingSection,
    owner_id: &str,
) -> Result<(), String> {
    let Some(kind) = section
        .elements
        .iter()
        .find(|element| element.id == owner_id)
        .and_then(|element| UmlType::from_kind(&element.kind))
    else {
        return Ok(());
    };
    let contents = classifier_contents(section, owner_id);
    let minimum = if kind == UmlType::Interface {
        contents.minimum_interface_height()
    } else {
        44.0 + contents.compartments_height()
    };
    // Resizing can also reroute other Views in the same diagram. Preflight
    // before mutating any geometry until that projection understands children.
    for diagram in &section.diagrams {
        let needs_growth = diagram.views.iter().any(|view| {
            view.subject.as_deref() == Some(owner_id)
                && matches!(view.geometry, Geometry::Rect { height, .. } if height < minimum)
        });
        if needs_growth
            && diagram.all_views().any(|view| {
                !view.children.is_empty() || matches!(view.geometry, Geometry::Layout { .. })
            })
        {
            return Err(format!(
                "adding this member requires resizing classifier {} in diagram {}, which needs hierarchy-aware layout; the command was not applied",
                owner_id, diagram.id
            ));
        }
    }
    let mut diagrams = Vec::new();
    for diagram in &mut section.diagrams {
        let mut changed = false;
        for view in diagram
            .views
            .iter_mut()
            .filter(|view| view.subject.as_deref() == Some(owner_id))
        {
            if let Geometry::Rect { height, .. } = &mut view.geometry
                && *height < minimum
            {
                *height = minimum;
                changed = true;
            }
        }
        if changed {
            diagrams.push(diagram.id.clone());
        }
    }
    for diagram_id in diagrams {
        reroute_relationships(section, &diagram_id);
    }
    Ok(())
}

pub(super) fn append_member(
    section: &mut ModelingSection,
    owner_id: &str,
    kind: ModelingMemberKind,
    revision: u64,
    project_id: &str,
) -> Result<String, String> {
    let owner = section
        .elements
        .iter()
        .find(|element| element.id == owner_id)
        .ok_or("member owner not found")?;
    let rust_classifier = matches!(owner.kind.as_str(), "rust.struct" | "rust.trait");
    let (model_kind, prefix, base) = match kind {
        ModelingMemberKind::Attribute => (
            if rust_classifier {
                "rust.field"
            } else {
                "uml.attribute"
            },
            "attribute",
            "Attribute",
        ),
        ModelingMemberKind::Operation => (
            if rust_classifier {
                "rust.function"
            } else {
                "uml.operation"
            },
            "operation",
            "Operation",
        ),
        ModelingMemberKind::Reception => ("uml.reception", "reception", "Reception"),
        ModelingMemberKind::Parameter => ("uml.parameter", "parameter", "Parameter"),
    };
    let mut name_sequence = 1_u64;
    let name = loop {
        let candidate = format!("{base}{name_sequence}");
        let duplicate = section.relations.iter().any(|relation| {
            relation.kind == "owns"
                && relation.from == owner_id
                && section
                    .elements
                    .iter()
                    .any(|element| element.id == relation.to && element.name == candidate)
        });
        if !duplicate {
            break candidate;
        }
        name_sequence = name_sequence
            .checked_add(1)
            .ok_or("name sequence overflow")?;
    };
    let member_prefix = format!("mde-{prefix}");
    let relation_prefix = format!("mde-owns-{prefix}");
    let [member_id, relation_id] = super::fresh_ids(
        section,
        [member_prefix.as_str(), relation_prefix.as_str()],
        revision,
        project_id,
    )?;
    let mut properties = std::collections::BTreeMap::new();
    properties.insert("visibility".into(), serde_json::json!("public"));
    if model_kind == "rust.field" {
        properties.insert(
            "declaration".into(),
            serde_json::json!(format!("pub {prefix}: String")),
        );
    } else if model_kind == "rust.function" {
        properties.insert(
            "signature".into(),
            serde_json::json!(format!("pub fn {prefix}(&self)")),
        );
    }
    section.elements.push(Element {
        id: member_id.clone(),
        kind: model_kind.into(),
        name,
        label: None,
        description: None,
        properties,
        annotations: Vec::new(),
        implementations: Vec::new(),
        content: None,
    });
    section.relations.push(Relation {
        id: relation_id,
        kind: "owns".into(),
        from: owner_id.into(),
        to: member_id.clone(),
        properties: Default::default(),
    });
    let classifier_id = section
        .relations
        .iter()
        .find(|relation| relation.kind == "owns" && relation.to == owner_id)
        .map(|relation| relation.from.clone());
    grow_classifier_views(section, owner_id)?;
    if kind == ModelingMemberKind::Parameter
        && let Some(classifier_id) = classifier_id
    {
        grow_classifier_views(section, &classifier_id)?;
    }
    Ok(member_id)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_member_command_accepts_classifiers_and_rejects_files() {
        for owner_kind in ["rust.struct", "rust.trait", "uml.class", "uml.interface"] {
            let model = serde_json::from_str(include_str!("../../../fixtures/todo.json")).unwrap();
            let mut section = ModelingSection::from_model(&model);
            section
                .elements
                .iter_mut()
                .find(|element| element.id == "type-task")
                .unwrap()
                .kind = owner_kind.into();
            for kind in [
                ModelingMemberKind::Attribute,
                ModelingMemberKind::Operation,
                ModelingMemberKind::Reception,
            ] {
                let id = append_member(&mut section, "type-task", kind, 0, "test-project").unwrap();
                assert!(
                    section
                        .relations
                        .iter()
                        .any(|relation| relation.kind == "owns"
                            && relation.from == "type-task"
                            && relation.to == id)
                );
            }
            let before = section.clone();
            assert!(
                append_member(
                    &mut section,
                    "file-main",
                    ModelingMemberKind::Attribute,
                    0,
                    "test-project"
                )
                .is_err()
            );
            assert_eq!(section, before);
            crate::Graph::new(&section.to_model("project-todo", "Task List / Todo Project"))
                .validate()
                .unwrap();
        }
    }
}
