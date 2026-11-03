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

fn populated(value: Option<&Value>) -> bool {
    value.is_some_and(|value| !value.is_null() && !value.as_array().is_some_and(Vec::is_empty))
}

fn validate_instance<'a>(
    instance: InstanceSpecificationRef<'a>,
    model: Model<'a>,
    references: &ReferenceIndex<'a>,
) -> Result<Vec<ElementRef<'a>>, AssignmentError> {
    let classifiers = instance.classifiers_with(references)?;
    if instance.0.properties().contains_key("slot") {
        return Err(AssignmentError::Unsupported(
            "slot must be projected from the canonical ownership edge".into(),
        ));
    }
    if instance
        .0
        .properties()
        .get("specification")
        .is_some_and(|value| !value.is_null())
        || model
            .owned_elements(instance)?
            .iter()
            .any(|element| element.conforms_to("ValueSpecification"))
    {
        return Err(AssignmentError::Unsupported(
            "InstanceSpecification specification values are not yet evaluated".into(),
        ));
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
    if slot.0.properties().contains_key("owningInstance")
        || owner.0.properties().contains_key("slot")
        || model
            .owner(slot)?
            .is_some_and(|previous| previous.properties().contains_key("slot"))
    {
        return Err(AssignmentError::Unsupported(
            "slot and owningInstance must be projected from the canonical ownership edge".into(),
        ));
    }
    let feature = slot.defining_feature_with(references)?;
    let classifiers = validate_instance(owner, model, references)?;
    if populated(slot.0.properties().get("value"))
        || model
            .owned_elements(slot)?
            .iter()
            .any(|element| element.conforms_to("ValueSpecification"))
    {
        return Err(AssignmentError::Unsupported(
            "Slot values are not yet evaluated".into(),
        ));
    }
    if classifiers.is_empty() {
        return Err(AssignmentError::Invalid(
            "A Slot definingFeature requires a classifier of its owning InstanceSpecification"
                .into(),
        ));
    }
    for other in owner.slots(model)? {
        if other.id() != slot.id() && other.defining_feature_with(references)?.id() == feature.id()
        {
            return Err(AssignmentError::Invalid(
                "InstanceSpecification cannot have two Slots with the same definingFeature".into(),
            ));
        }
    }
    let mut unresolved = None;
    for classifier in classifiers {
        let features = classifiers::query(references, classifier.id())
            .and_then(|query| {
                query.classifier.ok_or_else(|| {
                    QueryError::invalid(
                        classifier.id(),
                        "InstanceSpecification classifier must be a Classifier",
                    )
                })
            })
            .and_then(|query| query.all_slottable_features);
        match features {
            Ok(features)
                if features
                    .iter()
                    .any(|candidate| candidate.id() == feature.id()) =>
            {
                return Ok(());
            }
            Ok(_) => {}
            Err(error @ QueryError::Unsupported { .. }) => unresolved = Some(error),
            Err(error) => return Err(error.into()),
        }
    }
    if let Some(error) = unresolved {
        return Err(error.into());
    }
    Err(AssignmentError::Invalid(
        "Slot definingFeature is not a slottable feature of any owning InstanceSpecification classifier".into(),
    ))
}

pub(crate) fn validate<'a>(
    model: Model<'a>,
    references: &ReferenceIndex<'a>,
) -> Vec<StructuralDiagnostic> {
    let mut issues: Vec<_> = model
        .elements()
        .filter_map(InstanceSpecificationRef::from_element)
        .filter_map(|instance| {
            validate_instance(instance, model, references)
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
