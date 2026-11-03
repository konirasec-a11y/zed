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

    fn read_parents(&self, id: &str) -> Result<Vec<ElementRef<'model>>, QueryError> {
        let classifier = self
            .references
            .element(id)
            .map_err(|error| QueryError::invalid(id, error))?;
        if !classifier.conforms_to("Classifier") {
            return Err(QueryError::invalid(
                id,
                "generalization endpoints must be Classifiers",
            ));
        }
        for key in ["general", "generalization"] {
            if classifier.properties().get(key).is_some_and(|value| {
                !value.is_null() && !value.as_array().is_some_and(Vec::is_empty)
            }) {
                return Err(QueryError::unsupported(
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
            return Err(QueryError::unsupported(
                id,
                "Generalization objects require a canonical relationship adapter",
            ));
        }
        let mut seen = HashSet::new();
        self.references
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
            .collect()
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
