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
