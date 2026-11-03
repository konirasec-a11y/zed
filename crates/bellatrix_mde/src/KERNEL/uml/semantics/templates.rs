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

fn template_reference_target<'a>(
    references: &crate::uml::references::ReferenceIndex<'a>,
    element: crate::uml::root::ElementRef<'a>,
    property: &str,
) -> Result<crate::uml::root::ElementRef<'a>, crate::uml::classifiers::QueryError> {
    let evidence = template_reference_evidence(references, element, property)?;
    evidence.known.first().copied().ok_or_else(|| {
        crate::uml::classifiers::QueryError::unsupported(
            element.id(),
            format!("{property} identity is not resolved"),
        )
    })
}

fn checked_parametered_element<'a>(
    references: &crate::uml::references::ReferenceIndex<'a>,
    formal: crate::uml::root::ElementRef<'a>,
    required_metaclass: &str,
) -> Result<crate::uml::root::ElementRef<'a>, crate::uml::classifiers::QueryError> {
    use crate::uml::classifiers::QueryError;
    let mut relations = references
        .model()
        .relationship_records()
        .filter(|relation| {
            relation.kind == "uml.parametered_element" && relation.from == formal.id()
        });
    let canonical = relations.next();
    if relations.next().is_some() {
        return Err(QueryError::invalid(
            formal.id(),
            "parameteredElement has ambiguous canonical references",
        ));
    }
    let canonical = canonical
        .map(|relation| {
            references
                .element(&relation.to)
                .map_err(|error| QueryError::invalid(formal.id(), error))
        })
        .transpose()?;
    let explicit = formal
        .properties()
        .contains_key("parameteredElement")
        .then(|| template_reference_target(references, formal, "parameteredElement"));
    if let Some(Err(error @ QueryError::Invalid { .. })) = &explicit {
        return Err(error.clone());
    }
    if let (Some(canonical), Some(Ok(explicit))) = (canonical, &explicit)
        && canonical.id() != explicit.id()
    {
        return Err(QueryError::invalid(
            formal.id(),
            "parameteredElement projection contradicts its canonical reference",
        ));
    }
    let descriptor = formal
        .metaclass()
        .ok_or_else(|| QueryError::unsupported(formal.id(), "formal metaclass is unresolved"))
        .and_then(|metaclass| {
            crate::uml::schema::uml_property(metaclass, "parameteredElement")
                .map_err(|error| QueryError::unsupported(formal.id(), error.to_string()))
        })?
        .ok_or_else(|| {
            QueryError::unsupported(formal.id(), "parameteredElement contract is unavailable")
        })?;
    let mut unresolved = None;
    for target in canonical.into_iter().chain(
        explicit
            .as_ref()
            .and_then(|value| value.as_ref().ok())
            .copied(),
    ) {
        // Canonical references obey the same specialized range as an authored
        // property (e.g. OperationTemplateParameter exposes an Operation).
        match crate::uml::property_contracts::check_value(
            references,
            formal,
            descriptor,
            &serde_json::json!({"$ref": target.id()}),
        ) {
            crate::uml::property_contracts::PropertyOutcome::Satisfied => {}
            crate::uml::property_contracts::PropertyOutcome::Violation(message) => {
                return Err(QueryError::invalid(formal.id(), message));
            }
            crate::uml::property_contracts::PropertyOutcome::Unverified(message) => {
                unresolved = Some(QueryError::unsupported(formal.id(), message))
            }
        }
        if target.metaclass().is_none() {
            unresolved = Some(QueryError::unsupported(
                formal.id(),
                "parameteredElement metaclass is unresolved",
            ));
        } else if !target.conforms_to(required_metaclass) {
            return Err(QueryError::invalid(
                formal.id(),
                format!("parameteredElement must be a {required_metaclass}"),
            ));
        }
    }
    if let Some(error) = unresolved {
        return Err(error);
    }
    match explicit {
        Some(result) => result,
        None => canonical.ok_or_else(|| {
            QueryError::unsupported(
                formal.id(),
                "parameteredElement projection is not materialized",
            )
        }),
    }
}

fn checked_parametered_classifier<'a>(
    references: &crate::uml::references::ReferenceIndex<'a>,
    formal: crate::uml::root::ElementRef<'a>,
) -> Result<crate::uml::root::ElementRef<'a>, crate::uml::classifiers::QueryError> {
    checked_parametered_element(references, formal, "Classifier")
}

fn classifier_template_matching_abstract(
    references: &crate::uml::references::ReferenceIndex<'_>,
    substitution: crate::uml::root::ElementRef<'_>,
) -> Result<bool, crate::uml::classifiers::QueryError> {
    use crate::uml::{classifiers::QueryError, feature_compatibility::accept_any, schema};
    let (specialized, exposed_abstract) =
        match template_reference_target(references, substitution, "formal") {
            Err(error @ QueryError::Invalid { .. }) => return Err(error),
            Err(error) => (false, Err(error)),
            Ok(formal) if formal.metaclass().is_none() => (
                false,
                Err(QueryError::unsupported(
                    formal.id(),
                    "formal metaclass is unresolved",
                )),
            ),
            Ok(formal) if !formal.conforms_to("ClassifierTemplateParameter") => {
                // Do not apply the Classifier cast to an unrelated formal kind.
                return accept_any([
                    Ok(true),
                    template_reference_target(references, substitution, "actual").map(|_| true),
                ]);
            }
            Ok(formal) => (
                true,
                checked_parametered_classifier(references, formal).and_then(|exposed| {
                    schema::read_effective_boolean(exposed, "isAbstract")
                        .map_err(|error| QueryError::invalid(exposed.id(), error))
                }),
            ),
        };
    let actual_not_abstract = template_reference_target(references, substitution, "actual")
        .and_then(|actual| {
            if actual.metaclass().is_none() {
                return Err(QueryError::unsupported(
                    actual.id(),
                    "actual metaclass is unresolved",
                ));
            }
            if !actual.conforms_to("Classifier") {
                return Err(if specialized {
                    QueryError::invalid(
                        actual.id(),
                        "matching_abstract requires a Classifier actual",
                    )
                } else {
                    QueryError::unsupported(
                        actual.id(),
                        "formal applicability is unresolved for non-Classifier actual",
                    )
                });
            }
            schema::read_effective_boolean(actual, "isAbstract")
                .map(|is_abstract| !is_abstract)
                .map_err(|error| QueryError::invalid(actual.id(), error))
        });
    accept_any([exposed_abstract, actual_not_abstract])
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

fn classifier_template_constraints<'a>(
    references: &crate::uml::references::ReferenceIndex<'a>,
    formal: crate::uml::root::ElementRef<'a>,
) -> Result<TemplateReferenceEvidence<'a>, crate::uml::classifiers::QueryError> {
    use crate::uml::classifiers::QueryError;
    let canonical = references
        .model()
        .relationship_records()
        .filter(|relation| {
            relation.kind == "uml.constraining_classifier" && relation.from == formal.id()
        })
        .map(|relation| {
            references
                .element(&relation.to)
                .map_err(|error| QueryError::invalid(formal.id(), error))
        })
        .collect::<Result<Vec<_>, _>>()?;
    for target in &canonical {
        if target.metaclass().is_some() && !target.conforms_to("Classifier") {
            return Err(QueryError::invalid(
                formal.id(),
                "constrainingClassifier must be a Classifier",
            ));
        }
    }
    if formal.properties().contains_key("constrainingClassifier") {
        let mut explicit =
            template_reference_evidence(references, formal, "constrainingClassifier")?;
        if !canonical.is_empty() {
            if explicit
                .known
                .iter()
                .any(|target| !canonical.iter().any(|other| other.id() == target.id()))
                || (!explicit.unknown
                    && canonical.iter().any(|target| {
                        !explicit.known.iter().any(|other| other.id() == target.id())
                    }))
            {
                return Err(QueryError::invalid(
                    formal.id(),
                    "constrainingClassifier projection contradicts canonical references",
                ));
            }
            for target in canonical {
                if !explicit.known.iter().any(|other| other.id() == target.id()) {
                    explicit.known.push(target);
                }
            }
        }
        return Ok(explicit);
    }
    if canonical.is_empty() {
        return Err(QueryError::unsupported(
            formal.id(),
            "constrainingClassifier extent is not materialized",
        ));
    }
    Ok(TemplateReferenceEvidence {
        cardinality: Some(canonical.len()),
        known: canonical,
        unknown: false,
    })
}

fn classifier_template_constrained_actual(
    references: &crate::uml::references::ReferenceIndex<'_>,
    substitution: crate::uml::root::ElementRef<'_>,
) -> Result<bool, crate::uml::classifiers::QueryError> {
    use crate::uml::{
        classifier_hierarchy,
        classifiers::QueryError,
        feature_compatibility::{accept_any, require_all},
        schema,
    };
    let formal = template_reference_target(references, substitution, "formal");
    let actual = template_reference_target(references, substitution, "actual");
    // Validate independent reference shapes even if the formal cannot be resolved.
    if let Err(error @ QueryError::Invalid { .. }) = &actual {
        return Err(error.clone());
    }
    let formal = formal?;
    if formal.metaclass().is_none() {
        return Err(QueryError::unsupported(
            formal.id(),
            "formal metaclass is unresolved",
        ));
    }
    if !formal.conforms_to("ClassifierTemplateParameter") {
        return Ok(true);
    }
    let constraints = classifier_template_constraints(references, formal);
    let allow = schema::read_effective_boolean(formal, "allowSubstitutable")
        .map_err(|error| QueryError::invalid(formal.id(), error));
    if let Err(error @ QueryError::Invalid { .. }) = &allow {
        return Err(error.clone());
    }
    // A known non-Classifier actual cannot satisfy the normative cast, even
    // when the inner collection is empty. External actuals remain unresolved.
    if let Ok(actual) = actual {
        if actual.metaclass().is_some() && !actual.conforms_to("Classifier") {
            return Err(QueryError::invalid(
                actual.id(),
                "constraining_classifiers_constrain_args requires a Classifier actual",
            ));
        }
    }
    let constraints = constraints?;
    let mut checks = Vec::new();
    if constraints.unknown {
        checks.push(Err(QueryError::unsupported(
            formal.id(),
            "constrainingClassifier includes external identities",
        )));
    }
    for constraint in constraints.known {
        checks.push((|| {
            let actual = actual.clone()?;
            if actual.metaclass().is_none() || constraint.metaclass().is_none() {
                // Exact identity proves the first disjunct without guessing type
                // conformance. Independent property contracts retain type Unknown.
                return if actual.id() == constraint.id() {
                    Ok(true)
                } else {
                    Err(QueryError::unsupported(
                        actual.id(),
                        "Classifier metaclass is unresolved",
                    ))
                };
            }
            let conformance = if actual.id() == constraint.id() {
                Ok(true)
            } else {
                classifier_hierarchy::has_ancestor_matching(references, actual.id(), |parent| {
                    parent.id() == constraint.id()
                })
            };
            if matches!(conformance, Ok(true)) {
                return conformance;
            }
            let substitutable = match allow.clone()? {
                false => Ok(false),
                true => classifier_hierarchy::is_substitutable_for(
                    references,
                    actual.id(),
                    constraint.id(),
                ),
            };
            accept_any([conformance, substitutable])
        })());
    }
    require_all(checks)
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

fn template_substitution_compatible(
    references: &crate::uml::references::ReferenceIndex<'_>,
    substitution: crate::uml::root::ElementRef<'_>,
) -> Result<bool, crate::uml::classifiers::QueryError> {
    use crate::uml::classifiers::QueryError;
    let actual = template_reference_target(references, substitution, "actual");
    let exposed =
        template_reference_target(references, substitution, "formal").and_then(|formal| {
            if formal.metaclass().is_none() {
                return Err(QueryError::unsupported(
                    formal.id(),
                    "formal metaclass is unresolved",
                ));
            }
            checked_parametered_element(references, formal, "ParameterableElement")
        });
    parameterable_compatible(actual, exposed)
}

fn parameterable_compatible(
    actual: Result<crate::uml::root::ElementRef<'_>, crate::uml::classifiers::QueryError>,
    exposed: Result<crate::uml::root::ElementRef<'_>, crate::uml::classifiers::QueryError>,
) -> Result<bool, crate::uml::classifiers::QueryError> {
    use crate::uml::classifiers::QueryError;
    // A malformed independent operand must not disappear behind Unknown.
    for operand in [&actual, &exposed] {
        if let Err(error @ QueryError::Invalid { .. }) = operand {
            return Err(error.clone());
        }
    }
    let actual = actual?;
    let exposed = exposed?;
    let exposed_metaclass = exposed.metaclass().ok_or_else(|| {
        QueryError::unsupported(exposed.id(), "parameteredElement metaclass is unresolved")
    })?;
    if actual.metaclass().is_none() {
        return Err(QueryError::unsupported(
            actual.id(),
            "actual metaclass is unresolved",
        ));
    }
    // These UML operations override the base query. Property's published body
    // additionally references TypeElement (not TypedElement); do not silently
    // repair it or substitute the base algorithm for either override.
    if actual.conforms_to("Property") || actual.conforms_to("ValueSpecification") {
        return Err(QueryError::unsupported(
            actual.id(),
            "Property/ValueSpecification isCompatibleWith override is not implemented by this base-query adapter",
        ));
    }
    // ParameterableElement::isCompatibleWith compares UML metaclasses, not
    // model-level Classifier generalization, equality, or substitutability.
    Ok(actual.conforms_to(exposed_metaclass))
}

fn template_default_compatible(
    references: &crate::uml::references::ReferenceIndex<'_>,
    parameter: crate::uml::root::ElementRef<'_>,
) -> Result<bool, crate::uml::classifiers::QueryError> {
    use crate::uml::{
        classifiers::{self, QueryError},
        feature_compatibility::require_all,
    };
    let owned = parameter
        .properties()
        .get("ownedDefault")
        .filter(|value| !value.is_null())
        .map(|_| template_reference_target(references, parameter, "ownedDefault"));
    if let Some(Err(error @ QueryError::Invalid { .. })) = &owned {
        return Err(error.clone());
    }
    if parameter
        .properties()
        .get("default")
        .is_none_or(serde_json::Value::is_null)
    {
        if owned.is_some() {
            return Err(QueryError::unsupported(
                parameter.id(),
                "default/ownedDefault projection is not materialized",
            ));
        }
        for child in references
            .explicit_owned_elements(parameter.id())
            .map_err(|error| QueryError::invalid(parameter.id(), error))?
        {
            if (child.metaclass().is_none() || child.conforms_to("ParameterableElement"))
                && classifiers::contributes_to_owned_union(
                    parameter,
                    child,
                    "TemplateParameter-ownedDefault",
                )?
            {
                return Err(QueryError::unsupported(
                    parameter.id(),
                    "default/ownedDefault role is not materialized",
                ));
            }
        }
        return Ok(true);
    }
    let default = template_reference_target(references, parameter, "default");
    let owned_consistency = match owned {
        None => Ok(true),
        Some(owned) => match (&default, owned) {
            (Ok(default), Ok(owned)) if default.id() != owned.id() => Err(QueryError::invalid(
                parameter.id(),
                "ownedDefault contradicts its default superset",
            )),
            (Ok(_), Ok(_)) => Ok(true),
            (_, Err(error)) => Err(error),
            (Err(error), _) => Err(error.clone()),
        },
    };
    require_all([
        owned_consistency,
        parameterable_compatible(
            default,
            checked_parametered_element(references, parameter, "ParameterableElement"),
        ),
    ])
}

fn template_signature_unique_parameters(
    references: &crate::uml::references::ReferenceIndex<'_>,
    signature: crate::uml::root::ElementRef<'_>,
) -> Result<bool, crate::uml::classifiers::QueryError> {
    use crate::uml::{classifiers::QueryError, feature_compatibility::require_all};
    let parameters = template_reference_evidence(references, signature, "parameter")?;
    let mut unknown = parameters
        .cardinality
        .unwrap_or(parameters.known.len())
        .saturating_sub(parameters.known.len());
    let mut names = std::collections::HashMap::new();
    let mut seen = HashSet::new();
    let mut checks = Vec::new();
    for parameter in parameters.known {
        // The inequality guard compares formal identities, not exposed elements.
        if !seen.insert(parameter.id()) {
            continue;
        }
        if parameter.metaclass().is_none() {
            unknown += 1;
            continue;
        }
        match checked_parametered_element(references, parameter, "ParameterableElement") {
            Err(error @ QueryError::Invalid { .. }) => checks.push(Err(error)),
            Err(QueryError::Unsupported { .. }) => unknown += 1,
            Ok(exposed) => {
                if let Some(named) = exposed.named() {
                    if matches!(exposed, crate::uml::root::ElementRef::Relationship(_))
                        && let Some(value) = exposed.properties().get("name")
                    {
                        let descriptor = crate::uml::schema::uml_property("NamedElement", "name")
                            .map_err(|error| {
                                QueryError::unsupported(exposed.id(), error.to_string())
                            })?
                            .ok_or_else(|| {
                                QueryError::unsupported(
                                    exposed.id(),
                                    "NamedElement name contract is unavailable",
                                )
                            })?;
                        match crate::uml::property_contracts::check_value(
                            references, exposed, descriptor, value,
                        ) {
                            crate::uml::property_contracts::PropertyOutcome::Satisfied => {}
                            crate::uml::property_contracts::PropertyOutcome::Violation(message) => {
                                checks.push(Err(QueryError::invalid(exposed.id(), message)));
                                continue;
                            }
                            crate::uml::property_contracts::PropertyOutcome::Unverified(
                                message,
                            ) => {
                                checks.push(Err(QueryError::unsupported(exposed.id(), message)));
                                continue;
                            }
                        }
                    }
                    // Empty editor text is UML's absent name. Two absent names
                    // compare equal in this constraint; neither is invented.
                    if names.insert(named.name(), parameter.id()).is_some() {
                        checks.push(Ok(false));
                    }
                }
            }
        }
    }
    if unknown > 0 && unknown + names.len() > 1 {
        checks.push(Err(QueryError::unsupported(
            signature.id(),
            "parameter names or identities are not fully resolved",
        )));
    }
    require_all(checks)
}

fn signature_redefines_parents(
    references: &crate::uml::references::ReferenceIndex<'_>,
    signature: crate::uml::root::ElementRef<'_>,
) -> Result<bool, crate::uml::classifiers::QueryError> {
    use crate::uml::{classifiers::QueryError, feature_compatibility::require_all};
    let signature = references.element(signature.id())
        .map_err(|error| QueryError::invalid(signature.id(), error))?;
    let mut extended = HashSet::new();
    let mut pending = vec![signature];
    let mut visited = HashSet::new();
    let mut unknown_extension = false;
    while let Some(current) = pending.pop() {
        if !visited.insert(current.id().to_owned()) { continue; }
        if current.metaclass().is_none() {
            unknown_extension = true;
            continue;
        }
        if !current.properties().contains_key("extendedSignature") { continue; }
        let evidence = template_reference_evidence(references, current, "extendedSignature")?;
        unknown_extension |= evidence.unknown;
        for target in evidence.known {
            extended.insert(target.id().to_owned());
            pending.push(target);
        }
    }
    let owner = references.model().owner(signature)
        .map_err(|error| QueryError::invalid(signature.id(), error))?
        .ok_or_else(|| QueryError::invalid(signature.id(), "RedefinableTemplateSignature requires a Classifier owner"))?;
    let owner = references.element(owner.id()).map_err(|error| QueryError::invalid(signature.id(), error))?;
    let mut checks = Vec::new();
    for property in ["classifier", "template"] {
        if let Some(value) = signature.properties().get(property) {
            // classifier redefines TemplateSignature::template, so resolving
            // template on the specialized metaclass intentionally omits it.
            let metaclass = if property == "classifier" { "RedefinableTemplateSignature" } else { "TemplateSignature" };
            let descriptor = crate::uml::schema::uml_property(metaclass, property)
                .map_err(|error| QueryError::unsupported(signature.id(), error.to_string()))?
                .ok_or_else(|| QueryError::unsupported(signature.id(), "signature owner contract is unavailable"))?;
            match crate::uml::property_contracts::check_value(references, signature, descriptor, value) {
                crate::uml::property_contracts::PropertyOutcome::Satisfied => {}
                crate::uml::property_contracts::PropertyOutcome::Violation(message) => return Err(QueryError::invalid(signature.id(), message)),
                crate::uml::property_contracts::PropertyOutcome::Unverified(message) => checks.push(Err(QueryError::unsupported(signature.id(), message))),
            }
            if crate::uml::references::external_reference_href(value)
                .map_err(|error| QueryError::invalid(signature.id(), error))?.is_none()
                && crate::uml::references::reference_id(value)
                    .map_err(|error| QueryError::invalid(signature.id(), error))? != Some(owner.id())
            {
                return Err(QueryError::invalid(signature.id(), "signature owner projection contradicts canonical ownership"));
            }
        }
    }
    if owner.metaclass().is_none() {
        return Err(QueryError::unsupported(signature.id(), "signature Classifier owner is unresolved"));
    }
    if !owner.conforms_to("Classifier") {
        return Err(QueryError::invalid(signature.id(), "signature owner must be a Classifier"));
    }
    // Retain known ancestors even when another hierarchy branch is external.
    // Returning false keeps the hierarchy query's unresolved evidence visible.
    let ancestors = std::cell::RefCell::new(std::collections::BTreeSet::new());
    checks.push(crate::uml::classifier_hierarchy::has_ancestor_matching(
        references, owner.id(), |ancestor| {
            ancestors.borrow_mut().insert(ancestor.id().to_owned());
            false
        },
    ).map(|_| true));
    for ancestor_id in ancestors.into_inner() {
        let ancestor = references.element(&ancestor_id)
            .map_err(|error| QueryError::invalid(signature.id(), error))?;
        let children = references.explicit_owned_elements(ancestor.id())
            .map_err(|error| QueryError::invalid(ancestor.id(), error))?;
        let owned = children.iter().copied().filter(|child| child.conforms_to("TemplateSignature")).collect::<Vec<_>>();
        if owned.len() > 1 {
            return Err(QueryError::invalid(ancestor.id(), "Classifier may own at most one template signature"));
        }
        if children.iter().any(|child| child.metaclass().is_none()) {
            checks.push(Err(QueryError::unsupported(ancestor.id(), "unknown owned metaclass may be a template signature")));
        }
        let mut required = owned.clone();
        if ancestor.properties().contains_key("ownedTemplateSignature") {
            let evidence = template_reference_evidence(references, ancestor, "ownedTemplateSignature")?;
            let authored = evidence.known.iter().map(|target| target.id()).collect::<HashSet<_>>();
            let known_owned = owned.iter().map(|target| target.id()).collect::<HashSet<_>>();
            let candidates = children.iter().map(|target| target.id()).collect::<HashSet<_>>();
            // Unknown owned metaclasses remain candidates, not evidence that
            // the canonical ownedTemplateSignature collection is empty.
            if !evidence.unknown && (!known_owned.is_subset(&authored) || !authored.is_subset(&candidates)) {
                return Err(QueryError::invalid(ancestor.id(), "ownedTemplateSignature contradicts canonical ownership"));
            }
            if evidence.unknown {
                checks.push(Err(QueryError::unsupported(ancestor.id(), "ownedTemplateSignature is external")));
            }
            for target in evidence.known {
                if !required.iter().any(|known| known.id() == target.id()) {
                    required.push(target);
                }
            }
        }
        for required in required {
            if extended.contains(required.id()) {
                checks.push(Ok(true));
            } else if unknown_extension {
                checks.push(Err(QueryError::unsupported(signature.id(), "external extendedSignature may supply required ancestor signature")));
            } else {
                checks.push(Ok(false));
            }
        }
    }
    require_all(checks)
}

pub(crate) fn validate_bindings(
    references: &crate::uml::references::ReferenceIndex<'_>,
) -> Vec<StructuralDiagnostic> {
    use crate::uml::classifiers::QueryError;
    let mut issues = Vec::new();
    for binding in references.model().elements() {
        if binding.conforms_to("RedefinableTemplateSignature") {
            let outcome = match signature_redefines_parents(references, binding) {
                Ok(true) => None,
                Ok(false) => Some((StructuralSeverity::Error, "UML_TEMPLATE_SIGNATURE_REDEFINES_PARENTS", "extendedSignature closure must include every ancestor Classifier's ownedTemplateSignature".to_owned())),
                Err(QueryError::Invalid { message, .. }) => Some((StructuralSeverity::Error, "MDE_TEMPLATE_SIGNATURE_REDEFINES_PARENTS_INVALID", message)),
                Err(QueryError::Unsupported { message, .. }) => Some((StructuralSeverity::Warning, "MDE_TEMPLATE_SIGNATURE_REDEFINES_PARENTS_UNVERIFIED", message)),
            };
            if let Some((severity, code, message)) = outcome {
                issues.push(StructuralDiagnostic { severity, code, subject_id: binding.id().into(),
                    message: format!("RedefinableTemplateSignature-redefines_parents: {message}") });
            }
        }
        if binding.conforms_to("TemplateSignature") {
            let unique = match template_signature_unique_parameters(references, binding) {
                Ok(true) => None,
                Ok(false) => Some((
                    StructuralSeverity::Error,
                    "UML_TEMPLATE_SIGNATURE_UNIQUE_PARAMETERS",
                    "Distinct formal parameters exposing NamedElements must have distinct exposed names".to_owned(),
                )),
                Err(QueryError::Invalid { message, .. }) => Some((
                    StructuralSeverity::Error,
                    "MDE_TEMPLATE_SIGNATURE_UNIQUE_PARAMETERS_INVALID",
                    message,
                )),
                Err(QueryError::Unsupported { message, .. }) => Some((
                    StructuralSeverity::Warning,
                    "MDE_TEMPLATE_SIGNATURE_UNIQUE_PARAMETERS_UNVERIFIED",
                    message,
                )),
            };
            if let Some((severity, code, message)) = unique {
                issues.push(StructuralDiagnostic {
                    severity, code, subject_id: binding.id().into(),
                    message: format!("TemplateSignature-unique_parameters: {message}"),
                });
            }
        }

        if binding.conforms_to("TemplateParameter") {
            let default = match template_default_compatible(references, binding) {
                Ok(true) => None,
                Ok(false) => Some((
                    StructuralSeverity::Error,
                    "UML_TEMPLATE_DEFAULT_COMPATIBLE",
                    "Default UML metaclass must equal or specialize the exposed ParameterableElement metaclass".to_owned(),
                )),
                Err(QueryError::Invalid { message, .. }) => Some((
                    StructuralSeverity::Error,
                    "MDE_TEMPLATE_DEFAULT_COMPATIBLE_INVALID",
                    message,
                )),
                Err(QueryError::Unsupported { message, .. }) => Some((
                    StructuralSeverity::Warning,
                    "MDE_TEMPLATE_DEFAULT_COMPATIBLE_UNVERIFIED",
                    message,
                )),
            };
            if let Some((severity, code, message)) = default {
                issues.push(StructuralDiagnostic {
                    severity,
                    code,
                    subject_id: binding.id().into(),
                    message: format!("TemplateParameter-must_be_compatible: {message}"),
                });
            }
        }
        if binding.conforms_to("ClassifierTemplateParameter")
            && binding
                .properties()
                .contains_key("templateParameterSubstitution")
        {
            issues.push(StructuralDiagnostic {
                severity: StructuralSeverity::Warning,
                code: "MDE_CLASSIFIER_TEMPLATE_CONSTRAINED_ACTUAL_UNVERIFIED",
                subject_id: binding.id().into(),
                message: "ClassifierTemplateParameter-constraining_classifiers_constrain_args: authored templateParameterSubstitution inverse projection is not reconciled; only canonical substitution.formal witnesses are evaluated".into(),
            });
            // Forward formal links do not reconcile an independently authored
            // inverse collection. Keep that representation explicitly unverified.
            issues.push(StructuralDiagnostic {
                severity: StructuralSeverity::Warning,
                code: "MDE_CLASSIFIER_TEMPLATE_MATCHING_ABSTRACT_UNVERIFIED",
                subject_id: binding.id().into(),
                message: "ClassifierTemplateParameter-matching_abstract: authored templateParameterSubstitution inverse projection is not reconciled; only canonical substitution.formal witnesses are evaluated".into(),
            });
            issues.push(StructuralDiagnostic {
                severity: StructuralSeverity::Warning,
                code: "MDE_CLASSIFIER_TEMPLATE_ACTUAL_UNVERIFIED",
                subject_id: binding.id().into(),
                message: "ClassifierTemplateParameter-actual_is_classifier: authored templateParameterSubstitution inverse projection is not reconciled; only canonical substitution.formal witnesses are evaluated".into(),
            });
        }
        if binding.conforms_to("TemplateParameterSubstitution") {
            let compatible = match template_substitution_compatible(references, binding) {
                Ok(true) => None,
                Ok(false) => Some((
                    StructuralSeverity::Error,
                    "UML_TEMPLATE_SUBSTITUTION_COMPATIBLE",
                    "Actual UML metaclass must equal or specialize the exposed ParameterableElement metaclass".to_owned(),
                )),
                Err(QueryError::Invalid { message, .. }) => Some((
                    StructuralSeverity::Error,
                    "MDE_TEMPLATE_SUBSTITUTION_COMPATIBLE_INVALID",
                    message,
                )),
                Err(QueryError::Unsupported { message, .. }) => Some((
                    StructuralSeverity::Warning,
                    "MDE_TEMPLATE_SUBSTITUTION_COMPATIBLE_UNVERIFIED",
                    message,
                )),
            };
            if let Some((severity, code, message)) = compatible {
                issues.push(StructuralDiagnostic {
                    severity,
                    code,
                    subject_id: binding.id().into(),
                    message: format!("TemplateParameterSubstitution-must_be_compatible: {message}"),
                });
            }
            let constrained = match classifier_template_constrained_actual(references, binding) {
                        Ok(true) => None,
                        Ok(false) => Some((StructuralSeverity::Error, "UML_CLASSIFIER_TEMPLATE_CONSTRAINED_ACTUAL", "Actual must equal, conform to, or satisfy the permitted Substitution alternative for every constraining Classifier".to_owned())),
                        Err(QueryError::Invalid { message, .. }) => Some((StructuralSeverity::Error, "MDE_CLASSIFIER_TEMPLATE_CONSTRAINED_ACTUAL_INVALID", message)),
                        Err(QueryError::Unsupported { message, .. }) => Some((StructuralSeverity::Warning, "MDE_CLASSIFIER_TEMPLATE_CONSTRAINED_ACTUAL_UNVERIFIED", message)),
                    };
            if let Some((severity, code, message)) = constrained {
                issues.push(StructuralDiagnostic {
                    severity,
                    code,
                    subject_id: binding.id().into(),
                    message: format!(
                        "ClassifierTemplateParameter-constraining_classifiers_constrain_args: {message}"
                    ),
                });
            }
            let abstract_outcome = match classifier_template_matching_abstract(references, binding)
            {
                Ok(true) => None,
                Ok(false) => Some((
                    StructuralSeverity::Error,
                    "UML_CLASSIFIER_TEMPLATE_MATCHING_ABSTRACT",
                    "A nonabstract exposed Classifier requires a nonabstract actual Classifier"
                        .to_owned(),
                )),
                Err(QueryError::Invalid { message, .. }) => Some((
                    StructuralSeverity::Error,
                    "MDE_CLASSIFIER_TEMPLATE_MATCHING_ABSTRACT_INVALID",
                    message,
                )),
                Err(QueryError::Unsupported { message, .. }) => Some((
                    StructuralSeverity::Warning,
                    "MDE_CLASSIFIER_TEMPLATE_MATCHING_ABSTRACT_UNVERIFIED",
                    message,
                )),
            };
            if let Some((severity, code, message)) = abstract_outcome {
                issues.push(StructuralDiagnostic {
                    severity,
                    code,
                    subject_id: binding.id().into(),
                    message: format!("ClassifierTemplateParameter-matching_abstract: {message}"),
                });
            }
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
