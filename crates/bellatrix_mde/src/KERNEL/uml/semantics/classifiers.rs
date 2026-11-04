// SPDX-License-Identifier: Apache-2.0
// Copyright 2019-2026 Starion Group S.A.
// Adapted query/fixture patterns from uml4net at 8ef13af2c68ae1994c8fa4f5b3e231dee7e6715d:
// uml4net/Extend/{Namespace,Classifier}Extensions.cs. See the retained license and
// attribution in docs/bellatrix/references/uml4net/. The graph traversal and
// storage adapters below are new; no upstream runtime or object lists are used.

use super::{
    classifier_hierarchy::HierarchyQueries,
    references::{ReferenceIndex, external_reference_href, reference_id},
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
        .ok_or_else(|| QueryError::unsupported(element.id(), "normative metaclass properties are unavailable"))?;
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

pub(crate) struct OwnershipRoles {
    pub properties: Vec<&'static PropertyDescriptor>,
    pub is_resolved: bool,
}

fn contains_reference(
    value: &Value,
    id: &str,
    subject: &str,
    property: &PropertyDescriptor,
) -> Result<bool, QueryError> {
    let collection = !matches!(property.upper, "0" | "1");
    let values = if collection {
        value.as_array().map(Vec::as_slice).ok_or_else(||
            QueryError::invalid(subject, format!("{} requires a reference collection", property.id.fragment)))?
    } else {
        std::slice::from_ref(value)
    };
    let mut contains = false;
    let mut unknown = None;
    let mut seen = HashSet::new();
    let mut external = HashSet::new();
    for value in values {
        if schema::uses_legacy_text_adapter(property, value) {
            unknown = Some("text declaration is not a resolved UML reference");
            continue;
        }
        if let Some(href) = external_reference_href(value)
            .map_err(|error| QueryError::invalid(subject, error))?
        {
            if property.is_unique && !external.insert(href) {
                return Err(QueryError::invalid(subject, "duplicate external composite reference"));
            }
            unknown = Some("external composite reference has no loaded resolver");
            continue;
        }
        let target = reference_id(value).map_err(|error| QueryError::invalid(subject, error))?;
        match target {
            Some(target) => {
                if property.is_unique && !seen.insert(target) {
                    return Err(QueryError::invalid(subject, "duplicate composite reference"));
                }
                contains |= target == id;
            }
            None if !collection && property.lower == "0" => {}
            None => return Err(QueryError::invalid(subject, "composite reference cannot be null")),
        }
    }
    // Existential membership can be proved without resolving every neighbor;
    // malformed/duplicate entries were still inspected before accepting it.
    if contains { Ok(true) }
    else { unknown.map_or(Ok(false), |message| Err(QueryError::unsupported(subject, message))) }
}

pub(crate) fn ownership_roles(
    owner: ElementRef<'_>,
    child: ElementRef<'_>,
) -> Result<OwnershipRoles, QueryError> {
    let mut explicit = Vec::new();
    let mut candidates = Vec::new();
    let mut excluded_by_explicit_role = false;
    let mut unresolved_type = false;
    let mut unresolved_role = None;
    for property in properties(owner)? {
        if property.aggregation != Aggregation::Composite || property.is_derived {
            continue;
        }
        let type_is_resolved = property.property_type.resolution == schema::ReferenceResolution::Exact
            && property.property_type.target.and_then(schema::classifier).is_some()
            && child.metaclass().is_some();
        let compatible = matches_type(child, property);
        unresolved_type |= !type_is_resolved;
        if let Some(value) = owner.properties().get(property.name) {
            if type_is_resolved && !compatible {
                // An unrelated malformed composite cannot prevent resolving this
                // child's role. A local reference to this child still proves a
                // type violation, even beside another malformed collection item.
                let values = value.as_array().map(Vec::as_slice)
                    .unwrap_or_else(|| std::slice::from_ref(value));
                if values.iter().any(|value|
                    matches!(reference_id(value), Ok(Some(id)) if id == child.id()))
                {
                    return Err(QueryError::invalid(child.id(), format!(
                        "{} has an incompatible composite target", property.id.fragment,
                    )));
                }
                continue;
            }
            let contains = match contains_reference(value, child.id(), owner.id(), property) {
                Ok(contains) => contains,
                Err(error @ QueryError::Invalid { .. }) => return Err(error),
                Err(error @ QueryError::Unsupported { .. }) => {
                    unresolved_role.get_or_insert(error);
                    continue;
                }
            };
            if contains {
                if !type_is_resolved {
                    unresolved_role.get_or_insert_with(|| QueryError::unsupported(owner.id(),
                        "composite ownership role has an unresolved target type"));
                    continue;
                }
                explicit.push(property);
            } else if compatible {
                excluded_by_explicit_role = true;
            }
        } else if compatible {
            candidates.push(property);
        }
    }
    // A known role proves membership, but only after all relevant stored roles
    // have been inspected: unresolved evidence must not mask a proven violation.
    if !explicit.is_empty() {
        return Ok(OwnershipRoles { properties: explicit, is_resolved: true });
    }
    if let Some(error) = unresolved_role {
        return Err(error);
    }
    // A canonical Association memberEnd plus ownership identifies ownedEnd,
    // including AssociationClass, whose ownedAttribute has the same target type.
    if owner.conforms_to("Association") && child.conforms_to("Property")
        && let Some(member_ends) = owner.properties().get("memberEnd")
    {
        let member_end = schema::uml_property("Association", "memberEnd")
            .map_err(|error| QueryError::unsupported(owner.id(), error.to_string()))?
            .ok_or_else(|| QueryError::unsupported(owner.id(), "missing Association::memberEnd contract"))?;
        let is_end = contains_reference(member_ends, child.id(), owner.id(), member_end)?;
        let mut filtered = Vec::new();
        for property in candidates {
            if contributes(property, "Association-ownedEnd")
                .map_err(|error| QueryError::unsupported(owner.id(), error))? == is_end
            {
                filtered.push(property);
            }
        }
        candidates = filtered;
    }
    if candidates.is_empty() && excluded_by_explicit_role && !unresolved_type {
        return Err(QueryError::invalid(child.id(), format!(
            "canonical ownership by {} contradicts its explicit composite role references",
            owner.id(),
        )));
    }
    Ok(OwnershipRoles {
        is_resolved: candidates.len() == 1,
        properties: candidates,
    })
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
    let ownership = ownership_roles(owner, child)?;
    let mut roles = HashSet::new();
    for property in ownership.properties {
        roles.insert(
            contributes(property, target)
                .map_err(|error| QueryError::unsupported(owner.id(), error))?,
        );
    }
    if !ownership.is_resolved && roles.len() > 1 {
        return Err(QueryError::unsupported(
            child.id(),
            "ambiguous composite membership role",
        ));
    }
    if roles.is_empty() {
        return Err(
        QueryError::unsupported(
            child.id(),
            format!(
                "cannot determine a composite membership role under {}",
                owner.id(),
            ),
        ));
    }
    Ok(roles.contains(&true))
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
        super::element_graph_queries::ensure_canonical_ownership(self.references, element)
            .map_err(|error| match error {
                super::element_graph_queries::ElementGraphError::Invalid { subject_id, message } =>
                    QueryError::invalid(&subject_id, message),
                super::element_graph_queries::ElementGraphError::Unsupported { subject_id, message } =>
                    QueryError::unsupported(&subject_id, message),
            })?;
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
        for property in descriptors {
            if element.conforms_to("Association") && property.name == "navigableOwnedEnd"
                && element.properties().get(property.name).is_some_and(populated)
            {
                // This subset selects existing owned ends; it contributes no
                // additional members beyond the canonical ownership projection.
                super::associations::validate_navigation_declarations(self.references, id)
                    .map_err(|error| QueryError::unsupported(id, error))?;
                continue;
            }
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
                && (property.aggregation != Aggregation::Composite || property.is_derived)
                && !(element.conforms_to("Association") && property.name == "memberEnd")
            {
                return Err(QueryError::unsupported(
                    id,
                    format!("{} has no canonical membership adapter", property.name,),
                ));
            }
        }
        let mut owned_members = Vec::new();
        let mut features = Vec::new();
        let mut attributes = Vec::new();
        for child in owned {
            let ownership = ownership_roles(element, child)?;
            let roles = ownership.properties.into_iter().map(|property| {
                let contribution = |target| contributes(property, target)
                    .map_err(|error| QueryError::unsupported(id, error));
                Ok((contribution("Namespace-ownedMember")?,
                    complete && contribution("Classifier-feature")?,
                    complete && contribution("Classifier-attribute")?))
            }).collect::<Result<HashSet<_>, QueryError>>()?;
            if !ownership.is_resolved && roles.len() > 1 {
                return Err(QueryError::unsupported(
                    child.id(),
                    format!(
                        "ambiguous composite membership roles under {}; ownership-slot differentiation is required",
                        element.id(),
                    ),
                ));
            }
            if !roles.is_empty() {
                let (member, feature, attribute) = roles.into_iter().fold(
                    (false, false, false),
                    |left, right| (left.0 || right.0, left.1 || right.1, left.2 || right.2),
                );
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
        members: &[ElementRef<'model>],
        ancestors: &[ElementRef<'model>],
        redefined: &HashSet<&str>,
    ) -> Result<Vec<ElementRef<'model>>, QueryError> {
        let mut attributes = Vec::new();
        for ancestor in ancestors {
            let query = self
                .completed
                .get(ancestor.id())
                .and_then(|query| query.classifier.as_ref())
                .ok_or_else(|| QueryError::invalid(ancestor.id(), "missing ancestor attributes"))?;
            attributes.extend(query.attributes.iter().copied());
        }
        // UML.xmi Classifier-allSlottableFeatures-spec calls self.inherit over
        // allParents().attribute, not each parent's allSlottableFeatures().
        // Classifier-inherit-spec excludes only targets of this classifier's
        // owned members. Thus an intermediate ancestor's replaced attribute can
        // reappear here. The machine-readable normative expression takes
        // precedence over the prose's broader "excluding redefined" wording
        // (UML 2.5.1 clause 2); do not add transitive suppression.
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
        if value.is_null() && property.upper == "1" && property.lower == "0" {
            continue;
        }
        let values = if property.upper == "1" {
            std::slice::from_ref(value)
        } else {
            value.as_array().map(Vec::as_slice).ok_or_else(|| {
                QueryError::invalid(element.id(), format!(
                    "{} must be an array of local Element references", property.name,
                ))
            })?
        };
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
            if schema::read_effective_boolean(target, "isLeaf")
                .map_err(|error| QueryError::invalid(target.id(), error))? {
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

/// UML StructuredClassifier::allRoles filters allFeatures, not allAttributes.
/// Keep the complete query exact: inheritance, imports and redefinition are
/// handled by the existing namespace graph engine.
pub fn all_roles<'a>(
    references: &ReferenceIndex<'a>,
    classifier_id: &str,
) -> Result<Vec<ElementRef<'a>>, QueryError> {
    let classifier = references.element(classifier_id)
        .map_err(|error| QueryError::invalid(classifier_id, error))?;
    if !classifier.conforms_to("StructuredClassifier") {
        return Err(QueryError::invalid(classifier_id, "allRoles requires a StructuredClassifier"));
    }
    let namespace = query(references, classifier_id)?;
    let features = namespace.classifier
        .ok_or_else(|| QueryError::invalid(classifier_id, "missing Classifier query"))?;
    Ok(features.all_features.into_iter()
        .filter(|feature| feature.conforms_to("ConnectableElement")).collect())
}

/// Boolean membership may be proved by an owned member even when the complete
/// namespace has unresolved branches. Never expose that evidence as a complete
/// allRoles collection. Inherited/imported evidence stays conservative whenever
/// the namespace engine cannot settle visibility or redefinition.
pub fn has_role(
    references: &ReferenceIndex<'_>,
    classifier_id: &str,
    role_id: &str,
) -> Result<bool, QueryError> {
    let complete = all_roles(references, classifier_id);
    match &complete {
        Ok(roles) => return Ok(roles.iter().any(|role| role.id() == role_id)),
        Err(error @ QueryError::Invalid { .. }) => return Err(error.clone()),
        Err(_) => {}
    }
    let classifier = references.element(classifier_id)
        .map_err(|error| QueryError::invalid(classifier_id, error))?;
    let role = references.element(role_id)
        .map_err(|error| QueryError::invalid(role_id, error))?;
    // Visit all canonical ownership contracts and all known ancestry branches
    // before accepting direct evidence; malformed siblings/cycles dominate.
    let ownership = super::element_graph_queries::ensure_canonical_ownership(references, classifier);
    if let Err(super::element_graph_queries::ElementGraphError::Invalid { subject_id, message }) = ownership {
        return Err(QueryError::invalid(&subject_id, message));
    }
    if let Err(error @ QueryError::Invalid { .. }) =
        super::classifier_hierarchy::has_ancestor(references, classifier_id, classifier_id)
    { return Err(error); }
    let mut found = false;
    for child in references.explicit_owned_elements(classifier_id)
        .map_err(|error| QueryError::invalid(classifier_id, error))?
    {
        match is_owned_member(classifier, child) {
            Ok(true) if child.id() == role_id && role.conforms_to("Feature")
                && role.conforms_to("ConnectableElement") => found = true,
            Err(error @ QueryError::Invalid { .. }) => return Err(error),
            _ => {}
        }
    }
    if found { Ok(true) } else { complete.map(|_| false) }
}

/// Partial allFeatures membership: a direct canonical Feature remains known
/// when an unrelated inheritance/import branch cannot be enumerated completely.
pub(crate) fn has_feature(
    references: &ReferenceIndex<'_>, classifier_id: &str, feature_id: &str,
) -> Result<bool, QueryError> {
    let complete = query(references, classifier_id).and_then(|query| {
        query.classifier.map(|classifier| classifier.all_features)
            .ok_or_else(|| QueryError::invalid(classifier_id, "allFeatures requires a Classifier"))
    });
    match &complete {
        Ok(features) => return Ok(features.iter().any(|feature| feature.id() == feature_id)),
        Err(error @ QueryError::Invalid { .. }) => return Err(error.clone()),
        Err(_) => {},
    }
    let classifier = references.element(classifier_id)
        .map_err(|error| QueryError::invalid(classifier_id, error))?;
    let feature = references.element(feature_id)
        .map_err(|error| QueryError::invalid(feature_id, error))?;
    if let Err(super::element_graph_queries::ElementGraphError::Invalid { subject_id, message }) =
        super::element_graph_queries::ensure_canonical_ownership(references, classifier) {
        return Err(QueryError::invalid(&subject_id, message));
    }
    if let Err(error @ QueryError::Invalid { .. }) =
        super::classifier_hierarchy::has_ancestor(references, classifier_id, classifier_id) { return Err(error); }
    let mut found = false;
    for child in references.explicit_owned_elements(classifier_id)
        .map_err(|error| QueryError::invalid(classifier_id, error))? {
        match contributes_to_owned_union(classifier, child, "Classifier-feature") {
            Ok(true) if child.id() == feature_id && feature.conforms_to("Feature") => found = true,
            Err(error @ QueryError::Invalid { .. }) => return Err(error),
            _ => {},
        }
    }
    if found { Ok(true) } else { complete.map(|_| false) }
}

fn compare_redefinition_consistency(
    references: &ReferenceIndex<'_>,
    source: ElementRef<'_>,
    target: ElementRef<'_>,
) -> Result<bool, QueryError> {
    // isConsistentWith is invoked on the redefined element, not the redefiner.
    // Resolve the effective override from normative metaclass inheritance so a
    // newly supported metaclass cannot silently fall through to Operation or
    // the base false body when it actually declares a specialized predicate.
    let descriptor = target.metaclass().and_then(schema::uml_classifier)
        .ok_or_else(|| QueryError::unsupported(target.id(), "missing redefinition metaclass"))?;
    let mut pending = vec![descriptor];
    let mut visited = HashSet::new();
    let mut operations = Vec::new();
    while let Some(descriptor) = pending.pop() {
        if !visited.insert(descriptor.id) { continue; }
        for reference in descriptor.owned_operations {
            let operation = reference.target.and_then(schema::operation_by_id)
                .ok_or_else(|| QueryError::unsupported(target.id(), "unresolved metaclass operation"))?;
            if operation.name == "isConsistentWith" { operations.push(operation); }
        }
        for reference in descriptor.generals {
            pending.push(reference.target.and_then(schema::classifier)
                .ok_or_else(|| QueryError::unsupported(target.id(), "unresolved metaclass generalization"))?);
        }
    }
    let mut effective = Vec::new();
    for operation in &operations {
        let mut overridden = false;
        for other in &operations {
            if other.owner != operation.owner && schema::conforms_to(other.owner, operation.owner)
                .map_err(|error| QueryError::unsupported(target.id(), error.to_string()))? {
                overridden = true;
            }
        }
        if !overridden { effective.push(*operation); }
    }
    let [operation] = effective.as_slice() else {
        return Err(QueryError::unsupported(target.id(), "ambiguous or missing isConsistentWith override"));
    };
    match operation.id.fragment {
        "Property-isConsistentWith" =>
            super::feature_compatibility::compare_property_consistency(references, source, target),
        "Operation-isConsistentWith" =>
            super::operation_signatures::compare_operation_consistency(references, source, target),
        "RedefinableElement-isConsistentWith" => Ok(false),
        "ActivityEdge-isConsistentWith" => Ok(source.conforms_to("ActivityEdge")),
        "ActivityNode-isConsistentWith" => Ok(source.conforms_to("ActivityNode")),
        "RedefinableTemplateSignature-isConsistentWith" =>
            Ok(source.conforms_to("RedefinableTemplateSignature")),
        operation => Err(QueryError::unsupported(target.id(), format!(
            "{operation} consistency predicate is not implemented",
        ))),
    }
}

pub(crate) fn redefinition_consistency_diagnostic(
    references: &ReferenceIndex<'_>,
    element: ElementRef<'_>,
) -> Option<StructuralDiagnostic> {
    if !element.conforms_to("RedefinableElement") { return None; }
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

// UML 2.5.1 Class-passive_class, BehavioralFeature-abstract_no_method and
// Interface-visibility constrain existing roles; they do not execute Behaviors.
fn role_present(
    references: &ReferenceIndex<'_>,
    element: ElementRef<'_>,
    name: &str,
) -> Result<bool, QueryError> {
    let Some(value) = element.properties().get(name) else { return Ok(false); };
    let descriptor = schema::uml_property(element.metaclass().ok_or_else(||
        QueryError::unsupported(element.id(), "missing metaclass"))?, name)
        .map_err(|error| QueryError::unsupported(element.id(), error.to_string()))?
        .ok_or_else(|| QueryError::unsupported(element.id(), "missing role contract"))?;
    match super::property_contracts::check_value(references, element, descriptor, value) {
        super::property_contracts::PropertyOutcome::Satisfied => Ok(populated(value)),
        super::property_contracts::PropertyOutcome::Violation(error) =>
            Err(QueryError::invalid(element.id(), error)),
        super::property_contracts::PropertyOutcome::Unverified(error) => {
            // Existence needs only one checked member, unlike verification of
            // the whole collection. An unresolved neighbor cannot erase it.
            let member = PropertyDescriptor { lower: "1", upper: "1", ..*descriptor };
            if value.as_array().is_some_and(|values| values.iter().any(|value|
                reference_id(value).ok().flatten().is_some()
                    && matches!(super::property_contracts::check_value(
                        references, element, &member, value,
                    ), super::property_contracts::PropertyOutcome::Satisfied)))
            {
                Ok(true)
            } else {
                Err(QueryError::unsupported(element.id(), error))
            }
        }
    }
}

fn any_present(results: impl IntoIterator<Item = Result<bool, QueryError>>)
    -> Result<bool, QueryError>
{
    let mut unresolved = None;
    for result in results {
        match result {
            Ok(true) => return Ok(true),
            Ok(false) => {}
            Err(error) => { unresolved.get_or_insert(error); }
        }
    }
    unresolved.map_or(Ok(false), Err)
}

fn validate_feature_rules(
    references: &ReferenceIndex<'_>,
    element: ElementRef<'_>,
    issues: &mut Vec<StructuralDiagnostic>,
) {
    let mut report = |rule: &'static str, result: Result<bool, QueryError>| {
        let (severity, message) = match result {
            Ok(false) => return,
            Ok(true) => (StructuralSeverity::Error, format!("{rule}: violated")),
            Err(QueryError::Invalid { message, .. }) =>
                (StructuralSeverity::Error, format!("{rule}: {message}")),
            Err(QueryError::Unsupported { message, .. }) =>
                (StructuralSeverity::Warning, format!("{rule}: {message}")),
        };
        issues.push(StructuralDiagnostic {
            severity,
            code: if severity == StructuralSeverity::Error {
                "UML_FEATURE_CONSTRAINT"
            } else { "MDE_FEATURE_CONSTRAINT_UNVERIFIED" },
            subject_id: element.id().into(), message,
        });
    };
    if element.conforms_to("Port") {
        report("Port-encapsulated_owner", (|| {
            // UML.xmi ownedPort.1 derives ownedPort from ownedAttribute Ports;
            // its opposite encapsulatedClassifier is therefore the canonical
            // owner only when that owner/role satisfies this projection.
            // No diagram placement or stored inverse establishes ownership.
            let Some(owner) = references.model().owner(element)
                .map_err(|error| QueryError::invalid(element.id(), error))?
            else {
                // The equality is null = null; Element::mustBeOwned diagnoses
                // an ownerless Port independently, not by inventing this rule.
                return Ok(false);
            };
            if owner.metaclass().is_none() {
                return Err(QueryError::unsupported(element.id(),
                    "Port owner has no resolved normative metaclass"));
            }
            if !owner.conforms_to("EncapsulatedClassifier") { return Ok(true); }
            contributes_to_owned_union(owner, element, "StructuredClassifier-ownedAttribute")
                .map(|owned_attribute| !owned_attribute)
        })());
    }
    if element.conforms_to("BehavioredClassifier") {
        report("BehavioredClassifier-class_behavior", (|| {
            let Some(value) = element.properties().get("classifierBehavior") else {
                return Ok(false);
            };
            if !role_present(references, element, "classifierBehavior")? { return Ok(false); }
            let Some(id) = reference_id(value)
                .map_err(|error| QueryError::invalid(element.id(), error))? else {
                return Ok(false);
            };
            let behavior = references.element(id)
                .map_err(|error| QueryError::invalid(element.id(), error))?;
            role_present(references, behavior, "specification")
        })());
    }
    if element.conforms_to("Class") {
        report("Class-passive_class", (|| {
            if schema::read_effective_boolean(element, "isActive")
                .map_err(|error| QueryError::invalid(element.id(), error))? { return Ok(false); }
            let receptions = references.explicit_owned_elements(element.id())
                .map_err(|error| QueryError::invalid(element.id(), error))?
                .into_iter().filter(|child| child.conforms_to("Reception"))
                .map(|child| contributes_to_owned_union(element, child, "Class-ownedReception"));
            any_present(receptions.chain([
                role_present(references, element, "ownedReception"),
                role_present(references, element, "classifierBehavior"),
            ]))
        })());
    }
    if element.conforms_to("BehavioralFeature") {
        report("BehavioralFeature-abstract_no_method", (|| {
            if !schema::read_effective_boolean(element, "isAbstract")
                .map_err(|error| QueryError::invalid(element.id(), error))? { return Ok(false); }
            let specification = schema::uml_property("Behavior", "specification")
                .map_err(|error| QueryError::unsupported(element.id(), error.to_string()))?
                .ok_or_else(|| QueryError::unsupported(element.id(), "missing Behavior specification"))?;
            any_present([
                role_present(references, element, "method"),
                references.has_reference_source(specification, element.id())
                    .map_err(|error| QueryError::unsupported(element.id(), error)),
            ])
        })());
    }
    if element.conforms_to("Interface") {
        report("Interface-visibility", (|| {
            let owned = references.explicit_owned_elements(element.id())
                .map_err(|error| QueryError::invalid(element.id(), error))?;
            let mut checks = Vec::new();
            for child in owned.into_iter().filter(|child| child.conforms_to("Feature")) {
                checks.push(contributes_to_owned_union(element, child, "Classifier-feature")
                    .and_then(|feature| if feature {
                        visibility(child).map(|visibility| visibility != Some(VisibilityKind::Public))
                    } else { Ok(false) }));
            }
            // A stored feature union or external composite role is not evidence
            // that the complete set of Interface Features has been inspected.
            for property in properties(element)? {
                if contributes(property, "Classifier-feature")
                    .map_err(|error| QueryError::unsupported(element.id(), error))?
                    && element.properties().get(property.name).is_some_and(populated)
                {
                    if let Some(value) = element.properties().get(property.name) {
                        checks.push(match super::property_contracts::check_value(
                            references, element, property, value,
                        ) {
                            super::property_contracts::PropertyOutcome::Satisfied => Ok(false),
                            super::property_contracts::PropertyOutcome::Violation(error) =>
                                Err(QueryError::invalid(element.id(), error)),
                            super::property_contracts::PropertyOutcome::Unverified(error) =>
                                Err(QueryError::unsupported(element.id(), error)),
                        });
                    }
                }
            }
            any_present(checks)
        })());
    }
}

struct GeneralizationSetEvidence<'a> {
    generalizations: Vec<&'a crate::Relation>,
    checks: Vec<Result<bool, QueryError>>,
}

fn generalization_set_members<'a>(
    references: &ReferenceIndex<'a>,
    set: ElementRef<'a>,
) -> Result<GeneralizationSetEvidence<'a>, QueryError> {
    use super::property_contracts::{PropertyOutcome, check_value};
    let descriptor = schema::uml_property("GeneralizationSet", "generalization")
        .map_err(|error| QueryError::unsupported(set.id(), error.to_string()))?
        .ok_or_else(|| {
            QueryError::unsupported(
                set.id(),
                "missing GeneralizationSet::generalization contract",
            )
        })?;
    let value = set.properties().get("generalization").ok_or_else(|| {
        QueryError::unsupported(set.id(), "GeneralizationSet membership is not materialized")
    })?;
    let mut checks = Vec::new();
    match check_value(references, set, descriptor, value) {
        PropertyOutcome::Satisfied => {}
        PropertyOutcome::Violation(message) => return Err(QueryError::invalid(set.id(), message)),
        PropertyOutcome::Unverified(message) => {
            checks.push(Err(QueryError::unsupported(set.id(), message)))
        }
    }
    let values = value
        .as_array()
        .ok_or_else(|| QueryError::invalid(set.id(), "generalization must be a collection"))?;
    let mut generalizations = Vec::new();
    for value in values {
        if external_reference_href(value)
            .map_err(|error| QueryError::invalid(set.id(), error))?
            .is_some()
        {
            continue;
        }
        let id = reference_id(value)
            .map_err(|error| QueryError::invalid(set.id(), error))?
            .ok_or_else(|| {
                QueryError::invalid(set.id(), "Generalization member cannot be absent")
            })?;
        let generalization = references
            .element(id)
            .map_err(|error| QueryError::invalid(set.id(), error))?;
        let ElementRef::Relationship(relation) = generalization else {
            checks.push(Err(QueryError::unsupported(
                id,
                "Generalization objects require a canonical relationship adapter",
            )));
            continue;
        };
        if generalization.conforms_to("Generalization") {
            generalizations.push(relation);
        }
    }
    Ok(GeneralizationSetEvidence {
        generalizations,
        checks,
    })
}

fn generalization_classifier<'a>(
    references: &ReferenceIndex<'a>,
    relation: &'a crate::Relation,
    property: &str,
    target_id: &str,
    checks: &mut Vec<Result<bool, QueryError>>,
) -> Result<ElementRef<'a>, QueryError> {
    use super::property_contracts::{PropertyOutcome, check_value};
    let id = relation.id.as_str();
    let generalization = ElementRef::Relationship(relation);
    if let Some(value) = generalization.properties().get(property) {
        let descriptor = schema::uml_property("Generalization", property)
            .map_err(|error| QueryError::unsupported(id, error.to_string()))?
            .ok_or_else(|| {
                QueryError::unsupported(id, format!("missing Generalization::{property} contract"))
            })?;
        match check_value(references, generalization, descriptor, value) {
            PropertyOutcome::Violation(message) => {
                return Err(QueryError::invalid(id, message));
            }
            PropertyOutcome::Unverified(message) => {
                checks.push(Err(QueryError::unsupported(id, message)))
            }
            PropertyOutcome::Satisfied => {}
        }
        // A local identity can contradict the canonical endpoint even when
        // its metaclass is unresolved. External references provide no such ID.
        if external_reference_href(value)
            .map_err(|error| QueryError::invalid(id, error))?
            .is_none()
            && reference_id(value).map_err(|error| QueryError::invalid(id, error))?
                != Some(target_id)
        {
            return Err(QueryError::invalid(
                id,
                "endpoint projection contradicts the canonical Generalization endpoint",
            ));
        }
    }
    let general = references
        .element(target_id)
        .map_err(|error| QueryError::invalid(id, error))?;
    if general.metaclass().is_none() {
        checks.push(Err(QueryError::unsupported(
            id,
            format!("{property} Classifier metaclass is unresolved"),
        )));
    } else if !general.conforms_to("Classifier") {
        return Err(QueryError::invalid(
            id,
            format!("Generalization::{property} must be a Classifier"),
        ));
    }
    Ok(general)
}

fn generalization_set_same_classifier(
    references: &ReferenceIndex<'_>,
    set: ElementRef<'_>,
) -> Result<bool, QueryError> {
    let GeneralizationSetEvidence {
        generalizations,
        mut checks,
    } = generalization_set_members(references, set)?;
    let mut generals = HashSet::new();
    for relation in generalizations {
        let general =
            generalization_classifier(references, relation, "general", &relation.to, &mut checks)?;
        // Identity remains evidence even if the target metaclass is unresolved.
        generals.insert(general.id());
    }
    checks.push(Ok(generals.len() <= 1));
    super::feature_compatibility::require_all(checks)
}

fn generalization_set_powertype(
    references: &ReferenceIndex<'_>,
    set: ElementRef<'_>,
) -> Result<bool, QueryError> {
    use super::property_contracts::{PropertyOutcome, check_value};
    let value = set.properties().get("powertype").unwrap_or(&Value::Null);
    let descriptor = schema::uml_property("GeneralizationSet", "powertype")
        .map_err(|error| QueryError::unsupported(set.id(), error.to_string()))?
        .ok_or_else(|| {
            QueryError::unsupported(set.id(), "missing GeneralizationSet::powertype contract")
        })?;
    let mut power_checks = Vec::new();
    match check_value(references, set, descriptor, value) {
        PropertyOutcome::Violation(message) => return Err(QueryError::invalid(set.id(), message)),
        PropertyOutcome::Unverified(message) => {
            power_checks.push(Err(QueryError::unsupported(set.id(), message)))
        }
        PropertyOutcome::Satisfied => {}
    }
    if value.is_null() {
        return Ok(true);
    }
    let power = if external_reference_href(value)
        .map_err(|error| QueryError::invalid(set.id(), error))?
        .is_some()
    {
        None
    } else {
        let id = reference_id(value)
            .map_err(|error| QueryError::invalid(set.id(), error))?
            .ok_or_else(|| QueryError::invalid(set.id(), "powertype identity is missing"))?;
        Some(
            references
                .element(id)
                .map_err(|error| QueryError::invalid(set.id(), error))?,
        )
    };
    powertype_constraints(references, set, power, power_checks)
}

fn powertype_constraints(
    references: &ReferenceIndex<'_>,
    set: ElementRef<'_>,
    power: Option<ElementRef<'_>>,
    power_checks: Vec<Result<bool, QueryError>>,
) -> Result<bool, QueryError> {
    let GeneralizationSetEvidence {
        generalizations,
        mut checks,
    } = generalization_set_members(references, set)?;
    // A proven empty set makes forAll true without resolving the powertype.
    // Malformed powertype references have already been rejected above.
    if generalizations.is_empty() && checks.is_empty() {
        return Ok(true);
    }
    checks.extend(power_checks);
    for relation in generalizations {
        let general =
            generalization_classifier(references, relation, "general", &relation.to, &mut checks)?;
        let specific = generalization_classifier(
            references,
            relation,
            "specific",
            &relation.from,
            &mut checks,
        )?;
        // Validate known ancestry even when powertype is external: its unknown
        // identity cannot hide a malformed or cyclic local general hierarchy.
        checks.push(
            super::classifier_hierarchy::has_ancestor_matching(
                references,
                general.id(),
                |ancestor| power.is_some_and(|power| ancestor.id() == power.id()),
            )
            .map(|contains| !contains),
        );
        let Some(power) = power else {
            continue;
        };
        checks.push(Ok(general.id() != power.id()));
        checks.push(Ok(specific.id() != power.id()));
        // The OCL is asymmetric: power must not be an ancestor of general,
        // and specific must not be an ancestor of power (not the reverse).
        checks.push(
            super::classifier_hierarchy::has_ancestor_matching(
                references,
                power.id(),
                |ancestor| ancestor.id() == specific.id(),
            )
            .map(|contains| !contains),
        );
    }
    super::feature_compatibility::require_all(checks)
}

fn classifier_powertype_extent(
    references: &ReferenceIndex<'_>,
    classifier: ElementRef<'_>,
) -> Result<bool, QueryError> {
    use super::property_contracts::{PropertyOutcome, check_value};
    let mut checks = Vec::new();
    let mut canonical = HashSet::new();
    let mut unresolved = false;
    for set in references.model().elements().filter(|element| element.conforms_to("GeneralizationSet")) {
        let Some(value) = set.properties().get("powertype") else { continue; };
        if value.is_null() { continue; }
        match reference_id(value) {
            Ok(Some(id)) if id == classifier.id() => {
                canonical.insert(set.id());
                checks.push(generalization_set_powertype(references, set));
            }
            Ok(_) => {}
            Err(_) => {
                // An unknown powertype cannot matter when this set has a
                // proven empty generalization collection. Still inspect the
                // reference contract so malformed values never become vacuous.
                match generalization_set_powertype(references, set) {
                    Ok(true) => {}
                    Err(error @ QueryError::Invalid { .. }) => checks.push(Err(error)),
                    _ => unresolved = true,
                }
            }
        }
    }
    if unresolved {
        checks.push(Err(QueryError::unsupported(classifier.id(), "unresolved GeneralizationSet powertype prevents complete powertypeExtent membership")));
    }
    if let Some(value) = classifier.properties().get("powertypeExtent") {
        let descriptor = schema::uml_property("Classifier", "powertypeExtent")
            .map_err(|error| QueryError::unsupported(classifier.id(), error.to_string()))?
            .ok_or_else(|| QueryError::unsupported(classifier.id(), "missing powertypeExtent contract"))?;
        match check_value(references, classifier, descriptor, value) {
            PropertyOutcome::Satisfied => {}
            PropertyOutcome::Violation(message) => checks.push(Err(QueryError::invalid(classifier.id(), message))),
            PropertyOutcome::Unverified(message) => checks.push(Err(QueryError::unsupported(classifier.id(), message))),
        }
        if let Some(values) = value.as_array() {
            let mut authored = HashSet::new();
            let mut external = false;
            for value in values {
                match reference_id(value) {
                    Ok(Some(id)) => {
                        authored.insert(id);
                        if let Ok(set) = references.element(id) {
                            if set.metaclass().is_none() { continue; }
                            if !set.conforms_to("GeneralizationSet") { continue; }
                            // Both association ends are authored in UML. An
                            // omitted opposite is not an explicit null: local
                            // extent membership still supplies the powertype.
                            for other in references.model().elements().filter(|other| other.id() != classifier.id() && other.conforms_to("Classifier")) {
                                if other.properties().get("powertypeExtent").and_then(Value::as_array).is_some_and(|values| values.iter().any(|value| reference_id(value).ok().flatten() == Some(id))) {
                                    checks.push(Err(QueryError::invalid(classifier.id(), "GeneralizationSet has multiple authored powertype Classifiers")));
                                }
                            }
                            if let Some(opposite) = set.properties().get("powertype") {
                                let descriptor = schema::uml_property("GeneralizationSet", "powertype")
                                    .map_err(|error| QueryError::unsupported(set.id(), error.to_string()))?
                                    .ok_or_else(|| QueryError::unsupported(set.id(), "missing powertype contract"))?;
                                match check_value(references, set, descriptor, opposite) {
                                    PropertyOutcome::Violation(message) => checks.push(Err(QueryError::invalid(set.id(), message))),
                                    PropertyOutcome::Unverified(message) => checks.push(Err(QueryError::unsupported(set.id(), message))),
                                    PropertyOutcome::Satisfied => {}
                                }
                                match reference_id(opposite) {
                                    Ok(actual) if actual != Some(classifier.id()) => checks.push(Err(QueryError::invalid(classifier.id(), "powertypeExtent contradicts explicit GeneralizationSet::powertype"))),
                                    Err(_) => checks.push(Err(QueryError::unsupported(classifier.id(), "explicit powertype opposite is unresolved"))),
                                    _ => {}
                                }
                            }
                            if !canonical.contains(id) {
                                checks.push(powertype_constraints(references, set, Some(classifier), Vec::new()));
                            }
                        }
                    }
                    _ => external = true,
                }
            }
            if !external && !canonical.is_subset(&authored) {
                checks.push(Err(QueryError::invalid(classifier.id(), "powertypeExtent omits a canonical GeneralizationSet")));
            } else if external && !canonical.is_subset(&authored) {
                checks.push(Err(QueryError::unsupported(classifier.id(), "external extent entries cannot establish canonical inverse coverage")));
            }
        }
    }
    super::feature_compatibility::require_all(checks)
}

pub fn validate(references: &ReferenceIndex<'_>) -> Vec<StructuralDiagnostic> {
    let mut queries = Queries::new(references);
    let mut issues = super::classifier_hierarchy::validate_substitutions(references);
    for element in references.model().elements() {
        if element.conforms_to("Classifier") {
            let outcome = match classifier_powertype_extent(references, element) {
                Ok(true) => None,
                Ok(false) => Some((StructuralSeverity::Error, "UML_CLASSIFIER_POWERTYPE_EXTENT", "powertypeExtent violates the published endpoint or ancestry exclusions".to_owned())),
                Err(QueryError::Invalid { message, .. }) => Some((StructuralSeverity::Error, "MDE_CLASSIFIER_POWERTYPE_EXTENT_INVALID", message)),
                Err(QueryError::Unsupported { message, .. }) => Some((StructuralSeverity::Warning, "MDE_CLASSIFIER_POWERTYPE_EXTENT_UNVERIFIED", message)),
            };
            if let Some((severity, code, message)) = outcome {
                issues.push(StructuralDiagnostic { severity, code, subject_id: element.id().into(),
                    message: format!("Classifier-maps_to_generalization_set: {message}") });
            }
        }
        if element.conforms_to("Property") {
            let outcome = match super::redefinition_contexts::property_redefined_inherited(references, element) {
                Ok(true) => None,
                Ok(false) => Some((StructuralSeverity::Error, "UML_PROPERTY_REDEFINED_PROPERTY_INHERITED", "redefined Properties must be ancestral allFeatures members".to_owned())),
                Err(QueryError::Invalid { message, .. }) => Some((StructuralSeverity::Error, "UML_PROPERTY_REDEFINED_PROPERTY_INHERITED", message)),
                Err(QueryError::Unsupported { message, .. }) => Some((StructuralSeverity::Warning, "MDE_PROPERTY_REDEFINED_PROPERTY_INHERITED_UNVERIFIED", message)),
            };
            if let Some((severity, code, message)) = outcome {
                issues.push(StructuralDiagnostic { severity, code, subject_id: element.id().into(),
                    message: format!("Property-redefined_property_inherited: {message}") });
            }
        }
        if element.conforms_to("GeneralizationSet") {
            let power_outcome = match generalization_set_powertype(references, element) {
                Ok(true) => None,
                Ok(false) => Some((StructuralSeverity::Error, "UML_GENERALIZATION_SET_POWERTYPE",
                    "The powertype must not equal either endpoint, be an ancestor of the general, or descend from the specific".to_owned())),
                Err(QueryError::Invalid { message, .. }) => Some((StructuralSeverity::Error,
                    "MDE_GENERALIZATION_SET_POWERTYPE_INVALID", message)),
                Err(QueryError::Unsupported { message, .. }) => Some((StructuralSeverity::Warning,
                    "MDE_GENERALIZATION_SET_POWERTYPE_UNVERIFIED", message)),
            };
            if let Some((severity, code, message)) = power_outcome {
                issues.push(StructuralDiagnostic { severity, code, subject_id: element.id().into(),
                    message: format!("GeneralizationSet-maps_to_generalization_set: {message}") });
            }
            let outcome = match generalization_set_same_classifier(references, element) {
                Ok(true) => None,
                Ok(false) => Some((StructuralSeverity::Error, "UML_GENERALIZATION_SET_SAME_CLASSIFIER",
                    "Generalizations in a GeneralizationSet must share one general Classifier".to_owned())),
                Err(QueryError::Invalid { message, .. }) => Some((StructuralSeverity::Error,
                    "MDE_GENERALIZATION_SET_SAME_CLASSIFIER_INVALID", message)),
                Err(QueryError::Unsupported { message, .. }) => Some((StructuralSeverity::Warning,
                    "MDE_GENERALIZATION_SET_SAME_CLASSIFIER_UNVERIFIED", message)),
            };
            if let Some((severity, code, message)) = outcome {
                issues.push(StructuralDiagnostic { severity, code, subject_id: element.id().into(),
                    message: format!("GeneralizationSet-generalization_same_classifier: {message}") });
            }
        }
        validate_feature_rules(references, element, &mut issues);
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
    let targets = {
        let references = ReferenceIndex::new(super::root::Model::from_section(section));
        redefined_ids.iter().map(|id| references.element(id))
            .collect::<Result<Vec<_>, _>>()?
    };
    let mut candidate = section.clone();
    let element = candidate
        .elements
        .iter_mut()
        .find(|element| element.id == element_id)
        .ok_or("redefining Element not found")?;
    let source = ElementRef::Object(element);
    if !source.conforms_to("RedefinableElement") {
        return Err("a RedefinableElement is required".into());
    }
    let mut roles = Vec::new();
    for property in properties(source).map_err(|error| error.to_string())? {
        if !property.is_derived && contributes(property, "RedefinableElement-redefinedElement")? {
            roles.push(property);
        }
    }
    // A narrower subset is usable only when its type admits every target.
    // For example, Port::redefinedPort must not hide the inherited Property
    // role when a Port redefines a non-Port Property or a mixed collection.
    let compatible: Vec<_> = roles.iter().copied()
        .filter(|role| targets.iter().all(|target| matches_type(*target, role)))
        .collect();
    let mut effective = Vec::new();
    for role in &compatible {
        let mut subsetted = false;
        for other in &compatible {
            if role.id != other.id && contributes(other, role.id.fragment)? { subsetted = true; }
        }
        if !subsetted { effective.push(*role); }
    }
    let [role] = effective.as_slice() else {
        return Err("ambiguous or missing writable redefinition role".into());
    };
    let value = if role.upper == "1" {
        match redefined_ids {
            [] => Value::Null,
            [id] => json!({"$ref": id}),
            _ => return Err(format!("{} permits at most one target", role.name)),
        }
    } else {
        Value::Array(redefined_ids.iter().map(|id| json!({"$ref": id})).collect())
    };
    // This command replaces the derived union, including authored values in
    // broader subsetted roles, rather than leaving stale targets in the union.
    for property in roles { element.properties.remove(property.name); }
    element.properties.insert(role.name.into(), value);
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
