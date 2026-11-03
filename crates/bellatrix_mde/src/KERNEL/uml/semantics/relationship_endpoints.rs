use super::{
    associations, classifiers,
    element_graph_queries::ElementGraphError,
    references::ReferenceIndex,
    root::ElementRef,
    schema::{self, PropertyDescriptor},
};
use crate::{StructuralDiagnostic, StructuralSeverity};
use serde_json::{Value, json};
use std::collections::HashSet;

fn invalid(id: &str, message: impl Into<String>) -> ElementGraphError {
    ElementGraphError::Invalid { subject_id: id.into(), message: message.into() }
}

fn unsupported(id: &str, message: impl Into<String>) -> ElementGraphError {
    ElementGraphError::Unsupported { subject_id: id.into(), message: message.into() }
}

fn element<'a>(
    references: &ReferenceIndex<'a>,
    id: &str,
    metaclass: &str,
) -> Result<ElementRef<'a>, ElementGraphError> {
    let element = references.element(id).map_err(|error| invalid(id, error))?;
    if !element.conforms_to(metaclass) {
        return Err(invalid(id, format!("expected a UML {metaclass}")));
    }
    Ok(element)
}

fn descriptor(
    id: &str,
    metaclass: &str,
    name: &str,
) -> Result<&'static PropertyDescriptor, ElementGraphError> {
    schema::uml_property(metaclass, name)
        .map_err(|error| unsupported(id, error.to_string()))?
        .ok_or_else(|| unsupported(id, format!("missing {metaclass}::{name} contract")))
}

fn read<'a>(
    references: &ReferenceIndex<'a>,
    subject: ElementRef<'a>,
    property: &PropertyDescriptor,
    value: &Value,
) -> Result<Vec<ElementRef<'a>>, ElementGraphError> {
    if property.property_type.resolution != schema::ReferenceResolution::Exact {
        return Err(unsupported(subject.id(), format!(
            "{} requires resolved local Element references", property.id.fragment,
        )));
    }
    let result = if matches!(property.upper, "0" | "1") {
        references.validate(property, value)
            .map(|value| value.into_iter().collect())
    } else {
        references.validate_many(property, value)
    };
    result.map_err(|error| {
        // ReferenceIndex's string error cannot distinguish an unsupported
        // endpoint adapter from a proven type violation. Check the complete
        // collection so an unknown endpoint cannot hide a malformed neighbor.
        let contract = PropertyDescriptor {
            is_derived: false,
            aggregation: schema::Aggregation::None,
            ..*property
        };
        match super::property_contracts::check_value(references, subject, &contract, value) {
            super::property_contracts::PropertyOutcome::Unverified(message) =>
                unsupported(subject.id(), message),
            super::property_contracts::PropertyOutcome::Violation(message) =>
                invalid(subject.id(), message),
            super::property_contracts::PropertyOutcome::Satisfied => invalid(subject.id(), error),
        }
    })
}

fn unique<'a>(values: impl IntoIterator<Item = ElementRef<'a>>) -> Vec<ElementRef<'a>> {
    let mut seen = HashSet::new();
    values.into_iter().filter(|element| seen.insert(element.id())).collect()
}

fn references_value(values: &[ElementRef<'_>]) -> Value {
    Value::Array(values.iter().map(|element| json!({"$ref": element.id()})).collect())
}

fn stored_subsets<'a>(
    references: &ReferenceIndex<'a>,
    subject: ElementRef<'a>,
    target: &'static str,
) -> Result<Vec<ElementRef<'a>>, ElementGraphError> {
    let properties = classifiers::properties(subject)
        .map_err(|error| unsupported(subject.id(), error.to_string()))?;
    let mut values = Vec::new();
    for property in properties {
        if property.id.fragment == target || property.is_derived {
            continue;
        }
        if classifiers::contributes(property, target)
            .map_err(|error| unsupported(subject.id(), error))?
            && let Some(value) = subject.properties().get(property.name)
        {
            values.extend(read(references, subject, property, value)?);
        }
    }
    Ok(unique(values))
}

fn dependency_role<'a>(
    references: &ReferenceIndex<'a>,
    id: &str,
    name: &'static str,
) -> Result<Vec<ElementRef<'a>>, ElementGraphError> {
    let dependency = element(references, id, "Dependency")?;
    let property = descriptor(id, "Dependency", name)?;
    let subsets = stored_subsets(references, dependency, property.id.fragment)?;
    if let Some(value) = dependency.properties().get(name) {
        let values = read(references, dependency, property, value)?;
        if subsets.iter().any(|subset| !values.iter().any(|value| value.id() == subset.id())) {
            return Err(invalid(id, format!("specialized roles must subset Dependency::{name}")));
        }
        return Ok(values);
    }
    if !subsets.is_empty() {
        return read(references, dependency, property, &references_value(&subsets));
    }
    // Existing binary records supply the role only when no explicit collection
    // or specialized role exists. Their diagram projection never truncates it.
    let value = match dependency {
        ElementRef::Relationship(relation) => {
            let endpoint = if name == "client" { &relation.from } else { &relation.to };
            json!([{"$ref": endpoint}])
        }
        ElementRef::Object(_) => Value::Array(Vec::new()),
    };
    read(references, dependency, property, &value)
}

pub fn clients<'a>(references: &ReferenceIndex<'a>, id: &str)
    -> Result<Vec<ElementRef<'a>>, ElementGraphError>
{
    dependency_role(references, id, "client")
}

pub fn suppliers<'a>(references: &ReferenceIndex<'a>, id: &str)
    -> Result<Vec<ElementRef<'a>>, ElementGraphError>
{
    dependency_role(references, id, "supplier")
}

fn ensure_derived(subject: ElementRef<'_>, name: &str) -> Result<(), ElementGraphError> {
    if subject.properties().contains_key(name) {
        return Err(unsupported(subject.id(), format!(
            "{name} must be derived from canonical relationship roles",
        )));
    }
    Ok(())
}

fn directed_role<'a>(references: &ReferenceIndex<'a>, id: &str, name: &'static str)
    -> Result<Vec<ElementRef<'a>>, ElementGraphError>
{
    let subject = element(references, id, "DirectedRelationship")?;
    ensure_derived(subject, name)?;
    if subject.conforms_to("Dependency") {
        return if name == "source" { clients(references, id) } else { suppliers(references, id) };
    }
    let property = descriptor(id, "DirectedRelationship", name)?;
    let mut values = stored_subsets(references, subject, property.id.fragment)?;
    if values.is_empty() && let ElementRef::Relationship(relation) = subject {
        let endpoint = if name == "source" { &relation.from } else { &relation.to };
        values.push(references.element(endpoint).map_err(|error| invalid(id, error))?);
    }
    read(references, subject, property, &references_value(&values))
}

pub fn sources<'a>(references: &ReferenceIndex<'a>, id: &str)
    -> Result<Vec<ElementRef<'a>>, ElementGraphError>
{
    directed_role(references, id, "source")
}

pub fn targets<'a>(references: &ReferenceIndex<'a>, id: &str)
    -> Result<Vec<ElementRef<'a>>, ElementGraphError>
{
    directed_role(references, id, "target")
}

pub fn related_elements<'a>(references: &ReferenceIndex<'a>, id: &str)
    -> Result<Vec<ElementRef<'a>>, ElementGraphError>
{
    let subject = element(references, id, "Relationship")?;
    ensure_derived(subject, "relatedElement")?;
    let values = if subject.conforms_to("DirectedRelationship") {
        unique(sources(references, id)?.into_iter().chain(targets(references, id)?))
    } else if subject.conforms_to("Association")
        && (subject.properties().contains_key("memberEnd") || matches!(subject, ElementRef::Object(_)))
    {
        if !subject.properties().contains_key("memberEnd") {
            return Err(unsupported(id, "Association memberEnd identities are not materialized; related Elements are unverified"));
        }
        associations::end_types(references, id).map_err(|error| invalid(id, error))?
    } else if let ElementRef::Relationship(relation) = subject {
        unique([
            references.element(&relation.from).map_err(|error| invalid(id, error))?,
            references.element(&relation.to).map_err(|error| invalid(id, error))?,
        ])
    } else {
        stored_subsets(references, subject, "Relationship-relatedElement")?
    };
    let property = descriptor(id, "Relationship", "relatedElement")?;
    read(references, subject, property, &references_value(&values))
}

pub(crate) fn validate(references: &ReferenceIndex<'_>) -> Vec<StructuralDiagnostic> {
    let mut issues = Vec::new();
    for subject in references.model().elements().filter(|element| element.conforms_to("Relationship")) {
        let result = related_elements(references, subject.id()).and_then(|_| {
            if subject.conforms_to("DirectedRelationship") && let ElementRef::Relationship(relation) = subject {
                for (name, endpoint, values) in [
                    ("source", relation.from.as_str(), sources(references, subject.id())?),
                    ("target", relation.to.as_str(), targets(references, subject.id())?),
                ] {
                    if !values.iter().any(|value| value.id() == endpoint) {
                        return Err(invalid(subject.id(), format!(
                            "binary projection endpoint {endpoint} must belong to DirectedRelationship::{name}",
                        )));
                    }
                }
            }
            Ok(())
        });
        if let Err(error) = result {
            let (severity, code) = match error {
                ElementGraphError::Invalid { .. } => (StructuralSeverity::Error, "UML_RELATIONSHIP_ENDPOINTS"),
                ElementGraphError::Unsupported { .. } => (StructuralSeverity::Warning, "MDE_RELATIONSHIP_ENDPOINTS_UNVERIFIED"),
            };
            issues.push(StructuralDiagnostic {
                severity, code, subject_id: subject.id().into(), message: error.to_string(),
            });
        }
    }
    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Project, uml::root::Model};

    fn project(source_kind: &str, properties: Value) -> serde_json::Result<Project> {
        serde_json::from_value(json!({
            "id": "project", "name": "Endpoints",
            "elements": [
                {"id": "source", "name": "Source", "kind": source_kind},
                {"id": "target", "name": "Target", "kind": "uml.interface"},
                {"id": "invalid", "name": "Comment", "kind": "uml.comment"}
            ],
            "relations": [{"id": "edge", "kind": "uml.interface_realization",
                "from": "source", "to": "target", "properties": properties}]
        }))
    }

    #[test]
    fn unknown_endpoint_adapter_is_unverified_not_invalid() -> serde_json::Result<()> {
        let project = project("uml.template", json!({}))?;
        let references = ReferenceIndex::new(Model::from_project(&project));
        assert!(matches!(clients(&references, "edge"),
            Err(ElementGraphError::Unsupported { .. })));
        assert!(super::validate(&references).iter().all(|issue|
            issue.severity == StructuralSeverity::Warning));
        Ok(())
    }

    #[test]
    fn unverified_endpoint_does_not_hide_invalid_collection_members() -> serde_json::Result<()> {
        for invalid in [json!({"$ref": "invalid"}), json!({"$ref": "missing"}), Value::Null] {
            let project = project("uml.template", json!({
                "client": [{"$ref": "source"}, invalid]
            }))?;
            let references = ReferenceIndex::new(Model::from_project(&project));
            assert!(matches!(clients(&references, "edge"),
                Err(ElementGraphError::Invalid { .. })));
        }
        Ok(())
    }

    #[test]
    fn external_endpoint_does_not_hide_violations_in_either_order() -> serde_json::Result<()> {
        let external = json!({"href": "external.uml#Classifier"});
        for invalid in [json!({"$ref": "invalid"}), json!({"$ref": "missing"}), Value::Null] {
            for values in [json!([external, invalid]), json!([invalid, external])] {
                let project = project("uml.class", json!({"client": values}))?;
                let references = ReferenceIndex::new(Model::from_project(&project));
                assert!(matches!(clients(&references, "edge"),
                    Err(ElementGraphError::Invalid { .. })));
            }
        }
        Ok(())
    }

    #[test]
    fn unresolved_endpoints_do_not_hide_duplicate_references() -> serde_json::Result<()> {
        let external = json!({"href": "external.uml#Classifier"});
        let local = json!({"$ref": "target"});
        let unknown = json!({"$ref": "source"});
        for values in [
            json!([external, local, local]), json!([local, local, external]),
            json!([unknown, local, local]), json!([local, local, unknown]),
            json!([external, external]), json!([unknown, unknown]),
        ] {
            let project = project("uml.template", json!({"client": values}))?;
            let references = ReferenceIndex::new(Model::from_project(&project));
            assert!(matches!(clients(&references, "edge"),
                Err(ElementGraphError::Invalid { .. })));
        }
        Ok(())
    }

    #[test]
    fn external_endpoint_without_proven_violation_stays_unverified() -> serde_json::Result<()> {
        let project = project("uml.class", json!({
            "client": [{"href": "external.uml#Classifier"}, {"$ref": "source"}]
        }))?;
        let references = ReferenceIndex::new(Model::from_project(&project));
        assert!(matches!(clients(&references, "edge"),
            Err(ElementGraphError::Unsupported { .. })));
        Ok(())
    }

    #[test]
    fn interface_realization_source_keeps_specialized_error_code() -> serde_json::Result<()> {
        let project = project("uml.artifact", json!({}))?;
        let references = ReferenceIndex::new(Model::from_project(&project));
        assert!(super::super::property_contracts::validate(&references).iter().any(|issue|
            issue.code == "UML_INTERFACE_REALIZATION_SOURCE"
                && issue.subject_id == "edge"
                && issue.severity == StructuralSeverity::Error));
        Ok(())
    }
}
