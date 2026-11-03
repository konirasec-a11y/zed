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
`crates/bellatrix_mde/src/uml/associations.rs`; adapted tests are in
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
qualifiers, specialization, redefinition and owned ValueSpecification evaluation
remain outside this slice. Literal multiplicity notation is a declared adapter,
not a claim that lowerValue/upperValue model identities exist.

## Derived Operation return properties

The pinned XMI `Operation-isOrdered.1`, `Operation-isUnique.1`,
`Operation-lower.1` and `Operation-upper.1`, also specified by UML 2.5.1
§9.9.11.6, derive all four fields from the single canonical return Parameter.
Without a return Parameter, the results are false, true, null and null.
With a return Parameter, ordering/uniqueness use its generated scalar defaults;
missing multiplicity notation is 1..1. The existing shared BigInt Integer and
UnlimitedNatural interval adapter evaluates literal notation without a machine
integer limit. Property and Parameter use that same adapter. Present authored
ValueSpecifications or nonliteral bounds remain explicitly unsupported and
never masquerade as absent/default bounds. No ValueSpecification identities or
expression evaluator are invented.

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

`crates/bellatrix_mde/src/uml/classifiers.rs` adapts the derived-union and
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
context before committing. It does not implement all Property or Operation
`isConsistentWith` constraints: full type/multiplicity compatibility, operation
signature and pre/postcondition conformance remain outside this slice. Every
nonempty supported redefinition list therefore retains an explicit
`UML_REDEFINITION_CONSISTENCY_UNSUPPORTED` capability warning; successful
membership computation does not certify redefinition consistency. The
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

`crates/bellatrix_mde/src/uml/namespace_imports.rs` adapts import and name
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
Complete Property/Operation redefinition consistency remains unevaluated.

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
