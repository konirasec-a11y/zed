# Behavioral acceptance fixtures

Each `*.valid.json` / `*.invalid.json` is a standalone Bellatrix document with
one UMLDI diagram, a designated Model root, local references, canonical ownership,
semantic relationships and attached edge views. Load the document, then wrap it
in the version-1 `validate_uml_diagram` request. `cases.json` supplies the requested
kind, diagram ID and expected diagnostic witnesses; warnings are not success
certificates and must remain visible in reports.

- **Use case:** customer specialization, payment actor, system subject, four use
  cases, two includes, an extension point and extend, and two associations with
  materialized Property ends, derived association links and synchronized role
  projections. Named relationships remain distinguishable in their namespaces.
  The negative case empties
  `discount-extension.extensionLocation`, violating `Extend::extensionLocation`
  multiplicity 1..* (`UML_PROPERTY_CONTRACT`).
- **Activity:** initial/final nodes, decision/merge, fork/join, five opaque actions,
  warehouse partition, typed input/output pins, object flow, control flows and
  owned Boolean guards with explicitly absent visibility because their edge
  owners are not Namespaces. The negative case changes the authored `flow-1.target`
  reference to the Class `order` instead of an ActivityNode
  (`UML_PROPERTY_CONTRACT`). All IDs still resolve; this is not a missing-ID test.
- **State machine:** initial, ordinary and final vertices, nested composite state
  with its own Region, cancellation branch, and seven explicitly contained
  named transitions. The negative case moves `complete` to originate at `done` and end
  at `pending`, updating semantic endpoint properties and View attachments.
  `FinalState-no_outgoing_transitions` must report
  `UML_FINAL_STATE_NO_OUTGOING_TRANSITIONS` for `done`.

Sources: the pinned `docs/bellatrix/standards/uml/20161101/UML.xmi` and UMLDI
catalog, `src/model/{graph,diagram,project}.rs`, and
`src/KERNEL/uml/{metamodel,semantics,validation}`. Names identify fixture content,
not runtime action implementations. The Activity's Boolean guards select the
stock-available path deterministically; no expression evaluator is implied.

Coverage is intentionally `implemented_model_and_diagram_rules_only`. These
fixtures do not demonstrate execution of actions, token scheduling, event
processing, use-case execution, behavioral code generation, or every normative
UML constraint. Include/Extend and activity-edge endpoint properties are authored
explicitly because their complete canonical role adapters are not implied by
binary `from`/`to` fields. They must not be replaced by visual drafts to hide
unsupported rules. Baselines exercise the supported representation, not a claim
that every feature of each UML family is exhaustively covered.

Activity incidence regressions additionally change canonical endpoints and their
explicit properties together, keeping diagram attachments synchronized. Seven
negative lifecycle scenarios cover InitialNode incoming/control edges, FinalNode
outgoing edges (including ActivityFinalNode inheritance), ForkNode incoming count,
JoinNode/MergeNode outgoing count and DecisionNode incoming count. The existing
Activity positive scenario witnesses their permitted cardinalities. The rules
read semantic ActivityEdge source/target, not stored inverse caches or drawings.
Normative bodies: UML.xmi InitialNode-no_incoming_edges/control_edges,
FinalNode-no_outgoing_edges, ForkNode-one_incoming_edge,
JoinNode-one_outgoing_edge, MergeNode-one_outgoing_edge and
DecisionNode-incoming_outgoing_edges. Other Activity constraints remain outside
this increment; these scenarios do not claim exhaustive Activity conformance.

The additional positive presentation round-trip scenario retains decimal
Rect/Ellipse/Polygon/Path coordinates, a cubic segment, authored LayoutBounds
values alongside null and missing fields, and an arbitrary-precision decimal in
presentation properties. This checks document persistence without giving those
presentation values semantic meaning in UML validation.

Two further incidence flows retain a malformed ObjectFlow source and an external
ControlFlow source. A malformed endpoint must report its edge property contract,
not invent a FinalNode/InitialNode violation. The external ControlFlow may leave
cardinality unresolved, but must not claim its already known ControlFlow type is
unverified. The manifest's forbidden diagnostic witnesses enforce this distinction.

The dedicated `bdd_activity_edge_flows` lifecycle harness covers
`ControlFlow-object_nodes` from the pinned UML.xmi: both ObjectNode endpoints,
including InputPin/OutputPin subtypes, require `isControlType=true` (default false).
It exercises relation and object-record representations, independent endpoint
repairs, malformed Booleans, contradictory endpoint properties, unresolved hrefs,
and unknown metaclasses through JSON/BLX save-close-reopen flows. Malformed scalar
or endpoint data retains its own property-contract errors; it does not falsely
prove `ControlFlow-object_nodes` violated. Unresolved or contradictory endpoints
leave that rule unverified, while an independently known violating endpoint fails.

The same Activity lifecycle harness also covers pinned `ForkNode-edges` and
`MergeNode-edges`: the set union of incoming and outgoing edges must consist
entirely of ControlFlows or entirely of ObjectFlows, using subtype conformance.
An empty union satisfies this predicate; cardinality constraints remain separate.
Each edge identity is considered once, including a self-loop. Known mixed families
prove a violation despite unrelated unresolved candidates. A known endpoint can
establish incidence despite an unresolved opposite endpoint, while ambiguous
incidence alone cannot prove a mixture. Possible edges already known to share the
established family do not invalidate homogeneity. JoinNode and DecisionNode have
different normative predicates and are not folded into this rule.

`JoinNode-incoming_object_flow` is checked as its own conditional: any incoming
ObjectFlow requires ObjectFlow outputs; otherwise outputs must be ControlFlows.
Mixed input families are permitted. Incidence is directional, and unresolved
input incidence/families never become proof of ObjectFlow absence. A known
ObjectFlow witness selects the object branch despite other unknown inputs. If
both output-family branches have the same proved outcome, that result is
independent of possible completions of the unknown guard, not an evaluation of
an OCL null/invalid condition. Outgoing cardinality is checked separately.

DecisionNode-edges follows the pinned UML.xmi Set union and excludes only the
resolved decisionInputFlow identity, including outgoing/self-loop identities.
An absent or null optional selector excludes nothing. An unresolved or invalid
selector leaves a mixed union unverified; a homogeneous union is safe under any
removal. Property-contract diagnostics remain separate. This slice does not
implement decision_input_flow_incoming and does not discount the selected edge
from the existing incoming/outgoing cardinality checks. The lifecycle flow saves
and reopens every edited state as JSON and BLX before restoring the baseline.

ObjectFlow-is_multicast_or_is_multireceive forbids both distribution flags being
true. Effective Boolean defaults come from UML.xmi and are never materialized
into the document. Malformed flags retain property-value errors; the rule stays
unverified unless a known false independently disproves the conjunction. The
lifecycle covers both canonical relations and object records, all Boolean
combinations, either unknown operand, JSON/BLX reopen and baseline repair.

ObjectFlow-selection_behavior checks that a present selection Behavior has an
ObjectNode source, including Pin subclasses. Absent/null optional selection
makes the implication vacuous. A well-formed external selection reference proves presence without requiring
the external Behavior definition; its type remains separately unverified.
Malformed references stay unknown, while an independently proved ObjectNode
source satisfies the consequent.
Property-contract errors remain separate. Parameter-count/type and side-effect
rules of the selected Behavior are outside this slice. Both edge representations
are exercised through JSON/BLX save, reopen and repair.

ActivityParameterNode-no_edges requires at least one of incoming/outgoing to be
empty. Direction is derived from ActivityEdge target/source, with known edges in
both directions proving a violation even alongside unknown edges. Unresolved
incidence or family remains unverified; a self-loop proves both directions.
Parameter type and direction constraints are separate rules outside this slice.
The lifecycle covers relation/object edges, JSON/BLX reopen and baseline repair.

ActivityParameterNode-no_incoming_edges and no_outgoing_edges enforce the
Parameter direction only when edges exist exclusively in the corresponding
opposite direction. Direction uses the normative enum/default, without writing
it into data. Invalid scalars and unresolved Parameter references remain separate
from rule violations. Empty/bidirectional guards are false, and an allowed
direction proves the implication despite uncertain incidence. The lifecycle
covers every enum value, default in, malformed/external values, uncertain guards,
JSON/BLX persistence and repair. Ownership and type equality remain separate.

ActivityParameterNode-same_type compares exact resolved type identities, not
subtype compatibility. Absent/null optional TypedElement types both mean no type;
one typed and one untyped is a violation. External or invalid references remain
unverified with their separate contract findings. A missing required Parameter
cannot establish equality even when the node is untyped. The lifecycle includes
generalization-backed subtype mismatches, nullable/unknown types and JSON/BLX
reopen and repair. It does not infer ownership or resolve external definitions.

ObjectNode-object_flow_edges requires every incident edge to be an ObjectFlow
when effective isControlType is false. The union uses endpoint identity and
includes subclasses, with self-loops once. Known non-ObjectFlow incidence proves
a violation; unknown families/endpoints remain unverified and possible known
ObjectFlows cannot invalidate the universal. Malformed flags have separate
scalar errors, and a proven consequent satisfies the implication independently.
The lifecycle covers CentralBufferNode, both edge representations, default and
malformed flags, incoming/outgoing witnesses, JSON/BLX reopen and repair.

ObjectNode-selection_behavior implements the equivalence between selection
presence and ordering=ordered, including ObjectNode subclasses. Ordering defaults
to normative FIFO without being materialized. Well-formed external selection
references establish presence; unresolved local or malformed references and
invalid ordering leave equality unverified with separate contract findings.
This is not a one-way implication and does not validate Behavior parameters.
The OutputPin lifecycle covers all ordering literals, both selection states,
external/malformed values, JSON/BLX reopen and repair.

Pin-control_pins requires isControlType when isControl is true; it does not impose
the reverse implication or depend on incident edge kinds. Effective defaults
come from UML.xmi and malformed flags remain separate scalar errors. A known
false antecedent or true consequent proves the implication despite the other
unknown flag. The lifecycle exercises InputPin/OutputPin, every Boolean pair,
null/malformed flags, implicit defaults, JSON/BLX reopen and baseline repair.

Pin-not_unique requires isUnique=false even though the inherited normative
MultiplicityElement default is true. The Activity corpus now authors false
explicitly on its two Pins: this corrects previously under-validated fixtures,
not the metamodel default or expected error manifests. The lifecycle removes
that property to prove absence still violates the rule without data mutation,
then checks true/false, null/malformed values, JSON/BLX reopening and explicit
repair. Invalid scalars retain PROPERTY_VALUE and do not fabricate rule errors.

ActivityParameterNode-has_parameters checks direct canonical Activity containment
and Behavior::ownedParameter membership. It reuses ordered parameter evidence;
a known local witness survives unrelated external entries, while contradictory
projections or incomplete ownership remain unverified. A Parameter known to
belong to another Activity or Operation proves exclusion when the Activity's
parameter collection is complete. No ancestor containingActivity or visual tree
is substituted for ActivityNode::activity. The lifecycle includes references,
projection membership/order conflicts, JSON/BLX reopening and repair.

ObjectFlow-no_executable_nodes excludes ExecutableNode subclasses at either
endpoint, not merely Action's direct metaclass. Endpoint contracts and ambiguous
representations remain separate; a known executable endpoint proves violation
even when the opposite endpoint is unknown. The lifecycle covers both binary
relations and object records, source/target witnesses, malformed and unresolved
endpoints, JSON/BLX reopen and repair.
