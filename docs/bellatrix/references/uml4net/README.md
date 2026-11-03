# Kernel graph references

Bellatrix adapts query logic and selected fixtures from
[STARIONGROUP/uml4net](https://github.com/STARIONGROUP/uml4net/tree/8ef13af2c68ae1994c8fa4f5b3e231dee7e6715d),
commit `8ef13af2c68ae1994c8fa4f5b3e231dee7e6715d`:

- `uml4net/Extend/AssociationExtensions.cs`: all-member, deduplicated `endType`.
- `uml4net/Extend/PropertyExtensions.cs`: own `aggregation` determines
  `isComposite`; only binary membership has a unique `opposite`.
- `uml4net/Extend/OperationExtensions.cs` and
  `uml4net.Tests/Extend/OperationExtensionsTestFixture.cs`: return-property
  derivation and the no-return/default scenarios, adapted into the existing
  Cordis Class Quick Edit feature flow.
- `uml4net.Tests/Extend/AssociationExtensionsTestFixture.cs`: mixed ownership
  and self-association fixtures.
- `uml4net.Tests/Extend/PropertyExtensionsTestFixture.cs`: binary and n-ary
  opposites, duplicate-end and reparenting scenarios.

Copyright 2019–2026 Starion Group S.A. The adjacent `LICENSE` and `NOTICE` are
copied unchanged from that source. Bellatrix's modified Rust implementation is
`crates/bellatrix_mde/src/KERNEL/uml/associations.rs`; adapted tests are in
`crates/bellatrix_mde/tests/association_kernel.rs`.
The upstream code and tests were inspected, not executed.

## Authoritative contracts

The pinned official `docs/bellatrix/standards/uml/20161101/UML.xmi` and UML
2.5.1 §§9.9.17 and 11.8.1 govern behavior, rather than upstream implementation
quirks. Executable checks cover `Association-memberEnd` ordered uniqueness and
minimum 2, `Association-ends_must_be_typed`, `Association-association_ends`,
`Association-binary_associations`, `Property-type_of_opposite_end`,
`Property-multiplicity_of_composite`, and ownership/subset consistency.

`memberEnd` is an ordered array of local references to canonical Property
records. `owns` edges alone determine `ownedEnd` and classifier ownership.
`Property.association`, `owningAssociation`, and `opposite` are derived, not
independently stored lists. The shared immutable `ReferenceIndex` resolves
identities and generated descriptors define reference types.

The existing binary Relation remains the Association identity. Its `from`,
`to`, role, multiplicity and diamond fields are compatibility projections,
refreshed atomically by supported mutations and checked against the canonical
Properties. `end1Aggregation` marks the source-side diamond and therefore
projects the **target-typed** Property's aggregation. A composite Property's
own multiplicity may be many; its opposite upper bound must be at most one.

New binary associations acquire real ends during command application. Explicit
migration and legacy end-field edits materialize only the touched association;
read/reopen and unrelated edits do not rewrite old authored data. Role names,
verbatim multiplicity notation, Association identity and diagram Views survive.
Invalid literal intervals reject atomically. Unevaluated notation remains
explicitly unsupported and cannot establish valid composite multiplicity.

Pure Association objects can represent n-ary membership and queries return no
single opposite. The existing binary Relation adapter refuses more than two
ends as its implementation limit, not a UML prohibition. N-ary diagram edge
rendering, AssociationClass ownership-slot differentiation, navigability,
qualifiers and specialized Association semantics remain outside this slice.
Literal ValueSpecification bounds and Property redefinition consistency have
subsequent bounded implementations documented below; unevaluated expressions
remain unsupported.

## Literal ValueSpecifications and identity bounds

`uml/value_specifications.rs` is a pure in-memory evaluator using the shared
ReferenceIndex and generated scalar/default contracts. It evaluates
LiteralInteger, LiteralUnlimitedNatural, LiteralBoolean, LiteralString and
LiteralNull identities. Integer and UnlimitedNatural retain arbitrary precision;
`*` is not replaced by a machine maximum. Missing Integer/UnlimitedNatural values
use their generated zero defaults, Boolean uses false, and an absent optional
LiteralString value remains absent rather than becoming an empty string.

The authoritative sources are UML 2.5.1 §7.8.8 and §§8.6.8–8.6.14, plus XMI
`MultiplicityElement-lower_is_integer`, `upper_is_unlimitedNatural`,
`lower_ge_0`, `upper_ge_lower`, and each literal's value query/default.
`uml4net/Extend/MultiplicityElementExtensions.cs` and generated LiteralInteger /
LiteralUnlimitedNatural definitions were consulted, not copied. Its tolerant
cross-conversions between Integer and UnlimitedNatural are deliberately not
adopted: the semantic `integerValue()` / `unlimitedValue()` result must satisfy
the official bound predicate. A future evaluator can return the same typed
values from other ValueSpecifications without restricting valid UML to literals.

Property and Parameter multiplicities resolve lowerValue/upperValue local
references and verify the canonical owns relation. A reference identifies the
bound role; it does not create another owner list. A missing bound defaults to
one independently of the other bound. Old multiplicity notation remains an
adapter when identity bounds are absent; once bound references exist, it is
preserved as read-only authored data and cannot override evaluated bounds.
Binary Association compatibility projections track changes to bound literals.
Operation return derivation and Property redefinition checks reuse this same
resolver and the existing interval-containment mathematics.

Malformed/dangling local references, invalid scalar representations, wrong
known semantic value types and invalid intervals produce Invalid diagnostics.
External references, opaque/other unevaluated expressions and unsupported
literal categories (including Real) produce Unsupported diagnostics, not false
claims of invalid UML. Unassigned owned ValueSpecification roles remain
unsupported. Computation performs no filesystem/network access and does not
mutate the graph. Editing policy for semantic models and permissive visual
drafts remains outside the evaluator. A literal-creation UI, arbitrary
expression interpreter and general ValueSpecification execution are not added.

The existing public Association flow imports identity-bound Property/Parameter
records, edits an unbounded-size literal through commands, checks projections,
retains old notation, rejects invalid edits, distinguishes unknown expressions,
and reopens JSON/Protobuf. No new test function is introduced.

## Derived Operation return properties

The pinned XMI `Operation-isOrdered.1`, `Operation-isUnique.1`,
`Operation-lower.1` and `Operation-upper.1`, also specified by UML 2.5.1
§9.9.11.6, derive all four fields from the single canonical return Parameter.
Without a return Parameter, the results are false, true, null and null.
With a return Parameter, ordering/uniqueness use its generated scalar defaults;
missing multiplicity notation is 1..1. The existing shared BigInt Integer and
UnlimitedNatural interval adapter evaluates literal notation without a machine
integer limit. Property and Parameter use that same adapter. Canonical literal ValueSpecifications now resolve through the shared evaluator
described above. Unevaluated expressions remain explicitly unsupported and
never masquerade as absent/default bounds; no general expression interpreter is
claimed.

`uml/operations.rs::return_properties` and the read-only Inspector projection
ignore historical stored Operation return fields, while preserving and showing
those fields separately as authored data. Mutations edit the Parameter instead.
`Operation-only_body_for_query` is enforced for populated bodyCondition, whose
reference must resolve to an owned Constraint. Expression truth is not evaluated
and remains an explicit capability warning. The upstream `FirstOrDefault`
behavior is not copied: multiple return Parameters remain invalid.

The existing public Cordis Quick Edit flow adds Parameters, edits their Boolean
flags and bounds, rejects invalid intervals and writes to derived fields, keeps
unknown expressions visible, reopens JSON/Protobuf, and deletes the return
Parameter to recover the no-return defaults. It also exercises an imported body
Constraint and rejects changing its Operation to non-query. No new isolated
test functions are introduced.

## Atomic Association-end editing

`commands/association_ends.rs` adds `Command::EditAssociationEnds`. One ordered
batch can insert a new Property, attach an existing Property, move an end,
change its Type or owner, and remove an end. Replacement combines removal and
insertion without publishing an intermediate one-end Association. The final
state uses the existing generated collection contracts, ownership helpers,
Association validators and binary projections.

Removal explicitly retains a classifier-owned Property, reparents and retains
it, or deletes an Association-owned Property through the existing owned-subtree
deletion helper. Unrequested Property identities, authored metadata and other
classifier attributes survive. A Property cannot be silently taken from another
Association. N-ary Association objects can gain and remove ends; binary Relation
notation still refuses a third end as an adapter limitation.

The Cordis `ModelingEditAssociationEnds` event delegates the complete batch to
the existing revision-checked mutation path, yielding one project replacement.
Failure leaves the input snapshot unchanged. Snapshot-based undo/redo retains
exact identities and metadata; this command does not introduce separate history
storage. The existing public Association feature flow covers replacement,
retention, coordinated Type/owner edits, removal, ordering and JSON/Protobuf
reopen without adding isolated unit tests.

## Namespace and Classifier queries

`crates/bellatrix_mde/src/KERNEL/uml/classifiers.rs` adapts the derived-union and
inheritance query patterns in the same pinned source's
`uml4net/Extend/NamespaceExtensions.cs` and
`uml4net/Extend/ClassifierExtensions.cs`. The parent/public/private and
owned-redefinition fixtures in
`uml4net.Tests/Extend/NamespaceExtensionsTestFixture.cs` and
`uml4net.Tests/Extend/ClassifierExtensionsTestFixture.cs` inform the existing
`crates/bellatrix_mde/tests/inherited_capabilities.rs` integration flow. The
copyright and Apache-2.0 notices above apply to these adaptations too.

The original OMG XMI, rather than the upstream's recursive mutable lists,
controls this slice: `Namespace-ownedMember`, `Namespace-member`,
`Classifier-attribute`, `Classifier-feature`, `Classifier-inheritedMember.1`,
`Classifier-inherit`, `Classifier-hasVisibilityOf`, `Classifier-allFeatures`,
`Classifier-allAttributes`, `Package-visibleMembers`, and the Property and
Operation redefinition reference properties. See UML 2.5.1 §§7.8.10, 9.9.4.6–7,
9.9.15, 9.9.17 and 12.4.5.

Owned members, direct features and direct attributes are projected from
canonical `owns` edges using generated composite-property subsetting and
redefinition contracts. An owned NamedElement is not automatically an owned
member: `Classifier::collaborationUse`, for example, does not contribute. The existing
NamedElement namespace and qualified-name queries use that same subset rule.
`Operation::ownedParameter` retains the subset constraints of the property it
redefines. Association ends contribute members and direct features when owned,
but do not become Classifier attributes. Association membership uses the same
canonical `memberEnd` adapter described above.

`Classifier::feature` and `attribute` are direct-only. `allFeatures()` selects
Features from the complete supported member set; `allAttributes()` returns
owned attributes before inherited ones, with more-specific ancestor attributes
before more-general ancestor attributes. A topological traversal preserves that
order even in diamonds. Identity-based sets remove repeated inheritance paths.
An explicit traversal stack detects generalization cycles without recursive
calls. All query state is local to a borrowed immutable model snapshot, using
the existing `ReferenceIndex` as the identity authority. No derived members,
ancestry lists or inverse references are persisted or globally cached.

Inheritance excludes private members. Protected, package and absent visibility
follow the pinned `hasVisibilityOf` body rather than a programming-language
visibility rule. Public package members are separately projected through
`Package::visibleMembers()`. Property and Operation redefinition lists contain
local references and filter inherited candidates according to the *direct*
`Classifier::inherit` contract at each inheritance step. A diamond may therefore
retain an original member supplied by a second parent until the receiving
classifier explicitly redefines it; no extra global suppression rule is added.

`Command::SetRedefinedElements` checks reference shape, target metaclass,
uniqueness, self-redefinition, `isLeaf`, and the supported ancestor/owned-feature
context before committing. Property and Operation signature consistency now use
the bounded comparisons described below. Behavioral/template contract
conformance remains unevaluated. Unsupported Type/bound or contract evaluation retains an explicit
`UML_REDEFINITION_CONSISTENCY_UNSUPPORTED` capability warning; successful
membership computation alone does not certify full redefinition semantics. The
query results are consumed by model/command validation and read-only derived
rows in the existing Inspector. The integration flow creates a diamond and
members through commands, changes visibility, redefines and clears
redefinitions, rejects malformed targets and cycles atomically, and reopens
JSON and Protobuf before querying again.

Explicit collaboration roles and other unimplemented membership sources do
not produce a partial successful member result. A query returns an explicit
unsupported-capability error when their data appears, including in an ancestor;
model diagnostics retain this as a capability warning rather than a UML
prohibition. The Inspector shows the unavailable-query reason. Dependent
redefinition commands still require a complete member query. Independently
stored derived member/feature/attribute arrays are not an authoritative
alternate graph.

## Namespace and Package imports

`crates/bellatrix_mde/src/KERNEL/uml/namespace_imports.rs` adapts import and name
patterns from the pinned `NamespaceExtensions.cs`, `PackageExtensions.cs`,
`ElementImportExtensions.cs`, and `NamedElementExtensions.cs`. The retained
Apache-2.0 attribution applies. The finite graph solver is new; it does not
reuse the upstream thread-local recursion guard or mutable result lists.

The authoritative contracts are `Namespace-importedMember.1`,
`Namespace-getNamesOfMember`, `Namespace-importMembers`,
`Namespace-excludeCollisions`, `NamedElement-isDistinguishableFrom`,
`ElementImport-getName`, `Package-visibleMembers`, and `Package-makesVisible`,
plus import target, visibility and ownership constraints in the original XMI.
See UML 2.5.1 §§7.4.3.3, 7.8.7, 7.8.9–11 and 12.4.5.

Canonical `uml.element_import` and `uml.package_import` relationship records
hold source and target identities. Their source derives `importingNamespace`
and their owner; no inverse ownership or member arrays are added.
`ElementImport` may store an alias. Both imports store public/private visibility,
with the generated public default. Imported identities remain in their original
owner and retain their original qualified name. Import names belong to the
receiving Namespace, including multiple aliases for one identity.

The query resolves public and absent-visibility package members, transitive
public re-exports and private imports. An explicit ElementImport overrides
PackageImport for the same identity, including its aliases and restricted
re-export visibility, as stated in §7.4.3.3. Indistinguishable imported identities
are excluded together; owned members take precedence over imported names.
Collision checks use metaclass conformance and names, not string-only equality.

Two literal OCL defects are corrected against the accompanying normative prose:
`excludeCollisions` must compare distinct identities, and `makesVisible` must
check public import visibility rather than compare an element to an enumeration
literal or test whether a collected Boolean list is nonempty. Private owned
members are not exported. These corrections avoid silently exposing private
members or excluding every import as a collision with itself.

Strongly connected components separate cyclic package imports from their
external dependencies. A collision-free component reaches a finite monotone
fixed point over identities, names and public visibility; no recursive stack or
arbitrary traversal limit is used. Collisions within such a component require
non-monotone resolution and remain explicitly unsupported. These graphs can be
stored and reopened, with a capability warning; no partial member result or
false UML-invalid diagnosis is returned. Non-cyclic import collisions are
resolved completely. Import objects or independently stored import-reference
arrays remain unsupported alternate representations, not second authorities.

`Command::SetNamespaceImport` atomically adds or updates an import, checks its
known target/visibility/ownership constraints and preserves unrelated records.
The existing Namespace query, structural diagnostics and read-only Inspector
consume the results and imported names. The existing hierarchy feature flow
also creates packages and imports, changes aliases and visibility, resolves
collisions, traverses a cycle, repairs an unsupported cyclic collision, and
reopens JSON and Protobuf. No isolated import tests or alternate diagram UI are
introduced.


## Classifier ancestry and inherited Slots

`uml/classifier_hierarchy.rs` adapts the pinned internal
`uml4net/Extend/ClassifierExtensions.cs::QueryAllGeneralClassifiers` and public
`uml4net.Extensions/ClassifierExtensions.cs::QueryConformsTo` intent. Their
ancestor-chain fixtures inform the existing Slot ownership flow. The two
upstream helpers disagree on self inclusion and the public traversal recurses;
Bellatrix follows `Classifier-allParents` and `Classifier-conformsTo` in the
original XMI instead. The retained Apache-2.0 notices apply.

User-model conformance compares canonical Classifier identities and their
Generalization closure. It is separate from immutable metaclass conformance:
two unrelated Class objects both conform to the metaclass Class without one
user-model Classifier conforming to the other. `allParents` excludes self;
`conformsTo` includes identity equality. Iterative active-path detection reports
cycles as the known UML `no_cycles_in_generalization` violation. Diamonds use
identity deduplication and a topological ordering. Namespace inheritance and
redefinition context checks reuse this same traversal, rather than keeping a
second ancestry implementation. Results belong to one immutable graph snapshot.

`Classifier-allSlottableFeatures` combines StructuralFeature members with all
ancestor attributes, including private attributes, and applies the direct
`Classifier-inherit` redefinition filter. It is not identical to the visible
member or allAttributes sets. Instance Slot assignment now consumes that result
instead of rejecting every generalized Classifier. Association member ends can
contribute through the existing canonical member query.

The literal allSlottableFeatures OCL can reintroduce a feature redefined by an
intermediate ancestor, while its description says redefined features are
excluded. When that ambiguity is present and the receiving Classifier has not
explicitly resolved it, the slottable query returns Unsupported. Ordinary
Namespace members remain queryable and the Inspector shows the separate
slottable-query reason. This is a capability warning, not a new UML constraint.
Property and Operation signature consistency use the bounded comparisons below;
behavioral/template contract consistency remains unevaluated.

The existing `graph_rules.rs` Slot feature flow now moves a Slot for a private
inherited attribute, changes direct redefinition, diagnoses ambiguous ancestry
and cycles, repairs the graph, and reopens JSON/Protobuf. The existing Namespace
hierarchy/import flow continues exercising diamond ordering and no stale cache.
No new isolated test functions were added. Official sections: UML 2.5.1
§§9.9.4.7, 9.9.9 and 9.9.20.

## Element and NamedElement graph queries

`uml/element_graph_queries.rs` implements the official UML 2.5.1 §7.8.6 and
§7.8.9 query bodies from `UML.xmi`. The pinned uml4net NamedElement fixture was
consulted to compare namespace order and exposed template parameters; no
implementation code was adapted for this slice. The immutable ReferenceIndex
and Model ownership projection are the source of graph identity and edges.
`Model::qualified_name` shares the same `allNamespaces` query.

The ownership query returns an identity set in deterministic traversal order;
namespace order is inner to outer. Owning packages stop at the first non-Package,
while exposed template parameters affect `allNamespaces` only. Dependency client
lookup uses canonical relationship endpoints and includes Dependency subclasses.
Unadapted object-form dependencies, populated alternate composite or derived
slots, duplicate identities and cycles fail closed instead of returning a
partial result. Multi-client dependency representations remain unsupported.
The existing root graph and template-signature flows exercise ordinary queries,
exposed parameters and serialization; they do not imply complete Kernel support.


## Bounded Property redefinition consistency

`uml/feature_compatibility.rs` newly implements the official
`Property-isConsistentWith` composition (§9.9.17.7); it does not claim to adapt a
Property consistency implementation from uml4net, whose pinned Property
extensions do not implement that operation. It reuses the already-attributed
Classifier conformance and Property composite queries, the shared ReferenceIndex,
and the existing arbitrary-precision multiplicity interval adapter. The reusable
TypedElement and MultiplicityElement comparisons can also serve Parameters.

For a redefinition context established by the Classifier query, the redefining
Type must conform to the redefined Type, its multiplicity interval must be
contained in the original interval, and a composite Property must remain
composite. Infinity uses UnlimitedNatural rather than a machine-number sentinel.
The XMI and PDF upper-bound clause accidentally repeat the lower-bound
comparison; the implementation follows their explicit interval-containment
prose rather than requiring equal lower bounds and ignoring upper bounds.

Missing Types, authored Type strings, external Types and unevaluated bounds
return Unsupported. Owned ValueSpecifications are not treated as absent bounds
or silently replaced with 1; their unresolved role/evaluation remains explicit.
Malformed references and invalid literal intervals are errors. A proven false
condition establishes incompatibility even if another condition is unknown.
`Type::conformsTo` retains its normative false default; Classifier conformance
uses canonical user-model ancestry, never metaclass equality or name matching.

The existing redefinition command rejects known incompatibility atomically.
Loaded graphs, member queries and Inspector diagnostics use the same comparison;
unknown cases remain storable with capability warnings. A fully resolved,
compatible Property no longer receives the blanket unsupported-consistency
warning. Property subsetting has the subsequent bounded implementation below;
qualifiers and all template contexts remain outside this comparison. Operation
signatures reuse these comparisons as described below. The existing hierarchy
feature flow now mutates compatible Types/bounds/aggregation, rejects each
incompatibility, preserves unknown bounds and reopens the canonical graph.
No new test functions were introduced.


## Bounded Operation signature consistency

`uml/operation_signatures.rs` newly implements the original XMI
`Operation-isConsistentWith` composition, UML 2.5.1 §9.9.11.6, using
`MultiplicityElement-compatibleWith` (§7.8.8). The pinned uml4net
`OperationExtensions.cs` provides the previously credited return-property query
patterns, not an implementation of this signature comparison. Bellatrix reuses
its existing ordered canonical Parameter projection, generated scalar defaults,
TypedElement/Classifier conformance and reference-aware multiplicity adapters;
there is no second ownership or inheritance graph.

In an established redefinition context, ordered Parameter counts must match and
corresponding directions, ordering and uniqueness must be equal. Names and
visibility are not extra signature constraints. Corresponding Types must
conform in either direction. This follows the official OCL and §9.6.3.1, which
intentionally does not choose a programming language's variance policy. Input
multiplicities may widen, output/return multiplicities may narrow, and inout
multiplicities must remain equal. Arbitrary-precision bounds and infinity retain
the shared interval representation and literal ValueSpecification evaluator.

The Classifier redefinition consumer now rejects proven incompatibility and
removes the previous blanket warning for resolved compatible signatures,
including empty signatures. Unsupported Type/bound evaluation remains a
capability warning and storable data. Shared conjunction/disjunction helpers
preserve known false/true results when other operands are unsupported, while
malformed graph data retains its invalid diagnostic. Both the signature query
and the normative consistency query use this same comparison. The official
`Operation-isConsistentWith-spec` in the pinned 20161101 UML.xmi requires no
behavioral or template-equivalence proof. Pre/post/body conditions, methods,
exceptions and templates therefore do not make this query unsupported. Their
own structural validators and expression-evaluation limits remain independent;
a compatible signature does not certify those other contracts. The Classifier
consumer still checks `Operation-isConsistentWith-pre` through the shared
redefinition-context query before comparing signatures.

The existing Namespace/Classifier feature workflow now constructs and edits
three ordered signatures through commands, establishes redefinitions, changes
Parameter order/count/Type/bounds/flags, repairs the model, preserves an unknown
bound with Inspector diagnostics, and reopens JSON and Protobuf. It also creates
templates through commands and imports conditions, methods and exceptions on
either side of a redefinition, since those reference roles do not yet have an
editable Inspector adapter. It keeps incompatible signatures and invalid
contexts rejected and checks condition/reference diagnostics on imported drafts
independently through repair and reopen.
No test function or isolated unit test was added.


## Bounded Property subsetting

`uml/property_subsetting.rs` implements the original XMI
`Property-subsettingContext`, `Property-subsetting_context_conforms`,
`Property-subsetting_rules` and `Property-subsetted_property_names`, also specified
by UML 2.5.1 §§9.9.17.7–8. This composition is new: the pinned uml4net Property
extensions do not implement those queries. It reuses Bellatrix's previously
attributed Association membership and Classifier conformance adaptations, the
existing generated ownership-subset walker, one snapshot ReferenceIndex and the
shared literal ValueSpecification evaluator. No separate AST, model ownership
list or persistent subset-closure cache is created.

Canonical `subsettedProperty` reference collections use generated cardinality,
uniqueness and target-metaclass checks. A subsetting Property must have a
different name, a Type conforming to the subsetted Type, and an upper bound no
greater than the subsetted upper bound. No lower-bound containment rule is
invented. The shared upperBound query evaluates the upper independently of an
unknown lower, without certifying the whole interval. Mixed external/local
collections preserve known local errors; unresolved targets never turn them
into mere capability warnings. The shared UnlimitedNatural bound comparison also serves the existing
multiplicity interval containment; unbounded values never become machine
maximums.

The context is the owning Classifier for an attribute, or the Types at every
other Association end when it is an end. This applies to n-ary Associations and
takes precedence over classifier ownership. Each source context must conform to
at least one context of each target Property. Equal contexts are permitted;
this is not strict redefinition ancestry. Subsetting does not suppress inherited
features or remove the original Property from Slot eligibility.

`Command::SetSubsettedProperties`, structural validation and the existing
Inspector consume the same query. Known violations reject explicit semantic
mutations atomically. Missing/external Types, unevaluated bounds, ambiguous
ownership and alternate association/context representations return Unsupported
capability diagnostics and remain storable; they are not added as visual
placement prohibitions. Literal bounds are evaluated in memory and no engine
query accesses the filesystem or network. Qualifier semantics, arbitrary
expressions and general external-document identity resolution remain outside
this slice.

The existing Slot/classifier lifecycle now switches from redefinition to
subsetting, preserves the original feature's Slot eligibility, reparents within
valid contexts, edits names/Types/finite and unlimited bounds, keeps an unknown
expression with Inspector diagnostics, adds a n-ary end using the same rule,
diagnoses a generalization cycle, and reopens JSON/Protobuf. No test function or
isolated unit test was added.


## RedefinableElement contexts and Association-end inheritance

`uml/redefinition_contexts.rs` adapts the derived-union selection pattern from
pinned `uml4net/Extend/RedefinableElementExtensions.cs::QueryRedefinitionContext`
and `IsRedefinableElementOf`. The authoritative source is the original XMI
`RedefinableElement-redefinitionContext`, its opposite subsets and
`RedefinableElement-isRedefinitionContextValid`, plus UML 2.5.1 §9.9.18.7.
Bellatrix uses its existing generated subset/redefinition walker to identify
ownership contributions, rather than copying a metaclass switch or storing
another context list. The upstream code and fixtures were inspected, not run.

A Classifier owner contributes only through a property in the redefinition
context union: attributes, Association-owned ends, Operations, nested
Classifiers, owned Connectors and redefinable template signatures can contribute.
Arbitrary ownership does not: a Reception or ActivityNode is not assigned its
owner as a context merely because that owner is a Classifier. Derived Behavior
contexts and specialized state-machine context queries/predicates remain
explicit Unsupported capabilities; authored context/owner-reference lists do
not replace canonical ownership.

The default predicate follows the OCL precisely: at least one source context's
`allParents` must include every target context. It does not use metaclass equality
or allow the same source/target context. An empty source context yields false;
with a source context, the empty target set follows the OCL's vacuous includesAll.
The existing cycle-safe user-model ancestry implementation is reused. The
context precondition is established before Property or Operation consistency is
compared, so an unevaluated context cannot become a false consistency verdict.

Property has the additional `Property-redefined_property_inherited` condition:
the target must occur in an ancestor's allFeatures. The consumer previously
restricted Properties to attributes; it now correctly allows an Association's
ownedEnd, which is a Feature but not an attribute. Context derivation remains
separate from subsetting context, where an Association end instead uses the
Types at its other ends. The existing redefinition command, structural queries
and Inspector consume the same computations; no new UI prohibition is added.
General specialized redefinedElement unions and their consistency predicates
are not claimed by this bounded slice.

The existing public Association lifecycle now imports a specialization,
redefines an owned end, rejects a context-breaking reparent and a generalization
cycle atomically, and reopens JSON/Protobuf while checking inherited Features and
derived Inspector context. No new test function or isolated unit test was added.


## Classifier specialization type and final parents

The canonical Generalization reader in `uml/classifier_hierarchy.rs` now
consumes the official `Classifier-specialize_type`,
`Classifier-maySpecializeType` and `Classifier-non_final_parents` contracts
(UML 2.5.1 §§9.9.4.7–8). This composition is new; the pinned uml4net Classifier
extensions provided the already-attributed ancestry patterns, not these
validation rules.

`maySpecializeType` uses the exact default OCL
`self.oclIsKindOf(c.oclType())`: the specializing Classifier's metaclass must
conform to the parent's metaclass. This is intentionally separate from
user-model `Classifier::conformsTo` and its Generalization closure. For example,
a Component may specialize a Class; a Component cannot use Generalization to
specialize an Interface. The pinned UML XMI declares no specialized override of
this predicate. Unresolved metaclass metadata yields an Unsupported query
result rather than an invented type policy.

Each parent must also have `isFinalSpecialization = false`, using its generated
Boolean/default contract. A final Classifier may itself inherit from another
Classifier; finality only prevents it from being a parent. The existing scalar
reader was moved from the Operation adapter into `schema.rs` and is shared by
Classifier checks, Operation return properties and Operation signature
comparison. No duplicate defaults or second validation graph were introduced.

Invalid specialization edges are diagnosed before ancestry is used by
Namespace inheritance, Type conformance, redefinition or Slot eligibility.
Existing semantic command validation therefore rejects a known-invalid edit
atomically; this adds no diagram-placement gate or filesystem/network access.
GeneralizationSet/powertype semantics remain outside these two bounded rules.

The existing Classifier/Slot workflow rejects making an existing parent final,
keeps inherited Slots usable on a final Component specializing a Class, imports
and diagnoses an incompatible Interface parent, repairs the model, and reopens
JSON/Protobuf. No new test function or isolated unit test was added.
