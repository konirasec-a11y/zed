# Public workflow migration

Existing scenarios are being moved to public application or kernel APIs without
adding scenarios or running builds/tests. Production consumers already cross
Cordis and the explicit MDE facade; legacy integration tests still require the
temporary internal exports listed in `src/mde-api/README.md`.

## Existing flows already adapted

- `bdd_diagram_flows.rs`: 40 existing fixture-driven import, validation,
  JSON/BLX save and reopen scenarios across diagram families.
- `bdd_activity_edge_flows.rs`: existing activity authoring, diagnostics,
  repair and persistence flows through kernel/document APIs.
- `bdd_kernel_boundary_flows.rs`: existing semantic/presentation boundary,
  incomplete-authoring and persistence flows through public APIs.
- `bdd_mdj_association_flows.rs`: existing MDJ multiplicity/navigation editing,
  verified export and reopening flows. Five internal derived-query assertions
  were consolidated into existing public inspector/marker/diagnostic assertions;
  the two scenarios remain.
- `bdd_mdj_presentation_flows.rs`: the three existing MDJ style/residue,
  unsupported-edit and save/reopen scenarios retain their assertions.

Existing Cordis workflows remain in `../../bellatrix_mde_cordis/tests/`.

## Additional existing flows migrated

These files retain their original scenario names and counts. No new scenario was
added. The table counts source `#[test]` functions, not runtime coverage.

| Existing file | Scenarios | Preserved workflow |
| --- | ---: | --- |
| `ownership_namespace_flow.rs` | 4 | Rename/inspect, rejected ownership, invalid roles, repair, external references, JSON/Protobuf reopen |
| `property_contract_flow.rs` | 3 | Scalar/reference editing, atomic rejection, invalid literal drafts, repair and both codec formats |
| `property_invariants_flow.rs` | 4 | Derived unions, qualifiers and classifier feature rules, rejected edits, repair and both codec formats |
| `relationship_roles_flow.rs` | 3 | Object/edge multiple roles, deletion rollback, invalid/unresolved drafts, subset roles, repair and reopen |
| `namespace_distinguishability_flow.rs` | 3 | Overloads/default inputs, rejected collisions, parameter order, aliases/inheritance, Gate/Message overrides and reopen |
| `view_style.rs` | 3 | Route/copy isolation, curved endpoint movement, invalid styles, rollback, stacking and JSON/Protobuf |
| `diagram_visual_creation.rs` | 2 | Diagram contexts, creation, selected view identity, retargeting, nested views, drafts, messages, rollback and reopen |

The next transport-only batch preserves another 27 existing scenarios and all
143 explicit assertion macros in these 14 files:

- `association_specialization_flow.rs`: 2 scenarios, association-end editing,
  ordered type conformance, inherited navigation, Inspector and persistence.
- `class_behavior.rs`, `component_constraints.rs`, `enumeration_immutability.rs`,
  `operation_body_condition.rs`, `operation_return_count.rs`: 10 scenarios,
  existing validation matrices, authoring, drafts, repairs and persistence.
- `parameter_object_effect.rs`, `parameter_scalar_constraints.rs`,
  `parameter_set_entity.rs`, `port_default_value.rs`, `port_ownership.rs`:
  12 scenarios, existing reference/type/ownership checks and authoring workflows.
- `theme_inheritance.rs`, `inspector_optional_null.rs`,
  `kernel_api_validation.rs`: 3 scenarios, style inheritance and idempotence,
  property clearing and protected fields, rooted/rootless validation and repair.

These are preserved expectations, not confirmation that they match current
production behavior. The baseline discrepancy below applies to semantic edit
rejection and required scalar-null assertions in this batch.

### Assertion consolidation and limitations

- Ownership: private graph traversal/error-category and reflexive signature
  assertions were consolidated into existing diagnostics and Inspector fields.
  Hierarchy rows are not claimed to verify every `all_owned_elements` derivation.
- Property contracts: 37 internal evaluation/getter assertions were replaced by
  12 public Inspector/diagnostic/value assertions. Defaults Integer=0/Boolean=false,
  typed-getter mismatch, `isComputable` and `isNull` are no longer directly tested.
  All edit, rejection, draft, repair and persistence steps remain. The extension
  fixture uses the public mixed-domain diagnostic contract, not private UML codes.
- Relationship roles: direct trait/query enumeration of sources, targets, unions
  and inverse clients was consolidated into Inspector values, canonical endpoint
  identity, diagnostic classification and persistence. Exact private query return
  collections are not asserted through a substitute implementation.
- Namespace: direct per-pair query assertions were consolidated into public
  namespace diagnostics and Inspector exclusions. Parent imported aliases and
  child inherited member identities remain checked; the effective alias returned
  by `get_names_of_member(child, imported)` is not directly proved by those two
  observations. This is an explicit gap, not a claim of one-for-one equivalence.
- Style: only the final microcase constructing an internal diagram width as
  `f64::NAN` was removed, because that value is not representable by the JSON
  document API. Invalid non-finite Style DTO rejection remains in the flow.
- Visual creation: five assertions for the retired beta `Draft.source_view_id` /
  old protobuf-tag compatibility were removed. Current endpoint identities,
  retargeting, source preservation and complete JSON/Protobuf flows remain.

These are source-level migrations, not executed verification. Review identified
no static blockers in ownership/property/relationship helpers, including all four
property-invariant scenarios with their existing assertions. Public projections
are read-only derivatives of canonical JSON; they are never the test's persistence
source or an alternate semantic implementation.

## Existing behavior requires alignment

Read-only review found expectations in the original tests that already disagree
with the production checkpoint. The command executor validates hierarchy,
presentation and navigation-knowledge integrity, but does not run the former
general UML change gate. Several tests still expect arbitrary semantic edits to
be rejected. The Inspector writer also permits null scalar references as authored
drafts, while `inspector_optional_null.rs` expects required `Reception.signal` to
reject null. These expectations have not been silently removed, and no semantic
gate has been restored as part of API migration. Migrated source is not evidence
that those scenarios pass; their intended draft/validation behavior needs alignment.

## Remaining scenario-level review

The other 69 tracked integration files are retained and still await individual
migration/classification. They are not all microtests: complete edit, rollback,
identity, reference-protection and reopen flows exist throughout the original
258 scenarios in the 90 files that initially needed migration. Removing an entire
file based on its name or an internal import is not a valid coverage decision.

Each scenario must be classified separately. Preserve complete workflows through
public commands, projections, document APIs or events. Remove only redundant
microimplementation assertions/scenarios with an explicit rationale; do not
create test-only APIs or move integration tests into private modules. Visibility
closure follows migration, not the other way around. No one-for-one transfer or
runtime coverage has been claimed.
