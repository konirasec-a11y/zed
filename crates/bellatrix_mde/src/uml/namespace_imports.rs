// SPDX-License-Identifier: Apache-2.0
// Copyright 2019-2026 Starion Group S.A.
// Import/name patterns adapted from uml4net NamespaceExtensions and PackageExtensions,
// pinned at 8ef13af2c68ae1994c8fa4f5b3e231dee7e6715d. The finite graph solver is new.
// Normative corrections and Apache notices: docs/bellatrix/references/uml4net/.

use super::{
    classifiers::{self, QueryError},
    references::ReferenceIndex,
    root::{ElementRef, Model, VisibilityKind},
    schema,
};
use crate::{ModelingSection, Relation};
use petgraph::{algo::kosaraju_scc, graphmap::DiGraphMap};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};

#[derive(Clone, Debug)]
pub struct ResolvedImports<'a> {
    pub imported_members: Vec<ElementRef<'a>>,
    pub visible_members: Vec<ElementRef<'a>>,
    pub imported_names: BTreeMap<String, Vec<Option<String>>>,
    pub excluded_members: Vec<ElementRef<'a>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Binding {
    names: BTreeSet<Option<String>>,
    exported: bool,
}

type Bindings<'a> = BTreeMap<&'a str, Binding>;

struct Import<'a> {
    target: ElementRef<'a>,
    is_package: bool,
    exported: bool,
    alias: Option<&'a str>,
}

struct Namespace<'a> {
    element: ElementRef<'a>,
    owned: Vec<ElementRef<'a>>,
    imports: Vec<Import<'a>>,
}

fn read_import_visibility(element: ElementRef<'_>) -> Result<bool, QueryError> {
    let metaclass = element
        .metaclass()
        .ok_or_else(|| QueryError::invalid(element.id(), "import metaclass missing"))?;
    let descriptor = schema::uml_property(metaclass, "visibility")
        .map_err(|error| QueryError::invalid(element.id(), error.to_string()))?
        .ok_or_else(|| QueryError::invalid(element.id(), "import visibility contract missing"))?;
    let value = element.properties().get("visibility").cloned().or_else(|| {
        schema::effective_default(descriptor)
            .ok()
            .flatten()
            .and_then(|default| default.scalar_json())
    });
    match value.as_ref().and_then(Value::as_str) {
        Some("public") => Ok(true),
        Some("private") => Ok(false),
        _ => Err(QueryError::invalid(
            element.id(),
            "import visibility must be public or private",
        )),
    }
}

fn read_namespace<'a>(
    references: &ReferenceIndex<'a>,
    id: &str,
) -> Result<Namespace<'a>, QueryError> {
    let element = references
        .element(id)
        .map_err(|error| QueryError::invalid(id, error))?;
    if !element.conforms_to("Namespace") {
        return Err(QueryError::invalid(id, "import source must be a Namespace"));
    }
    for key in ["elementImport", "packageImport", "importedMember"] {
        if element.properties().contains_key(key) {
            return Err(QueryError::unsupported(
                id,
                format!("{key} must be projected from canonical import relationships"),
            ));
        }
    }
    let owned = classifiers::owned_members(references, id)?;
    for child in references
        .explicit_owned_elements(id)
        .map_err(|error| QueryError::invalid(id, error))?
    {
        if matches!(child, ElementRef::Object(_))
            && (child.conforms_to("ElementImport") || child.conforms_to("PackageImport"))
        {
            return Err(QueryError::unsupported(
                child.id(),
                "imports require canonical relationship records, not object properties",
            ));
        }
    }
    let mut imports = Vec::new();
    for relation in references
        .model()
        .relationship_records()
        .filter(|relation| {
            relation.from == id
                && matches!(
                    relation.kind.as_str(),
                    "uml.element_import" | "uml.package_import"
                )
        })
    {
        imports.push(read_import(references, relation)?);
    }
    Ok(Namespace {
        element,
        owned,
        imports,
    })
}

pub(crate) fn validate_import_record(
    references: &ReferenceIndex<'_>,
    element: ElementRef<'_>,
) -> Result<(), QueryError> {
    let ElementRef::Relationship(relation) = element else {
        return Err(QueryError::unsupported(
            element.id(),
            "imports require canonical relationship records, not object properties",
        ));
    };
    read_import(references, relation).map(|_| ())
}

fn read_import<'a>(
    references: &ReferenceIndex<'a>,
    relation: &'a Relation,
) -> Result<Import<'a>, QueryError> {
    let import = references
        .element(&relation.id)
        .map_err(|error| QueryError::invalid(&relation.id, error))?;
    let namespace = references
        .element(&relation.from)
        .map_err(|error| QueryError::invalid(&relation.id, error))?;
    if !namespace.conforms_to("Namespace") {
        return Err(QueryError::invalid(
            &relation.id,
            "import source must be a Namespace",
        ));
    }
    let id = namespace.id();
    let is_package = relation.kind == "uml.package_import";
    let target = references
        .element(&relation.to)
        .map_err(|error| QueryError::invalid(&relation.id, error))?;
    let required = if is_package {
        "Package"
    } else {
        "PackageableElement"
    };
    if !target.conforms_to(required) {
        return Err(QueryError::invalid(
            &relation.id,
            format!("import target must be a {required}"),
        ));
    }
    if is_package && target.id() == id {
        return Err(QueryError::invalid(
            &relation.id,
            "a Namespace cannot PackageImport itself",
        ));
    }
    if references
        .model()
        .owner(import)
        .map_err(|error| QueryError::invalid(&relation.id, error))?
        .is_none_or(|owner| owner.id() != id)
    {
        return Err(QueryError::invalid(
            &relation.id,
            "importingNamespace must be the import's canonical owner",
        ));
    }
    for key in ["importingNamespace", "importedElement", "importedPackage"] {
        if import.properties().contains_key(key) {
            return Err(QueryError::unsupported(
                &relation.id,
                format!("{key} must be projected from relationship endpoints"),
            ));
        }
    }
    let alias = match import.properties().get("alias") {
        None | Some(Value::Null) => None,
        Some(Value::String(alias)) if !is_package => Some(alias.as_str()),
        _ => {
            return Err(QueryError::invalid(
                &relation.id,
                "only ElementImport may have a string alias",
            ));
        }
    };
    let exported = read_import_visibility(import)?;
    if !is_package
        && !matches!(
            classifiers::visibility(target)?,
            None | Some(VisibilityKind::Public)
        )
    {
        return Err(QueryError::invalid(
            &relation.id,
            "ElementImport target must have public or absent visibility",
        ));
    }
    if !is_package
        && classifiers::owned_members(references, id)?
            .iter()
            .any(|member| member.id() == target.id())
    {
        return Err(QueryError::invalid(
            &relation.id,
            "a Namespace cannot ElementImport its owned members",
        ));
    }
    Ok(Import {
        target,
        is_package,
        exported,
        alias,
    })
}

fn collect_owned_bindings<'a>(namespace: &Namespace<'a>) -> Result<Bindings<'a>, QueryError> {
    namespace
        .owned
        .iter()
        .map(|element| {
            let name = element
                .named()
                .and_then(|named| named.name())
                .map(str::to_owned);
            Ok((
                element.id(),
                Binding {
                    names: BTreeSet::from([name]),
                    exported: element.conforms_to("PackageableElement")
                        && matches!(
                            classifiers::visibility(*element)?,
                            None | Some(VisibilityKind::Public)
                        ),
                },
            ))
        })
        .collect()
}

fn collect_candidates<'a>(
    namespace: &Namespace<'a>,
    resolved: &HashMap<&'a str, Bindings<'a>>,
) -> Result<Bindings<'a>, QueryError> {
    let mut candidates = BTreeMap::<&str, Binding>::new();
    for import in namespace.imports.iter().filter(|import| import.is_package) {
        let imported = resolved.get(import.target.id()).ok_or_else(|| {
            QueryError::invalid(import.target.id(), "missing package import state")
        })?;
        for (id, binding) in imported.iter().filter(|(_, binding)| binding.exported) {
            let candidate = candidates.entry(id).or_insert_with(|| Binding {
                names: BTreeSet::new(),
                exported: false,
            });
            candidate.names.extend(binding.names.iter().cloned());
            candidate.exported |= import.exported;
        }
    }
    let mut explicit = BTreeMap::<&str, Binding>::new();
    for import in namespace.imports.iter().filter(|import| !import.is_package) {
        let candidate = explicit
            .entry(import.target.id())
            .or_insert_with(|| Binding {
                names: BTreeSet::new(),
                exported: false,
            });
        candidate.names.insert(
            import
                .alias
                .or_else(|| import.target.named().and_then(|named| named.name()))
                .map(str::to_owned),
        );
        candidate.exported |= import.exported;
    }
    // UML 7.4.3.3 gives an explicit ElementImport precedence over PackageImport,
    // including its alias and its restriction on re-export visibility.
    candidates.extend(explicit);
    for owned in &namespace.owned {
        candidates.remove(owned.id());
    }
    Ok(candidates)
}

fn names_collide(
    references: &ReferenceIndex<'_>,
    left: &str,
    left_names: &Binding,
    right: &str,
    right_names: &Binding,
) -> Result<bool, QueryError> {
    if left == right || left_names.names.is_disjoint(&right_names.names) {
        return Ok(false);
    }
    let left_element = references
        .element(left)
        .map_err(|error| QueryError::invalid(left, error))?;
    let right_element = references
        .element(right)
        .map_err(|error| QueryError::invalid(right, error))?;
    Ok(left_element
        .metaclass()
        .is_some_and(|kind| right_element.conforms_to(kind))
        || right_element
            .metaclass()
            .is_some_and(|kind| left_element.conforms_to(kind)))
}

fn exclude_collisions<'a>(
    references: &ReferenceIndex<'a>,
    candidates: &Bindings<'a>,
    owned: &Bindings<'a>,
) -> Result<BTreeSet<&'a str>, QueryError> {
    let mut excluded = BTreeSet::new();
    for (id, binding) in candidates {
        for (other_id, other) in candidates.iter().chain(owned.iter()) {
            // The original OCL lacks this identity guard; otherwise a candidate
            // collides with itself. Follow the specification's "each other" text.
            if names_collide(references, id, binding, other_id, other)? {
                excluded.insert(*id);
            }
        }
    }
    Ok(excluded)
}

fn derive_bindings<'a>(
    references: &ReferenceIndex<'a>,
    namespace: &Namespace<'a>,
    resolved: &HashMap<&'a str, Bindings<'a>>,
    filter: bool,
) -> Result<(Bindings<'a>, BTreeSet<&'a str>), QueryError> {
    let mut owned = collect_owned_bindings(namespace)?;
    let mut candidates = collect_candidates(namespace, resolved)?;
    let excluded = if filter {
        exclude_collisions(references, &candidates, &owned)?
    } else {
        BTreeSet::new()
    };
    candidates.retain(|id, _| !excluded.contains(id));
    owned.extend(candidates);
    Ok((owned, excluded))
}

pub fn resolve_namespace_imports<'a>(
    references: &ReferenceIndex<'a>,
    namespace_id: &str,
) -> Result<ResolvedImports<'a>, QueryError> {
    let mut namespaces = HashMap::new();
    let mut graph = DiGraphMap::<&str, ()>::new();
    let mut pending = VecDeque::from([namespace_id.to_owned()]);
    while let Some(id) = pending.pop_front() {
        if namespaces.contains_key(id.as_str()) {
            continue;
        }
        let namespace = read_namespace(references, &id)?;
        graph.add_node(namespace.element.id());
        for import in namespace.imports.iter().filter(|import| import.is_package) {
            graph.add_edge(namespace.element.id(), import.target.id(), ());
            pending.push_back(import.target.id().to_owned());
        }
        namespaces.insert(namespace.element.id(), namespace);
    }
    let mut resolved = HashMap::new();
    let mut excluded_by_namespace = HashMap::new();
    let mut components = kosaraju_scc(&graph);
    while !components.is_empty() {
        let position = components
            .iter()
            .position(|component| {
                component.iter().all(|id| {
                    graph
                        .neighbors(*id)
                        .all(|target| component.contains(&target) || resolved.contains_key(target))
                })
            })
            .ok_or_else(|| {
                QueryError::invalid(namespace_id, "invalid package import component order")
            })?;
        let mut component = components.remove(position);
        component.sort_unstable();
        for id in &component {
            let namespace = namespaces
                .get(id)
                .ok_or_else(|| QueryError::invalid(id, "missing namespace"))?;
            resolved.insert(*id, collect_owned_bindings(namespace)?);
        }
        if component.len() > 1 {
            // Ignoring exclusions gives a monotone finite reachability problem.
            // No arbitrary depth limit or thread-local recursive state is needed.
            loop {
                let next = component
                    .iter()
                    .map(|id| {
                        let namespace = namespaces
                            .get(id)
                            .ok_or_else(|| QueryError::invalid(id, "missing namespace"))?;
                        Ok((
                            *id,
                            derive_bindings(references, namespace, &resolved, false)?.0,
                        ))
                    })
                    .collect::<Result<Vec<_>, QueryError>>()?;
                let changed = next
                    .iter()
                    .any(|(id, state)| resolved.get(id) != Some(state));
                resolved.extend(next);
                if !changed {
                    break;
                }
            }
        }
        for id in &component {
            let namespace = namespaces
                .get(id)
                .ok_or_else(|| QueryError::invalid(id, "missing namespace"))?;
            let (state, excluded) = derive_bindings(references, namespace, &resolved, true)?;
            if component.len() > 1 && !excluded.is_empty() {
                return Err(QueryError::unsupported(
                    id,
                    "cyclic package imports with name collisions require non-monotone resolution; no partial member set is returned",
                ));
            }
            resolved.insert(*id, state);
            excluded_by_namespace.insert(*id, excluded);
        }
    }
    let namespace = namespaces
        .get(namespace_id)
        .ok_or_else(|| QueryError::invalid(namespace_id, "missing import root"))?;
    let bindings = resolved
        .remove(namespace_id)
        .ok_or_else(|| QueryError::invalid(namespace_id, "missing resolved import root"))?;
    let owned: HashSet<_> = namespace.owned.iter().map(|member| member.id()).collect();
    let mut imported_members = Vec::new();
    let mut visible_members = Vec::new();
    let mut imported_names = BTreeMap::new();
    for (id, binding) in bindings {
        let element = references
            .element(id)
            .map_err(|error| QueryError::invalid(id, error))?;
        if binding.exported {
            visible_members.push(element);
        }
        if !owned.contains(id) {
            imported_members.push(element);
            imported_names.insert(id.into(), binding.names.into_iter().collect());
        }
    }
    let excluded_members = excluded_by_namespace
        .remove(namespace_id)
        .unwrap_or_default()
        .into_iter()
        .map(|id| {
            references
                .element(id)
                .map_err(|error| QueryError::invalid(id, error))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ResolvedImports {
        imported_members,
        visible_members,
        imported_names,
        excluded_members,
    })
}

pub(crate) fn set_namespace_import(
    section: &mut ModelingSection,
    import_id: &str,
    namespace_id: &str,
    target_id: &str,
    is_package: bool,
    alias: Option<&str>,
    visibility: &str,
) -> Result<(), String> {
    let mut candidate = section.clone();
    if import_id.is_empty()
        || candidate
            .elements
            .iter()
            .any(|element| element.id == import_id)
        || candidate.diagrams.iter().any(|diagram| {
            diagram.id == import_id || diagram.views.iter().any(|view| view.id == import_id)
        })
    {
        return Err("import identity is empty or already used by a different model object".into());
    }
    let kind = if is_package {
        "uml.package_import"
    } else {
        "uml.element_import"
    };
    if let Some(existing) = candidate
        .relations
        .iter_mut()
        .find(|relation| relation.id == import_id)
    {
        if existing.kind != kind {
            return Err("import identity belongs to a different relationship kind".into());
        }
        if existing.from != namespace_id {
            return Err("change of importingNamespace requires explicit reparenting".into());
        }
        existing.to = target_id.into();
        existing
            .properties
            .insert("visibility".into(), json!(visibility));
        if let Some(alias) = alias {
            existing.properties.insert("alias".into(), json!(alias));
        } else {
            existing.properties.remove("alias");
        }
    } else {
        let mut properties = BTreeMap::from([("visibility".into(), json!(visibility))]);
        if let Some(alias) = alias {
            properties.insert("alias".into(), json!(alias));
        }
        candidate.relations.push(Relation {
            id: import_id.into(),
            kind: kind.into(),
            from: namespace_id.into(),
            to: target_id.into(),
            properties,
        });
    }
    let references = ReferenceIndex::new(Model::from_section(&candidate));
    let import = references.element(import_id)?;
    let ElementRef::Relationship(relation) = import else {
        return Err("import requires a canonical relationship".into());
    };
    match read_import(&references, relation) {
        Ok(_) | Err(QueryError::Unsupported { .. }) => {}
        Err(error) => return Err(error.to_string()),
    }
    match resolve_namespace_imports(&references, namespace_id) {
        Ok(_) | Err(QueryError::Unsupported { .. }) => {}
        Err(error) => return Err(error.to_string()),
    }
    *section = candidate;
    Ok(())
}
