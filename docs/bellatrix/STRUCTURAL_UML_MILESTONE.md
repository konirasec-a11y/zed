# Bellatrix structural UML milestone

## Scope and authority

This increment extends the single `bellatrix_mde` domain. It does not claim
complete UML 2.5.1 support or seven complete specialized editor palettes.
The normative inputs remain `standards/uml/20161101/UML.xmi` and `UMLDI.xmi`.
RUML and StarUML remain implementation/interaction references, not authorities.

Reuse order: inspect Zed protocol contracts and follow their handlers; where
no local diagram contract exists, use native Item/Panel/Action APIs and the
existing typed Cordis bridge. No Zed core edits are required by this increment.

## Implemented contracts (runtime verification recorded separately)

| Contract | Normative basis / implementation | Focused evidence |
| --- | --- | --- |
| Metaclass ancestry | Generated from all 242 UML metaclasses; `uml::metamodel` | Generator `--check`, unknown-name and inheritance tests |
| Model-wide generalization | `Classifier-no_cycles_in_generalization` | Hidden/unrendered edges and cycles tested |
| Compatible specialization | `Classifier-specialize_type`, `Classifier-maySpecializeType-spec` | Metaclass conformance uses the generated catalog |
| Final parent | `Classifier-non_final_parents` | `isFinalSpecialization`, not a substitute `isLeaf` flag |
| Stereotype specialization | `Stereotype-generalize` | Both directions reject non-stereotype peers |
| Diagram metadata | Required nonblank `uml_di_type`, explicit context | Missing/null/blank JSON and missing/blank protobuf are rejected |
| Context of six diagram categories | UMLDI `*-no_modelElement` | Context is not the list of depicted subjects |
| Composite Structure context | `UMLCompositeStructureDiagram-modelElement_mult/modelElement_type` | Exactly one StructuredClassifier context; instance context is explicitly unsupported |
| Safe domain mutation | `commands::apply`, Project revision integration | New invalid generalizations roll back; unrelated drafts remain editable |
| Diagram lifecycle | Create/rename/delete reuse domain commands and Cordis | Delete removes only diagram/views, never semantic elements |
| UI integration | Native Zed controls in Bellatrix Workbench | New-diagram menu, stored type display, coordinate/selection tests |

## Seven-category coverage boundary

Class, Object, Package, Component, Composite Structure, Deployment and Profile
have explicit type identities and shared model/projection checks. This is not
proof of complete per-element constraint coverage, rendering or editing for
each category. The toolbox remains labelled as Class editing tools.

Composite Structure context may also be an InstanceSpecification with a
StructuredClassifier classifier under UMLDI. The current model adapter does not
yet define a validated instance-classifier encoding: this case emits an explicit
unsupported diagnostic, not a false conformance success. Complete Component,
Deployment, Object and Profile editing still needs dedicated model/rule/view
acceptance. Absence of a rule must never be reported as verified conformance.

## Persistence and feedback

The beta contract now requires a nonblank `uml_di_type`. Missing type data is
rejected rather than inferred as Class. Protobuf field 6 keeps presence detection
so a missing value is explicitly rejected by the decoder. `model_elements`
represents context, not visible subjects. Explicit future type strings may be
stored, but the current validator rejects unsupported categories.

The only persisted MDE namespace is `bellatrix.mde`; old namespaces and event
prefixes have no fallback readers, adapters or compatibility aliases. Producers,
consumers, fixtures and tests are updated together. Old beta project documents
must be recreated or explicitly converted outside the runtime. This intentionally
supersedes the compatibility policy of earlier checkpoints.

Validation is read-only. Domain mutations publish through the existing
revision-checked ProjectService path. Code, model and native GUI verification
are distinct: static checks or passing headless tests do not establish native
interaction success.

## Fast feedback commands

```text
python script/bellatrix_mde/static_check.py
python script/bellatrix_mde/bellatrix_uml_catalog.py --check
cargo +1.98.1 test -p bellatrix_mde
cargo +1.98.1 test -p bellatrix_mde_cordis
```

Use focused headless checks before a full Zed build. Preserve warmed Cargo caches.
The maintainer validates native Windows behavior on the working branch; no PR
or manual CI run is required to obtain that feedback.

## Verification record — 2026-11-02

- Exact copied sources of seven headless packages: **154 tests passed, zero failures**.
- Repository `script/clippy` on the isolated domain/Cordis packages: **passed**, release/all-targets/all-features with warnings denied.
- Rust toolchain: 1.98.1. Shared Cargo/target caches retained between runs.
- Source fingerprint recorded by `bellatrix_headless.py`; copied sources were
  unchanged during the run. Root workspace patches/profiles/GUI dependencies
  are deliberately outside this supplemental check.
- Root `cargo metadata --offline --no-deps`: passed, 278 workspace packages.
- Catalog deterministic check and source-contract check: passed.
- Two exact extracted Canvas coordinate/selection helper tests: passed with
  rustc; this is not a GPUI compilation or interaction test.
- Native Zed/GPUI/Windows behavior: not executed here.
- Root Cargo.lock resolution: pending; no dependency hashes invented.

The earlier class-diagram acceptance fixture attempted Class-to-Interface
Generalization. It now uses Class-to-Class for Generalization, keeps
Class-to-Interface for InterfaceRealization, and explicitly checks rejection
and rollback for the invalid Generalization. Aggregation/composition checks
use the existing canonical Association/end1Aggregation representation.

Focused reproducible check, without the graphical dependency graph:

```text
python script/bellatrix_mde/bellatrix_headless.py --cache-dir <outside-repository-cache> -- test --workspace --no-fail-fast
```

This command tests only the seven isolated headless packages and preserves its
build cache. It does not replace native workspace acceptance.

## Represented structural relationship rules

The shared `validate_structural_model` entry point also runs
`crates/bellatrix_mde/src/structural_rules.rs`. This is a bounded interpretation
of the existing generic model, not full UML conformance or a new OCL runtime.
Official references below are in `standards/uml/20161101/UML.xmi` and the OMG
UML 2.5.1 specification.

| Represented check | Normative trace | Interpretation and limits |
| --- | --- | --- |
| InterfaceRealization source | `InterfaceRealization-implementingClassifier`; §10.5.6.4 | Source must conform to `BehavioredClassifier`, using the generated metaclass catalog. Being merely a `Classifier` is insufficient. Target checking remains in the existing validator. |
| Binary Association endpoint Types | `Association-ends_must_be_typed`, `TypedElement-type`; §11.8.1 | `from` and `to` represent member-end Types, not Property identities. Applies to `uml.association` and the current directed-association editor representation. Graph validation separately checks unresolved IDs. |
| Package-owned visibility | `Package-elements_public_or_private`; §12.4.5.8 | Direct `owns` links from Package-conforming elements to PackageableElement-conforming children represent packaged elements. Present visibility must be public/private; absence/null is skipped. Does not constrain protected Class-owned features or visual nesting. |
| Literal association multiplicities | `MultiplicityElement-lower_ge_0`, `MultiplicityElement-upper_is_unlimitedNatural`, `MultiplicityElement-upper_ge_lower`; §7.8.8.8 | Checks `sourceMultiplicity` and `targetMultiplicity` string literals: single integer, `lower..upper`, or `*` (0..*). Decimal comparison does not impose a machine-integer bound. |

Errors in numeric multiplicity bounds use `UML_MULTIPLICITY_BOUNDS`.
Absent multiplicity properties are skipped. Empty or nonliteral values produce
`MDE_MULTIPLICITY_UNSUPPORTED` warnings, not claims that UML expressions are
invalid: `MultiplicityElement-value_specification_constant` permits constant
ValueSpecifications, which this validator does not evaluate. Nonstring
multiplicity and visibility properties produce explicit MDE representation
errors, distinct from normative diagnostics.

These checks neither prohibit reflexive Associations nor impose dependency or
aggregation acyclicity. Composite opposite-end multiplicity is deferred until
the current diamond-side `end1Aggregation` convention has a documented mapping
to UML Property aggregation and its opposite end. Association specialization,
n-ary ends, Property ownership, and expression evaluation remain outside this
slice. Focused tests are in the rule module, with positive and negative cases
for each represented rule and unsupported-state diagnostics.

Unmapped `uml.*` element adapters, including recognized editor concepts without
normative metaclass mappings, emit `MDE_UML_METACLASS_UNSUPPORTED` warnings at
model level. Endpoint rules likewise report unsupported mappings rather than
inferring normative type errors. Known mapped but nonconforming metaclasses
remain errors. Standalone non-UML kinds are not blanket-rejected.

## Beta canonicalization and notation

The next increment removes legacy namespace readers, compatibility reexports
and the implicit Class-diagram fallback. `bellatrix.mde` is defined once in the
domain. Missing/blank diagram type is a format error. The old aggregate/composite
persisted relation kinds are rejected; their visual notations still derive from
canonical Association properties. Template no longer pretends to be a normative
Classifier merely for compatibility.

Mandatory UML keywords and user-provided stereotype text are independent header
fields. Component, Artifact and Stereotype reuse supported classifier rectangles;
Profile reuses package tabs. Node/Device, InstanceSpecification and Collaboration
are not silently drawn as Class. Shared header sizing is used by display, routing
and resize. A stereotype text label is not a fully validated ProfileApplication.

The extended validation also checks represented InterfaceRealization source
metaclasses, Association endpoint Types, literal multiplicity bounds, and direct
Package ownership visibility. Unsupported expressions and unresolved mappings
produce warnings rather than false normative violations. These checks are not a
complete implementation of Association ends or all seven diagram metamodels.

## Root workspace and view-only editing checkpoint

Cargo itself refreshed the real root `Cargo.lock`; the delta is limited to the
consolidated Bellatrix packages and Tera. Root metadata resolution succeeded.
Actual root workspace tests with `--locked --offline` passed **147 tests** for
`bellatrix_mde`, `bellatrix_mde_cordis` and `bellatrix_project`. This is a different
scope from the seven-package supplemental suite, not a decrease in its coverage.
No native graphical crate was built by that check.

RemoveView is distinct from DeleteElement: ordinary diagram deletion removes a
selected View and newly unrenderable local edge Views, preserving semantic objects,
other diagrams and alternate Views. Explicit Delete from Model retains semantic
cascading and context-reference protection. Endpoint labels project existing
role/multiplicity/name properties without changing arrow semantics or model IDs.

The unused imported structural/behavioral DTO family was removed after checking
all consumers, includes, manifests and scripts. The active metamodel catalog and
canonical domain remain; original reference/licensing files remain documentation.

One necessary native integration-line change replaces the existing Bellatrix
bootstrap `expect` with Zed's existing `log_err`. An MDE startup failure must not
panic the entire Zed editor. This changes only the already-added Bellatrix hook,
not Zed's editor/runtime logic; native startup behavior still needs Windows QA.

The view-only slice also adds PlaceElementView: existing elements are shown using
fresh view identities, copied geometry/style where available, and shared factory
geometry otherwise. Explorer Show in active diagram focuses an existing view
instead of duplicating it as a convenience; multiple views remain legal data.

Root Clippy on MDE/Cordis passed with the real repository configuration and
warnings denied. The redundant Add prefix was removed by adopting UmlQuickCreate
noun variants, and the obsolete Interface-only event wrapper was removed rather
than kept as a compatibility envelope. Interface behavior uses the same canonical
Quick Edit path and its acceptance test remains active.
