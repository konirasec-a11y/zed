# Bellatrix normative UML extraction

`bellatrix_uml_catalog.py` compiles the pinned OMG XMI definitions into Rust
structural contracts. It is a reusable, standard-library-only Python 3.11+
program. It does not download mutable inputs, run OCL, or create a second user
model. Handwritten Rust consumes its output against Bellatrix's canonical
`Project` / `ModelingSection` records.

## Graph architecture

The engine operates on the canonical typed UML graph. Extracted XMI identities
and references describe its type/contract graph; the user-authored graph is
constrained by that metamodel. Generated descriptors and borrowed Rust handles
query those graphs, not a parallel object-model store. UI projections, commands
and validation share the same authoritative user-model state.

## Generate and verify

From the repository root:

```sh
python3 script/bellatrix_mde/bellatrix_uml_catalog.py
python3 script/bellatrix_mde/bellatrix_uml_catalog.py --check
```

The generated files are `crates/bellatrix_mde/src/contracts/uml/metamodel.rs` and
`crates/bellatrix_mde/src/contracts/uml/metamodel.inventory.json`. Do not edit
these files manually or use a formatter as a substitute for generation. The
extractor locates its inputs relative to its own repository location, so it
also works from another working directory. To inspect a separate output:

```sh
python3 script/bellatrix_mde/bellatrix_uml_catalog.py --output /tmp/uml-metamodel.rs
python3 script/bellatrix_mde/bellatrix_uml_catalog.py --check --output /tmp/uml-metamodel.rs
```

The coverage JSON defaults to the Rust output's `.inventory.json` sibling;
`--inventory-output PATH` selects another destination. `--check` checks both
outputs, performs no writes, and exits nonzero on drift, missing inputs, or an
unsupported extraction shape. Output is deterministic for the same inputs and
extractor. It imports descriptor types from the handwritten `bellatrix_mde::contracts::uml::schema`;
it is not a standalone Rust crate.

## Windows checkouts

The scoped `.gitattributes` rules preserve the eight normative XMI files
byte-for-byte (`-text`) and keep the extractor and both generated outputs in LF
form, including with `core.autocrlf=true`. The extractor reads/writes UTF-8 bytes;
Python text-mode newline translation does not affect its output or `--check`.

A checkout made before these attributes were added may already contain
converted XMI files. Changing attributes alone need not rewrite untouched
files. If input hashes differ, preserve any intentional local edits before
refreshing only the official inputs from the committed revision; regeneration
alone cannot repair changed input bytes.

## Pinned inputs and identities

The four required roots are in `docs/bellatrix/standards/uml/20161101/`:

- `UML.xmi`: the 242 UML metaclass definitions, associations and enumerations.
- `PrimitiveTypes.xmi`: the five normative primitive definitions.
- `StandardProfile.xmi`: 33 stereotypes and 34 Extensions, including their ends.
- `UMLDI.xmi`: 33 named diagram metaclass definitions and their contracts.

Their transitive imports are also required, under
`docs/bellatrix/standards/uml/imports/20131001/`:

- `PrimitiveTypes.xmi`, referenced by both UML versions and UMLDI.
- `DI.xmi`, referenced by UMLDI.
- `DC.xmi`, referenced by DI.
- `UML.xmi`, referenced by DI.

The import retrieval record is `docs/bellatrix/standards/uml/imports/PROVENANCE.json`.
Generated `SOURCES` contains the eight paths, source URLs, exact SHA-256 hashes,
and resolution counts. No network connection is needed for regeneration.

Every generated identity is `(Document, XMI fragment)`. The 20131001 and
20161101 UML and primitive documents remain distinct. The extractor does not
alias versions by matching names. An external `href` node is a reference, not a
classifier definition. All 522 external href occurrences in these pinned
inputs resolve to a definition in the eight-document set.

The official inputs also contain internal defects, which are not repaired
silently:

- UML 20161101 repeats eight IDs around `State-isConsistentWith`. Ten reference
  occurrences therefore have ambiguous targets.
- Its `UML_mofTag0` points to `UML`, which is not that document's root ID.
- UML 20131001 repeats seven package-import IDs in different packages.

`AMBIGUOUS_DEFINITIONS` records all 15 conflicting identities and their owners.
`UNRESOLVED_REFERENCES` records the 11 ambiguous or missing reference occurrences.
Ambiguous definitions are excluded from the executable descriptor tables;
the resolver never selects the first definition as if it were unique.

## Generated contracts

The generated tables contain:

- Classifier kinds, abstractness, source-qualified generalizations, properties,
  ordered association member ends, operations, and enumeration literal names and IDs.
- 2,059 Property/ExtensionEnd descriptors with Type references, lower/upper
  bounds, typed defaults, ordering, uniqueness, derived/read-only flags,
  aggregation, association, binary opposite, subsets, and redefinitions.
- 405 unambiguous operation declarations with ordered parameters, redefinitions,
  preconditions and body conditions, plus 510 typed parameter declarations.
- 1,370 constraint/specification pairs, preserving the actual bodies and declared
  languages in source order. These are not compiled or interpreted OCL.
- 3,341 documentation comments with their text, annotated targets and provenance.
- A reproducible JSON inventory of every identified XMI definition category,
  its occurrence/unique/unambiguous counts and representation. It separately
  names the executable consumers, excluded ambiguities and unsupported categories.
- Compatibility name/inheritance tables used by existing Bellatrix callers.

Bounds follow the literal and operation contracts in UML 2.5.1: an omitted
MultiplicityElement bound defaults to 1, a present empty LiteralInteger or
LiteralUnlimitedNatural evaluates to 0, and ExtensionEnd's omitted lower bound
is 0. Boolean defaults, enumeration InstanceValue references, and literal
values are resolved by type. No Integer is narrowed to a machine-sized type.

## Executable Rust consumers

`crates/bellatrix_mde/src/contracts/uml/schema.rs` is handwritten and separate from the
extractor. It provides source-qualified conformance and inherited-property
queries, follows explicit redefinition edges, resolves effective defaults,
and reports ambiguous or incomplete lookup. It also exposes source documentation,
operation parameter contracts and specification bodies for implementation/review.
Subsetting and equal property
names are not treated as redefinition.

The engine retains authored Element readers, structural diagnostics and edit policy in
`crates/bellatrix_mde/src/model/uml/metamodel/schema.rs`; the contracts module has no dependency
on engine validation or editor state. Existing engine paths re-export and use these contracts:

- `uml/metamodel.rs` delegates inheritance and property queries to this runtime.
- `uml/root.rs` reads effective visibility defaults and validates explicitly
  supplied scalar property values on canonical model Elements.
- `inspector.rs` uses the same property types, enum literals, defaults and
  read-only flags. Its command setter checks values before changing canonical
  records. Derived properties with no implemented setter remain non-editable;
  this is an implementation limit, not an assertion that all derived UML
  properties are normatively read-only.

Supported scalar checks include Boolean, String, declared enumeration literals,
exact decimal Integer, nonnegative UnlimitedNatural or `*`, and finite JSON
Real values. Decimal strings preserve unbounded integer values through JSON
and Protobuf. Explicit null is accepted only for lower bound 0. A supplied
value is rejected when the upper bound is 0. Omitted draft fields are not
materialized or rejected wholesale, and reading defaults does not mutate the
model.

`Property::type` now uses a local canonical reference: `properties.type =
{"$ref":"element-id"}`. Omission/null is untyped, following UML 2.5.1
§§7.5.3.1 and 7.8.22 (`TypedElement-type` in UML.xmi). The generated inherited
property descriptor determines the multiplicity and required target metaclass.
A borrowed `ReferenceIndex` for each model snapshot rejects missing, ambiguous,
or nonconforming targets without copying model objects or globally caching IDs.
Graph validation protects commands such as Type deletion as well as field edits.

The Property Inspector offers existing model Types through its realtime command
queue. Model-aware labels resolve the current target name for Inspector, hierarchy,
and diagram members/ports/parts. A Type rename never rewrites reference identities.
Operation's read-only return Type is now derived from its sole return Parameter.
The existing Class Quick Edit flow covers ordered Parameter identities, generated
direction defaults/choices, typed optional references, atomic second-return
rejection, reordering/deletion, and JSON/Protobuf reopen. Parameter order is the
canonical `owns` edge order; no parallel ownership list or stored Operation Type
is introduced. Native Inspector navigation and shared labels consume the same
queries. Full Operation/BehavioralFeature semantics remain outside this slice;
see `docs/bellatrix/references/RUML.md` for exact contracts and boundaries.

Historical `properties.type` strings are unresolved authored text, never matched
to Types by name. They remain visibly distinguished and unchanged on read. The
first reference edit preserves a legacy string in `properties.typeText`; a conflict
with existing different text rejects the edit rather than losing either value.
`properties.defaultValue` strings remain declarations, not validated
ValueSpecification references. Unknown extension/editor properties retain their
existing policy.

## Verification and remaining work

Commit and push before running checks, builds or tests. `verify.ps1` checks the
four retained crates and invokes `smoke.ps1`, which selects the existing BDD
integration targets explicitly. It does not run unit tests. The former CLI
artifact smoke has been retired; the application entrypoint is `cargo run -p zed`.

Supplemental headless validation copies only `bellatrix_mde` and
`bellatrix_mde_cordis`, without the Zed UI workspace. With no Cargo arguments,
the helper runs all five existing BDD targets. For one target:

```sh
python3 script/bellatrix_mde/bellatrix_headless.py \
  --cache-dir /tmp/bellatrix-headless -- test -p bellatrix_mde --test bdd_diagram_flows
```

This increment does not implement general OCL execution, generic collection or
external-document reference validation, association/subsetting/redefinition consistency, derived-expression
maintenance, stereotype application, all UMLDI rendering rules, or a complete
UML ValueSpecification evaluator. Package/Profile containers, package-import
object identities and MOF tags are reference-audited and inventoried, not yet
represented by their own typed descriptors. A resolved property Type is not proof that
its storage adapter exists. `scalar_kind` and its unsupported-contract results
make that boundary explicit. Finite JSON Real checking is a transport boundary,
not a complete implementation of mathematical Real semantics. The descriptors
cover behavioral metaclasses present in the sources, but do not implement
behavioral execution.

## Shared graph-query indexes

Schema queries share immutable, source-qualified descriptor indexes and lazily
cached ancestor/inherited-property results. These caches contain only pinned
metamodel data; they do not cache user-model validity, copy a user graph, or
implement incremental validation. Returned inherited-property collections
remain isolated from callers' edits.

A fixed release-mode fixture of 201 objects and 299 relations was measured
before and after the change. The median complete structural-validation pass
went from 189.6 ms to 50.6 ms across five warm runs, with identical diagnostics.
An exhaustive conformance/inheritance/property/default result fingerprint
also stayed unchanged. This is a scoped measurement, not a guaranteed speedup
for every model or a native UI benchmark. Root ownership scans remain outside
this optimization.
