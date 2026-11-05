use crate::{
    EndpointMarkerTarget, Geometry, ModelingSection, RelationshipRouting, View, ViewEndpoints,
};
use serde_json::{Value, json};
use std::collections::BTreeSet;

// Called only inside inspector_edit's staged transaction. Source edits change
// UML composite ownership, not merely the two disposable graph endpoint strings.
pub(crate) fn set_endpoint(
    section: &mut ModelingSection,
    id: &str,
    path: &str,
    value: &Value,
) -> Result<bool, String> {
    let Some(relation) = section.relations.iter().find(|relation| relation.id == id) else {
        return Ok(false);
    };
    let (source, required, composite, dependency) = match (relation.kind.as_str(), path) {
        ("uml.generalization", "properties.general") => {
            (false, "Classifier", "generalization", false)
        }
        ("uml.generalization", "properties.specific") => {
            (true, "Classifier", "generalization", false)
        }
        ("uml.interface_realization", "properties.contract") => {
            (false, "Interface", "interfaceRealization", true)
        }
        ("uml.interface_realization", "properties.implementingClassifier") => {
            (true, "BehavioredClassifier", "interfaceRealization", true)
        }
        _ => return Ok(false),
    };
    let key = path
        .strip_prefix("properties.")
        .ok_or("missing endpoint property")?;
    let target = crate::uml::references::reference_id(value)?
        .ok_or_else(|| format!("{key} requires one non-null {required} reference"))?
        .to_owned();
    let model = crate::uml::root::Model::from_section(section);
    let target_element = model
        .element(&target)
        .ok_or_else(|| format!("{key}: unknown target {target}"))?;
    if !target_element.conforms_to(required) {
        return Err(format!("{key}: target {target} must be a UML {required}"));
    }
    let old = if source { &relation.from } else { &relation.to }.clone();
    let declared = relation
        .properties
        .get(key)
        .ok_or_else(|| format!("{id}.{key}: missing canonical endpoint"))?;
    if crate::uml::references::reference_id(declared)? != Some(old.as_str()) {
        return Err(format!(
            "{id}.{key}: endpoint differs from its graph projection; repair the inconsistent relationship first"
        ));
    }
    if old == target {
        return Ok(true);
    }
    for diagram in &section.diagrams {
        if let Some(view) = diagram.all_views().find(|view| {
            view.subject.as_deref() == Some(id)
                && matches!(view.geometry, Geometry::Path { .. })
                && view.style.routing == Some(RelationshipRouting::Manual)
        }) {
            return Err(format!(
                "View {} uses Manual routing: semantic endpoint retargeting requires manual-path reanchoring, which is not yet supported; the relationship and authored path were preserved",
                view.id
            ));
        }
    }
    let next_source = if source {
        target.as_str()
    } else {
        relation.from.as_str()
    };
    let next_target = if source {
        relation.to.as_str()
    } else {
        target.as_str()
    };
    if !dependency {
        let mut pending = vec![next_target];
        let mut visited = BTreeSet::new();
        while let Some(current) = pending.pop() {
            if current == next_source {
                return Err("Generalization retargeting would create an inheritance cycle".into());
            }
            if visited.insert(current) {
                pending.extend(
                    section
                        .relations
                        .iter()
                        .filter(|edge| {
                            edge.kind == "uml.generalization"
                                && edge.id != id
                                && edge.from == current
                        })
                        .map(|edge| edge.to.as_str()),
                );
            }
        }
    }
    if source {
        let ownerships: Vec<_> = section
            .relations
            .iter()
            .filter(|edge| edge.kind == "owns" && edge.to == id)
            .collect();
        let [ownership] = ownerships.as_slice() else {
            return Err(format!(
                "{id}: source retargeting requires exactly one canonical ownership edge"
            ));
        };
        if ownership.from != old
            || ownership.properties.get("role").and_then(Value::as_str) != Some(composite)
        {
            return Err(format!(
                "{id}: specialized source and ownership role disagree"
            ));
        }
        let ownership_id = ownership.id.clone();
        let old_owner = owner_properties_mut(section, &old)?;
        let mut old_members = reference_collection(old_owner.get(composite), composite)?;
        if !old_members.iter().any(|member| member == id) {
            return Err(format!("{old}.{composite}: relationship {id} is missing"));
        }
        old_members.retain(|member| member != id);
        old_owner.insert(composite.into(), references(old_members));
        let new_owner = owner_properties_mut(section, &target)?;
        let mut new_members = reference_collection(new_owner.get(composite), composite)?;
        if new_members.iter().any(|member| member == id) {
            return Err(format!(
                "{target}.{composite}: relationship already has contradictory ownership"
            ));
        }
        new_members.push(id.into());
        new_owner.insert(composite.into(), references(new_members));
        let ownership = section
            .relations
            .iter_mut()
            .find(|edge| edge.id == ownership_id)
            .ok_or("ownership edge disappeared")?;
        ownership.from = target.clone();
    }
    let relation = section
        .relations
        .iter_mut()
        .find(|relation| relation.id == id)
        .ok_or("relationship disappeared")?;
    if dependency {
        let superset = if source { "client" } else { "supplier" };
        let mut members = reference_collection(relation.properties.get(superset), superset)?;
        if !members.iter().any(|member| member == &old) {
            return Err(format!(
                "{id}.{superset}: specialized endpoint is missing from its UML superset"
            ));
        }
        // Preserve extra authored members and their order, including an already
        // present new target; replace only the old specialized membership.
        if members.contains(&target) {
            members.retain(|member| member != &old);
        } else if let Some(member) = members.iter_mut().find(|member| **member == old) {
            *member = target.clone();
        }
        relation
            .properties
            .insert(superset.into(), references(members));
    }
    relation
        .properties
        .insert(key.into(), json!({"$ref": target}));
    if source {
        relation.from = target.clone();
    } else {
        relation.to = target.clone();
    }
    retarget_views(section, id, source, &old, &target)?;
    Ok(true)
}

fn owner_properties_mut<'a>(
    section: &'a mut ModelingSection,
    id: &str,
) -> Result<&'a mut std::collections::BTreeMap<String, Value>, String> {
    if let Some(element) = section.elements.iter_mut().find(|element| element.id == id) {
        return Ok(&mut element.properties);
    }
    // Association is a Classifier even when its disposable projection is an edge.
    section
        .relations
        .iter_mut()
        .find(|relation| relation.id == id && relation.kind != "owns")
        .map(|relation| &mut relation.properties)
        .ok_or_else(|| format!("Unknown UML ownership source {id}"))
}

fn reference_collection(value: Option<&Value>, field: &str) -> Result<Vec<String>, String> {
    let values = value
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{field}: expected a canonical reference collection"))?;
    let mut seen = BTreeSet::new();
    let mut result = Vec::new();
    for value in values {
        let id = crate::uml::references::reference_id(value)?
            .ok_or_else(|| format!("{field}: null collection member"))?;
        if !seen.insert(id) {
            return Err(format!("{field}: duplicate reference {id}"));
        }
        result.push(id.to_owned());
    }
    Ok(result)
}

fn references(ids: Vec<String>) -> Value {
    Value::Array(ids.into_iter().map(|id| json!({"$ref": id})).collect())
}

fn retarget_views(
    section: &mut ModelingSection,
    relation_id: &str,
    source: bool,
    old: &str,
    target: &str,
) -> Result<(), String> {
    let relation = section
        .relations
        .iter()
        .find(|relation| relation.id == relation_id)
        .ok_or("relationship disappeared before retargeting its views")?;
    let unchanged = if source { &relation.to } else { &relation.from }.clone();
    let target_kind = crate::uml::root::Model::from_section(section)
        .element(target)
        .ok_or("new endpoint disappeared")?
        .kind()
        .to_owned();
    for diagram in &mut section.diagrams {
        let candidates: Vec<_> = diagram
            .all_views()
            .filter(|view| {
                view.subject.as_deref() == Some(target)
                    && endpoint_node_view(view, &target_kind)
                    && matches!(view.geometry, Geometry::Rect { x, y, width, height }
                        if width > 0.0 && height > 0.0
                            && [x, y, width, height, x + width, y + height]
                                .iter().all(|value| value.is_finite()
                                    && (*value as f32).is_finite()))
            })
            .map(|view| view.id.clone())
            .collect();
        let replacement = match candidates.as_slice() {
            [view] => Some(view.clone()),
            _ => None,
        };
        let unchanged_view = diagram
            .all_views()
            .find(|view| view.subject.as_deref() == Some(unchanged.as_str()))
            .map(|view| view.id.clone());
        let subjects: std::collections::BTreeMap<_, _> = diagram
            .all_views()
            .map(|view| (view.id.clone(), view.subject.clone()))
            .collect();
        retarget_children(
            &mut diagram.views,
            relation_id,
            source,
            old,
            replacement.as_deref(),
            unchanged_view.as_deref(),
            &subjects,
        )?;
    }
    Ok(())
}

fn endpoint_node_view(view: &View, semantic_kind: &str) -> bool {
    if view.draft.is_some()
        || view.presentation.visible == Some(false)
        || view.presentation.internal
        || view.presentation.label_binding.is_some()
        || (view.presentation.kind.is_some() && view.presentation.notation_kind.is_none())
    {
        return false;
    }
    let kind = view
        .presentation
        .notation_kind
        .as_deref()
        .unwrap_or(semantic_kind);
    // Use the same notation resolver as the canvas; a rectangle alone may be
    // an imported name label or compartment, not a classifier occurrence.
    matches!(
        crate::uml_notation(kind),
        crate::UmlNotation::Rectangle
            | crate::UmlNotation::ClassifierRectangle
            | crate::UmlNotation::AssociationDiamond
            | crate::UmlNotation::Actor
            | crate::UmlNotation::UseCase
            | crate::UmlNotation::DeploymentNode
            | crate::UmlNotation::Collaboration
            | crate::UmlNotation::Action
            | crate::UmlNotation::State
            | crate::UmlNotation::InteractionUse
            | crate::UmlNotation::Frame
    )
}

fn retarget_children(
    views: &mut [View],
    relation_id: &str,
    source: bool,
    old: &str,
    replacement: Option<&str>,
    unchanged_view: Option<&str>,
    subjects: &std::collections::BTreeMap<String, Option<String>>,
) -> Result<(), String> {
    for view in views {
        if view.subject.as_deref() == Some(relation_id)
            && matches!(view.geometry, Geometry::Path { .. })
        {
            if let Some(bindings) = &mut view.presentation.relationship_endpoints {
                let binding = if source {
                    &mut bindings.source
                } else {
                    &mut bindings.target
                };
                match &binding.target {
                    EndpointMarkerTarget::View { id } => {
                        if subjects.get(id).and_then(Option::as_deref) != Some(old) {
                            return Err(format!(
                                "View {}: authored endpoint marker does not refer to the old semantic endpoint",
                                view.id
                            ));
                        }
                        binding.target = match replacement {
                            Some(id) => EndpointMarkerTarget::View { id: id.into() },
                            None => EndpointMarkerTarget::RelationshipRole {
                                role: if source { "source" } else { "target" }.into(),
                            },
                        };
                    }
                    EndpointMarkerTarget::RelationshipRole { .. } => {}
                    EndpointMarkerTarget::ModelEnd { .. } => {
                        return Err(format!(
                            "View {}: ModelEnd marker retargeting is unsupported for this relationship",
                            view.id
                        ));
                    }
                }
            }
            // An explicit missing attachment keeps routing diagnostic instead of
            // silently selecting an arbitrary occurrence or retaining the old one.
            let endpoints = view.endpoints.get_or_insert_with(|| ViewEndpoints {
                source_view_id: if source {
                    None
                } else {
                    unchanged_view.map(str::to_owned)
                },
                target_view_id: if source {
                    unchanged_view.map(str::to_owned)
                } else {
                    None
                },
            });
            if source {
                endpoints.source_view_id = replacement.map(str::to_owned);
            } else {
                endpoints.target_view_id = replacement.map(str::to_owned);
            }
        }
        retarget_children(
            &mut view.children,
            relation_id,
            source,
            old,
            replacement,
            unchanged_view,
            subjects,
        )?;
    }
    Ok(())
}
