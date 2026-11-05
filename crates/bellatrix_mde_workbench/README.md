# Bellatrix MDE Workbench

`bellatrix_mde_workbench` contributes a native `workspace::Item` and Zed Dock `Panel` implementations to the application in this fork. Zed owns the editor, Workspace, GPUI lifecycle, focus, layout, themes and status bar. Bellatrix supplies model commands and transient Canvas interaction.

## Source boundaries

- `src/lib.rs` is the public facade; existing public imports remain available.
- `src/UI/workbench.rs` owns the GPUI item, gestures, entities, focus and rendering.
  The other `src/UI/` modules own widgets, canvases, panels, native document and
  Workspace integration, and the lifecycle-owned source adapter. Panel children
  live in `src/UI/panels/`.
- `src/view-model/` contains renderer-independent inspector/style drafts,
  snapshots and document guards. These files do not depend on GPUI. The UI
  composes them through explicit module paths; they do not own another model.
- Persisted data and commands remain in `bellatrix_mde`, with its `KERNEL/` and
  `view-model/` source boundaries. The workbench is the native UI adapter, not
  another kernel.

The private `workbench_ui` module keeps widget implementation and state together, while
root reexports preserve the crate API. The pure view-model files still belong
to this GPUI-dependent crate; this layout does not provide a separately
compilable view-model package or claim that every MDE dependency is one-way.

## Integration

The built-in `bellatrix_zed_host` is installed directly in `crates/zed/src/main.rs` after `initialize_workspace(app_state.clone(), cx)`. No separate GPUI runtime, third-party overlay or `scripts/enable_zed_desktop.py` is required.

The host owns the Cordis Project/MDE plugins and a `MdeUiBridge`. Every Workspace has a visible **MDE** text button in the native status bar by default. Click it to open the MDE Workbench and its panels, or invoke `bellatrix_mde::OpenMdeWorkbench` from the command palette. Repeating the action focuses the existing tab; reopening a closed tab reuses its Item without attaching duplicate panels. Registering the button does not open a tab or change focus in an ordinary repository.

Setting `BELLATRIX_PROJECT_FILE` or `BELLATRIX_MODEL_FILE` before launching explicitly selects a startup source and automatically opens its MDE Workbench. With neither variable set, the button opens the host's existing demo source. It does **not** automatically load the JSON file selected in the native editor. There is no empty-workspace or restored-session auto-open heuristic. To hide only this status-bar entry, set `BELLATRIX_MDE_STATUS_BUTTON=false` before launch; the default is visible, and the command-palette action remains available. This Bellatrix-specific startup option does not change native editor/status-bar settings.

The adjacent **Open project** dropdown offers **Open MDE project (read-only preview)**. It uses the native file dialog, Project/worktree and shared text Buffer, validates the selected Bellatrix document, and renders its captured model through the existing Workbench. The preview has its own diagram chooser and pure validation control; selection and pan/zoom remain available. It does not enable model editing or native MDE save. Changes to the source text, file identity, dirty/conflict state or imported semantic snapshot mark the preview stale without overwriting source text. Close and reopen after resolving source changes. A document ID already owned by a configured/demo source is rejected explicitly, without substituting the fixture or cloning its ID.

The native Explorer, Inspector, diagram list and diagnostics follow the active MDE document. Preview panels show that captured model and canonical read-only properties/styles, without edit controls. Switching back restores the configured Workbench's selection and unsent property/style drafts. Ordinary Editor tabs clear the MDE panel context and block stale editing callbacks. Panel bindings are weak and do not keep a closed preview session alive. The primary **MDE** button intentionally returns to the configured/demo Workbench. General native in-place persistence and session restoration remain unfinished; the restricted MDJ copy flow is described below.

The bridge relays typed Cordis commands and observes model revisions. The interim host still supplies one configured model or demo fixture; per-document `ProjectPath`/Buffer integration, native save acknowledgement and `SerializableItem` restoration remain incomplete. The visible-entry recovery does not claim those later integration gates passed.

Canvas, Inspector, Explorer and Toolbox rendering retain the last complete Cordis
projection while the next revision is projected asynchronously. This read-only
presentation must belong to the same open document session; closing or replacing
that session invalidates it. Commands continue to require a projection matching
the current document, so retaining a frame never makes an outdated edit current.
Superseded projection results are discarded before publication rather than
temporarily replacing the visible model with a stale-projection error.

Implemented operation paths include Class/Interface creation, rename, view move/resize, project save and a drag-and-drop Toolbox. These use the MDE graph validation and versioned `bellatrix_project` service, rather than writing model data in the GUI.

## Limited native MDJ editing and copy export

The same open dialog admits a StarUML file for restricted editing only when the
shared Buffer is clean and writable and raw UTF-8 disk bytes match its normalized
text. Provenance uses those raw bytes (including CRLF), not reconstructed Buffer
text. If that proof is unavailable, the existing read-only preview remains.
Bellatrix JSON previews remain read-only.

MDJ edits use the session's last confirmed snapshot, never a fresh project-ID
lookup. Supported commands are names, labels, mapped properties and top-level
styles; unsupported gestures and commands do not mutate the imported model.
Native MDJ validation is not enabled: its control is disabled rather than
dispatching a project-ID-only validation against another possible session.
Inherited/composite styles and rendering remain partial. This is not full StarUML
parity or native in-place persistence.

Explicit Save opens a new-name copy dialog; Save As and autosave cannot open that
flow. An unchanged document is a no-op. Export, reservation, atomic new-file
publication and exact acknowledgement must complete before reporting a saved
copy. Existing files and symlinks are never replaced. Source bytes and Buffer
state are checked around I/O. The destination is compared with dirty text Buffers using canonical parent
paths, including nonexistent filenames; unrelated dirty Buffers are allowed. No source Buffer is edited by saving.

After confirmed publication, the tab is frozen and shows the copy's path; it no
longer reports the original as its active project path. Close is clean only while
the saved snapshot and source state still match and no command is pending. Reopen
the copy to continue. Publication/acknowledgement conflicts retain dirty state and
an explicit recovery message. A published copy is reported even if acknowledgement
fails. The tab is not silently rebound to another Buffer. Cleanup first drains the
bridge and waits for any in-flight copy save, then closes only its last confirmed
snapshot. Explicit discard closes the projected session without modifying the
original Buffer; the ended tab must be closed and the source reopened to resume.
A save completing after discard cannot reactivate the tab.

## UI conventions

Follow the Zed Workspace and Editor UX. Use existing Zed UI components and docks. Do not implement a second Dock manager, another theme system or a parallel project tree. Bellatrix presentation components in `crates/bellatrix_ui_*` provide MDE-specific symbols and diagrams without owning backend state.

## Validation

```powershell
cargo check -p bellatrix_mde_workbench -p bellatrix_zed_host
cargo test -p bellatrix_mde_cordis
cargo clippy -p bellatrix_mde_workbench --all-targets -- -D warnings
cargo check -p zed
cargo run -p zed
```

The native code must still pass Windows build and interaction tests before the new Workbench is considered fully validated.

Prefer a few feature-level behavior scenarios that exercise complete flows across
public module boundaries, rather than a test for each helper or method. Assert
observable results and preserved state so internal refactors do not require
rewriting the scenarios. The focused `property_receipts` scenario covers editing,
no-op acknowledgement, stale-revision rejection and independent targets through
the public Cordis bridge. Extend a relevant flow for a concrete regression;
do not grow a large suite of overlapping implementation tests.

## Existing-design frontend integration

The visual source is `../bellatrix_design_system/docs/design/src/components/panels/`,
with reference captures in that package's `preview/` directory. Its React controls
are layout specimens, not permission to introduce unsupported model operations.

The native workbench uses compact rows of native Zed controls and
`bellatrix_ui_panels::PaneHeader` for panel disclosure/actions. Inspector editing
is event-driven: typing, toggling a boolean or choosing an option queues the
captured property command immediately. There is no Apply or Cancel step.
The existing presentation state keeps only transient input, validation and
receipt state keyed by project, element and property; the Cordis project model
remains authoritative.

Properties and styles share a single in-flight command boundary. While a command
is pending, newer input is coalesced; its receipt releases the next valid edit
against the current revision. The input stays mounted so typing and cursor focus
are preserved. Canonical projection events do not submit another edit. Numeric
intermediate input and domain rejections remain visible inline without corrupting
the model. **Updating…** distinguishes queued input from acknowledged model state.

Clean fields follow model updates. A conflicting canonical change preserves the
local edit and offers **Reload model value** rather than silently overwriting the
other change. Changing selection or document cannot redirect a captured command;
queued input is revalidated against its own target before submission. Read-only
previews show canonical values without write controls. ID and type stay read-only.
**Name** is the canonical diagram name; **Label metadata (not diagram name)**
remains stored and editable under Advanced. Documentation and less-common
properties stay reachable in their sections; opening the Inspector does not
mutate the model.

The toolbox presents supported quick tools, searchable categories and existing
symbol glyphs. Collapse state belongs to the panel, not the semantic model.
Zed still owns dock widths and persisted placement; this increment does not
reset a user's saved workspace layout or introduce a separate docking system.

Native acceptance remains required for this increment:

- Compare light/dark and compact/wide windows with the existing design captures.
- Collapse and reopen every panel section; retain selection and split sizes.
- Search tools, clear the query, select a tool and create through the canvas.
- Select classes, members and relations from the canvas and Model Explorer.
- Type continuously, toggle booleans and choose enums; observe live canonical updates, queued feedback and retained cursor focus.
- Open Documentation/Advanced and ensure all schema properties are reachable.
- Confirm full figure bounds, context menus, resize and toolbar zoom still work.

Pure-helper tests, Cargo metadata and source checks do not establish native GPUI
compilation or successful interaction. Keep that distinction in release reports.

### View styles and routing

Select a concrete canvas View to edit Styles. Fill, stroke, text color, font,
font size, stroke width and opacity use the existing View style contract; blank
fields inherit the theme. Valid edits update the model immediately through tracked
Cordis commands, serialized with semantic property edits. Rapid typing is coalesced
while a receipt is pending; later text and focus are retained. Incomplete colors
and invalid numbers stay visible with field errors and do not enter the model.
Concurrent changes to the same edited field preserve the draft and require explicit
reload instead of silently overwriting a newer style. Theme restoration is an explicit
action; its receipt clears only text that has not changed since the reset request. Selecting only a semantic model element does
not choose an arbitrary representation to modify.

Relationship Views offer Straight, Automatic and Curve routing. Curve authors a
cubic path and moving an endpoint reanchors that representation without changing
other Views of the same semantic relation. Orthogonal/rounded routing and direct
control-point dragging remain pending. Straight self-relations retain a visible
loop. UML arrowheads and relationship dashing remain derived from semantics.

Documentation uses the native multiline editor; semantic properties still use
the existing Inspector command boundary. These controls require the native
acceptance scenarios above, independently of headless command tests.

The Model Explorer lists each diagram once, with its persisted view occurrences
beneath it. A single resolved UMLDI context can place a diagram beneath its
unambiguous external element occurrence. Missing, ambiguous or cyclic contexts
remain at the project root. Only the designated root_model container is omitted from the tree
and its children are promoted; explicit Model elements stay visible and renamable; model data remains unchanged. When a semantic subject has a diagram occurrence,
its standalone row is omitted; distinct subjects with equal names and legitimate
occurrences in different diagrams remain distinct. Items without a diagram view
remain accessible through semantic ownership. Unpictured members of a pictured
owner remain accessible under each owner occurrence, with diagram-scoped row
identities, instead of recreating the owner above the diagrams. Diagram/view
identity is retained for selection and presentation editing, including in
read-only previews.
Relationship rows show source, authored relationship name (or relationship type),
relationship type, semantic endpoint markers and line style, and destination. Projection does not delete elements,
change ownership, or modify any diagram or view. Explicit diagram placements are read from
the shared hierarchy contract; semantic elements keep their domain ownership.

Explorer navigation follows the StarUML interaction contract for selecting a
right-clicked target before opening its menu, revealing selected nodes through
collapsed ancestors, and expanding or collapsing the tree. These navigation
controls are icon-only, have tooltips and accessible names, and remain available
in read-only previews. Reveal can be requested again for an unchanged selection.
Disclosure clicks change expansion without selecting the row. This is a bounded
parity step: keyboard tree navigation, element inline rename and full StarUML
menu coverage are not claimed as implemented.

Dragging an existing semantic element from the Explorer onto the active canvas
reuses PlaceElementView at the canvas coordinates. An existing view in that
diagram is selected and revealed at the drop point by panning, instead of duplicated; the source element and its owner are
unchanged. Drags are bound to the source project, model revision and active target
diagram, and stale/cross-project/read-only drops are rejected. View-only drafts
and relationships are not offered for placement by this element-only command.
Internal tree drops onto semantic element rows validate the proposed owner with
the kernel's SetOwner/reparent command, then send it through Cordis with the
captured revision. They work without an active diagram and preserve the active
diagram, semantic identity and existing views. Read-only, stale, cross-project,
self, unchanged-owner and unsupported/cyclic moves are rejected. Diagrams,
relationships, visual-only drafts and the project row are not ownership drop
targets or relocation sources. A visual occurrence represents its semantic
subject here: changing ownership affects that subject everywhere, not only the
dragged occurrence. Explicit visual containment is not rewritten, and cosmetic
hierarchy placements never override semantic ownership. This is a semantic move, not sibling reordering
or diagram nesting.

Explorer quick search filters incrementally by displayed text, type and identity,
retaining ancestors and the contents of matching containers. Search expansion is
temporary and does not overwrite the unfiltered collapse state; clear or close
search to restore it. Reveal Selection clears the filter and opens the selected
item's path. Added/Name ordering only sorts siblings, never changes ownership or
IDs, and keeps operation parameters in their semantic order. Added uses persisted
collection order rather than inventing creation timestamps for imported models.
Search and ordering are local panel state and remain usable in read-only previews.

Project context menus offer diagram kinds that require no UML context, without
inferring a context from an unrelated selected element. Kinds requiring a specific
model context are not offered at the project root. Diagram context menus select the target and offer Open, Rename
and Delete. Rename and delete use a compact draft/confirmation row with icon-only
controls, tooltips and accessible labels. Delete explicitly warns that diagram
views are removed while model elements remain. Commands retain the menu's project,
diagram identity and revision and pass through Cordis CAS; stale, read-only or
suspended operations are rejected. Switching documents clears pending drafts and
restores focus. Project rename/delete and nested-diagram creation are not offered
without a supported semantic command.

Element context menus offer contextual diagrams only when domain metaclass
conformance accepts the exact semantic subject: Activity, StateMachine,
Interaction diagram variants, and CompositeStructure for a verified
StructuredClassifier. InstanceSpecification composite contexts remain unavailable
in this menu until its eligibility check consumes the domain classifier evidence. The captured
subject ID becomes the single UMLDI model context; no semantic owner, visual
occurrence parent or cosmetic placement is changed. The same revision guard and
Cordis CreateDiagram command are used. The existing projection nests the diagram
under an unambiguous external placement of its context; multiple placements remain
at the project level rather than inventing ownership or duplicating the diagram.

Semantic owner context menus expose Attribute, Operation, Reception and Parameter
creation according to the same UmlType capability checks used by AddMember.
Actions capture the actual owner ID and menu revision, submit AddMember through
Cordis, and expand the clicked owner occurrence after successful enqueue. They
work without an active diagram. The kernel assigns fresh identities, ownership
and unique default names, and may grow classifier views to fit new compartments.
No canvas coordinates, duplicate semantic elements, cosmetic parents or invented
container rules are used. New-member selection/rename on commit is not yet wired;
queue acceptance is not reported as completed publication.

Element context menus now rename existing semantic elements through the Rename
command, preserving identity and ownership. The editor starts with the semantic
name (including the parametered element for template parameters), not a rendered
relationship label or visual occurrence caption. Enter commits and Escape cancels
only while this name editor has focus; neither key creates content or confirms
diagram deletion. Blank names stay in the editor with feedback. The same captured
project/revision guard, read-only checks and document-switch cleanup apply.
Explorer creation/placement routes use AddMember, CreateDiagram and PlaceElementView;
they do not call semantic-to-visual-draft fallback commands.

Explorer creation uses separate Add Diagram and Add submenus. Add on the project,
Model or Package creates supported semantic elements, including Model, without
requiring an open diagram or adding a canvas view. Classifier members remain
contextual Add entries. Add Diagram on a Model or Package stores its Explorer parent separately from
UMLDI model_elements. Creation and placement are one revision-checked mutation;
behavior diagrams still require their supported UMLDI contexts. Commands
pass through Cordis and the kernel with the captured project revision. Rename
changes the semantic name; Explorer occurrences show that name even when their
canvas view has a separately authored label. Projection and menu capabilities
live in view-model/explorer.rs, while panels and hierarchy render the controls.

When Add Diagram is invoked on a visual occurrence or an unpictured descendant,
`hierarchy[diagram_id]` stores `parent_id` and an optional `occurrence` referencing
an existing View through `diagram_id`, `view_id`, and the ordered `member_path`.
An empty path identifies the View's subject. This is a shared model contract,
not Explorer UI state. The Explorer retains its own row keys and expansion state.

This first hierarchy slice covers authored diagram containment. It does not
replace domain ownership of semantic elements or visual containment within a
canvas. Semantic ownership commands remain the only way to move elements.
`Diagram.model_elements` remains notation-specific context, never an implicit
hierarchy parent. Newly created diagrams record an explicit root placement unless
the caller chooses another parent. Files without a placement display diagrams at
the project root while retaining their diagram context unchanged.

JSON, protobuf, binary envelopes, Project and ModelingSection preserve the same
placement and View reference. JSON rejects duplicate hierarchy keys. Protobuf uses
typed HierarchyPlacement and ViewOccurrence messages, not Explorer JSON blobs.
Opening and saving preserve pending parent and member references, even when UML
ownership is incomplete or inconsistent. Neither path rewrites those references
or consults UML conformance rules. New placement operations require an identifiable
destination; the occurrence graph remains cycle-checked for safe navigation.
Semantic and unresolved-reference diagnostics belong to explicit kernel validation.
Deleting a semantic parent moves its dependent diagrams to the project root.


The retired `explorer_parents` and `explorer_diagram_occurrences` payloads are
rejected on import and require explicit conversion. They are never silently
ignored, migrated, or written alongside the new contract.

Explorer element and diagram menus expose validated Move into destinations.
Element movement uses SetOwner, while diagram movement uses the shared hierarchy
placement and leaves notation context and Views unchanged. Dropping a diagram
onto an existing View or its semantic member records that exact ViewOccurrence,
using the same validation as Add Diagram. Menu destinations choose semantic
parents instead; the complete semantic ancestor path is retained when needed
to display an authored diagram placement without guessing between multiple Views. Diagram rows have a
separate GPUI drag payload so dropping a diagram cannot create an element on the
canvas. Element and diagram drops onto Model/Package rows use the same revision
checks and kernel validation as menu actions; stale or invalid moves report an
error instead of changing the document.

A pictured element with an explicit semantic owner remains visible beneath that
owner (for example, NewModel), while its existing eye-marked View occurrences
remain beneath their diagrams. These are projections of the same subject, not
new semantic elements or duplicated Views. Pictured root-level subjects still
avoid redundant standalone rows unless an authored diagram needs that parent.
Moving an occurrence onto an Explorer owner
moves its semantic subject; it does not move its View between diagrams. Tooltips
and menu labels distinguish those operations.

Relationship creation requests carry an explicit optional `owner_id` through
Cordis and the command layer, including the visual-draft fallback. The executor
never infers ownership from UMLDI context, adopts a root Model, or requires matching
Activity/Region containers before recording a relation. It stores a requested
owner without testing UML conformance; the kernel diagnoses incomplete or invalid
ownership. Source-encoded ownership is not duplicated as an `owns` edge.

The canvas supplies the source identity only for source-owned relationship tools,
including Deployment's location. Other tools currently submit no owner because
the canvas has no explicit owner picker. The relation remains editable and
incomplete; completing that owner-selection interaction is still pending.

## Imported member labels

The imported-layout preview draws `UMLAttributeView` and `UMLOperationView` as
selectable labels at their stored bounds. Text and typography come from the
Cordis read projection's typed subject binding, not from MDJ interpretation in
the UI. Model edits refresh the rendered member label while the source text is
preserved for interchange. Generic labels continue to display literal text.
The View Inspector distinguishes stored and rendered labels and shows fallback
diagnostics when a binding cannot resolve. The style editor exposes member
multiplicity and operation-signature visibility through existing style events.

This is still a bounded imported preview: it does not relayout compartments,
resize labels automatically, enable nested geometry gestures or establish
pixel-for-pixel StarUML parity. No runtime/build verification is implied.

Imported Interface icon Views use projected ball/socket primitives from Cordis.
The UI does not derive provided/required UML semantics, inspect source view kinds
or replace an unresolved icon with a class rectangle. Existing child labels stay
at their stored positions. Icon/icon-label style controls preserve that layout;
the preview and Inspector report missing geometry and the absence of font/edge
relayout. The generic native interface figure remains separate from this imported
snapshot renderer, which does not add the native figure's fixed stem.

## Imported ERD canvas

The canvas consumes typed entity-box and crowfoot projections through Cordis.
It draws empty-column entities without dividers, current bound names, preserved
paths and endpoint markers. Paint defaults and the ERD shadow are resolved before
reaching UI; imported labels have an explicit font-sized line height. Unknown
cardinality, unresolved bindings or populated column layouts remain explicit
preview limitations. ERD objects are not silently converted into UML elements.
Canvas background/grid preferences and full font/layout parity are separate from
these persisted drawing roles and have not been established by this slice.
