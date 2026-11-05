use crate::{ModelingSection, Project};
use bellatrix_mde_contracts::model::{
    ModelingSection as StoredSection, Project as StoredProject, SubjectPresentation,
};
use std::collections::BTreeMap;

pub(crate) fn project_section(
    stored: StoredSection,
    project_id: &str,
) -> Result<ModelingSection, String> {
    let opaque =
        opaque_source_objects(stored.mdj_provenance.as_ref(), project_id, &stored.diagrams);
    let opaque_ids: std::collections::BTreeSet<_> =
        opaque.iter().map(|(id, _)| id.as_str()).collect();
    for id in stored.presentation.keys() {
        if stored.registry.get_entry(id).is_none() && !opaque_ids.contains(id.as_str()) {
            return Err(format!(
                "presentation subject {id} is absent from the registry"
            ));
        }
    }
    let (mut elements, mut relations) =
        crate::contract_projection::project_registry(&stored.registry)?;
    for (id, source) in opaque {
        if stored.registry.get_entry(id).is_some() {
            return Err(format!(
                "source object {id} is both opaque and a UML contract"
            ));
        }
        elements.push(project_opaque_object(id, source));
    }
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

pub(crate) fn store_section(
    snapshot: &ModelingSection,
    project_id: &str,
) -> Result<StoredSection, String> {
    let opaque = opaque_source_objects(
        snapshot.mdj_provenance.as_ref(),
        project_id,
        &snapshot.diagrams,
    );
    for (id, source) in &opaque {
        let element = snapshot
            .elements
            .iter()
            .find(|element| element.id == **id)
            .ok_or_else(|| {
                format!("opaque source object {id} cannot be deleted through semantic commands")
            })?;
        let expected = project_opaque_object(id, source);
        if element.kind != expected.kind
            || element.name != expected.name
            || element.properties != expected.properties
        {
            return Err(format!(
                "opaque source object {id} is preserved as interchange data and cannot be rewritten as UML"
            ));
        }
    }
    let opaque_ids: std::collections::BTreeSet<_> =
        opaque.iter().map(|(id, _)| id.as_str()).collect();
    for element in &snapshot.elements {
        if element.kind.starts_with("mdj.") && !opaque_ids.contains(element.id.as_str()) {
            return Err(format!(
                "opaque object {} has no retained source data",
                element.id
            ));
        }
    }
    let semantic_elements: Vec<_> = snapshot
        .elements
        .iter()
        .filter(|element| !opaque_ids.contains(element.id.as_str()))
        .cloned()
        .collect();
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
    let registry = crate::contract_edits::registry_from_projection(&semantic_elements, &relations)?;
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
    let section = project_section(stored.model, &stored.id)?;
    let mut project = section.to_model(&stored.id, &stored.name);
    project.schema_version = 2;
    Ok(project)
}

pub(crate) fn store_document(snapshot: &Project) -> Result<StoredProject, String> {
    Ok(
        store_section(&ModelingSection::from_model(snapshot), &snapshot.id)?
            .to_model(&snapshot.id, &snapshot.name),
    )
}

fn opaque_source_objects<'a>(
    provenance: Option<&'a crate::MdjProvenance>,
    project_id: &str,
    diagrams: &[crate::Diagram],
) -> Vec<(&'a String, &'a crate::MdjObjectResidual)> {
    provenance
        .into_iter()
        .flat_map(|provenance| &provenance.object_residuals)
        .filter(|(id, source)| {
            id.as_str() != project_id
                && source.mapped_fields.is_empty()
                && !diagrams.iter().any(|diagram| {
                    &diagram.id == *id || diagram.all_views().any(|view| &view.id == *id)
                })
        })
        .collect()
}

fn project_opaque_object(id: &str, source: &crate::MdjObjectResidual) -> crate::Element {
    let mut properties = source.residual.clone();
    properties.remove("name");
    crate::Element {
        id: id.into(),
        kind: format!("mdj.{}", source.type_name),
        name: source
            .residual
            .get("name")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .into(),
        label: None,
        description: None,
        properties,
        annotations: Vec::new(),
        implementations: Vec::new(),
        content: None,
    }
}
