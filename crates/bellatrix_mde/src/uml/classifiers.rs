// SPDX-License-Identifier: Apache-2.0
// Copyright 2019-2026 Starion Group S.A.
// Adapted query/fixture patterns from uml4net at 8ef13af2c68ae1994c8fa4f5b3e231dee7e6715d:
// uml4net/Extend/{Namespace,Classifier}Extensions.cs. See the retained license and
// attribution in docs/bellatrix/references/uml4net/. The graph traversal and
// storage adapters below are new; no upstream runtime or object lists are used.

use super::{
    classifier_hierarchy::HierarchyQueries,
    references::{ReferenceIndex, reference_id},
    root::{ElementRef, VisibilityKind},
    schema::{self, Aggregation, Document, PropertyDescriptor, XmiId},
};
use crate::{ModelingSection, StructuralDiagnostic, StructuralSeverity};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum QueryError {
    #[error("Invalid UML namespace {subject_id}: {message}")]
    Invalid { subject_id: String, message: String },
    #[error("Unsupported UML namespace capability on {subject_id}: {message}")]
    Unsupported { subject_id: String, message: String },
}

impl QueryError {
    pub(crate) fn invalid(subject_id: &str, message: impl Into<String>) -> Self {
        Self::Invalid {
            subject_id: subject_id.into(),
            message: message.into(),
        }
    }

    pub(crate) fn unsupported(subject_id: &str, message: impl Into<String>) -> Self {
        Self::Unsupported {
            subject_id: subject_id.into(),
            message: message.into(),
        }
    }

    fn diagnostic(self) -> StructuralDiagnostic {
        let severity = if matches!(self, Self::Unsupported { .. }) {
            StructuralSeverity::Warning
        } else {
            StructuralSeverity::Error
        };
        let (code, subject_id, message) = match self {
            Self::Invalid {
                subject_id,
                message,
            } => ("UML_NAMESPACE_QUERY", subject_id, message),
            Self::Unsupported {
                subject_id,
                message,
            } => ("UML_NAMESPACE_QUERY_UNSUPPORTED", subject_id, message),
        };
        StructuralDiagnostic {
            severity,
            code,
            subject_id,
            message,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ClassifierQuery<'a> {
    pub parents: Vec<ElementRef<'a>>,
    pub all_parents: Vec<ElementRef<'a>>,
    pub all_slottable_features: Result<Vec<ElementRef<'a>>, QueryError>,
    pub inherited_members: Vec<ElementRef<'a>>,
    /// UML Classifier::feature is direct, unlike allFeatures().
    pub features: Vec<ElementRef<'a>>,
    /// Association::ownedEnd contributes features, but never attributes.
    pub attributes: Vec<ElementRef<'a>>,
    pub all_features: Vec<ElementRef<'a>>,
    pub all_attributes: Vec<ElementRef<'a>>,
}

#[derive(Clone, Debug)]
pub struct NamespaceQuery<'a> {
    pub owned_members: Vec<ElementRef<'a>>,
    pub members: Vec<ElementRef<'a>>,
    pub imported_members: Vec<ElementRef<'a>>,
    pub imported_names: std::collections::BTreeMap<String, Vec<Option<String>>>,
    pub excluded_imports: Vec<ElementRef<'a>>,
    pub public_imported_members: Vec<ElementRef<'a>>,
    /// Present only for Packages; this is Package::visibleMembers().
    pub visible_members: Option<Vec<ElementRef<'a>>>,
    pub classifier: Option<ClassifierQuery<'a>>,
}

fn populated(value: &Value) -> bool {
    !value.is_null() && !value.as_array().is_some_and(Vec::is_empty)
}

pub(crate) fn contributes(
    property: &PropertyDescriptor,
    target: &'static str,
) -> Result<bool, String> {
    let target = XmiId::new(Document::Uml, target);
    let mut pending = vec![property.id];
    let mut visited = HashSet::new();
    while let Some(id) = pending.pop() {
        if id == target {
            return Ok(true);
        }
        if !visited.insert(id) {
            continue;
        }
        let property = schema::property_by_id(id).ok_or("unresolved UML subsetting contract")?;
        // A redefinition retains the constraints of the redefined property. This
        // matters for Operation::ownedParameter and Extension::ownedEnd.
        for reference in property.subsets.iter().chain(property.redefines) {
            pending.push(
                reference
                    .target
                    .ok_or("unresolved UML subsetting contract")?,
            );
        }
    }
    Ok(false)
}

pub(crate) fn properties(
    element: ElementRef<'_>,
) -> Result<Vec<&'static PropertyDescriptor>, QueryError> {
    let descriptor = element
        .metaclass()
        .and_then(schema::uml_classifier)
        .ok_or_else(|| QueryError::invalid(element.id(), "a UML Namespace is required"))?;
    let inherited = schema::inherited_properties(descriptor.id)
        .map_err(|error| QueryError::unsupported(element.id(), error.to_string()))?;
    if !inherited.unresolved.is_empty() {
        return Err(QueryError::unsupported(
            element.id(),
            "unresolved inherited metamodel properties",
        ));
    }
    Ok(inherited.properties)
}

fn matches_type(element: ElementRef<'_>, property: &PropertyDescriptor) -> bool {
    property
        .property_type
        .target
        .and_then(schema::classifier)
        .is_some_and(|required| element.conforms_to(required.name))
}

pub(crate) fn is_owned_member(
    owner: ElementRef<'_>,
    child: ElementRef<'_>,
) -> Result<bool, QueryError> {
    contributes_to_owned_union(owner, child, "Namespace-ownedMember")
}

pub(crate) fn is_owned_attribute(
    owner: ElementRef<'_>,
    child: ElementRef<'_>,
) -> Result<bool, QueryError> {
    contributes_to_owned_union(owner, child, "Classifier-attribute")
}

pub(crate) fn contributes_to_owned_union(
    owner: ElementRef<'_>,
    child: ElementRef<'_>,
    target: &'static str,
) -> Result<bool, QueryError> {
    let mut roles = HashSet::new();
    for property in properties(owner)? {
        if property.aggregation == Aggregation::Composite
            && !property.is_derived
            && matches_type(child, property)
        {
            roles.insert(
                contributes(property, target)
                    .map_err(|error| QueryError::unsupported(owner.id(), error))?,
            );
        }
    }
    if roles.len() > 1 {
        return Err(QueryError::unsupported(
            child.id(),
            "ambiguous composite membership role",
        ));
    }
    roles.into_iter().next().ok_or_else(|| {
        QueryError::unsupported(
            child.id(),
            format!(
                "cannot determine a composite membership role under {}",
                owner.id(),
            ),
        )
    })
}

fn unique<'a>(values: impl IntoIterator<Item = ElementRef<'a>>) -> Vec<ElementRef<'a>> {
    let mut seen = HashSet::new();
    values
        .into_iter()
        .filter(|element| seen.insert(element.id()))
        .collect()
}

struct DirectNamespace<'a> {
    element: ElementRef<'a>,
    owned_members: Vec<ElementRef<'a>>,
    features: Vec<ElementRef<'a>>,
    attributes: Vec<ElementRef<'a>>,
    extra_members: Vec<ElementRef<'a>>,
    parents: Vec<ElementRef<'a>>,
}

struct Queries<'index, 'model> {
    references: &'index ReferenceIndex<'model>,
    completed: HashMap<&'model str, NamespaceQuery<'model>>,
    hierarchy: HierarchyQueries<'index, 'model>,
}

impl<'index, 'model> Queries<'index, 'model> {
    fn new(references: &'index ReferenceIndex<'model>) -> Self {
        Self {
            references,
            completed: HashMap::new(),
            hierarchy: HierarchyQueries::new(references),
        }
    }

    fn element(&self, id: &str) -> Result<ElementRef<'model>, QueryError> {
        self.references
            .element(id)
            .map_err(|error| QueryError::invalid(id, error))
    }

    fn direct(&self, id: &str, complete: bool) -> Result<DirectNamespace<'model>, QueryError> {
        let element = self.element(id)?;
        if !element.conforms_to("Namespace") {
            return Err(QueryError::invalid(id, "a UML Namespace is required"));
        }
        for key in [
            "member",
            "importedMember",
            "ownedMember",
            "feature",
            "attribute",
            "inheritedMember",
            "general",
            "role",
        ] {
            if (complete || key == "ownedMember") && element.properties().contains_key(key) {
                return Err(QueryError::unsupported(
                    id,
                    format!("{key} must be derived from the canonical graph"),
                ));
            }
        }
        if complete
            && element
                .properties()
                .get("generalization")
                .is_some_and(populated)
        {
            return Err(QueryError::unsupported(
                id,
                "generalization must use canonical relationship records",
            ));
        }
        let parents = if complete && element.conforms_to("Classifier") {
            self.hierarchy
                .get(id)
                .ok_or_else(|| QueryError::invalid(id, "missing Classifier ancestry"))?
                .parents
                .clone()
        } else {
            if complete
                && self
                    .references
                    .model()
                    .relationship_records()
                    .any(|relation| relation.kind == "uml.generalization" && relation.from == id)
            {
                return Err(QueryError::invalid(
                    id,
                    "Generalization source must be a Classifier",
                ));
            }
            Vec::new()
        };
        let owned = self
            .references
            .explicit_owned_elements(id)
            .map_err(|error| QueryError::invalid(id, error))?;
        let descriptors = properties(element)?;
        let mut projections = Vec::new();
        for property in descriptors {
            let member = contributes(property, "Namespace-ownedMember")
                .map_err(|error| QueryError::unsupported(id, error))?;
            let feature = contributes(property, "Classifier-feature")
                .map_err(|error| QueryError::unsupported(id, error))?;
            let attribute = contributes(property, "Classifier-attribute")
                .map_err(|error| QueryError::unsupported(id, error))?;
            let contributes_member = contributes(property, "Namespace-member")
                .map_err(|error| QueryError::unsupported(id, error))?;
            if element
                .properties()
                .get(property.name)
                .is_some_and(populated)
                && (member || (complete && (feature || attribute || contributes_member)))
                && !(element.conforms_to("Association") && property.name == "memberEnd")
            {
                return Err(QueryError::unsupported(
                    id,
                    format!("{} has no canonical membership adapter", property.name,),
                ));
            }
            if property.aggregation == Aggregation::Composite && !property.is_derived {
                projections.push((property, member, feature, attribute));
            }
        }
        let mut owned_members = Vec::new();
        let mut features = Vec::new();
        let mut attributes = Vec::new();
        for child in owned {
            let roles = projections
                .iter()
                .filter(|(property, ..)| matches_type(child, property))
                .map(|(_, member, feature, attribute)| {
                    (*member, complete && *feature, complete && *attribute)
                })
                .collect::<HashSet<_>>();
            if roles.len() > 1 {
                return Err(QueryError::unsupported(
                    child.id(),
                    format!(
                        "ambiguous composite membership roles under {}; ownership-slot differentiation is required",
                        element.id(),
                    ),
                ));
            }
            if let Some((member, feature, attribute)) = roles.into_iter().next() {
                if member {
                    owned_members.push(child);
                }
                if feature {
                    features.push(child);
                }
                if attribute {
                    attributes.push(child);
                }
            } else if child.named().is_some() {
                return Err(QueryError::unsupported(
                    child.id(),
                    format!(
                        "cannot determine a composite membership role under {}",
                        element.id(),
                    ),
                ));
            }
        }
        let extra_members = if complete && element.conforms_to("Association") {
            super::associations::member_ends(self.references, id)
                .map_err(|error| {
                    if element.properties().contains_key("memberEnd") {
                        QueryError::invalid(id, error)
                    } else {
                        QueryError::unsupported(id, error)
                    }
                })?
                .into_iter()
                .map(ElementRef::Object)
                .collect()
        } else {
            Vec::new()
        };
        Ok(DirectNamespace {
            element,
            owned_members: unique(owned_members),
            features: unique(features),
            attributes: unique(attributes),
            extra_members,
            parents: unique(parents),
        })
    }

    fn finish(
        &self,
        direct: DirectNamespace<'model>,
    ) -> Result<NamespaceQuery<'model>, QueryError> {
        let mut inherited_members = Vec::new();
        for parent in &direct.parents {
            let query = self
                .completed
                .get(parent.id())
                .ok_or_else(|| QueryError::invalid(parent.id(), "missing parent query"))?;
            for member in &query.members {
                let imported = query
                    .imported_members
                    .iter()
                    .any(|imported| imported.id() == member.id());
                let visible = if imported {
                    query
                        .public_imported_members
                        .iter()
                        .any(|imported| imported.id() == member.id())
                } else {
                    inheritable(*member)?
                };
                if visible {
                    inherited_members.push(*member);
                }
            }
        }
        let ancestry = self.hierarchy.get(direct.element.id());
        let ancestors = ancestry
            .map(|ancestry| ancestry.ancestors.clone())
            .unwrap_or_default();
        let mut redefined = HashSet::new();
        for member in &direct.owned_members {
            for target in redefinitions(self.references, *member)? {
                // Property adds a separate condition: its target must occur in
                // an ancestor's allFeatures, not merely in its attributes.
                // Association::ownedEnd is a Feature with an Association context.
                if member.conforms_to("Property")
                    && !ancestors.iter().any(|ancestor| {
                        self.completed
                            .get(ancestor.id())
                            .and_then(|query| query.classifier.as_ref())
                            .is_some_and(|query| {
                                query
                                    .all_features
                                    .iter()
                                    .any(|feature| feature.id() == target.id())
                            })
                    })
                {
                    return Err(QueryError::invalid(
                        member.id(),
                        "a redefined Property must be inherited from an ancestor Classifier's features",
                    ));
                }
                redefined.insert(target.id());
            }
        }
        inherited_members = unique(inherited_members)
            .into_iter()
            .filter(|member| !redefined.contains(member.id()))
            .collect();
        let imports = super::namespace_imports::resolve_namespace_imports(
            self.references,
            direct.element.id(),
        )?;
        let public_imported_members = imports
            .imported_members
            .iter()
            .copied()
            .filter(|member| {
                imports
                    .visible_members
                    .iter()
                    .any(|visible| visible.id() == member.id())
            })
            .collect();
        let members = unique(
            direct
                .owned_members
                .iter()
                .copied()
                .chain(inherited_members.iter().copied())
                .chain(imports.imported_members.iter().copied())
                .chain(direct.extra_members),
        );
        let member_ids: HashSet<_> = members.iter().map(|member| member.id()).collect();
        let classifier = if direct.element.conforms_to("Classifier") {
            let mut all_attributes = direct.attributes.clone();
            // A depth-first concatenation puts a diamond's common ancestor before
            // the second parent's attributes. Topological order honors 9.9.4.7.
            for ancestor in &ancestors {
                let query = self
                    .completed
                    .get(ancestor.id())
                    .and_then(|query| query.classifier.as_ref())
                    .ok_or_else(|| QueryError::invalid(ancestor.id(), "missing parent query"))?;
                all_attributes.extend(query.attributes.iter().copied());
            }
            let all_slottable_features = self.collect_slottable_features(
                direct.element.id(),
                &members,
                &ancestors,
                &redefined,
            );
            Some(ClassifierQuery {
                parents: direct.parents,
                all_parents: ancestors,
                all_slottable_features,
                inherited_members,
                features: direct.features,
                attributes: direct.attributes,
                all_features: members
                    .iter()
                    .copied()
                    .filter(|member| member.conforms_to("Feature"))
                    .collect(),
                all_attributes: unique(all_attributes)
                    .into_iter()
                    .filter(|member| member_ids.contains(member.id()))
                    .collect(),
            })
        } else {
            None
        };
        let visible_members = direct
            .element
            .conforms_to("Package")
            .then_some(imports.visible_members);
        Ok(NamespaceQuery {
            owned_members: direct.owned_members,
            members,
            imported_members: imports.imported_members,
            imported_names: imports.imported_names,
            excluded_imports: imports.excluded_members,
            public_imported_members,
            visible_members,
            classifier,
        })
    }

    fn collect_slottable_features(
        &self,
        id: &str,
        members: &[ElementRef<'model>],
        ancestors: &[ElementRef<'model>],
        redefined: &HashSet<&str>,
    ) -> Result<Vec<ElementRef<'model>>, QueryError> {
        let mut attributes = Vec::new();
        let mut ancestor_redefinitions = HashSet::new();
        for ancestor in ancestors {
            let query = self
                .completed
                .get(ancestor.id())
                .and_then(|query| query.classifier.as_ref())
                .ok_or_else(|| QueryError::invalid(ancestor.id(), "missing ancestor attributes"))?;
            attributes.extend(query.attributes.iter().copied());
            for attribute in &query.attributes {
                ancestor_redefinitions.extend(
                    redefinitions(self.references, *attribute)?
                        .into_iter()
                        .map(|target| target.id()),
                );
            }
        }
        // The literal allSlottableFeatures body can reintroduce a feature replaced
        // by an intermediate ancestor. Do not invent a transitive suppression rule
        // or certify ambiguous membership against the prose's exclusion contract.
        if attributes.iter().any(|attribute| {
            ancestor_redefinitions.contains(attribute.id()) && !redefined.contains(attribute.id())
        }) {
            return Err(QueryError::unsupported(
                id,
                "ancestor redefinitions make allSlottableFeatures ambiguous; explicit resolution is required",
            ));
        }
        Ok(unique(
            members
                .iter()
                .copied()
                .filter(|member| member.conforms_to("StructuralFeature"))
                .chain(
                    attributes
                        .into_iter()
                        .filter(|attribute| !redefined.contains(attribute.id())),
                ),
        ))
    }

    fn evaluate(&mut self, id: &str) -> Result<NamespaceQuery<'model>, QueryError> {
        let root = self.element(id)?;
        let mut order = if root.conforms_to("Classifier") {
            self.hierarchy
                .resolve(id)?
                .ancestors
                .into_iter()
                .rev()
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        order.push(root);
        for element in order {
            if self.completed.contains_key(element.id()) {
                continue;
            }
            let direct = self.direct(element.id(), true)?;
            let result = self.finish(direct)?;
            self.completed.insert(element.id(), result);
        }
        self.completed
            .get(root.id())
            .cloned()
            .ok_or_else(|| QueryError::invalid(id, "missing namespace query"))
    }
}

pub(crate) fn visibility(element: ElementRef<'_>) -> Result<Option<VisibilityKind>, QueryError> {
    let named = element
        .named()
        .ok_or_else(|| QueryError::invalid(element.id(), "member must be a NamedElement"))?;
    if let Some(value) = element.properties().get("visibility")
        && !value.is_null()
        && value.as_str().and_then(VisibilityKind::from_name).is_none()
    {
        return Err(QueryError::invalid(
            element.id(),
            "visibility must be a VisibilityKind or absent",
        ));
    }
    Ok(named.visibility())
}

fn inheritable(element: ElementRef<'_>) -> Result<bool, QueryError> {
    // UML 9.9.4.7: protected and package are inheritable too; no guessed Java rule.
    Ok(visibility(element)? != Some(VisibilityKind::Private))
}

fn redefinitions<'a>(
    references: &ReferenceIndex<'a>,
    element: ElementRef<'a>,
) -> Result<Vec<ElementRef<'a>>, QueryError> {
    for key in ["redefinedElement", "redefinitionContext"] {
        if element.properties().contains_key(key) {
            return Err(QueryError::unsupported(
                element.id(),
                format!("{key} must be derived from canonical references"),
            ));
        }
    }
    let mut targets = Vec::new();
    for property in properties(element)? {
        if property.is_derived
            || !contributes(property, "RedefinableElement-redefinedElement")
                .map_err(|error| QueryError::unsupported(element.id(), error))?
        {
            continue;
        }
        let Some(value) = element.properties().get(property.name) else {
            continue;
        };
        if !matches!(property.name, "redefinedProperty" | "redefinedOperation") && populated(value)
        {
            return Err(QueryError::unsupported(
                element.id(),
                format!("{} filtering is not implemented", property.name),
            ));
        }
        let values = value.as_array().ok_or_else(|| {
            QueryError::invalid(
                element.id(),
                format!(
                    "{} must be an array of local Element references",
                    property.name,
                ),
            )
        })?;
        let mut seen = HashSet::new();
        for value in values {
            let id = reference_id(value)
                .map_err(|error| QueryError::invalid(element.id(), error))?
                .ok_or_else(|| {
                    QueryError::invalid(element.id(), "redefinition cannot contain null")
                })?;
            if !seen.insert(id) {
                return Err(QueryError::invalid(
                    element.id(),
                    "redefinition references must be unique",
                ));
            }
            let target = references
                .element(id)
                .map_err(|error| QueryError::invalid(element.id(), error))?;
            if !matches_type(target, property) {
                return Err(QueryError::invalid(
                    element.id(),
                    format!("{id} has the wrong metaclass for {}", property.name),
                ));
            }
            if id == element.id() {
                return Err(QueryError::invalid(
                    element.id(),
                    "an Element cannot redefine itself",
                ));
            }
            if target.properties().get("isLeaf").and_then(Value::as_bool) == Some(true) {
                return Err(QueryError::invalid(
                    element.id(),
                    format!("{id} isLeaf prohibits redefinition"),
                ));
            }
            if !super::redefinition_contexts::compare_redefinition_contexts(
                references, element, target,
            )? {
                return Err(QueryError::invalid(
                    element.id(),
                    format!(
                        "redefinition target {} requires a context in an ancestor Classifier",
                        target.id(),
                    ),
                ));
            }
            match compare_redefinition_consistency(references, element, target) {
                Ok(true) | Err(QueryError::Unsupported { .. }) => {}
                Ok(false) => {
                    return Err(QueryError::invalid(
                        element.id(),
                        format!(
                            "{} redefinition of {} is inconsistent with its typed feature contract",
                            element.metaclass().unwrap_or("Element"),
                            target.id()
                        ),
                    ));
                }
                Err(error) => return Err(error),
            }
            targets.push(target);
        }
    }
    Ok(unique(targets))
}

/// The owned union does not depend on imports or the generalization hierarchy.
pub fn owned_members<'a>(
    references: &ReferenceIndex<'a>,
    namespace_id: &str,
) -> Result<Vec<ElementRef<'a>>, QueryError> {
    Ok(Queries::new(references)
        .direct(namespace_id, false)?
        .owned_members)
}

/// One immutable graph snapshot, with local traversal state and no global user-model cache.
/// Unimplemented membership sources return an error, never a partial success.
pub fn query<'a>(
    references: &ReferenceIndex<'a>,
    namespace_id: &str,
) -> Result<NamespaceQuery<'a>, QueryError> {
    Queries::new(references).evaluate(namespace_id)
}

fn compare_redefinition_consistency(
    references: &ReferenceIndex<'_>,
    source: ElementRef<'_>,
    target: ElementRef<'_>,
) -> Result<bool, QueryError> {
    if source.conforms_to("Property") {
        super::feature_compatibility::compare_property_consistency(references, source, target)
    } else {
        super::operation_signatures::compare_operation_consistency(references, source, target)
    }
}

pub(crate) fn redefinition_consistency_diagnostic(
    references: &ReferenceIndex<'_>,
    element: ElementRef<'_>,
) -> Option<StructuralDiagnostic> {
    let key = if element.conforms_to("Property") {
        "redefinedProperty"
    } else if element.conforms_to("Operation") {
        "redefinedOperation"
    } else {
        return None;
    };
    if !element.properties().get(key).is_some_and(populated) {
        return None;
    }
    let targets = match redefinitions(references, element) {
        Ok(targets) => targets,
        Err(error) => return Some(error.diagnostic()),
    };
    let unsupported = targets
        .into_iter()
        .find_map(|target| compare_redefinition_consistency(references, element, target).err());
    let message = unsupported?.to_string();
    Some(StructuralDiagnostic {
        severity: StructuralSeverity::Warning,
        code: "UML_REDEFINITION_CONSISTENCY_UNSUPPORTED",
        subject_id: element.id().into(),
        message,
    })
}

pub fn validate(references: &ReferenceIndex<'_>) -> Vec<StructuralDiagnostic> {
    let mut queries = Queries::new(references);
    let mut issues = Vec::new();
    for element in references.model().elements() {
        let result = (|| {
            if element.conforms_to("ElementImport") || element.conforms_to("PackageImport") {
                super::namespace_imports::validate_import_record(references, element)?;
            }
            if element.conforms_to("RedefinableElement") {
                let targets = redefinitions(references, element)?;
                if !targets.is_empty() {
                    let owner = references
                        .model()
                        .owner(element)
                        .map_err(|error| QueryError::invalid(element.id(), error))?
                        .ok_or_else(|| {
                            QueryError::invalid(
                                element.id(),
                                "redefining member requires a Classifier owner",
                            )
                        })?;
                    queries.evaluate(owner.id())?;
                    if let Some(warning) = redefinition_consistency_diagnostic(references, element)
                    {
                        issues.push(warning);
                    }
                }
            }
            if element.conforms_to("Namespace")
                && let Some(classifier) = queries.evaluate(element.id())?.classifier
            {
                classifier.all_slottable_features?;
            }
            Ok::<_, QueryError>(())
        })();
        if let Err(error) = result {
            let issue = error.diagnostic();
            if !issues.iter().any(|existing: &StructuralDiagnostic| {
                existing.code == issue.code
                    && existing.subject_id == issue.subject_id
                    && existing.message == issue.message
            }) {
                issues.push(issue);
            }
        }
    }
    issues
}

pub(crate) fn set_redefined_elements(
    section: &mut ModelingSection,
    element_id: &str,
    redefined_ids: &[String],
) -> Result<(), String> {
    let mut candidate = section.clone();
    let element = candidate
        .elements
        .iter_mut()
        .find(|element| element.id == element_id)
        .ok_or("redefining Element not found")?;
    let key = if ElementRef::Object(element).conforms_to("Property") {
        "redefinedProperty"
    } else if ElementRef::Object(element).conforms_to("Operation") {
        "redefinedOperation"
    } else {
        return Err("only Property and Operation redefinition is supported".into());
    };
    element.properties.insert(
        key.into(),
        Value::Array(redefined_ids.iter().map(|id| json!({"$ref": id})).collect()),
    );
    let references = ReferenceIndex::new(super::root::Model::from_section(&candidate));
    let element = references.element(element_id)?;
    redefinitions(&references, element).map_err(|error| error.to_string())?;
    let owner = references
        .model()
        .owner(element)?
        .ok_or("redefining member requires a Classifier owner")?;
    query(&references, owner.id()).map_err(|error| error.to_string())?;
    *section = candidate;
    Ok(())
}
