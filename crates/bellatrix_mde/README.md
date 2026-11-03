# MDE source boundaries

`src/mde.rs` is the public facade. Its explicit module paths preserve the existing
Rust API while placing implementations beside their actual responsibilities.
This is a source organization boundary, not a new runtime, model copy, or crate.

- `src/KERNEL/` owns the persisted model and wire format, semantic UML contracts,
  graph queries, validation, commands, generation, publication, and execution
  traces. `KERNEL/inspector_edit.rs` applies validated Inspector edits.
- `src/view-model/` owns renderer-independent projections and authoring intent:
  Inspector descriptors, hierarchy rows, labels, notation, geometry, relationship
  routing, Quick Edit definitions, and the visual-component inventory. It has no
  GPUI dependency and does not own a second persisted model.
- GPUI views remain in the workbench and `bellatrix_ui_*` crates. In the workbench,
  `src/view-model/` contains only draft state, snapshots, and document guards;
  rendering, focus, widgets, and GPUI entities remain outside that folder.

## Similar names with different contexts

- `KERNEL/model/graph.rs` defines persisted elements and relations;
  `KERNEL/graph.rs` operates on the graph. Neither is a diagram renderer.
- `KERNEL/uml/` implements metamodel and semantic contracts;
  `view-model/uml_notation.rs` and `uml_geometry.rs` project their visual form.
- `KERNEL/commands/quick_edit.rs` executes edits;
  `view-model/quick_edit.rs` describes contextual controls and available actions.
- `KERNEL/uml/associations.rs` owns Association semantics;
  `view-model/relationships.rs` projects diagram paths, anchors, and routing.
  Its `RelationshipGraph` is a diagram projection, not another semantic graph.
- `view-model/hierarchy.rs` creates read-only model rows;
  the workbench's `hierarchy.rs` builds the native UI tree from those projections.
- `KERNEL/templates.rs` owns template identities and validation;
  `view-model/template_presentation.rs` formats their labels.
- `view-model/inspector.rs` exposes fields and choices;
  `KERNEL/inspector_edit.rs` validates and applies the chosen change.

Commands still consume shared authoring/geometry projections, and Inspector
editing consumes field descriptors. The facade preserves those existing
cross-module dependencies; folder placement does not claim a fully one-way
kernel/view-model dependency graph. Persisted `View`, geometry, and style types
remain in `KERNEL/model/diagram.rs` because they are part of the document format,
while layout and notation algorithms live in `view-model/`.

The generated metamodel and inventory retain their generator:
`script/bellatrix_mde/bellatrix_uml_catalog.py`. Moving the source does not make a
handwritten copy of the normative catalog.
