use crate::{
    Graph, StructuralDiagramKind, StructuralSeverity, validate_structural_diagram,
    validate_structural_model,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;

use crate::api_contract::{API_VERSION, decode_model, error_response, invalid_request};
use crate::kernel_api_contract::decode_uml_model;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ValidationRequest {
    api_version: u32,
    model: Box<serde_json::value::RawValue>,
}

#[derive(Serialize)]
struct ValidationResponse {
    api_version: u32,
    coverage: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    validity: Option<&'static str>,
    has_errors: bool,
    diagnostics: Vec<Diagnostic>,
}

#[derive(Serialize)]
struct Diagnostic {
    severity: &'static str,
    code: String,
    subject_id: String,
    message: String,
}

/// Validate a model through the versioned, function-only kernel boundary.
///
/// Input is a JSON object containing exactly `api_version: 1` and `model`, the
/// existing Bellatrix model document. Output contains `api_version`,
/// `coverage`, `validity`, `has_errors` and ordered `diagnostics`. Coverage is
/// `implemented_rules_only` for pure UML, `implemented_uml_subset_rules_only`
/// for a closed UML graph alongside external domains,
/// `implemented_uml_scalars_and_integrity_only` for cross-domain dependencies, or
/// `implemented_rules_with_incomplete_authoring` for unresolved navigation.
/// Each diagnostic contains `severity`, `code`, `subject_id` and `message`.
/// Diagnostics are ordered by code, subject identity, severity and message.
/// Array order inside the input model retains its existing semantic meaning.
///
/// # Validation scope
/// Document presentation, composition and provenance are ignored, even when their
/// contents are malformed. Semantic identities are checked first. Mixed-domain
/// models return explicit unverified diagnostics and a capacity-blocking error,
/// with `validity: "unverified"`. Their closed UML projection runs implemented
/// rules without external aliases; cross-domain dependencies conservatively
/// suspend relational/global rules while retaining scalar and integrity checks.
/// Pure UML models check root, endpoints and ownership before evaluating
/// implemented rules. Typed authoring knowledge is semantic input: unresolved
/// Association navigation emits a capacity blocker without hiding proved UML
/// violations. No errors does not mean complete UML conformance, and an
/// unverified result must not be interpreted as successful semantic validation.
///
/// # Errors
/// Invalid JSON/envelopes, unsupported API/model versions and malformed model
/// documents return an error JSON string with `api_version` and an `error` object
/// containing `code` and `message`. A well-formed but invalid model is a successful
/// API call with `has_errors: true`, not a transport/contract error.
///
/// # Effects and determinism
/// No external side effects: no filesystem, clock, process, network, logging sink
/// or external callback is used. The input is not mutated; all indices and model
/// values are local to this invocation. For the same input and implementation,
/// the result is independent of preceding calls. No internal Rust model, index
/// or diagnostic type is part of this function's public signature.
pub fn validate_uml_model(input_json: &str) -> Result<String, String> {
    let request: ValidationRequest = serde_json::from_str(input_json).map_err(invalid_request)?;
    let model = decode_uml_model(request.api_version, &request.model)?;
    let mut diagnostics = match validate_semantic_identifiers(&model) {
        Err(error) => vec![integrity_diagnostic(&model, error)],
        Ok(()) => {
            let unsupported = mixed_domain_diagnostics(&model);
            if !unsupported.is_empty() {
                unsupported
            } else {
                match validate_uml_integrity(&model) {
                    Err(error) => vec![integrity_diagnostic(&model, error)],
                    Ok(()) => validate_structural_model(&model)
                        .into_iter()
                        .map(semantic_diagnostic)
                        .collect(),
                }
            }
        }
    };
    diagnostics.sort_by(|left, right| {
        (&left.code, &left.subject_id, left.severity, &left.message).cmp(&(
            &right.code,
            &right.subject_id,
            right.severity,
            &right.message,
        ))
    });
    let unverified = diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "UML_MODEL_SEMANTICS_UNVERIFIED");
    let has_errors = diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == "error");
    let response = ValidationResponse {
        api_version: API_VERSION,
        coverage: if diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "UML_CROSS_DOMAIN_RULES_UNVERIFIED")
        {
            "implemented_uml_scalars_and_integrity_only"
        } else if diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "MDE_ASSOCIATION_NAVIGATION_UNVERIFIED")
        {
            "implemented_rules_with_incomplete_authoring"
        } else if unverified {
            "implemented_uml_subset_rules_only"
        } else {
            "implemented_rules_only"
        },
        validity: Some(if unverified {
            "unverified"
        } else if has_errors {
            "invalid"
        } else {
            "no_violations_found"
        }),
        has_errors,
        diagnostics,
    };
    serde_json::to_string(&response)
        .map_err(|error| error_response("RESPONSE_SERIALIZATION_FAILED", error.to_string()))
}

fn integrity_diagnostic(model: &crate::Project, error: crate::GraphError) -> Diagnostic {
    Diagnostic {
        severity: "error",
        code: "GRAPH_INTEGRITY".into(),
        subject_id: model.id.clone(),
        message: error.to_string(),
    }
}

fn validate_semantic_identifiers(model: &crate::Project) -> Result<(), crate::GraphError> {
    let mut identities = HashSet::new();
    for identifier in std::iter::once(&model.id)
        .chain(model.elements.iter().map(|element| &element.id))
        .chain(model.relations.iter().map(|relation| &relation.id))
    {
        if identifier.trim().is_empty() || !identities.insert(identifier.as_str()) {
            return Err(crate::GraphError::Duplicate(identifier.clone()));
        }
    }
    Ok(())
}

fn validate_uml_integrity(project: &crate::Project) -> Result<(), crate::GraphError> {
    let model = crate::uml::root::Model::from_project(project);
    model.root().map_err(crate::GraphError::InvalidRoot)?;
    let identities: HashSet<_> = model.elements().map(|element| element.id()).collect();
    let mut owners = HashSet::new();
    for relation in &project.relations {
        for endpoint in [&relation.from, &relation.to] {
            if !identities.contains(endpoint.as_str()) {
                return Err(crate::GraphError::MissingEndpoint {
                    relation: relation.id.clone(),
                    endpoint: endpoint.clone(),
                });
            }
        }
        if relation.kind == "owns" && !owners.insert(relation.to.as_str()) {
            return Err(crate::GraphError::MultipleOwners(relation.to.clone()));
        }
    }
    if !crate::GraphIndex::new(project)
        .cyclic_components("owns")
        .is_empty()
    {
        return Err(crate::GraphError::Cycle("owns".into()));
    }
    Ok(())
}

fn mixed_domain_diagnostics(project: &crate::Project) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut external = HashSet::new();
    for (identifier, kind) in project
        .elements
        .iter()
        .map(|element| (&element.id, &element.kind))
        .chain(
            project
                .relations
                .iter()
                .filter(|relation| relation.kind != "owns")
                .map(|relation| (&relation.id, &relation.kind)),
        )
    {
        if !kind.starts_with("uml.") {
            external.insert(identifier.as_str());
            diagnostics.push(Diagnostic {
                severity: "warning",
                code: "UML_EXTERNAL_DOMAIN_UNVERIFIED".into(),
                subject_id: identifier.clone(),
                message: format!("Kind {kind} belongs to an unmapped external domain; its UML semantics are not verified"),
            });
        }
    }
    if external.is_empty() {
        return diagnostics;
    }
    for relation in &project.relations {
        if relation.kind == "owns"
            && external.contains(relation.from.as_str())
            && external.contains(relation.to.as_str())
        {
            external.insert(relation.id.as_str());
        }
    }
    let mut reported_references = HashSet::new();
    let mut reference_warning = |subject: &str, target: &str| {
        if !reported_references.insert((subject.to_owned(), target.to_owned())) {
            return;
        }
        diagnostics.push(Diagnostic {
            severity: "warning",
            code: "UML_EXTERNAL_REFERENCE_UNVERIFIED".into(),
            subject_id: subject.into(),
            message: format!(
                "Reference to {target} involves an unmapped external-domain participant; UML semantics are not verified"
            ),
        });
    };
    if let Some(root) = project
        .root_model
        .as_deref()
        .filter(|root| external.contains(root))
    {
        reference_warning(&project.id, root);
    }
    let uml: HashSet<_> = project
        .elements
        .iter()
        .filter(|element| element.kind.starts_with("uml."))
        .map(|element| element.id.as_str())
        .chain(
            project
                .relations
                .iter()
                .filter(|relation| relation.kind.starts_with("uml."))
                .map(|relation| relation.id.as_str()),
        )
        .collect();
    for relation in &project.relations {
        for endpoint in [&relation.from, &relation.to] {
            if external.contains(endpoint.as_str())
                || (external.contains(relation.id.as_str()) && uml.contains(endpoint.as_str()))
            {
                reference_warning(&relation.id, endpoint);
            }
        }
    }
    for (subject, properties) in project
        .elements
        .iter()
        .map(|element| (&element.id, &element.properties))
        .chain(
            project
                .relations
                .iter()
                .map(|relation| (&relation.id, &relation.properties)),
        )
    {
        for value in properties.values() {
            visit_external_references(value, &external, &mut |target| {
                reference_warning(subject, target)
            });
            if external.contains(subject.as_str()) {
                visit_external_references(value, &uml, &mut |target| {
                    reference_warning(subject, target)
                });
            }
        }
    }
    let mut projection = project.clone();
    projection
        .elements
        .retain(|element| element.kind.starts_with("uml."));
    projection.relations.retain(|relation| {
        relation.kind.starts_with("uml.")
            || (relation.kind == "owns"
                && !external.contains(relation.from.as_str())
                && !external.contains(relation.to.as_str()))
    });
    if projection
        .root_model
        .as_deref()
        .is_some_and(|root| external.contains(root))
    {
        projection.root_model = None;
    }
    if has_cross_domain_dependencies(project, &external) {
        // Some existing rules search every candidate of a metaclass, including
        // malformed candidates without explicit references. Graph components are
        // not sufficient evidence of closure for those global/absence queries.
        diagnostics.push(Diagnostic {
            severity: "warning",
            code: "UML_CROSS_DOMAIN_RULES_UNVERIFIED".into(),
            subject_id: project.id.clone(),
            message: "Cross-domain dependencies prevent closed UML evaluation: relational and global rules are suspended; independent scalar contracts and local graph integrity are still checked".into(),
        });
        if let Err(error) = validate_partial_uml_integrity(&projection, project, &external) {
            diagnostics.push(integrity_diagnostic(project, error));
        }
        for element in crate::uml::root::Model::from_project(&projection).elements() {
            diagnostics.extend(
                crate::uml::schema::validate_element(element)
                    .into_iter()
                    .map(semantic_diagnostic),
            );
        }
    } else {
        match validate_uml_integrity(&projection) {
            Err(error) => diagnostics.push(integrity_diagnostic(project, error)),
            Ok(()) => diagnostics.extend(
                validate_structural_model(&projection)
                    .into_iter()
                    .map(semantic_diagnostic),
            ),
        }
    }
    diagnostics.push(Diagnostic {
        severity: "error",
        code: "UML_MODEL_SEMANTICS_UNVERIFIED".into(),
        subject_id: project.id.clone(),
        message: "Mixed-domain model: implemented UML checks ran only within the reported coverage; external-domain semantics remain unverified and this result does not establish whole-model validity".into(),
    });
    diagnostics
}

fn semantic_diagnostic(diagnostic: crate::StructuralDiagnostic) -> Diagnostic {
    Diagnostic {
        severity: match diagnostic.severity {
            StructuralSeverity::Error => "error",
            StructuralSeverity::Warning => "warning",
        },
        code: diagnostic.code.into(),
        subject_id: diagnostic.subject_id,
        message: diagnostic.message,
    }
}

fn has_cross_domain_dependencies(project: &crate::Project, external: &HashSet<&str>) -> bool {
    if project
        .root_model
        .as_deref()
        .is_some_and(|root| external.contains(root))
    {
        return true;
    }
    let uml: HashSet<_> = project
        .elements
        .iter()
        .filter(|element| element.kind.starts_with("uml."))
        .map(|element| element.id.as_str())
        .chain(
            project
                .relations
                .iter()
                .filter(|relation| relation.kind.starts_with("uml."))
                .map(|relation| relation.id.as_str()),
        )
        .collect();
    for relation in &project.relations {
        let source_external = external.contains(relation.from.as_str());
        let target_external = external.contains(relation.to.as_str());
        if (relation.kind.starts_with("uml.") && (source_external || target_external))
            || (external.contains(relation.id.as_str())
                && (uml.contains(relation.from.as_str()) || uml.contains(relation.to.as_str())))
            || (relation.kind == "owns" && source_external != target_external)
        {
            return true;
        }
    }
    for (identifier, properties) in project
        .elements
        .iter()
        .map(|element| (&element.id, &element.properties))
        .chain(
            project
                .relations
                .iter()
                .map(|relation| (&relation.id, &relation.properties)),
        )
    {
        let mut crosses = false;
        let targets = if external.contains(identifier.as_str()) {
            &uml
        } else {
            external
        };
        for value in properties.values() {
            visit_external_references(value, targets, &mut |_| crosses = true);
        }
        if crosses {
            return true;
        }
    }
    false
}

fn validate_partial_uml_integrity(
    projection: &crate::Project,
    authored: &crate::Project,
    external: &HashSet<&str>,
) -> Result<(), crate::GraphError> {
    let model = crate::uml::root::Model::from_project(projection);
    model.root().map_err(crate::GraphError::InvalidRoot)?;
    let identities: HashSet<_> = model.elements().map(|element| element.id()).collect();
    // `owns` is the canonical ownership contract, even when a participant's
    // metaclass is unmapped. Keep its original paths to prove cycles/ambiguity,
    // but do not validate disconnected, entirely external ownership components.
    let mut ownership = petgraph::graphmap::DiGraphMap::<&str, ()>::new();
    for relation in authored
        .relations
        .iter()
        .filter(|relation| relation.kind == "owns")
    {
        ownership.add_edge(relation.from.as_str(), relation.to.as_str(), ());
    }
    let mut relevant = HashSet::new();
    let mut pending: Vec<_> = authored
        .elements
        .iter()
        .filter(|element| element.kind.starts_with("uml."))
        .map(|element| element.id.as_str())
        .chain(
            authored
                .relations
                .iter()
                .filter(|relation| relation.kind.starts_with("uml."))
                .map(|relation| relation.id.as_str()),
        )
        .collect();
    while let Some(identifier) = pending.pop() {
        if relevant.insert(identifier) {
            pending.extend(ownership.neighbors_directed(identifier, petgraph::Direction::Outgoing));
            pending.extend(ownership.neighbors_directed(identifier, petgraph::Direction::Incoming));
        }
    }
    let mut owners = HashSet::new();
    for relation in &authored.relations {
        let relevant_ownership = relation.kind == "owns"
            && (relevant.contains(relation.from.as_str())
                || relevant.contains(relation.to.as_str()));
        if !relation.kind.starts_with("uml.") && !relevant_ownership {
            continue;
        }
        for endpoint in [&relation.from, &relation.to] {
            if !external.contains(endpoint.as_str()) && !identities.contains(endpoint.as_str()) {
                return Err(crate::GraphError::MissingEndpoint {
                    relation: relation.id.clone(),
                    endpoint: endpoint.clone(),
                });
            }
        }
        if relevant_ownership && !owners.insert(relation.to.as_str()) {
            return Err(crate::GraphError::MultipleOwners(relation.to.clone()));
        }
    }
    if petgraph::algo::kosaraju_scc(&ownership)
        .iter()
        .any(|component| {
            component
                .iter()
                .any(|identifier| relevant.contains(identifier))
                && (component.len() > 1
                    || component
                        .iter()
                        .any(|identifier| ownership.contains_edge(identifier, identifier)))
        })
    {
        return Err(crate::GraphError::Cycle("owns".into()));
    }
    Ok(())
}

fn visit_external_references(
    value: &Value,
    external: &HashSet<&str>,
    report: &mut impl FnMut(&str),
) {
    match value {
        Value::Object(object) => {
            if let Some(target) = object.get("$ref").and_then(Value::as_str)
                && external.contains(target)
            {
                report(target);
            }
            for nested in object.values() {
                visit_external_references(nested, external, report);
            }
        }
        Value::Array(values) => {
            for nested in values {
                visit_external_references(nested, external, report);
            }
        }
        _ => {}
    }
}

/// Validate graph integrity and presentation without structural UML rule evaluation.
///
/// Input uses the same versioned `{api_version:1, model:...}` envelope as
/// `validate_uml_model`. Output has the same diagnostic shape, with coverage
/// `graph_integrity_and_presentation`. This deliberately preserves the complete
/// legacy Graph::validate behavior used by the CLI, including geometry/style,
/// rather than the topology-only precondition used by graph queries.
///
/// # Errors
/// Contract errors are versioned JSON errors. A decoded but invalid graph returns
/// Ok with `has_errors:true` and the first `GRAPH_INTEGRITY` diagnostic.
///
/// # Effects and determinism
/// No external effects, I/O, clock, sink or callbacks. No caller mutation or state
/// retention; the same input yields identical output independently of prior calls.
pub fn validate_graph_model(input_json: &str) -> Result<String, String> {
    let request: ValidationRequest = serde_json::from_str(input_json).map_err(invalid_request)?;
    let model = decode_model(request.api_version, &request.model)?;
    let diagnostics = match Graph::new(&model).validate() {
        Ok(()) => Vec::new(),
        Err(error) => vec![Diagnostic {
            severity: "error",
            code: "GRAPH_INTEGRITY".into(),
            subject_id: model.id.clone(),
            message: error.to_string(),
        }],
    };
    let response = ValidationResponse {
        api_version: API_VERSION,
        coverage: "graph_integrity_and_presentation",
        validity: None,
        has_errors: !diagnostics.is_empty(),
        diagnostics,
    };
    serde_json::to_string(&response).map_err(|_| {
        error_response(
            "RESPONSE_SERIALIZATION_FAILED",
            "validation response cannot be encoded as JSON",
        )
    })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DiagramValidationRequest {
    api_version: u32,
    model: Box<serde_json::value::RawValue>,
    diagram_id: String,
    diagram_kind: String,
}

/// Validate one diagram projection and whole-model semantics through a pure function.
///
/// Input contains exactly `api_version: 1`, `model`, `diagram_id` and
/// `diagram_kind`. Kind names are `class`, `object`, `package`, `component`,
/// `composite_structure`, `deployment`, `profile`, `use_case`, `activity`,
/// `state_machine`, `sequence`, `communication`, `interaction_overview`, `timing`.
/// The explicit kind is checked against the stored UMLDI type and interaction kind.
///
/// Output uses the model validation diagnostic envelope with coverage
/// `implemented_model_and_diagram_rules_only`. Graph integrity and presentation
/// run first; failure produces one `GRAPH_INTEGRITY` error and skips semantics.
/// Otherwise whole-model rules and the selected diagram's existing projection
/// rules run. Missing diagrams and kind mismatches are validation diagnostics,
/// not contract errors. Other diagrams' projection rules are not evaluated.
/// Unsupported semantic coverage and visual drafts retain their warnings.
/// This does not certify full UML/UMLDI conformance or generation readiness.
///
/// # Errors
/// Contract failures use the same versioned JSON errors as `validate_uml_model`.
/// An unrecognized requested kind returns `UNSUPPORTED_DIAGRAM_KIND`.
///
/// # Effects and determinism
/// No I/O, clock, callbacks, logging, retained state or input mutation. Diagnostics
/// are sorted by code, subject identity, severity and message. Internal model and
/// diagram types do not appear in the public signature.
pub fn validate_uml_diagram(input_json: &str) -> Result<String, String> {
    let request: DiagramValidationRequest =
        serde_json::from_str(input_json).map_err(invalid_request)?;
    let model = decode_model(request.api_version, &request.model)?;
    let kind = match request.diagram_kind.as_str() {
        "class" => StructuralDiagramKind::Class,
        "object" => StructuralDiagramKind::Object,
        "package" => StructuralDiagramKind::Package,
        "component" => StructuralDiagramKind::Component,
        "composite_structure" => StructuralDiagramKind::CompositeStructure,
        "deployment" => StructuralDiagramKind::Deployment,
        "profile" => StructuralDiagramKind::Profile,
        "use_case" => StructuralDiagramKind::UseCase,
        "activity" => StructuralDiagramKind::Activity,
        "state_machine" => StructuralDiagramKind::StateMachine,
        "sequence" => StructuralDiagramKind::Sequence,
        "communication" => StructuralDiagramKind::Communication,
        "interaction_overview" => StructuralDiagramKind::InteractionOverview,
        "timing" => StructuralDiagramKind::Timing,
        _ => {
            return Err(error_response(
                "UNSUPPORTED_DIAGRAM_KIND",
                "diagram_kind must name a supported version-1 diagram category",
            ));
        }
    };
    let mut diagnostics: Vec<Diagnostic> = match Graph::new(&model).validate() {
        Err(error) => vec![Diagnostic {
            severity: "error",
            code: "GRAPH_INTEGRITY".into(),
            subject_id: model.id.clone(),
            message: error.to_string(),
        }],
        Ok(()) => validate_structural_diagram(&model, &request.diagram_id, kind)
            .into_iter()
            .map(|diagnostic| Diagnostic {
                severity: match diagnostic.severity {
                    StructuralSeverity::Error => "error",
                    StructuralSeverity::Warning => "warning",
                },
                code: diagnostic.code.into(),
                subject_id: diagnostic.subject_id,
                message: diagnostic.message,
            })
            .collect(),
    };
    diagnostics.sort_by(|left, right| {
        (&left.code, &left.subject_id, left.severity, &left.message).cmp(&(
            &right.code,
            &right.subject_id,
            right.severity,
            &right.message,
        ))
    });
    let response = ValidationResponse {
        api_version: API_VERSION,
        coverage: "implemented_model_and_diagram_rules_only",
        validity: None,
        has_errors: diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == "error"),
        diagnostics,
    };
    serde_json::to_string(&response).map_err(|_| {
        error_response(
            "RESPONSE_SERIALIZATION_FAILED",
            "validation response cannot be encoded as JSON",
        )
    })
}
