# UML canvas notation corrections

Reference: OMG UML 2.5.1, formal/2017-12-05. PDF pages below are one-based file pages, not printed page numbers. These changes concern presentation, not executable behavior.

- §11.5.4, printed 202–203 / PDF 244–245: relationship endpoint marker bodies now extend inward along the relationship at either end. Source arrows point at the source; target aggregation diamonds no longer protrude into the target classifier. Shared `paint_marker` in `bellatrix_ui_uml/src/relationship.rs` handles both endpoints.
- §9.8.4, printed 128 / PDF 170: `instance_presentation.rs` resolves classifier identities for the underlined `instance : Classifier` heading and canonical Slot defining-feature/value references for compartment rows. `figures.rs` separates un-underlined slot rows from the heading. Stored literal values are displayed without evaluation. Unresolved data is marked rather than invented. Slot authoring/evaluation remains bounded by the Kernel capabilities.
- §17.6.4.3, printed 585 / PDF 627: CombinedFragment operator comes from `interactionOperator` and appears in the frame descriptor, rather than replacing it with the fragment name. An absent operator uses the official XMI default `seq` (UML.xmi `CombinedFragment-interactionOperator-_defaultValue`).
- §17.2.4.1, printed 568 / PDF 610: Interaction frame names appear alongside `sd`, rather than at the center of the body. InteractionUse retains `ref` with its reference label in the body.

Still incomplete: semantic AssociationClass binding (a View-only attachment is available below); event-aligned guard positioning; timing data-driven traces; semantic Message/MessageEnd creation and numbered communication labels. SVG currently emits basic paths without UML endpoint markers, so native/export visual parity is not claimed. Explicit line-style overrides can deliberately depart from normative UML strokes; the editor warns about this rather than removing user control. Native GUI acceptance remains pending; headless feature flows are not visual screenshot acceptance.

## Presentation freedom and structural follow-up

- §12.2.4, printed248/PDF290: package labels move to the tab when other views are visibly nested. This is a spatial presentation decision, not a restriction or an inferred semantic ownership change.
- §11.2.4, printed185/PDF227: StructuredClassifier can display its internal-structure compartment below member compartments. Composite roles use solid borders, noncomposite roles use dashed borders through the same shared style resolver. New Part quick-create stores composite aggregation; existing model values are not rewritten.
- View-only drafts are persisted separately from semantic subjects. Native placement first attempts the supported semantic command and preserves rejected placement as a visible draft with its diagnostic. An explicit pencil toggle chooses visual-only creation directly. Drafts reuse native figures, View styles, move/resize, and view-ID relationship endpoints. Drafts are not executable UML and are not included as fake semantic elements in code generation. Draft labels are edited by double-click or F2 through a View-only command; promotion remains pending. Selecting a draft identifies its status.
- Validation mode selection is separate from placement. Configurable validation does not make unsupported Kernel contracts implemented, and a headless flow does not establish native visual acceptance.

- §12.2.4 / §12.3.4: semantic and draft relationships share keyword/name projection, including ElementImport «import»/«access» and strict ProfileApplication «strict»/«apply». AssociationClass no longer receives an invented «associationClass» keyword; its semantic association binding remains incomplete; the visual attachment tool is described below.

## Additional direct placement tools

Port and Part are now direct toolbox entries, alongside DeploymentSpecification (§19.2.4, printed654/PDF696), CollaborationUse (§11.7.4), DestructionOccurrenceSpecification and Continuation (§17.8 notation tables). They reuse existing square, role box, classifier, dashed ellipse, X and rounded-rectangle silhouettes. The deployment specification heading uses the optional official «deployment spec» keyword. Unsupported semantic ownership remains a persisted visual draft, not a reason to prohibit placement. Independent Port, deep-history H*, and junction glyphs correct the toolbox identities without copying proprietary StarUML artwork.

## Frame descriptors, guards and association-class attachment

- §17.2.4.1 / §17.6.4.3: frame pentagons now size to shaped descriptor text using the active font, rather than a fixed fraction of the frame. Long Interaction names are clipped within the descriptor instead of overflowing its border; frame keywords inherit text styling.
- §17.6.4.1–2, printed585/PDF627: InteractionOperand guard text is projected from canonical guard/specification references or ownership. Stored opaque-expression bodies and literal values appear in brackets; an absent guard stays elided (the normative assumed value is true). Unresolvable references are visibly marked. No Boolean evaluation or event-position semantics is claimed.
- §11.5.4, printed202–203/PDF244–245: the Association-class attachment toolbox connector creates a dashed, arrowless View-only link between an association path and a classifier box. It can be used in either click order and does not invent an AssociationClass semantic binding. Endpoint routing uses a point on the actual middle line/cubic segment, updates dependent visual links after their source paths, and preserves visual cycles without rejecting layout. Automatic semantic association-class binding remains pending.

## Sequence message presentation

- §17.4.4.1, printed576–577/PDF618–619: synchronous, asynchronous and reply toolbox connectors use the common relationship style projection for filled, open and dashed-open arrows. These tools explicitly create View-only messages, not semantic MessageEnds or executable calls.
- Pointer locations on lifelines become persisted normalized visual anchors. Moving/resizing a lifeline keeps messages attached; dragging a message changes its drawn event heights. Two clicks on the same lifeline produce a three-segment self-message. These are authored presentation coordinates, not occurrence ordering or execution validation.
- Communication needs its own numbered labels and small arrows near those labels, rather than the sequence endpoint arrows. That presentation, found/lost endpoints, and timing-specific messages remain pending.
