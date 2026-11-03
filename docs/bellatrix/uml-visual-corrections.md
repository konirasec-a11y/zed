# UML canvas notation corrections

Reference: OMG UML 2.5.1, formal/2017-12-05. PDF pages below are one-based file pages, not printed page numbers. These changes concern presentation, not executable behavior.

- §11.5.4, printed 202–203 / PDF 244–245: relationship endpoint marker bodies now extend inward along the relationship at either end. Source arrows point at the source; target aggregation diamonds no longer protrude into the target classifier. Shared `paint_marker` in `bellatrix_ui_uml/src/relationship.rs` handles both endpoints.
- §9.8.4, printed 128 / PDF 170: `instance_presentation.rs` resolves classifier identities for the underlined `instance : Classifier` heading and canonical Slot defining-feature/value references for compartment rows. `figures.rs` separates un-underlined slot rows from the heading. Stored literal values are displayed without evaluation. Unresolved data is marked rather than invented. Slot authoring/evaluation remains bounded by the Kernel capabilities.
- §17.6.4.3, printed 585 / PDF 627: CombinedFragment operator comes from `interactionOperator` and appears in the frame descriptor, rather than replacing it with the fragment name. An absent operator uses the official XMI default `seq` (UML.xmi `CombinedFragment-interactionOperator-_defaultValue`).
- §17.2.4.1, printed 568 / PDF 610: Interaction frame names appear alongside `sd`, rather than at the center of the body. InteractionUse retains `ref` with its reference label in the body.

Still incomplete: semantic AssociationClass binding (a View-only attachment is available below); event-aligned guard positioning; timing event semantics and MessageLabel shorthand; semantic Message/MessageEnd creation and multiple messages grouped on one communication link. SVG currently emits basic paths without UML endpoint markers, so native/export visual parity is not claimed. Explicit line-style overrides can deliberately depart from normative UML strokes; the editor warns about this rather than removing user control. Native GUI acceptance remains pending; headless feature flows are not visual screenshot acceptance.

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
- Semantic timing-event attachment remains pending.

## Communication message presentation

- §17.4.4.1 and §17.9.1, Table17.4, printed600/PDF642: Communication messages render an arrowless link with a separate small open directional arrow beside the label, instead of a sequence arrowhead at the target object. Canonical and View-only message paths use the same renderer mode and shared styles.
- New Communication visual messages begin with the editable label `1: message()`. This is a presentation placeholder, not a computed semantic sequence. The user can author nested sequence expressions through the existing label editor. Endpoint routing clips at object heads; sequence event-height anchors are not applied in this diagram context.
- Grouping multiple directional messages on a single shared link and independent label placement remain pending. No sequence-expression execution or ordering validation is claimed.

## Structural relationship toolbox coverage

- §12.2.4/12.3.4: ElementImport, ProfileApplication and Extension are now direct toolbox choices. Extension has a solid line and filled triangle toward the metaclass, with an independently drawn matching icon. ElementImport uses the existing canonical namespace-import adapter; ProfileApplication and Extension remain explicitly View-only while their authoring adapters are incomplete.
- §11.2.4 and §7.7.4: Connector and comment/constraint links are directly placeable using existing solid/dashed arrowless geometry. They are explicitly unverified visual links, not untyped semantic records. Unsupported endpoint arrangements remain drawable. ConnectorEnd authoring, ExtensionEnd binding, required-extension labels and profile reference distinctions remain pending.

## Authored Timing Lifeline presentation

- §17.11.1 Table17.6 and Figures17.28–30, printed604–606/PDF646–648: optional typed `View.timing` stores the chosen state/condition or general-value form and authored time/value samples. The native renderer builds stepped state traces or crossing value ribbons from those samples along a linear time axis. Empty data shows only name and axis; the former fixed example waveform is not used for timing Lifelines.
- A compact inspector provides state/ribbon icon choices and editable time/value rows. Edits use revision-tracked View-only commands and the existing draft/conflict state, with pending text preserved on rejection and explicit reload/replace choices. This is presentation data, not evaluated states, occurrence ordering, or executable UML.
- New Timing Lifeline views default to480×180; Communication views to140×80. Existing authored dimensions are preserved. Native message attachment in Timing preserves both selected coordinates instead of forcing the sequence stem center. Event binding, constraints/units, MessageLabel shorthand, point dragging, and export parity remain pending. No native GUI acceptance is claimed.

## Diagram-context frame headings

- AnnexA/B, printed684/688: when an Activity or StateMachine View represents the declared context of its matching diagram, it renders as an `act` or `stm` frame, not an ordinary action/state node. Non-context Activity/StateMachine representations retain their existing shapes; no blanket metaclass-to-frame conversion is made.
- Generic Frame, contextual act/stm frame, sd frame, ref use and fragment operator now share the same measured pentagonal outline path. Keyword width uses the active font; generic names no longer have a hardcoded142px tab. Frame kind is bold. This replaces the unused separate native frame-heading painter without changing semantic records.

## GeneralOrdering presentation (UML 2.5.1 §17.5.4.4, Figure 17.10; Annex B.4.2 printed700/PDF742)

The interaction toolbox creates a persisted, explicitly visual-only `uml.general_ordering` View. The official notation uses a dotted line and a filled directional arrow within the line, not at its endpoint. `relationship_notation.rs` now supplies shared `LineMode` defaults and a middle marker; `relationship.rs` places that marker on the authored path with the local line/cubic tangent. The existing occurrence-position anchors preserve the selected positions, including two positions on one Lifeline, without creating OccurrenceSpecifications or evaluating before/after semantics. Explicit View line-style overrides remain available and can intentionally differ from UML notation. SVG uses the same default stroke resolver for canonical and visual-only relationships; native marker/timing geometry export remains pending.

The bundled StarUML reference was checked under `crates/bellatrix_design_system/docs/ref/staruml` (UML extension and vector-icon sources); no GeneralOrdering-specific identifier was found. Its new toolbox SVG is an independent rendering of the official dotted-line/middle-arrow notation, not copied proprietary artwork.

## Provided and required Interface forms

UML 2.5.1 §10.4.4, Figures 10.8–10.10, and §11.3.4/Figure 11.11 distinguish a provided Interface circle from a required Interface semicircle, each labeled and attached by a solid stem. `interface_presentation.rs` renders these opt-in forms through the common ViewAppearance and existing editable title. The toolbox creates persisted `uml.interface` visual Views with `interface_presentation: provided|required`; they appear in the shared Explorer inventory, can be moved, resized, styled, labeled and connected using the common View endpoints. The stem faces left; authors can connect it to their classifier/port. Creation does not invent InterfaceRealization or Usage semantics. Existing canonical expanded Interface rendering and members are unchanged. Glyphs are independently authored after checking the bundled StarUML ball/socket source and official figures. Native GUI execution remains unverified; rendered SVG asset inspection is not native acceptance.

## Found and lost Message presentations

UML 2.5.1 §17.4.4 and Annex B.4.2 specify a small filled circle at the unknown starting end (found) or receiving end (lost). The two interaction tools now create one persisted message View: found uses an empty-canvas source followed by a receiving View; lost uses a sending View followed by an empty-canvas target. The circle is an edge marker, never a fabricated UML element or additional View. Table17.2 (printed598/PDF640) retains the lost-message arrowhead at the circle boundary; the renderer preserves the open or filled arrow selected by messageSort rather than replacing it with a plain line. The shared partial-endpoint router moves the attached end with its exact View while retaining the authored free point and intermediate geometry; a missing referenced ID never becomes a free endpoint or redirects. `messageKind` here is explicit View-only notation metadata; its canonical UML property is derived/read-only, and no semantic Message or OccurrenceSpecification is created. Common styles, labels and Explorer inventory apply. The native marker remains an explicit SVG export gap.
