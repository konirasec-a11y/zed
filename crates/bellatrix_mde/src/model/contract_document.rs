use crate::{ModelingSection, Project};
use bellatrix_mde_contracts::model::{
    ModelingSection as StoredSection, Project as StoredProject, SubjectPresentation,
};
use std::collections::BTreeMap;

pub(crate) fn project_section(stored: StoredSection) -> Result<ModelingSection, String> {
    for id in stored.presentation.keys() {
        if stored.registry.get_entry(id).is_none() {
            return Err(format!(
                "presentation subject {id} is absent from the registry"
            ));
        }
    }
    let (mut elements, mut relations) =
        crate::contract_projection::project_registry(&stored.registry)?;
    for relation in &mut relations {
        if let Some(metadata) = stored.presentation.get(&relation.id) {
            relation.properties.insert(
                "$presentation".into(),
                serde_json::to_value(metadata).map_err(|error| error.to_string())?,
            );
        }
    }
    for element in &mut elements {
        if let Some(presentation) = stored.presentation.get(&element.id) {
            element.label = presentation.label.clone();
            element.description = presentation.description.clone();
            element.annotations = presentation.annotations.clone();
            element.implementations = presentation.implementations.clone();
            element.content = presentation.content.clone();
        }
    }
    Ok(ModelingSection {
        root_model: stored.root_model,
        authoring_knowledge: stored.authoring_knowledge,
        mdj_provenance: stored.mdj_provenance,
        hierarchy: stored.hierarchy,
        composition: stored.composition,
        elements,
        relations,
        diagrams: stored.diagrams,
    })
}

pub(crate) fn store_section(snapshot: &ModelingSection) -> Result<StoredSection, String> {
    let mut relations = snapshot.relations.clone();
    let mut presentation = BTreeMap::new();
    // Metadata travels with the disposable relation, then leaves it before the
    // semantic converter. No generated UML contract has this reserved field.
    for relation in &mut relations {
        if let Some(value) = relation.properties.remove("$presentation") {
            if relation.kind == "owns" {
                return Err("derived ownership cannot own presentation metadata".into());
            }
            let metadata = serde_json::from_value::<SubjectPresentation>(value)
                .map_err(|error| error.to_string())?;
            presentation.insert(relation.id.clone(), metadata);
        }
    }
    let registry = crate::contract_edits::registry_from_projection(&snapshot.elements, &relations)?;
    for element in &snapshot.elements {
        let metadata = SubjectPresentation {
            label: element.label.clone(),
            description: element.description.clone(),
            annotations: element.annotations.clone(),
            implementations: element.implementations.clone(),
            content: element.content.clone(),
        };
        if metadata != SubjectPresentation::default() {
            presentation.insert(element.id.clone(), metadata);
        }
    }
    let mut composition = snapshot.composition.clone();
    // An editor edge is a disposable projection. Persist the actual container
    // identity rather than an edge ID that does not exist in the typed registry.
    for (subject, placement) in &mut composition {
        if matches!(
            placement.parent,
            crate::CompositionParent::SemanticOwner { .. }
        ) {
            let parent = placement
                .parent_id(subject, &snapshot.relations)
                .ok_or_else(|| format!("composition owner not found for {subject}"))?
                .to_owned();
            placement.parent = crate::CompositionParent::Object { id: parent };
        }
    }
    Ok(StoredSection {
        registry,
        root_model: snapshot.root_model.clone(),
        authoring_knowledge: snapshot.authoring_knowledge.clone(),
        mdj_provenance: snapshot.mdj_provenance.clone(),
        hierarchy: snapshot.hierarchy.clone(),
        composition,
        presentation,
        diagrams: snapshot.diagrams.clone(),
    })
}

pub(crate) fn project_document(stored: StoredProject) -> Result<Project, String> {
    if stored.schema_version != 2 {
        return Err(format!(
            "unsupported model schema version: {}",
            stored.schema_version
        ));
    }
    let section = project_section(stored.model)?;
    let mut project = section.to_model(&stored.id, &stored.name);
    project.schema_version = 2;
    Ok(project)
}

pub(crate) fn store_document(snapshot: &Project) -> Result<StoredProject, String> {
    Ok(store_section(&ModelingSection::from_model(snapshot))?
        .to_model(&snapshot.id, &snapshot.name))
}
