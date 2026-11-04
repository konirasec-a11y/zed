# Functional MDE API: version 1

This directory is the explicit application/document boundary of `bellatrix_mde`.
It is separate from [`kernel-api/`](../kernel-api/README.md), which exposes kernel
validation, graph queries and generation analysis. MDE operations coordinate the
existing document codec and application commands; they do not redefine UML rules
or introduce another model.

The external communication boundary is Cordis events. These Rust functions are
the engine entry points used by the Cordis adapter, not a second direct route
for the UI, Zed host, CLI or exporters. The adapter receives integration/project
events and invokes MDE; calls inside MDE remain direct. MDE does not depend on
Cordis, UI or Zed types. Purely visual interaction and presentation state remain
inside the UI, rather than becoming integration events.

The public functions implemented in this directory are reexported directly from
the crate root:

- `bellatrix_mde::decode_model_document`, implemented in `document.rs`.
- `bellatrix_mde::encode_model_document`, implemented in `document.rs`.
- `bellatrix_mde::convert_model_document`, implemented in `document.rs`.
- `bellatrix_mde::import_mdj_document`, implemented in `interchange.rs`.
- `bellatrix_mde::export_mdj_document`, implemented in `interchange.rs`.
- `bellatrix_mde::reparent_uml_element`, implemented in `commands.rs`.
- `bellatrix_mde::project_documentation`, implemented in `presentation.rs`.

Their implementation modules and request types are private. There is no second
public namespace, compatibility alias, public command enum or registry for this
boundary. `src/api_contract.rs` privately owns the shared envelope version, errors
and full-document decoder. Both API directories depend on that neutral helper;
neither API depends on the other. The semantic-only UML decoding projection stays
in `kernel-api/contract.rs` and must not be used to save or replace a document.

## Document decoding

`decode_model_document(document: &[u8], format: &str) -> Result<String, String>`
accepts original bytes and an explicit `json` or `protobuf` format (case-sensitive).
Protobuf uses the existing BLX1 header. It returns the version-1 JSON envelope
`{"api_version":1,"model":...}`, directly accepted by the validation functions.
No base64, filesystem access or public Rust model type is required. Existing
codec defaults and normalization are preserved; this is not lossless preservation
of source JSON formatting or unknown fields. Decoding does not validate semantics.
Errors are `UNSUPPORTED_FORMAT`, `INVALID_MODEL`, or
`RESPONSE_SERIALIZATION_FAILED`, with the shared versioned error envelope.

Protobuf can encode non-finite diagram dimensions; JSON cannot. The decoder
returns `RESPONSE_SERIALIZATION_FAILED` for that case instead of silently replacing
numbers with null. The underlying protobuf codec is unchanged.

## Document encoding

`encode_model_document(input_json: &str, format: &str) -> Result<Vec<u8>, String>`
accepts exactly `{"api_version":1,"model":...}` plus the explicit case-sensitive
`json` or `protobuf` format. The result is the existing codec's document bytes:
pretty JSON or BLX1-prefixed Protobuf, without another envelope or base64 layer.
The original model is unchanged. The codec's schema, composition, provenance and
identity checks still apply; no UML validation or full graph validation is added.

Errors use `INVALID_REQUEST`, `UNSUPPORTED_API_VERSION`, `UNSUPPORTED_FORMAT` or
`INVALID_MODEL`. Encoding is not source JSON formatting preservation. Current
authoring knowledge, composition, typed presentation and MDJ provenance use the
same existing JSON/BLX representation rather than a new persisted DTO.

`convert_model_document(document: &[u8], source_format: &str,
destination_format: &str) -> Result<Vec<u8>, String>` converts through the codec
without a JSON envelope. It preserves non-finite binary diagram dimensions for
binary output; JSON output fails with `RESPONSE_SERIALIZATION_FAILED` rather than
replacing them with null. This distinction is necessary for byte-oriented codec
consumers. Other errors are `UNSUPPORTED_FORMAT` and `INVALID_MODEL`. Output uses
the existing codec's normalization; this is not original source-byte replay.

## MDJ interchange

`import_mdj_document(document: &[u8]) -> Result<String, String>` passes the original
bytes to the existing MDJ converter and returns
`{"api_version":1,"model":...,"diagnostics":[{"path":"...","message":"..."}]}`.
Diagnostics retain their adapter order and text, including unsupported mapping
and unresolved-reference notices. They are separate response information, not a
second stored model or a UML-validation result. The model contains the original
source bytes in existing provenance, plus source residue, composition and current
authoring knowledge. Import errors use `MDJ_IMPORT_FAILED`; response encoding
errors use `RESPONSE_SERIALIZATION_FAILED`.

`export_mdj_document(input_json: &str) -> Result<Vec<u8>, String>` accepts exactly
`{"api_version":1,"model":...}` and returns MDJ bytes. Import diagnostics are not
accepted as an extra request field. The full document decoder runs its existing
schema/integrity checks, then the existing MDJ exporter requires provenance.
An unchanged projection returns original source bytes. For supported edits, the
exporter reimports its actual output and compares the complete supported
projection. Unsupported edits fail instead of dropping data or replaying stale
source. Errors use `INVALID_REQUEST`, `UNSUPPORTED_API_VERSION`, `INVALID_MODEL`
or `MDJ_EXPORT_FAILED`, with the existing adapter explanation for export failures.
This introduces no new MDJ mappings and does not certify UML or StarUML parity.

The import response and model request are deliberately different envelopes.
To validate, encode or export an imported model, retain `diagnostics` separately
and construct the request from the response's `api_version` and complete `model`.
Do not route this extraction through an IEEE-754-only JSON object parser, which
can round large integer/residue values. In Rust, deserialize the response's
`model` field as `Box<serde_json::value::RawValue>` and insert its raw JSON text
verbatim as the request's `model` value, without quoting it as a JSON string.
Alternatively, a JSON representation must preserve arbitrary-precision numbers
and reject duplicate fields rather than silently overwrite them. The facade
itself keeps request models raw until the existing document decoder consumes
them. Do not remove or reconstruct provenance, source residue or unfamiliar
properties while extracting the model. The public original MDJ byte slice is
not re-encoded before the adapter captures it.

All these operations are in-memory. Exported/encoded bytes do not reserve a file,
write a destination, publish a revision or acknowledge a save. The host must keep
its snapshot/revision checks and write/acknowledgement protocol.

## Read-only documentation projection

`project_documentation(input_json: &str) -> Result<String, String>` accepts exactly
`{"api_version":1,"model":...}`. It applies the existing graph validation gate
and returns `{"api_version":1,"document":...}` with display text, links and
diagram primitives. UML labels and relationship stroke patterns are resolved
inside MDE. The Cordis export handler supplies this neutral projection to the
HTML renderer, which does not depend on MDE or query UML semantics.

The result is not a persisted model, mutation input or source backup. It omits
source provenance and current authoring knowledge and cannot replace the model.
Text is not HTML-escaped; escaping belongs to the output renderer. No UI events,
GPUI objects, filesystem effects or retained renderer state enter MDE.

## Cordis document integration

The current integration events live in `crates/bellatrix_mde_cordis/src/`:

- `mdj_interchange.rs`: `ModelingPrepareDocument` accepts bytes and
  `ModelingDocumentFormat::{ProjectJson, ModelJson, ModelProtobuf, StarUml}`. It
  returns `PreparedModelingDocument` containing the platform `ProjectDocument`
  and diagnostic strings. Preparation runs the existing presentation safety
  checks, not UML validation, and does not publish, open a session or save.
  These request/response records contain no MDE model or UI types.
- `document_codec.rs`: `ModelingConvertDocument` accepts bytes plus source and
  destination format strings and returns converted bytes. Its handler delegates
  to the byte-to-byte codec facade without an intermediate JSON model envelope.
- `mdj_interchange.rs`: `ModelingExportMdj` retains the exact-snapshot save
  preparation contract. `ModelingApplyMdjEdit` retains the inverse-export gate
  before atomic snapshot publication. Their existing snapshot/revision guards
  are not replaced by the stateless document preparation event.

The Zed host dispatches preparation through Cordis and injects an asynchronous
preparation callback into the native-document UI adapter. The UI passes source
bytes and retains its buffer guards, local interaction state and messages.
`native_document.rs` no longer parses the imported MDE model or calls MDE
import/graph functions. `ProjectDocument` belongs to the platform project crate, which does not
depend on MDE. The Zed host has no direct Cargo dependency on MDE.

This is an incremental integration checkpoint. The host still obtains the
workbench's typed snapshot through its existing `from_document` helper. That
helper and other workbench snapshots and
event payloads still expose MDE typed contracts, and other consumer paths remain
to migrate. In particular, the existing MDJ edit request still carries `Command`.
Removing direct calls on one path does not establish complete UI independence
or eliminate those remaining transitive dependencies.

## Semantic reparenting candidate

`reparent_uml_element(input_json: &str) -> Result<String, String>` accepts exactly
`api_version:1`, `model`, `element_id` and `owner_id`. The IDs are required strings;
null/omitted owners are rejected rather than silently detaching an element. The
result is `{"api_version":1,"model":...}`, suitable for subsequent validation.

This pure function reuses the existing transactional `SetOwner` command, not an
Explorer placement override. It first requires graph/presentation integrity,
then applies the supported UML composite ownership rules, root/derived-owner
restrictions, cycle checks and the command's semantic change-validation policy.
A rejected candidate returns no partial model and never changes the input.
Existing ownership relation identity is retained when reparenting; a new ownership
relation receives the existing deterministic, collision-checked model-derived ID.
Other normalization performed by the command, including association projection
synchronization, remains in force; this is not a raw single-edge rewrite.

Errors use the shared envelope: `INVALID_REQUEST`, `UNSUPPORTED_API_VERSION`,
`INVALID_MODEL`, `GRAPH_INTEGRITY`, `OWNERSHIP_REPARENT_REJECTED`, or
`RESPONSE_SERIALIZATION_FAILED`. The rejection message carries the existing
command reason. Existing semantic errors may remain under the command's delta
policy: success is not full UML conformance certification. Full validation remains
a separate operation and should inform the caller's publication policy.

Existing Explorer overrides are preserved, so an old presentation override can
still visually mask the new semantic owner. Reconciliation of those placements
requires an explicit application policy; semantic reparenting does not silently
delete presentation information or reinterpret diagram placement as UML ownership.

## Shared contract and effects

`api_version` is independent of the document's `schema_version`; both currently
use version 1. Request envelope fields are explicit and unknown fields are
rejected. Nested model input remains raw JSON until it reaches the document
decoder, preserving duplicate-key rejection for typed authoring knowledge.
The single document contract remains in `contracts/`, including composition,
source provenance and current authoring knowledge. This facade does not create
a second DTO model or reinterpret source-specific fields as normative UML.

Errors are JSON strings with `api_version` and an `error` object containing
`code` and `message`. Codes identify the failure; message text is explanatory.
None of these facade functions performs I/O, reads clocks, invokes callbacks or retains state.
Results depend only on the input and implementation. They do not publish, save,
advance revisions or mark documents clean. The application must compare the
source snapshot/revision and publish atomically; a client-supplied revision alone
cannot provide that guarantee.

## Existing exposure and remaining migration

This is the first physical separation, not complete public API encapsulation.
`mde.rs` still exposes `contracts`, `commands` and `uml`
modules, together with typed graph, compiler, persistence and projection exports.
CLI, Cordis and workbench consumers still use those types and, in some cases,
UML internals. They must migrate before the corresponding visibility can close.
Those exports are not duplicated under this facade.

MDJ conversion still lives in `persistence/mdj_conversion.rs`; the new MDE
interchange functions delegate to it. Both `mdj` and `mdj_conversion` are now
private after their consumers migrated; no compatibility aliases remain.
No general command-serialization protocol is
introduced here. Further application-command entry points belong in this
directory, with explicit requests and without leaking the internal `Command`
enum. Snapshot comparison and save acknowledgement remain in the host integration
rather than a second session manager here.

The existing `validate_graph_model` and `validate_uml_diagram` kernel facades still
include presentation checks. Their behavior and placement are unchanged in this
slice; this directory split does not claim that all existing dependency or
validation boundaries are already isolated. UML rule implementations remain in
`KERNEL/uml/`. This beta does not guarantee compatibility with earlier API or
document versions. This relocation preserves current function contracts without
retaining old private module aliases.

## Inventory of the current public surface

This inventory follows the declarations and reexports in `src/mde.rs`, expands
`pub use contracts::*`, and identifies the actual definition files rather than
only the intermediate reexport modules. All source paths below are relative to
`crates/bellatrix_mde/src/`. A name in a root table means
`bellatrix_mde::<name>`; a module table gives the namespace to prepend.

Functions, structs, enums, traits, type aliases, constants and modules are
different API items. The thirteen function-only facades below are not the entire
crate API. The remaining tables enumerate the current named root exports and the
top-level declarations in the public contract, command and UML modules. Private
MDJ implementation items are identified separately, not counted as public paths.
This is not a complete inventory of inherent methods, trait methods, public
fields, enum variants or derived trait implementations. Those remain additional
public surface attached to the listed types. No visibility is changed by this
inventory, and it is based on source inspection, not a build or rustdoc run.

### Function-only facades

These are the explicit functional boundaries. Request/response implementation
types remain private; their current public signatures contain no domain structs.

| Root functions | Definition file | Responsibility |
| --- | --- | --- |
| `decode_model_document` | `mde-api/document.rs` | MDE document decoding |
| `encode_model_document` | `mde-api/document.rs` | MDE document encoding |
| `convert_model_document` | `mde-api/document.rs` | Byte-to-byte codec conversion without a JSON envelope |
| `import_mdj_document`, `export_mdj_document` | `mde-api/interchange.rs` | MDE interchange with explicit diagnostics and inverse verification |
| `project_documentation` | `mde-api/presentation.rs` | Read-only neutral documentation projection for the Cordis export handler |
| `reparent_uml_element` | `mde-api/commands.rs` | MDE application-command candidate |
| `validate_uml_model`, `validate_uml_diagram`, `validate_graph_model` | `kernel-api/validation.rs` | Kernel validation adapters, with the presentation overlap noted above |
| `reachable_graph_nodes`, `shortest_graph_path` | `kernel-api/graph.rs` | Kernel graph queries |
| `analyze_generation_dependencies` | `kernel-api/generation.rs` | Kernel generation dependency analysis |

### Other functions and types reexported directly at the root

These are the existing typed surface, not additional function-only facades.
They expose implementation contracts directly and remain pending consumer
migration or an explicit decision to keep a supported typed API. A private
implementation module does not make its reexported items private.

| Definition file | Root functions | Root structs, enums and type aliases |
| --- | --- | --- |
| `persistence/io.rs` | `decode`, `encode` | Enums: `Format`, `ModelIoError` |
| `observability/execution_trace.rs` | `execution_trace_jsonl`, `validate_structural_diagram_traced`, `validate_structural_model_traced` | Structs: `ExecutionEvent`, `ExecutionTrace`. Enums: `TraceOutcome`, `TracePhase` |
| `KERNEL/compiler/emission/construction_script.rs` | `python_construction_script` | Enum: `ConstructionScriptError` |
| `KERNEL/compiler/analysis/generation_graph.rs` | `analyze_generation_graph` | Structs: `GenerationModel`, `GenerationReference`, `GenerationUnit`. Enum: `GenerationGraphError` |
| `KERNEL/graph/graph.rs` | None; operations are methods on `Graph` | Struct: `Graph`. Enum: `GraphError` |
| `KERNEL/graph/index.rs` | None; operations are methods on `GraphIndex` | Struct: `GraphIndex`. Enum: `GraphLookupError` |
| `KERNEL/graph/scope.rs` | `select_elements` | Enums: `Scope`, `ScopeError` |
| `KERNEL/compiler/operations/operation_graph.rs` | None; operations are methods on the exported types | Structs: `InputPort`, `OperationDefinition`, `OperationGraph`. Enums: `Operation`, `OperationGraphError`, `OperationValue`, `PortType`. Type aliases: `OperationImplementation`, `OperationValues` |
| `KERNEL/compiler/artifacts/path.rs` | `physical_path`, `validate_segment` | Enum: `PathError` |
| `KERNEL/compiler/artifacts/plan.rs` | None; operations are methods on the exported types | Structs: `Artifact`, `Diagnostic`, `Plan`. Enum: `PlanError` |
| `KERNEL/I_O/publish.rs` | `publish_new`, `publish_replace` | None |
| `KERNEL/uml/validation/structural_diagrams.rs` | `element_root_kind`, `relationship_root_kind`, `validate_structural_diagram`, `validate_structural_model` | Struct: `StructuralDiagnostic`. Enums: `RootMetaclass`, `StructuralDiagramKind`, `StructuralSeverity` |
| `KERNEL/uml/semantics/templates.rs` | `template_parametered_element`, `template_parameters` | None |
| `KERNEL/uml/capabilities.rs` | None; operations are methods on the exported types | Enums: `UmlCapability`, `UmlQuickCreate`, `UmlType` |
| `view-model/hierarchy/hierarchy.rs` | `project_hierarchy` | Struct: `HierarchyRow` |
| `view-model/hierarchy/visual_consistency.rs` | `visual_components` | Struct: `VisualComponent` |
| `view-model/inspector/inspector.rs` | `inspect_element` | Structs: `ElementInspector`, `InspectorMember`, `InspectorProperty`, `InspectorReferenceChoice`. Enum: `PropertyEditor` |
| `view-model/notation/presentation.rs` | `classifier_presentation`, `element_display_label`, `element_display_label_with_references`, `format_element_label_for_view`, `member_presentation`, `semantic_identity` | Structs: `ClassifierPresentation`, `ClassifierPresentationOptions`, `MemberPresentation`, `SemanticIdentity` |
| `view-model/notation/instance_presentation.rs` | `format_instance_heading`, `collect_instance_slot_labels` | None |
| `view-model/notation/interaction_presentation.rs` | `format_operand_guard` | None |
| `view-model/notation/endpoint_markers.rs` | `resolve_relationship_view` | Struct: `ResolvedRelationshipView` |
| `view-model/notation/relationship_notation.rs` | `format_relationship_label`, `resolve_relationship_style`, `relationship_presentation`, `relationship_style_for_notation` | Structs: `RelationshipEndpointLabels`, `RelationshipPresentation`, `RelationshipStyle`. Enum: `RelationshipMarker` |
| `view-model/notation/uml_notation.rs` | `stereotype_presentation`, `resolve_element_notation`, `resolve_pseudostate_notation`, `uml_header`, `uml_notation` | Struct: `UmlHeader`. Enums: `StereotypePresentation`, `UmlNotation` |
| `view-model/notation/template_presentation.rs` | `template_parameter_label`, `template_parameter_label_with_references` | None |
| `view-model/authoring/quick_edit.rs` | `quick_edit_for` | Struct: `QuickEditAction`. Enums: `QuickEditAvailability`, `QuickEditControl`, `QuickEditSide` |
| `view-model/authoring/relationship_tools.rs` | None | Enum: `ModelingRelationKind`, reexported through `relationships` |
| `view-model/diagram/relationships.rs` | `projected_relationship_paths`, `quick_relation_rule`, `relation_kind_for_creation`, `reroute_relationships`, `route_relationship` (definition name: `route`) | Structs: `QuickRelationRule`, `RelationshipGraph` |
| `view-model/diagram/uml_geometry.rs` | `classifier_contents`, `folded_note_layout`, `frame_heading_layout`, `interface_layout`, `port_label_layout`, `template_signature_layout` | Structs: `ClassifierContents`, `FoldedNoteLayout`, `FrameHeadingLayout`, `InterfaceLayout`, `PortLabelLayout`, `TemplateSignatureLayout` |
| `view-model/diagram/visual_draft_routing.rs` | `reroute_visual_drafts`, `resolve_visual_center` | None |
| `application/edits/inspector_edit.rs` | `set_inspected_property` | None |
| `application/edits/hierarchy_placement.rs` | `validate_diagram_occurrence`, `validate_model_hierarchy` | None |

`MODELING_NAMESPACE` is a public constant defined directly in `mde.rs`, not a
function or module. `publish_new` and `publish_replace` perform filesystem
publication, and traced validation uses the caller's execution trace. They must
not be described as sharing the no-effects contract of the thirteen facades.
Other typed exports include mutable model/projection operations. This inventory
does not classify all of them as read-only merely because they are functions.

### Shared contracts expanded from the root glob reexport

Every named item in this table is reachable both as `bellatrix_mde::<name>` and
`bellatrix_mde::contracts::<name>`. The definitions remain the single current
document contract, not duplicated facade DTOs. Their Rust exposure remains part
of the existing typed API while consumers migrate; that is distinct from the
versioned JSON document format consumed by the facades.

| Definition file | Functions | Structs and enums |
| --- | --- | --- |
| `contracts/composition.rs` | `composition_paths` | Struct: `CompositionPlacement`. Enum: `CompositionParent` |
| `contracts/project.rs` | None | Struct: `Project` |
| `contracts/section.rs` | None | Struct: `ModelingSection` |
| `contracts/provenance.rs` | None | Structs: `MdjObjectResidual`, `MdjProjectionFingerprint`, `MdjProvenance` |
| `contracts/semantic.rs` | None | Structs: `Annotation`, `Element`, `Implementation`, `Relation`. Enums: `FileContent`, `StereotypeValue` |
| `contracts/uml_authoring.rs` | None | Structs: `AssociationKnowledge`, `AuthoringKnowledge`. Enum: `NavigationKnowledge` |
| `contracts/uml_presentation.rs` | None | Struct: `UmlStyle`. Enums: `AssociationEndConvention`, `StereotypeDisplay` |
| `contracts/endpoint_presentation.rs` | None | Structs: `EndpointMarkerBinding`, `EndpointMarkerComposition`, `EndpointMarkerPart`, `RelationshipEndpointPresentation`. Enums: `EndpointMarkerGlyph`, `EndpointMarkerPlacement`, `EndpointMarkerSource`, `EndpointMarkerTarget` |
| `contracts/presentation.rs` | None | Structs: `Diagram`, `HierarchyPlacement`, `LabelPlacement`, `LayoutBounds`, `Point`, `PresentationSource`, `Style`, `TimingPoint`, `TimingPresentation`, `View`, `ViewEndpoints`, `ViewOccurrence`, `ViewPresentation`, `VisualDraft`. Enums: `Authored`, `Geometry`, `LineMode`, `PathSegment`, `PresentationReference`, `PresentationResource`, `RelationshipRouting`, `TextAlignment`, `TextVerticalAlignment`, `TimingMode` |

### Public modules and additional paths

The public modules below expose more than the root functions. They are the
remaining module-level exposure; the `uml` namespace in particular exposes
kernel implementation details directly rather than enforcing `kernel-api`.

| Public path below `bellatrix_mde::` | Definition file | Current status |
| --- | --- | --- |
| `contracts` | `contracts.rs` | Shared typed contract module; exposure remains |
| `commands` | `application/commands.rs` | Typed application boundary; consumer migration pending |
| `uml` | `KERNEL/uml/uml.rs` | Direct kernel module exposure; consumer migration pending |
| `contracts::endpoint_presentation` and `endpoint_presentation` | `contracts/endpoint_presentation.rs` | Public contract submodule, also imported by the root glob |
| `contracts::semantic` and `semantic` | `contracts/semantic.rs` | Public contract submodule, also imported by the root glob |
| `contracts::uml_authoring` and `uml_authoring` | `contracts/uml_authoring.rs` | Public contract submodule, also imported by the root glob |
| `contracts::uml_presentation` and `uml_presentation` | `contracts/uml_presentation.rs` | Public contract submodule, also imported by the root glob |
| `contracts::presentation` | `contracts/presentation.rs` | Public contract submodule; the explicit private root `presentation` module shadows the glob's module name |

The private root `presentation` module is implemented in
`view-model/notation/presentation.rs`; it is not the public contract module of
the same name. `contracts::composition` is only crate-visible, while its named
reexports above are public. `wire`, `api_contract`, the facade implementation
modules and other private root modules are not independently external modules.

### Command module items and private MDJ implementation

The command names below require `bellatrix_mde::commands::`; they are not
reexported unqualified at the crate root. MDJ internals are no longer public:
`persistence/mdj.rs` defines `decode`, the structs `Object`, `Diagnostic`,
`Reference`, `Index`, and enum `MdjError`; `persistence/mdj_conversion.rs` defines
`import`, `export` and struct `Import`. They are used only inside MDE. There is
no longer an external `bellatrix_mde::mdj` or `bellatrix_mde::mdj_conversion` path.

| Public namespace | Definition file | Top-level public items |
| --- | --- | --- |
| `commands` | `application/commands.rs` | Function: `apply` |
| `commands` | `application/commands/executor.rs` | Enums: `Command`, `ViewOrder`, reexported by `application/commands.rs` |
| `commands` | `application/commands/association_ends.rs` | Struct: `AssociationEndDraft`. Enums: `AssociationEndEdit`, `AssociationEndOwner`, `RemovedAssociationEnd`, reexported through the executor |
| `commands` | `application/commands/kinds.rs` | Enums: `ModelingElementKind`, `ModelingTool`, reexported through the executor |
| `commands` | `application/commands/members.rs` | Enum: `ModelingMemberKind`, reexported through the executor |
| `commands` | `application/commands/name_expression.rs` | Function: `name_expression_text` (definition name: `format`), reexported through the executor |

### Directly exposed UML modules

`KERNEL/uml/uml.rs` declares each of the following public modules. Names in the
last column require `bellatrix_mde::uml::<module>::`. These are top-level source
declarations, not an inventory of their public methods. All remain directly
accessible kernel internals pending consumer migration; they are not additional
versioned functional API endpoints.

| Module | Definition file | Top-level public items |
| --- | --- | --- |
| `classifier_hierarchy` | `KERNEL/uml/semantics/classifier_hierarchy.rs` | Functions: `may_specialize_type`, `resolve_ancestry`, `conforms_to`, `has_ancestor`, `is_substitutable_for`. Struct: `ClassifierAncestry` |
| `feature_compatibility` | `KERNEL/uml/semantics/feature_compatibility.rs` | Functions: `resolve_feature_type`, `compare_type_conformance`, `compare_resolved_types`, `resolve_feature_multiplicity`, `resolve_feature_upper_bound`, `compare_multiplicity_containment`, `compare_property_consistency` |
| `element_graph_queries` | `KERNEL/uml/semantics/element_graph_queries.rs` | Functions: `all_owned_elements`, `all_namespaces`, `all_owning_packages`, `client_dependencies`. Enum: `ElementGraphError` |
| `classifiers` | `KERNEL/uml/semantics/classifiers.rs` | Functions: `owned_members`, `query`, `all_roles`, `has_role`, `validate`. Structs: `ClassifierQuery`, `NamespaceQuery`. Enum: `QueryError` |
| `associations` | `KERNEL/uml/semantics/associations.rs` | Functions: `member_ends`, `owned_ends`, `navigable_owned_ends`, `navigation_knowledge`, `is_navigable`, `association_of`, `owning_association`, `opposite`, `property_type`, `end_types`, `aggregation`, `is_composite`, `multiplicity`, `validate` |
| `instances` | `KERNEL/uml/semantics/instances.rs` | Structs: `InstanceSpecificationRef`, `SlotRef`; operations are methods on these types |
| `namespace_imports` | `KERNEL/uml/semantics/namespace_imports.rs` | Functions: `get_names_of_member`, `is_distinguishable_from`, `members_are_distinguishable`, `validate`, `resolve_namespace_imports`. Struct: `ResolvedImports` |
| `operations` | `KERNEL/uml/semantics/operations.rs` | Functions: `owned_parameters`, `direction`, `return_parameter`, `return_type`, `return_properties`, `validate`. Struct: `ReturnProperties` |
| `parameter_sets` | `KERNEL/uml/semantics/parameter_sets.rs` | No externally public top-level declarations; the module itself is public |
| `operation_signatures` | `KERNEL/uml/semantics/operation_signatures.rs` | Functions: `compare_operation_signature`, `compare_operation_consistency` |
| `property_subsetting` | `KERNEL/uml/semantics/property_subsetting.rs` | Functions: `subsetted_properties`, `subsetting_context`, `compare_subsetting`, `subsetting_diagnostic`, `validate`, `set_subsetted_properties` |
| `property_contracts` | `KERNEL/uml/semantics/property_contracts.rs` | Function: `check_value`. Enum: `PropertyOutcome` |
| `redefinition_contexts` | `KERNEL/uml/semantics/redefinition_contexts.rs` | Functions: `resolve_redefinition_contexts`, `compare_redefinition_contexts` |
| `references` | `KERNEL/uml/semantics/references.rs` | Function: `reference_id`. Struct: `ReferenceIndex` |
| `relationship_endpoints` | `KERNEL/uml/semantics/relationship_endpoints.rs` | Functions: `clients`, `suppliers`, `sources`, `targets`, `related_elements` |
| `root` | `KERNEL/uml/semantics/root.rs` | Functions: `relationship_metaclass`, `object_metaclass`. Structs: `NamedElementRef`, `NamespaceRef`, `RelationshipRef`, `DirectedRelationshipRef`, `CommentRef`, `Model`. Enums: `ElementRef`, `VisibilityKind`. Traits: `Element`, `NamedElement`, `Namespace`, `Relationship`, `DirectedRelationship` |
| `types` | `KERNEL/uml/metamodel/types.rs` | Functions: `literal_lower_bound`, `literal_upper_bound`, `literal_multiplicity`. Structs: `Integer`, `UnlimitedNatural`, `MultiplicityRange`, `RelationEndMultiplicity`. Enums: `LiteralValueError`, `StandardPrimitiveType`, `MultiplicityError`, `RelationEnd` |
| `value_specifications` | `KERNEL/uml/semantics/value_specifications.rs` | Functions: `resolve_value`, `evaluate`, `referenced_instance`, `is_computable`, `integer_value`, `unlimited_value`, `real_value`, `boolean_value`, `string_value`, `is_null`, `has_bound_references`, `lower_bound`, `upper_bound`, `multiplicity`, `validate`. Enums: `EvaluationError`, `LiteralValue`, `ResolvedValue` |
| `components` | `KERNEL/uml/semantics/components.rs` | No externally public top-level declarations; the module itself is public |
| `enumerations` | `KERNEL/uml/semantics/enumerations.rs` | No externally public top-level declarations; the module itself is public |
| `connectors` | `KERNEL/uml/semantics/connectors.rs` | Functions: `defining_end`, `kind`, `validate`. Enum: `ConnectorKind` |
| `deployments` | `KERNEL/uml/semantics/deployments.rs` | No externally public top-level declarations; the module itself is public |
| `profiles` | `KERNEL/uml/semantics/profiles.rs` | Function: `is_required` |
| `metamodel` | `KERNEL/uml/metamodel/metamodel.rs` | Functions: `has_property`, `is_known_metaclass`, `conforms_to`. Public constants are listed below |
| `schema` | `KERNEL/uml/metamodel/schema.rs` | Public functions, structs and enums are listed below |

`uml::metamodel` additionally exposes these constants: `SOURCE_URL`, `SOURCE_PATH`,
`SOURCE_SHA256`, `DIRECT_GENERALIZATIONS`, `DECLARED_PROPERTIES`, `SOURCES`,
`AMBIGUOUS_DEFINITIONS`, `UNRESOLVED_REFERENCES`, `CLASSIFIERS`, `PROPERTIES`,
`UNEVALUATED_CONSTRAINTS`, `OPERATIONS`, `PARAMETERS`, `SPECIFICATIONS`,
`DOCUMENTATION`. They are data, not validation functions.

`uml::schema`, all defined in `KERNEL/uml/metamodel/schema.rs`, exposes:

- Functions: `classifier`, `uml_classifier`, `property_by_id`, `operation_by_id`,
  `parameter_by_id`, `specification_by_id`, `documentation_for`, `conforms_to`,
  `inherited_properties`, `property`, `uml_property`, `effective_default`,
  `read_effective_scalar`, `read_effective_boolean`, `scalar_kind`,
  `validate_scalar`, `validate_element`, `uses_legacy_text_adapter`, `validate_edit`.
- Structs: `XmiId`, `XmiReference`, `SourceDescriptor`, `UnresolvedReference`,
  `AmbiguousDefinition`, `ClassifierDescriptor`, `PropertyDescriptor`,
  `ConstraintDescriptor`, `OperationDescriptor`, `ParameterDescriptor`,
  `SpecificationDescriptor`, `DocumentationDescriptor`, `InheritedProperties`.
- Enums: `Document`, `ReferenceResolution`, `ClassifierKind`, `Aggregation`,
  `DefaultValue`, `ParameterDirection`, `LookupError`, `ScalarKind`,
  `UnsupportedContract`, `ValueError`.

`uml::state_machines` and `uml::activities` are private modules, not additional
public namespaces. The template functions exported at the crate root come from
the separate private root `templates` module; there is no public `uml::templates`
module in the current declaration tree.

### Methods and migration limits

Exporting a type also exposes its public methods and applicable trait interfaces.
For example, `Graph::{new, validate_identifiers, validate_presentation, validate}`
is defined in `KERNEL/graph/graph.rs`, `Project::{element, validate_hierarchy}` in
`contracts/project.rs`, and `ModelingSection::{from_model, to_model}` in
`contracts/section.rs`. `GraphIndex` has traversal/query methods in
`KERNEL/graph/index.rs`; `uml::root::Model` and `uml::references::ReferenceIndex`
have their own public query/validation methods in their listed files. These
examples demonstrate why counting free functions is not a complete API audit.
The remaining public methods and trait/field/variant surface have not been
exhaustively inventoried here and must be reviewed before closing visibility.

The migration distinction is:

1. The thirteen facades already have explicit private implementation locations.
   Only the Cordis adapter should invoke them across the engine boundary.
2. Shared `contracts` remains the authoritative data representation; its broad
   Rust export is existing exposure, not a new facade contract or a duplicate model.
3. External document, command and projection consumers migrate to Cordis events;
   they do not gain a separate direct MDE route. Neutral event contracts and UI
   presentation state must replace remaining external implementation types.
4. The Cordis adapter invokes bounded MDE/kernel operations on behalf of those
   consumers before the remaining internal modules/types can close. Effectful publication and
   observability require their own explicit effect contracts, not a blanket claim
   that the public API is pure.

Nothing in this inventory grants a compatibility guarantee or claims that the
remaining migration is done. The two private MDJ modules are the closed portion;
the other legacy exposures above remain explicitly visible in the source.
