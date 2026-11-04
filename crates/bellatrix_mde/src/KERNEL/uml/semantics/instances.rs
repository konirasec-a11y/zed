//! Borrowed instance/slot views over canonical Elements and ownership edges.
//! Slottable features use the shared Classifier graph, including inherited private attributes.
// SPDX-License-Identifier: MIT
// Slot ownership transitions adapt Slot::set_owning_instance in javapak/ruml-wasm,
// commit 5c90d7399acbce4c37847b272759ed85d677ba92, slot.rs:167-188.
// Upstream declares MIT in Cargo.toml; its source contains no copyright notice.
// Provenance and adaptation boundaries: docs/bellatrix/references/RUML.md.

use super::{
    classifiers::{self, QueryError},
    references::{ReferenceIndex, reference_id},
    root::{Element, ElementRef, Model},
    schema,
    value_specifications::{self, EvaluationError, LiteralValue},
};
use crate::{StructuralDiagnostic, StructuralSeverity};
use serde_json::Value;
use std::collections::HashSet;

#[derive(Clone, Copy, Debug)]
pub struct InstanceSpecificationRef<'a>(ElementRef<'a>);

#[derive(Clone, Copy, Debug)]
pub struct SlotRef<'a>(ElementRef<'a>);

impl<'a> Element<'a> for InstanceSpecificationRef<'a> {
    fn as_element(self) -> ElementRef<'a> {
        self.0
    }
}

impl<'a> Element<'a> for SlotRef<'a> {
    fn as_element(self) -> ElementRef<'a> {
        self.0
    }
}

impl<'a> InstanceSpecificationRef<'a> {
    pub(crate) fn from_element(element: ElementRef<'a>) -> Option<Self> {
        matches!(element, ElementRef::Object(_))
            .then_some(element)
            .filter(|element| element.conforms_to("InstanceSpecification"))
            .map(Self)
    }

    pub fn slots(self, model: Model<'a>) -> Result<Vec<SlotRef<'a>>, String> {
        Ok(model
            .owned_elements(self)?
            .into_iter()
            .filter_map(SlotRef::from_element)
            .collect())
    }

    /// Evaluate the optional owned specification without executing expression text.
    pub fn specification(self, model: Model<'a>) -> Result<Option<LiteralValue>, QueryError> {
        Ok(evaluate_owned_values(
            &ReferenceIndex::new(model), self.0, "InstanceSpecification", "specification",
            "InstanceSpecification-specification",
        )?.into_iter().next())
    }

    /// The adapter stores classifier identities, never classifier names or DTO copies.
    pub fn classifiers(self, model: Model<'a>) -> Result<Vec<ElementRef<'a>>, String> {
        self.classifiers_with(&ReferenceIndex::new(model))
    }

    fn classifiers_with(
        self,
        references: &ReferenceIndex<'a>,
    ) -> Result<Vec<ElementRef<'a>>, String> {
        let Some(value) = self.0.properties().get("classifier") else {
            return Ok(Vec::new());
        };
        let identifiers = value
            .as_array()
            .ok_or("classifier must be an array of local Element references")?;
        let mut seen = HashSet::new();
        identifiers
            .iter()
            .map(|identifier| {
                let identifier = reference_id(identifier)
                    .map_err(|error| format!("classifier: {error}"))?
                    .ok_or("classifier cannot contain a null reference")?;
                if !seen.insert(identifier) {
                    return Err("InstanceSpecification classifiers must be unique".into());
                }
                let element = references
                    .element(identifier)
                    .map_err(|error| format!("classifier: {error}"))?;
                if !element.conforms_to("Classifier") {
                    return Err(format!(
                        "classifier {identifier} must resolve to a Classifier"
                    ));
                }
                Ok(element)
            })
            .collect()
    }
}

impl<'a> SlotRef<'a> {
    pub(crate) fn from_element(element: ElementRef<'a>) -> Option<Self> {
        matches!(element, ElementRef::Object(_))
            .then_some(element)
            .filter(|element| element.conforms_to("Slot"))
            .map(Self)
    }

    /// Preserve the authored UML value order, rather than inventing one from ownership edges.
    pub fn values(self, model: Model<'a>) -> Result<Vec<LiteralValue>, QueryError> {
        evaluate_owned_values(
            &ReferenceIndex::new(model), self.0, "Slot", "value", "Slot-value",
        )
    }

    pub fn owning_instance(self, model: Model<'a>) -> Result<InstanceSpecificationRef<'a>, String> {
        model
            .owner(self)?
            .and_then(InstanceSpecificationRef::from_element)
            .ok_or_else(|| format!("Slot {} requires an InstanceSpecification owner", self.id()))
    }

    pub fn defining_feature(self, model: Model<'a>) -> Result<ElementRef<'a>, String> {
        self.defining_feature_with(&ReferenceIndex::new(model))
    }

    fn defining_feature_with(
        self,
        references: &ReferenceIndex<'a>,
    ) -> Result<ElementRef<'a>, String> {
        let property = schema::uml_property("Slot", "definingFeature")
            .map_err(|error| error.to_string())?
            .ok_or("missing Slot definingFeature contract")?;
        references
            .validate(
                property,
                self.0
                    .properties()
                    .get("definingFeature")
                    .unwrap_or(&Value::Null),
            )
            .map_err(|error| format!("Slot definingFeature: {error}"))?
            .ok_or_else(|| "Slot definingFeature requires a StructuralFeature reference".into())
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum AssignmentError {
    #[error("{0}")]
    Invalid(String),
    #[error("Unsupported Slot capability: {0}")]
    Unsupported(String),
}

impl From<QueryError> for AssignmentError {
    fn from(error: QueryError) -> Self {
        match error {
            QueryError::Invalid { message, .. } => Self::Invalid(message),
            QueryError::Unsupported { message, .. } => Self::Unsupported(message),
        }
    }
}

impl From<String> for AssignmentError {
    fn from(message: String) -> Self {
        Self::Invalid(message)
    }
}

fn evaluate_owned_values(
    references: &ReferenceIndex<'_>,
    owner: ElementRef<'_>,
    metaclass: &str,
    property: &str,
    role: &'static str,
) -> Result<Vec<LiteralValue>, QueryError> {
    Ok(evaluated_owned_values(references, owner, metaclass, property, role)?
        .into_iter().map(|(_, value)| value).collect())
}

struct OwnedValueEvidence<'a> {
    specifications: Vec<ElementRef<'a>>,
    values: Vec<(ElementRef<'a>, LiteralValue)>,
    unknown: Option<QueryError>,
}

fn evaluated_owned_values<'a>(
    references: &ReferenceIndex<'a>,
    owner: ElementRef<'_>,
    metaclass: &str,
    property: &str,
    role: &'static str,
) -> Result<Vec<(ElementRef<'a>, LiteralValue)>, QueryError> {
    let evidence = owned_value_evidence(references, owner, metaclass, property, role)?;
    match evidence.unknown {
        Some(error) => Err(error),
        None => Ok(evidence.values),
    }
}

fn owned_value_evidence<'a>(
    references: &ReferenceIndex<'a>,
    owner: ElementRef<'_>,
    metaclass: &str,
    property: &str,
    role: &'static str,
) -> Result<OwnedValueEvidence<'a>, QueryError> {
    use super::property_contracts::{self, PropertyOutcome};
    let descriptor = schema::uml_property(metaclass, property)
        .map_err(|error| QueryError::unsupported(owner.id(), error.to_string()))?
        .ok_or_else(|| QueryError::unsupported(owner.id(), "missing owned value contract"))?;
    let mut unknown = None;
    let authored = owner.properties().get(property);
    let mut values = Vec::new();
    if let Some(authored) = authored {
        match property_contracts::check_value(references, owner, descriptor, authored) {
            PropertyOutcome::Violation(message) => return Err(QueryError::invalid(owner.id(), message)),
            PropertyOutcome::Unverified(message) => unknown = Some(QueryError::unsupported(owner.id(), message)),
            PropertyOutcome::Satisfied => {}
        }
        let members = if matches!(descriptor.upper, "0" | "1") {
            std::slice::from_ref(authored)
        } else {
            authored.as_array().ok_or_else(|| QueryError::invalid(owner.id(), "value requires a reference collection"))?
        };
        for member in members {
            if super::references::external_reference_href(member)
                .map_err(|error| QueryError::invalid(owner.id(), error))?.is_some()
            {
                continue;
            }
            if let Some(id) = reference_id(member).map_err(|error| QueryError::invalid(owner.id(), error))? {
                values.push(references.element(id).map_err(|error| QueryError::invalid(owner.id(), error))?);
            }
        }
    }
    let mut specifications = values.clone();
    let mut owned_values = Vec::new();
    for child in references.explicit_owned_elements(owner.id())
        .map_err(|error| QueryError::invalid(owner.id(), error))?
    {
        if child.conforms_to("ValueSpecification") && !specifications.iter().any(|value| value.id() == child.id()) {
            specifications.push(child);
        }
        match classifiers::contributes_to_owned_union(owner, child, role) {
            Ok(true) => owned_values.push(child),
            Ok(false) => {}
            Err(error @ QueryError::Invalid { .. }) => return Err(error),
            Err(error) => {
                // An unresolved role does not repair malformed literal data on
                // a known canonical child. Check that independently without
                // assigning the child to this property or inventing its order.
                if child.conforms_to("ValueSpecification") {
                    match value_specifications::evaluate(references, child.id()) {
                        Err(EvaluationError::Invalid { subject_id, message }) => {
                            return Err(QueryError::invalid(&subject_id, message));
                        }
                        Ok(_) | Err(EvaluationError::Unsupported { .. }) => {}
                    }
                }
                unknown = Some(error);
            }
        }
    }
    if matches!(descriptor.upper, "0" | "1") && owned_values.len() > 1 {
        return Err(QueryError::invalid(owner.id(), format!("{property} permits at most one owned ValueSpecification")));
    }
    if authored.is_none() {
        values = owned_values;
        // Canonical ownership establishes membership, not the order of UML's
        // ordered Slot::value. A singleton needs no separate ordering witness.
        if descriptor.is_ordered && values.len() > 1 {
            unknown = Some(QueryError::unsupported(owner.id(), "ordered Slot values require an explicit value reference collection"));
        }
    } else {
        for child in owned_values {
            if !values.iter().any(|value| value.id() == child.id()) {
                return Err(QueryError::invalid(owner.id(), format!("{property} omits a canonical owned value")));
            }
        }
    }
    let mut evaluated = Vec::new();
    for value in values {
        match value_specifications::evaluate(references, value.id()) {
            Ok(literal) => evaluated.push((value, literal)),
            Err(EvaluationError::Invalid { subject_id, message }) => return Err(QueryError::invalid(&subject_id, message)),
            Err(EvaluationError::Unsupported { subject_id, message }) => {
                unknown = Some(QueryError::unsupported(&subject_id, message));
            }
        }
    }
    Ok(OwnedValueEvidence { specifications, values: evaluated, unknown })
}

fn slot_values_conform(
    references: &ReferenceIndex<'_>,
    slot: ElementRef<'_>,
    feature: ElementRef<'_>,
) -> Result<bool, QueryError> {
    use super::feature_compatibility::{self, require_all};
    let values = owned_value_evidence(references, slot, "Slot", "value", "Slot-value");
    let multiplicity = feature_compatibility::resolve_feature_multiplicity(references, feature);
    let unique = schema::read_effective_boolean(feature, "isUnique")
        .map_err(|error| QueryError::invalid(feature.id(), error));
    // A malformed feature contract stays Invalid even if a Slot expression is
    // opaque. Do not short-circuit on evaluation's Unsupported result.
    let mut checks = vec![
        multiplicity.as_ref().map(|_| true).map_err(Clone::clone),
        unique.as_ref().map(|_| true).map_err(Clone::clone),
    ];
    if let Err(error) = value_specifications::standard_primitive_reference(
        feature.properties().get("type").unwrap_or(&Value::Null),
    ) {
        checks.push(Err(QueryError::invalid(feature.id(), error)));
    }
    match feature_compatibility::resolve_feature_type(references, feature) {
        Err(error @ QueryError::Invalid { .. }) => checks.push(Err(error)),
        // An empty value set requires no type witness. Nonempty values below
        // still require conformance, including resolution of the feature Type.
        Ok(_) | Err(QueryError::Unsupported { .. }) => {}
    }
    match values {
        Err(error) => checks.push(Err(error)),
        Ok(evidence) => {
            // Reference validity belongs to the specification, even when it
            // contributes no value (LiteralNull) or cannot yet be evaluated.
            for specification in evidence.specifications {
                checks.push(declared_value_type_contract(references, specification));
            }
            let complete = evidence.unknown.is_none();
            if let Some(error) = evidence.unknown {
                checks.push(Err(error));
            }
            let values = evidence.values;
            // LiteralNull denotes absence, whereas a LiteralString without its
            // scalar value leaves both cardinality and equality unresolved.
            let present = values.iter().filter(|(_, value)| !matches!(value, LiteralValue::Null))
                .collect::<Vec<_>>();
            let unspecified = present.iter().any(|(_, value)| matches!(value, LiteralValue::String(None)));
            if let Ok(multiplicity) = multiplicity {
                let known_count = present.iter()
                    .filter(|(_, value)| !matches!(value, LiteralValue::String(None)))
                    .count().to_string().parse::<super::types::Integer>()
                    .map_err(|error| QueryError::invalid(slot.id(), error.to_string()))?;
                checks.push(if multiplicity.upper_bound().finite_value().is_some_and(|upper| &known_count > upper) {
                    // Unknown expressions can add values, never remove these
                    // already established members exceeding the upper bound.
                    Ok(false)
                } else if !complete || unspecified {
                    Err(QueryError::unsupported(slot.id(), "Slot cardinality includes unresolved values"))
                } else {
                    Ok(multiplicity.includes_cardinality(&known_count))
                });
            }
            for (specification, value) in &present {
                checks.push(literal_type_conformance(references, *specification, value, feature));
            }
            if matches!(unique, Ok(true)) {
                for (position, (_, value)) in present.iter().enumerate() {
                    for (_, other) in present.iter().skip(position + 1) {
                        checks.push(literal_values_distinct(slot.id(), value, other));
                    }
                }
            }
        }
    }
    require_all(checks)
}

fn declared_value_type_contract(
    references: &ReferenceIndex<'_>,
    specification: ElementRef<'_>,
) -> Result<bool, QueryError> {
    let Some(declared) = specification.properties().get("type").filter(|value| !value.is_null())
    else { return Ok(true); };
    match value_specifications::standard_primitive_reference(declared) {
        Ok(Some(_)) => Ok(true),
        Err(error) => Err(QueryError::invalid(specification.id(), error)),
        Ok(None) => super::feature_compatibility::resolve_feature_type(references, specification).map(|_| true),
    }
}

fn literal_type_conformance(
    references: &ReferenceIndex<'_>,
    specification: ElementRef<'_>,
    value: &LiteralValue,
    feature: ElementRef<'_>,
) -> Result<bool, QueryError> {
    use super::feature_compatibility::{self, require_all};
    let intrinsic = value.primitive_type();
    let required = value_specifications::standard_primitive_reference(
        feature.properties().get("type").unwrap_or(&Value::Null),
    ).map_err(|error| QueryError::invalid(feature.id(), error));
    let mut checks = Vec::new();
    match required {
        Ok(Some(required)) => checks.push(Ok(intrinsic == Some(required))),
        Err(error) => checks.push(Err(error)),
        Ok(None) => {
            checks.push(feature_compatibility::compare_type_conformance(references, specification, feature));
            checks.push(Err(QueryError::unsupported(specification.id(),
                "literal conformance requires a source-qualified normative PrimitiveType identity")));
        }
    }
    if let Some(declared) = specification.properties().get("type").filter(|value| !value.is_null()) {
        match value_specifications::standard_primitive_reference(declared) {
            Ok(Some(declared)) => checks.push(Ok(intrinsic == Some(declared))),
            Err(error) => checks.push(Err(QueryError::invalid(specification.id(), error))),
            Ok(None) => {
                checks.push(feature_compatibility::resolve_feature_type(references, specification).map(|_| true));
                checks.push(Err(QueryError::unsupported(specification.id(),
                    "declared literal Type has no proven link to its normative primitive domain")));
            }
        }
    }
    require_all(checks)
}

fn literal_values_distinct(
    slot_id: &str,
    value: &LiteralValue,
    other: &LiteralValue,
) -> Result<bool, QueryError> {
    let numeric = |value: &LiteralValue| matches!(value,
        LiteralValue::Integer(_) | LiteralValue::UnlimitedNatural(_) | LiteralValue::Real(_));
    if matches!(value, LiteralValue::String(None)) || matches!(other, LiteralValue::String(None))
        || numeric(value) && numeric(other) && std::mem::discriminant(value) != std::mem::discriminant(other)
    {
        return Err(QueryError::unsupported(slot_id, "Slot value equality requires unavailable cross-type or unspecified-value semantics"));
    }
    Ok(value != other)
}

fn validate_instance<'a>(
    instance: InstanceSpecificationRef<'a>,
    references: &ReferenceIndex<'a>,
) -> Result<Vec<ElementRef<'a>>, AssignmentError> {
    let empty = Value::Array(Vec::new());
    let (classifiers, unknown) = instance_reference_evidence(
        references, instance.0, "InstanceSpecification", "classifier",
        instance.0.properties().get("classifier").unwrap_or(&empty),
    )?;
    // Value errors are independent of whether every classifier identity is
    // resolved. A missing local target remains Invalid, an external one Unknown.
    evaluate_owned_values(
        references, instance.0, "InstanceSpecification", "specification",
        "InstanceSpecification-specification",
    )?;
    if instance.0.properties().contains_key("slot") {
        return Err(AssignmentError::Unsupported(
            "slot must be projected from the canonical ownership edge".into(),
        ));
    }
    if unknown {
        return Err(AssignmentError::Unsupported("InstanceSpecification classifier identity is unresolved".into()));
    }
    Ok(classifiers)
}

/// UML 2.5.1 §§9.9.9, 9.9.20 and Classifier::allSlottableFeatures (§9.9.4.7).
/// This preflight replaces upstream's unchecked opposite writes with one atomic edge change.
pub(crate) fn validate_assignment<'a>(
    slot: SlotRef<'a>,
    owner: InstanceSpecificationRef<'a>,
    model: Model<'a>,
) -> Result<(), AssignmentError> {
    validate_assignment_with(slot, owner, model, &ReferenceIndex::new(model))
}

fn validate_assignment_with<'a>(
    slot: SlotRef<'a>,
    owner: InstanceSpecificationRef<'a>,
    model: Model<'a>,
    references: &ReferenceIndex<'a>,
) -> Result<(), AssignmentError> {
    use super::feature_compatibility::{accept_any, require_all};
    let mut checks = Vec::new();
    if slot.0.properties().contains_key("owningInstance")
        || owner.0.properties().contains_key("slot")
        || model.owner(slot)?.is_some_and(|previous| previous.properties().contains_key("slot"))
    {
        checks.push(Err(QueryError::unsupported(slot.id(),
            "slot and owningInstance must be projected from the canonical ownership edge")));
    }
    let (features, unknown_feature) = instance_reference_evidence(
        references, slot.0, "Slot", "definingFeature",
        slot.0.properties().get("definingFeature").unwrap_or(&Value::Null),
    )?;
    let feature = features.first().copied();
    if unknown_feature {
        checks.push(Err(QueryError::unsupported(slot.id(), "definingFeature Type or identity is unresolved")));
    }
    let empty = Value::Array(Vec::new());
    let (classifiers, unknown_classifier) = instance_reference_evidence(
        references, owner.0, "InstanceSpecification", "classifier",
        owner.0.properties().get("classifier").unwrap_or(&empty),
    )?;
    checks.push(evaluate_owned_values(
        references, owner.0, "InstanceSpecification", "specification",
        "InstanceSpecification-specification",
    ).map(|_| true));
    if let Some(feature) = feature.filter(|feature| feature.conforms_to("StructuralFeature")) {
        checks.push(slot_values_conform(references, slot.0, feature));
    } else {
        // The missing feature prevents compatibility proof, not independent
        // checks of the value records and their declared Type references.
        match owned_value_evidence(references, slot.0, "Slot", "value", "Slot-value") {
            Err(error) => checks.push(Err(error)),
            Ok(evidence) => {
                for specification in evidence.specifications {
                    checks.push(declared_value_type_contract(references, specification));
                }
                if let Some(error) = evidence.unknown { checks.push(Err(error)); }
            }
        }
        checks.push(Err(QueryError::unsupported(slot.id(), "value compatibility needs a resolved definingFeature")));
    }
    let mut membership = Vec::new();
    if unknown_classifier {
        membership.push(Err(QueryError::unsupported(owner.id(), "unresolved classifier may supply definingFeature")));
    }
    for classifier in classifiers {
        membership.push(match feature {
            Some(feature) => slottable_membership(references, classifier, feature.id()),
            None => slottable_features(references, classifier).and_then(|features| {
                if features.is_empty() { Ok(false) }
                else { Err(QueryError::unsupported(slot.id(), "external definingFeature identity is unresolved")) }
            }),
        });
    }
    // Assignment requires an existential witness in the classifier graph, not
    // resolution of every classifier. Malformed graph evidence still dominates.
    checks.push(accept_any(membership));
    let (slots, unknown_slot) = instance_slots_evidence(references, owner.0)?;
    if unknown_slot {
        checks.push(Err(QueryError::unsupported(owner.id(), "unresolved owned Slot may duplicate definingFeature")));
    }
    for other in slots.into_iter().filter(|other| other.id() != slot.id()) {
        let (other_features, unknown) = instance_reference_evidence(
            references, other, "Slot", "definingFeature",
            other.properties().get("definingFeature").unwrap_or(&Value::Null),
        )?;
        if let (Some(feature), Some(other_feature)) = (feature, other_features.first()) {
            checks.push(Ok(feature.id() != other_feature.id()));
        } else if unknown || unknown_feature {
            checks.push(Err(QueryError::unsupported(other.id(), "Slot definingFeature equality is unresolved")));
        }
    }
    match require_all(checks)? {
        true => Ok(()),
        false => Err(AssignmentError::Invalid(
            "Slot assignment violates definingFeature membership, uniqueness or value compatibility".into(),
        )),
    }
}

fn instance_reference_evidence<'a>(
    references: &ReferenceIndex<'a>,
    owner: ElementRef<'a>,
    metaclass: &str,
    property: &str,
    value: &Value,
) -> Result<(Vec<ElementRef<'a>>, bool), QueryError> {
    let descriptor = schema::uml_property(metaclass, property)
        .map_err(|error| QueryError::unsupported(owner.id(), error.to_string()))?
        .ok_or_else(|| QueryError::unsupported(owner.id(), "instance property contract is unavailable"))?;
    match super::property_contracts::check_value(references, owner, descriptor, value) {
        super::property_contracts::PropertyOutcome::Violation(message) => return Err(QueryError::invalid(owner.id(), message)),
        _ => {}
    }
    let values = if matches!(descriptor.upper, "0" | "1") {
        std::slice::from_ref(value)
    } else {
        value.as_array().ok_or_else(|| QueryError::invalid(owner.id(), "reference collection must be an array"))?
    };
    let mut known = Vec::new();
    let mut unknown = false;
    for value in values {
        if super::references::external_reference_href(value)
            .map_err(|error| QueryError::invalid(owner.id(), error))?.is_some()
        {
            unknown = true;
        } else if let Some(id) = reference_id(value).map_err(|error| QueryError::invalid(owner.id(), error))? {
            let target = references.element(id).map_err(|error| QueryError::invalid(owner.id(), error))?;
            unknown |= target.metaclass().is_none();
            known.push(target);
        }
    }
    Ok((known, unknown))
}

fn slottable_features<'a>(
    references: &ReferenceIndex<'a>, classifier: ElementRef<'_>,
) -> Result<Vec<ElementRef<'a>>, QueryError> {
    if classifier.metaclass().is_none() {
        return Err(QueryError::unsupported(classifier.id(), "classifier metaclass is unresolved"));
    }
    classifiers::query(references, classifier.id()).and_then(|query| {
        query.classifier.ok_or_else(|| QueryError::unsupported(classifier.id(), "Classifier query is unresolved"))?
            .all_slottable_features
    })
}

fn slottable_membership(
    references: &ReferenceIndex<'_>, classifier: ElementRef<'_>, feature_id: &str,
) -> Result<bool, QueryError> {
    let complete = slottable_features(references, classifier);
    match complete {
        Ok(features) => Ok(features.iter().any(|feature| feature.id() == feature_id)),
        Err(error @ QueryError::Invalid { .. }) => Err(error),
        // has_feature also requires a known Namespace metaclass. An unresolved
        // classifier cannot enter that fallback without becoming falsely Invalid.
        Err(error) if classifier.metaclass().is_none() => Err(error),
        Err(error) => match classifiers::has_feature(references, classifier.id(), feature_id) {
            // A known StructuralFeature in allFeatures is also in member and
            // therefore allSlottableFeatures. Do not infer the converse.
            Ok(true) => Ok(true),
            Err(error @ QueryError::Invalid { .. }) => Err(error),
            _ => Err(error),
        },
    }
}

fn instance_slots_evidence<'a>(
    references: &ReferenceIndex<'a>, instance: ElementRef<'a>,
) -> Result<(Vec<ElementRef<'a>>, bool), QueryError> {
    let children = references.explicit_owned_elements(instance.id())
        .map_err(|error| QueryError::invalid(instance.id(), error))?;
    let slots = children.iter().copied().filter(|child| child.conforms_to("Slot")).collect::<Vec<_>>();
    let mut unknown_slot = children.iter().any(|child| child.metaclass().is_none());
    if let Some(value) = instance.properties().get("slot") {
        let (authored, unknown) = instance_reference_evidence(references, instance, "InstanceSpecification", "slot", value)?;
        unknown_slot |= unknown;
        let authored_ids = authored.iter().map(|slot| slot.id()).collect::<HashSet<_>>();
        if !unknown && slots.iter().any(|slot| !authored_ids.contains(slot.id())) {
            return Err(QueryError::invalid(instance.id(), "slot projection omits canonical owned Slots"));
        }
    }
    Ok((slots, unknown_slot))
}

fn instance_structural_feature(
    references: &ReferenceIndex<'_>, instance: ElementRef<'_>,
) -> Result<bool, QueryError> {
    let instance = references.element(instance.id())
        .map_err(|error| QueryError::invalid(instance.id(), error))?;
    let classifier_value = instance.properties().get("classifier").cloned().unwrap_or_else(|| serde_json::json!([]));
    let (classifiers, unknown_classifier) = instance_reference_evidence(references, instance, "InstanceSpecification", "classifier", &classifier_value)?;
    let (slots, unknown_slot) = instance_slots_evidence(references, instance)?;
    let mut counts = std::collections::BTreeMap::<String, usize>::new();
    let mut unknown_feature = false;
    for slot in &slots {
        let value = slot.properties().get("definingFeature").unwrap_or(&Value::Null);
        let (features, unknown) = instance_reference_evidence(references, *slot, "Slot", "definingFeature", value)?;
        unknown_feature |= unknown;
        for feature in features {
            *counts.entry(feature.id().to_owned()).or_default() += 1;
        }
    }
    // The rule counts Slots only, not their values. Known singleton collections
    // and an empty classifier collection satisfy the quantification directly.
    if classifiers.is_empty() && !unknown_classifier
        || slots.len() <= 1 && !unknown_slot
        || !unknown_slot && !unknown_feature && counts.values().all(|count| *count <= 1)
    {
        return Ok(true);
    }
    let mut checks = Vec::new();
    if unknown_slot || unknown_feature || unknown_classifier {
        checks.push(Err(QueryError::unsupported(instance.id(), "unresolved Slot, definingFeature or classifier may affect duplicate feature counts")));
    }
    for (feature_id, count) in counts {
        if count <= 1 { continue; }
        for classifier in &classifiers {
            checks.push(slottable_membership(references, *classifier, &feature_id).map(|member| !member));
        }
    }
    super::feature_compatibility::require_all(checks)
}

fn instance_defining_feature(
    references: &ReferenceIndex<'_>, instance: ElementRef<'_>,
) -> Result<bool, QueryError> {
    use super::feature_compatibility::{accept_any, require_all};
    let instance = references.element(instance.id())
        .map_err(|error| QueryError::invalid(instance.id(), error))?;
    let classifier_value = instance.properties().get("classifier").cloned().unwrap_or_else(|| serde_json::json!([]));
    let (classifiers, unknown_classifier) = instance_reference_evidence(references, instance, "InstanceSpecification", "classifier", &classifier_value)?;
    let (slots, unknown_slot) = instance_slots_evidence(references, instance)?;
    let mut checks = Vec::new();
    if unknown_slot {
        checks.push(Err(QueryError::unsupported(instance.id(), "unresolved owned Slot membership")));
    }
    for slot in slots {
        let value = slot.properties().get("definingFeature").unwrap_or(&Value::Null);
        let (features, _) = instance_reference_evidence(references, slot, "Slot", "definingFeature", value)?;
        let mut candidates = Vec::new();
        if unknown_classifier {
            candidates.push(Err(QueryError::unsupported(instance.id(), "external or unresolved classifier may supply the definingFeature")));
        }
        for classifier in &classifiers {
            let membership = if let Some(feature) = features.first() {
                slottable_membership(references, *classifier, feature.id())
            } else {
                // An unresolved feature cannot belong to a proven empty set.
                // Otherwise its identity is needed to decide membership.
                slottable_features(references, *classifier).and_then(|features| {
                    if features.is_empty() { Ok(false) }
                    else { Err(QueryError::unsupported(slot.id(), "external definingFeature identity is unresolved")) }
                })
            };
            candidates.push(membership);
        }
        checks.push(accept_any(candidates));
    }
    require_all(checks)
}

pub(crate) fn validate<'a>(
    model: Model<'a>,
    references: &ReferenceIndex<'a>,
) -> Vec<StructuralDiagnostic> {
    let mut issues: Vec<_> = model
        .elements()
        .filter_map(InstanceSpecificationRef::from_element)
        .filter_map(|instance| {
            validate_instance(instance, references)
                .err()
                .map(|error| StructuralDiagnostic {
                    severity: match &error {
                        AssignmentError::Invalid(_) => StructuralSeverity::Error,
                        AssignmentError::Unsupported(_) => StructuralSeverity::Warning,
                    },
                    code: match &error {
                        AssignmentError::Invalid(_) => "UML_INSTANCE_CLASSIFIER",
                        AssignmentError::Unsupported(_) => "MDE_INSTANCE_CAPABILITY_UNSUPPORTED",
                    },
                    subject_id: instance.id().into(),
                    message: error.to_string(),
                })
        })
        .collect();
    for instance in model.elements().filter_map(InstanceSpecificationRef::from_element) {
        let defining = match instance_defining_feature(references, instance.0) {
            Ok(true) => None,
            Ok(false) => Some((StructuralSeverity::Error, "UML_INSTANCE_DEFINING_FEATURE", "each owned Slot must refer to a slottable feature of at least one instance Classifier".to_owned())),
            Err(QueryError::Invalid { message, .. }) => Some((StructuralSeverity::Error, "MDE_INSTANCE_DEFINING_FEATURE_INVALID", message)),
            Err(QueryError::Unsupported { message, .. }) => Some((StructuralSeverity::Warning, "MDE_INSTANCE_DEFINING_FEATURE_UNVERIFIED", message)),
        };
        if let Some((severity, code, message)) = defining {
            issues.push(StructuralDiagnostic { severity, code, subject_id: instance.id().into(),
                message: format!("InstanceSpecification-defining_feature: {message}") });
        }
        let outcome = match instance_structural_feature(references, instance.0) {
            Ok(true) => None,
            Ok(false) => Some((StructuralSeverity::Error, "UML_INSTANCE_STRUCTURAL_FEATURE", "each slottable feature may define at most one owned Slot".to_owned())),
            Err(QueryError::Invalid { message, .. }) => Some((StructuralSeverity::Error, "MDE_INSTANCE_STRUCTURAL_FEATURE_INVALID", message)),
            Err(QueryError::Unsupported { message, .. }) => Some((StructuralSeverity::Warning, "MDE_INSTANCE_STRUCTURAL_FEATURE_UNVERIFIED", message)),
        };
        if let Some((severity, code, message)) = outcome {
            issues.push(StructuralDiagnostic { severity, code, subject_id: instance.id().into(),
                message: format!("InstanceSpecification-structural_feature: {message}") });
        }
    }
    issues.extend(
        model
            .elements()
            .filter_map(SlotRef::from_element)
            .filter_map(|slot| {
                let result = slot
                    .owning_instance(model)
                    .map_err(AssignmentError::Invalid)
                    .and_then(|owner| validate_assignment_with(slot, owner, model, references));
                result.err().map(|error| StructuralDiagnostic {
                    severity: match &error {
                        AssignmentError::Invalid(_) => StructuralSeverity::Error,
                        AssignmentError::Unsupported(_) => StructuralSeverity::Warning,
                    },
                    code: match &error {
                        AssignmentError::Invalid(_) => "UML_SLOT_CONTRACT",
                        AssignmentError::Unsupported(_) => "MDE_SLOT_CAPABILITY_UNSUPPORTED",
                    },
                    subject_id: slot.id().into(),
                    message: error.to_string(),
                })
            }),
    );
    issues
}
