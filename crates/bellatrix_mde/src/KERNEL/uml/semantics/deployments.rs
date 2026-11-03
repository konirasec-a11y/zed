use super::{
    classifiers::{self, QueryError},
    feature_compatibility::require_all,
    property_contracts::{self, PropertyOutcome},
    references::{ReferenceIndex, reference_id},
    root::ElementRef,
    schema,
};
use crate::{StructuralDiagnostic, StructuralSeverity};

struct OwnerEvidence<'a> {
    canonical: Result<Option<ElementRef<'a>>, QueryError>,
    unknown: Option<QueryError>,
}

fn opposite_owner<'a>(
    references: &ReferenceIndex<'a>,
    child: ElementRef<'a>,
    inverse: &str,
    owner_type: &str,
    composite: &'static str,
) -> Result<OwnerEvidence<'a>, QueryError> {
    let stored = child.properties().get(inverse);
    let mut unknown = None;
    if let Some(value) = stored {
        let descriptor = schema::uml_property(
            child
                .metaclass()
                .ok_or_else(|| QueryError::unsupported(child.id(), "unknown metaclass"))?,
            inverse,
        )
        .map_err(|error| QueryError::unsupported(child.id(), error.to_string()))?
        .ok_or_else(|| QueryError::unsupported(child.id(), "missing inverse property contract"))?;
        match property_contracts::check_value(references, child, descriptor, value) {
            PropertyOutcome::Satisfied => {}
            PropertyOutcome::Violation(message) => {
                return Err(QueryError::invalid(child.id(), message));
            }
            PropertyOutcome::Unverified(message) => {
                unknown = Some(QueryError::unsupported(child.id(), message))
            }
        }
    }
    let owner = references
        .model()
        .owner(child)
        .map_err(|error| QueryError::invalid(child.id(), error))?
        .map(|owner| {
            references
                .element(owner.id())
                .map_err(|error| QueryError::invalid(child.id(), error))
        })
        .transpose()?;
    if let Some(value) = stored
        && let Ok(Some(id)) = reference_id(value)
        && owner.map(ElementRef::id) != Some(id)
    {
        return Err(QueryError::invalid(
            child.id(),
            format!("{inverse} contradicts canonical ownership"),
        ));
    }
    let mut projected = None;
    let mut projection_unknown = None;
    if let Some(owner) = owner {
        if owner.metaclass().is_none() {
            projection_unknown = Some(QueryError::unsupported(
                child.id(),
                "canonical owner metaclass is unresolved",
            ));
        } else if owner.conforms_to(owner_type) {
            for property in classifiers::properties(owner)? {
                if classifiers::contributes(property, composite)
                    .map_err(|error| QueryError::unsupported(owner.id(), error))?
                    && let Some(value) = owner.properties().get(property.name)
                {
                    match property_contracts::check_value(references, owner, property, value) {
                        PropertyOutcome::Satisfied => {}
                        PropertyOutcome::Violation(message) => {
                            return Err(QueryError::invalid(owner.id(), message));
                        }
                        PropertyOutcome::Unverified(message) => {
                            unknown = Some(QueryError::unsupported(owner.id(), message));
                        }
                    }
                }
            }
            match classifiers::contributes_to_owned_union(owner, child, composite) {
                Ok(true) => projected = Some(owner),
                Ok(false) => {}
                Err(error @ QueryError::Invalid { .. }) => return Err(error),
                Err(error) => projection_unknown = Some(error),
            }
        }
    }
    if let Some(value) = stored
        && let Ok(id) = reference_id(value)
        && projection_unknown.is_none()
        && id != projected.map(ElementRef::id)
    {
        return Err(QueryError::invalid(
            child.id(),
            format!("{inverse} contradicts canonical {composite} ownership"),
        ));
    }
    Ok(OwnerEvidence {
        canonical: match projection_unknown {
            Some(error) => Err(error),
            None => Ok(projected),
        },
        unknown,
    })
}

fn invalid_target(
    references: &ReferenceIndex<'_>,
    specification: ElementRef<'_>,
) -> Result<bool, QueryError> {
    let deployment = opposite_owner(
        references,
        specification,
        "deployment",
        "Deployment",
        "Deployment-configuration",
    )?;
    let Some(canonical_deployment) = deployment.canonical? else {
        return match deployment.unknown {
            Some(error) => Err(error),
            None => Ok(false),
        };
    };
    let location = opposite_owner(
        references,
        canonical_deployment,
        "location",
        "DeploymentTarget",
        "DeploymentTarget-deployment",
    )?;
    // Unresolved inverse references do not erase canonical ownership evidence.
    // Keep checking the location so a proven violation dominates uncertainty,
    // but never present incomplete ownership checks as a verified success.
    let canonical_location = location.canonical?.ok_or_else(|| {
        QueryError::invalid(
            canonical_deployment.id(),
            "Deployment::location requires a canonical DeploymentTarget owner",
        )
    })?;
    if !canonical_location.conforms_to("ExecutionEnvironment") {
        return Ok(true);
    }
    match deployment.unknown.or(location.unknown) {
        Some(error) => Err(error),
        None => Ok(false),
    }
}

// Evidence is deliberately separate from a complete derived collection: an
// unresolved branch cannot conceal a known non-Component or malformed sibling.
struct RoleEvidence<'a> {
    values: Vec<ElementRef<'a>>,
    checks: Vec<Result<bool, QueryError>>,
}

fn reference_targets<'a>(
    references: &ReferenceIndex<'a>,
    owner: ElementRef<'a>,
    property: &'static schema::PropertyDescriptor,
) -> RoleEvidence<'a> {
    let mut evidence = RoleEvidence {
        values: Vec::new(),
        checks: Vec::new(),
    };
    let Some(value) = owner.properties().get(property.name) else {
        evidence.checks.push(Err(QueryError::unsupported(
            owner.id(),
            format!(
                "{} requires explicit canonical role references",
                property.id.fragment
            ),
        )));
        return evidence;
    };
    match property_contracts::check_value(references, owner, property, value) {
        PropertyOutcome::Satisfied => {}
        PropertyOutcome::Violation(message) => {
            evidence
                .checks
                .push(Err(QueryError::invalid(owner.id(), message)));
            return evidence;
        }
        PropertyOutcome::Unverified(message) => {
            evidence
                .checks
                .push(Err(QueryError::unsupported(owner.id(), message)));
        }
    }
    let values = value
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_else(|| std::slice::from_ref(value));
    for value in values {
        if let Ok(Some(id)) = reference_id(value) {
            match references.element(id) {
                Ok(target) => evidence.values.push(target),
                Err(error) => evidence
                    .checks
                    .push(Err(QueryError::invalid(owner.id(), error))),
            }
        }
    }
    evidence
}

fn composite_targets<'a>(
    references: &ReferenceIndex<'a>,
    owner: ElementRef<'a>,
    property: &'static schema::PropertyDescriptor,
) -> RoleEvidence<'a> {
    let mut evidence = RoleEvidence {
        values: Vec::new(),
        checks: Vec::new(),
    };
    if owner.properties().contains_key(property.name) {
        evidence
            .checks
            .extend(reference_targets(references, owner, property).checks);
    }
    let children = match references.explicit_owned_elements(owner.id()) {
        Ok(children) => children,
        Err(error) => {
            evidence
                .checks
                .push(Err(QueryError::invalid(owner.id(), error)));
            return evidence;
        }
    };
    for child in children {
        match classifiers::contributes_to_owned_union(owner, child, property.id.fragment) {
            Ok(true) => evidence.values.push(child),
            Ok(false) => {}
            Err(error) => evidence.checks.push(Err(error)),
        }
    }
    evidence
}

fn deployed_components(
    references: &ReferenceIndex<'_>,
    specification: ElementRef<'_>,
) -> Result<bool, QueryError> {
    let deployment = opposite_owner(
        references,
        specification,
        "deployment",
        "Deployment",
        "Deployment-configuration",
    )?;
    let mut checks = Vec::new();
    if let Some(error) = deployment.unknown {
        checks.push(Err(error));
    }
    let Some(deployment) = deployment.canonical? else {
        return require_all(checks);
    };
    let location = opposite_owner(
        references,
        deployment,
        "location",
        "DeploymentTarget",
        "DeploymentTarget-deployment",
    )?;
    if let Some(error) = location.unknown {
        checks.push(Err(error));
    }
    let location = location.canonical?.ok_or_else(|| {
        QueryError::invalid(
            deployment.id(),
            "Deployment::location requires a canonical DeploymentTarget owner",
        )
    })?;
    let property = |id: &'static str| {
        schema::property_by_id(schema::XmiId::new(schema::Document::Uml, id)).ok_or_else(|| {
            QueryError::unsupported(specification.id(), format!("missing {id} contract"))
        })
    };
    let deployments = composite_targets(
        references,
        location,
        property("DeploymentTarget-deployment")?,
    );
    checks.extend(deployments.checks);
    for deployment in deployments.values {
        // The inverse can be unknown without preventing inspection of this
        // deployment's known artifacts, but a contradictory inverse is Invalid.
        match opposite_owner(
            references,
            deployment,
            "location",
            "DeploymentTarget",
            "DeploymentTarget-deployment",
        ) {
            Ok(owner) => {
                if let Some(error) = owner.unknown {
                    checks.push(Err(error));
                }
                checks.push(owner.canonical.map(|_| true));
            }
            Err(error) => checks.push(Err(error)),
        }
        let artifacts = reference_targets(
            references,
            deployment,
            property("Deployment-deployedArtifact")?,
        );
        checks.extend(artifacts.checks);
        for artifact in artifacts.values {
            if artifact.metaclass().is_none() {
                checks.push(Err(QueryError::unsupported(
                    artifact.id(),
                    "deployed artifact metaclass is unresolved",
                )));
                continue;
            }
            if !artifact.conforms_to("Artifact") {
                continue;
            }
            let manifestations =
                composite_targets(references, artifact, property("Artifact-manifestation")?);
            checks.extend(manifestations.checks);
            for manifestation in manifestations.values {
                let utilized = reference_targets(
                    references,
                    manifestation,
                    property("Manifestation-utilizedElement")?,
                );
                checks.extend(utilized.checks);
                for element in utilized.values {
                    checks.push(if element.metaclass().is_none() {
                        Err(QueryError::unsupported(
                            element.id(),
                            "utilized element metaclass is unresolved",
                        ))
                    } else {
                        Ok(element.conforms_to("Component"))
                    });
                }
            }
        }
    }
    require_all(checks)
}

fn validate_targets(references: &ReferenceIndex<'_>) -> Vec<StructuralDiagnostic> {
    let mut issues = Vec::new();
    for specification in references
        .model()
        .elements()
        .filter(|element| element.conforms_to("DeploymentSpecification"))
    {
        let (severity, code, message) = match invalid_target(references, specification) {
            Ok(false) => continue,
            Ok(true) => (
                StructuralSeverity::Error,
                "UML_DEPLOYMENT_SPECIFICATION_TARGET",
                "Deployment location must be an ExecutionEnvironment".into(),
            ),
            Err(QueryError::Invalid { message, .. }) => (
                StructuralSeverity::Error,
                "MDE_DEPLOYMENT_SPECIFICATION_OWNERSHIP",
                message,
            ),
            Err(QueryError::Unsupported { message, .. }) => (
                StructuralSeverity::Warning,
                "MDE_DEPLOYMENT_SPECIFICATION_UNVERIFIED",
                message,
            ),
        };
        issues.push(StructuralDiagnostic {
            severity,
            code,
            subject_id: specification.id().into(),
            message: format!("DeploymentSpecification-deployment_target: {message}"),
        });
    }
    issues
}

pub(crate) fn validate(references: &ReferenceIndex<'_>) -> Vec<StructuralDiagnostic> {
    let mut issues = validate_targets(references);
    for specification in references
        .model()
        .elements()
        .filter(|element| element.conforms_to("DeploymentSpecification"))
    {
        let (severity, code, message) = match deployed_components(references, specification) {
            Ok(true) => continue,
            Ok(false) => (
                StructuralSeverity::Error,
                "UML_DEPLOYMENT_SPECIFICATION_DEPLOYED_ELEMENTS",
                "Deployment location's deployed elements must be Components".into(),
            ),
            Err(QueryError::Invalid { message, .. }) => (
                StructuralSeverity::Error,
                "MDE_DEPLOYMENT_SPECIFICATION_DEPLOYED_ELEMENTS_INVALID",
                message,
            ),
            Err(QueryError::Unsupported { message, .. }) => (
                StructuralSeverity::Warning,
                "MDE_DEPLOYMENT_SPECIFICATION_DEPLOYED_ELEMENTS_UNVERIFIED",
                message,
            ),
        };
        issues.push(StructuralDiagnostic {
            severity,
            code,
            subject_id: specification.id().into(),
            message: format!("DeploymentSpecification-deployed_elements: {message}"),
        });
    }
    issues
}
