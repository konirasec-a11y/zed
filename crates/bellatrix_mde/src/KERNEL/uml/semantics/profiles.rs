use super::{
    classifiers::{self, QueryError},
    feature_compatibility,
    property_contracts::{self, PropertyOutcome},
    references::ReferenceIndex,
    root::ElementRef,
    schema,
    types::{Integer, UnlimitedNatural},
    value_specifications::{self, EvaluationError},
};
use crate::{StructuralDiagnostic, StructuralSeverity};

fn query_value<T>(result: Result<T, EvaluationError>) -> Result<T, QueryError> {
    result.map_err(|error| match error {
        EvaluationError::Invalid {
            subject_id,
            message,
        } => QueryError::invalid(&subject_id, message),
        EvaluationError::Unsupported {
            subject_id,
            message,
        } => QueryError::unsupported(&subject_id, message),
    })
}

fn end_multiplicity(
    references: &ReferenceIndex<'_>,
    end: ElementRef<'_>,
) -> Result<bool, QueryError> {
    let interval = query_value(value_specifications::multiplicity(references, end));
    let valid = match interval {
        Err(error @ QueryError::Invalid { .. }) => Err(error),
        _ => Ok(true),
    };
    // The known bound can disprove the conjunction even when its opposite is
    // opaque. The interval guard still retains malformed bound values.
    feature_compatibility::require_all([
        valid,
        query_value(value_specifications::lower_bound(references, end))
            .map(|lower| lower == Integer::from(0) || lower == Integer::from(1)),
        query_value(value_specifications::upper_bound(references, end))
            .map(|upper| upper == UnlimitedNatural::from(1)),
    ])
}

fn end_aggregation(end: ElementRef<'_>) -> Result<bool, QueryError> {
    schema::read_effective_scalar(end, "aggregation")
        .map(|value| value.as_str() == Some("composite"))
        .map_err(|error| QueryError::invalid(end.id(), error))
}

fn property_check(
    references: &ReferenceIndex<'_>,
    element: ElementRef<'_>,
    name: &str,
) -> Result<bool, QueryError> {
    let descriptor = schema::uml_property(
        element
            .metaclass()
            .ok_or_else(|| QueryError::unsupported(element.id(), "unresolved metaclass"))?,
        name,
    )
    .map_err(|error| QueryError::unsupported(element.id(), error.to_string()))?
    .ok_or_else(|| QueryError::unsupported(element.id(), format!("missing {name} contract")))?;
    let value = element.properties().get(name).ok_or_else(|| {
        QueryError::unsupported(element.id(), format!("{name} is not materialized"))
    })?;
    match property_contracts::check_value(references, element, descriptor, value) {
        PropertyOutcome::Satisfied => Ok(true),
        PropertyOutcome::Violation(message) => Err(QueryError::invalid(element.id(), message)),
        PropertyOutcome::Unverified(message) => Err(QueryError::unsupported(element.id(), message)),
    }
}

fn binary(references: &ReferenceIndex<'_>, extension: ElementRef<'_>) -> Result<bool, QueryError> {
    let contract = property_check(references, extension, "memberEnd");
    if let Err(error @ QueryError::Invalid { .. }) = contract {
        return Err(error);
    }
    match extension
        .properties()
        .get("memberEnd")
        .and_then(serde_json::Value::as_array)
    {
        Some(ends) => Ok(ends.len() == 2),
        None => contract,
    }
}

fn owned_end_projection<'a>(
    references: &ReferenceIndex<'a>,
    extension: ElementRef<'a>,
) -> (Option<ElementRef<'a>>, Result<bool, QueryError>) {
    let contract = property_check(references, extension, "ownedEnd");
    let mut checks = vec![contract.clone()];
    let children = references.explicit_owned_elements(extension.id());
    match children {
        Ok(children) => {
            let known = children
                .iter()
                .filter(|child| child.conforms_to("ExtensionEnd"))
                .count();
            if known > 1 {
                checks.push(Err(QueryError::invalid(
                    extension.id(),
                    "Extension::ownedEnd permits exactly one canonical ExtensionEnd",
                )));
            } else if children.iter().any(|child| child.metaclass().is_none()) {
                checks.push(Err(QueryError::unsupported(
                    extension.id(),
                    "unknown owned metaclass may contribute to Extension::ownedEnd",
                )));
            }
        }
        Err(error) => checks.push(Err(QueryError::invalid(extension.id(), error))),
    }
    let element = if matches!(contract, Ok(true)) {
        let resolved = (|| {
            let value = extension.properties().get("ownedEnd").ok_or_else(|| {
                QueryError::unsupported(extension.id(), "ownedEnd is not materialized")
            })?;
            let id = super::references::reference_id(value)
                .map_err(|error| QueryError::invalid(extension.id(), error))?
                .ok_or_else(|| QueryError::invalid(extension.id(), "ownedEnd is required"))?;
            references
                .element(id)
                .map_err(|error| QueryError::invalid(extension.id(), error))
        })();
        match resolved {
            Ok(element) => Some(element),
            Err(error) => {
                checks.push(Err(error));
                None
            }
        }
    } else {
        None
    };
    (element, feature_compatibility::require_all(checks))
}

/// UML Extension::isRequired, independent of stored isRequired/metaclass caches.
pub fn is_required(
    references: &ReferenceIndex<'_>,
    extension: ElementRef<'_>,
) -> Result<bool, QueryError> {
    if extension.metaclass().is_none() {
        return Err(QueryError::unsupported(
            extension.id(),
            "Extension metaclass is unresolved",
        ));
    }
    if !extension.conforms_to("Extension") {
        return Err(QueryError::invalid(
            extension.id(),
            "isRequired requires an Extension",
        ));
    }
    let (owned, contract) = owned_end_projection(references, extension);
    let lower = match owned {
        Some(owned) => query_value(value_specifications::lower_bound(references, owned))
            .map(|lower| lower == Integer::from(1)),
        None => Err(QueryError::unsupported(
            extension.id(),
            "ownedEnd identity is unresolved",
        )),
    };
    // A false derived Boolean is not permission to ignore an unresolved required
    // reference contract, unlike a false operand of a normative conjunction.
    feature_compatibility::require_all([
        contract,
        lower.as_ref().map(|_| true).map_err(Clone::clone),
    ])?;
    lower
}

fn attribute_associations_satisfy(
    references: &ReferenceIndex<'_>,
    attribute: ElementRef<'_>,
    predicate: impl Fn(ElementRef<'_>) -> Result<bool, QueryError>,
) -> Result<bool, QueryError> {
    let associations = references
        .association_membership_candidates(attribute.id())
        .map_err(|error| QueryError::invalid(attribute.id(), error))?;
    let mut checks = Vec::new();
    if associations.len() > 1 {
        checks.push(Err(QueryError::invalid(
            attribute.id(),
            "Property::association permits at most one Association",
        )));
    }
    for association in &associations {
        checks.push(predicate(*association));
    }
    // The reverse index contains only explicit local identities. Missing or
    // external member lists cannot establish that no further Association exists.
    for association in references
        .model()
        .elements()
        .filter(|element| element.conforms_to("Association"))
    {
        if associations
            .iter()
            .any(|known| known.id() == association.id())
        {
            continue;
        }
        let members = association.properties().get("memberEnd");
        if members.is_some_and(|value| {
            super::references::reference_id(value).ok().flatten() == Some(attribute.id())
        }) {
            checks.push(property_check(references, association, "memberEnd"));
        } else if members
            .and_then(serde_json::Value::as_array)
            .is_none_or(|members| {
                members
                    .iter()
                    .any(|value| super::references::reference_id(value).is_err())
            })
        {
            checks.push(Err(QueryError::unsupported(
                attribute.id(),
                "unresolved Association member lists prevent complete association membership",
            )));
        }
    }
    if let Some(stored) = attribute.properties().get("association") {
        checks.push(property_check(references, attribute, "association"));
        if let Ok(id) = super::references::reference_id(stored) {
            if let Some(known) = associations.first() {
                if id != Some(known.id()) {
                    checks.push(Err(QueryError::invalid(
                        attribute.id(),
                        "association inverse contradicts canonical memberEnd membership",
                    )));
                }
            } else if let Some(id) = id {
                match references.element(id) {
                    Ok(association) if association.conforms_to("Association") => {
                        let members = association.properties().get("memberEnd");
                        checks.push(match members {
                            Some(_) => property_check(references, association, "memberEnd"),
                            None => Err(QueryError::unsupported(
                                attribute.id(),
                                "Association::memberEnd is not materialized",
                            )),
                        });
                        if members
                            .and_then(serde_json::Value::as_array)
                            .is_some_and(|members| {
                                members
                                    .iter()
                                    .all(|value| super::references::reference_id(value).is_ok())
                            })
                        {
                            checks.push(Err(QueryError::invalid(
                                attribute.id(),
                                "association inverse has no corresponding memberEnd",
                            )));
                        } else {
                            checks.push(Err(QueryError::unsupported(
                                attribute.id(),
                                "association inverse cannot establish canonical membership",
                            )));
                        }
                    }
                    Ok(_) => {}
                    Err(error) => checks.push(Err(QueryError::invalid(attribute.id(), error))),
                }
            }
        }
    }
    feature_compatibility::require_all(checks)
}

fn stereotype_attributes_satisfy(
    references: &ReferenceIndex<'_>,
    stereotype: ElementRef<'_>,
    predicate: impl Fn(ElementRef<'_>) -> Result<bool, QueryError>,
) -> Result<bool, QueryError> {
    let mut checks = Vec::new();
    // Inspect only direct attribute roles; inherited namespace members are not
    // Stereotype::ownedAttribute and must not create this obligation.
    for descriptor in classifiers::properties(stereotype)? {
        if descriptor.aggregation != schema::Aggregation::Composite || descriptor.is_derived {
            continue;
        }
        if classifiers::contributes(descriptor, "Classifier-attribute")
            .map_err(|error| QueryError::unsupported(stereotype.id(), error))?
            && stereotype.properties().contains_key(descriptor.name)
        {
            checks.push(property_check(references, stereotype, descriptor.name));
        }
    }
    for child in references
        .explicit_owned_elements(stereotype.id())
        .map_err(|error| QueryError::invalid(stereotype.id(), error))?
    {
        if child.metaclass().is_none() {
            checks.push(Err(QueryError::unsupported(
                child.id(),
                "unknown owned metaclass may be an attribute",
            )));
        } else if child.conforms_to("Property") {
            match classifiers::is_owned_attribute(stereotype, child) {
                Ok(true) => checks.push(predicate(child)),
                Ok(false) => {}
                Err(error) => checks.push(Err(error)),
            }
        }
    }
    feature_compatibility::require_all(checks)
}

fn opposite_cache_check(
    references: &ReferenceIndex<'_>,
    attribute: ElementRef<'_>,
) -> Result<bool, QueryError> {
    let Some(value) = attribute.properties().get("opposite") else {
        return Ok(true);
    };
    let descriptor = schema::uml_property("Property", "opposite")
        .map_err(|error| QueryError::unsupported(attribute.id(), error.to_string()))?
        .ok_or_else(|| QueryError::unsupported(attribute.id(), "missing opposite contract"))?;
    // Validate the reference envelope only. The cache is never evidence for
    // the derived identity, which is checked against memberEnd separately.
    let descriptor = schema::PropertyDescriptor {
        is_derived: false,
        ..*descriptor
    };
    match property_contracts::check_value(references, attribute, &descriptor, value) {
        PropertyOutcome::Satisfied => Ok(true),
        PropertyOutcome::Violation(message) => Err(QueryError::invalid(attribute.id(), message)),
        PropertyOutcome::Unverified(message) => {
            Err(QueryError::unsupported(attribute.id(), message))
        }
    }
}

// This implication is about the attribute's Type, not the visual target of
// an Association edge. The opposite comes only from ordered member identities.
fn association_end_ownership(
    references: &ReferenceIndex<'_>,
    attribute: ElementRef<'_>,
    association: ElementRef<'_>,
) -> Result<bool, QueryError> {
    let members = property_check(references, association, "memberEnd");
    let typed = if attribute
        .properties()
        .get("type")
        .is_none_or(serde_json::Value::is_null)
    {
        Ok(false)
    } else {
        feature_compatibility::resolve_feature_type(references, attribute)
            .map(|target| target.conforms_to("Stereotype"))
    };
    // Malformed known operands remain invalid even if an antecedent is false.
    for evidence in [&members, &typed] {
        if let Err(error @ QueryError::Invalid { .. }) = evidence {
            return Err(error.clone());
        }
    }
    let owner = (|| {
        if attribute.properties().contains_key("opposite") {
            let contract = opposite_cache_check(references, attribute);
            if let Err(error @ QueryError::Invalid { .. }) = contract {
                return Err(error);
            }
        }
        members.clone()?;
        let opposite = super::associations::opposite(references, attribute.id())
            .map_err(|error| QueryError::invalid(attribute.id(), error))?;
        let Some(opposite) = opposite else {
            return Ok(false);
        };
        let opposite = ElementRef::Object(opposite);
        let cache = if let Some(stored) = attribute.properties().get("opposite") {
            let contract = opposite_cache_check(references, attribute);
            if let Ok(id) = super::references::reference_id(stored) {
                if id != Some(opposite.id()) {
                    return Err(QueryError::invalid(
                        attribute.id(),
                        "opposite cache contradicts canonical memberEnd",
                    ));
                }
            }
            contract
        } else {
            Ok(true)
        };
        let owner = references
            .model()
            .owner(opposite)
            .map_err(|error| QueryError::invalid(opposite.id(), error))?;
        feature_compatibility::require_all([
            cache,
            Ok(owner.is_some_and(|owner| owner.id() == association.id())),
        ])
    })();
    // not(Type is Stereotype) implies owner equality; a true consequent wins
    // unresolved Type, but malformed evidence always wins both.
    feature_compatibility::accept_any([Ok(association.conforms_to("Extension")), typed, owner])
}

// Profile imports have a narrower role than ordinary ElementImports. Only
// explicit metaclassReference membership selects that role in this adapter.
fn profile_metaclass_reference(
    references: &ReferenceIndex<'_>,
    profile: ElementRef<'_>,
) -> Result<bool, QueryError> {
    let mut checks = Vec::new();
    let mut imported = Vec::new();
    let owned = references
        .explicit_owned_elements(profile.id())
        .map_err(|error| QueryError::invalid(profile.id(), error))?;
    if let Some(value) = profile.properties().get("metaclassReference") {
        checks.push(property_check(references, profile, "metaclassReference"));
        if let Some(values) = value.as_array() {
            for value in values {
                let Ok(Some(id)) = super::references::reference_id(value) else {
                    continue;
                };
                let Ok(import) = references.element(id) else {
                    continue;
                };
                if !import.conforms_to("ElementImport") {
                    continue;
                }
                let ElementRef::Relationship(record) = import else {
                    checks.push(Err(QueryError::unsupported(
                        id,
                        "metaclassReference requires a canonical ElementImport relationship",
                    )));
                    continue;
                };
                // The import endpoint is semantic (not a diagram endpoint).
                let target = match references.element(&record.to) {
                    Ok(target) => target,
                    Err(error) => {
                        checks.push(Err(QueryError::invalid(id, error)));
                        continue;
                    }
                };
                for (name, expected) in [
                    ("importedElement", target.id()),
                    ("importingNamespace", profile.id()),
                ] {
                    if let Some(stored) = import.properties().get(name) {
                        checks.push(property_check(references, import, name));
                        if let Ok(actual) = super::references::reference_id(stored) {
                            if actual != Some(expected) {
                                checks.push(Err(QueryError::invalid(
                                    id,
                                    format!("{name} contradicts canonical import endpoint"),
                                )));
                            }
                        }
                    }
                }
                if target.metaclass().is_none() {
                    checks.push(Err(QueryError::unsupported(
                        id,
                        "imported metaclass is unresolved",
                    )));
                } else {
                    checks.push(
                        super::namespace_imports::validate_import_record(references, import)
                            .map(|_| true),
                    );
                    if target.conforms_to("Classifier") {
                        imported.push(target);
                    }
                }
            }
        }
    } else if owned
        .iter()
        .any(|child| child.metaclass().is_none() || child.conforms_to("ElementImport"))
        || references
            .model()
            .relationship_records()
            .any(|record| record.kind == "uml.element_import" && record.from == profile.id())
    {
        checks.push(Err(QueryError::unsupported(profile.id(), "ElementImport records do not distinguish metaclassReference without explicit role membership")));
    }
    if profile.properties().contains_key("packagedElement") {
        checks.push(property_check(references, profile, "packagedElement"));
    }

    for child in owned {
        if child.metaclass().is_none() && !imported.is_empty() {
            checks.push(Err(QueryError::unsupported(
                child.id(),
                "unknown owned element may be a Classifier",
            )));
        } else if child.conforms_to("Classifier") {
            let membership =
                classifiers::contributes_to_owned_union(profile, child, "Package-packagedElement");
            match membership {
                Ok(true) => {
                    for imported in &imported {
                        // Both published conjuncts: neither the imported
                        // Classifier nor a local packaged Classifier may have
                        // the other among its allParents().
                        checks.push(
                            super::classifier_hierarchy::has_ancestor(
                                references,
                                imported.id(),
                                child.id(),
                            )
                            .map(|found| !found),
                        );
                        checks.push(
                            super::classifier_hierarchy::has_ancestor(
                                references,
                                child.id(),
                                imported.id(),
                            )
                            .map(|found| !found),
                        );
                    }
                }
                Ok(false) => {}
                Err(error) => checks.push(Err(error)),
            }
        }
    }
    feature_compatibility::require_all(checks)
}

// The second conjunct quantifies over all Classifiers, not just direct
// children of this Stereotype. Canonical paths can decide it despite unresolved
// neighboring branches, but cannot excuse malformed ancestry or cycles.
fn stereotype_specializations(references: &ReferenceIndex<'_>) -> Result<bool, QueryError> {
    let mut checks = Vec::new();
    for classifier in references.model().elements() {
        if classifier.metaclass().is_none() {
            checks.push(Err(QueryError::unsupported(
                classifier.id(),
                "unknown metaclass may be a Classifier",
            )));
        } else if classifier.conforms_to("Classifier") && !classifier.conforms_to("Stereotype") {
            checks.push(
                super::classifier_hierarchy::has_ancestor_matching(
                    references,
                    classifier.id(),
                    |parent| parent.conforms_to("Stereotype"),
                )
                .map(|found| !found),
            );
        }
    }
    feature_compatibility::require_all(checks)
}

pub(crate) fn validate(references: &ReferenceIndex<'_>) -> Vec<StructuralDiagnostic> {
    let mut issues = Vec::new();
    let stereotypes = references
        .model()
        .elements()
        .filter(|element| element.conforms_to("Stereotype"))
        .collect::<Vec<_>>();
    if !stereotypes.is_empty() {
        let specializations = stereotype_specializations(references);
        for stereotype in stereotypes {
            // Only the first conjunct uses self. A different Stereotype's
            // bad parents must not invalidate this one's allParents().
            let parents = super::classifier_hierarchy::has_ancestor_matching(
                references,
                stereotype.id(),
                |parent| !parent.conforms_to("Stereotype"),
            )
            .map(|found| !found);
            let result = feature_compatibility::require_all([parents, specializations.clone()]);
            let (severity, code, message) = match &result {
                Ok(true) => continue,
                Ok(false) => (
                    StructuralSeverity::Error,
                    "UML_STEREOTYPE_GENERALIZATION",
                    "Stereotypes may only generalize or specialize Stereotypes".to_owned(),
                ),
                Err(QueryError::Invalid { message, .. }) => (
                    StructuralSeverity::Error,
                    "UML_STEREOTYPE_GENERALIZATION",
                    message.clone(),
                ),
                Err(QueryError::Unsupported { message, .. }) => (
                    StructuralSeverity::Warning,
                    "MDE_STEREOTYPE_GENERALIZATION_UNVERIFIED",
                    message.clone(),
                ),
            };
            issues.push(StructuralDiagnostic {
                severity,
                code,
                subject_id: stereotype.id().into(),
                message: format!("Stereotype-generalize: {message}"),
            });
        }
    }
    for end in references
        .model()
        .elements()
        .filter(|element| element.conforms_to("ExtensionEnd"))
    {
        for (rule, error_code, unknown_code, result) in [
            (
                "ExtensionEnd-multiplicity",
                "UML_EXTENSION_END_MULTIPLICITY",
                "MDE_EXTENSION_END_MULTIPLICITY_UNVERIFIED",
                end_multiplicity(references, end),
            ),
            (
                "ExtensionEnd-aggregation",
                "UML_EXTENSION_END_AGGREGATION",
                "MDE_EXTENSION_END_AGGREGATION_UNVERIFIED",
                end_aggregation(end),
            ),
        ] {
            let (severity, code, message) = match result {
                Ok(true) => continue,
                Ok(false) => (
                    StructuralSeverity::Error,
                    error_code,
                    "normative constraint is violated".to_owned(),
                ),
                Err(QueryError::Invalid { message, .. }) => {
                    (StructuralSeverity::Error, error_code, message)
                }
                Err(QueryError::Unsupported { message, .. }) => {
                    (StructuralSeverity::Warning, unknown_code, message)
                }
            };
            issues.push(StructuralDiagnostic {
                severity,
                code,
                subject_id: end.id().into(),
                message: format!("{rule}: {message}"),
            });
        }
    }
    for extension in references
        .model()
        .elements()
        .filter(|element| element.conforms_to("Extension"))
    {
        for (rule, error_code, unknown_code, result) in [
            (
                "Extension-is_binary",
                "UML_EXTENSION_IS_BINARY",
                "MDE_EXTENSION_IS_BINARY_UNVERIFIED",
                binary(references, extension),
            ),
            (
                "Extension-ownedEnd",
                "UML_EXTENSION_OWNED_END",
                "MDE_EXTENSION_OWNED_END_UNVERIFIED",
                owned_end_projection(references, extension).1,
            ),
            (
                "Extension-isRequired.1-spec",
                "UML_EXTENSION_IS_REQUIRED",
                "MDE_EXTENSION_IS_REQUIRED_UNVERIFIED",
                is_required(references, extension).map(|_| true),
            ),
        ] {
            let (severity, code, message) = match result {
                Ok(true) => continue,
                Ok(false) => (
                    StructuralSeverity::Error,
                    error_code,
                    "normative constraint is violated".to_owned(),
                ),
                Err(QueryError::Invalid { message, .. }) => {
                    (StructuralSeverity::Error, error_code, message)
                }
                Err(QueryError::Unsupported { message, .. }) => {
                    (StructuralSeverity::Warning, unknown_code, message)
                }
            };
            issues.push(StructuralDiagnostic {
                severity,
                code,
                subject_id: extension.id().into(),
                message: format!("{rule}: {message}"),
            });
        }
    }
    for stereotype in references
        .model()
        .elements()
        .filter(|element| element.conforms_to("Stereotype"))
    {
        let outcome = stereotype_attributes_satisfy(references, stereotype, |attribute| {
            attribute_associations_satisfy(references, attribute, |association| {
                binary(references, association)
            })
        });
        let (severity, code, message) = match outcome {
            Ok(true) => continue,
            Ok(false) => (
                StructuralSeverity::Error,
                "UML_STEREOTYPE_BINARY_ASSOCIATIONS",
                "owned attributes may participate only in binary Associations".to_owned(),
            ),
            Err(QueryError::Invalid { message, .. }) => (
                StructuralSeverity::Error,
                "UML_STEREOTYPE_BINARY_ASSOCIATIONS",
                message,
            ),
            Err(QueryError::Unsupported { message, .. }) => (
                StructuralSeverity::Warning,
                "MDE_STEREOTYPE_BINARY_ASSOCIATIONS_UNVERIFIED",
                message,
            ),
        };
        issues.push(StructuralDiagnostic {
            severity,
            code,
            subject_id: stereotype.id().into(),
            message: format!("Stereotype-binaryAssociationsOnly: {message}"),
        });
    }
    for stereotype in references
        .model()
        .elements()
        .filter(|element| element.conforms_to("Stereotype"))
    {
        let outcome = stereotype_attributes_satisfy(references, stereotype, |attribute| {
            attribute_associations_satisfy(references, attribute, |association| {
                association_end_ownership(references, attribute, association)
            })
        });
        let (severity, code, message) = match outcome {
            Ok(true) => continue,
            Ok(false) => (
                StructuralSeverity::Error,
                "UML_STEREOTYPE_ASSOCIATION_END_OWNERSHIP",
                "opposite end must be owned by its Association".to_owned(),
            ),
            Err(QueryError::Invalid { message, .. }) => (
                StructuralSeverity::Error,
                "UML_STEREOTYPE_ASSOCIATION_END_OWNERSHIP",
                message,
            ),
            Err(QueryError::Unsupported { message, .. }) => (
                StructuralSeverity::Warning,
                "MDE_STEREOTYPE_ASSOCIATION_END_OWNERSHIP_UNVERIFIED",
                message,
            ),
        };
        issues.push(StructuralDiagnostic {
            severity,
            code,
            subject_id: stereotype.id().into(),
            message: format!("Stereotype-associationEndOwnership: {message}"),
        });
    }
    // UML's Stereotype-base_property_upper_bound supplies normative prose,
    // not an OCL body: the upper bound of base-properties is exactly one.
    for stereotype in references
        .model()
        .elements()
        .filter(|element| element.conforms_to("Stereotype"))
    {
        let outcome = stereotype_attributes_satisfy(references, stereotype, |attribute| {
            attribute_associations_satisfy(references, attribute, |association| {
                if !association.conforms_to("Extension") {
                    return Ok(true);
                }
                // Base-property identity comes from canonical ownership and
                // Extension membership, never the display name's base_ prefix.
                feature_compatibility::require_all([
                    property_check(references, association, "memberEnd"),
                    query_value(value_specifications::upper_bound(references, attribute))
                        .map(|upper| upper == UnlimitedNatural::from(1)),
                ])
            })
        });
        let (severity, code, message) = match outcome {
            Ok(true) => continue,
            Ok(false) => (
                StructuralSeverity::Error,
                "UML_STEREOTYPE_BASE_PROPERTY_UPPER_BOUND",
                "base-properties must have upper bound exactly 1".to_owned(),
            ),
            Err(QueryError::Invalid { message, .. }) => (
                StructuralSeverity::Error,
                "UML_STEREOTYPE_BASE_PROPERTY_UPPER_BOUND",
                message,
            ),
            Err(QueryError::Unsupported { message, .. }) => (
                StructuralSeverity::Warning,
                "MDE_STEREOTYPE_BASE_PROPERTY_UPPER_BOUND_UNVERIFIED",
                message,
            ),
        };
        issues.push(StructuralDiagnostic {
            severity,
            code,
            subject_id: stereotype.id().into(),
            message: format!("Stereotype-base_property_upper_bound: {message}"),
        });
    }
    for profile in references
        .model()
        .elements()
        .filter(|element| element.conforms_to("Profile"))
    {
        let (severity, code, message) = match profile_metaclass_reference(references, profile) {
            Ok(true) => continue,
            Ok(false) => (
                StructuralSeverity::Error,
                "UML_PROFILE_METACLASS_REFERENCE_SPECIALIZATION",
                "referenced metaclasses may not be generalized or specialized in the Profile"
                    .to_owned(),
            ),
            Err(QueryError::Invalid { message, .. }) => (
                StructuralSeverity::Error,
                "UML_PROFILE_METACLASS_REFERENCE_SPECIALIZATION",
                message,
            ),
            Err(QueryError::Unsupported { message, .. }) => (
                StructuralSeverity::Warning,
                "MDE_PROFILE_METACLASS_REFERENCE_SPECIALIZATION_UNVERIFIED",
                message,
            ),
        };
        issues.push(StructuralDiagnostic {
            severity,
            code,
            subject_id: profile.id().into(),
            message: format!("Profile-metaclass_reference_not_specialized: {message}"),
        });
    }
    issues
}

#[cfg(test)]
mod base_property_gate_tests {
    use serde_json::json;

    #[test]
    fn base_property_upper_gate_rejects_zero_and_accepts_repair()
    -> Result<(), Box<dyn std::error::Error>> {
        let before: crate::Project = serde_json::from_value(json!({
            "id":"project", "name":"Base-property upper gate",
            "elements":[
                {"id":"profile","kind":"uml.profile","name":"Profile"},
                {"id":"class","kind":"uml.class","name":"Metaclass"},
                {"id":"stereotype","kind":"uml.stereotype","name":"Stereotype"},
                {"id":"extension","kind":"uml.extension","name":"Extension","properties":{"memberEnd":[{"$ref":"base"},{"$ref":"end"}],"ownedEnd":{"$ref":"end"}}},
                {"id":"base","kind":"uml.attribute","name":"base_Class","properties":{"type":{"$ref":"class"}}},
                {"id":"end","kind":"uml.extension_end","name":"extension_Stereotype","properties":{"type":{"$ref":"stereotype"},"aggregation":"composite"}}
            ],
            "relations":[
                {"id":"owns-stereotype","kind":"owns","from":"profile","to":"stereotype"},
                {"id":"owns-extension","kind":"owns","from":"profile","to":"extension"},
                {"id":"owns-base","kind":"owns","from":"stereotype","to":"base"},
                {"id":"owns-end","kind":"owns","from":"extension","to":"end"}
            ]
        }))?;
        let unchanged = crate::encode(&before, crate::Format::Json)?;
        let mut after = before.clone();
        after
            .elements
            .iter_mut()
            .find(|element| element.id == "base")
            .ok_or("base")?
            .properties
            .insert("multiplicity".into(), json!("0"));
        let error = super::super::root::validate_changes(&before, &after)
            .expect_err("base-properties require upper exactly one");
        assert!(
            error.contains("Stereotype-base_property_upper_bound"),
            "{error}"
        );
        assert_eq!(crate::encode(&before, crate::Format::Json)?, unchanged);
        super::super::root::validate_changes(&after, &before)?;
        Ok(())
    }
}
