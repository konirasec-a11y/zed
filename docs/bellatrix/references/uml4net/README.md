# Kernel graph references

Bellatrix adapts query logic and selected fixtures from
[STARIONGROUP/uml4net](https://github.com/STARIONGROUP/uml4net/tree/8ef13af2c68ae1994c8fa4f5b3e231dee7e6715d),
commit `8ef13af2c68ae1994c8fa4f5b3e231dee7e6715d`:

- `uml4net/Extend/AssociationExtensions.cs`: all-member, deduplicated `endType`.
- `uml4net/Extend/PropertyExtensions.cs`: own `aggregation` determines
  `isComposite`; only binary membership has a unique `opposite`.
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

ElementImport, PackageImport, import aliases/collisions, explicit collaboration
roles and other unimplemented membership sources do not produce a partial
successful member result. A query returns an explicit unsupported-capability
error when their data appears, including in an ancestor; model diagnostics
retain this as a capability warning rather than a UML prohibition. The
Inspector shows the unavailable-query reason. Dependent redefinition commands
still fail closed. Independently stored derived member/feature/attribute arrays
are not accepted as an authoritative alternate graph.
