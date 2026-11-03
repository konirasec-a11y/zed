# Structural contracts retained for planned reuse

These contracts are relevant to the planned Object, Composite Structure and Deployment work.
The runtime DTO copies were unused and duplicated canonical state; they are not restored as a second model.
This consultation inventory is extracted from the retained official UML.xmi, not from the old string-typed DTOs.

## Provenance

- Normative input: `../standards/uml/20161101/UML.xmi`.
- Removed native sketches remain recoverable in Git at `9de0d1cd58b485d671b8338137f36d09fdbacbb6`,
  under `crates/bellatrix_mde/src/KERNEL/uml/structural/{classifiers,deployments,common_structure,values}.rs`.
- The retained RUML snapshot is partial and does not contain every corresponding original file.
- String-encoded booleans and capitalized enum serialization in the old sketches must not be copied into the canonical format.

## Direct declarations in the normative input

Inherited properties are intentionally not duplicated below. Empty default cardinalities mean UML defaults (1..1).
This list is a source inventory, not an implementation or validation claim.

### DeployedArtifact

| Feature | Type | Multiplicity |
| --- | --- | --- |

### Deployment

| Feature | Type | Multiplicity |
| --- | --- | --- |
| configuration | DeploymentSpecification | 0..* |
| deployedArtifact | DeployedArtifact | 0..* |
| location | DeploymentTarget | 1..1 |

### DeploymentSpecification

| Feature | Type | Multiplicity |
| --- | --- | --- |
| deployment | Deployment | 0..1 |
| deploymentLocation | String | 0..1 |
| executionLocation | String | 0..1 |

### DeploymentTarget

| Feature | Type | Multiplicity |
| --- | --- | --- |
| deployedElement | PackageableElement | 0..* |
| deployment | Deployment | 0..* |

### Manifestation

| Feature | Type | Multiplicity |
| --- | --- | --- |
| utilizedElement | PackageableElement | 1..1 |

### VisibilityKind

| Feature | Type | Multiplicity |
| --- | --- | --- |
| public | enumeration literal | — |
| private | enumeration literal | — |
| protected | enumeration literal | — |
| package | enumeration literal | — |

### InstanceSpecification

| Feature | Type | Multiplicity |
| --- | --- | --- |
| classifier | Classifier | 0..* |
| slot | Slot | 0..* |
| specification | ValueSpecification | 0..1 |

### Slot

| Feature | Type | Multiplicity |
| --- | --- | --- |
| definingFeature | StructuralFeature | 1..1 |
| owningInstance | InstanceSpecification | 1..1 |
| value | ValueSpecification | 0..* |

## Planned consumers and limits

- InstanceSpecification/Slot: explicit classifier and slot adapters; resolve references before claiming conformance.
- Composite Structure: an instance context requires at least one StructuredClassifier classifier, not an invented string convention.
- Deployment: explicit location/artifact/configuration semantics; do not equate diagram arrow direction with dependency client/supplier storage.
- Visibility: use canonical lowercase values and validate Package ownership context.
- Keep one canonical Element/Relation graph and typed projections; do not revive duplicated persistent IDs and inherited fields.
