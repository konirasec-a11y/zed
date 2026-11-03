use super::{
    classifiers::{self, QueryError},
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

pub(crate) fn validate(references: &ReferenceIndex<'_>) -> Vec<StructuralDiagnostic> {
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
