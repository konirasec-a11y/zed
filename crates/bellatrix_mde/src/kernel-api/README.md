# Functional kernel API: version 1

The initial public entry point is `bellatrix_mde::validate_uml_model`:

```rust
pub fn validate_uml_model(input_json: &str) -> Result<String, String>
```

Implementation lives in this literal `kernel-api/` directory. Its Rust module,
request/response types and helpers are private. Only the function is reexported;
no new public structs, enums, modules, handles or registries are introduced.

Application/document operations are located separately in
[`mde-api/`](../mde-api/README.md). In particular, document decoding and semantic
reparenting are MDE facades, not kernel entry points. Both boundaries use the
private `src/api_contract.rs` envelope/codec helper without depending on each
other. This directory's `contract.rs` owns only the semantic UML input projection.

## Input contract

A JSON object with exactly these fields:

```json
{
  "api_version": 1,
  "model": {
    "schema_version": 1,
    "id": "example",
    "name": "Example",
    "elements": [],
    "relations": [],
    "diagrams": []
  }
}
```

`api_version` is a required unsigned integer and must equal 1. Unknown envelope
fields are rejected. `model` uses the existing Bellatrix document representation;
its model `schema_version` is independent of the API version and currently must
be 1 (the existing codec defaults an omitted model version to 1). This boundary
exposes a documented data format, not the internal Rust structs.

## Successful call

`Ok` contains a JSON object with:

- `api_version`: 1.
- `coverage`: `"implemented_rules_only"`, `"implemented_uml_subset_rules_only"`,
  `"implemented_uml_scalars_and_integrity_only"`, or
  `"implemented_rules_with_incomplete_authoring"` (described below).
- `validity`: `"unverified"` for a mixed-domain or incomplete-authoring capacity blocker, `"invalid"`
  when errors were found, or `"no_violations_found"`. None certifies conformance.
- `has_errors`: whether any reported diagnostic has severity `"error"`, including
  a capacity blocker that prevents semantic evaluation.
- `diagnostics`: an array of objects with `severity` (`"error"` or `"warning"`),
  `code`, `subject_id` and `message`, all strings.

Diagnostics are ordered by `(code, subject_id, severity, message)`. Input array
order is preserved when invoking existing semantics because ordered UML members
are meaningful. A well-formed invalid model returns `Ok` with diagnostics.

The UML entry point decodes only the semantic projection of the Bellatrix document:
`schema_version`, `id`, `name`, `root_model`, `authoring_knowledge`, `elements` and `relations`.
Known document fields `diagrams`, `hierarchy`, `composition` and `mdj_provenance`
are ignored without parsing their contents or checking presentation identities.
Malformed visual content cannot hide UML errors. Unknown document fields and
malformed semantic fields are still contract errors. This projection neither
changes the input nor repairs or replaces a persisted document.

Semantic identities are checked first. A failure returns one `GRAPH_INTEGRITY`
error and skips evaluation. For pure UML models, root, endpoint and ownership
integrity are checked next, followed by the existing structural-model rules.
The graph and diagram APIs retain their own presentation validation.

A model containing non-`uml.*` elements or relationships other than the `owns`
ownership link is a mixed-domain model. It returns
`UML_EXTERNAL_DOMAIN_UNVERIFIED`, explicit `UML_EXTERNAL_REFERENCE_UNVERIFIED`
warnings for represented cross-domain endpoints and `$ref` values, and
the error `UML_MODEL_SEMANTICS_UNVERIFIED`. This error reports an unsupported
validation capability, not a proven UML violation. `validity` is `unverified`
and `has_errors` is `true`, so legacy error-only consumers cannot report success.
The semantic projection preserves the original relative order, properties and
collection values of UML records, excluding external-domain objects and relations
from evaluation rather than interpreting Rust aliases as UML. The source document
is never changed. Two conservative coverage levels apply:

- `implemented_uml_subset_rules_only`: all implemented checks run on the complete
  UML projection when no relation endpoint, ownership link, designated root or
  nested local `$ref` crosses between UML and an external domain, in either
  direction. External records remain explicitly unverified. This is a whole-UML
  projection, not independent evaluation of apparently disconnected components.
- `implemented_uml_scalars_and_integrity_only`: a cross-domain dependency prevents
  proving closure. `UML_CROSS_DOMAIN_RULES_UNVERIFIED` states that relational and
  global rules, including namespace and inverse-reference queries, are suspended
  for the UML projection. Normative scalar contracts still run for every UML
  record, and local root, missing-endpoint and ownership integrity checks still
  report independently provable errors. Known external endpoints are unresolved
  boundaries, not falsely reported as missing or wrongly typed UML elements.
  Ownership checks traverse the original authored `owns` graph, including opaque
  external identities, for every ownership component touching UML. Multiple
  owners and cycles crossing a domain boundary are still provable integrity
  violations. Entirely external, disconnected ownership components are not
  evaluated as UML.

The conservative global suspension is necessary because some rules inspect all
candidate objects of a metaclass, including malformed candidates without explicit
references. Connected-component partitioning alone cannot justify absence or
multiplicity conclusions. Finer rule-level dependency tracking remains future
work. Proved UML errors are returned alongside the capacity blocker; `validity`
remains `unverified` rather than implying that suspended checks were evaluated.

`has_errors: false` means only that no error diagnostic was reported. It is
not proof of validity; mixed-domain evaluation always returns a capacity error
and explicit `unverified` validity. No response proves complete UML 2.5.1
conformance or generation readiness. Warnings and unsupported-capability
diagnostics remain visible.

## Contract errors

`Err` contains JSON, not an unstructured panic message:

```json
{
  "api_version": 1,
  "error": {"code": "UNSUPPORTED_API_VERSION", "message": "expected api_version 1, received 2"}
}
```

Codes are `INVALID_REQUEST` (JSON/envelope), `UNSUPPORTED_API_VERSION`,
`INVALID_MODEL` (document decoding/version), and
`RESPONSE_SERIALIZATION_FAILED` (response encoding). Message text explains the
failure but is not a substitute for the stable code.

## Effects and transition

No external effects: no filesystem, network, process, clock, sink or external
callback. It does not mutate the input or retain caller state. Returned bytes
are repeatable for the same input and implementation, including A-B-A calls.

This is a functional migration slice, not completed encapsulation. Some current
consumers still access internal types and modules; they must migrate to this
boundary before those exports can be removed. This beta does not guarantee
backward compatibility with earlier APIs or document formats.

## Graph queries

Association validation evaluates `Association-ends_must_be_typed` and
`Association-binary_associations` from UML 2.5.1 §11.8.1.8 even when some
`memberEnd` references are unresolved. A known untyped Property produces
`UML_ASSOCIATION_ENDS_MUST_BE_TYPED`. Two distinct known aggregated ends, or
aggregation among at least three known ends, produce
`UML_ASSOCIATION_BINARY_ASSOCIATIONS`. External references never become invented
local Properties or distinct local identities.

Without a decisive local counterexample, unresolved evidence produces
`MDE_ASSOCIATION_END_TYPES_UNVERIFIED` or
`MDE_ASSOCIATION_AGGREGATION_UNVERIFIED`. Malformed collections and local values
remain errors. The editing gate uses the same predicates; validation does not
materialize or repair incomplete ends. Ownership and navigation checks remain
independent of resolved end Types. These diagnostics describe the implemented
rules, not complete UML conformance or a complete `memberEnd` query.

UML type compatibility uses one shared, iterative Generalization traversal for
complete ancestry and existential reachability. UML 2.5.1
`Classifier-conformsTo-spec` is identity or a path to the requested Classifier,
so a known local path can prove compatibility even when another branch remains
unresolved. It does not certify the unknown branch or complete UML validity.
Without a witness, an incomplete graph remains unverified rather than returning
false. Known malformed edges, invalid specialization, final parents and cycles
still take precedence over a positive witness. Complete ancestry queries inspect
known branches before returning an unresolved result, and never cache a partial
closure as complete. Unordered Generalization edges use stable relationship IDs
for traversal and error precedence; ordered members retain their existing order.
These semantics feed existing validation diagnostics, including Property
subsetting and redefinition, without adding another validator or API contract.

Two additional functions use the same `&str -> Result<String, String>` boundary:

- `reachable_graph_nodes`: request fields `api_version`, `model`, `roots` (array
  of IDs) and `relation_kinds` (array of strings). Returns
  `{"api_version":1,"node_ids":[...]}` with sorted unique IDs, including roots.
  Traversal is outgoing and follows only listed kinds. Empty roots return no
  nodes; empty kinds return only roots. Unknown kinds match no edges.
- `shortest_graph_path`: request fields `api_version`, `model`, `from`, `to`,
  optional `relation_kind` (string or null). Absent/null selects all kinds.
  Returns `{"api_version":1,"relation_ids":[...]}` in traversal order, null
  when unreachable, and an empty array for identical existing endpoints.
  Ties use the existing index ordering by next node ID, kind and relation ID.

Unknown envelope fields are rejected. Both functions first run graph integrity
validation, returning `GRAPH_INTEGRITY` rather than silently dropping broken
edges. This is not structural UML conformance validation. Unknown queried nodes
return `UNKNOWN_NODE`, including unknown reachability roots (unlike the legacy
index's permissive helper). Graph vertices include model elements and semantic
UML relationships recognized by the index; diagram/view/document IDs are not
vertices. Request array order is preserved; output node IDs are sorted explicitly.
Neither function exposes internal graph objects or relation structs. The same
no-effects and A-B-A determinism contract applies. Legacy exports still remain.

Private `src/api_contract.rs` shares envelope version/error formatting and full
document codec checks between the two API directories; it is not an additional
public interface.

Graph-query integrity deliberately excludes numeric presentation constraints
(diagram dimensions, geometry, style and timing), so a visual defect cannot block
an unrelated topology query. It still checks global IDs, semantic root, relation
endpoints, ownership/declaration/physical-parent constraints, hierarchy cycles,
and view subject references. Existing full validation retains presentation
checks. Schema decoding still requires well-formed presentation data when present;
this is not a lenient document recovery API. Neither path claims checks beyond
the existing graph validator (for example, it does not add view endpoint checks).

Request/document decoding errors use boundary-owned messages instead of exposing
Serde's internal Rust type names. Request errors include JSON line/column;
document errors identify unsupported versions or an invalid document schema.

## Legacy graph validation

Document decoding is documented in the
[MDE API](../mde-api/README.md#document-decoding). Its versioned model envelope is
accepted by the kernel validation functions without changing their contracts.

`validate_graph_model(input_json: &str) -> Result<String, String>` accepts the
same envelope as `validate_uml_model`. It returns the same diagnostic shape with
coverage `graph_integrity_and_presentation`, preserving full legacy
`Graph::validate` behavior (including visual geometry/style validation) for future
CLI migration. It does not run structural UML rules. Invalid decoded graphs return
`Ok` with `has_errors:true` and the first `GRAPH_INTEGRITY` diagnostic, so consumers
must check that flag before proceeding. Contract errors remain `Err`.

Graph validation is a pure in-memory operation with no I/O, clock, callback,
sink or retained state. Existing typed exports remain exposed. Topology queries
retain their narrower presentation-independent integrity check; that is
intentionally not the full document/presentation validation gate.

## Generation dependency analysis

`analyze_generation_dependencies(input_json: &str) -> Result<String, String>`
accepts exactly `api_version:1`, `model`, and `selected_ids` (source element IDs).
It reuses the existing pure dependency analysis without exposing its Rust IR.
Selection is exact, not a scope-expansion request: duplicate IDs collapse, empty
selection yields no units, and unknown IDs (including relation/diagram IDs) return
`UNKNOWN_SELECTION`. No dependency or ownership closure is added automatically.

The response contains `api_version:1`, coverage `generation_dependency_analysis`,
and `units` in prerequisite-first order; independent ties are ordered by source
ID. A unit has `id`, `kind`, `name`, and `requires`, whose entries have `id`,
`kind`, `name`, and `relation_kind`. Requirements retain external dependencies
outside the selection, sorted and deduplicated by `(id, relation_kind)`.

Ordering edges are the existing `depends_on`, `uml.dependency`, `uml.usage`,
`uml.generalization`, `uml.interface_realization`, `uml.realization`,
`uml.template_binding`, and `build.depends_on`. Source edges point from client to
supplier; scheduling reverses that direction. Association, ownership and physical
containment do not become compilation dependencies. Only selected-to-selected
edges impose order, so cycles outside that subgraph are not errors for this call.
Selected dependency cycles return `GENERATION_CYCLE` with a stable message, not
an insertion-order-dependent internal cycle witness.

The function requires reference/identity/hierarchy graph integrity, excluding
numeric presentation checks. It does not run structural UML rules or certify a
model can generate a target project. Other errors use the established
`INVALID_REQUEST`, `UNSUPPORTED_API_VERSION`, `INVALID_MODEL`, `GRAPH_INTEGRITY`
contract. No files, paths, templates, processes, clocks, sinks or callbacks are
used; all state is invocation-local and input is unchanged. Tests cover A-B-A,
concurrent calls, reordered inputs, selection errors, cycles and external
requirements. This is dependency analysis only, not complete target-graph,
trace-graph, DPO/SPO/TGG transformation, artifact planning or publication. See
`docs/bellatrix/MDE_GRAPH_TRANSFORMATION_ARCHITECTURE.md` for the remaining stages.

A graph-valid ordering dependency may target a semantic UML relationship identity
rather than a source Element. The current generation IR cannot represent that
requirement. For selected clients, this returns `GENERATION_UNSUPPORTED_TARGET`
(including external targets outside the selection), rather than silently omitting
it from `requires`. The shared analyzer enforces this rule for legacy consumers
as well. Unsupported dependencies of unselected clients do not expand the scope.

## Diagram validation

`validate_uml_diagram(input_json: &str) -> Result<String, String>` accepts exactly
`api_version:1`, `model`, `diagram_id` (string) and `diagram_kind` (string).
The explicit requested kind prevents a stored diagram from silently being checked
as a different category. Accepted kind names are `class`, `object`, `package`,
`component`, `composite_structure`, `deployment`, `profile`, `use_case`, `activity`,
`state_machine`, `sequence`, `communication`, `interaction_overview` and `timing`.
Names are case-sensitive; other names return `UNSUPPORTED_DIAGRAM_KIND` in the
versioned error envelope. API/model decoding is checked before the kind name.

The successful response uses the validation diagnostic shape with coverage
`implemented_model_and_diagram_rules_only`. Full graph integrity and presentation
checks run first; a failure returns one `GRAPH_INTEGRITY` error and skips semantic
and projection validation. Otherwise existing whole-model semantics and the
selected diagram's projection rules run. A missing diagram produces
`UMLDI_DIAGRAM_MISSING`; an incompatible stored category or interaction kind
produces `UMLDI_DIAGRAM_KIND_MISMATCH`. Unknown stored categories retain the
existing `UMLDI_UNSUPPORTED_DIAGRAM_TYPE` diagnostic.

The projection checks include context type/multiplicity, shape/edge geometry and
visible endpoints under existing editor policy. Visual draft and unsupported
coverage warnings are preserved. Whole-model errors may concern elements outside
the selected diagram. Other diagrams receive graph/presentation integrity checks,
but their projection rules do not run. This is not complete UML/UMLDI conformance
or generation certification. No new normative rules are claimed by this adapter.

Like the other boundary functions, this function has no external effects or
retained state and returns sorted diagnostics. The original model-only and graph
validation contracts remain unchanged. No UI or CLI migration is included.

### Existing frontend consumers

The legacy `validate_structural_model` and `validate_structural_diagram` functions
now order diagnostics by the same `(code, subject_id, severity, message)` key as
the JSON boundary. This also applies to their traced adapters and existing Cordis
and read-only preview consumers, which delegate to those kernel functions. Input
member ordering and diagnostic multiplicity are preserved; this changes report
ordering only, not validation rules or severity. Early-return diagram diagnostics
are sorted too. Consumers that display or return only the first qualifying error,
including `application/commands/validation.rs`, may now select a different error
message when several violations exist. Command acceptance/rejection is unchanged;
the ordering change is observable in the selected explanation.

These consumers still use the legacy typed contracts, not the function-only JSON
boundary. Their migration remains separate. Execution trace timestamps and host
run identifiers are observability data, not deterministic kernel results. Report
ordering does not promise invariance under input array permutations: those arrays
can express ordered UML members and graph first-error selection remains unchanged.

## Semantic reparenting

`reparent_uml_element` is implemented and documented in the
[MDE API](../mde-api/README.md#semantic-reparenting-candidate) because it invokes
an application command and computes a new complete document. Normative ownership
rules remain in the UML kernel. The application retains responsibility for atomic
publication and revision checks.

## Ownership checks in rootless drafts

Model and diagram validation check authored semantic ownership even when the
model has no designated `root_model`. The existing ownership-role checks and
semantic cycle traversal now run with the shared contracts, including ownership
derived from specialized relationship sources. An absent root does not make an
invalid owner/child composite role or a mixed explicit/derived ownership cycle
valid. Unsupported role evidence remains a warning, not a fabricated violation.

Rootless drafts may still omit owners: this change does not enable the rooted
model's `mustBeOwned`, namespace visibility or designated-root requirements for
those drafts. Graph integrity remains a separate prerequisite and its explicit
`owns` cycle checks are unchanged. Rooted models retain the same checks without
duplicating their ownership diagnostics. This is implemented-rule coverage only.

Legacy textual composite declarations retain their unresolved status when
ownership roles are inspected, just as in property-contract validation. A known
explicit composite role can still establish membership independently. Relevant
stored roles are all inspected before accepting that proof or returning unknown,
so a malformed/duplicate reference remains an error even alongside unresolved
legacy or external evidence; unknown evidence is not treated as false membership.

## Composite structure instance contexts

Diagram validation now evaluates the InstanceSpecification branch of
`UMLCompositeStructureDiagram-modelElement_type` from the pinned UMLDI.xmi.
At least one resolved classifier must conform to StructuredClassifier. A known
witness establishes this existential condition even alongside unresolved external
or extension classifiers. Property-contract checks inspect the entire classifier
collection first: malformed, duplicate, missing local or known wrong-type
references remain errors even when a valid witness is present.

With no witness, a fully resolved empty/non-structured classifier collection
produces `UMLDI_CONTEXT_TYPE`; unresolved evidence retains the
`UMLDI_INSTANCE_CONTEXT_UNSUPPORTED` warning. The classifier property still emits
its independent model diagnostics. This replaces the previous blanket unsupported
warning for instance contexts; it adds no I/O, resolver, stored result or public
Rust type, and is reached through `validate_uml_diagram` as well as legacy callers.

Verification limit: the existing unit case
`composite_structure_requires_one_resolved_context` still expects the former
blanket unsupported warning for an instance without classifiers. Its expectation
is now outdated: an absent classifier property represents an empty collection,
so the implemented existential check reports `UMLDI_CONTEXT_TYPE`. Unit tests
were neither changed nor run in this slice; a successful kernel `cargo check`
is compilation evidence only, not a green test-suite claim.

## Normative context metaclass checks

Direct diagram contexts use the kernel's normative metaclass mapping and
inheritance, rather than the smaller editor-tool type catalog. In particular,
CompositeStructure accepts supported Behavior subtypes that inherit Class and
therefore StructuredClassifier (for example Activity, StateMachine and
Interaction), as established by the pinned UML.xmi inheritance graph.

For CompositeStructure and the behavioral/interaction context checks, a known
incompatible metaclass produces `UMLDI_CONTEXT_TYPE`; an existing context with an
unmapped extension metaclass produces warning `UMLDI_CONTEXT_TYPE_UNVERIFIED`.
Unknown is not treated as either proven conformance or a proven type violation.
Missing identities and context cardinality retain their existing errors, and
other independent model diagnostics remain visible. InstanceSpecification
contexts retain the separate classifier-witness check described above.

## Basic classifier authoring and semantic views

The existing application `CreateElement` command now permits Class and Interface
creation without an explicitly selected owner by using its existing deterministic
Model adoption path. Their views receive real semantic subject IDs. An explicit
owner is never silently replaced: the canonical `set_owner` composite-role check
accepts valid package/nested-classifier ownership and rejects unsupported pairs.
Other tool categories retain their existing owner requirements.

The operation remains copy-on-write and goes through the existing graph,
association, template and semantic change-validation gates before returning.
`CreateElementOrVisualDraft` still preserves genuine semantic failures as visual
drafts with their diagnostic reason; the change removes the avoidable early
selection restriction only for basic Class/Interface authoring. No UI, revision
publication or new public Rust contract is introduced.

Model adoption may assign existing unowned packageable elements to its root,
according to the existing adoption policy. Basic classifier ownership identities
now come from `set_owner`'s collision-checked model-derived allocation, rather
than the package-only command's revision-seeded ownership edge allocation.
Existing semantic associations already materialize member ends and AddMember
already creates semantic operations; those paths are unchanged by this slice.
Unit tests were neither edited nor run; compilation is not a suite pass.


## Incomplete Association navigation

`authoring_knowledge` is a typed semantic input of the universal document, outside
OMG UML properties and outside presentation or source provenance. Its
`associations` map is keyed by Association identity. Each entry contains a
`navigation_unverified` array of association-owned Property identities. Duplicate
Association keys are rejected during JSON and BLX decoding. Duplicate, unresolved,
non-owned or contradictory end references are preserved by the document codec
and diagnosed only by the UML kernel.

The kernel requires these identities to be unique, a subset of `ownedEnd`, and
disjoint from `navigableOwnedEnd`. Classifier-owned ends cannot be marked unknown.
`navigation_knowledge` returns `Known(true)`, `Known(false)` or `Unverified`.
The normative Boolean query and complete `navigableOwnedEnd` collection query
return an unsupported result if their required knowledge is incomplete. Known
members still contribute to namespace checks without claiming set completeness.

A valid unresolved declaration produces
`MDE_ASSOCIATION_NAVIGATION_UNVERIFIED` and the error-severity capacity blocker
`UML_MODEL_SEMANTICS_UNVERIFIED`. The API reports `validity: "unverified"`,
`has_errors: true` and `coverage: "implemented_rules_with_incomplete_authoring"`.
Independent UML violations remain in `diagnostics`. As with mixed domains,
unverified status takes precedence and does not hide those violations.

`SetAssociationEndNavigation` updates the normative navigable set and the
knowledge gap atomically. Reparenting, removal and other edits cannot leave new
stale knowledge references. Model markers and the Inspector consume the kernel's
knowledge result rather than interpreting unknown as false. The Protobuf schema
records this typed input in Project field 13; JSON and BLX preserve the same state.

The MDJ adapter translates StarUML's absent or explicit `"unspecified"`
navigation to this knowledge gap. Source field presence is encoding metadata
only. StarUML's omitted multiplicity becomes the canonical empty string, not an
invented UML `1..1`. Untouched export returns the source bytes, and edited export
preserves absent fields while their defaults remain unchanged. Explicit decisions
are serialized from current authoring knowledge and UML values. Aggregation
`shared`/`composite`, `ownerAttribute`, qualifiers and unrecognized fields remain
opaque until their inverse mappings are implemented.

Defaults are verified against the bundled StarUML reference:
`crates/bellatrix_design_system/docs/ref/staruml/extensions/essential/uml/elements.js`
(`UMLRelationshipEnd`) and its `metamodel.json`. The save/reopen/decision flows are
specified in `tests/features/mdj_association_lifecycle.feature` and implemented in
`tests/bdd_mdj_association_flows.rs`.

## Standalone UML Class to Rust struct

`generate_rust_struct(&str) -> Result<String, String>` accepts exactly
`api_version:1`, `model` and `element_id`. The existing semantic document contract
is used, including its `id` and `name` metadata, but no UML root, Cargo package,
filesystem model, declaration placement or project dependency graph is required.
Presentation is ignored as in `validate_uml_model`. The same UML evaluator runs
first; errors block emission and return `UML_GENERATION_BLOCKED` together with
its full `uml_validation` response. Warnings remain visible in successful output.
This does not certify complete UML conformance or Rust compilability.

The initial mapping is deliberately bounded: one `uml.class`, its explicitly
owned `uml.attribute` fields in existing ownership order, multiplicity `1..1`,
and source-qualified Boolean/String identities from the pinned OMG primitive
libraries. Those types map to `bool` and `::std::string::String`. A local type
named `String` or authored type text is not substituted for a library identity.
Numeric ranges/precision, optional or collection fields, associations, inheritance,
generics, methods, nested declarations, defaults and other unmapped properties
return `RUST_STRUCT_UNSUPPORTED` rather than disappearing or acquiring guessed
semantics. Explicit inactive/default flags are accepted; nondefault flags require
an implemented mapping. This is a target transformation limit, not a UML violation.

Public visibility maps to `pub`; private or absent visibility emits no export.
Other visibility kinds are unsupported. Names are preserved verbatim: the kernel
does not parse Rust, escape keywords or claim that a valid UML name is a valid
Rust identifier. That remains the external compiler's responsibility. The existing
standalone filename policy lowercases ASCII letters and replaces characters other
than ASCII alphanumerics/underscore with `_`, then appends `.rs`. The existing
artifact path gate checks the result; it does not consult a filesystem graph.
For example `UserProfile` resolves to `userprofile.rs`.

The response contains `coverage:"uml_class_to_rust_struct_subset"`,
`target:{language:"rust",edition:"2024",crate_type:"lib"}`, one standard artifact
`{path,contents,origin}`, source-ID `trace` entries and the original UML/renderer
diagnostics. `contents` is UTF-8 source text; `origin` is the UML Class ID. Trace
entries correlate class/field IDs with target kinds and names within that artifact.
These are derived return values, never an independently editable target model.
The existing Rust renderer/templates and artifact plan are reused.

An isolated struct is a **library**, not an executable. No `main` is invented.
A caller can later compile the returned content with
`rustc --edition 2024 --crate-type lib userprofile.rs`, but this API performs no
filesystem writes, compiler invocation, project resolution or caching. Repeated
calls, edits and restoration derive their results from the current input only.
`tests/uml_rust_struct_flow.rs` is one public-boundary scenario covering generation,
identity/filename changes, unsupported mappings, UML rejection, repair and external
Rust compilation through stdin. A Rust keyword name is intentionally left for
rustc to reject, demonstrating the two validation authorities.
