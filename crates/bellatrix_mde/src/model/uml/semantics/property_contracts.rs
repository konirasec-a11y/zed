//! UML 2.5.1 property contracts over the existing canonical snapshot.
//!
//! The generated PropertyDescriptor supplies type, bounds, order and uniqueness.
//! This module completes the reference/collection branch of Model::validate; it
//! neither stores a second model nor evaluates arbitrary OCL. Satisfied below
//! means that this property's representation contract was checked, not that all
//! constraints of its metaclass were evaluated.

use super::{
    associations, operations,
    references::{ReferenceIndex, external_reference_href, reference_id},
    root::ElementRef,
    schema::{self, Aggregation, PropertyDescriptor, ScalarKind, UnsupportedContract},
    types::{Integer, MultiplicityRange, StandardPrimitiveType, UnlimitedNatural},
    value_specifications::{self, LiteralValue},
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
    fn diagnostic(
        self,
        element: ElementRef<'_>,
        property: &str,
        code: &'static str,
    ) -> Option<StructuralDiagnostic> {
        let (severity, code, message) = match self {
            Self::Satisfied => return None,
            Self::Violation(message) => (
                StructuralSeverity::Error,
                if property == "InterfaceRealization-implementingClassifier" {
                    "UML_INTERFACE_REALIZATION_SOURCE"
                } else {
                    code
                },
                message,
            ),
            Self::Unverified(message) => (
                StructuralSeverity::Warning,
                "MDE_PROPERTY_CONTRACT_UNVERIFIED",
                message,
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
    let range = match format!("{}..{}", property.lower, property.upper).parse::<MultiplicityRange>()
    {
        Ok(range) => range,
        Err(error) => return Unverified(format!("unresolved normative multiplicity: {error}")),
    };
    let many = !matches!(property.upper, "0" | "1");
    let values: &[Value] = if many {
        match value.as_array() {
            Some(values) => values,
            None => {
                return Violation(
                    "expected a collection, including [] for an empty collection".into(),
                );
            }
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
            "requires {}..{} values; received {cardinality}",
            property.lower, property.upper,
        ));
    }
    // Scalar checking of each member is independent of the collection's bounds.
    // Do not alter the generated descriptor or replace ordered/bag collections
    // with sets: uniqueness is checked only when the descriptor requires it.
    let member = PropertyDescriptor {
        lower: "1",
        upper: "1",
        ..*property
    };
    let scalar_kind = match schema::scalar_kind(&member) {
        Ok(kind) => Some(kind),
        Err(UnsupportedContract::Reference) => None,
        Err(error) => return Unverified(error.to_string()),
    };
    let mut seen = HashSet::new();
    let mut external_references = HashSet::new();
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
                unverified.get_or_insert_with(|| {
                    "text declaration is not a resolved UML reference".into()
                });
                continue;
            }
            let external = match external_reference_href(value) {
                Ok(external) => external,
                Err(error) => return Violation(format!("reference[{position}]: {error}")),
            };
            if let Some(href) = external {
                if property.is_unique && !external_references.insert(href) {
                    return Violation(format!(
                        "duplicate external reference at index {position}; isUnique is true"
                    ));
                }
                unverified.get_or_insert_with(|| {
                    "external model reference has no loaded document resolver".into()
                });
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
                (Some(required), Some(actual)) => match schema::conforms_to(actual.id, required.id)
                {
                    Ok(true) => {}
                    Ok(false) => {
                        return Violation(format!("{id} must conform to UML {}", required.name));
                    }
                    Err(error) => {
                        unverified.get_or_insert_with(|| error.to_string());
                    }
                },
                (None, _) => {
                    unverified
                        .get_or_insert_with(|| "normative reference Type is unresolved".into());
                }
                (_, None) => {
                    unverified
                        .get_or_insert_with(|| format!("{id} has no supported UML metaclass"));
                }
            }
            if property.aggregation == Aggregation::Composite {
                match references.model().owner(target) {
                    Ok(Some(actual_owner)) if actual_owner.id() == owner.id() => {}
                    Ok(_) => {
                        return Violation(format!(
                            "composite target {id} must be owned by {} in canonical ownership",
                            owner.id(),
                        ));
                    }
                    Err(error) => return Violation(error),
                }
            }
            id.to_owned()
        };
        if property.is_unique && !seen.insert(key) {
            return Violation(format!(
                "duplicate value at index {position}; isUnique is true"
            ));
        }
    }
    unverified.map_or(Satisfied, Unverified)
}

fn scalar_identity(kind: ScalarKind, value: &Value) -> String {
    let decimal = value
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| value.to_string());
    match kind {
        ScalarKind::Primitive(StandardPrimitiveType::Integer) => decimal
            .parse::<Integer>()
            .map(|value| value.to_string())
            .unwrap_or(decimal),
        ScalarKind::Primitive(StandardPrimitiveType::UnlimitedNatural) => decimal
            .parse::<UnlimitedNatural>()
            .map(|value| value.to_string())
            .unwrap_or(decimal),
        ScalarKind::Primitive(StandardPrimitiveType::Real) => value
            .as_f64()
            .map(|value| {
                if value == 0.0 {
                    "0".into()
                } else {
                    value.to_string()
                }
            })
            .unwrap_or_else(|| value.to_string()),
        _ => value.to_string(),
    }
}

// Binary relationship endpoints supply canonical roles without creating stored copies.
// The returned values are borrowed-state projections, never persisted copies.
fn endpoint_value(element: ElementRef<'_>, property: &PropertyDescriptor) -> Option<Value> {
    let ElementRef::Relationship(relation) = element else {
        return None;
    };
    let target = match property.id.fragment {
        "Generalization-specific"
        | "ElementImport-importingNamespace"
        | "PackageImport-importingNamespace"
        | "InterfaceRealization-implementingClassifier"
        | "Include-includingCase"
        | "Extend-extension"
        | "PackageMerge-receivingPackage"
        | "ActivityEdge-source"
        | "Transition-source"
        | "Deployment-location"
        | "TemplateBinding-boundElement" => &relation.from,
        "Generalization-general"
        | "ElementImport-importedElement"
        | "PackageImport-importedPackage"
        | "InterfaceRealization-contract"
        | "Include-addition"
        | "Extend-extendedCase"
        | "PackageMerge-mergedPackage"
        | "ActivityEdge-target"
        | "Transition-target"
        | "Deployment-deployedArtifact"
        | "Manifestation-utilizedElement"
        | "TemplateBinding-signature" => &relation.to,
        _ => return None,
    };
    Some(if matches!(property.upper, "0" | "1") {
        json!({"$ref": target})
    } else {
        json!([{"$ref": target}])
    })
}

fn constraint_specification<'a>(
    references: &ReferenceIndex<'a>,
    constraint: ElementRef<'a>,
) -> Result<Option<ElementRef<'a>>, String> {
    let specifications = references
        .explicit_owned_elements(constraint.id())?
        .into_iter()
        .filter(|child| child.conforms_to("ValueSpecification"))
        .collect::<Vec<_>>();
    if specifications.is_empty() && !constraint.properties().contains_key("specification") {
        // The editor can retain an unfinished Constraint without inventing an
        // expression. An explicit null or malformed reference is still invalid.
        return Ok(None);
    }
    let [specification] = specifications.as_slice() else {
        return Err(
            "Constraint::specification requires exactly one owned ValueSpecification".into(),
        );
    };
    if let Some(value) = constraint.properties().get("specification") {
        if reference_id(value)? != Some(specification.id()) {
            return Err(
                "Constraint::specification must identify its canonical owned ValueSpecification"
                    .into(),
            );
        }
    }
    Ok(Some(*specification))
}

fn required_role<'a>(
    references: &ReferenceIndex<'a>,
    element: ElementRef<'a>,
    property: &PropertyDescriptor,
) -> Result<Option<Value>, String> {
    if let Some(value) = endpoint_value(element, property) {
        return Ok(Some(value));
    }
    match property.id.fragment {
        "Dependency-client" | "Dependency-supplier" => {
            let result = if property.name == "client" {
                super::relationship_endpoints::clients(references, element.id())
            } else {
                super::relationship_endpoints::suppliers(references, element.id())
            };
            let values = match result {
                Ok(values) => values,
                Err(super::element_graph_queries::ElementGraphError::Unsupported { .. }) => {
                    return Ok(None);
                }
                Err(error) => return Err(error.to_string()),
            };
            Ok(Some(Value::Array(
                values
                    .into_iter()
                    .map(|element| json!({"$ref": element.id()}))
                    .collect(),
            )))
        }
        "Constraint-specification" => {
            constraint_specification(references, element).map(|specification| {
                specification.map(|specification| json!({"$ref": specification.id()}))
            })
        }
        "Slot-owningInstance" => references
            .model()
            .owner(element)
            .map(|owner| Some(owner.map_or(Value::Null, |owner| json!({"$ref": owner.id()})))),
        "Association-memberEnd" => Ok(None),
        "Slot-definingFeature" => Err(format!(
            "{} requires explicit canonical reference identities",
            property.id.fragment
        )),
        _ => Ok(None),
    }
}

fn port_default_value(references: &ReferenceIndex<'_>, port: ElementRef<'_>) -> PropertyOutcome {
    use PropertyOutcome::{Satisfied, Unverified, Violation};
    let interface_type = (|| -> Result<bool, String> {
        let Some(value) = port
            .properties()
            .get("type")
            .filter(|value| !value.is_null())
        else {
            return Ok(false);
        };
        let property = schema::uml_property("Port", "type")
            .map_err(|error| error.to_string())?
            .ok_or("Port type contract is unavailable")?;
        match check_value(references, port, property, value) {
            Satisfied => {}
            Violation(error) | Unverified(error) => return Err(error),
        }
        let identifier = reference_id(value)?.ok_or("Port type identity is absent")?;
        Ok(references.element(identifier)?.conforms_to("Interface"))
    })();
    if matches!(interface_type, Ok(false)) {
        return Satisfied;
    }
    let default_present = (|| -> Result<bool, String> {
        if let Some(value) = port.properties().get("defaultValue") {
            if value.is_null() {
                return Ok(false);
            }
            let property = schema::uml_property("Port", "defaultValue")
                .map_err(|error| error.to_string())?
                .ok_or("Port defaultValue contract is unavailable")?;
            return match check_value(references, port, property, value) {
                Satisfied => Ok(true),
                Violation(error) | Unverified(error) => Err(error),
            };
        }
        // An owned ValueSpecification can also be a multiplicity bound. Do not
        // invent a defaultValue role from containment alone.
        let mut unresolved = false;
        for child in references.explicit_owned_elements(port.id())? {
            if !child.conforms_to("ValueSpecification") {
                continue;
            }
            let roles = super::classifiers::ownership_roles(port, child)
                .map_err(|error| error.to_string())?;
            if roles
                .properties
                .iter()
                .any(|property| property.id.fragment == "Property-defaultValue")
            {
                if roles.is_resolved {
                    return Ok(true);
                }
                unresolved = true;
            }
        }
        if unresolved {
            Err(
                "owned ValueSpecification has no resolved defaultValue or multiplicity-bound role"
                    .into(),
            )
        } else {
            Ok(false)
        }
    })();
    // The consequent being true establishes the implication even if the type
    // cannot be resolved. Otherwise both operands must be known to prove a violation.
    match (interface_type, default_present) {
        (_, Ok(false)) => Satisfied,
        (Ok(true), Ok(true)) => {
            Violation("a Port typed by an Interface cannot specify a defaultValue".into())
        }
        (Err(error), _) | (_, Err(error)) => Unverified(error),
        (Ok(false), _) => Satisfied,
    }
}

// Constraints are evaluated structurally; their truth against the modeled
// application's instances is not confused with UML model well-formedness.
fn opaque_expression_parameters(
    references: &ReferenceIndex<'_>,
    expression: ElementRef<'_>,
) -> (
    Result<bool, super::classifiers::QueryError>,
    Result<bool, super::classifiers::QueryError>,
) {
    use super::{classifiers::QueryError, feature_compatibility::require_all};
    let evidence = || {
        let Some(value) = expression
            .properties()
            .get("behavior")
            .filter(|value| !value.is_null())
        else {
            return Ok(None);
        };
        let descriptor = schema::uml_property("OpaqueExpression", "behavior")
            .map_err(|error| QueryError::unsupported(expression.id(), error.to_string()))?
            .ok_or_else(|| {
                QueryError::unsupported(
                    expression.id(),
                    "missing OpaqueExpression behavior contract",
                )
            })?;
        match check_value(references, expression, descriptor, value) {
            PropertyOutcome::Violation(message) => {
                return Err(QueryError::invalid(expression.id(), message));
            }
            PropertyOutcome::Unverified(message) => {
                return Err(QueryError::unsupported(expression.id(), message));
            }
            PropertyOutcome::Satisfied => {}
        }
        let behavior_id = reference_id(value)
            .map_err(|error| QueryError::invalid(expression.id(), error))?
            .ok_or_else(|| QueryError::invalid(expression.id(), "behavior identity is missing"))?;
        operations::owned_parameter_evidence(references, behavior_id).map(Some)
    };
    let (parameters, completeness) = match evidence() {
        Ok(None) => return (Ok(true), Ok(true)),
        Ok(Some(evidence)) => evidence,
        Err(error) => return (Err(error.clone()), Err(error)),
    };
    let mut return_checks = Vec::new();
    let mut permitted_checks = vec![completeness.clone().map(|_| true)];
    let mut returns = 0;
    for parameter in parameters {
        let direction = operations::direction(parameter)
            .map_err(|error| QueryError::invalid(&parameter.id, error));
        match direction {
            Ok(direction) => {
                if direction == "return" {
                    returns += 1;
                }
                permitted_checks.push(Ok(matches!(direction.as_str(), "in" | "return")));
            }
            Err(error) => {
                return_checks.push(Err(error.clone()));
                permitted_checks.push(Err(error));
            }
        }
        permitted_checks.push(
            schema::read_effective_boolean(ElementRef::Object(parameter), "isStream")
                .map(|stream| !stream)
                .map_err(|error| QueryError::invalid(&parameter.id, error)),
        );
    }
    // Unresolved members can add return Parameters but cannot remove two
    // already established ones. Likewise a known out/stream member refutes
    // the universal constraint even when other members are unavailable.
    return_checks.push(if returns > 1 {
        Ok(false)
    } else {
        completeness.map(|_| returns == 1)
    });
    (require_all(return_checks), require_all(permitted_checks))
}
