//! UML 2.5.1 property contracts over the existing canonical snapshot.
//!
//! The generated PropertyDescriptor supplies type, bounds, order and uniqueness.
//! This module completes the reference/collection branch of Model::validate; it
//! neither stores a second model nor evaluates arbitrary OCL. Satisfied below
//! means that this property's representation contract was checked, not that all
//! constraints of its metaclass were evaluated.

use super::{
    associations,
    references::{ReferenceIndex, reference_id},
    operations,
    value_specifications::{self, LiteralValue},
    root::ElementRef,
    schema::{self, Aggregation, PropertyDescriptor, ScalarKind, UnsupportedContract},
    types::{Integer, MultiplicityRange, StandardPrimitiveType, UnlimitedNatural},
};
use crate::{StructuralDiagnostic, StructuralSeverity};
use serde_json::{Value, json};
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PropertyOutcome {
    Satisfied,
    Violation(String),
    Unverified(String),
}

impl PropertyOutcome {
    fn diagnostic(self, element: ElementRef<'_>, property: &str, code: &'static str)
        -> Option<StructuralDiagnostic>
    {
        let (severity, code, message) = match self {
            Self::Satisfied => return None,
            Self::Violation(message) => (StructuralSeverity::Error, code, message),
            Self::Unverified(message) => (
                StructuralSeverity::Warning, "MDE_PROPERTY_CONTRACT_UNVERIFIED", message,
            ),
        };
        Some(StructuralDiagnostic {
            severity,
            code,
            subject_id: element.id().into(),
            message: format!("{property}: {message}"),
        })
    }
}

/// Check one stored value, retaining the distinction between a violated contract
/// and a contract whose normative type or representation is not supported.
pub fn check_value(
    references: &ReferenceIndex<'_>,
    owner: ElementRef<'_>,
    property: &PropertyDescriptor,
    value: &Value,
) -> PropertyOutcome {
    use PropertyOutcome::{Satisfied, Unverified, Violation};
    if property.is_derived {
        return Unverified("a stored value does not establish its UML derivation".into());
    }
    if property.property_type.resolution != schema::ReferenceResolution::Exact {
        return Unverified("normative property Type is unresolved or ambiguous".into());
    }
    let range = match format!("{}..{}", property.lower, property.upper)
        .parse::<MultiplicityRange>()
    {
        Ok(range) => range,
        Err(error) => return Unverified(format!("unresolved normative multiplicity: {error}")),
    };
    let many = !matches!(property.upper, "0" | "1");
    let values: &[Value] = if many {
        match value.as_array() {
            Some(values) => values,
            None => return Violation("expected a collection, including [] for an empty collection".into()),
        }
    } else if value.is_null() {
        &[]
    } else {
        std::slice::from_ref(value)
    };
    let cardinality = match values.len().to_string().parse::<Integer>() {
        Ok(cardinality) => cardinality,
        Err(error) => return Unverified(error.to_string()),
    };
    if !range.includes_cardinality(&cardinality) {
        return Violation(format!(
            "requires {}..{} values; received {cardinality}", property.lower, property.upper,
        ));
    }
    // Scalar checking of each member is independent of the collection's bounds.
    // Do not alter the generated descriptor or replace ordered/bag collections
    // with sets: uniqueness is checked only when the descriptor requires it.
    let member = PropertyDescriptor { lower: "1", upper: "1", ..*property };
    let scalar_kind = match schema::scalar_kind(&member) {
        Ok(kind) => Some(kind),
        Err(UnsupportedContract::Reference) => None,
        Err(error) => return Unverified(error.to_string()),
    };
    let mut seen = HashSet::new();
    let mut unverified = None;
    // An unresolved member cannot hide a violation proved by another member.
    for (position, value) in values.iter().enumerate() {
        if value.is_null() {
            return Violation(format!("value[{position}] cannot be null"));
        }
        let key = if let Some(kind) = scalar_kind {
            match schema::validate_scalar(&member, value) {
                Ok(Ok(())) => {}
                Ok(Err(error)) => return Violation(format!("value[{position}]: {error}")),
                Err(error) => {
                    unverified.get_or_insert_with(|| error.to_string());
                    continue;
                }
            }
            scalar_identity(kind, value)
        } else {
            // Historical free-form declarations must remain editable as drafts,
            // but they are not resolved to Type identities by matching names.
            if schema::uses_legacy_text_adapter(&member, value) {
                unverified.get_or_insert_with(|| "text declaration is not a resolved UML reference".into());
                continue;
            }
            if value.as_object().is_some_and(|object| object.contains_key("href")) {
                unverified.get_or_insert_with(|| "external model reference has no loaded document resolver".into());
                continue;
            }
            let id = match reference_id(value) {
                Ok(Some(id)) => id,
                Ok(None) => return Violation(format!("reference[{position}] cannot be null")),
                Err(error) => return Violation(format!("reference[{position}]: {error}")),
            };
            let target = match references.element(id) {
                Ok(target) => target,
                Err(error) => return Violation(error),
            };
            match (
                property.property_type.target.and_then(schema::classifier),
                target.metaclass().and_then(schema::uml_classifier),
            ) {
                (Some(required), Some(actual)) => match schema::conforms_to(actual.id, required.id) {
                    Ok(true) => {}
                    Ok(false) => return Violation(format!("{id} must conform to UML {}", required.name)),
                    Err(error) => { unverified.get_or_insert_with(|| error.to_string()); }
                },
                (None, _) => {
                    unverified.get_or_insert_with(|| "normative reference Type is unresolved".into());
                }
                (_, None) => {
                    unverified.get_or_insert_with(|| format!("{id} has no supported UML metaclass"));
                }
            }
            if property.aggregation == Aggregation::Composite {
                match references.model().owner(target) {
                    Ok(Some(actual_owner)) if actual_owner.id() == owner.id() => {}
                    Ok(_) => return Violation(format!(
                        "composite target {id} must be owned by {} in canonical ownership", owner.id(),
                    )),
                    Err(error) => return Violation(error),
                }
            }
            id.to_owned()
        };
        if property.is_unique && !seen.insert(key) {
            return Violation(format!("duplicate value at index {position}; isUnique is true"));
        }
    }
    unverified.map_or(Satisfied, Unverified)
}

fn scalar_identity(kind: ScalarKind, value: &Value) -> String {
    let decimal = value.as_str().map(str::to_owned).unwrap_or_else(|| value.to_string());
    match kind {
        ScalarKind::Primitive(StandardPrimitiveType::Integer) => decimal.parse::<Integer>()
            .map(|value| value.to_string()).unwrap_or(decimal),
        ScalarKind::Primitive(StandardPrimitiveType::UnlimitedNatural) => decimal.parse::<UnlimitedNatural>()
            .map(|value| value.to_string()).unwrap_or(decimal),
        ScalarKind::Primitive(StandardPrimitiveType::Real) => value.as_f64()
            .map(|value| if value == 0.0 { "0".into() } else { value.to_string() })
            .unwrap_or_else(|| value.to_string()),
        _ => value.to_string(),
    }
}

// These roles already have a canonical binary storage adapter in root.rs.
// The returned values are borrowed-state projections, never persisted copies.
fn endpoint_value(element: ElementRef<'_>, property: &PropertyDescriptor) -> Option<Value> {
    let ElementRef::Relationship(relation) = element else { return None; };
    let target = match property.id.fragment {
        "Generalization-specific" | "ElementImport-importingNamespace"
        | "PackageImport-importingNamespace" | "InterfaceRealization-implementingClassifier" => &relation.from,
        "Generalization-general" | "ElementImport-importedElement"
        | "PackageImport-importedPackage" | "InterfaceRealization-contract" => &relation.to,
        _ => return None,
    };
    Some(if matches!(property.upper, "0" | "1") { json!({"$ref": target}) }
        else { json!([{"$ref": target}]) })
}

fn constraint_specification<'a>(
    references: &ReferenceIndex<'a>,
    constraint: ElementRef<'a>,
) -> Result<Option<ElementRef<'a>>, String> {
    let specifications = references.explicit_owned_elements(constraint.id())?
        .into_iter().filter(|child| child.conforms_to("ValueSpecification")).collect::<Vec<_>>();
    if specifications.is_empty() && !constraint.properties().contains_key("specification") {
        // The editor can retain an unfinished Constraint without inventing an
        // expression. An explicit null or malformed reference is still invalid.
        return Ok(None);
    }
    let [specification] = specifications.as_slice() else {
        return Err("Constraint::specification requires exactly one owned ValueSpecification".into());
    };
    if let Some(value) = constraint.properties().get("specification") {
        if reference_id(value)? != Some(specification.id()) {
            return Err("Constraint::specification must identify its canonical owned ValueSpecification".into());
        }
    }
    Ok(Some(*specification))
}

fn required_role<'a>(
    references: &ReferenceIndex<'a>,
    element: ElementRef<'a>,
    property: &PropertyDescriptor,
) -> Result<Option<Value>, String> {
    if let Some(value) = endpoint_value(element, property) { return Ok(Some(value)); }
    match property.id.fragment {
        "Dependency-client" | "Dependency-supplier" => {
            let result = if property.name == "client" {
                super::relationship_endpoints::clients(references, element.id())
            } else {
                super::relationship_endpoints::suppliers(references, element.id())
            };
            let values = match result {
                Ok(values) => values,
                Err(super::element_graph_queries::ElementGraphError::Unsupported { .. }) =>
                    return Ok(None),
                Err(error) => return Err(error.to_string()),
            };
            Ok(Some(Value::Array(values.into_iter()
                .map(|element| json!({"$ref": element.id()})).collect())))
        }
        "Constraint-specification" => constraint_specification(references, element)
            .map(|specification| specification.map(|specification| json!({"$ref": specification.id()}))),
        "Slot-owningInstance" => references.model().owner(element)
            .map(|owner| Some(owner.map_or(Value::Null, |owner| json!({"$ref": owner.id()})))),
        "Slot-definingFeature" | "Association-memberEnd" =>
            Err(format!("{} requires explicit canonical reference identities", property.id.fragment)),
        _ => Ok(None),
    }
}

/// Called by the existing ReferenceIndex/Model::validate path. Scalar values
/// already covered by schema::validate_element are not reported a second time.
pub(crate) fn validate(references: &ReferenceIndex<'_>) -> Vec<StructuralDiagnostic> {
    use PropertyOutcome::{Unverified, Violation};
    let mut issues = Vec::new();
    for element in references.model().elements() {
        let Some(metaclass) = element.metaclass() else {
            if element.kind().starts_with("uml.") {
                issues.extend(Unverified("unknown UML metaclass".into())
                    .diagnostic(element, element.kind(), "UML_METACLASS"));
            }
            continue;
        };
        let Some(classifier) = schema::uml_classifier(metaclass) else {
            issues.extend(Unverified("missing normative classifier".into())
                .diagnostic(element, metaclass, "UML_METACLASS"));
            continue;
        };
        if classifier.is_abstract {
            issues.extend(Violation("an abstract metaclass cannot be instantiated directly".into())
                .diagnostic(element, metaclass, "UML_ABSTRACT_METACLASS"));
        }
        if let Err(error) = references.element(element.id()) {
            issues.extend(Violation(error).diagnostic(element, metaclass, "UML_ELEMENT_IDENTITY"));
        }
        let inherited = match schema::inherited_properties(classifier.id) {
            Ok(properties) if properties.unresolved.is_empty() => properties,
            Ok(_) => {
                issues.extend(Unverified("unresolved normative inheritance".into())
                    .diagnostic(element, metaclass, "UML_PROPERTY_CONTRACT"));
                continue;
            }
            Err(error) => {
                issues.extend(Unverified(error.to_string())
                    .diagnostic(element, metaclass, "UML_PROPERTY_CONTRACT"));
                continue;
            }
        };
        for (name, value) in element.properties() {
            let property = match schema::uml_property(metaclass, name) {
                Ok(Some(property)) => property,
                Ok(None) => {
                    // Editor/target extension fields are retained. They are not
                    // silently promoted into normative UML properties.
                    issues.extend(Unverified("no normative property or registered extension contract".into())
                        .diagnostic(element, name, "UML_PROPERTY_CONTRACT"));
                    continue;
                }
                Err(error) => {
                    issues.extend(Unverified(error.to_string())
                        .diagnostic(element, name, "UML_PROPERTY_CONTRACT"));
                    continue;
                }
            };
            if !property.is_derived && schema::validate_scalar(property, value).is_ok() {
                continue;
            }
            let code = if name == "type" { "UML_TYPED_ELEMENT_TYPE" } else { "UML_PROPERTY_CONTRACT" };
            if endpoint_value(element, property).is_some_and(|canonical| canonical != *value) {
                issues.extend(Unverified("stored role differs from the canonical binary endpoint; a matching endpoint adapter is required".into())
                    .diagnostic(element, property.id.fragment, code));
            }
            issues.extend(check_value(references, element, property, value)
                .diagnostic(element, property.id.fragment, code));
        }
        validate_property_rules(references, element, &mut issues);
        validate_value_rules(references, element, &mut issues);
        for property in inherited.properties {
            if property.is_derived || element.properties().contains_key(property.name) {
                continue;
            }
            let default = match schema::effective_default(property) {
                Ok(default) => default,
                Err(error) => {
                    issues.extend(Unverified(error.to_string())
                        .diagnostic(element, property.id.fragment, "UML_PROPERTY_CONTRACT"));
                    continue;
                }
            };
            if let Some(default) = default {
                match default.scalar_json() {
                    Some(value) => issues.extend(check_value(references, element, property, &value)
                        .diagnostic(element, property.id.fragment, "UML_PROPERTY_CONTRACT")),
                    None => issues.extend(Unverified("normative default has no supported value adapter".into())
                        .diagnostic(element, property.id.fragment, "UML_PROPERTY_CONTRACT")),
                }
                continue;
            }
            let role = match required_role(references, element, property) {
                Ok(role) => role,
                Err(error) => {
                    issues.extend(Violation(error)
                        .diagnostic(element, property.id.fragment, "UML_PROPERTY_CONTRACT"));
                    continue;
                }
            };
            if let Some(value) = role {
                issues.extend(check_value(references, element, property, &value)
                    .diagnostic(element, property.id.fragment, "UML_PROPERTY_CONTRACT"));
            } else if property.lower != "0" {
                // Missing primitive values are unambiguously absent. Reference
                // roles may instead be projected from canonical ownership by a
                // specialized adapter; do not invent a second representation.
                let outcome = if schema::scalar_kind(property).is_ok() {
                    Violation("required value is absent and has no normative default".into())
                } else {
                    Unverified("required reference/collection needs its canonical role projection".into())
                };
                issues.extend(outcome.diagnostic(element, property.id.fragment, "UML_PROPERTY_CONTRACT"));
            }
        }
    }
    issues
}

fn validate_property_rules(
    references: &ReferenceIndex<'_>,
    element: ElementRef<'_>,
    issues: &mut Vec<StructuralDiagnostic>,
) {
    use PropertyOutcome::{Satisfied, Unverified, Violation};
    if !element.conforms_to("Property") { return; }
    let derived_union = schema::read_effective_boolean(element, "isDerivedUnion");
    for (property, rule, code) in [
        ("isReadOnly", "Property-derived_union_is_read_only", "UML_PROPERTY_DERIVED_UNION_READ_ONLY"),
        ("isDerived", "Property-derived_union_is_derived", "UML_PROPERTY_DERIVED_UNION_DERIVED"),
    ] {
        let outcome = match &derived_union {
            Ok(false) => Satisfied,
            Ok(true) => match schema::read_effective_boolean(element, property) {
                Ok(true) => Satisfied,
                Ok(false) => Violation(format!("isDerivedUnion requires {property} to be true")),
                Err(error) => Unverified(error),
            },
            Err(error) => Unverified(error.clone()),
        };
        issues.extend(outcome.diagnostic(element, rule, code));
    }
    let explicit_qualifiers = element.properties().get("qualifier")
        .and_then(Value::as_array).is_some_and(|values| !values.is_empty());
    let owned_qualifiers = match references.explicit_owned_elements(element.id()) {
        Ok(owned) => owned.into_iter().any(|child| child.conforms_to("Property")),
        Err(error) => {
            issues.extend(Violation(error).diagnostic(element,
                "Property-qualified_is_association_end", "UML_PROPERTY_QUALIFIED_ASSOCIATION_END"));
            return;
        }
    };
    if !explicit_qualifiers && !owned_qualifiers { return; }
    // association is the inverse of canonical memberEnd, never a stored copy.
    let outcome = match associations::association_of(references, element.id()) {
        Ok(Some(_)) => Satisfied,
        Ok(None) => Violation("a Property with qualifiers must be an Association memberEnd".into()),
        Err(error) => Violation(error),
    };
    issues.extend(outcome.diagnostic(element, "Property-qualified_is_association_end",
        "UML_PROPERTY_QUALIFIED_ASSOCIATION_END"));
}

// Constraints are evaluated structurally; their truth against the modeled
// application's instances is not confused with UML model well-formedness.
fn validate_value_rules<'a>(
    references: &ReferenceIndex<'a>,
    element: ElementRef<'a>,
    issues: &mut Vec<StructuralDiagnostic>,
) {
    use PropertyOutcome::{Unverified, Violation};
    if element.conforms_to("Constraint") {
        if element.properties().get("constrainedElement")
            .and_then(Value::as_array).is_some_and(|values| values.iter().any(|value|
                reference_id(value).ok().flatten() == Some(element.id())))
        {
            issues.extend(Violation("a Constraint must not constrain itself".into())
                .diagnostic(element, "Constraint-not_apply_to_self", "UML_CONSTRAINT_NOT_SELF"));
        }
        match constraint_specification(references, element) {
            Ok(None) => issues.extend(Unverified("Constraint draft has no authored specification; Boolean semantics and side effects are unverified".into())
                .diagnostic(element, "Constraint-specification", "UML_CONSTRAINT_SPECIFICATION")),
            Ok(Some(specification)) => match value_specifications::evaluate(references, specification.id()) {
                // false is still a Boolean expression, not a metamodel type error.
                Ok(LiteralValue::Boolean(_)) => {}
                Ok(_) => issues.extend(Violation("specification must evaluate to a Boolean".into())
                    .diagnostic(element, "Constraint-boolean_value", "UML_CONSTRAINT_BOOLEAN")),
                Err(error) if error.is_unsupported() => issues.extend(Unverified(format!(
                    "expression evaluation and absence of side effects were not verified: {error}"))
                    .diagnostic(element, "Constraint-boolean_value/Constraint-no_side_effects", "UML_CONSTRAINT_BOOLEAN")),
                Err(error) => issues.extend(Violation(error.to_string())
                    .diagnostic(element, "Constraint-specification", "UML_CONSTRAINT_SPECIFICATION")),
            },
            Err(error) => issues.extend(Violation(error)
                .diagnostic(element, "Constraint-specification", "UML_CONSTRAINT_SPECIFICATION")),
        }
    }
    if !element.conforms_to("OpaqueExpression") { return; }
    let length = |key: &str| -> Option<usize> {
        match element.properties().get(key) {
            None => Some(0),
            Some(Value::Array(values)) => Some(values.len()),
            _ => None, // collection representation is diagnosed by check_value
        }
    };
    if let (Some(language), Some(body)) = (length("language"), length("body")) {
        if language > 0 && language != body {
            issues.extend(Violation("nonempty language and body collections must have equal size".into())
                .diagnostic(element, "OpaqueExpression-language_body_size", "UML_OPAQUE_EXPRESSION_LANGUAGES"));
        }
    }
    let Some(behavior_id) = element.properties().get("behavior")
        .and_then(|value| reference_id(value).ok().flatten()) else { return; };
    let Ok(behavior) = references.element(behavior_id) else { return; };
    if !behavior.conforms_to("Behavior") { return; } // typed-reference diagnostic already emitted
    let parameters = match operations::owned_parameters(references, behavior_id) {
        Ok(parameters) => parameters,
        Err(error) => {
            issues.extend(Unverified(error)
                .diagnostic(element, "OpaqueExpression-one_return_result_parameter", "UML_OPAQUE_EXPRESSION_PARAMETERS"));
            return;
        }
    };
    let mut returns = 0;
    let mut permitted = true;
    for record in parameters {
        let direction = match operations::direction(record) {
            Ok(direction) => direction,
            Err(error) => {
                issues.extend(Violation(error)
                    .diagnostic(element, "OpaqueExpression-only_in_or_return_parameters", "UML_OPAQUE_EXPRESSION_PARAMETERS"));
                return;
            }
        };
        let stream = match schema::read_effective_boolean(ElementRef::Object(record), "isStream") {
            Ok(stream) => stream,
            Err(error) => {
                issues.extend(Unverified(error)
                    .diagnostic(element, "OpaqueExpression-only_in_or_return_parameters", "UML_OPAQUE_EXPRESSION_PARAMETERS"));
                return;
            }
        };
        if direction == "return" { returns += 1; }
        permitted &= !stream && matches!(direction.as_str(), "in" | "return");
    }
    if returns != 1 {
        issues.extend(Violation("referenced Behavior must have exactly one return Parameter".into())
            .diagnostic(element, "OpaqueExpression-one_return_result_parameter", "UML_OPAQUE_EXPRESSION_PARAMETERS"));
    }
    if !permitted {
        issues.extend(Violation("referenced Behavior allows only non-stream in/return Parameters".into())
            .diagnostic(element, "OpaqueExpression-only_in_or_return_parameters", "UML_OPAQUE_EXPRESSION_PARAMETERS"));
    }
}
