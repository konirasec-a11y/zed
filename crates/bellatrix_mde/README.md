# MDE source responsibilities

`src/mde.rs` preserves the public Rust API through explicit module paths and
reexports. The directories describe responsibilities, not separate runtimes or
copies of the model. KERNEL includes UML specification semantics, graph
formalism and the mini-compiler implementation from model analysis to project
artifacts and emission; it is not limited to passive validation.

- `kernel-api/`: private function implementations for the public kernel boundary:
  validation, graph queries, generation analysis and standalone UML Class-to-Rust
  artifacts in memory.
- `mde-api/`: private function implementations for the public MDE document and
  application boundary: document codecs, MDJ interchange and semantic reparenting
  candidates.
  Both boundaries share the private `api_contract.rs` envelope/codec helper;
  neither depends on the other.
- `KERNEL/graph/`: graph integrity, indexing and graph-scope queries.
- `KERNEL/uml/metamodel/`: adapters over `bellatrix_mde_contracts::uml`, with
  engine-specific semantic reads and editing policy.
- `KERNEL/uml/semantics/`: canonical UML identities, ownership, references,
  classifiers, properties, associations, operations, values and template rules.
- `KERNEL/uml/validation/`: structural-model and diagram conformance checks.
- `bellatrix_mde_contracts`: the separate shared crate containing canonical model
  records, command inputs, read projection contracts and normative UML descriptors.
  Kernel, application, persistence and adapters use the same type definitions.
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

- `bellatrix_mde_contracts::model::semantic` defines stored elements and relations;
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
The contracts crate enforces a one-way Cargo dependency boundary without changing
aggregate JSON fields or existing kernel/presentation behavior. It does not claim
complete UMLDI interchange. Consumers still
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

MDJ export is **experimental** and remains available as an optional save-copy path.
It supports only the existing mappings and rejects unsupported edits; it does not
guarantee full StarUML compatibility or visual equivalence. Current development
prioritizes MDJ import into the universal contract and faithful visualization.

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

### Binary association aggregation

The MDJ adapter imports binary association ends with `none`, `shared` or
`composite` aggregation into the existing UML Association and Property contracts.
StarUML stores aggregation on the end beside the diamond; UML stores it on the
opposite typed Property. The adapter translates only this field between ends,
keeping IDs, roles, type references, multiplicities and navigation on their
original ends. Source field presence remains in provenance, including the
absence of aggregation versus an explicit `none`.

Canonical association Views use the existing UML endpoint convention, so the
same canvas draws the diamond on its original side. Opaque associations keep
the source convention. Experimental export applies the inverse translation and
reimports the actual bytes before accepting them. Aggregation is stored only on
its Property. Canvas, Explorer and Inspector derive diamond placement from those
Properties; `end1Aggregation` and `end2Aggregation` are editor command paths, not
persisted relation fields. Validation reports stale relation copies; JSON/BLX
codecs and the platform section boundary reject them when canonical `memberEnd`
is present instead of persisting a competing value or silently discarding it.

This does not translate `ownerAttribute`, qualifiers or unknown association/end
fields. Those objects and any classifier whose completeness depends on them
remain opaque. UML validation still reports unresolved types, multiplicity and
other semantic problems independently of import. Projection version 3 applies
on a fresh MDJ import; previously saved native documents are not silently
reinterpreted or migrated. Keep edited native files before reimporting source.

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

- `bellatrix_mde_rust_generator/src/KERNEL/compiler/`: pure Rust/Cargo rendering
  of prepared emission tasks with the existing target templates. Source graph
  selection and placement stay in the kernel projection. The standalone
  `generate_rust_struct` kernel API lowers one UML Class to those tasks without
  resolving a Cargo project or persisting a target model.
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

`decode_model_document`, `encode_model_document`, `convert_model_document`,
`import_mdj_document`, `export_mdj_document`, `reparent_uml_element` and
`project_documentation` are implemented separately in
[`mde-api/`](src/mde-api/README.md). They use the same document contract and shared
versioned envelope without routing application operations through the kernel API.
The Cordis adapter invokes these functions for integration/project events.
External UI, Zed, CLI and exporter consumers must use the event boundary, not a
second direct MDE route. Internal MDE calls remain direct, and MDE has no UI/Zed
dependency. Visual interaction state and events stay inside the UI.

`mdj` and `mdj_conversion` are private after consumer migration. The existing
`uml`, `contracts` and `commands` public modules and typed reexports remain
exposed while other consumers and event payloads migrate. That incomplete
encapsulation is not the intended external communication contract.

UML style settings are serialized only under `style.uml`. Both `Style` and
`UmlStyle` reject unknown fields. Documents that put those settings directly
under `style` are rejected, rather than migrated or loaded with missing data.
An absent UML extension means default notation preferences; an empty extension
is omitted when saving. JSON and Protobuf use the same style decoding contract.

## Imported classifier ownership

The MDJ adapter promotes Classes and Interfaces with their supported Attributes,
Operations and Parameters into the existing UML contract. Associations stored in
`ownedElements` are checked locally before their owner's ancestry, avoiding a
circular completeness dependency. Explicit `_parent` references, physical source
slots, member identities and order must agree; an unsupported or misplaced child
keeps the affected classifier and association opaque instead of hiding it.

Supported binary Associations keep both Property ends and their canonical
ownership. Omitted navigation remains unverified, not `notNavigable`. Empty
multiplicity, false uniqueness and public end visibility are source defaults in
the canonical state; original field absence is only encoding metadata. Reference
targets and multiplicity strings remain kernel inputs, not parser UML verdicts.

This covers the class-owned self-association and interface-owned association in
the supplied MDJ sample. Receptions, behaviors, qualifiers, `ownerAttribute`,
aggregation diamonds and other unmapped semantics remain outside this semantic
promotion. Their source data is retained. Interface ball/socket snapshot drawing
is a separate typed presentation projection described below; classifier relayout
remains pending. No new export mode is introduced.

## Bound member labels

`ViewPresentation.label_binding` distinguishes a subject-derived Attribute or
Operation label from literal `View.label` text. It records the role and an
explicit options source (defaults, another View, or unresolved), without storing
a second model or overwriting the source's cached text. JSON and the Protobuf
`presentation_json` field serialize the same typed binding. `Style.uml` also
contains `show_multiplicity` and `show_operation_signature`.

The MDJ adapter recognizes `UMLAttributeView` and `UMLOperationView`, resolving
their visual grandparent options independently of `parentStyle`. Supported
visibility/type/property/multiplicity/signature flags and stereotype `none`,
`label`, `icon` or `icon-label` have inverse mappings. This follows the supplied StarUML
`extensions/essential/uml/elements.js` member View updates and `getString`
methods, not the different quick-edit `textualNotation` expressions. Other
stereotype modes, malformed options and unsupported option-owner contracts
remain conservative fallbacks with diagnostics.

The MDE projection supplies separate rendered text and presentation diagnostics
through Cordis. It uses current names, ordered Parameters, authored textual
types/defaults, multiplicity notation and explicit property suffixes. Textual
types are still unresolved declarations, not proof of UML conformance. Authored
multiplicity is displayed literally, with no implicit `[1]`; if identity-based
bounds exist, the kernel query remains authoritative. Static underline and
abstract Operation italic use the kernel's effective scalar queries. No UML
acceptance rule is added to the UI or adapter.

Generic labels remain literal; their subject alone never implies a name,
namespace, property or association-end label role. Unresolved bound subjects or
options retain cached text with a visible diagnostic, not a claimed live label.
Stored geometry is not recalculated after edits, so compartment relayout,
automatic sizing, qualifier bindings and full StarUML rendering remain pending.

## Authored layout extents

`Geometry::Layout` preserves authored coordinates separately from usable rendered
bounds. Width and height retain missing/null intent or a typed `LayoutExtent`.
Resolved extents serialize as nonnegative finite JSON numbers. An unresolved
negative extent serializes as `{"original":-9.0,"reason":"negative_extent"}`;
its negative finite value remains explicit model data, not adapter residue or a
clamped zero. The same geometry JSON is carried by protobuf's `geometry_json`.
Bare negative resolved extents, invalid reason/value combinations and nonfinite
values are rejected. Native Rect/Ellipse geometry retains its size invariants.

StarUML compartment arrangement can assign child width as compartment width
minus padding. A saved negative result is not treated as a special `-9` sentinel.
The MDJ adapter imports it as an unresolved extent and exports the original value;
no-edit export still returns the original source bytes. Unresolved bounds do not
become renderer coordinates: the imported preview and interface icon projection
require resolved extents, and the inspector reports the retained original value.
This allows importing the complete project without claiming font measurement or
compartment relayout parity. Resolving those pending bounds remains future work.

## Imported interface icons

`ViewPresentation.interface_icon` binds an interface icon to its label-compartment
View, while `interface_connection` records a connection's visual required/provided
role. The endpoint identity still lives only in `View.endpoints`. The adapter
recognizes MDJ Interface, Dependency and InterfaceRealization Views; it does not
derive those roles from semantic UML relationships. The name-compartment reference
is mapped out of source residue and has an inverse mapping. Both bindings use the
same JSON/Protobuf presentation contract as other View presentation fields.

Only explicit `icon`/`icon-label` style selects the icon preview. An absent
stereotype-display field follows the source metamodel's `label` load default,
not the Interface constructor's creation preference. The MDE projection recovers
a square icon from the stored boundary between the Interface and its visible
name compartment. Missing, nonfinite or inconsistent bounds produce a diagnostic,
not a fixed-size icon or substitute rectangle.

For a coherent arranged snapshot, zero incoming dependency Views produce a ball;
dependencies produce one socket arc per connection; dependencies plus interface
realization Views add an inset ball. Socket orientation follows the reference's
rectangle junction, floored center and half-circle sweep, using the stored final
path point. These are drawing rules, not a UML validity verdict. Cordis transports
only projected circle/arc primitives and diagnostics to the renderer, which adds
no synthetic stem and does not inspect MDJ fields.

The reference implementation recalculates text metrics and edge layout before
drawing. This preview does not: it reports its use of stored compartment bounds
and endpoints. Shadow placement, endpoint rerouting, dynamic label visibility,
canonical/decoration Interface silhouettes and full pixel parity remain pending.
Changing icon modes preserves the stored compartment layout. Unknown connection
targets or directions do not become fabricated socket orientations.

## Imported entity-relationship presentation

ERD models remain opaque source objects, not UML Classes or Associations. Typed
`entity_box`, `relationship_notation`, `relationship_endpoints` and
`relationship_labels` bindings express their drawing roles. Names and cardinality
remain in their identified model objects; projections never persist a second copy.
The entity name label uses the bound entity name and bold font. Relationship and
end labels use their current bound names, with name-label visibility derived from
whether the relationship name is empty. Existing bounds and label text are retained
for inverse export and unresolved fallbacks, not rewritten by rendering.

The supported empty-column entity presentation is a rectangle without dividers.
Populated column compartments report an unsupported-preview diagnostic rather than
inventing measured column widths. Crowfoot markers cover `1`, `1..*`, `0..1` and
`0..*`, using the reference's bar/diagonal/circle geometry. Omitted cardinality
uses the notation's documented `1` default; unknown values produce diagnostics,
not guessed UML multiplicity. `identifying` controls solid versus `[3]` dotted
strokes, with the documented omission default `true`. Stored paths stay manual.

`ViewPresentation.default_style` is a typed fallback layer, separate from authored
`View.style`. The adapter maps proved missing-field defaults for colors, label
alignment and supported shadow behavior; null or unsupported source encodings do
not acquire a fallback. Inheritance includes the source View's defaults. These
values do not cause absent fields to be written to MDJ, and the global export
reimport/equivalence check rejects presentation changes without an inverse.
Cordis sends resolved paint values, including a neutral shadow description, to UI.
The renderer does not inspect source format/type strings to select ERD notation.

Empty-column ERD entities use the source's 7-pixel, 20%-opaque light-gray shadow.
Imported labels use a line height equal to the projected font size, avoiding the
GUI framework's larger default line height within stored single-line bounds.
Font availability, dynamic relayout, column rendering and pixel parity still need
runtime verification; this implementation is not a claim of complete ERD editing.
