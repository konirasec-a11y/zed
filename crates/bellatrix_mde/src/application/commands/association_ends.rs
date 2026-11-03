use crate::{
    Element, ModelingSection,
    uml::{
        associations,
        references::ReferenceIndex,
        root::{ElementRef, Model},
        schema::{self, Aggregation},
    },
};
use serde_json::json;
use std::collections::{BTreeMap, HashSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AssociationEndOwner {
    Association,
    Classifier(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssociationEndDraft {
    pub name: String,
    pub type_id: String,
    pub owner: AssociationEndOwner,
    pub aggregation: Aggregation,
    pub multiplicity: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RemovedAssociationEnd {
    KeepClassifierOwned,
    ReparentToClassifier(String),
    DeleteAssociationOwned,
}

/// One ordered transaction permits temporary incomplete membership but never
/// publishes it. Removal always states what happens to the Property identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AssociationEndEdit {
    Insert {
        position: usize,
        end: AssociationEndDraft,
    },
    Attach {
        position: usize,
        property_id: String,
        owner: Option<AssociationEndOwner>,
    },
    Remove {
        property_id: String,
        disposition: RemovedAssociationEnd,
    },
    Move {
        property_id: String,
        position: usize,
    },
    Reparent {
        property_id: String,
        owner: AssociationEndOwner,
    },
    Retype {
        property_id: String,
        type_id: String,
    },
}

fn owner_id(
    section: &ModelingSection,
    association_id: &str,
    owner: &AssociationEndOwner,
) -> Result<String, String> {
    match owner {
        AssociationEndOwner::Association => Ok(association_id.into()),
        AssociationEndOwner::Classifier(id) => {
            let references = ReferenceIndex::new(Model::from_section(section));
            let owner = references.element(id)?;
            if !owner.conforms_to("Classifier") {
                return Err("Association end owner must be a Classifier".into());
            }
            if owner.conforms_to("Association") {
                return Err("Use Association ownership for this Association; specialized or different Association ownership is unsupported".into());
            }
            Ok(id.clone())
        }
    }
}

fn position(member_end_ids: &[String], property_id: &str) -> Result<usize, String> {
    member_end_ids
        .iter()
        .position(|id| id == property_id)
        .ok_or_else(|| {
            format!("Property {property_id} is not a current member end of this transaction")
        })
}

fn insertion_position(member_end_ids: &[String], position: usize) -> Result<(), String> {
    if position > member_end_ids.len() {
        return Err("Association member-end insertion position is out of range".into());
    }
    Ok(())
}

fn retype(section: &mut ModelingSection, property_id: &str, type_id: &str) -> Result<(), String> {
    let references = ReferenceIndex::new(Model::from_section(section));
    let property = references.element(property_id)?;
    if !property.conforms_to("Property") {
        return Err("Association member end must be a Property".into());
    }
    let descriptor = schema::uml_property(
        property
            .metaclass()
            .ok_or("Property metaclass is missing")?,
        "type",
    )
    .map_err(|error| error.to_string())?
    .ok_or("Property::type contract is missing")?;
    let value = json!({"$ref": type_id});
    references.validate(descriptor, &value)?;
    let property = section
        .elements
        .iter_mut()
        .find(|element| element.id == property_id)
        .ok_or("Association member Property not found")?;
    if let Some(text) = property
        .properties
        .get("type")
        .filter(|value| value.is_string())
        .cloned()
    {
        if property
            .properties
            .get("typeText")
            .is_some_and(|preserved| preserved != &text)
        {
            return Err("existing typeText differs from the legacy type declaration; preserve both before choosing a Type".into());
        }
        property.properties.insert("typeText".into(), text);
    }
    property.properties.insert("type".into(), value);
    Ok(())
}

fn fresh_property_id(
    section: &ModelingSection,
    project_id: &str,
    revision: u64,
    reserved: &mut HashSet<String>,
) -> Result<String, String> {
    let mut sequence = revision;
    loop {
        let [id] = super::fresh_ids(section, ["mde-association-end"], sequence, project_id)?;
        if reserved.insert(id.clone()) {
            return Ok(id);
        }
        sequence = sequence.checked_add(1).ok_or("identifier overflow")?;
    }
}

pub(super) fn apply(
    section: &mut ModelingSection,
    association_id: &str,
    edits: &[AssociationEndEdit],
    revision: u64,
    project_id: &str,
) -> Result<(), String> {
    let association = Model::from_section(section)
        .element(association_id)
        .ok_or("Association not found")?;
    if association.metaclass() != Some("Association") {
        return Err("Unsupported specialized Association constraints: atomic end editing currently supports Association itself".into());
    }
    if !association.properties().contains_key("memberEnd")
        && matches!(association, ElementRef::Relationship(_))
    {
        associations::migrate(section, project_id, association_id)?;
    }
    // Deleted identities stay reserved for this transaction. Reusing one would
    // silently retarget references that still name the removed Property.
    let mut reserved = section
        .elements
        .iter()
        .map(|element| element.id.clone())
        .chain(section.relations.iter().map(|relation| relation.id.clone()))
        .chain(section.diagrams.iter().map(|diagram| diagram.id.clone()))
        .chain(
            section
                .diagrams
                .iter()
                .flat_map(|diagram| diagram.views.iter().map(|view| view.id.clone())),
        )
        .chain(std::iter::once(project_id.to_owned()))
        .collect::<HashSet<_>>();
    let references = ReferenceIndex::new(Model::from_section(section));
    let association = references.element(association_id)?;
    let mut member_end_ids = if association.properties().contains_key("memberEnd") {
        associations::member_ends(&references, association_id)?
            .iter()
            .map(|end| end.id.clone())
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    for edit in edits {
        match edit {
            AssociationEndEdit::Insert { position, end } => {
                insertion_position(&member_end_ids, *position)?;
                let owner_id = owner_id(section, association_id, &end.owner)?;
                let id = fresh_property_id(section, project_id, revision, &mut reserved)?;
                let mut properties = BTreeMap::new();
                let aggregation = match end.aggregation {
                    Aggregation::None => "none",
                    Aggregation::Shared => "shared",
                    Aggregation::Composite => "composite",
                };
                properties.insert("aggregation".into(), json!(aggregation));
                if let Some(multiplicity) = &end.multiplicity {
                    properties.insert("multiplicity".into(), json!(multiplicity));
                }
                section.elements.push(Element {
                    id: id.clone(),
                    kind: "uml.attribute".into(),
                    name: end.name.clone(),
                    label: None,
                    description: None,
                    properties,
                    annotations: Vec::new(),
                    implementations: Vec::new(),
                    content: None,
                });
                retype(section, &id, &end.type_id)?;
                crate::uml::root::set_owner(section, project_id, &id, &owner_id)?;
                member_end_ids.insert(*position, id);
            }
            AssociationEndEdit::Attach {
                position,
                property_id,
                owner,
            } => {
                insertion_position(&member_end_ids, *position)?;
                let references = ReferenceIndex::new(Model::from_section(section));
                if !references.element(property_id)?.conforms_to("Property") {
                    return Err("Association member end must be a Property".into());
                }
                if member_end_ids.contains(property_id) {
                    return Err("Association::memberEnd requires unique Property identities".into());
                }
                if associations::association_of(&references, property_id)?
                    .is_some_and(|association| association.id() != association_id)
                {
                    return Err("Property already belongs to another Association; its membership cannot be silently transferred".into());
                }
                if let Some(owner) = owner {
                    let owner_id = owner_id(section, association_id, owner)?;
                    crate::uml::root::reparent(section, project_id, property_id, Some(&owner_id))?;
                }
                member_end_ids.insert(*position, property_id.clone());
            }
            AssociationEndEdit::Remove {
                property_id,
                disposition,
            } => {
                let position = position(&member_end_ids, property_id)?;
                let model = Model::from_section(section);
                let property = model
                    .element(property_id)
                    .ok_or("Association member Property not found")?;
                let owner = model
                    .owner(property)?
                    .ok_or("Association member Property has no owner")?;
                match disposition {
                    RemovedAssociationEnd::KeepClassifierOwned => {
                        if !owner.conforms_to("Classifier") || owner.conforms_to("Association") {
                            return Err("An Association-owned end needs explicit deletion or reparenting when removed; its Property cannot be silently orphaned".into());
                        }
                    }
                    RemovedAssociationEnd::ReparentToClassifier(id) => {
                        let owner_id = owner_id(
                            section,
                            association_id,
                            &AssociationEndOwner::Classifier(id.clone()),
                        )?;
                        crate::uml::root::reparent(
                            section,
                            project_id,
                            property_id,
                            Some(&owner_id),
                        )?;
                    }
                    RemovedAssociationEnd::DeleteAssociationOwned => {
                        if owner.id() != association_id {
                            return Err("Removing a classifier-owned end must retain its Property; delete it separately after detaching".into());
                        }
                        super::remove_element(section, property_id)?;
                    }
                }
                member_end_ids.remove(position);
            }
            AssociationEndEdit::Move {
                property_id,
                position: target,
            } => {
                let current = position(&member_end_ids, property_id)?;
                if *target >= member_end_ids.len() {
                    return Err("Association member-end move position is out of range".into());
                }
                let id = member_end_ids.remove(current);
                member_end_ids.insert(*target, id);
            }
            AssociationEndEdit::Reparent { property_id, owner } => {
                position(&member_end_ids, property_id)?;
                let owner_id = owner_id(section, association_id, owner)?;
                crate::uml::root::reparent(section, project_id, property_id, Some(&owner_id))?;
            }
            AssociationEndEdit::Retype {
                property_id,
                type_id,
            } => {
                position(&member_end_ids, property_id)?;
                retype(section, property_id, type_id)?;
            }
        }
        let value = json!(
            member_end_ids
                .iter()
                .map(|id| json!({"$ref": id}))
                .collect::<Vec<_>>()
        );
        if let Some(association) = section
            .relations
            .iter_mut()
            .find(|relation| relation.id == association_id)
        {
            association.properties.insert("memberEnd".into(), value);
        } else {
            section
                .elements
                .iter_mut()
                .find(|element| element.id == association_id)
                .ok_or("Association disappeared during end editing")?
                .properties
                .insert("memberEnd".into(), value);
        }
    }
    associations::set_member_ends(section, association_id, &member_end_ids)
}
