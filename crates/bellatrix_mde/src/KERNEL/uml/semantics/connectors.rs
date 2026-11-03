//! Executable Connector and ConnectorEnd constraints over canonical references.
//! ConnectorEnd multiplicity and definingEnd derivation remain separate.
use super::{
    classifier_hierarchy,
    classifiers::{self, QueryError},
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

struct StructuredClassifier<'a> {
    element: Option<ElementRef<'a>>,
    outcome: Result<bool, QueryError>,
}

fn structured_classifier<'a>(
    references: &ReferenceIndex<'a>,
    connector: ElementRef<'a>,
) -> StructuredClassifier<'a> {
    let canonical_owner = match references.model().owner(connector) {
        Ok(owner) => owner,
        Err(error) => return StructuredClassifier {
            element: None, outcome: Err(QueryError::invalid(connector.id(), error)),
        },
    };
    let owner = (|| match canonical_owner {
        Some(owner) if owner.conforms_to("StructuredClassifier") => {
            if classifiers::contributes_to_owned_union(owner, connector, "StructuredClassifier-ownedConnector")? {
                Ok(Some(owner))
            } else { Ok(None) }
        }
        Some(owner) if owner.metaclass().is_none() =>
            Err(QueryError::unsupported(connector.id(), "owner metaclass is unresolved")),
        _ => Ok(None),
    })();
    let stored = (|| {
        let Some(stored) = connector.properties().get("structuredClassifier") else { return Ok(true); };
        let descriptor = schema::property_by_id(schema::XmiId::new(schema::Document::Uml,
            "A_ownedConnector_structuredClassifier-structuredClassifier"))
            .ok_or_else(|| QueryError::unsupported(connector.id(), "missing structuredClassifier contract"))?;
        let checked = check(references, connector, descriptor, stored);
        // An unknown owner metaclass cannot hide a contradictory local inverse.
        // When its semantic role is known, compare against that derived owner.
        if let Ok(id) = super::references::reference_id(stored) {
            let expected = owner.as_ref().copied().unwrap_or(canonical_owner);
            if id != expected.map(|owner| owner.id()) {
                return Err(QueryError::invalid(connector.id(),
                    "structuredClassifier must agree with canonical ownedConnector ownership"));
            }
        }
        checked
    })();
    let outcome = feature_compatibility::require_all([
        stored,
        owner.as_ref().map(|owner| owner.is_some()).map_err(Clone::clone),
    ]);
    // An unresolved stored inverse must not discard known canonical membership.
    StructuredClassifier { element: owner.ok().flatten(), outcome }
}

fn end_role(
    references: &ReferenceIndex<'_>,
    owner: Option<ElementRef<'_>>,
    end: ElementRef<'_>,
) -> Result<bool, QueryError> {
    // Resolve both inputs before combining so a malformed part is not hidden by
    // an unresolved role, or by a role already present in allRoles.
    let role = reference(references, end, property(end, "role")?,
        end.properties().get("role").unwrap_or(&Value::Null));
    let part = reference(references, end, property(end, "partWithPort")?,
        end.properties().get("partWithPort").unwrap_or(&Value::Null));
    if let Err(error @ QueryError::Invalid { .. }) = &role { return Err(error.clone()); }
    if let Err(error @ QueryError::Invalid { .. }) = &part { return Err(error.clone()); }
    let Some(owner) = owner else {
        return feature_compatibility::require_all([
            role.map(|_| true), part.map(|_| true),
        ]);
    };
    let direct = match &role {
        Ok(Some(role)) => classifiers::has_role(references, owner.id(), role.id()),
        Ok(None) => Err(QueryError::invalid(end.id(), "ConnectorEnd::role is required")),
        Err(error) => Err(error.clone()),
    };
    let port = match &role {
        Ok(Some(role)) => Ok(role.conforms_to("Port")),
        Ok(None) => Err(QueryError::invalid(end.id(), "ConnectorEnd::role is required")),
        Err(error) => Err(error.clone()),
    };
    let part_member = match part {
        Ok(Some(part)) => classifiers::has_role(references, owner.id(), part.id()),
        Ok(None) => Ok(false),
        Err(error) => Err(error),
    };
    feature_compatibility::accept_any([
        direct, feature_compatibility::require_all([port, part_member]),
    ])
}

fn roles_rule(references: &ReferenceIndex<'_>, connector: ElementRef<'_>) -> Result<bool, QueryError> {
    let owner = structured_classifier(references, connector);
    let descriptor = property(connector, "end")?;
    let ends = connector.properties().get("end");
    let mut checks = vec![owner.outcome];
    match ends {
        None => checks.push(Err(QueryError::unsupported(connector.id(),
            "Connector::end role is not materialized"))),
        Some(value) => {
            checks.push(check(references, connector, descriptor, value));
            if let Some(ends) = value.as_array() {
                let single = PropertyDescriptor { lower:"1", upper:"1", ..*descriptor };
                for value in ends {
                    // Resolve every known end even if its owner is unknown.
                    let result = reference(references, connector, &single, value);
                    checks.push(match result {
                        Err(error) => Err(error),
                        Ok(Some(end)) => end_role(references, owner.element, end),
                        Ok(None) => Err(QueryError::invalid(connector.id(), "end cannot be null")),
                    });
                }
            } else {
                checks.push(Err(QueryError::invalid(connector.id(), "end must be a collection")));
            }
        }
    }
    feature_compatibility::require_all(checks)
}

fn end_reference<'a>(references: &ReferenceIndex<'a>, end: ElementRef<'a>, name: &str) -> Result<Option<ElementRef<'a>>, QueryError> {
    reference(references, end, property(end, name)?, end.properties().get(name).unwrap_or(&Value::Null))
}

fn invalid_input<T>(result: &Result<T, QueryError>) -> Result<bool, QueryError> {
    match result {
        Err(error @ QueryError::Invalid { .. }) => Err(error.clone()),
        _ => Ok(true),
    }
}

fn role_and_part_with_port(references: &ReferenceIndex<'_>, end: ElementRef<'_>) -> Result<bool, QueryError> {
    let role = end_reference(references, end, "role");
    let part = end_reference(references, end, "partWithPort");
    let present = !end.properties().get("partWithPort").unwrap_or(&Value::Null).is_null();
    let port = role.as_ref().map(|role| role.is_some_and(|role| role.conforms_to("Port"))).map_err(Clone::clone);
    let members = match &part {
        Ok(Some(part)) => (|| {
            let part_type = end_reference(references, *part, "type")?
                .ok_or_else(|| QueryError::unsupported(part.id(), "partWithPort Type is not materialized"))?;
            if !part_type.conforms_to("Namespace") {
                return Err(QueryError::invalid(part.id(), "partWithPort Type must conform to Namespace"));
            }
            classifiers::query(references, part_type.id()).map(|namespace| namespace.members)
        })(),
        Ok(None) => Ok(Vec::new()),
        Err(error) => Err(error.clone()),
    };
    let membership = match (&members, &role) {
        (Err(error @ QueryError::Invalid { .. }), _) | (_, Err(error @ QueryError::Invalid { .. })) => Err(error.clone()),
        (Err(error), _) | (_, Err(error)) => Err(error.clone()),
        (Ok(members), Ok(role)) => Ok(role.is_some_and(|role| members.iter().any(|member| member.id() == role.id()))),
    };
    feature_compatibility::require_all([
        invalid_input(&role), invalid_input(&part),
        feature_compatibility::accept_any([
            Ok(!present), feature_compatibility::require_all([port, membership]),
        ]),
    ])
}

fn part_with_port_empty(references: &ReferenceIndex<'_>, end: ElementRef<'_>) -> Result<bool, QueryError> {
    let role = end_reference(references, end, "role");
    let part = end_reference(references, end, "partWithPort");
    let port = role.as_ref().map(|role| role.is_some_and(|role| role.conforms_to("Port"))).map_err(Clone::clone);
    let same_owner = (|| {
        let connector = references.model().owner(end)
            .map_err(|error| QueryError::invalid(end.id(), error))?
            .ok_or_else(|| QueryError::invalid(end.id(), "ConnectorEnd requires a canonical owning Connector"))?;
        if let Some(stored) = end.properties().get("connector") {
            let descriptor = schema::property_by_id(schema::XmiId::new(schema::Document::Uml, "A_end_connector-connector"))
                .ok_or_else(|| QueryError::unsupported(end.id(), "missing connector inverse contract"))?;
            invalid_input(&check(references, end, descriptor, stored))?;
            if let Ok(id) = super::references::reference_id(stored) {
                if id != Some(connector.id()) {
                    return Err(QueryError::invalid(end.id(), "connector inverse contradicts canonical ownership"));
                }
            }
        }
        if connector.metaclass().is_none() {
            return Err(QueryError::unsupported(end.id(), "ConnectorEnd owner metaclass is unresolved"));
        }
        if !connector.conforms_to("Connector") {
            return Err(QueryError::invalid(end.id(), "ConnectorEnd owner must conform to Connector"));
        }
        if !classifiers::contributes_to_owned_union(connector, end, "Connector-end")? {
            return Err(QueryError::invalid(end.id(), "ConnectorEnd must belong to Connector::end"));
        }
        let connector_owner = references.model().owner(connector)
            .map_err(|error| QueryError::invalid(connector.id(), error))?;
        let role = role.as_ref().map_err(Clone::clone)?.as_ref()
            .ok_or_else(|| QueryError::invalid(end.id(), "ConnectorEnd::role is required"))?;
        let role_owner = references.model().owner(*role)
            .map_err(|error| QueryError::invalid(role.id(), error))?;
        Ok(role_owner.map(|owner| owner.id()) == connector_owner.map(|owner| owner.id()))
    })();
    let empty = end.properties().get("partWithPort").is_none_or(Value::is_null);
    feature_compatibility::require_all([
        invalid_input(&role), invalid_input(&part),
        feature_compatibility::accept_any([
            feature_compatibility::require_all([port, same_owner]).map(|antecedent| !antecedent),
            Ok(empty),
        ]),
    ])
}

fn self_part_with_port(references: &ReferenceIndex<'_>, end: ElementRef<'_>) -> Result<bool, QueryError> {
    end_reference(references, end, "partWithPort")
        .map(|part| part.is_none_or(|part| !part.conforms_to("Port")))
}

pub fn validate(references: &ReferenceIndex<'_>) -> Vec<StructuralDiagnostic> {
    let mut issues = Vec::new();
    for connector in references.model().elements().filter(|element| element.conforms_to("Connector")) {
        for (rule, error_code, unknown_code, result) in [
            ("Connector-types", "UML_CONNECTOR_TYPES", "MDE_CONNECTOR_TYPES_UNVERIFIED", types_rule(references, connector)),
            ("Connector-roles", "UML_CONNECTOR_ROLES", "MDE_CONNECTOR_ROLES_UNVERIFIED", roles_rule(references, connector)),
        ] {
            let (severity, code, message) = match result {
                Ok(true) => continue,
                Ok(false) => (StructuralSeverity::Error, error_code, "normative constraint is violated".to_owned()),
                Err(QueryError::Invalid { message, .. }) => (StructuralSeverity::Error, error_code, message),
                Err(QueryError::Unsupported { message, .. }) => (StructuralSeverity::Warning, unknown_code, message),
            };
            issues.push(StructuralDiagnostic { severity, code, subject_id:connector.id().into(), message:format!("{rule}: {message}") });
        }
    }
    for end in references.model().elements().filter(|element| element.conforms_to("ConnectorEnd")) {
        for (rule, error_code, unknown_code, result) in [
            ("ConnectorEnd-role_and_part_with_port", "UML_CONNECTOR_END_ROLE_AND_PART_WITH_PORT", "MDE_CONNECTOR_END_ROLE_AND_PART_WITH_PORT_UNVERIFIED", role_and_part_with_port(references, end)),
            ("ConnectorEnd-part_with_port_empty", "UML_CONNECTOR_END_PART_WITH_PORT_EMPTY", "MDE_CONNECTOR_END_PART_WITH_PORT_EMPTY_UNVERIFIED", part_with_port_empty(references, end)),
            ("ConnectorEnd-self_part_with_port", "UML_CONNECTOR_END_SELF_PART_WITH_PORT", "MDE_CONNECTOR_END_SELF_PART_WITH_PORT_UNVERIFIED", self_part_with_port(references, end)),
        ] {
            let (severity, code, message) = match result {
                Ok(true) => continue,
                Ok(false) => (StructuralSeverity::Error, error_code, "normative constraint is violated".to_owned()),
                Err(QueryError::Invalid { message, .. }) => (StructuralSeverity::Error, error_code, message),
                Err(QueryError::Unsupported { message, .. }) => (StructuralSeverity::Warning, unknown_code, message),
            };
            issues.push(StructuralDiagnostic { severity, code, subject_id:end.id().into(), message:format!("{rule}: {message}") });
        }
    }
    issues
}
