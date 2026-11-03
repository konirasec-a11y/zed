use super::{
    classifiers::{self, QueryError},
    feature_compatibility::{accept_any, require_all},
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
        // A known incompatible child cannot witness this composite role; its
        // other roles are validated separately and cannot make this set unknown.
        if child.metaclass().is_some()
            && let Some(required) = property.property_type.target.and_then(schema::classifier)
            && !child.conforms_to(required.name)
        {
            continue;
        }
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

fn property_deployment_target(
    references: &ReferenceIndex<'_>,
    property: ElementRef<'_>,
) -> Result<bool, QueryError> {
    let descriptor = schema::property_by_id(schema::XmiId::new(
        schema::Document::Uml,
        "DeploymentTarget-deployment",
    ))
    .ok_or_else(|| QueryError::unsupported(property.id(), "missing deployment contract"))?;
    let deployments = composite_targets(references, property, descriptor);
    let mut checks = deployments.checks;
    for deployment in &deployments.values {
        let check = match opposite_owner(
            references,
            *deployment,
            "location",
            "DeploymentTarget",
            "DeploymentTarget-deployment",
        ) {
            Ok(owner) => {
                if let Some(error) = owner.unknown {
                    checks.push(Err(error));
                }
                owner.canonical.map(|_| true)
            }
            Err(error) => Err(error),
        };
        checks.push(check);
    }
    let mut unknown = None;
    for check in checks {
        match check {
            Err(error @ QueryError::Invalid { .. }) => return Err(error),
            Err(error) => unknown = Some(error),
            Ok(_) => {}
        }
    }
    // A known canonical member proves nonemptiness even when other members or
    // inverse references remain unresolved. No complete collection is claimed.
    let present = if !deployments.values.is_empty() {
        Ok(true)
    } else if let Some(error) = unknown {
        Err(error)
    } else {
        return Ok(true);
    };
    let composite = match property {
        ElementRef::Object(element) => super::associations::aggregation(element)
            .map(|aggregation| aggregation == "composite")
            .map_err(|error| QueryError::invalid(property.id(), error)),
        _ => Err(QueryError::invalid(
            property.id(),
            "Property requires an object identity",
        )),
    };
    let node_part = (|| {
        let owner = references
            .model()
            .owner(property)
            .map_err(|error| QueryError::invalid(property.id(), error))?;
        let Some(owner) = owner else {
            return Ok(false);
        };
        references
            .element(owner.id())
            .map_err(|error| QueryError::invalid(property.id(), error))?;
        if owner.metaclass().is_none() {
            return Err(QueryError::unsupported(
                property.id(),
                "canonical owner metaclass is unresolved",
            ));
        }
        if !owner.conforms_to("Node") {
            return Ok(false);
        }
        // UML part derives only from ownedAttribute, not inherited attributes.
        // Unique canonical ownership makes the owning Node the only possible
        // witness for Node.allInstances()->exists(n | n.part->includes(self)).
        classifiers::contributes_to_owned_union(
            owner,
            property,
            "StructuredClassifier-ownedAttribute",
        )
    })();
    accept_any([
        present.map(|present| !present),
        require_all([composite, node_part]),
    ])
}

fn deployment_artifact_presence(
    references: &ReferenceIndex<'_>,
    instance: ElementRef<'_>,
) -> Result<bool, QueryError> {
    let descriptor = |id| {
        schema::property_by_id(schema::XmiId::new(schema::Document::Uml, id))
            .ok_or_else(|| QueryError::unsupported(instance.id(), format!("missing {id} contract")))
    };
    let deployed = descriptor("Deployment-deployedArtifact")?;
    let mut known = Vec::new();
    let mut unresolved = Vec::new();
    let mut checks = Vec::new();
    for deployment in references
        .model()
        .elements()
        .filter(|element| element.conforms_to("Deployment"))
    {
        let evidence = reference_targets(references, deployment, deployed);
        if evidence
            .values
            .iter()
            .any(|target| target.id() == instance.id())
        {
            known.push(deployment.id());
        }
        if evidence.checks.iter().any(Result::is_err) {
            unresolved.push(deployment.id());
        }
        checks.extend(evidence.checks);
    }
    if instance.properties().contains_key("deploymentForArtifact") {
        let inverse = reference_targets(
            references,
            instance,
            descriptor("A_deployedArtifact_deploymentForArtifact-deploymentForArtifact")?,
        );
        let inverse_unknown = inverse.checks.iter().any(Result::is_err);
        checks.extend(inverse.checks);
        // Authored inverse references must agree with the forward association;
        // they never establish deployment membership on their own.
        for deployment in &inverse.values {
            if deployment.metaclass().is_none() || unresolved.contains(&deployment.id()) {
                continue;
            }
            if !known.contains(&deployment.id()) {
                checks.push(Err(QueryError::invalid(
                    instance.id(),
                    "deploymentForArtifact contradicts canonical Deployment::deployedArtifact",
                )));
            }
        }
        if !inverse_unknown
            && known.iter().any(|id| {
                !inverse
                    .values
                    .iter()
                    .any(|deployment| deployment.id() == *id)
            })
        {
            checks.push(Err(QueryError::invalid(
                instance.id(),
                "deploymentForArtifact omits a canonical Deployment::deployedArtifact membership",
            )));
        }
    }
    // Existential evidence is sufficient for this implication. Invalid remains
    // decisive, but unresolved neighbors cannot erase a known local membership.
    checks.push(Ok(!known.is_empty()));
    accept_any(checks)
}

fn instance_deployment_artifact(
    references: &ReferenceIndex<'_>,
    instance: ElementRef<'_>,
) -> Result<bool, QueryError> {
    let present = deployment_artifact_presence(references, instance);
    if matches!(present, Ok(false)) {
        return Ok(true);
    }
    let classifiers = if instance.properties().contains_key("classifier") {
        let descriptor = schema::uml_property("InstanceSpecification", "classifier")
            .map_err(|error| QueryError::unsupported(instance.id(), error.to_string()))?
            .ok_or_else(|| QueryError::unsupported(instance.id(), "missing classifier contract"))?;
        let mut evidence = reference_targets(references, instance, descriptor);
        evidence.checks.extend(
            evidence
                .values
                .iter()
                .map(|classifier| Ok(classifier.conforms_to("Artifact"))),
        );
        accept_any(evidence.checks)
    } else {
        Ok(false)
    };
    accept_any([present.map(|present| !present), classifiers])
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
    for property in references
        .model()
        .elements()
        .filter(|element| element.conforms_to("Property"))
    {
        let (severity, code, message) = match property_deployment_target(references, property) {
            Ok(true) => continue,
            Ok(false) => (
                StructuralSeverity::Error,
                "UML_PROPERTY_DEPLOYMENT_TARGET",
                "A deployed Property must be a composite ownedAttribute of a Node".into(),
            ),
            Err(QueryError::Invalid { message, .. }) => (
                StructuralSeverity::Error,
                "MDE_PROPERTY_DEPLOYMENT_TARGET_INVALID",
                message,
            ),
            Err(QueryError::Unsupported { message, .. }) => (
                StructuralSeverity::Warning,
                "MDE_PROPERTY_DEPLOYMENT_TARGET_UNVERIFIED",
                message,
            ),
        };
        issues.push(StructuralDiagnostic {
            severity,
            code,
            subject_id: property.id().into(),
            message: format!("Property-deployment_target: {message}"),
        });
    }
    for instance in references
        .model()
        .elements()
        .filter(|element| element.conforms_to("InstanceSpecification"))
    {
        let (severity, code, message) = match instance_deployment_artifact(references, instance) {
            Ok(true) => continue,
            Ok(false) => (
                StructuralSeverity::Error,
                "UML_INSTANCE_SPECIFICATION_DEPLOYMENT_ARTIFACT",
                "A deployed InstanceSpecification must have an Artifact classifier".into(),
            ),
            Err(QueryError::Invalid { message, .. }) => (
                StructuralSeverity::Error,
                "MDE_INSTANCE_SPECIFICATION_DEPLOYMENT_ARTIFACT_INVALID",
                message,
            ),
            Err(QueryError::Unsupported { message, .. }) => (
                StructuralSeverity::Warning,
                "MDE_INSTANCE_SPECIFICATION_DEPLOYMENT_ARTIFACT_UNVERIFIED",
                message,
            ),
        };
        issues.push(StructuralDiagnostic {
            severity,
            code,
            subject_id: instance.id().into(),
            message: format!("InstanceSpecification-deployment_artifact: {message}"),
        });
    }
    issues
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn instance_deployment_artifact_gate_rejects_new_violation_and_accepts_repair()
    -> Result<(), Box<dyn std::error::Error>> {
        let before: crate::Project = serde_json::from_value(json!({"id":"project","name":"Gate",
            "elements":[
                {"id":"node","kind":"uml.node","name":"Node"},
                {"id":"artifact","kind":"uml.artifact","name":"Artifact"},
                {"id":"class","kind":"uml.class","name":"Class"},
                {"id":"instance","kind":"uml.instance_specification","name":"Instance","properties":{"classifier":[{"$ref":"artifact"}]}}
            ],"relations":[{"id":"deployment","kind":"uml.deployment","from":"node","to":"instance","properties":{"deployedArtifact":[{"$ref":"instance"}]}}]
        }))?;
        let unchanged = before.clone();
        let mut after = before.clone();
        after
            .elements
            .iter_mut()
            .find(|element| element.id == "instance")
            .ok_or("instance")?
            .properties
            .insert("classifier".into(), json!([{"$ref":"class"}]));
        let error = super::super::root::validate_changes(&before, &after)
            .expect_err("deployed instance must have Artifact classifier");
        assert!(
            error.contains("InstanceSpecification-deployment_artifact:"),
            "{error}"
        );
        assert_eq!(before, unchanged);
        super::super::root::validate_changes(&after, &before)?;
        Ok(())
    }
}
