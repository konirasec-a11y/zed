//! Executable Connector-types over ordered canonical role references.
//! Connector-roles/allRoles and the other ConnectorEnd constraints remain separate.
use super::{
    classifier_hierarchy,
    classifiers::QueryError,
    feature_compatibility,
    property_contracts::{self, PropertyOutcome},
    references::ReferenceIndex,
    root::ElementRef,
    schema::{self, PropertyDescriptor},
};
use crate::{StructuralDiagnostic, StructuralSeverity};
use serde_json::Value;

fn property(element: ElementRef<'_>, name: &str) -> Result<&'static PropertyDescriptor, QueryError> {
    schema::uml_property(element.metaclass().ok_or_else(|| QueryError::unsupported(element.id(), "unresolved metaclass"))?, name)
        .map_err(|error| QueryError::unsupported(element.id(), error.to_string()))?
        .ok_or_else(|| QueryError::unsupported(element.id(), format!("missing {name} contract")))
}
fn check(references: &ReferenceIndex<'_>, element: ElementRef<'_>, property: &PropertyDescriptor, value: &Value) -> Result<bool, QueryError> {
    match property_contracts::check_value(references, element, property, value) {
        PropertyOutcome::Satisfied => Ok(true),
        PropertyOutcome::Violation(message) => Err(QueryError::invalid(element.id(), message)),
        PropertyOutcome::Unverified(message) => Err(QueryError::unsupported(element.id(), message)),
    }
}
fn reference<'a>(references: &ReferenceIndex<'a>, element: ElementRef<'a>, property: &PropertyDescriptor, value: &Value) -> Result<Option<ElementRef<'a>>, QueryError> {
    check(references, element, property, value)?;
    references.validate(property, value).map_err(|error| QueryError::invalid(element.id(), error))
}
fn pair_conforms(
    references: &ReferenceIndex<'_>,
    connector: ElementRef<'_>,
    end_descriptor: &PropertyDescriptor,
    end: &Value,
    association: ElementRef<'_>,
    member_descriptor: &PropertyDescriptor,
    member: &Value,
) -> Result<bool, QueryError> {
    let specific = (|| {
        let end = reference(references, connector, end_descriptor, end)?
            .ok_or_else(|| QueryError::invalid(connector.id(), "ConnectorEnd cannot be absent"))?;
        let role = reference(references, end, property(end, "role")?, end.properties().get("role").unwrap_or(&Value::Null))?
            .ok_or_else(|| QueryError::invalid(end.id(), "ConnectorEnd::role is required"))?;
        feature_compatibility::resolve_feature_type(references, role)
    })();
    let general = (|| {
        let member = reference(references, association, member_descriptor, member)?
            .ok_or_else(|| QueryError::invalid(association.id(), "memberEnd cannot be absent"))?;
        feature_compatibility::resolve_feature_type(references, member)
    })();
    // Inspect both sides before selecting an unknown result. A malformed Type
    // cannot be concealed by an unrelated external role or end reference.
    let (specific, general) = match (specific, general) {
        (Err(error @ QueryError::Invalid { .. }), _) | (_, Err(error @ QueryError::Invalid { .. })) => return Err(error),
        (Err(error), _) | (_, Err(error)) => return Err(error),
        (Ok(specific), Ok(general)) => (specific, general),
    };
    if specific.conforms_to("Classifier") && general.conforms_to("Classifier") {
        if specific.id() == general.id() { return Ok(true); }
        classifier_hierarchy::has_ancestor(references, specific.id(), general.id())
    } else {
        feature_compatibility::compare_resolved_types(references, specific, general)
    }
}
fn types_rule(references: &ReferenceIndex<'_>, connector: ElementRef<'_>) -> Result<bool, QueryError> {
    let Some(association) = reference(references, connector, property(connector, "type")?,
        connector.properties().get("type").unwrap_or(&Value::Null))?
    else { return Ok(true); };
    let end_property = property(connector, "end")?;
    let member_property = property(association, "memberEnd")?;
    let ends = connector.properties().get("end");
    let members = association.properties().get("memberEnd");
    // Check both contracts before combining. Missing ordering on one side is
    // unknown, not permission to hide an explicit malformed opposite role.
    let mut checks = Vec::new();
    for (element, descriptor, value) in [
        (connector, end_property, ends),
        (association, member_property, members),
    ] {
        checks.push(match value {
            Some(value) => check(references, element, descriptor, value),
            None => Err(QueryError::unsupported(element.id(),
                format!("ordered {} role is not materialized", descriptor.name))),
        });
        if value.is_some_and(|value| !value.is_array()) {
            checks.push(Err(QueryError::invalid(element.id(),
                format!("{} must be an ordered collection", descriptor.name))));
        }
    }
    if let (Some(ends), Some(members)) = (
        ends.and_then(Value::as_array),
        members.and_then(Value::as_array),
    ) {
        checks.push(Ok(ends.len() == members.len()));
        let single_end = PropertyDescriptor { lower:"1", upper:"1", ..*end_property };
        let single_member = PropertyDescriptor { lower:"1", upper:"1", ..*member_property };
        for (end, member) in ends.iter().zip(members) {
            checks.push(pair_conforms(references, connector, &single_end, end, association, &single_member, member));
        }
    }
    // All positions are checked: false is decisive beside unresolved values,
    // while malformed references remain Invalid rather than a successful result.
    feature_compatibility::require_all(checks)
}

pub fn validate(references: &ReferenceIndex<'_>) -> Vec<StructuralDiagnostic> {
    let mut issues = Vec::new();
    for connector in references.model().elements().filter(|element| element.conforms_to("Connector")) {
        let (severity, code, message) = match types_rule(references, connector) {
            Ok(true) => continue,
            Ok(false) => (StructuralSeverity::Error, "UML_CONNECTOR_TYPES", "end count or positional role Type conformance is violated".to_owned()),
            Err(QueryError::Invalid { message, .. }) => (StructuralSeverity::Error, "UML_CONNECTOR_TYPES", message),
            Err(QueryError::Unsupported { message, .. }) => (StructuralSeverity::Warning, "MDE_CONNECTOR_TYPES_UNVERIFIED", message),
        };
        issues.push(StructuralDiagnostic { severity, code, subject_id:connector.id().into(), message:format!("Connector-types: {message}") });
    }
    issues
}
