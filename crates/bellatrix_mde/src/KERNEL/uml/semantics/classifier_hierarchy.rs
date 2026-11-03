// SPDX-License-Identifier: Apache-2.0
// Copyright 2019-2026 Starion Group S.A.
// Ancestry/conformance patterns adapted from the pinned uml4net ClassifierExtensions.
// UML 2.5.1 Classifier::allParents and conformsTo govern this user-model query;
// ancestry differs from immutable metaclass conformance. may_specialize_type below
// intentionally uses metaclasses, as required by its separate normative predicate.

use super::{classifiers::QueryError, references::ReferenceIndex, root::ElementRef, schema};
use std::collections::{HashMap, HashSet, VecDeque};

/// UML Classifier::maySpecializeType default body: self.oclIsKindOf(c.oclType()).
/// Unlike user-model conformsTo(), this compares the classifiers' metaclasses.
/// The pinned UML metamodel declares no specialized override of this operation.
pub fn may_specialize_type(
    specific: ElementRef<'_>,
    general: ElementRef<'_>,
) -> Result<bool, QueryError> {
    let descriptor = |element: ElementRef<'_>| {
        element
            .metaclass()
            .and_then(schema::uml_classifier)
            .ok_or_else(|| {
                QueryError::unsupported(element.id(), "Classifier metaclass is not resolved")
            })
    };
    let specific_metaclass = descriptor(specific)?;
    let general_metaclass = descriptor(general)?;
    if !specific.conforms_to("Classifier") || !general.conforms_to("Classifier") {
        return Err(QueryError::invalid(
            specific.id(),
            "specialization requires Classifiers",
        ));
    }
    schema::conforms_to(specific_metaclass.id, general_metaclass.id)
        .map_err(|error| QueryError::unsupported(specific.id(), error.to_string()))
}

#[derive(Clone, Debug)]
pub struct ClassifierAncestry<'a> {
    pub classifier: ElementRef<'a>,
    pub parents: Vec<ElementRef<'a>>,
    pub ancestors: Vec<ElementRef<'a>>,
}

impl ClassifierAncestry<'_> {
    pub fn conforms_to(&self, other: ElementRef<'_>) -> bool {
        other.conforms_to("Classifier")
            && (self.classifier.id() == other.id()
                || self
                    .ancestors
                    .iter()
                    .any(|ancestor| ancestor.id() == other.id()))
    }
}

struct ParentEvidence<'a> {
    parents: Vec<ElementRef<'a>>,
    unresolved: Option<QueryError>,
}

pub(crate) struct HierarchyQueries<'index, 'model> {
    references: &'index ReferenceIndex<'model>,
    completed: HashMap<&'model str, ClassifierAncestry<'model>>,
}

impl<'index, 'model> HierarchyQueries<'index, 'model> {
    pub(crate) fn new(references: &'index ReferenceIndex<'model>) -> Self {
        Self {
            references,
            completed: HashMap::new(),
        }
    }

    pub(crate) fn get(&self, id: &str) -> Option<&ClassifierAncestry<'model>> {
        self.completed.get(id)
    }

    fn parent_evidence(&self, id: &str) -> Result<ParentEvidence<'model>, QueryError> {
        let classifier = self
            .references
            .element(id)
            .map_err(|error| QueryError::invalid(id, error))?;
        if classifier.metaclass().is_none() {
            return Err(QueryError::unsupported(id, "Classifier metaclass is unresolved"));
        }
        if !classifier.conforms_to("Classifier") {
            return Err(QueryError::invalid(
                id,
                "generalization endpoints must be Classifiers",
            ));
        }
        let mut unresolved = None;
        for key in ["general", "generalization"] {
            if let Some(value) = classifier.properties().get(key) {
                let descriptor = schema::uml_property("Classifier", key)
                    .map_err(|error| QueryError::unsupported(id, error.to_string()))?
                    .ok_or_else(|| QueryError::unsupported(id, "missing ancestry property contract"))?;
                let descriptor = schema::PropertyDescriptor { is_derived: false, ..*descriptor };
                match super::property_contracts::check_value(self.references, classifier, &descriptor, value) {
                    super::property_contracts::PropertyOutcome::Violation(message) =>
                        return Err(QueryError::invalid(id, message)),
                    super::property_contracts::PropertyOutcome::Unverified(message) =>
                        unresolved = Some(QueryError::unsupported(id, message)),
                    super::property_contracts::PropertyOutcome::Satisfied => {}
                }
            }
            if classifier.properties().get(key).is_some_and(|value| {
                !value.is_null() && !value.as_array().is_some_and(Vec::is_empty)
            }) {
                unresolved = Some(QueryError::unsupported(
                    id,
                    format!("{key} must be derived from canonical Generalization relationships"),
                ));
            }
        }
        if self
            .references
            .explicit_owned_elements(id)
            .map_err(|error| QueryError::invalid(id, error))?
            .iter()
            .any(|child| {
                matches!(child, ElementRef::Object(_)) && child.conforms_to("Generalization")
            })
        {
            unresolved = Some(QueryError::unsupported(
                id,
                "Generalization objects require a canonical relationship adapter",
            ));
        }
        let mut seen = HashSet::new();
        let parents = self.references
            .model()
            .relationship_records()
            .filter(|relation| relation.kind == "uml.generalization" && relation.from == id)
            .map(|relation| {
                self.references
                    .element(&relation.id)
                    .map_err(|error| QueryError::invalid(&relation.id, error))?;
                let parent = self
                    .references
                    .element(&relation.to)
                    .map_err(|error| QueryError::invalid(&relation.id, error))?;
                if parent.metaclass().is_none() {
                    return Err(QueryError::unsupported(parent.id(), "parent metaclass is unresolved"));
                }
                if !parent.conforms_to("Classifier") {
                    return Err(QueryError::invalid(
                        &relation.id,
                        "generalization endpoints must be Classifiers",
                    ));
                }
                if !may_specialize_type(classifier, parent)? {
                    return Err(QueryError::invalid(&relation.id, format!(
                        "Classifier::specialize_type: {} cannot specialize {} because its metaclass does not conform to the parent's metaclass",
                        classifier.id(), parent.id(),
                    )));
                }
                if schema::read_effective_boolean(parent, "isFinalSpecialization")
                    .map_err(|error| QueryError::invalid(parent.id(), error))?
                {
                    return Err(QueryError::invalid(&relation.id, format!(
                        "Classifier::non_final_parents: parent {} is final", parent.id(),
                    )));
                }
                Ok(parent)
            })
            .filter(|result| match result {
                Ok(parent) => seen.insert(parent.id()),
                Err(_) => true,
            })
            .filter_map(|result| match result {
                Err(error @ QueryError::Unsupported { .. }) => {
                    unresolved = Some(error);
                    None
                }
                other => Some(other),
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ParentEvidence { parents, unresolved })
    }

    fn read_parents(&self, id: &str) -> Result<Vec<ElementRef<'model>>, QueryError> {
        let evidence = self.parent_evidence(id)?;
        evidence.unresolved.map_or(Ok(evidence.parents), Err)
    }

    fn order_ancestors(
        &self,
        parents: &[ElementRef<'model>],
    ) -> Result<Vec<ElementRef<'model>>, QueryError> {
        let mut pending = VecDeque::from(parents.to_vec());
        let mut ancestors = Vec::new();
        let mut seen = HashSet::new();
        while let Some(parent) = pending.pop_front() {
            if !seen.insert(parent.id()) {
                continue;
            }
            ancestors.push(parent);
            let query = self
                .get(parent.id())
                .ok_or_else(|| QueryError::invalid(parent.id(), "missing parent ancestry"))?;
            pending.extend(query.parents.iter().copied());
        }
        let mut children: HashMap<&str, usize> =
            ancestors.iter().map(|parent| (parent.id(), 0)).collect();
        for ancestor in &ancestors {
            let query = self
                .get(ancestor.id())
                .ok_or_else(|| QueryError::invalid(ancestor.id(), "missing parent ancestry"))?;
            for parent in &query.parents {
                if let Some(count) = children.get_mut(parent.id()) {
                    *count += 1;
                }
            }
        }
        let mut ready = ancestors
            .iter()
            .copied()
            .filter(|parent| children.get(parent.id()) == Some(&0))
            .collect::<VecDeque<_>>();
        let mut result = Vec::new();
        while let Some(ancestor) = ready.pop_front() {
            result.push(ancestor);
            let query = self
                .get(ancestor.id())
                .ok_or_else(|| QueryError::invalid(ancestor.id(), "missing parent ancestry"))?;
            for parent in &query.parents {
                if let Some(count) = children.get_mut(parent.id()) {
                    *count -= 1;
                    if *count == 0 {
                        ready.push_back(*parent);
                    }
                }
            }
        }
        if result.len() != ancestors.len() {
            return Err(QueryError::invalid(
                parents.first().map_or("", |parent| parent.id()),
                "cycle in UML generalization",
            ));
        }
        Ok(result)
    }

    pub(crate) fn resolve(&mut self, id: &str) -> Result<ClassifierAncestry<'model>, QueryError> {
        let root = self
            .references
            .element(id)
            .map_err(|error| QueryError::invalid(id, error))?;
        let mut pending = vec![(root.id(), false)];
        let mut active = HashSet::new();
        let mut direct = HashMap::<&str, Vec<ElementRef<'model>>>::new();
        while let Some((id, exiting)) = pending.pop() {
            if self.completed.contains_key(id) {
                continue;
            }
            if exiting {
                let parents = direct
                    .remove(id)
                    .ok_or_else(|| QueryError::invalid(id, "missing ancestry traversal state"))?;
                let ancestors = self.order_ancestors(&parents)?;
                let classifier = self
                    .references
                    .element(id)
                    .map_err(|error| QueryError::invalid(id, error))?;
                self.completed.insert(
                    id,
                    ClassifierAncestry {
                        classifier,
                        parents,
                        ancestors,
                    },
                );
                active.remove(id);
            } else {
                if !active.insert(id) {
                    return Err(QueryError::invalid(id, "cycle in UML generalization"));
                }
                let parents = self.read_parents(id)?;
                pending.push((id, true));
                pending.extend(parents.iter().rev().map(|parent| (parent.id(), false)));
                direct.insert(id, parents);
            }
        }
        self.get(root.id())
            .cloned()
            .ok_or_else(|| QueryError::invalid(id, "missing ancestry result"))
    }
}

pub fn resolve_ancestry<'a>(
    references: &ReferenceIndex<'a>,
    classifier_id: &str,
) -> Result<ClassifierAncestry<'a>, QueryError> {
    HierarchyQueries::new(references).resolve(classifier_id)
}

pub fn conforms_to(
    references: &ReferenceIndex<'_>,
    classifier_id: &str,
    other_id: &str,
) -> Result<bool, QueryError> {
    let other = references
        .element(other_id)
        .map_err(|error| QueryError::invalid(other_id, error))?;
    Ok(resolve_ancestry(references, classifier_id)?.conforms_to(other))
}

/// Decide allParents()->includes without claiming to enumerate a complete
/// ancestry. A known canonical path survives unresolved neighboring branches;
/// invalid paths and cycles still take precedence over the positive evidence.
pub fn has_ancestor(
    references: &ReferenceIndex<'_>,
    classifier_id: &str,
    ancestor_id: &str,
) -> Result<bool, QueryError> {
    let ancestor = references.element(ancestor_id)
        .map_err(|error| QueryError::invalid(ancestor_id, error))?;
    if !ancestor.conforms_to("Classifier") {
        return Err(QueryError::invalid(ancestor_id, "ancestor must be a Classifier"));
    }
    has_ancestor_matching(references, classifier_id, |parent| parent.id() == ancestor_id)
}

pub(crate) fn has_ancestor_matching(
    references: &ReferenceIndex<'_>,
    classifier_id: &str,
    predicate: impl Fn(ElementRef<'_>) -> bool,
) -> Result<bool, QueryError> {
    let queries = HierarchyQueries::new(references);
    let mut pending = vec![(classifier_id.to_owned(), false)];
    let mut active = HashSet::new();
    let mut completed = HashSet::new();
    let mut found = false;
    let mut unknown = None;
    while let Some((id, exiting)) = pending.pop() {
        if exiting {
            active.remove(&id);
            completed.insert(id);
            continue;
        }
        if completed.contains(&id) { continue; }
        if !active.insert(id.clone()) {
            return Err(QueryError::invalid(&id, "cycle in UML generalization"));
        }
        let evidence = queries.parent_evidence(&id)?;
        if let Some(error) = evidence.unresolved { unknown = Some(error); }
        pending.push((id, true));
        for parent in evidence.parents {
            found |= predicate(parent);
            pending.push((parent.id().to_owned(), false));
        }
    }
    if found { Ok(true) } else { unknown.map_or(Ok(false), Err) }
}

fn substitution_reference<'a>(
    references: &ReferenceIndex<'a>,
    substitution: ElementRef<'a>,
    name: &str,
    value: &serde_json::Value,
) -> Result<ElementRef<'a>, QueryError> {
    let property = schema::uml_property("Substitution", name)
        .map_err(|error| QueryError::unsupported(substitution.id(), error.to_string()))?
        .ok_or_else(|| QueryError::unsupported(substitution.id(), "missing Substitution property contract"))?;
    match super::property_contracts::check_value(references, substitution, property, value) {
        super::property_contracts::PropertyOutcome::Satisfied => {}
        super::property_contracts::PropertyOutcome::Violation(message) =>
            return Err(QueryError::invalid(substitution.id(), message)),
        super::property_contracts::PropertyOutcome::Unverified(message) =>
            return Err(QueryError::unsupported(substitution.id(), message)),
    }
    references.validate(property, value)
        .map_err(|error| QueryError::invalid(substitution.id(), error))?
        .ok_or_else(|| QueryError::invalid(substitution.id(), "Substitution requires a Classifier reference"))
}

/// Resolve a Substitution's contract while checking its canonical owner/client.
/// The owner is the substituting Classifier; a stored inverse may only agree.
pub(crate) fn substituting_classifier<'a>(
    references: &ReferenceIndex<'a>,
    substitution: ElementRef<'a>,
) -> Result<ElementRef<'a>, QueryError> {
    let owner = references.model().owner(substitution)
        .map_err(|error| QueryError::invalid(substitution.id(), error))?
        .ok_or_else(|| QueryError::invalid(substitution.id(), "Substitution requires a substituting Classifier owner"))?;
    if !owner.conforms_to("Classifier") {
        return Err(QueryError::invalid(substitution.id(), "Substitution owner must be a Classifier"));
    }
    if let Some(value) = owner.properties().get("substitution") {
        let property = schema::uml_property("Classifier", "substitution")
            .map_err(|error| QueryError::unsupported(owner.id(), error.to_string()))?
            .ok_or_else(|| QueryError::unsupported(owner.id(), "missing substitution contract"))?;
        let unknown = match super::property_contracts::check_value(references, owner, property, value) {
            super::property_contracts::PropertyOutcome::Satisfied => None,
            super::property_contracts::PropertyOutcome::Violation(message) =>
                return Err(QueryError::invalid(owner.id(), message)),
            super::property_contracts::PropertyOutcome::Unverified(message) => Some(message),
        };
        let contains = value.as_array().is_some_and(|values| values.iter().any(|value|
            matches!(super::references::reference_id(value), Ok(Some(id)) if id == substitution.id())));
        if !contains {
            return Err(unknown.map_or_else(
                || QueryError::invalid(substitution.id(), "canonical ownership contradicts Classifier::substitution"),
                |message| QueryError::unsupported(substitution.id(), message),
            ));
        }
    }
    let derived_owner = serde_json::json!({"$ref": owner.id()});
    let client = substitution_reference(references, substitution, "substitutingClassifier",
        substitution.properties().get("substitutingClassifier").unwrap_or(&derived_owner))?;
    if client.id() != owner.id() {
        return Err(QueryError::invalid(substitution.id(), "Substitution::substitutingClassifier must equal its canonical owner"));
    }
    Ok(owner)
}

fn substitution_contract<'a>(
    references: &ReferenceIndex<'a>,
    substitution: ElementRef<'a>,
) -> Result<ElementRef<'a>, QueryError> {
    substituting_classifier(references, substitution)?;
    let contract = substitution_reference(references, substitution, "contract",
        substitution.properties().get("contract").unwrap_or(&serde_json::Value::Null))?;
    // Keep the specialized roles consistent with the existing Dependency
    // projections; do not maintain another client/supplier interpretation.
    super::relationship_endpoints::validate_dependency_subsets(references, substitution.id())
        .map_err(|error| match error {
            super::element_graph_queries::ElementGraphError::Invalid { subject_id, message } =>
                QueryError::invalid(&subject_id, message),
            super::element_graph_queries::ElementGraphError::Unsupported { subject_id, message } =>
                QueryError::unsupported(&subject_id, message),
        })?;
    Ok(contract)
}

/// UML.xmi Classifier-isSubstitutableFor-spec: substitution.contract includes
/// contract. This is an owned relationship query, not transitive inheritance,
/// structural compatibility, or inference from the names of Classifiers.
pub fn is_substitutable_for(
    references: &ReferenceIndex<'_>,
    classifier_id: &str,
    contract_id: &str,
) -> Result<bool, QueryError> {
    let classifier = references.element(classifier_id)
        .map_err(|error| QueryError::invalid(classifier_id, error))?;
    let contract = references.element(contract_id)
        .map_err(|error| QueryError::invalid(contract_id, error))?;
    if !classifier.conforms_to("Classifier") || !contract.conforms_to("Classifier") {
        return Err(QueryError::invalid(classifier_id, "isSubstitutableFor requires two Classifiers"));
    }
    let owned = references.model().owned_elements(classifier)
        .map_err(|error| QueryError::invalid(classifier_id, error))?;
    // An authored collection is a projection of the same ownership graph, not
    // an independent source of relationships. Validate it using its descriptor.
    let mut unresolved_collection = None;
    if let Some(value) = classifier.properties().get("substitution") {
        let property = schema::uml_property("Classifier", "substitution")
            .map_err(|error| QueryError::unsupported(classifier_id, error.to_string()))?
            .ok_or_else(|| QueryError::unsupported(classifier_id, "missing Classifier::substitution contract"))?;
        match super::property_contracts::check_value(references, classifier, property, value) {
            super::property_contracts::PropertyOutcome::Satisfied => {}
            super::property_contracts::PropertyOutcome::Violation(message) =>
                return Err(QueryError::invalid(classifier_id, message)),
            super::property_contracts::PropertyOutcome::Unverified(message) =>
                unresolved_collection = Some(Err(QueryError::unsupported(classifier_id, message))),
        }
    }
    let unresolved_binary = references.model().relationship_records().any(|relation| {
        relation.kind == "uml.substitution" && (relation.from == classifier_id
            || references.model().ownership_links().any(|owner|
                owner.from == classifier_id && owner.to == relation.id))
    }).then(|| Err(QueryError::unsupported(classifier_id,
        "binary Substitution records require a canonical ownership/role adapter")));
    super::feature_compatibility::accept_any(owned.into_iter()
        .filter(|element| element.conforms_to("Substitution"))
        .map(|substitution| {
            substitution_contract(references, substitution)
                .map(|target| target.id() == contract.id())
        }).chain(unresolved_collection).chain(unresolved_binary))
}

pub(crate) fn validate_substitutions(
    references: &ReferenceIndex<'_>,
) -> Vec<crate::StructuralDiagnostic> {
    references.model().elements()
        .filter(|element| element.conforms_to("Substitution"))
        .filter_map(|element| substitution_contract(references, element).err())
        .map(|error| {
            let (severity, code, subject_id, message) = match error {
                QueryError::Invalid { subject_id, message } =>
                    (crate::StructuralSeverity::Error, "UML_SUBSTITUTION_CONTRACT", subject_id, message),
                QueryError::Unsupported { subject_id, message } =>
                    (crate::StructuralSeverity::Warning, "MDE_SUBSTITUTION_UNVERIFIED", subject_id, message),
            };
            crate::StructuralDiagnostic { severity, code, subject_id, message }
        }).collect()
}
