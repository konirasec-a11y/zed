//! Executable structural contract queries over generated, source-qualified XMI metadata.
//! These are borrowed schema views, not another store of user-model Elements.

use super::{
    metamodel,
    root::ElementRef,
    types::{Integer, StandardPrimitiveType, UnlimitedNatural},
};
use crate::{StructuralDiagnostic, StructuralSeverity};
use serde_json::Value;
use std::{
    collections::{BTreeSet, HashMap},
    sync::{LazyLock, OnceLock},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Document {
    Uml,
    PrimitiveTypes,
    StandardProfile,
    UmlDi,
    PrimitiveTypes20131001,
    Uml20131001,
    Di,
    Dc,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct XmiId {
    pub document: Document,
    pub fragment: &'static str,
}

impl XmiId {
    pub const fn new(document: Document, fragment: &'static str) -> Self {
        Self { document, fragment }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReferenceResolution {
    Exact,
    ExternalDocument,
    MissingIdentifier,
    AmbiguousIdentifier,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XmiReference {
    pub raw: &'static str,
    pub target: Option<XmiId>,
    pub resolution: ReferenceResolution,
}

#[derive(Clone, Copy, Debug)]
pub struct SourceDescriptor {
    pub document: Document,
    pub path: &'static str,
    pub url: &'static str,
    pub sha256: &'static str,
    pub resolved_references: usize,
    pub unresolved_references: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct UnresolvedReference {
    pub owner: XmiId,
    pub role: &'static str,
    pub reference: XmiReference,
}

#[derive(Clone, Copy, Debug)]
pub struct AmbiguousDefinition {
    pub id: XmiId,
    pub owners: &'static [XmiId],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClassifierKind {
    Class,
    Stereotype,
    Primitive,
    Enumeration,
    Association,
    Extension,
    DataType,
}

#[derive(Clone, Copy, Debug)]
pub struct ClassifierDescriptor {
    pub id: XmiId,
    pub name: &'static str,
    pub kind: ClassifierKind,
    pub is_abstract: bool,
    pub generals: &'static [XmiReference],
    pub owned_properties: &'static [XmiId],
    pub owned_operations: &'static [XmiReference],
    pub member_ends: &'static [XmiReference],
    pub literal_names: &'static [&'static str],
    pub literal_ids: &'static [XmiId],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Aggregation {
    None,
    Shared,
    Composite,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DefaultValue {
    Boolean(bool),
    Integer(&'static str),
    UnlimitedNatural(&'static str),
    Real(&'static str),
    String(&'static str),
    Enumeration { literal: XmiId, name: &'static str },
    Unsupported(XmiId),
}

#[derive(Clone, Copy, Debug)]
pub struct PropertyDescriptor {
    pub id: XmiId,
    pub owner: XmiId,
    pub name: &'static str,
    pub property_type: XmiReference,
    pub visibility: Option<&'static str>,
    // Decimal bounds preserve UML Integer/UnlimitedNatural rather than a machine limit.
    pub lower: &'static str,
    pub upper: &'static str,
    pub default: Option<DefaultValue>,
    pub is_derived: bool,
    pub is_derived_union: bool,
    pub is_read_only: bool,
    pub is_ordered: bool,
    pub is_unique: bool,
    pub aggregation: Aggregation,
    pub association: Option<XmiReference>,
    pub opposites: &'static [XmiReference],
    pub subsets: &'static [XmiReference],
    pub redefines: &'static [XmiReference],
}

/// The generator does not compile these specifications; handwritten rules are separate.
#[derive(Clone, Copy, Debug)]
pub struct ConstraintDescriptor {
    pub id: XmiId,
    pub owner: XmiId,
    pub name: &'static str,
    pub specification: Option<XmiId>,
    pub constrained_elements: &'static [XmiReference],
    pub languages: &'static [&'static str],
}

#[derive(Clone, Copy, Debug)]
pub struct OperationDescriptor {
    pub id: XmiId,
    pub owner: XmiId,
    pub name: &'static str,
    pub is_query: bool,
    pub is_abstract: bool,
    pub visibility: Option<&'static str>,
    pub parameters: &'static [XmiReference],
    pub redefines: &'static [XmiReference],
    pub preconditions: &'static [XmiReference],
    pub postconditions: &'static [XmiReference],
    pub body_conditions: &'static [XmiReference],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParameterDirection {
    In,
    Out,
    InOut,
    Return,
}

#[derive(Clone, Copy, Debug)]
pub struct ParameterDescriptor {
    pub id: XmiId,
    pub owner: XmiId,
    pub name: &'static str,
    pub parameter_type: Option<XmiReference>,
    pub visibility: Option<&'static str>,
    pub direction: ParameterDirection,
    pub lower: &'static str,
    pub upper: &'static str,
    pub default: Option<DefaultValue>,
    pub is_ordered: bool,
    pub is_unique: bool,
    pub is_exception: bool,
    pub is_stream: bool,
    pub effect: Option<&'static str>,
}

/// Bodies and their declared languages are preserved in source order, not interpreted.
#[derive(Clone, Copy, Debug)]
pub struct SpecificationDescriptor {
    pub id: XmiId,
    pub owner: XmiId,
    pub name: Option<&'static str>,
    pub bodies: &'static [&'static str],
    pub languages: &'static [&'static str],
}

#[derive(Clone, Copy, Debug)]
pub struct DocumentationDescriptor {
    pub id: XmiId,
    pub owner: XmiId,
    pub annotated_elements: &'static [XmiReference],
    pub body: Option<&'static str>,
}

#[derive(Clone, Debug, Default)]
pub struct InheritedProperties {
    pub properties: Vec<&'static PropertyDescriptor>,
    pub unresolved: Vec<XmiReference>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum LookupError {
    #[error("the classifier is not a generated normative definition")]
    UnknownClassifier,
    #[error("property inheritance depends on unresolved normative references")]
    IncompleteInheritance,
    #[error("multiple inherited properties have this name without an explicit redefinition")]
    AmbiguousProperty,
    #[error("redefined properties provide conflicting inherited defaults")]
    AmbiguousDefault,
}

struct ClassifierQueries {
    descriptor: &'static ClassifierDescriptor,
    ancestors: OnceLock<Ancestors>,
    properties: OnceLock<Result<InheritedProperties, LookupError>>,
}

struct Ancestors {
    ids: BTreeSet<XmiId>,
    incomplete: bool,
}

// Only immutable normative descriptors are indexed here, never mutable user-model IDs.
// Keep the first definition, matching the source-order lookup if the input repeats an ID.
static CLASSIFIER_INDEX: LazyLock<HashMap<XmiId, ClassifierQueries>> = LazyLock::new(|| {
    let mut index = HashMap::with_capacity(metamodel::CLASSIFIERS.len());
    for descriptor in metamodel::CLASSIFIERS {
        index
            .entry(descriptor.id)
            .or_insert_with(|| ClassifierQueries {
                descriptor,
                ancestors: OnceLock::new(),
                properties: OnceLock::new(),
            });
    }
    index
});

static UML_CLASSIFIER_INDEX: LazyLock<HashMap<&'static str, &'static ClassifierDescriptor>> =
    LazyLock::new(|| {
        let mut index = HashMap::new();
        for descriptor in metamodel::CLASSIFIERS {
            if descriptor.id.document == Document::Uml && descriptor.kind == ClassifierKind::Class {
                index.entry(descriptor.name).or_insert(descriptor);
            }
        }
        index
    });

static PROPERTY_INDEX: LazyLock<HashMap<XmiId, &'static PropertyDescriptor>> =
    LazyLock::new(|| {
        let mut index = HashMap::with_capacity(metamodel::PROPERTIES.len());
        for descriptor in metamodel::PROPERTIES {
            index.entry(descriptor.id).or_insert(descriptor);
        }
        index
    });

pub fn classifier(id: XmiId) -> Option<&'static ClassifierDescriptor> {
    CLASSIFIER_INDEX.get(&id).map(|queries| queries.descriptor)
}

pub fn uml_classifier(name: &str) -> Option<&'static ClassifierDescriptor> {
    UML_CLASSIFIER_INDEX.get(name).copied()
}

pub fn property_by_id(id: XmiId) -> Option<&'static PropertyDescriptor> {
    PROPERTY_INDEX.get(&id).copied()
}

pub fn operation_by_id(id: XmiId) -> Option<&'static OperationDescriptor> {
    metamodel::OPERATIONS
        .iter()
        .find(|descriptor| descriptor.id == id)
}

pub fn parameter_by_id(id: XmiId) -> Option<&'static ParameterDescriptor> {
    metamodel::PARAMETERS
        .iter()
        .find(|descriptor| descriptor.id == id)
}

pub fn specification_by_id(id: XmiId) -> Option<&'static SpecificationDescriptor> {
    metamodel::SPECIFICATIONS
        .iter()
        .find(|descriptor| descriptor.id == id)
}

pub fn documentation_for(id: XmiId) -> impl Iterator<Item = &'static DocumentationDescriptor> {
    metamodel::DOCUMENTATION
        .iter()
        .filter(move |documentation| {
            documentation.owner == id
                || documentation
                    .annotated_elements
                    .iter()
                    .any(|reference| reference.target == Some(id))
        })
}

pub fn conforms_to(specific: XmiId, general: XmiId) -> Result<bool, LookupError> {
    let queries = CLASSIFIER_INDEX
        .get(&specific)
        .ok_or(LookupError::UnknownClassifier)?;
    classifier(general).ok_or(LookupError::UnknownClassifier)?;
    let ancestors = queries.ancestors.get_or_init(|| {
        let mut pending = vec![queries.descriptor];
        let mut ancestors = Ancestors {
            ids: BTreeSet::new(),
            incomplete: false,
        };
        while let Some(descriptor) = pending.pop() {
            if !ancestors.ids.insert(descriptor.id) {
                continue;
            }
            for reference in descriptor.generals {
                match reference.target.and_then(classifier) {
                    Some(parent) => pending.push(parent),
                    None => ancestors.incomplete = true,
                }
            }
        }
        ancestors
    });
    // A known path still proves conformance when another branch is unresolved.
    if ancestors.ids.contains(&general) {
        Ok(true)
    } else if ancestors.incomplete {
        Err(LookupError::IncompleteInheritance)
    } else {
        Ok(false)
    }
}

pub fn inherited_properties(id: XmiId) -> Result<InheritedProperties, LookupError> {
    cached_inherited_properties(id).cloned()
}

fn cached_inherited_properties(id: XmiId) -> Result<&'static InheritedProperties, LookupError> {
    let queries = CLASSIFIER_INDEX
        .get(&id)
        .ok_or(LookupError::UnknownClassifier)?;
    queries
        .properties
        .get_or_init(|| collect_inherited_properties(queries.descriptor))
        .as_ref()
        .map_err(|error| *error)
}

fn collect_inherited_properties(
    descriptor: &'static ClassifierDescriptor,
) -> Result<InheritedProperties, LookupError> {
    let mut result = InheritedProperties::default();
    let mut pending = vec![descriptor];
    let mut visited = BTreeSet::new();
    while let Some(descriptor) = pending.pop() {
        if !visited.insert(descriptor.id) {
            continue;
        }
        for id in descriptor.owned_properties {
            result
                .properties
                .push(property_by_id(*id).ok_or(LookupError::IncompleteInheritance)?);
        }
        for reference in descriptor.generals {
            match reference.target.and_then(classifier) {
                Some(general) => pending.push(general),
                None => result.unresolved.push(*reference),
            }
        }
    }
    let mut redefined = BTreeSet::new();
    let mut pending = result
        .properties
        .iter()
        .flat_map(|property| property.redefines)
        .copied()
        .collect::<Vec<_>>();
    while let Some(reference) = pending.pop() {
        match reference.target.and_then(property_by_id) {
            Some(property) if redefined.insert(property.id) => pending.extend(property.redefines),
            Some(_) => {}
            None => result.unresolved.push(reference),
        }
    }
    // Equal names alone do not replace properties; subsetting also does not redefine them (§9.5.3).
    result
        .properties
        .retain(|property| !redefined.contains(&property.id));
    result.properties.sort_by_key(|property| property.id);
    result.properties.dedup_by_key(|property| property.id);
    Ok(result)
}

pub fn property(id: XmiId, name: &str) -> Result<Option<&'static PropertyDescriptor>, LookupError> {
    let inherited = cached_inherited_properties(id)?;
    if !inherited.unresolved.is_empty() {
        return Err(LookupError::IncompleteInheritance);
    }
    let mut candidates = inherited
        .properties
        .iter()
        .copied()
        .filter(|property| property.name == name);
    let candidate = candidates.next();
    if candidates.next().is_some() {
        return Err(LookupError::AmbiguousProperty);
    }
    Ok(candidate)
}

pub fn uml_property(
    metaclass: &str,
    name: &str,
) -> Result<Option<&'static PropertyDescriptor>, LookupError> {
    property(
        uml_classifier(metaclass)
            .ok_or(LookupError::UnknownClassifier)?
            .id,
        name,
    )
}

pub fn effective_default(
    property: &PropertyDescriptor,
) -> Result<Option<DefaultValue>, LookupError> {
    let mut pending = vec![property];
    let mut visited = BTreeSet::new();
    let mut result = None;
    while let Some(property) = pending.pop() {
        if !visited.insert(property.id) {
            continue;
        }
        if let Some(default) = property.default {
            if result.is_some_and(|previous| previous != default) {
                return Err(LookupError::AmbiguousDefault);
            }
            result = Some(default);
        } else {
            for reference in property.redefines {
                pending.push(
                    reference
                        .target
                        .and_then(property_by_id)
                        .ok_or(LookupError::IncompleteInheritance)?,
                );
            }
        }
    }
    Ok(result)
}

impl DefaultValue {
    pub fn scalar_json(self) -> Option<Value> {
        Some(match self {
            Self::Boolean(value) => Value::Bool(value),
            Self::String(value) | Self::Enumeration { name: value, .. } => {
                Value::String(value.into())
            }
            // Decimal strings preserve integers outside serde_json's native integer range.
            Self::Integer(value) | Self::UnlimitedNatural(value) => Value::String(value.into()),
            Self::Real(value) => value
                .parse::<serde_json::Number>()
                .ok()
                .map(Value::Number)?,
            Self::Unsupported(_) => return None,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScalarKind {
    Primitive(StandardPrimitiveType),
    Enumeration(&'static [&'static str]),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum UnsupportedContract {
    #[error("the property Type is unresolved or not a generated classifier")]
    UnresolvedType,
    #[error("reference-valued properties require a canonical reference adapter")]
    Reference,
    #[error("collection-valued properties require a collection adapter")]
    Collection,
    #[error("derived properties require their handwritten derivation/setter")]
    Derivation,
}

pub fn scalar_kind(property: &PropertyDescriptor) -> Result<ScalarKind, UnsupportedContract> {
    if !matches!(property.upper, "0" | "1") {
        return Err(UnsupportedContract::Collection);
    }
    let descriptor = property
        .property_type
        .target
        .and_then(classifier)
        .ok_or(UnsupportedContract::UnresolvedType)?;
    if descriptor.kind == ClassifierKind::Enumeration {
        return Ok(ScalarKind::Enumeration(descriptor.literal_names));
    }
    if descriptor.kind == ClassifierKind::Primitive
        && matches!(
            descriptor.id.document,
            Document::PrimitiveTypes | Document::PrimitiveTypes20131001
        )
        && let Some(primitive) = StandardPrimitiveType::ALL
            .into_iter()
            .find(|primitive| primitive.name() == descriptor.name)
    {
        return Ok(ScalarKind::Primitive(primitive));
    }
    Err(UnsupportedContract::Reference)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ValueError {
    #[error("a value is required by the property's lower bound")]
    Required,
    #[error("the property's upper bound is zero")]
    Empty,
    #[error("expected a Boolean")]
    Boolean,
    #[error("expected a String")]
    String,
    #[error("expected an exact decimal Integer (a decimal string preserves unbounded values)")]
    Integer,
    #[error("expected a nonnegative exact Integer or '*' for UnlimitedNatural")]
    UnlimitedNatural,
    #[error("expected a finite JSON Real number")]
    Real,
    #[error("expected a declared enumeration literal")]
    Enumeration,
}

pub fn validate_scalar(
    property: &PropertyDescriptor,
    value: &Value,
) -> Result<Result<(), ValueError>, UnsupportedContract> {
    let kind = scalar_kind(property)?;
    if value.is_null() {
        return Ok(if property.lower == "0" {
            Ok(())
        } else {
            Err(ValueError::Required)
        });
    }
    if property.upper == "0" {
        return Ok(Err(ValueError::Empty));
    }
    let decimal = || match value {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        _ => None,
    };
    Ok(match kind {
        ScalarKind::Primitive(StandardPrimitiveType::Boolean) => {
            value.is_boolean().then_some(()).ok_or(ValueError::Boolean)
        }
        ScalarKind::Primitive(StandardPrimitiveType::String) => {
            value.is_string().then_some(()).ok_or(ValueError::String)
        }
        ScalarKind::Primitive(StandardPrimitiveType::Integer) => decimal()
            .and_then(|text| text.parse::<Integer>().ok())
            .map(|_| ())
            .ok_or(ValueError::Integer),
        ScalarKind::Primitive(StandardPrimitiveType::UnlimitedNatural) => decimal()
            .and_then(|text| text.parse::<UnlimitedNatural>().ok())
            .map(|_| ())
            .ok_or(ValueError::UnlimitedNatural),
        ScalarKind::Primitive(StandardPrimitiveType::Real) => value
            .as_f64()
            .filter(|value| value.is_finite())
            .map(|_| ())
            .ok_or(ValueError::Real),
        ScalarKind::Enumeration(literals) => value
            .as_str()
            .filter(|value| literals.contains(value))
            .map(|_| ())
            .ok_or(ValueError::Enumeration),
    })
}

pub fn validate_element(element: ElementRef<'_>) -> Vec<StructuralDiagnostic> {
    let Some(metaclass) = element.metaclass() else {
        return Vec::new();
    };
    element
        .properties()
        .iter()
        .filter_map(|(name, value)| {
            let descriptor = match uml_property(metaclass, name) {
                Ok(Some(descriptor)) => descriptor,
                _ => return None,
            };
            // References/collections and derived consistency are intentionally not scalar validation.
            let Err(error) = validate_scalar(descriptor, value).ok()? else {
                return None;
            };
            Some(StructuralDiagnostic {
                severity: StructuralSeverity::Error,
                code: "UML_PROPERTY_VALUE",
                subject_id: element.id().into(),
                message: format!("{metaclass}::{name}: {error}"),
            })
        })
        .collect()
}

/// These strings are editor declarations, not serialized UML Type/ValueSpecification identities.
pub fn uses_legacy_text_adapter(property: &PropertyDescriptor, value: &Value) -> bool {
    matches!(property.name, "type" | "defaultValue")
        && value.is_string()
        && matches!(scalar_kind(property), Err(UnsupportedContract::Reference))
}

pub fn validate_edit(metaclass: &str, name: &str, value: &Value) -> Result<(), String> {
    let Some(property) = uml_property(metaclass, name).map_err(|error| error.to_string())? else {
        return Ok(());
    };
    if property.is_read_only {
        return Err(format!(
            "{} is read-only in the UML metamodel",
            property.name
        ));
    }
    if property.is_derived {
        return Err(format!(
            "{}: {}",
            property.name,
            UnsupportedContract::Derivation
        ));
    }
    if property.name == "type" {
        super::references::reference_id(value)?;
        return Ok(());
    }
    if uses_legacy_text_adapter(property, value) {
        return Ok(());
    }
    match validate_scalar(property, value) {
        Ok(result) => result.map_err(|error| format!("{}: {error}", property.name)),
        // Other reference/collection adapters retain their own contract.
        Err(_) => Ok(()),
    }
}

#[cfg(test)]
#[path = "schema_tests.rs"]
mod tests;
