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

fn dependency_subsets<'a>(
    references: &ReferenceIndex<'a>,
    dependency: ElementRef<'a>,
    property: &PropertyDescriptor,
) -> Result<Vec<ElementRef<'a>>, ElementGraphError> {
    let name = property.name;
    let mut subsets = stored_subsets(references, dependency, property.id.fragment)?;
    if name == "client" && dependency.conforms_to("Substitution") {
        let owner = super::classifier_hierarchy::substituting_classifier(references, dependency)
            .map_err(|error| match error {
                super::classifiers::QueryError::Invalid { subject_id, message } => invalid(&subject_id, message),
                super::classifiers::QueryError::Unsupported { subject_id, message } => unsupported(&subject_id, message),
            })?;
        if !subsets.iter().any(|element| element.id() == owner.id()) { subsets.push(owner); }
    }
    Ok(subsets)
}

/// Prove specialized-role inclusion without pretending an unresolved endpoint
/// collection is completely known. Malformed collections still take precedence;
/// global property/endpoint validation retains unresolved-reference diagnostics.
pub(crate) fn validate_dependency_subsets(
    references: &ReferenceIndex<'_>,
    id: &str,
) -> Result<(), ElementGraphError> {
    let dependency = element(references, id, "Dependency")?;
    let mut unknown = None;
    for name in ["client", "supplier"] {
        let result = (|| {
            let property = descriptor(id, "Dependency", name)?;
            let subsets = dependency_subsets(references, dependency, property)?;
            let Some(value) = dependency.properties().get(name) else {
                return dependency_role(references, id, name).map(|_| ());
            };
            match read(references, dependency, property, value) {
                Ok(values) => {
                    if subsets.iter().all(|subset| values.iter().any(|value| value.id() == subset.id())) {
                        Ok(())
                    } else {
                        Err(invalid(id, format!("specialized roles must subset Dependency::{name}")))
                    }
                }
                Err(error @ ElementGraphError::Unsupported { .. }) => {
                    // A known identity proves includes even beside unknowns.
                    // read() has already rejected any independently bad member.
                    let known = value.as_array().into_iter().flatten()
                        .filter_map(|value| super::references::reference_id(value).ok().flatten())
                        .collect::<HashSet<_>>();
                    if subsets.iter().all(|subset| known.contains(subset.id())) {
                        Ok(())
                    } else {
                        Err(error)
                    }
                }
                Err(error) => Err(error),
            }
        })();
        match result {
            Ok(()) => {}
            Err(error @ ElementGraphError::Unsupported { .. }) => unknown = Some(error),
            Err(error) => return Err(error),
        }
    }
    unknown.map_or(Ok(()), Err)
}

fn dependency_role<'a>(
    references: &ReferenceIndex<'a>,
    id: &str,
    name: &'static str,
) -> Result<Vec<ElementRef<'a>>, ElementGraphError> {
    let dependency = element(references, id, "Dependency")?;
    let property = descriptor(id, "Dependency", name)?;
    let subsets = dependency_subsets(references, dependency, property)?;
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

fn containment_owner<'a>(
    references: &ReferenceIndex<'a>,
    subject: ElementRef<'a>,
) -> Result<ElementRef<'a>, ElementGraphError> {
    references.model().owner(subject)
        .map_err(|error| unsupported(subject.id(), error))?
        .ok_or_else(|| unsupported(subject.id(), "canonical containment is incomplete"))
}

fn containment_role(
    references: &ReferenceIndex<'_>,
    subject: ElementRef<'_>,
    owner: ElementRef<'_>,
    role: &str,
    composite: &'static str,
    alternatives: &[&str],
) -> Result<(), ElementGraphError> {
    references.element(subject.id()).map_err(|error| unsupported(subject.id(), error))?;
    references.element(owner.id()).map_err(|error| unsupported(subject.id(), error))?;
    for name in alternatives {
        if let Some(value) = subject.properties().get(*name) {
            let expected = if *name == role { Some(owner.id()) } else { None };
            if super::references::reference_id(value).ok() != Some(expected) {
                return Err(unsupported(subject.id(), format!(
                    "authored {name} does not resolve to canonical containment",
                )));
            }
        }
    }
    match classifiers::contributes_to_owned_union(owner, subject, composite) {
        Ok(true) => Ok(()),
        Ok(false) => Err(unsupported(subject.id(), format!("canonical {composite} membership is unresolved"))),
        Err(error) => Err(unsupported(subject.id(), error.to_string())),
    }
}

fn containing_context<'a>(
    references: &ReferenceIndex<'a>,
    subject: ElementRef<'a>,
    activity: bool,
) -> Result<ElementRef<'a>, ElementGraphError> {
    let mut current = subject;
    let mut visited = HashSet::new();
    loop {
        if !visited.insert(current.id()) {
            return Err(unsupported(subject.id(), "containment query encountered a cycle"));
        }
        let owner = containment_owner(references, current)?;
        let (role, composite, alternatives): (&str, &'static str, &[&str]) = if activity {
            if !current.conforms_to("ActivityNode") {
                return Err(unsupported(current.id(), "containingActivity requires a resolved ActivityNode"));
            }
            if owner.conforms_to("StructuredActivityNode") {
                ("inStructuredNode", "StructuredActivityNode-node", &["activity", "inStructuredNode"])
            } else if owner.conforms_to("Activity") {
                ("activity", "Activity-node", &["activity", "inStructuredNode"])
            } else {
                return Err(unsupported(current.id(), "containingActivity has no supported containment path"));
            }
        } else if current.conforms_to("Region") {
            if owner.conforms_to("StateMachine") {
                ("stateMachine", "StateMachine-region", &["state", "stateMachine"])
            } else if owner.conforms_to("State") {
                ("state", "State-region", &["state", "stateMachine"])
            } else {
                return Err(unsupported(current.id(), "Region containingStateMachine is unresolved"));
            }
        } else if current.conforms_to("Transition") && owner.conforms_to("Region") {
            ("container", "Region-transition", &["container"])
        } else if current.conforms_to("Vertex") && owner.conforms_to("Region") {
            ("container", "Region-subvertex", &["container"])
        } else if current.conforms_to("ConnectionPointReference") && owner.conforms_to("State") {
            ("state", "State-connection", &["container", "state"])
        } else if current.conforms_to("Pseudostate") && owner.conforms_to("StateMachine") {
            let kind = schema::read_effective_scalar(current, "kind")
                .map_err(|error| unsupported(current.id(), error))?;
            if !matches!(kind.as_str(), Some("entryPoint" | "exitPoint")) {
                return Err(unsupported(current.id(), "Vertex containingStateMachine has no supported connection-point path"));
            }
            ("stateMachine", "StateMachine-connectionPoint", &["container", "state", "stateMachine"])
        } else {
            return Err(unsupported(current.id(), "containingStateMachine has no supported containment path"));
        };
        containment_role(references, current, owner, role, composite, alternatives)?;
        if owner.conforms_to(if activity { "Activity" } else { "StateMachine" }) {
            return Ok(owner);
        }
        current = owner;
    }
}

fn contained_endpoint<'a>(
    references: &ReferenceIndex<'a>,
    subject: ElementRef<'a>,
    role: &str,
) -> Result<ElementRef<'a>, ElementGraphError> {
    let metaclass = subject.metaclass().ok_or_else(|| unsupported(subject.id(), "relationship metaclass is unresolved"))?;
    let property = descriptor(subject.id(), metaclass, role)?;
    let canonical = match subject {
        ElementRef::Relationship(relation) => Some(json!({"$ref": if role == "source" { &relation.from } else { &relation.to }})),
        ElementRef::Object(_) => None,
    };
    let authored = subject.properties().get(role);
    if let (Some(canonical), Some(authored)) = (&canonical, authored)
        && canonical != authored
    {
        return Err(unsupported(subject.id(), format!("{role} differs from its binary projection")));
    }
    let value = canonical.as_ref().or(authored)
        .ok_or_else(|| unsupported(subject.id(), format!("{role} is not materialized")))?;
    let values = read(references, subject, property, value)?;
    match values.as_slice() {
        [endpoint] => Ok(*endpoint),
        _ => Err(unsupported(subject.id(), format!("{role} does not resolve to one endpoint"))),
    }
}

fn endpoint_containment(
    references: &ReferenceIndex<'_>,
    subject: ElementRef<'_>,
    activity: bool,
) -> Result<(), ElementGraphError> {
    let context = if activity {
        let owner = containment_owner(references, subject)?;
        if owner.conforms_to("StructuredActivityNode") {
            containment_role(references, subject, owner, "inStructuredNode", "StructuredActivityNode-edge", &["activity", "inStructuredNode"])?;
            return Ok(());
        }
        if !owner.conforms_to("Activity") {
            return Err(unsupported(subject.id(), "ActivityEdge::activity is unresolved"));
        }
        containment_role(references, subject, owner, "activity", "Activity-edge", &["activity", "inStructuredNode"])?;
        owner
    } else {
        containing_context(references, subject, false)?
    };
    let mut unresolved = None;
    for role in ["source", "target"] {
        let result = contained_endpoint(references, subject, role)
            .and_then(|endpoint| containing_context(references, endpoint, activity));
        match result {
            Ok(actual) if actual.id() != context.id() => return Err(invalid(subject.id(), format!(
                "{role} belongs to {}, but the relationship belongs to {}", actual.id(), context.id(),
            ))),
            Ok(_) => {}
            Err(error @ ElementGraphError::Invalid { .. }) => return Err(error),
            Err(error) => unresolved = Some(error),
        }
    }
    unresolved.map_or(Ok(()), Err)
}

pub(crate) fn validate(references: &ReferenceIndex<'_>) -> Vec<StructuralDiagnostic> {
    let mut issues = Vec::new();
    for subject in references.model().elements().filter(|element| {
        element.conforms_to("Relationship") || element.conforms_to("ActivityEdge") || element.conforms_to("Transition")
    }) {
        let containment_rule = if subject.conforms_to("ActivityEdge") {
            Some((true, "ActivityEdge-source_and_target", "UML_ACTIVITY_EDGE_SOURCE_AND_TARGET", "MDE_ACTIVITY_EDGE_SOURCE_AND_TARGET_UNVERIFIED"))
        } else if subject.conforms_to("Transition") {
            Some((false, "Transition-transition_vertices", "UML_TRANSITION_TRANSITION_VERTICES", "MDE_TRANSITION_TRANSITION_VERTICES_UNVERIFIED"))
        } else {
            None
        };
        if let Some((activity, rule, code, unknown)) = containment_rule
            && let Err(error) = endpoint_containment(references, subject, activity)
        {
            let (severity, code) = match &error {
                ElementGraphError::Invalid { .. } => (StructuralSeverity::Error, code),
                ElementGraphError::Unsupported { .. } => (StructuralSeverity::Warning, unknown),
            };
            issues.push(StructuralDiagnostic {
                severity, code, subject_id: subject.id().into(), message: format!("{rule}: {error}"),
            });
        }
        if !subject.conforms_to("Relationship") { continue; }
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
