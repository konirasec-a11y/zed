# Structural authored project corpus

## Canonical persistence acceptance

The document files are explicit schema-2 projects: `schemaVersion`, project identity,
and `model.registry.entries`. The registry contains typed ruml contracts; it is the
only persisted UML state. No schema-1 loader, projection-to-storage adapter or
legacy compatibility fixture is part of the executable BDD.

`tests/bdd_diagram_flows.rs` imports each document through the public document API,
prepares its canonical section, saves to JSON and BLX2, closes and reopens both
files, and compares the complete typed project and canonical API envelope. It
also persists the prepared section through the public section API. The manifests
supply explicit authored witnesses checked before and after persistence. Every
Gherkin example must have exactly one manifest entry. Of the 40 preserved
requirements, 32 have executable canonical documents and eight are explicitly
pending because their original intent is not representable by the current
contract. Pending cases have no substitute JSON fixture.

Names ending in `valid` or `invalid` are historical scenario identifiers, not a
claim of UML certification. `historical_diagnostics` and
`historical_forbidden_diagnostics` preserve the original requirement context but
are not evaluated by these persistence flows. Semantically questionable contracts
must remain inspectable and survive persistence; no removed validator is revived.

Multiplicities are authored as `lowerValue`/`upperValue` references to
LiteralInteger/LiteralUnlimitedNatural contracts. Ownership is encoded in UML
composite fields, not persisted `owns` edge records. Required scalar fields use
the current contract's enum spelling and string representation. Diagram geometry,
styles, view endpoints and identities remain part of the equality assertion.

The 14 examples cover class, object, package, component, composite structure,
deployment and profile pairs. Each project retains one diagram and its full domain
model. Negative witnesses retain the selected direction, classifier, visibility,
nested classifier, connector role, deployment metaclass or generalization change.
Association display roles and multiplicities are represented by the actual
Property end contracts, rather than duplicate projection properties.

## Historical semantic requirements

The following original notes preserve the intended domain scenarios and normative
references. Mentions of old validation executables, legacy paths or additional
lifecycles describe historical requirements, not coverage of the current runner.

# Isolated structural UML corpus

Each pair is a native Bellatrix Project JSON with its own root, graph, one UMLDI diagram and no external references. Load each file separately. `cases.json` specifies the targeted invalid rule and diagnostic identity. The invalid variant is a semantic mutation of the valid baseline, not a parsing failure.

The names `valid` and `invalid` state the expected scenario outcome, not a claim of complete UML 2.5.1 certification. Keep unsupported/unverified diagnostics visible. Class includes multiple major structural concepts, not every UML metaclass or every legal combination. Profile extension metaclass interpretation and broader kernel constraints may remain unsupported.

BDD consumers should deserialize, validate graph/model/diagram, inspect semantic projections, persist JSON and Protobuf, reopen and compare diagnostics/identities. Invalid documents must remain inspectable and round-trippable; rejected transactional edits must leave the baseline unchanged. Do not equate compiler backend availability with UML semantic validity. The six other structural UMLDI kinds have no diagram context; composite structure has exactly one StructuredClassifier context.

The baselines explicitly mark Interface operations public, give Usage dependencies a package owner and distinct UML names, and leave visibility absent (`null`) for owned values, InterfaceRealizations and Manifestations that are not namespace members. These ownership and visibility choices are preserved in the invalid variants so their failures remain focused on the documented mutation.

## Invalid scenario mapping

- `class.invalid.json`: `submit-request.direction` changes from `in` to `return`, violating Operation-at_most_one_return on `submit` and preventing its return-parameter projection (`UML_OPERATION_PARAMETERS`).
- `object.invalid.json`: `order-1042.classifier` changes to `Unrelated`, whose features do not define its existing slots. The instance-level violation also invalidates the `first-number` and `first-total` Slot assignments (`UML_SLOT_CONTRACT`).
- `package.invalid.json`: the packaged `Order` Class changes visibility from `public` to `protected`.
- `component.invalid.json`: CheckoutService introduces a forbidden nestedClassifier reference to its implementation.
- `composite_structure.invalid.json`: the connector's storage end references a foreign classifier's role.
- `deployment.invalid.json`: the configured ApplicationRuntime changes from ExecutionEnvironment to Node.
- `profile.invalid.json`: LocalHelper specializes the imported UML metaclass, forbidden by Profile-metaclass_reference_not_specialized.

## Contract sources in this repository

The mutations follow `tests/operation_return_count.rs`, `tests/instance_defining_feature.rs`, `src/KERNEL/uml/validation/structural_rules.rs` (Package-elements_public_or_private), `tests/component_constraints.rs`, `tests/connector_roles.rs`, `tests/deployment_specification.rs`, and `tests/profile_metaclass_reference.rs`. Canonical association/extension/path representations follow `tests/property_invariants_flow.rs`, `tests/extension_contracts.rs`, and `tests/communication_path.rs`. UMLDI context rules are in `src/KERNEL/uml/validation/structural_diagrams.rs` and the generated normative catalog under `src/KERNEL/uml/metamodel/`.
