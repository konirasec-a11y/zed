# MDE source responsibilities

`src/mde.rs` preserves the public Rust API through explicit module paths and
reexports. The directories describe responsibilities, not separate runtimes or
copies of the model. KERNEL includes UML specification semantics, graph
formalism and the mini-compiler implementation from model analysis to project
artifacts and emission; it is not limited to passive validation.

- `kernel-api/`: private function implementations for the public kernel boundary:
  validation, graph queries and generation analysis.
- `mde-api/`: private function implementations for the public MDE document and
  application boundary, currently decoding and semantic reparenting candidates.
  Both boundaries share the private `api_contract.rs` envelope/codec helper;
  neither depends on the other.
- `KERNEL/graph/`: graph integrity, indexing and graph-scope queries.
- `KERNEL/uml/metamodel/`: normative catalog, schema descriptors, primitive types
  and their tests/inventory.
- `KERNEL/uml/semantics/`: canonical UML identities, ownership, references,
  classifiers, properties, associations, operations, values and template rules.
- `KERNEL/uml/validation/`: structural-model and diagram conformance checks.
- `contracts/`: the single persisted document contract consumed by the kernel,
  application, persistence, projections and adapters. `semantic.rs` owns generic
  elements and relations; `presentation.rs` owns diagrams, Views, geometry,
  base style and diagram hierarchy placements. `uml_presentation.rs` defines
  UML-specific style preferences under `Style.uml`. `project.rs` and `section.rs`
  compose these records, without a second semantic or presentation model.
- `persistence/`: JSON/Protobuf encoding, decoding and wire representation.
- `application/commands/`: transactional command execution, authoring workflows,
  revision-sensitive identifiers and command validation. `application/commands.rs`
  composes the executor with the presentation services.
- `application/edits/`: property-edit contracts, Inspector mutations and Explorer
  placement. Editor-specific contracts are not normative UML validation rules.
- `application/diagram/`: the bounded geometry-query contract consumed by the
  command executor.
- `KERNEL/compiler/analysis/`: source-graph dependency analysis, deterministic
  scheduling and the intermediate generation-unit representation.
- `KERNEL/compiler/operations/`: typed transformation operations, port compatibility,
  dependencies and execution of the operation graph into an artifact plan.
- `KERNEL/compiler/artifacts/`: the output artifact IR, path derivation, collision
  detection and output-tree constraints.
- `KERNEL/compiler/emission/`: pure construction-script generation from artifact
  plans, without executing the generated program.
- `KERNEL/I_O/`: explicit filesystem effects: project materialization, staged
  publication, ownership-manifest reads/writes and output-tree inspection.
  Manifest encoding, decoding and path membership remain pure functions in
  `KERNEL/compiler/artifacts/ownership_manifest.rs`.
- `observability/`: external execution events and validation wrappers. Kernel
  calculations receive no trace and perform no observation clock reads.
- `view-model/`: renderer-independent projections grouped by `inspector/`,
  `hierarchy/`, `notation/`, `diagram/` and `authoring/`. These do not depend on GPUI.

GPUI rendering, controls, focus and entities belong to the workbench/UI crates,
not to this crate's kernel or view-model. Workbench draft state and snapshots
remain distinct from its native UI implementation.

## Similar names with different contexts

- `contracts/semantic.rs` defines stored elements and relations;
  `KERNEL/graph/graph.rs` validates the graph and `index.rs` queries it.
- `KERNEL/uml/semantics/associations.rs` defines Association contracts;
  `view-model/diagram/relationships.rs` computes diagram paths and anchors.
  Its `RelationshipGraph` is a diagram projection, not another semantic graph.
- `application/commands/quick_edit.rs` executes authoring actions;
  `view-model/authoring/quick_edit.rs` describes contextual controls.
- `KERNEL/uml/semantics/templates.rs` resolves and validates template identities;
  `view-model/notation/template_presentation.rs` formats their labels.
- `view-model/inspector/inspector.rs` projects fields and choices;
  `application/edits/property_edit.rs` provides shared edit contracts and
  `application/edits/inspector_edit.rs` applies a validated edit.
- `view-model/hierarchy/hierarchy.rs` produces read-only rows;
  `application/edits/hierarchy_placement.rs` changes document organization.
- `KERNEL/compiler/operations/operation_graph.rs` schedules typed generation operations;
  it is not the persisted UML relationship graph.

## Shared contracts and references

The generic semantic records are not a second UML metamodel. Normative UML
identities, ownership and reference rules remain in `KERNEL/uml/`; they constrain
the same elements and relations stored by the shared contracts. Presentation
links to those records through `View.subject` and `Diagram.model_elements`.
`HierarchyPlacement` locates diagrams; its optional `ViewOccurrence` identifies
a diagram/View and semantic member path, without replacing UML ownership.

The crate root reexports these exact types for existing consumers; there are no
copied DTOs or conversion layer between contracts and the application. The wire
representation remains a persistence encoding, not another authoring model.
This change separates contract definitions, not the aggregate JSON fields or
all existing kernel/presentation behavior. It does not claim complete UMLDI
interchange or a separately enforced Cargo dependency boundary. Consumers still
depend on `bellatrix_mde`; this is not yet a standalone, UML-independent contract
crate. `Style` holds generic colors, fonts and line styling; its explicit `uml`
namespace references `UmlStyle` for stereotype labels, compartment suppression
and member visibility/type/property display. These notation settings are not
semantic UML ownership. The extension is strongly typed and still part of the
same crate; it is not a dynamically extensible notation registry. Other UMLDI
fields on `Diagram` and the timing presentation contract remain to be separated
if fully notation-independent document contracts are required.

Views use one current JSON shape: endpoint identities belong to `endpoints`,
not `draft`. Unknown View, endpoint and draft fields are rejected with a decoding
error, including the former draft-local endpoint fields; they are not migrated
or silently discarded.

## StarUML presentation interchange

The MDJ adapter maps explicit `fillColor`, `lineColor` and `fontColor` to generic
`Style` colors when they use `#RRGGBB` or `#RRGGBBAA`. Complete `font` strings map
to family, size, normal/bold weight and italic. Underline, word wrap, shadow and
horizontal/vertical alignment also have bidirectional mappings, including nested
Views. Mapped values are removed from source residue and authored only once.

This encoding follows the supplied StarUML reference's
`resources/default/metamodel.json` and `src/core/graphics.js` (`Font.__read`,
`Font.__write` and Canvas alignment constants), under
`crates/bellatrix_design_system/docs/ref/staruml`. The adapter is an independent
implementation of those file encodings, not copied rendering or application code.

Missing values stay absent. Nulls, partial fonts (`?`), unknown encodings and
noncanonical font-number spelling stay in source residue with diagnostics.
The supported font-size range is 1–400; alternate numeric spelling, such as
`13.00` or `1.3e1`, is not normalized by unrelated edits. An explicit complete
font can be removed or edited; a partial typed font or unsupported weight is
rejected at export instead of supplying defaults. Residue conflicts are errors.
After an edit, export reimports its actual output and compares the entire
authored contract before returning bytes. Unedited replay retains original bytes.

This is an adapter/contract capability, not StarUML UI parity. The workbench
enables a restricted MDJ editing session only for verified clean UTF-8 source
bytes matching its native Buffer; other imports remain read-only. Its imported
renderer does not yet consume all typed style fields. `parentStyle` and computed appearance
remain unresolved source behavior. `SetViewStyle` reaches top-level Views only;
nested style is representable in the contract, without a matching authoring
command yet. Generic opacity, custom stroke width/dashes and other unmapped style
edits cannot be exported losslessly. Composition edits remain blocked.

`tests/features/mdj_presentation_lifecycle.feature` and
`tests/bdd_mdj_presentation_flows.rs` define integration flows for import, public
style commands, JSON/BLX disk persistence, MDJ export/reimport, source preservation
and rejection/repair. These flows do not establish native visual equivalence or
successful reopening in the StarUML application.

### Transactional MDJ adapter boundary

`bellatrix_mde_cordis::ModelingApplyMdjEdit` accepts the exact shared platform
snapshot and an existing model command. It computes a candidate without changing
the live document, verifies the MDJ inverse, then publishes through an atomic
snapshot comparison. Unsupported edits and stale or closed/reopened sessions
leave the shared snapshot and revision unchanged. Ordinary native authoring is
not restricted merely because a model carries MDJ provenance.

`ModelingExportMdj` returns verified bytes associated with that exact snapshot.
`ProjectPrepareEncodedSave` reserves an externally encoded write for the snapshot;
it does not write a file or mark a document clean. The adapter owns encoding.
`ProjectPublishNewFile` publishes complete reserved bytes at an absent destination
and returns a receipt without marking the document saved. `ProjectAcknowledgeExternalSave` clears dirty
state only for matching written bytes and the matching session/revision. Additional
platform engine sections are refused by MDJ interchange rather than omitted.

The native MDJ bridge routes enabled edits only through this verified boundary.
Its session retains the exact confirmed snapshot, counts queued and in-flight
commands as pending, and completes the active transaction before shutdown.
Generic native saves and validation dispatch are unavailable on that bridge.
Explicit native Save creates a new `.mdj` copy, then freezes the saved tab; reopen
that copy to continue. It does not replace the original source or claim general
in-place persistence. See `bellatrix_mde_workbench/README.md` for native lifecycle
gates and remaining limitations.

The new-file writer synchronizes staging contents and uses a same-directory hard
link to publish atomically only if no destination entry exists. Filesystems that
cannot create that link fail without an overwrite fallback. The directory is not
synced, so power-loss durability of the new directory entry is not guaranteed.
Failure to remove the staging link after publication is returned as a warning,
not reported as an unwritten file.

The existing Cordis BDD specification remains
`bellatrix_mde_cordis/tests/features/mdj_edit_save_lifecycle.feature`. Its published
integration harness covers file save/reopen, refused edits, stale sessions,
cancelled reservations and exact receipts. This native copy integration has only
been reviewed statically; it does not establish native GPUI interaction parity.

## Geometry boundary and remaining coupling

For `SetViewRouting`, the view-model receives an immutable document snapshot and
returns only path geometry and endpoint View identities. The application checks
the target, path shape and endpoint identities/diagram scope before applying the
patch to one View in the existing transaction. The geometry provider cannot
mutate the document. Other authoring commands still combine semantic operations
and diagram layout; their application location makes that composition explicit.

Two semantic modules still have presentation coupling to extract:
`KERNEL/uml/semantics/associations.rs` reroutes diagrams when binary projected
endpoints change, and `KERNEL/uml/semantics/root.rs` delegates display labels to
the presentation formatter. The source organization does not claim those edges
have been removed or that separate Cargo packages enforce the boundary.

The normative catalog remains generated by
`script/bellatrix_mde/bellatrix_uml_catalog.py`; its target and inventory paths
follow this layout. No handwritten catalog copy is introduced.

## Target backends in the compiler

The compiler is implemented across existing headless crates, with public APIs
kept stable rather than copying their implementations into this crate:

- `bellatrix_mde_rust_generator/src/KERNEL/compiler/`: Rust/Cargo planning,
  declaration lowering, physical file artifacts and target templates. Its public
  planner validates graph integrity, selects scope, analyzes dependencies,
  renders target artifacts and returns the shared `Plan`.
- `bellatrix_mde_templates/src/KERNEL/compiler/text/`: target-neutral Tera text
  emission from serialized contexts. It performs neither UML rules nor I/O.
- `bellatrix_mde_html_exporter`: an HTML/SVG documentation export adapter using
  `Plan` and notation projections. Its visual renderers are not moved into the
  UML/formal compiler kernel merely because they emit files.

The existing Rust planner does not prove complete UML 2.5.1 conformance or a
complete transformation from every valid UML diagram to an executable project.
Its graph-integrity check and bounded generation stages remain distinct from
full semantic acceptance and target-language compilation. The layout preserves
those implementation limits rather than claiming future compiler work exists.

`persistence/io.rs` is a memory codec despite its name and is not filesystem I/O.
Likewise, the Python source emitted by the compiler contains filesystem calls,
but composing that source does not execute them. Public publication functions
in `KERNEL/I_O/` document their actual effects and partial-failure limitations.

## Functional entry points

`validate_uml_model(&str) -> Result<String, String>` is implemented in
[`kernel-api/`](src/kernel-api/README.md) with a versioned JSON contract and no
external effects. It exposes no new internal types or public modules. Legacy
exports remain during migration; complete API isolation is not yet achieved.

`decode_model_document` and `reparent_uml_element` are implemented separately in
[`mde-api/`](src/mde-api/README.md). They use the same document contract and shared
versioned envelope without routing application operations through the kernel API.
The existing `uml`, `contracts`, `commands`, `mdj` and `mdj_conversion` public
modules and typed reexports remain available for consumers that have not migrated;
this first separation does not close those modules or claim full encapsulation.

UML style settings are serialized only under `style.uml`. Both `Style` and
`UmlStyle` reject unknown fields. Documents that put those settings directly
under `style` are rejected, rather than migrated or loaded with missing data.
An absent UML extension means default notation preferences; an empty extension
is omitted when saving. JSON and Protobuf use the same style decoding contract.
