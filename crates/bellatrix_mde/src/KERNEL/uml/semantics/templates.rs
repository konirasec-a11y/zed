use crate::{Element, ModelingSection, Project, StructuralDiagnostic, StructuralSeverity, UmlType};
use std::collections::HashSet;

pub(crate) const GENERATED_SIGNATURE: &str = "bellatrix.generatedTemplateSignature";

pub(crate) fn is_signature(kind: &str) -> bool {
    matches!(
        kind,
        "uml.template_signature" | "uml.redefinable_template_signature"
    )
}
pub(crate) fn is_parameter(kind: &str) -> bool {
    matches!(
        kind,
        "uml.template_parameter" | "uml.classifier_template_parameter"
    )
}

/// Includes invalid legacy direct parameters for diagnostic visibility, not conformance.
pub fn template_parameters<'a>(section: &'a ModelingSection, owner: &str) -> Vec<&'a Element> {
    let signatures: HashSet<_> = section
        .relations
        .iter()
        .filter(|relation| relation.kind == "owns" && relation.from == owner)
        .filter_map(|relation| {
            section
                .elements
                .iter()
                .find(|element| element.id == relation.to)
        })
        .filter(|element| is_signature(&element.kind))
        .map(|element| element.id.as_str())
        .collect();
    let mut seen = HashSet::new();
    section
        .relations
        .iter()
        .filter(|relation| {
            relation.kind == "owns"
                && (relation.from == owner || signatures.contains(relation.from.as_str()))
        })
        .filter_map(|relation| {
            section
                .elements
                .iter()
                .find(|element| element.id == relation.to)
        })
        .filter(|element| is_parameter(&element.kind) && seen.insert(element.id.as_str()))
        .collect()
}

pub fn template_parametered_element<'a>(
    section: &'a ModelingSection,
    parameter: &Element,
) -> Option<&'a Element> {
    if !is_parameter(&parameter.kind) {
        return None;
    }
    let mut references = section.relations.iter().filter(|relation| {
        relation.kind == "uml.parametered_element" && relation.from == parameter.id
    });
    let reference = references.next()?;
    if references.next().is_some() {
        return None;
    }
    section
        .elements
        .iter()
        .find(|element| element.id == reference.to)
}

pub(crate) fn rename_parameter(
    section: &mut ModelingSection,
    id: &str,
    name: &str,
) -> Result<bool, String> {
    if !section
        .elements
        .iter()
        .any(|element| element.id == id && is_parameter(&element.kind))
    {
        return Ok(false);
    }
    let references = section
        .relations
        .iter()
        .filter(|relation| relation.kind == "uml.parametered_element" && relation.from == id)
        .collect::<Vec<_>>();
    if references.is_empty() {
        return Ok(false);
    }
    if references.len() != 1 {
        return Err("parameter has ambiguous parameteredElement references".into());
    }
    let signature_ids = section
        .relations
        .iter()
        .filter(|relation| relation.kind == "owns" && relation.to == id)
        .map(|relation| relation.from.as_str())
        .collect::<HashSet<_>>();
    let template_ids = section
        .relations
        .iter()
        .filter(|relation| relation.kind == "owns" && signature_ids.contains(relation.to.as_str()))
        .map(|relation| relation.from.as_str())
        .collect::<HashSet<_>>();
    let model = section.to_model("template-rename-validation", "Template rename validation");
    if let Some(diagnostic) = validate(&model).into_iter().find(|diagnostic| {
        diagnostic.subject_id == id
            || ((signature_ids.contains(diagnostic.subject_id.as_str())
                || template_ids.contains(diagnostic.subject_id.as_str()))
                && matches!(
                    diagnostic.code,
                    "UML_TEMPLATE_SIGNATURE_OWNER"
                        | "UML_TEMPLATE_CLASSIFIER_SIGNATURE"
                        | "UML_TEMPLATE_SIGNATURE_MULTIPLICITY"
                ))
            || section
                .relations
                .iter()
                .any(|relation| relation.from == id && relation.id == diagnostic.subject_id)
    }) {
        return Err(format!("{}: {}", diagnostic.code, diagnostic.message));
    }
    let target = references
        .first()
        .ok_or("parameteredElement missing")?
        .to
        .clone();
    if name.trim().is_empty() {
        return Err("parameter name must not be blank".into());
    }
    section
        .elements
        .iter_mut()
        .find(|element| element.id == target)
        .ok_or("parameteredElement missing")?
        .name = name.into();
    section
        .elements
        .iter_mut()
        .find(|element| element.id == id)
        .ok_or("parameter missing")?
        .name = name.into();
    Ok(true)
}

fn conforms(element: &Element, metaclass: &str) -> bool {
    UmlType::from_kind(&element.kind)
        .and_then(UmlType::metaclass)
        .is_some_and(|name| crate::uml::metamodel::conforms_to(name, metaclass))
}
fn issue(code: &'static str, subject: &str, message: &str) -> StructuralDiagnostic {
    StructuralDiagnostic {
        severity: StructuralSeverity::Error,
        code,
        subject_id: subject.into(),
        message: message.into(),
    }
}

struct TemplateReferenceEvidence<'a> {
    known: Vec<crate::uml::root::ElementRef<'a>>,
    unknown: bool,
    cardinality: Option<usize>,
}

fn template_reference_evidence<'a>(
    references: &crate::uml::references::ReferenceIndex<'a>,
    element: crate::uml::root::ElementRef<'a>,
    property: &str,
) -> Result<TemplateReferenceEvidence<'a>, crate::uml::classifiers::QueryError> {
    use crate::uml::{
        classifiers::QueryError,
        property_contracts::{PropertyOutcome, check_value},
        references::{external_reference_href, reference_id},
        schema,
    };
    let metaclass = element.metaclass().ok_or_else(|| {
        QueryError::unsupported(element.id(), "template reference metaclass is unresolved")
    })?;
    let descriptor = schema::uml_property(metaclass, property)
        .map_err(|error| QueryError::unsupported(element.id(), error.to_string()))?
        .ok_or_else(|| {
            QueryError::unsupported(element.id(), "template reference contract is unavailable")
        })?;
    let value = element.properties().get(property).ok_or_else(|| {
        QueryError::unsupported(
            element.id(),
            format!("{property} projection is not materialized"),
        )
    })?;
    match check_value(references, element, descriptor, value) {
        PropertyOutcome::Satisfied | PropertyOutcome::Unverified(_) => {}
        PropertyOutcome::Violation(message) => {
            return Err(QueryError::invalid(element.id(), message));
        }
    }
    // Membership depends on identities, not the resolution of their metaclasses.
    // Full property-contract validation independently retains type uncertainty.
    let mut unknown = false;
    let values = if matches!(descriptor.upper, "0" | "1") {
        std::slice::from_ref(value)
    } else {
        value
            .as_array()
            .ok_or_else(|| {
                QueryError::invalid(element.id(), "template references must be a collection")
            })?
            .as_slice()
    };
    let mut known = Vec::new();
    for value in values {
        if external_reference_href(value)
            .map_err(|error| QueryError::invalid(element.id(), error))?
            .is_some()
        {
            unknown = true;
            continue;
        }
        if let Some(id) =
            reference_id(value).map_err(|error| QueryError::invalid(element.id(), error))?
        {
            known.push(
                references
                    .element(id)
                    .map_err(|error| QueryError::invalid(element.id(), error))?,
            );
        }
    }
    Ok(TemplateReferenceEvidence {
        known,
        unknown,
        cardinality: Some(values.len()),
    })
}

fn binding_signature_parameters<'a>(
    references: &crate::uml::references::ReferenceIndex<'a>,
    binding: crate::uml::root::ElementRef<'a>,
) -> Result<TemplateReferenceEvidence<'a>, crate::uml::classifiers::QueryError> {
    use crate::uml::classifiers::QueryError;
    let unknown = || {
        QueryError::unsupported(
            binding.id(),
            "TemplateBinding signature is not fully resolved",
        )
    };
    let signature = template_reference_evidence(references, binding, "signature")?;
    let signature = signature.known.first().copied().ok_or_else(unknown)?;
    // A local target contradiction is decidable before metaclass resolution.
    if let crate::uml::root::ElementRef::Relationship(relation) = binding
        && signature.id() != relation.to
    {
        return Err(QueryError::invalid(
            binding.id(),
            "signature projection contradicts the canonical TemplateBinding target",
        ));
    }
    if !signature.conforms_to("TemplateSignature") {
        return Err(unknown());
    }
    template_reference_evidence(references, signature, "parameter")
}

fn binding_unique_substitutions(
    references: &crate::uml::references::ReferenceIndex<'_>,
    binding: crate::uml::root::ElementRef<'_>,
) -> Result<bool, crate::uml::classifiers::QueryError> {
    use crate::uml::classifiers::QueryError;
    let unknown = || {
        QueryError::unsupported(
            binding.id(),
            "TemplateBinding substitution counts are not fully resolved",
        )
    };
    let substitutions =
        match template_reference_evidence(references, binding, "parameterSubstitution") {
            Ok(substitutions) => substitutions,
            Err(error @ QueryError::Invalid { .. }) => return Err(error),
            Err(QueryError::Unsupported { .. }) => TemplateReferenceEvidence {
                known: Vec::new(),
                unknown: true,
                cardinality: None,
            },
        };
    let parameters = match binding_signature_parameters(references, binding) {
        Ok(parameters) => Some(parameters),
        Err(error @ QueryError::Invalid { .. }) => return Err(error),
        Err(QueryError::Unsupported { .. }) => None,
    };
    let mut unknown_count = substitutions
        .cardinality
        .map(|size| size - substitutions.known.len());
    let mut counts = std::collections::HashMap::new();
    for substitution in substitutions.known {
        if !substitution.conforms_to("TemplateParameterSubstitution") {
            if let Some(count) = &mut unknown_count {
                *count += 1;
            }
            continue;
        }
        match template_reference_evidence(references, substitution, "formal") {
            Err(error @ QueryError::Invalid { .. }) => return Err(error),
            Err(QueryError::Unsupported { .. }) => {
                if let Some(count) = &mut unknown_count {
                    *count += 1;
                }
            }
            Ok(formals) => {
                if formals.unknown {
                    if let Some(count) = &mut unknown_count {
                        *count += 1;
                    }
                }
                for formal in formals.known {
                    *counts.entry(formal.id()).or_insert(0usize) += 1;
                }
            }
        }
    }
    // Each unresolved substitution can add at most one to a formal's count.
    // None means an unmaterialized collection, not a known empty collection.
    let at_most_one =
        |known: usize| known <= 1 && unknown_count.is_some_and(|unknown| unknown <= 1 - known);
    let greatest_count = counts.values().copied().max().unwrap_or(0);
    if at_most_one(greatest_count) {
        return Ok(true);
    }
    let Some(parameters) = parameters else {
        return Err(unknown());
    };
    let mut checks = Vec::new();
    for parameter in &parameters.known {
        let count = counts.get(parameter.id()).copied().unwrap_or(0);
        if count > 1 {
            checks.push(Ok(false));
        } else if at_most_one(count) {
            checks.push(Ok(true));
        } else {
            checks.push(Err(unknown()));
        }
    }
    if parameters.unknown {
        let greatest_unlisted_count = counts
            .iter()
            .filter(|(id, _)| {
                !parameters
                    .known
                    .iter()
                    .any(|parameter| parameter.id() == **id)
            })
            .map(|(_, count)| *count)
            .max()
            .unwrap_or(0);
        if !at_most_one(greatest_unlisted_count) {
            checks.push(Err(unknown()));
        }
    }
    crate::uml::feature_compatibility::require_all(checks)
}

fn binding_formals(
    references: &crate::uml::references::ReferenceIndex<'_>,
    binding: crate::uml::root::ElementRef<'_>,
) -> Result<bool, crate::uml::classifiers::QueryError> {
    use crate::uml::classifiers::QueryError;
    let substitutions = template_reference_evidence(references, binding, "parameterSubstitution");
    if substitutions
        .as_ref()
        .is_ok_and(|substitutions| substitutions.known.is_empty() && !substitutions.unknown)
    {
        return Ok(true);
    }
    let unknown = || {
        QueryError::unsupported(
            binding.id(),
            "TemplateBinding formal membership is not fully resolved",
        )
    };
    let mut checks = Vec::new();
    let substitutions = match substitutions {
        Ok(substitutions) => substitutions,
        Err(error) => {
            checks.push(Err(error));
            TemplateReferenceEvidence {
                known: Vec::new(),
                unknown: true,
                cardinality: None,
            }
        }
    };
    if substitutions.unknown {
        checks.push(Err(unknown()));
    }
    let parameters = binding_signature_parameters(references, binding);
    let parameters = match parameters {
        Ok(parameters) => Some(parameters),
        Err(error) => {
            checks.push(Err(error));
            None
        }
    };
    for substitution in substitutions.known {
        if !substitution.conforms_to("TemplateParameterSubstitution") {
            checks.push(Err(unknown()));
            continue;
        }
        match template_reference_evidence(references, substitution, "formal") {
            Err(error) => checks.push(Err(error)),
            Ok(formals) => {
                if formals.unknown {
                    checks.push(Err(unknown()));
                }
                for formal in formals.known {
                    if let Some(parameters) = &parameters {
                        if parameters
                            .known
                            .iter()
                            .any(|parameter| parameter.id() == formal.id())
                        {
                            checks.push(Ok(true));
                        } else if parameters.unknown {
                            checks.push(Err(unknown()));
                        } else {
                            checks.push(Ok(false));
                        }
                    }
                }
            }
        }
    }
    crate::uml::feature_compatibility::require_all(checks)
}

fn template_reference_is_kind(
    references: &crate::uml::references::ReferenceIndex<'_>,
    element: crate::uml::root::ElementRef<'_>,
    property: &str,
    metaclass: &str,
) -> Result<bool, crate::uml::classifiers::QueryError> {
    use crate::uml::classifiers::QueryError;
    let evidence = template_reference_evidence(references, element, property)?;
    let target = evidence.known.first().copied().ok_or_else(|| {
        QueryError::unsupported(element.id(), format!("{property} identity is not resolved"))
    })?;
    if target.metaclass().is_none() {
        return Err(QueryError::unsupported(
            element.id(),
            format!("{property} metaclass is not resolved"),
        ));
    }
    Ok(target.conforms_to(metaclass))
}

fn classifier_template_actual(
    references: &crate::uml::references::ReferenceIndex<'_>,
    substitution: crate::uml::root::ElementRef<'_>,
) -> Result<bool, crate::uml::classifiers::QueryError> {
    // formal is the mandatory opposite of templateParameterSubstitution in
    // A_formal_templateParameterSubstitution. Evaluate each canonical witness
    // without manufacturing an inverse list on ClassifierTemplateParameter.
    let not_classifier_parameter = template_reference_is_kind(
        references,
        substitution,
        "formal",
        "ClassifierTemplateParameter",
    )
    .map(|is_classifier_parameter| !is_classifier_parameter);
    let classifier_actual =
        template_reference_is_kind(references, substitution, "actual", "Classifier");
    crate::uml::feature_compatibility::accept_any([not_classifier_parameter, classifier_actual])
}

pub(crate) fn validate_bindings(
    references: &crate::uml::references::ReferenceIndex<'_>,
) -> Vec<StructuralDiagnostic> {
    use crate::uml::classifiers::QueryError;
    let mut issues = Vec::new();
    for binding in references.model().elements() {
        if binding.conforms_to("ClassifierTemplateParameter")
            && binding.properties().contains_key("templateParameterSubstitution")
        {
            // Forward formal links do not reconcile an independently authored
            // inverse collection. Keep that representation explicitly unverified.
            issues.push(StructuralDiagnostic {
                severity: StructuralSeverity::Warning,
                code: "MDE_CLASSIFIER_TEMPLATE_ACTUAL_UNVERIFIED",
                subject_id: binding.id().into(),
                message: "ClassifierTemplateParameter-actual_is_classifier: authored templateParameterSubstitution inverse projection is not reconciled; only canonical substitution.formal witnesses are evaluated".into(),
            });
        }
        if binding.conforms_to("TemplateParameterSubstitution") {
            let actual = match classifier_template_actual(references, binding) {
                Ok(true) => None,
                Ok(false) => Some((
                    StructuralSeverity::Error,
                    "UML_CLASSIFIER_TEMPLATE_ACTUAL",
                    "The actual argument of a ClassifierTemplateParameter must be a Classifier"
                        .to_owned(),
                )),
                Err(QueryError::Invalid { message, .. }) => Some((
                    StructuralSeverity::Error,
                    "MDE_CLASSIFIER_TEMPLATE_ACTUAL_INVALID",
                    message,
                )),
                Err(QueryError::Unsupported { message, .. }) => Some((
                    StructuralSeverity::Warning,
                    "MDE_CLASSIFIER_TEMPLATE_ACTUAL_UNVERIFIED",
                    message,
                )),
            };
            if let Some((severity, code, message)) = actual {
                issues.push(StructuralDiagnostic {
                    severity,
                    code,
                    subject_id: binding.id().into(),
                    message: format!("ClassifierTemplateParameter-actual_is_classifier: {message}"),
                });
            }
        }
        if !binding.conforms_to("TemplateBinding") {
            continue;
        }
        let unique = match binding_unique_substitutions(references, binding) {
            Ok(true) => None,
            Ok(false) => Some((
                StructuralSeverity::Error,
                "UML_TEMPLATE_BINDING_UNIQUE_SUBSTITUTION",
                "A target signature formal may have at most one substitution in this binding"
                    .to_owned(),
            )),
            Err(QueryError::Invalid { message, .. }) => Some((
                StructuralSeverity::Error,
                "MDE_TEMPLATE_BINDING_UNIQUE_SUBSTITUTION_INVALID",
                message,
            )),
            Err(QueryError::Unsupported { message, .. }) => Some((
                StructuralSeverity::Warning,
                "MDE_TEMPLATE_BINDING_UNIQUE_SUBSTITUTION_UNVERIFIED",
                message,
            )),
        };
        if let Some((severity, code, message)) = unique {
            issues.push(StructuralDiagnostic {
                severity,
                code,
                subject_id: binding.id().into(),
                message: format!("TemplateBinding-one_parameter_substitution: {message}"),
            });
        }
        let outcome = match binding_formals(references, binding) {
            Ok(true) => continue,
            Ok(false) => (
                StructuralSeverity::Error,
                "UML_TEMPLATE_BINDING_FORMAL",
                "Each substitution formal must belong to the target TemplateSignature".to_owned(),
            ),
            Err(QueryError::Invalid { message, .. }) => (
                StructuralSeverity::Error,
                "MDE_TEMPLATE_BINDING_FORMAL_INVALID",
                message,
            ),
            Err(QueryError::Unsupported { message, .. }) => (
                StructuralSeverity::Warning,
                "MDE_TEMPLATE_BINDING_FORMAL_UNVERIFIED",
                message,
            ),
        };
        issues.push(StructuralDiagnostic {
            severity: outcome.0,
            code: outcome.1,
            subject_id: binding.id().into(),
            message: format!(
                "TemplateBinding-parameter_substitution_formal: {}",
                outcome.2
            ),
        });
    }
    issues
}

pub(crate) fn validate(project: &Project) -> Vec<StructuralDiagnostic> {
    let mut issues = Vec::new();
    let owned = |owner: &str| {
        project
            .relations
            .iter()
            .filter(|relation| relation.kind == "owns" && relation.from == owner)
            .filter_map(|relation| {
                project
                    .elements
                    .iter()
                    .find(|element| element.id == relation.to)
            })
            .collect::<Vec<_>>()
    };
    let owners = |child: &str| {
        project
            .relations
            .iter()
            .filter(|relation| relation.kind == "owns" && relation.to == child)
            .filter_map(|relation| {
                project
                    .elements
                    .iter()
                    .find(|element| element.id == relation.from)
            })
            .collect::<Vec<_>>()
    };
    for element in &project.elements {
        if is_signature(&element.kind) {
            let signature_owners = owners(&element.id);
            if signature_owners.len() != 1
                || !signature_owners
                    .first()
                    .is_some_and(|owner| conforms(owner, "TemplateableElement"))
            {
                issues.push(issue(
                    "UML_TEMPLATE_SIGNATURE_OWNER",
                    &element.id,
                    "TemplateSignature must have exactly one TemplateableElement owner",
                ));
            }
            if let Some(owner) = signature_owners.first() {
                if conforms(owner, "Classifier")
                    && element.kind != "uml.redefinable_template_signature"
                {
                    issues.push(issue(
                        "UML_TEMPLATE_CLASSIFIER_SIGNATURE",
                        &element.id,
                        "Classifier ownedTemplateSignature must be a RedefinableTemplateSignature",
                    ));
                }
                if element.kind == "uml.redefinable_template_signature"
                    && !conforms(owner, "Classifier")
                {
                    issues.push(issue(
                        "UML_TEMPLATE_CLASSIFIER_SIGNATURE",
                        &element.id,
                        "RedefinableTemplateSignature must be owned by a Classifier",
                    ));
                }
                if owned(&owner.id)
                    .iter()
                    .filter(|child| is_signature(&child.kind))
                    .count()
                    > 1
                {
                    issues.push(issue(
                        "UML_TEMPLATE_SIGNATURE_MULTIPLICITY",
                        &owner.id,
                        "A TemplateableElement may own at most one TemplateSignature",
                    ));
                }
            }
            let parameters = owned(&element.id)
                .into_iter()
                .filter(|child| is_parameter(&child.kind))
                .collect::<Vec<_>>();
            if parameters.is_empty() {
                issues.push(issue(
                    "UML_TEMPLATE_SIGNATURE_PARAMETERS",
                    &element.id,
                    "TemplateSignature must have at least one parameter",
                ));
            }
            let mut names = HashSet::new();
            for parameter in parameters {
                for reference in project.relations.iter().filter(|relation| {
                    relation.kind == "uml.parametered_element" && relation.from == parameter.id
                }) {
                    if let Some(exposed) = project
                        .elements
                        .iter()
                        .find(|candidate| candidate.id == reference.to)
                        && !names.insert(exposed.name.as_str())
                    {
                        issues.push(issue(
                            "UML_TEMPLATE_PARAMETER_NAMES",
                            &element.id,
                            "Named parametered elements in a signature must have distinct names",
                        ));
                    }
                }
            }
        }
        if !is_parameter(&element.kind) {
            continue;
        }
        let parameter_owners = owners(&element.id);
        if parameter_owners.len() != 1
            || !parameter_owners
                .first()
                .is_some_and(|owner| is_signature(&owner.kind))
        {
            issues.push(issue("UML_TEMPLATE_PARAMETER_SIGNATURE", &element.id, "TemplateParameter must be owned by exactly one TemplateSignature; direct classifier ownership is not valid UML"));
        }
        let references = project
            .relations
            .iter()
            .filter(|relation| {
                relation.kind == "uml.parametered_element" && relation.from == element.id
            })
            .collect::<Vec<_>>();
        if references.len() != 1 {
            issues.push(issue(
                "UML_TEMPLATE_PARAMETERED_ELEMENT",
                &element.id,
                "TemplateParameter requires exactly one parameteredElement reference",
            ));
            continue;
        }
        let Some(reference) = references.first() else {
            continue;
        };
        let Some(exposed) = project
            .elements
            .iter()
            .find(|candidate| candidate.id == reference.to)
        else {
            issues.push(issue(
                "UML_TEMPLATE_PARAMETERED_ELEMENT",
                &element.id,
                "TemplateParameter parameteredElement must exist",
            ));
            continue;
        };
        let expected = if element.kind == "uml.classifier_template_parameter" {
            "Classifier"
        } else {
            "ParameterableElement"
        };
        if !conforms(exposed, expected) {
            issues.push(issue(
                "UML_TEMPLATE_PARAMETERED_TYPE",
                &element.id,
                "parameteredElement does not conform to the parameter's required metaclass",
            ));
        }
        let template = parameter_owners
            .first()
            .and_then(|signature| owners(&signature.id).first().map(|owner| owner.id.clone()));
        // UML.xmi TemplateSignature-own_elements requires the immediate owner,
        // despite the prose's broader "directly or indirectly" wording.
        let contained = project.relations.iter().any(|relation| {
            relation.kind == "owns"
                && relation.to == exposed.id
                && (relation.from == element.id
                    || template.as_deref() == Some(relation.from.as_str()))
        });
        if !contained {
            issues.push(issue(
                "UML_TEMPLATE_PARAMETERED_OWNERSHIP",
                &element.id,
                "parameteredElement must be owned directly by its template or by the TemplateParameter",
            ));
        }
        if element.kind == "uml.classifier_template_parameter" {
            if owned(&exposed.id)
                .iter()
                .any(|child| conforms(child, "Feature"))
            {
                issues.push(issue(
                    "UML_TEMPLATE_PARAMETERED_FEATURES",
                    &element.id,
                    "ClassifierTemplateParameter parameteredElement must not have features",
                ));
            }
            let constrained = project.relations.iter().any(|relation| {
                relation.kind == "uml.constraining_classifier" && relation.from == element.id
            });
            if element
                .properties
                .get("allowSubstitutable")
                .is_some_and(|value| !value.is_boolean())
            {
                issues.push(issue(
                    "UML_TEMPLATE_PARAMETER_FLAG_TYPE",
                    &element.id,
                    "allowSubstitutable must be a Boolean",
                ));
            }
            if constrained {
                // UML.xmi ClassifierTemplateParameter-
                // constraining_classifiers_constrain_parametered_element.
                // Identity and conformsTo decide the disjunction without requiring
                // Substitution. Do not confuse that alternative with structural
                // compatibility, or claim actual-argument substitution support.
                let index = crate::uml::references::ReferenceIndex::new(
                    crate::uml::root::Model::from_project(project),
                );
                let allow_substitutable = crate::uml::schema::read_effective_boolean(
                    crate::uml::root::ElementRef::Object(element), "allowSubstitutable",
                );
                for constraint in project.relations.iter().filter(|relation| {
                    relation.kind == "uml.constraining_classifier" && relation.from == element.id
                }) {
                    // Existing endpoint validation reports missing/wrong targets.
                    let Some(target) = project.elements.iter().find(|candidate| {
                        candidate.id == constraint.to && conforms(candidate, "Classifier")
                    }) else { continue; };
                    if !conforms(exposed, "Classifier") { continue; }
                    let conformance = if exposed.id == target.id {
                        Ok(true)
                    } else {
                        crate::uml::classifier_hierarchy::conforms_to(&index, &exposed.id, &target.id)
                    };
                    let decision = match conformance {
                        Ok(true) => continue,
                        result => match &allow_substitutable {
                            Ok(false) => result,
                            Ok(true) => crate::uml::feature_compatibility::accept_any([
                                result,
                                crate::uml::classifier_hierarchy::is_substitutable_for(
                                    &index, &exposed.id, &target.id,
                                ),
                            ]),
                            Err(error) => crate::uml::feature_compatibility::accept_any([
                                result, crate::uml::feature_compatibility::require_all([
                                    Err(crate::uml::classifiers::QueryError::unsupported(
                                        &element.id, error.to_string(),
                                    )),
                                    crate::uml::classifier_hierarchy::is_substitutable_for(
                                        &index, &exposed.id, &target.id,
                                    ),
                                ]),
                            ]),
                        },
                    };
                    let outcome = match decision {
                        Ok(true) => continue,
                        Ok(false) => (StructuralSeverity::Error,
                            "UML_TEMPLATE_PARAMETERED_CONSTRAINT",
                            "parameteredElement neither conforms nor satisfies the permitted Substitution alternative".to_owned()),
                        Err(crate::uml::classifiers::QueryError::Invalid { message, .. }) =>
                            (StructuralSeverity::Error, "UML_TEMPLATE_PARAMETERED_CONSTRAINT", message),
                        Err(error) => (StructuralSeverity::Warning,
                            "MDE_TEMPLATE_CONSTRAINT_UNSUPPORTED", error.to_string()),
                    };
                    // Keep all obligations: an unknown result cannot hide a
                    // proven violation of another constraining classifier.
                    issues.push(StructuralDiagnostic {
                        severity: outcome.0,
                        code: outcome.1,
                        subject_id: element.id.clone(),
                        message: format!("ClassifierTemplateParameter-constraining_classifiers_constrain_parametered_element [{}]: {}", target.id, outcome.2),
                    });
                }
            }
            if !constrained
                && (element
                    .properties
                    .get("allowSubstitutable")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(true)
                    || project.relations.iter().any(|relation| {
                        relation.kind == "uml.generalization" && relation.from == exposed.id
                    }))
            {
                issues.push(issue("UML_TEMPLATE_PARAMETER_CONSTRAINT", &element.id, "Substitutable or specialized parametered classifiers require a constraining classifier"));
            }
        }
    }
    for reference in project
        .relations
        .iter()
        .filter(|relation| relation.kind == "uml.parametered_element")
    {
        if !project
            .elements
            .iter()
            .any(|element| element.id == reference.from && is_parameter(&element.kind))
        {
            issues.push(issue(
                "UML_TEMPLATE_REFERENCE_SOURCE",
                &reference.id,
                "parameteredElement reference must originate at a TemplateParameter",
            ));
        }
    }
    for reference in project
        .relations
        .iter()
        .filter(|relation| relation.kind == "uml.constraining_classifier")
    {
        if !project.elements.iter().any(|element| {
            element.id == reference.from && element.kind == "uml.classifier_template_parameter"
        }) {
            issues.push(issue(
                "UML_TEMPLATE_CONSTRAINT_SOURCE",
                &reference.id,
                "constrainingClassifier must originate at a ClassifierTemplateParameter",
            ));
        }
        if !project
            .elements
            .iter()
            .any(|element| element.id == reference.to && conforms(element, "Classifier"))
        {
            issues.push(issue(
                "UML_TEMPLATE_CONSTRAINING_TYPE",
                &reference.id,
                "constrainingClassifier must reference a Classifier",
            ));
        }
    }
    issues
}

pub(crate) fn validate_changes(before: &Project, after: &Project) -> Result<(), String> {
    let existing = validate(before);
    for diagnostic in validate(after) {
        if !existing.iter().any(|previous| {
            previous.code == diagnostic.code
                && previous.subject_id == diagnostic.subject_id
                && previous.message == diagnostic.message
        }) {
            return Err(format!("{}: {}", diagnostic.code, diagnostic.message));
        }
    }
    Ok(())
}

pub(crate) fn extend_deletion(
    section: &ModelingSection,
    deleted: &mut HashSet<String>,
) -> Result<(), String> {
    for signature in section
        .elements
        .iter()
        .filter(|element| is_signature(&element.kind) && !deleted.contains(&element.id))
        .collect::<Vec<_>>()
    {
        let children = section
            .relations
            .iter()
            .filter(|relation| relation.kind == "owns" && relation.from == signature.id)
            .map(|relation| relation.to.as_str())
            .collect::<Vec<_>>();
        if !children.is_empty()
            && children.iter().all(|child| deleted.contains(*child))
            && signature.label.is_none()
            && signature.description.is_none()
            && signature.content.is_none()
            && signature.annotations.is_empty()
            && signature.implementations.is_empty()
            && signature.properties.len() == 1
            && signature
                .properties
                .get(GENERATED_SIGNATURE)
                .and_then(|metadata| metadata.get("name"))
                .and_then(serde_json::Value::as_str)
                == Some(signature.name.as_str())
        {
            deleted.insert(signature.id.clone());
        }
    }
    for element in section
        .elements
        .iter()
        .filter(|element| deleted.contains(&element.id))
    {
        let template_node = is_signature(&element.kind)
            || is_parameter(&element.kind)
            || section.relations.iter().any(|relation| {
                relation.kind == "uml.parametered_element" && relation.to == element.id
            });
        if template_node
            && section.relations.iter().any(|relation| {
                relation.kind != "owns"
                    && relation.to == element.id
                    && !deleted.contains(&relation.from)
            })
        {
            return Err(format!(
                "UML_TEMPLATE_IN_USE: {} is referenced outside the deleted template graph",
                element.id
            ));
        }
    }
    Ok(())
}
